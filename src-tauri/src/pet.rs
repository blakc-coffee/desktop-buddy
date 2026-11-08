//! Native, idle-static Windows companion and temporary copy-only file handoff.
use super::*;
use crate::pet_art::{self, AnimationPhase, AnimationSet, Appearance, Cue};
use std::{
    cell::RefCell,
    sync::atomic::{AtomicBool, AtomicIsize, AtomicU64, Ordering},
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
const HALO_REFRESH: u32 = WM_APP + 22;
const MOTION_TIMER: usize = 2;
const COUNTDOWN_TIMER: usize = 3;
const NOTICE_TIMER: usize = 4;
pub const APPEARANCES: [Appearance; 3] = [Appearance::Calico, Appearance::Dog, Appearance::Panda];
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
    pending: Mutex<Option<(u64, Result<Vec<PathBuf>, String>)>>,
    import_generation: AtomicU64,
    active_import: AtomicU64,
    importing: AtomicBool,
    copy_reply: Mutex<Option<std::sync::mpsc::SyncSender<Result<(), String>>>>,
}
impl Control {
    fn queue_import_result(&self, generation: u64, result: Result<Vec<PathBuf>, String>) -> bool {
        let Ok(mut pending) = self.pending.lock() else { return false; };
        if self.stopping.load(Ordering::Relaxed)
            || self.import_generation.load(Ordering::SeqCst) != generation {
            return false;
        }
        *pending = Some((generation, result));
        true
    }
}
struct State {
    app: tauri::AppHandle,
    control: Arc<Control>,
    files: Vec<PathBuf>,
    hold_generation: u64,
    press: Option<(POINT, RECT, bool)>,
    hover: bool,
    appearance: Appearance,
    indicators: Indicators,
    motion_set: usize,
    motion: Option<(AnimationPhase, usize, bool)>,
    motion_deadline: Option<Instant>,
    drop_notice: Option<Instant>,
}
thread_local! { static STATE: RefCell<Option<State>> = const { RefCell::new(None) }; }
fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}
fn with_state<T>(f: impl FnOnce(&mut State) -> T) -> Option<T> {
    STATE.with(|s| s.borrow_mut().as_mut().map(f))
}
pub(crate) unsafe fn held_paths() -> Vec<PathBuf> {
    if let Some(hwnd) = with_state(|state| HWND(state.control.hwnd.load(Ordering::Relaxed) as _)) { refresh_hold(hwnd); }
    with_state(|state| state.files.clone()).unwrap_or_default()
}
pub fn appearance() -> Appearance {
    with_state(|state| state.appearance).unwrap_or_default()
}
pub fn explanation_position(control: &Control) -> Option<(i32,i32)> {
    unsafe {
        let hwnd=HWND(control.hwnd.load(Ordering::Relaxed) as _);
        if hwnd.0.is_null() {return None;}
        let mut pet=RECT::default();GetWindowRect(hwnd,&mut pet).ok()?;
        let mut info=MONITORINFO{cbSize:std::mem::size_of::<MONITORINFO>() as u32,..Default::default()};
        if !GetMonitorInfoW(MonitorFromWindow(hwnd,MONITOR_DEFAULTTONEAREST),&mut info).as_bool(){return None;}
        let scale=GetDpiForWindow(hwnd).max(96) as i32;let width=440*scale/96;let height=520*scale/96;let work=info.rcWork;
        let x=if pet.right+width+12<=work.right {pet.right+12}else{pet.left-width-12};
        Some((x.clamp(work.left,(work.right-width).max(work.left)),(pet.top-height/2).clamp(work.top,(work.bottom-height).max(work.top))))
    }
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
pub fn reset(app: &tauri::AppHandle) -> Result<(), String> { message(app, RESET) }
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
        status.seconds_left = 0;
    }
}
unsafe fn hold(hwnd: HWND, state: &mut State, files: Vec<PathBuf>) {
    state.drop_notice = None;
    let _ = KillTimer(Some(hwnd), NOTICE_TIMER);
    state.files = files;
    state.hold_generation = state.hold_generation.wrapping_add(1);
    advance_motion(hwnd, state, Some((AnimationPhase::Pickup, 0, false)));
    if let Ok(mut status) = state.control.status.lock() {
        status.message =
            "Files held until you clear or hand them off. Drag to copy out, or use /copy.".into();
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
unsafe fn sync_halo(hwnd: HWND) {
    if let Some((files, palette)) = with_state(|s| (s.files.clone(), s.appearance.palette())) {
        crate::file_halo::sync(hwnd, &files, palette);
    }
}
unsafe fn refresh_hold(hwnd: HWND) {
    with_state(|s| publish(s));
    sync_halo(hwnd);
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
pub(super) fn validated(paths: Vec<PathBuf>) -> Result<Vec<PathBuf>, String> {
    if paths.is_empty() || paths.len() > 20 {
        return Err("Drop 1–20 local files or folders at a time.".into());
    }
    let mut total = 0;
    let mut out = Vec::new();
    for path in paths {
        // Incoming virtual files are materialized locally before reaching this check.
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
                .is_some_and(|d| d.QueryGetData(&format()).is_ok() || crate::drop_data::supported(d))
                && incoming_effect(*effect) != DROPEFFECT_NONE;
            self.supported.store(okay, Ordering::Relaxed);
            *effect = if okay {
                incoming_effect(*effect)
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
                && incoming_effect(*effect) != DROPEFFECT_NONE
            {
                incoming_effect(*effect)
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
            let accepted = incoming_effect(*effect);
            *effect = DROPEFFECT_NONE;
            let result = if accepted != DROPEFFECT_NONE {
                data.as_ref()
                    .ok_or("No file data.".into())
                    .and_then(|d| paths(d).map(crate::drop_data::DropData::Paths).or_else(|_| crate::drop_data::read(d)))
            } else {
                Err("The source did not allow copying or linking.".into())
            };
            let result = result.and_then(|data| match data {
                crate::drop_data::DropData::Paths(files) => validated(files).map(Some),
                data => start_import(data).map(|_| None),
            });
            with_state(|s| {
                s.hover = false;
                match result {
                    Ok(Some(files)) => {
                        s.control.import_generation.fetch_add(1, Ordering::SeqCst);
                        hold(hwnd, s, files);
                        *effect = accepted;
                    }
                    Ok(None) => {
                        s.drop_notice = None;
                        let _ = KillTimer(Some(hwnd), NOTICE_TIMER);
                        *effect = accepted;
                    }
                    Err(message) => {
                        if let Ok(mut v) = s.control.status.lock() {
                            v.message = message;
                        }
                        s.drop_notice = Some(Instant::now() + Duration::from_secs(8));
                        let _ = SetTimer(Some(hwnd), NOTICE_TIMER, 1000, None);
                    }
                }
                publish(s);
            });
            let _ = InvalidateRect(Some(hwnd), None, false);
            Ok(())
        }
    }
}

fn incoming_effect(offered: DROPEFFECT) -> DROPEFFECT {
    if offered & DROPEFFECT_COPY != DROPEFFECT_NONE { DROPEFFECT_COPY }
    else if offered & DROPEFFECT_LINK != DROPEFFECT_NONE { DROPEFFECT_LINK }
    else { DROPEFFECT_NONE }
}

fn start_import(data: crate::drop_data::DropData) -> Result<(), String> {
    let (control, root) = with_state(|state| {
        let core = state.app.state::<Shared>();
        (state.control.clone(), core.data_path.join("drag-cache"))
    }).ok_or("Pet is unavailable.")?;
    if control.importing.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst).is_err() {
        return Err("An attachment import is still finishing. Try again shortly.".into());
    }
    let generation = control.import_generation.fetch_add(1, Ordering::SeqCst) + 1;
    control.active_import.store(generation, Ordering::SeqCst);
    if let Ok(mut status) = control.status.lock() {
        status.message = "Importing attachment…".into();
    }
    let import_control = control.clone();
    let worker = std::thread::Builder::new().name("buddy-file-import".into()).spawn(move || {
        let result = crate::drop_data::materialize(data, &root).and_then(validated);
        import_control.importing.store(false, Ordering::SeqCst);
        if import_control.queue_import_result(generation, result) {
            let hwnd = HWND(import_control.hwnd.load(Ordering::Relaxed) as _);
            unsafe { let _ = PostMessageW(Some(hwnd), HOLD, WPARAM(0), LPARAM(0)); }
        }
    });
    if worker.is_err() {
        control.importing.store(false, Ordering::SeqCst);
        return Err("Could not start the attachment import. Try again shortly.".into());
    }
    Ok(())
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
    refresh_hold(hwnd);
    // Clone before entering OLE's nested message loop; a newer hold never invalidates this transfer.
    let Some((files, control, deadline)) =
        with_state(|s| (s.files.clone(), s.control.clone(), s.hold_generation))
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
                Ok(false) => "Drag cancelled. Files remain held.".into(),
                Err(e) => e,
            };
        }
    });
    refresh_hold(hwnd);
}
unsafe fn finish_handoff(hwnd: HWND, state: &mut State, generation: u64) {
    // OLE can dispatch messages during a handoff. Preserve any newer batch.
    if state.hold_generation == generation {
        state.files.clear();
        state.hold_generation = state.hold_generation.wrapping_add(1);
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
    refresh_hold(hwnd);
    let (files, deadline) = with_state(|s| (s.files.clone(), s.hold_generation)).unwrap_or_default();
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
    fn shows_todo(&self) -> bool {
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

const TODO_BOUNDS: RECT = RECT { left: 3, top: 3, right: 47, bottom: 31 };
const NOTICE_BOUNDS: RECT = RECT { left: 9, top: 91, right: 119, bottom: 109 };

unsafe fn show_todos(app: &tauri::AppHandle, _hwnd: HWND) {
    crate::bar::dismiss();
    crate::navigation::todo(app.clone(), appearance().palette());
}

unsafe fn show_drop_notice(hwnd: HWND, message: &str) {
    if let Ok(menu) = CreatePopupMenu() {
        let text = wide(message);
        let _ = AppendMenuW(menu, MF_STRING | MF_DISABLED, 0, PCWSTR(text.as_ptr()));
        let _ = AppendMenuW(menu, MF_STRING, 1, w!("Close"));
        let mut rect = RECT::default();
        let _ = GetWindowRect(hwnd, &mut rect);
        crate::bar::dismiss();
        let _ = crate::menu_theme::track(menu, hwnd, rect.left, rect.bottom, appearance().palette());
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
    let (count, hover, appearance, pose, file_motion, timer, paw, awake, cue, notice) = with_state(|state| {
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
        let paw = state.indicators.shows_todo().then_some((
            matches!(state.indicators.reminder, Some(ReminderMark::Due)), state.indicators.todo_count));
        let importing = state.control.importing.load(Ordering::SeqCst)
            && state.control.active_import.load(Ordering::SeqCst) == state.control.import_generation.load(Ordering::SeqCst);
        let notice = if importing { Some("Importing…") }
            else if state.drop_notice.is_some_and(|until| until > Instant::now()) { Some("Drop failed · click") }
            else { None };
        (count, state.hover, state.appearance, Some(pose), file_motion, timer, paw, awake, cue, notice)
    }).unwrap_or_default();
    let mapped = SaveDC(dc);
    SetMapMode(dc, MM_TEXT);
    pet_art::draw_pose(dc, appearance, hover, dpi, pose.unwrap_or_else(AnimationSet::neutral_still_pose), file_motion, awake, cue);
    let _ = RestoreDC(dc, mapped);
    let palette = appearance.palette();
    let font = if count > 0 || timer.is_some() || paw.is_some() || notice.is_some() {
        CreateFontW(-13, 0, 0, 0, 700, 0, 0, 0, DEFAULT_CHARSET, OUT_DEFAULT_PRECIS,
            CLIP_DEFAULT_PRECIS, ANTIALIASED_QUALITY, DEFAULT_PITCH.0 as u32, w!("Trebuchet MS"))
    } else {
        HFONT::default()
    };
    let previous_font = (!font.0.is_null()).then(|| SelectObject(dc, font.into()));
    if let Some((due, pending)) = paw {
        pet_art::draw_todo_icon(dc, appearance, due);
        if pending > 0 {
            let label = if pending > 9 { "9+".into() } else { pending.to_string() };
            badge(dc, RECT { left: 29, top: 3, right: 47, bottom: 20 }, &label, palette.surface, palette.ink);
        }
    }
    if let Some(label) = notice {
        badge(dc, NOTICE_BOUNDS, label, palette.surface, palette.accent);
    }
    if count > 0 {
        let compact = timer.is_some();
        let label = if compact { count.to_string() } else { format!("{count} held") };
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
    sync_halo(hwnd);
}
unsafe extern "system" fn proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if let Some(result) = crate::menu_theme::handle_message(msg, wp, lp) {
        return result;
    }
    match msg {
        WM_WINDOWPOSCHANGED | WM_SHOWWINDOW => {
            let result = DefWindowProcW(hwnd, msg, wp, lp);
            let _ = PostMessageW(Some(hwnd), HALO_REFRESH, WPARAM(0), LPARAM(0));
            result
        }
        HALO_REFRESH => { sync_halo(hwnd); LRESULT(0) }
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
            refresh_hold(hwnd);
            LRESULT(0)
        }
        WM_TIMER if wp.0 == NOTICE_TIMER => {
            with_state(|state| {
                if state.drop_notice.is_none_or(|until| until <= Instant::now()) {
                    state.drop_notice = None;
                    let _ = KillTimer(Some(hwnd), NOTICE_TIMER);
                }
            });
            let _ = InvalidateRect(Some(hwnd), None, false);
            LRESULT(0)
        }
        HOLD => {
            with_state(|s| {
                let pending = s.control.pending.lock().ok().and_then(|mut p| p.take());
                if let Some((generation, result)) = pending {
                    if generation != s.control.import_generation.load(Ordering::SeqCst) { return; }
                    match result {
                        Ok(files) => hold(hwnd, s, files),
                        Err(message) => {
                            if let Ok(mut status) = s.control.status.lock() { status.message = message; }
                            s.drop_notice = Some(Instant::now() + Duration::from_secs(8));
                            let _ = SetTimer(Some(hwnd), NOTICE_TIMER, 1000, None);
                        }
                    }
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
                let _ = state.app.emit("data-changed", ());
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
                    if let Some(Some(message)) = with_state(|s| {
                        let r = NOTICE_BOUNDS;
                        if s.drop_notice.is_some_and(|until| until > Instant::now())
                            && x >= r.left && x < r.right && y >= r.top && y < r.bottom {
                            s.control.status.lock().ok().map(|status| status.message.clone())
                        } else { None }
                    }) {
                        show_drop_notice(hwnd, &message);
                        return LRESULT(0);
                    }
                    if let Some((app, timer, paw)) = with_state(|s| {
                        let r = timer_bounds(!s.files.is_empty());
                        let hit = x >= r.left && x < r.right && y >= r.top && y < r.bottom;
                        let paw = TODO_BOUNDS;
                        let hits_paw = s.indicators.shows_todo()
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
            refresh_hold(hwnd);
            let mut p = POINT::default();
            let _ = GetCursorPos(&mut p);
            if let Ok(menu) = CreatePopupMenu() {
                let _ = AppendMenuW(menu, MF_STRING, 1, w!("Open tools"));
                let _ = AppendMenuW(menu, MF_STRING, 3, w!("Hide pet"));
                let _ = AppendMenuW(menu, MF_STRING, 2, w!("Clear held references"));
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
                let _ = AppendMenuW(menu, MF_STRING, 5, w!("Quit"));
                let action = crate::menu_theme::track(menu, hwnd, p.x, p.y, appearance().palette());
                match action {
                    20..=22 => {
                        if let Some(app) = with_state(|s| s.app.clone()) {
                            set_appearance(&app, [Appearance::Calico, Appearance::Dog, Appearance::Panda][action as usize - 20]);
                        }
                    }
                    1 => {
                        with_state(|s| post_main(&s.app, false));
                    }
                    2 => {
                        with_state(|s| {
                            s.control.import_generation.fetch_add(1, Ordering::SeqCst);
                            s.files.clear();
                            s.hold_generation = s.hold_generation.wrapping_add(1);
                            advance_motion(hwnd, s, None);
                            publish(s);
                        });
                        let _ = KillTimer(Some(hwnd), 1);
                        let _ = InvalidateRect(Some(hwnd), None, false);
                    }
                    3 => {
                        let _ = PostMessageW(Some(hwnd), HIDE, WPARAM(0), LPARAM(0));
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
            crate::file_halo::clear();
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
        import_generation: AtomicU64::new(0),
        active_import: AtomicU64::new(0),
        importing: AtomicBool::new(false),
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
                    hold_generation: 0,
                    press: None,
                    hover: false,
                    appearance,
                    motion_set: motion_setting(&app, appearance),
                    motion: None,
                    motion_deadline: None,
                    drop_notice: None,
                })
            });
            let hwnd = CreateWindowExW(
                WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_TOPMOST | WS_EX_NOACTIVATE,
                class,
                w!("Desktop Buddy pet — drop files to hold"),
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
    let mut pending = c.pending.lock().map_err(err)?;
    let generation = c.import_generation.fetch_add(1, Ordering::SeqCst) + 1;
    *pending = Some((generation, Ok(files)));
    drop(pending);
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
    fn old_imports_cannot_replace_a_newer_hold_or_reopen_a_cleared_batch() {
        let control = Control {
            hwnd: AtomicIsize::new(0),
            stopping: AtomicBool::new(false),
            status: Mutex::new(Status::default()),
            pending: Mutex::new(None),
            import_generation: AtomicU64::new(1),
            active_import: AtomicU64::new(0),
            importing: AtomicBool::new(false),
            copy_reply: Mutex::new(None),
        };
        control.import_generation.store(2, Ordering::SeqCst);
        assert!(control.queue_import_result(2, Ok(vec![PathBuf::from("latest")])));
        assert!(!control.queue_import_result(1, Ok(vec![PathBuf::from("old")])));
        let (_, result) = control.pending.lock().unwrap().take().unwrap();
        assert_eq!(result.unwrap(), vec![PathBuf::from("latest")]);
        control.import_generation.store(3, Ordering::SeqCst);
        assert!(!control.queue_import_result(2, Err("late error".into())));
        assert!(control.pending.lock().unwrap().is_none());
        assert_eq!(incoming_effect(DROPEFFECT_COPY | DROPEFFECT_MOVE), DROPEFFECT_COPY);
        assert_eq!(incoming_effect(DROPEFFECT_LINK), DROPEFFECT_LINK);
        assert_eq!(incoming_effect(DROPEFFECT_MOVE), DROPEFFECT_NONE);
    }

    #[test]
    fn imported_attachments_roundtrip_through_the_existing_copy_handoff() {
        unsafe {
            OleInitialize(None).unwrap();
            let root = std::env::temp_dir().join(format!("buddy-import-copy-{}", super::super::new_id()));
            let files = crate::drop_data::materialize(crate::drop_data::DropData::Files(vec![
                ("猫.txt".into(), b"attachment one".to_vec()),
                ("menu.pdf".into(), b"attachment two".to_vec()),
            ]), &root).unwrap();
            let data = shell_data(files.clone()).unwrap();
            prefer_copy(&data).unwrap();
            assert_eq!(paths(&data).unwrap(), files);
            assert_eq!(std::fs::read(&files[0]).unwrap(), b"attachment one");
            assert_eq!(std::fs::read(&files[1]).unwrap(), b"attachment two");
            drop(data);
            std::fs::remove_dir_all(root).unwrap();
            OleUninitialize();
        }
    }
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
    fn render_species_todo_badges_and_idle_held_poses_offscreen() {
        unsafe {
            let (width, height) = (880, 610);
            let dc = CreateCompatibleDC(None);
            let info = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: 40,
                    biWidth: width,
                    biHeight: -height,
                    biPlanes: 1,
                    biBitCount: 32,
                    ..Default::default()
                },
                ..Default::default()
            };
            let mut bits = std::ptr::null_mut();
            let bitmap = CreateDIBSection(Some(dc), &info, DIB_RGB_COLORS, &mut bits, None, 0).unwrap();
            let old = SelectObject(dc, bitmap.into());
            let background = CreateSolidBrush(pet_art::rgb(29, 32, 34));
            FillRect(
                dc,
                &RECT {
                    left: 0,
                    top: 0,
                    right: width,
                    bottom: height,
                },
                background,
            );
            let _ = DeleteObject(background.into());
            for (column, appearance) in [Appearance::Calico, Appearance::Dog, Appearance::Panda]
                .into_iter()
                .enumerate()
            {
                for row in 0..2 {
                    let saved = SaveDC(dc);
                    let _ = SetViewportOrgEx(dc, 12 + column as i32 * 290, 12 + row * 300, None);
                    let held = row == 1;
                    let pose = if held {
                        AnimationSet::reduced_motion_pose(AnimationPhase::Held)
                    } else {
                        AnimationSet::neutral_still_pose()
                    };
                    pet_art::draw_pose(
                        dc,
                        appearance,
                        false,
                        192,
                        pose,
                        held.then_some((0, 0)),
                        held,
                        None,
                    );
                    SetMapMode(dc, MM_ANISOTROPIC);
                    let _ = SetWindowExtEx(dc, 96, 96, None);
                    let _ = SetViewportExtEx(dc, 192, 192, None);
                    pet_art::draw_todo_icon(dc, appearance, held);
                    let _ = RestoreDC(dc, saved);
                }
            }
            let pixels = std::slice::from_raw_parts(bits as *const u8, (width * height * 4) as usize);
            assert!(
                pixels
                    .chunks_exact(4)
                    .filter(|p| p[0] > 100 && p[1] > 100 && p[2] > 100)
                    .count()
                    > 3000
            );
            if let Some(path) = std::env::var_os("BUDDY_PET_THEME_PREVIEW") {
                let rgb: Vec<_> = pixels
                    .chunks_exact(4)
                    .flat_map(|p| [p[2], p[1], p[0]])
                    .collect();
                let mut out = Vec::new();
                {
                    let mut encoder = png::Encoder::new(&mut out, width as u32, height as u32);
                    encoder.set_color(png::ColorType::Rgb);
                    encoder.set_depth(png::BitDepth::Eight);
                    encoder
                        .write_header()
                        .unwrap()
                        .write_image_data(&rgb)
                        .unwrap();
                }
                std::fs::write(path, out).unwrap();
            }
            SelectObject(dc, old);
            let _ = DeleteObject(bitmap.into());
            let _ = DeleteDC(dc);
        }
    }
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
                import_generation: AtomicU64::new(0),
                active_import: AtomicU64::new(0),
                importing: AtomicBool::new(false),
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
