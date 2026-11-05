//! Native, idle-static Windows companion and temporary copy-only file handoff.
use super::*;
use std::{
    cell::RefCell,
    sync::atomic::{AtomicBool, AtomicIsize, Ordering},
    time::Instant,
};
use windows::{
    core::{implement, w, Ref, BOOL, HRESULT, PCWSTR},
    Win32::{
        Foundation::*,
        Graphics::Gdi::*,
        System::{
            Com::*, DataExchange::RegisterClipboardFormatW, LibraryLoader::GetModuleHandleW,
            Memory::*, Ole::*, SystemServices::*,
        },
        UI::{
            HiDpi::GetDpiForWindow, Input::KeyboardAndMouse::*, Shell::*, WindowsAndMessaging::*,
        },
    },
};

const SHOW: u32 = WM_APP + 11;
const HIDE: u32 = WM_APP + 12;
const RESET: u32 = WM_APP + 13;
const HOLD: u32 = WM_APP + 14;
const COPY: u32 = WM_APP + 15;
const COMMANDS: u32 = WM_APP + 16;
const CLOSE_COMMANDS: u32 = WM_APP + 17;
const HOLD_SECS: u64 = 120;
const WIDTH: i32 = 128;
const HEIGHT: i32 = 136;
#[derive(Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    visible: bool,
    command_bar_visible: bool,
    count: usize,
    seconds_left: u64,
    message: String,
}
pub struct Control {
    hwnd: AtomicIsize,
    stopping: AtomicBool,
    status: Mutex<Status>,
    pending: Mutex<Option<Vec<PathBuf>>>,
    copy_reply: Mutex<Option<std::sync::mpsc::SyncSender<Result<(), String>>>>,
}
struct State {
    app: tauri::AppHandle,
    control: Arc<Control>,
    files: Vec<PathBuf>,
    deadline: Option<Instant>,
    press: Option<(POINT, RECT, bool)>,
    hover: bool,
}
thread_local! { static STATE: RefCell<Option<State>> = const { RefCell::new(None) }; }
fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}
fn with_state<T>(f: impl FnOnce(&mut State) -> T) -> Option<T> {
    STATE.with(|s| s.borrow_mut().as_mut().map(f))
}
fn seconds(deadline: Option<Instant>) -> u64 {
    deadline
        .map(|d| {
            d.saturating_duration_since(Instant::now())
                .as_millis()
                .div_ceil(1000) as u64
        })
        .unwrap_or(0)
}
fn publish(state: &State) {
    if let Ok(mut status) = state.control.status.lock() {
        status.count = state.files.len();
        status.seconds_left = seconds(state.deadline);
    }
}
unsafe fn hold(hwnd: HWND, state: &mut State, files: Vec<PathBuf>) {
    state.files = files;
    state.deadline = Some(Instant::now() + Duration::from_secs(HOLD_SECS));
    SetTimer(Some(hwnd), 1, 1000, None);
    if let Ok(mut status) = state.control.status.lock() {
        status.message =
            "Files held for 120 seconds. Drag to copy out, or use Copy held files.".into();
    }
    publish(state);
}
fn post_main(app: &tauri::AppHandle, quit: bool) {
    let h = app.clone();
    let _ = app.run_on_main_thread(move || {
        if quit {
            request_quit(&h)
        } else {
            show_panel(&h)
        }
    });
}
fn save(app: &tauri::AppHandle, key: &str, value: &str) {
    if let Some(core) = app.try_state::<Shared>() {
        if let Ok(db) = core.db.lock() {
            let _=db.execute("INSERT INTO settings(key,value) VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value",params![key,value]);
        }
    }
}
fn setting(app: &tauri::AppHandle, key: &str) -> Option<String> {
    let core = app.try_state::<Shared>()?;
    let db = core.db.lock().ok()?;
    db.query_row("SELECT value FROM settings WHERE key=?1", [key], |r| {
        r.get(0)
    })
    .ok()
}
unsafe fn expire(hwnd: HWND) {
    with_state(|s| {
        if seconds(s.deadline) == 0 && s.deadline.is_some() {
            s.files.clear();
            s.deadline = None;
            let _ = KillTimer(Some(hwnd), 1);
            if let Ok(mut v) = s.control.status.lock() {
                v.message = "Hold expired. Original files were left in place.".into();
            }
        }
        publish(s);
    });
    let _ = InvalidateRect(Some(hwnd), None, false);
}
fn format() -> FORMATETC {
    FORMATETC {
        cfFormat: CF_HDROP.0,
        dwAspect: DVASPECT_CONTENT.0,
        lindex: -1,
        tymed: TYMED_HGLOBAL.0 as u32,
        ..Default::default()
    }
}
fn validated(paths: Vec<PathBuf>) -> Result<Vec<PathBuf>, String> {
    if paths.is_empty() || paths.len() > 20 {
        return Err("Drop 1–20 local files or folders at a time.".into());
    }
    let mut total = 0;
    let mut out = Vec::new();
    for path in paths {
        // Avoid network shares, virtual attachments, and unbounded metadata work.
        let local = matches!(path.components().next(),Some(std::path::Component::Prefix(p)) if matches!(p.kind(), std::path::Prefix::Disk(_) | std::path::Prefix::VerbatimDisk(_)));
        total += path.as_os_str().len();
        if !local || !path.exists() || total > 65536 {
            return Err(
                "Only existing local file/folder paths are supported (64 KiB path limit).".into(),
            );
        }
        if !out.contains(&path) {
            out.push(path);
        }
    }
    Ok(out)
}
unsafe fn paths(data: &IDataObject) -> Result<Vec<PathBuf>, String> {
    let mut medium = data.GetData(&format()).map_err(err)?;
    let result = (|| {
        if medium.tymed != TYMED_HGLOBAL.0 as u32 {
            return Err("Unsupported file format.".into());
        }
        let drop = HDROP(medium.u.hGlobal.0);
        let count = DragQueryFileW(drop, u32::MAX, None);
        if count == 0 || count > 20 {
            return Err("Drop 1–20 local files or folders at a time.".into());
        }
        let mut out = Vec::new();
        for i in 0..count {
            let len = DragQueryFileW(drop, i, None);
            if len == 0 || len > 32767 {
                return Err("File path is too long.".into());
            }
            let mut buf = vec![0u16; len as usize + 1];
            if DragQueryFileW(drop, i, Some(&mut buf)) != len {
                return Err("Could not read the complete file path.".into());
            }
            use std::os::windows::ffi::OsStringExt;
            out.push(PathBuf::from(std::ffi::OsString::from_wide(
                &buf[..len as usize],
            )));
        }
        validated(out)
    })();
    ReleaseStgMedium(&mut medium);
    result
}
#[implement(IDropTarget)]
struct Target {
    hwnd: isize,
    supported: AtomicBool,
}
impl IDropTarget_Impl for Target_Impl {
    fn DragEnter(
        &self,
        data: Ref<IDataObject>,
        _: MODIFIERKEYS_FLAGS,
        _: &POINTL,
        effect: *mut DROPEFFECT,
    ) -> windows::core::Result<()> {
        unsafe {
            let okay = data
                .as_ref()
                .is_some_and(|d| d.QueryGetData(&format()).is_ok())
                && (*effect & DROPEFFECT_COPY) != DROPEFFECT_NONE;
            self.supported.store(okay, Ordering::Relaxed);
            *effect = if okay {
                DROPEFFECT_COPY
            } else {
                DROPEFFECT_NONE
            };
            with_state(|s| s.hover = okay);
            let _ = InvalidateRect(Some(HWND(self.hwnd as _)), None, false);
            Ok(())
        }
    }
    fn DragOver(
        &self,
        _: MODIFIERKEYS_FLAGS,
        _: &POINTL,
        effect: *mut DROPEFFECT,
    ) -> windows::core::Result<()> {
        unsafe {
            *effect = if self.supported.load(Ordering::Relaxed)
                && (*effect & DROPEFFECT_COPY) != DROPEFFECT_NONE
            {
                DROPEFFECT_COPY
            } else {
                DROPEFFECT_NONE
            };
        }
        Ok(())
    }
    fn DragLeave(&self) -> windows::core::Result<()> {
        unsafe {
            with_state(|s| s.hover = false);
            let _ = InvalidateRect(Some(HWND(self.hwnd as _)), None, false);
        }
        Ok(())
    }
    fn Drop(
        &self,
        data: Ref<IDataObject>,
        _: MODIFIERKEYS_FLAGS,
        _: &POINTL,
        effect: *mut DROPEFFECT,
    ) -> windows::core::Result<()> {
        unsafe {
            let hwnd = HWND(self.hwnd as _);
            let allowed = (*effect & DROPEFFECT_COPY) != DROPEFFECT_NONE;
            *effect = DROPEFFECT_NONE;
            let result = if allowed {
                data.as_ref()
                    .ok_or("No file data.".into())
                    .and_then(|d| paths(d))
            } else {
                Err("Source did not allow copying.".into())
            };
            with_state(|s| {
                s.hover = false;
                match result {
                    Ok(files) => {
                        hold(hwnd, s, files);
                        *effect = DROPEFFECT_COPY;
                    }
                    Err(message) => {
                        if let Ok(mut v) = s.control.status.lock() {
                            v.message = message;
                        }
                    }
                }
                publish(s);
            });
            let _ = InvalidateRect(Some(hwnd), None, false);
            Ok(())
        }
    }
}
#[implement(IDropSource)]
struct Source {
    control: Arc<Control>,
}
impl IDropSource_Impl for Source_Impl {
    fn QueryContinueDrag(&self, escape: BOOL, keys: MODIFIERKEYS_FLAGS) -> HRESULT {
        if escape.as_bool() || self.control.stopping.load(Ordering::Relaxed) {
            DRAGDROP_S_CANCEL
        } else if keys.0 & MK_LBUTTON.0 == 0 {
            DRAGDROP_S_DROP
        } else {
            HRESULT(0)
        }
    }
    fn GiveFeedback(&self, _: DROPEFFECT) -> HRESULT {
        DRAGDROP_S_USEDEFAULTCURSORS
    }
}
unsafe fn drag_out(hwnd: HWND) {
    expire(hwnd);
    // Clone before entering OLE's nested message loop; expiry never invalidates a transfer.
    let Some((files, control, deadline)) =
        with_state(|s| (s.files.clone(), s.control.clone(), s.deadline))
    else {
        return;
    };
    let result = (|| -> Result<bool, String> {
        let data = shell_data(files)?;
        let source: IDropSource = Source { control }.into();
        let mut effect = DROPEFFECT_NONE;
        let hr = DoDragDrop(&data, &source, DROPEFFECT_COPY, &mut effect);
        if hr == DRAGDROP_S_CANCEL {
            return Ok(false);
        }
        hr.ok().map_err(err)?;
        Ok(hr == DRAGDROP_S_DROP && effect == DROPEFFECT_COPY)
    })();
    with_state(|s| {
        if matches!(result, Ok(true)) {
            finish_handoff(hwnd, s, deadline);
        }
        if let Ok(mut v) = s.control.status.lock() {
            v.message = match result {
                Ok(true) => "Copy handoff accepted.".into(),
                Ok(false) => "Drag cancelled. Files remain until expiry.".into(),
                Err(e) => e,
            };
        }
    });
    expire(hwnd);
}
unsafe fn finish_handoff(hwnd: HWND, state: &mut State, deadline: Option<Instant>) {
    // OLE can dispatch messages during a handoff. Preserve any newer batch.
    if state.deadline == deadline {
        state.files.clear();
        state.deadline = None;
        let _ = KillTimer(Some(hwnd), 1);
        publish(state);
        let _ = InvalidateRect(Some(hwnd), None, false);
    }
}
unsafe fn shell_data(files: Vec<PathBuf>) -> Result<IDataObject, String> {
    let files = validated(files)?;
    let mut ids = Vec::new();
    for path in &files {
        use std::os::windows::ffi::OsStrExt;
        let text: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
        let id = ILCreateFromPathW(PCWSTR(text.as_ptr()));
        if id.is_null() {
            for p in ids {
                CoTaskMemFree(Some(p as _));
            }
            return Err("Windows could not prepare this file for transfer.".into());
        }
        ids.push(id);
    }
    let raw: Vec<_> = ids.iter().map(|p| *p as *const _).collect();
    let array = SHCreateShellItemArrayFromIDLists(&raw);
    for id in ids {
        CoTaskMemFree(Some(id as _));
    }
    array
        .map_err(err)?
        .BindToHandler(None::<&IBindCtx>, &BHID_DataObject)
        .map_err(err)
}
unsafe fn copy_held(hwnd: HWND) -> Result<(), String> {
    expire(hwnd);
    let (files, deadline) = with_state(|s| (s.files.clone(), s.deadline)).unwrap_or_default();
    let result = (|| -> Result<(), String> {
        let data = shell_data(files)?;
        prefer_copy(&data)?;
        OleSetClipboard(&data).map_err(err)?;
        OleFlushClipboard().map_err(err)?;
        Ok(())
    })();
    with_state(|s| {
        if result.is_ok() {
            finish_handoff(hwnd, s, deadline);
        }
        if let Ok(mut status) = s.control.status.lock() {
            status.message=match &result{Ok(())=>"Copied file references. Paste in a folder or compatible app. Clipboard is now owned by your copy action.".into(),Err(e)=>e.clone()};
        }
    });
    result
}
unsafe fn prefer_copy(data: &IDataObject) -> Result<(), String> {
    let cf = RegisterClipboardFormatW(w!("Preferred DropEffect"));
    if cf == 0 {
        return Err("Windows could not register copy metadata.".into());
    }
    let global = GlobalAlloc(GMEM_MOVEABLE, 4).map_err(err)?;
    let ptr = GlobalLock(global);
    if ptr.is_null() {
        let _ = GlobalFree(Some(global));
        return Err("Could not allocate copy metadata.".into());
    }
    (ptr as *mut u32).write(DROPEFFECT_COPY.0);
    let _ = GlobalUnlock(global);
    let f = FORMATETC {
        cfFormat: cf as u16,
        ..format()
    };
    let medium = STGMEDIUM {
        tymed: TYMED_HGLOBAL.0 as u32,
        u: STGMEDIUM_0 { hGlobal: global },
        pUnkForRelease: std::mem::ManuallyDrop::new(None),
    };
    if let Err(e) = data.SetData(&f, &medium, true) {
        let _ = GlobalFree(Some(global));
        return Err(err(e));
    }
    Ok(())
}
unsafe fn clamp(hwnd: HWND, reset: bool) {
    let mut rect = RECT::default();
    let _ = GetWindowRect(hwnd, &mut rect);
    let point = if reset {
        POINT { x: 0, y: 0 }
    } else {
        POINT {
            x: rect.left,
            y: rect.top,
        }
    };
    let monitor = MonitorFromPoint(
        point,
        if reset {
            MONITOR_DEFAULTTOPRIMARY
        } else {
            MONITOR_DEFAULTTONEAREST
        },
    );
    let mut info = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    if GetMonitorInfoW(monitor, &mut info).as_bool() {
        let r = info.rcWork;
        let (width, height) = (rect.right - rect.left, rect.bottom - rect.top);
        let x = if reset {
            r.right - width - 24
        } else {
            rect.left.clamp(r.left, (r.right - width).max(r.left))
        };
        let y = if reset {
            r.bottom - height - 24
        } else {
            rect.top.clamp(r.top, (r.bottom - height).max(r.top))
        };
        let _ = SetWindowPos(
            hwnd,
            Some(HWND_TOPMOST),
            x,
            y,
            0,
            0,
            SWP_NOSIZE | SWP_NOACTIVATE,
        );
    }
}
unsafe fn persist_position(hwnd: HWND) {
    let mut r = RECT::default();
    if GetWindowRect(hwnd, &mut r).is_ok() {
        with_state(|s| save(&s.app, "pet_position", &format!("{},{}", r.left, r.top)));
    }
}
unsafe fn paint(hwnd: HWND) {
    let mut ps = PAINTSTRUCT::default();
    let dc = BeginPaint(hwnd, &mut ps);
    let dpi = GetDpiForWindow(hwnd).max(96) as i32;
    SetMapMode(dc, MM_ANISOTROPIC);
    let _ = SetWindowExtEx(dc, 96, 96, None);
    let _ = SetViewportExtEx(dc, dpi, dpi, None);
    let bg = CreateSolidBrush(COLORREF(0));
    FillRect(
        dc,
        &RECT {
            left: 0,
            top: 0,
            right: WIDTH,
            bottom: HEIGHT,
        },
        bg,
    );
    let _ = DeleteObject(bg.into());
    let (count, left, hover) =
        with_state(|s| (s.files.len(), seconds(s.deadline), s.hover)).unwrap_or_default();
    let pen = CreatePen(PS_SOLID, 2, COLORREF(0x002c5137));
    let oldpen = SelectObject(dc, pen.into());
    let brush = CreateSolidBrush(COLORREF(if hover { 0x00c3f5ce } else { 0x009be5b9 }));
    let oldbrush = SelectObject(dc, brush.into());
    let _ = RoundRect(dc, 27, 23, 101, 99, 30, 30);
    let _ = RoundRect(dc, 31, 87, 51, 106, 10, 10);
    let _ = RoundRect(dc, 78, 87, 98, 106, 10, 10);
    let _ = Ellipse(dc, 39, 15, 50, 32);
    let _ = Ellipse(dc, 78, 15, 89, 32);
    let eyes = CreateSolidBrush(COLORREF(0x001e3020));
    SelectObject(dc, eyes.into());
    let _ = Ellipse(dc, 46, 44, 54, 55);
    let _ = Ellipse(dc, 75, 44, 83, 55);
    let _ = MoveToEx(dc, 59, 62, None);
    let _ = LineTo(dc, 65, 65);
    let _ = LineTo(dc, 71, 62);
    let pouch = CreateSolidBrush(COLORREF(0x00eff4f5));
    SelectObject(dc, pouch.into());
    let _ = RoundRect(dc, 39, 72, 90, 94, 7, 7);
    SetBkMode(dc, TRANSPARENT);
    SetTextColor(dc, COLORREF(0x001e3020));
    let text = if count > 0 {
        format!("{}", count)
    } else {
        "+".into()
    };
    let mut text = wide(&text);
    let text_len = text.len() - 1;
    DrawTextW(
        dc,
        &mut text[..text_len],
        &mut RECT {
            left: 39,
            top: 73,
            right: 90,
            bottom: 94,
        },
        DT_CENTER | DT_SINGLELINE | DT_VCENTER,
    );
    SelectObject(dc, oldbrush);
    SelectObject(dc, oldpen);
    let _ = DeleteObject(brush.into());
    let _ = DeleteObject(eyes.into());
    let _ = DeleteObject(pouch.into());
    let _ = DeleteObject(pen.into());
    if count == 0 {
        let _ = EndPaint(hwnd, &ps);
        return;
    }
    // Opaque dark caption remains readable on either light or dark desktops.
    let caption = CreateSolidBrush(COLORREF(0x001a1c1c));
    let previous = SelectObject(dc, caption.into());
    let _ = RoundRect(dc, 9, 111, 119, 132, 8, 8);
    SelectObject(dc, previous);
    let _ = DeleteObject(caption.into());
    SetTextColor(dc, COLORREF(0x00eff4f5));
    let label = format!("{count} held · {left}s");
    let mut text = wide(&label);
    let len = text.len() - 1;
    DrawTextW(
        dc,
        &mut text[..len],
        &mut RECT {
            left: 9,
            top: 111,
            right: 119,
            bottom: 132,
        },
        DT_CENTER | DT_SINGLELINE | DT_VCENTER,
    );
    let _ = EndPaint(hwnd, &ps);
}
unsafe extern "system" fn proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        WM_PAINT => {
            paint(hwnd);
            LRESULT(0)
        }
        WM_ERASEBKGND => LRESULT(1),
        WM_TIMER => {
            expire(hwnd);
            LRESULT(0)
        }
        HOLD => {
            with_state(|s| {
                let files = s.control.pending.lock().ok().and_then(|mut p| p.take());
                if let Some(files) = files {
                    hold(hwnd, s, files);
                }
            });
            let _ = InvalidateRect(Some(hwnd), None, false);
            LRESULT(0)
        }
        COPY => {
            let result = copy_held(hwnd);
            with_state(|s| {
                if let Ok(mut reply) = s.control.copy_reply.lock() {
                    if let Some(tx) = reply.take() {
                        let _ = tx.send(result);
                    }
                }
            });
            LRESULT(0)
        }
        COMMANDS => {
            if let Some(app) = with_state(|s| s.app.clone()) {
                crate::bar::toggle(&app, hwnd);
            }
            LRESULT(0)
        }
        CLOSE_COMMANDS => {
            crate::bar::dismiss();
            LRESULT(0)
        }
        SHOW | HIDE => {
            let visible = msg == SHOW;
            let _ = ShowWindow(hwnd, if visible { SW_SHOWNOACTIVATE } else { SW_HIDE });
            if visible {
                clamp(hwnd, false);
            }
            with_state(|s| {
                save(
                    &s.app,
                    "pet_visible",
                    if visible { "true" } else { "false" },
                );
                if let Ok(mut v) = s.control.status.lock() {
                    v.visible = visible;
                }
            });
            LRESULT(0)
        }
        RESET => {
            clamp(hwnd, true);
            persist_position(hwnd);
            LRESULT(0)
        }
        WM_DISPLAYCHANGE => {
            clamp(hwnd, false);
            LRESULT(0)
        }
        WM_DPICHANGED => {
            let r = *(lp.0 as *const RECT);
            let _ = SetWindowPos(
                hwnd,
                None,
                r.left,
                r.top,
                r.right - r.left,
                r.bottom - r.top,
                SWP_NOZORDER | SWP_NOACTIVATE,
            );
            clamp(hwnd, false);
            LRESULT(0)
        }
        WM_LBUTTONDOWN => {
            crate::bar::dismiss();
            let mut p = POINT::default();
            let mut r = RECT::default();
            let _ = GetCursorPos(&mut p);
            let _ = GetWindowRect(hwnd, &mut r);
            with_state(|s| s.press = Some((p, r, false)));
            SetCapture(hwnd);
            LRESULT(0)
        }
        WM_MOUSEMOVE => {
            let mut p = POINT::default();
            let _ = GetCursorPos(&mut p);
            let drag = with_state(|s| {
                if let Some((start, r, moved)) = s.press {
                    let dx = p.x - start.x;
                    let dy = p.y - start.y;
                    if moved || dx.abs() > 5 || dy.abs() > 5 {
                        if !s.files.is_empty() && GetAsyncKeyState(VK_MENU.0 as i32) >= 0 {
                            s.press = None;
                            return true;
                        }
                        s.press = Some((start, r, true));
                        let _ = SetWindowPos(
                            hwnd,
                            None,
                            r.left + dx,
                            r.top + dy,
                            0,
                            0,
                            SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
                        );
                    }
                }
                false
            })
            .unwrap_or(false);
            if drag {
                let _ = ReleaseCapture();
                drag_out(hwnd);
            }
            LRESULT(0)
        }
        WM_LBUTTONUP => {
            let press = with_state(|s| s.press.take()).flatten();
            let _ = ReleaseCapture();
            if let Some((_, _, moved)) = press {
                if moved {
                    clamp(hwnd, false);
                    persist_position(hwnd);
                } else {
                    if let Some(app) = with_state(|s| s.app.clone()) {
                        crate::bar::toggle(&app, hwnd);
                    }
                }
            }
            LRESULT(0)
        }
        WM_CAPTURECHANGED => {
            with_state(|s| s.press = None);
            LRESULT(0)
        }
        WM_CONTEXTMENU => {
            expire(hwnd);
            let mut p = POINT::default();
            let _ = GetCursorPos(&mut p);
            if let Ok(menu) = CreatePopupMenu() {
                let _ = AppendMenuW(menu, MF_STRING, 1, w!("Open tools (Ctrl+Alt+Space)"));
                let _ = AppendMenuW(
                    menu,
                    MF_STRING | MF_DISABLED,
                    0,
                    w!("Drop files; drag pouch to copy; Alt-drag to move"),
                );
                let _ = AppendMenuW(menu, MF_STRING, 2, w!("Clear held references"));
                let _ = AppendMenuW(menu, MF_STRING, 6, w!("Copy held files (then paste)"));
                let _ = AppendMenuW(menu, MF_STRING, 3, w!("Hide pet"));
                let _ = AppendMenuW(menu, MF_STRING, 4, w!("Reset position"));
                let _ = AppendMenuW(menu, MF_STRING, 5, w!("Quit (reminders stop)"));
                let _ = SetForegroundWindow(hwnd);
                let action = TrackPopupMenu(
                    menu,
                    TPM_RETURNCMD | TPM_RIGHTBUTTON,
                    p.x,
                    p.y,
                    None,
                    hwnd,
                    None,
                )
                .0;
                let _ = DestroyMenu(menu);
                match action {
                    6 => {
                        let _ = copy_held(hwnd);
                    }
                    1 => {
                        with_state(|s| post_main(&s.app, false));
                    }
                    2 => {
                        with_state(|s| {
                            s.files.clear();
                            s.deadline = None;
                            publish(s);
                        });
                        let _ = KillTimer(Some(hwnd), 1);
                        let _ = InvalidateRect(Some(hwnd), None, false);
                    }
                    3 => {
                        let _ = PostMessageW(Some(hwnd), HIDE, WPARAM(0), LPARAM(0));
                    }
                    4 => {
                        let _ = PostMessageW(Some(hwnd), RESET, WPARAM(0), LPARAM(0));
                    }
                    5 => {
                        with_state(|s| post_main(&s.app, true));
                    }
                    _ => {}
                }
            }
            LRESULT(0)
        }
        WM_KEYDOWN if wp.0 == VK_RETURN.0 as usize || wp.0 == VK_SPACE.0 as usize => {
            if let Some(app) = with_state(|s| s.app.clone()) {
                crate::bar::toggle(&app, hwnd);
            }
            LRESULT(0)
        }
        WM_CLOSE => {
            let _ = RevokeDragDrop(hwnd);
            let _ = DestroyWindow(hwnd);
            LRESULT(0)
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wp, lp),
    }
}
pub fn start(app: tauri::AppHandle, visible: bool) -> Result<Arc<Control>, String> {
    let control = Arc::new(Control {
        hwnd: AtomicIsize::new(0),
        stopping: AtomicBool::new(false),
        status: Mutex::new(Status::default()),
        pending: Mutex::new(None),
        copy_reply: Mutex::new(None),
    });
    let output = control.clone();
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || unsafe {
        if let Err(e) = OleInitialize(None) {
            let _ = tx.send(Err(e.to_string()));
            return;
        }
        let result = (|| -> windows::core::Result<HWND> {
            let instance = GetModuleHandleW(None)?;
            let class = w!("DesktopBuddyNativePet");
            let wc = WNDCLASSW {
                lpfnWndProc: Some(proc),
                hInstance: instance.into(),
                lpszClassName: class,
                hCursor: LoadCursorW(None, IDC_HAND)?,
                ..Default::default()
            };
            if RegisterClassW(&wc) == 0 {
                return Err(windows::core::Error::from_thread());
            }
            STATE.with(|s| {
                *s.borrow_mut() = Some(State {
                    app: app.clone(),
                    control: output.clone(),
                    files: vec![],
                    deadline: None,
                    press: None,
                    hover: false,
                })
            });
            let hwnd = CreateWindowExW(
                WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_TOPMOST | WS_EX_NOACTIVATE,
                class,
                w!("Desktop Buddy pet — drop files to hold for 120 seconds"),
                WS_POPUP,
                0,
                0,
                WIDTH,
                HEIGHT,
                None,
                None,
                Some(instance.into()),
                None,
            )?;
            SetLayeredWindowAttributes(hwnd, COLORREF(0), 255, LWA_COLORKEY)?;
            let dpi = GetDpiForWindow(hwnd).max(96) as i32;
            let _ = SetWindowPos(
                hwnd,
                None,
                0,
                0,
                WIDTH * dpi / 96,
                HEIGHT * dpi / 96,
                SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE,
            );
            if let Some(v) = setting(&app, "pet_position") {
                if let Some((x, y)) = v.split_once(',') {
                    if let (Ok(x), Ok(y)) = (x.parse::<i32>(), y.parse::<i32>()) {
                        let _ = SetWindowPos(
                            hwnd,
                            None,
                            x,
                            y,
                            0,
                            0,
                            SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
                        );
                    }
                }
            } else {
                clamp(hwnd, true);
            }
            clamp(hwnd, false);
            let target: IDropTarget = Target {
                hwnd: hwnd.0 as isize,
                supported: AtomicBool::new(false),
            }
            .into();
            RegisterDragDrop(hwnd, &target)?;
            let enabled = visible && setting(&app, "pet_visible").as_deref() != Some("false");
            let _ = ShowWindow(hwnd, if enabled { SW_SHOWNOACTIVATE } else { SW_HIDE });
            output.hwnd.store(hwnd.0 as isize, Ordering::Relaxed);
            output.status.lock().unwrap().visible = enabled;
            Ok(hwnd)
        })();
        match result {
            Ok(hwnd) => {
                if output.stopping.load(Ordering::Relaxed) || tx.send(Ok(())).is_err() {
                    let _ = RevokeDragDrop(hwnd);
                    let _ = DestroyWindow(hwnd);
                } else {
                    let mut msg = MSG::default();
                    while GetMessageW(&mut msg, None, 0, 0).0 > 0 {
                        let _ = TranslateMessage(&msg);
                        DispatchMessageW(&msg);
                    }
                }
            }
            Err(e) => {
                let _ = tx.send(Err(e.to_string()));
            }
        }
        output.hwnd.store(0, Ordering::Relaxed);
        STATE.with(|s| *s.borrow_mut() = None);
        OleUninitialize();
    });
    match rx.recv_timeout(Duration::from_secs(8)) {
        Ok(result) => result?,
        Err(e) => {
            control.stopping.store(true, Ordering::Relaxed);
            unsafe {
                let hwnd = control.hwnd.load(Ordering::Relaxed);
                if hwnd != 0 {
                    let _ = PostMessageW(Some(HWND(hwnd as _)), WM_CLOSE, WPARAM(0), LPARAM(0));
                }
            }
            return Err(e.to_string());
        }
    }
    Ok(control)
}
fn message(app: &tauri::AppHandle, msg: u32) -> Result<(), String> {
    let c = app
        .try_state::<Arc<Control>>()
        .ok_or("Pet is unavailable; use the tray tools.")?;
    let hwnd = c.hwnd.load(Ordering::Relaxed);
    if hwnd == 0 {
        return Err("Pet window is unavailable.".into());
    }
    unsafe { PostMessageW(Some(HWND(hwnd as _)), msg, WPARAM(0), LPARAM(0)).map_err(err) }
}
#[tauri::command]
pub fn pet_show(app: tauri::AppHandle) -> Result<(), String> {
    message(&app, SHOW)
}
#[tauri::command]
pub fn pet_hide(app: tauri::AppHandle) -> Result<(), String> {
    message(&app, HIDE)
}
#[tauri::command]
pub fn pet_commands(app: tauri::AppHandle) -> Result<(), String> {
    message(&app, COMMANDS)
}
pub fn close_commands(app: tauri::AppHandle) -> Result<(), String> {
    message(&app, CLOSE_COMMANDS)
}
pub fn bar_visibility(app: &tauri::AppHandle, visible: bool) {
    if let Some(c) = app.try_state::<Arc<Control>>() {
        if let Ok(mut status) = c.status.lock() {
            status.command_bar_visible = visible;
        }
    }
}
#[tauri::command]
pub fn pet_hold(app: tauri::AppHandle, paths: Vec<String>) -> Result<(), String> {
    let files = validated(paths.into_iter().map(PathBuf::from).collect())?;
    let c = app
        .try_state::<Arc<Control>>()
        .ok_or("Pet is unavailable")?;
    *c.pending.lock().map_err(err)? = Some(files);
    message(&app, HOLD)
}
#[tauri::command]
pub async fn pet_copy(app: tauri::AppHandle) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let c = app
            .try_state::<Arc<Control>>()
            .ok_or("Pet is unavailable")?;
        let (tx, rx) = std::sync::mpsc::sync_channel(1);
        {
            let mut reply = c.copy_reply.lock().map_err(err)?;
            if reply.is_some() {
                return Err("A copy action is already in progress.".into());
            }
            *reply = Some(tx);
        }
        if let Err(e) = message(&app, COPY) {
            *c.copy_reply.lock().map_err(err)? = None;
            return Err(e);
        }
        rx.recv_timeout(Duration::from_secs(5)).map_err(err)?
    })
    .await
    .map_err(err)?
}
#[tauri::command]
pub fn pet_status(app: tauri::AppHandle) -> Result<Status, String> {
    let c = app
        .try_state::<Arc<Control>>()
        .ok_or("Pet is unavailable")?;
    let result = c.status.lock().map_err(err)?.clone();
    Ok(result)
}
pub fn stop(app: &tauri::AppHandle) {
    if let Some(c) = app.try_state::<Arc<Control>>() {
        c.stopping.store(true, Ordering::Relaxed);
        unsafe {
            let _ = PostMessageW(
                Some(HWND(c.hwnd.load(Ordering::Relaxed) as _)),
                WM_CLOSE,
                WPARAM(0),
                LPARAM(0),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shell_file_data_roundtrips_multiple_unicode_paths_without_touching_originals() {
        unsafe {
            OleInitialize(None).unwrap();
            let root = std::env::temp_dir().join(format!("buddy-shell-{}", super::super::new_id()));
            std::fs::create_dir_all(&root).unwrap();
            let files = vec![root.join("handoff.txt"), root.join("猫.txt")];
            for p in &files {
                std::fs::write(p, b"unchanged source").unwrap();
            }
            let data = shell_data(files.clone()).unwrap();
            prefer_copy(&data).unwrap();
            let f = FORMATETC {
                cfFormat: RegisterClipboardFormatW(w!("Preferred DropEffect")) as u16,
                ..format()
            };
            let mut medium = data.GetData(&f).unwrap();
            let ptr = GlobalLock(medium.u.hGlobal);
            assert!(!ptr.is_null());
            assert_eq!((ptr as *const u32).read(), DROPEFFECT_COPY.0);
            let _ = GlobalUnlock(medium.u.hGlobal);
            ReleaseStgMedium(&mut medium);
            assert_eq!(paths(&data).unwrap(), files);
            for p in &files {
                assert_eq!(std::fs::read(p).unwrap(), b"unchanged source");
            }
            drop(data);
            std::fs::remove_dir_all(root).unwrap();
            OleUninitialize();
        }
    }
    #[test]
    fn unsupported_file_batches_and_expired_deadlines_are_rejected() {
        assert!(validated(vec![]).is_err());
        assert!(validated(vec![PathBuf::from("relative.txt")]).is_err());
        assert!(validated(vec![PathBuf::from("C:/missing"); 21]).is_err());
        assert_eq!(seconds(Some(Instant::now() - Duration::from_secs(1))), 0);
        assert_eq!(seconds(None), 0);
    }
    #[test]
    fn native_drag_source_cancels_on_escape_or_shutdown_and_drops_only_after_release() {
        unsafe {
            let control = Arc::new(Control {
                hwnd: AtomicIsize::new(0),
                stopping: AtomicBool::new(false),
                status: Mutex::new(Status::default()),
                pending: Mutex::new(None),
                copy_reply: Mutex::new(None),
            });
            let source: IDropSource = Source {
                control: control.clone(),
            }
            .into();
            assert_eq!(
                source.QueryContinueDrag(true, MK_LBUTTON),
                DRAGDROP_S_CANCEL
            );
            assert_eq!(
                source.QueryContinueDrag(false, MODIFIERKEYS_FLAGS(0)),
                DRAGDROP_S_DROP
            );
            assert_eq!(source.QueryContinueDrag(false, MK_LBUTTON), HRESULT(0));
            control.stopping.store(true, Ordering::Relaxed);
            assert_eq!(
                source.QueryContinueDrag(false, MK_LBUTTON),
                DRAGDROP_S_CANCEL
            );
        }
    }
}
