//! Native, idle-static Windows companion and temporary copy-only file handoff.
use super::*;
use crate::pet_art::{self, AnimationPhase, AnimationSet, Appearance, Cue};
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
const APPEARANCE: u32 = WM_APP + 18;
const PREVIEW: u32 = WM_APP + 19;
const REFRESH_INDICATORS: u32 = WM_APP + 20;
const PLAY_CUE: u32 = WM_APP + 21;
const MOTION_TIMER: usize = 2;
const COUNTDOWN_TIMER: usize = 3;
pub const APPEARANCES: [Appearance; 3] = [Appearance::Calico, Appearance::Dog, Appearance::Panda];
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
    appearance: Appearance,
    indicators: Indicators,
    motion_set: usize,
    motion: Option<(AnimationPhase, usize, bool)>,
    motion_deadline: Option<Instant>,
}
thread_local! { static STATE: RefCell<Option<State>> = const { RefCell::new(None) }; }
fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}
fn with_state<T>(f: impl FnOnce(&mut State) -> T) -> Option<T> {
    STATE.with(|s| s.borrow_mut().as_mut().map(f))
}
pub fn appearance() -> Appearance {
    with_state(|state| state.appearance).unwrap_or_default()
}

pub fn set_appearance(app: &tauri::AppHandle, appearance: Appearance) {
    if let Some(control) = app.try_state::<Arc<Control>>() {
        let hwnd = HWND(control.hwnd.load(Ordering::Relaxed) as _);
        if !hwnd.0.is_null() {
            if let Some(index) = APPEARANCES.iter().position(|candidate| *candidate == appearance) {
                let _ = unsafe { PostMessageW(Some(hwnd), APPEARANCE, WPARAM(index), LPARAM(0)) };
            }
        }
    }
}
pub fn preview(app: &tauri::AppHandle, index: usize) {
    if let Some(control) = app.try_state::<Arc<Control>>() {
        let hwnd = HWND(control.hwnd.load(Ordering::Relaxed) as _);
        if !hwnd.0.is_null() && index < 5 {
            let _ = unsafe { PostMessageW(Some(hwnd), PREVIEW, WPARAM(index), LPARAM(0)) };
        }
    }
}

impl State {
    fn frames(&self, phase: AnimationPhase) -> &'static [pet_art::PoseKeyframe] {
        let set = &self.appearance.animation_sets()[self.motion_set];
        match phase {
            AnimationPhase::Pickup => set.pickup,
            AnimationPhase::Held => set.held,
            AnimationPhase::Release => set.release,
            AnimationPhase::Activity(cue) => cue.frames(),
        }
    }
}

unsafe fn advance_motion(hwnd: HWND, state: &mut State, next: Option<(AnimationPhase, usize, bool)>) {
    let _ = KillTimer(Some(hwnd), MOTION_TIMER);
    state.motion = None;
    state.motion_deadline = None;
    if let Some((phase, index, _)) = next.filter(|_| IsWindowVisible(hwnd).as_bool()) {
        let mut enabled = BOOL(1);
        let _ = SystemParametersInfoW(
            SPI_GETCLIENTAREAANIMATION, 0, Some((&mut enabled as *mut BOOL).cast()),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        );
        if enabled.as_bool() {
            if let Some(frame) = state.frames(phase).get(index) {
                let delay = u32::from(frame.duration_ms).max(16);
                if SetTimer(Some(hwnd), MOTION_TIMER, delay, None) != 0 {
                    state.motion = next;
                    state.motion_deadline = Some(Instant::now() + Duration::from_millis(delay.into()));
                }
            }
        }
    }
    let _ = InvalidateRect(Some(hwnd), None, false);
}

fn can_play_cue(motion: Option<(AnimationPhase, usize, bool)>) -> bool {
    motion.is_none_or(|(phase, _, preview)| !preview && matches!(phase, AnimationPhase::Activity(_)))
}

pub fn cue(app: &tauri::AppHandle, cue: Cue) {
    if let Some(control) = app.try_state::<Arc<Control>>() {
        let hwnd = HWND(control.hwnd.load(Ordering::Relaxed) as _);
        if !hwnd.0.is_null() {
            let _ = unsafe { PostMessageW(Some(hwnd), PLAY_CUE, WPARAM(cue as usize), LPARAM(0)) };
        }
    }
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
    advance_motion(hwnd, state, Some((AnimationPhase::Pickup, 0, false)));
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
fn motion_setting(app: &tauri::AppHandle, appearance: Appearance) -> usize {
    setting(app, &format!("pet_motion_{}", appearance.key()))
        .and_then(|value| value.parse().ok())
        .filter(|index| *index < 5)
        .unwrap_or(0)
}
unsafe fn expire(hwnd: HWND) {
    with_state(|s| {
        if seconds(s.deadline) == 0 && s.deadline.is_some() {
            s.files.clear();
            s.deadline = None;
            let _ = KillTimer(Some(hwnd), 1);
            advance_motion(hwnd, s, Some((AnimationPhase::Activity(Cue::FileExpired), 0, false)));
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
        advance_motion(hwnd, state, Some((AnimationPhase::Release, 0, false)));
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
struct PetTimer {
    id: String,
    clock: TimerClock,
}

enum TimerClock {
    Running(Instant),
    Paused(u64),
}

impl PetTimer {
    fn display(&self) -> Option<(u64, bool)> {
        match self.clock {
            TimerClock::Running(deadline) => {
                let left = seconds(Some(deadline));
                (left > 0).then_some((left, true))
            }
            TimerClock::Paused(left) => Some((left, false)),
        }
    }

    fn action(&self) -> &'static str {
        match self.clock {
            TimerClock::Running(_) => "pause",
            TimerClock::Paused(_) => "resume",
        }
    }
}

#[derive(Default)]
struct Indicators {
    timer: Option<PetTimer>,
    reminder: Option<ReminderMark>,
    todo_count: usize,
}

#[derive(Clone, Copy)]
enum ReminderMark {
    Scheduled,
    Due,
}

impl Indicators {
    fn shows_paw(&self) -> bool {
        self.todo_count > 0 || self.reminder.is_some()
    }

    fn visible_timer(&self) -> Option<&PetTimer> {
        self.timer.as_ref().filter(|timer| timer.display().is_some())
    }

    fn is_counting(&self) -> bool {
        self.visible_timer().is_some_and(|timer| timer.action() == "pause")
    }
}

fn read_indicators(app: &tauri::AppHandle) -> Indicators {
    let read = || -> Option<Indicators> {
        let core = app.try_state::<Shared>()?;
        let db = core.db.lock().ok()?;
        let mut query = db.prepare(
            "SELECT payload FROM items WHERE kind IN ('timer', 'reminder', 'attention', 'todo') ORDER BY created DESC",
        ).ok()?;
        let items = query.query_map([], |row| row.get::<_, String>(0)).ok()?;
        let mut indicators = Indicators::default();
        let mut paused = None;
        for item in items.filter_map(Result::ok).filter_map(|payload| serde_json::from_str::<Item>(&payload).ok()) {
            match item.kind.as_str() {
                "todo" if !item.done => indicators.todo_count += 1,
                "timer" if item.state == "running" && indicators.timer.is_none() => {
                    let left = remaining(&item, now(), ticks(), &core.run);
                    if left > 0 {
                        indicators.timer = Some(PetTimer {
                            id: item.id,
                            clock: TimerClock::Running(Instant::now() + Duration::from_millis(left as u64)),
                        });
                    }
                }
                "timer" if item.state == "paused" && paused.is_none() => {
                    paused = Some(PetTimer {
                        id: item.id,
                        clock: TimerClock::Paused((item.remaining_ms.max(0) as u64).div_ceil(1000)),
                    });
                }
                "reminder" if !item.done && indicators.reminder.is_none() => {
                    indicators.reminder = Some(ReminderMark::Scheduled);
                }
                "attention" if !item.done && item.source == "Reminder due" => {
                    indicators.reminder = Some(ReminderMark::Due);
                }
                _ => {}
            }
        }
        indicators.timer = indicators.timer.or(paused);
        Some(indicators)
    };
    read().unwrap_or_default()
}

unsafe fn update_countdown_tick(hwnd: HWND, indicators: &Indicators) {
    let _ = KillTimer(Some(hwnd), COUNTDOWN_TIMER);
    if IsWindowVisible(hwnd).as_bool() && indicators.is_counting() {
        let _ = SetTimer(Some(hwnd), COUNTDOWN_TIMER, 1000, None);
    }
}

fn countdown_label(seconds: u64) -> String {
    let (hours, minutes, seconds) = (seconds / 3600, seconds / 60 % 60, seconds % 60);
    if hours > 0 {
        format!("{hours}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes:02}:{seconds:02}")
    }
}

fn timer_bounds(has_files: bool) -> RECT {
    RECT { left: if has_files { 51 } else { 9 }, top: 111, right: 119, bottom: 132 }
}

const PAW_BOUNDS: RECT = RECT { left: 3, top: 3, right: 47, bottom: 31 };

unsafe fn paw_badge(dc: HDC, due: bool, palette: pet_art::Palette) {
    let surface = CreateSolidBrush(if due { palette.accent } else { palette.surface });
    let pads = CreateSolidBrush(if due { palette.surface } else { palette.accent });
    let previous_brush = SelectObject(dc, surface.into());
    let previous_pen = SelectObject(dc, GetStockObject(NULL_PEN));
    let r = PAW_BOUNDS;
    let _ = Ellipse(dc, r.left, r.top, 31, r.bottom);
    SelectObject(dc, pads.into());
    for (left, top) in [(8, 13), (12, 9), (18, 9), (22, 13)] {
        let _ = Ellipse(dc, left, top, left + 5, top + 6);
    }
    let _ = Ellipse(dc, 11, 17, 23, 25);
    SelectObject(dc, previous_pen);
    SelectObject(dc, previous_brush);
    let _ = DeleteObject(surface.into());
    let _ = DeleteObject(pads.into());
}

unsafe fn show_todos(app: &tauri::AppHandle, hwnd: HWND) {
    const LIMIT: usize = 8;
    let todos = app.try_state::<Shared>()
        .and_then(|core| core.db.lock().ok().and_then(|db| rows(&db).ok()))
        .map(|items| items.into_iter().filter(|item| item.kind == "todo" && !item.done).collect::<Vec<_>>());
    let Ok(menu) = CreatePopupMenu() else {
        return;
    };
    let _ = AppendMenuW(menu, MF_STRING | MF_DISABLED, 0, w!("To-dos · choose an action"));
    let _ = AppendMenuW(menu, MF_SEPARATOR, 0, None);
    let mut actions = Vec::new();
    match todos {
        None => { let _ = AppendMenuW(menu, MF_STRING | MF_DISABLED, 0, w!("Could not load to-dos")); }
        Some(todos) if todos.is_empty() => {
            let _ = AppendMenuW(menu, MF_STRING | MF_DISABLED, 0, w!("No open to-dos"));
        }
        Some(todos) => {
            for item in todos.iter().take(LIMIT) {
                let title = item.title.split_whitespace().collect::<Vec<_>>().join(" ");
                let mut chars = title.chars();
                let mut title: String = chars.by_ref().take(56).collect();
                if chars.next().is_some() {
                    title.push('…');
                }
                let title = title.replace('&', "&&");
                let label: Vec<_> = title.encode_utf16().chain(Some(0)).collect();
                if let Ok(submenu) = CreatePopupMenu() {
                    let command = actions.len() + 100;
                    if AppendMenuW(submenu, MF_STRING, command, w!("Mark complete")).is_ok()
                        && AppendMenuW(menu, MF_POPUP, submenu.0 as usize, PCWSTR(label.as_ptr())).is_ok() {
                        actions.push(item.id.clone());
                    } else {
                        let _ = DestroyMenu(submenu);
                    }
                }
            }
            if todos.len() > LIMIT {
                let label: Vec<_> = format!("{} more in Tasks", todos.len() - LIMIT).encode_utf16().chain(Some(0)).collect();
                let _ = AppendMenuW(menu, MF_STRING | MF_DISABLED, 0, PCWSTR(label.as_ptr()));
            }
        }
    }
    let _ = AppendMenuW(menu, MF_SEPARATOR, 0, None);
    let _ = AppendMenuW(menu, MF_STRING, 2, w!("Open tools"));
    let _ = AppendMenuW(menu, MF_STRING, 1, w!("Close"));
    let mut rect = RECT::default();
    let _ = GetWindowRect(hwnd, &mut rect);
    let dpi = GetDpiForWindow(hwnd).max(96) as i32;
    crate::bar::dismiss();
    let action = crate::menu_theme::track(menu, hwnd,
        rect.left + PAW_BOUNDS.left * dpi / 96, rect.top + PAW_BOUNDS.bottom * dpi / 96,
        appearance().palette()) as usize;
    if action == 2 {
        post_main(app, false);
    } else if let Some(id) = action.checked_sub(100).and_then(|index| actions.get(index)) {
        finish_todo(app, id);
    }
}

fn finish_todo(app: &tauri::AppHandle, id: &str) {
    if let Some(core) = app.try_state::<Shared>() {
        let result = core.db.lock().map_err(err).and_then(|db| complete_todo(&db, id));
        match result {
            Ok(true) => {
                changed(app, &core);
                cue(app, Cue::TodoDone);
            }
            Ok(false) => {}
            Err(error) => {
                if let Ok(mut warning) = core.warning.lock() {
                    *warning = Some(format!("Could not complete to-do: {error}"));
                }
                let _ = app.emit("data-changed", ());
                post_main(app, false);
            }
        }
    }
}
unsafe fn badge(dc: HDC, mut rect: RECT, label: &str, surface: COLORREF, ink: COLORREF) {
    let fill = CreateSolidBrush(surface);
    let old_brush = SelectObject(dc, fill.into());
    let old_pen = SelectObject(dc, GetStockObject(NULL_PEN));
    let _ = RoundRect(dc, rect.left, rect.top, rect.right, rect.bottom, 8, 8);
    SelectObject(dc, old_pen);
    SelectObject(dc, old_brush);
    let _ = DeleteObject(fill.into());
    SetBkMode(dc, TRANSPARENT);
    SetTextColor(dc, ink);
    let mut text = wide(label);
    let len = text.len() - 1;
    DrawTextW(dc, &mut text[..len], &mut rect, DT_CENTER | DT_SINGLELINE | DT_VCENTER);
}
fn change_pet_timer(app: tauri::AppHandle, (id, action): (String, &'static str)) {
    if let Some(core) = app.try_state::<Shared>() {
        let _ = timer_action(app.clone(), core, id, action.into());
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
    let (count, left, hover, appearance, pose, file_motion, timer, paw, awake, cue) = with_state(|state| {
        let count = state.files.len();
        let (pose, file_motion) = match state.motion {
            Some((phase, index, _)) => (
                state.frames(phase).get(index).copied()
                    .unwrap_or_else(|| AnimationSet::reduced_motion_pose(phase)),
                match phase {
                    AnimationPhase::Pickup => Some((0, 120_i32.saturating_sub(index.min(4) as i32 * 30))),
                    AnimationPhase::Release if index < 2 => Some((index as i32 * 180, -(index as i32) * 120)),
                    AnimationPhase::Held => Some((0, 0)),
                    _ => (count > 0).then_some((0, 0)),
                },
            ),
            None if count > 0 => (AnimationSet::reduced_motion_pose(AnimationPhase::Held), Some((0, 0))),
            None => (AnimationSet::neutral_still_pose(), None),
        };
        let cue = state.motion.and_then(|(phase, _, _)| match phase {
            AnimationPhase::Activity(cue) => Some(cue),
            _ => None,
        });
        let bar_open = state.control.status.lock().is_ok_and(|status| status.command_bar_visible);
        let timer = state.indicators.visible_timer().and_then(PetTimer::display);
        let awake = file_motion.is_some() || bar_open || timer.is_some_and(|(_, running)| running)
            || cue.is_some_and(|cue| !matches!(cue, Cue::Settle | Cue::FocusPause | Cue::FileExpired));
        let paw = state.indicators.shows_paw().then_some((
            matches!(state.indicators.reminder, Some(ReminderMark::Due)), state.indicators.todo_count));
        (count, seconds(state.deadline), state.hover, state.appearance, Some(pose), file_motion, timer, paw, awake, cue)
    }).unwrap_or_default();
    let mapped = SaveDC(dc);
    SetMapMode(dc, MM_TEXT);
    pet_art::draw_pose(dc, appearance, hover, dpi, pose.unwrap_or_else(AnimationSet::neutral_still_pose), file_motion, awake, cue);
    let _ = RestoreDC(dc, mapped);
    let palette = appearance.palette();
    let font = if count > 0 || timer.is_some() || paw.is_some() {
        CreateFontW(-13, 0, 0, 0, 700, 0, 0, 0, DEFAULT_CHARSET, OUT_DEFAULT_PRECIS,
            CLIP_DEFAULT_PRECIS, ANTIALIASED_QUALITY, DEFAULT_PITCH.0 as u32, w!("Trebuchet MS"))
    } else {
        HFONT::default()
    };
    let previous_font = (!font.0.is_null()).then(|| SelectObject(dc, font.into()));
    if let Some((due, pending)) = paw {
        paw_badge(dc, due, palette);
        if pending > 0 {
            let label = if pending > 9 { "9+".into() } else { pending.to_string() };
            badge(dc, RECT { left: 27, top: 3, right: 47, bottom: 20 }, &label, palette.surface, palette.ink);
        }
    }
    if count > 0 {
        let compact = timer.is_some();
        let label = if compact { format!("{left}s") } else { format!("{count} held / {left}s") };
        badge(dc, RECT { left: 9, top: 111, right: if compact { 48 } else { 119 }, bottom: 132 },
            &label, palette.surface, palette.ink);
    }
    if let Some((left, running)) = timer {
        let time = countdown_label(left);
        let label = if count > 0 { time } else if running { format!("Ⅱ  {time}") } else { format!("▶  {time}") };
        let (surface, ink) = if running { (palette.accent, palette.surface) } else { (palette.surface, palette.accent) };
        badge(dc, timer_bounds(count > 0), &label, surface, ink);
    }
    if let Some(previous_font) = previous_font {
        SelectObject(dc, previous_font);
        let _ = DeleteObject(font.into());
    }
    let _ = EndPaint(hwnd, &ps);
}
unsafe extern "system" fn proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if let Some(result) = crate::menu_theme::handle_message(msg, wp, lp) {
        return result;
    }
    match msg {
        WM_PAINT => {
            paint(hwnd);
            LRESULT(0)
        }
        WM_ERASEBKGND => LRESULT(1),
        PLAY_CUE => {
            if let Some(&cue) = Cue::ALL.get(wp.0) {
                with_state(|state| {
                    if can_play_cue(state.motion) {
                        advance_motion(hwnd, state, Some((AnimationPhase::Activity(cue), 0, false)));
                    } else {
                        let _ = InvalidateRect(Some(hwnd), None, false);
                    }
                });
            }
            LRESULT(0)
        }
        WM_TIMER if wp.0 == MOTION_TIMER => {
            with_state(|state| {
                if state.motion_deadline.is_none_or(|deadline| Instant::now() < deadline) {
                    return;
                }
                let next = state.motion.and_then(|(phase, index, preview)| {
                    if index.saturating_add(1) < state.frames(phase).len() {
                        return Some((phase, index + 1, preview));
                    }
                    match phase {
                        AnimationPhase::Pickup => Some((AnimationPhase::Held, 0, preview)),
                        AnimationPhase::Held if preview => Some((AnimationPhase::Release, 0, true)),
                        _ => None,
                    }
                });
                advance_motion(hwnd, state, next);
            });
            LRESULT(0)
        }
        WM_TIMER if wp.0 == COUNTDOWN_TIMER => {
            with_state(|s| {
                if !s.indicators.is_counting() {
                    let _ = KillTimer(Some(hwnd), COUNTDOWN_TIMER);
                }
            });
            let _ = InvalidateRect(Some(hwnd), None, false);
            LRESULT(0)
        }
        WM_TIMER if wp.0 == 1 => {
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
        APPEARANCE => {
            let appearance = APPEARANCES.get(wp.0).copied().unwrap_or_default();
            with_state(|state| {
                state.appearance = appearance;
                save(&state.app, "pet_appearance", appearance.key());
                state.motion_set = motion_setting(&state.app, appearance);
                advance_motion(hwnd, state, None);
            });
            let _ = InvalidateRect(Some(hwnd), None, false);
            crate::bar::repaint_theme();
            LRESULT(0)
        }
        PREVIEW => {
            with_state(|state| {
                if wp.0 < 5 && state.files.is_empty() {
                    state.motion_set = wp.0;
                    save(&state.app, &format!("pet_motion_{}", state.appearance.key()), &wp.0.to_string());
                    let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
                    let _ = PostMessageW(Some(hwnd), SHOW, WPARAM(0), LPARAM(0));
                    advance_motion(hwnd, state, Some((AnimationPhase::Pickup, 0, true)));
                }
            });
            LRESULT(0)
        }
        REFRESH_INDICATORS => {
            with_state(|s| {
                s.indicators = read_indicators(&s.app);
                update_countdown_tick(hwnd, &s.indicators);
            });
            let _ = InvalidateRect(Some(hwnd), None, false);
            crate::bar::repaint_theme();
            LRESULT(0)
        }
        SHOW | HIDE => {
            let visible = msg == SHOW;
            let _ = ShowWindow(hwnd, if visible { SW_SHOWNOACTIVATE } else { SW_HIDE });
            if visible {
                clamp(hwnd, false);
            }
            with_state(|s| {
                update_countdown_tick(hwnd, &s.indicators);
                if !visible {
                    advance_motion(hwnd, s, None);
                }
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
                    crate::bar::dismiss();
                    clamp(hwnd, false);
                    persist_position(hwnd);
                } else {
                    let dpi = GetDpiForWindow(hwnd).max(96) as i32;
                    let x = lp.0 as i16 as i32 * 96 / dpi;
                    let y = (lp.0 >> 16) as i16 as i32 * 96 / dpi;
                    if let Some((app, timer, paw)) = with_state(|s| {
                        let r = timer_bounds(!s.files.is_empty());
                        let hit = x >= r.left && x < r.right && y >= r.top && y < r.bottom;
                        let paw = PAW_BOUNDS;
                        let hits_paw = s.indicators.shows_paw()
                            && x >= paw.left && x < paw.right && y >= paw.top && y < paw.bottom;
                        (s.app.clone(), s.indicators.visible_timer().filter(|_| hit).map(|timer| (timer.id.clone(), timer.action())), hits_paw)
                    }) {
                        if let Some(timer) = timer {
                            change_pet_timer(app, timer);
                        } else if paw {
                            show_todos(&app, hwnd);
                        } else {
                            crate::bar::toggle(&app, hwnd);
                        }
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
                let _ = AppendMenuW(menu, MF_STRING, 1, w!("Open tools"));
                let _ = AppendMenuW(menu, MF_STRING, 10, w!("Toggle toolbar (Ctrl+Alt+P)"));
                let _ = AppendMenuW(menu, MF_STRING, 8, w!("Show open to-dos"));
                let _ = AppendMenuW(menu, MF_STRING, 9, w!("Show pet"));
                let _ = AppendMenuW(
                    menu,
                    MF_STRING | MF_DISABLED,
                    0,
                    w!("Drop files; drag pouch to copy; Alt-drag to move"),
                );
                let _ = AppendMenuW(menu, MF_STRING, 2, w!("Clear held references"));
                let _ = AppendMenuW(menu, MF_STRING, 6, w!("Copy held files (then paste)"));
                let timer = with_state(|s| s.indicators.visible_timer().map(|timer| (timer.id.clone(), timer.action()))).flatten();
                if let Some((_, action)) = &timer {
                    let label = if *action == "pause" { w!("Pause timer") } else { w!("Resume timer") };
                    let _ = AppendMenuW(menu, MF_STRING, 7, label);
                }
                let _ = AppendMenuW(menu, MF_STRING, 3, w!("Hide pet"));
                let _ = AppendMenuW(menu, MF_STRING, 4, w!("Reset position"));
                if let Ok(pets) = CreatePopupMenu() {
                    for (index, (pet, label)) in [
                        (Appearance::Calico, w!("Calico")),
                        (Appearance::Dog, w!("Shiba Inu")),
                        (Appearance::Panda, w!("Panda")),
                    ].into_iter().enumerate() {
                        let flags = MF_STRING | if pet == appearance() { MF_CHECKED } else { MF_UNCHECKED };
                        let _ = AppendMenuW(pets, flags, 20 + index, label);
                    }
                    if AppendMenuW(menu, MF_POPUP, pets.0 as usize, w!("Choose pet")).is_err() {
                        let _ = DestroyMenu(pets);
                    }
                }
                if let Ok(motion) = CreatePopupMenu() {
                    for index in 0..5 {
                        let label = wide(&format!("Set {}", index + 1));
                        let _ = AppendMenuW(motion, MF_STRING, 30 + index, PCWSTR(label.as_ptr()));
                    }
                    if AppendMenuW(menu, MF_POPUP, motion.0 as usize, w!("Try motion set")).is_err() {
                        let _ = DestroyMenu(motion);
                    }
                }
                let _ = AppendMenuW(menu, MF_STRING, 5, w!("Quit (reminders stop)"));
                let action = crate::menu_theme::track(menu, hwnd, p.x, p.y, appearance().palette());
                match action {
                    9 => {
                        let _ = PostMessageW(Some(hwnd), SHOW, WPARAM(0), LPARAM(0));
                    }
                    10 => {
                        if let Some(app) = with_state(|s| s.app.clone()) {
                            crate::bar::toggle(&app, hwnd);
                        }
                    }
                    20..=22 => {
                        if let Some(app) = with_state(|s| s.app.clone()) {
                            set_appearance(&app, [Appearance::Calico, Appearance::Dog, Appearance::Panda][action as usize - 20]);
                        }
                    }
                    30..=34 => {
                        if let Some(app) = with_state(|s| s.app.clone()) {
                            preview(&app, action as usize - 30);
                        }
                    }
                    8 => {
                        if let Some(app) = with_state(|s| s.app.clone()) {
                            show_todos(&app, hwnd);
                        }
                    }
                    7 => {
                        if let (Some(app), Some(timer)) = (with_state(|s| s.app.clone()), timer) {
                            change_pet_timer(app, timer);
                        }
                    }
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
                            advance_motion(hwnd, s, None);
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
            let appearance = setting(&app, "pet_appearance")
                .as_deref()
                .map(Appearance::from_key)
                .unwrap_or_default();
            STATE.with(|s| {
                *s.borrow_mut() = Some(State {
                    indicators: read_indicators(&app),
                    app: app.clone(),
                    control: output.clone(),
                    files: vec![],
                    deadline: None,
                    press: None,
                    hover: false,
                    appearance,
                    motion_set: motion_setting(&app, appearance),
                    motion: None,
                    motion_deadline: None,
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
            with_state(|s| update_countdown_tick(hwnd, &s.indicators));
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
pub fn refresh_indicators(app: &tauri::AppHandle) {
    let _ = message(app, REFRESH_INDICATORS);
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
pub fn pet_menu(app: &tauri::AppHandle) -> Result<(), String> {
    message(app, WM_CONTEXTMENU)
}
pub fn close_commands(app: tauri::AppHandle) -> Result<(), String> {
    message(&app, CLOSE_COMMANDS)
}
pub fn bar_visibility(app: &tauri::AppHandle, visible: bool) {
    if let Some(c) = app.try_state::<Arc<Control>>() {
        let changed = c.status.lock().is_ok_and(|mut status| {
            let changed = status.command_bar_visible != visible;
            status.command_bar_visible = visible;
            changed
        });
        if changed {
            cue(app, if visible { Cue::Wake } else { Cue::Settle });
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
    #[test]
    fn task_reactions_cannot_interrupt_file_motion_or_preview() {
        assert!(can_play_cue(None));
        assert!(can_play_cue(Some((AnimationPhase::Activity(Cue::Wake), 0, false))));
        for phase in [AnimationPhase::Pickup, AnimationPhase::Held, AnimationPhase::Release] {
            assert!(!can_play_cue(Some((phase, 0, false))));
            assert!(!can_play_cue(Some((phase, 0, true))));
        }
        assert_eq!(Appearance::from_key("cat"), Appearance::Calico);
        assert_eq!(Appearance::from_key("husky"), Appearance::Calico);
        for (index, cue) in Cue::ALL.into_iter().enumerate() {
            assert_eq!(cue as usize, index);
            assert!(!cue.frames().is_empty());
            assert!(cue.frames().iter().map(|frame| u32::from(frame.duration_ms)).sum::<u32>() <= 350);
        }
    }

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
