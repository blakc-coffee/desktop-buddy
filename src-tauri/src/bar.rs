//! Small native command bar. WebView2 is only opened explicitly through /tools.
use super::*;
use crate::commands::{self, Command as Parsed};
use std::{
    cell::RefCell,
    sync::atomic::{AtomicBool, AtomicIsize, AtomicU64, Ordering},
};
use windows::{
    core::{w, PCWSTR},
    Win32::{
        Foundation::*,
        Graphics::Gdi::*,
        System::{
            LibraryLoader::GetModuleHandleW,
            SystemServices::{SS_ENDELLIPSIS, SS_NOPREFIX},
        },
        UI::{
            Controls::{EM_SETLIMITTEXT, EM_SETSEL},
            HiDpi::GetDpiForWindow,
            Input::KeyboardAndMouse::*,
            WindowsAndMessaging::*,
        },
    },
};
const DONE: u32 = WM_APP + 30;
const COPIED: u32 = WM_APP + 31;
const WIDTH: i32 = 392;
const HEIGHT: i32 = 46;
const ROW_HEIGHT: i32 = 25;
const VISIBLE_ROWS: usize = 5;
const SURFACE: COLORREF = COLORREF(0x0031222c);
const INK: COLORREF = COLORREF(0x00f6ebfa);
const ACCENT: COLORREF = COLORREF(0x00bae9c9);
#[derive(Clone)]
enum Action {
    Input(String),
    Complete(&'static str),
    Tools,
    Copy,
    Open(String),
    OpenFile(String),
    Editor(String),
}
struct Request {
    app: tauri::AppHandle,
    query: String,
    generation: u64,
}
struct Packet {
    generation: u64,
    results: Vec<SearchResult>,
    projects: Vec<Item>,
    limited: bool,
}
struct Search {
    hwnd: AtomicIsize,
    generation: AtomicU64,
    running: AtomicBool,
    pending: Mutex<Option<Request>>,
    result: Mutex<Option<Packet>>,
    copy_result: Mutex<Option<(u64, Result<(), String>)>>,
}
struct Bar {
    app: tauri::AppHandle,
    pet: HWND,
    hwnd: HWND,
    edit: HWND,
    status: HWND,
    old_edit: WNDPROC,
    choices: Vec<(String, Action)>,
    selected: usize,
    search: Arc<Search>,
    font: HFONT,
    dpi: u32,
    composing: bool,
    finder: bool,
}
thread_local! {static BAR:RefCell<Option<Bar>>=const{RefCell::new(None)};}
fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}
fn get<T>(f: impl FnOnce(&Bar) -> T) -> Option<T> {
    BAR.with(|s| s.borrow().as_ref().map(f))
}
fn change<T>(f: impl FnOnce(&mut Bar) -> T) -> Option<T> {
    BAR.with(|s| s.borrow_mut().as_mut().map(f))
}
unsafe fn note(text: &str) {
    if let Some((hwnd, status)) = get(|b| (b.hwnd, b.status)) {
        let value = wide(text);
        let _ = SetWindowTextW(status, PCWSTR(value.as_ptr()));
        let _ = ShowWindow(
            status,
            if text.is_empty() {
                SW_HIDE
            } else {
                SW_SHOWNOACTIVATE
            },
        );
        layout(hwnd);
    }
}
unsafe fn contents() -> String {
    let Some(edit) = get(|b| b.edit) else {
        return String::new();
    };
    let len = GetWindowTextLengthW(edit).min(1024).max(0) as usize;
    let mut v = vec![0; len + 1];
    let n = GetWindowTextW(edit, &mut v);
    String::from_utf16_lossy(&v[..n.max(0) as usize])
}
unsafe fn choices(rows: Vec<(String, Action)>) {
    change(|b| {
        b.choices = rows;
        b.selected = 0;
    });
    show_choice();
}
unsafe fn show_choice() {
    if let Some((true, hwnd)) = get(|b| (b.finder, b.hwnd)) {
        note("");
        let _ = InvalidateRect(Some(hwnd), None, false);
        return;
    }
    let label = get(|b| {
        let (label, _) = b.choices.get(b.selected)?;
        if label.is_empty() {
            return None;
        }
        Some(format!("{}/{}  {}", b.selected + 1, b.choices.len(), label))
    })
    .flatten();
    note(label.as_deref().unwrap_or(""));
}
fn projects(app: &tauri::AppHandle) -> Vec<Item> {
    app.try_state::<Shared>()
        .and_then(|c| c.db.lock().ok().and_then(|db| rows(&db).ok()))
        .unwrap_or_default()
        .into_iter()
        .filter(|i| i.kind == "project")
        .collect()
}
unsafe fn refresh() {
    let text = contents();
    get(|b| b.search.generation.fetch_add(1, Ordering::Relaxed));
    change(|b| b.finder = false);
    let command = if !text.trim().is_empty() && !text.trim_start().starts_with('/') {
        Ok(Parsed::Find(text.trim().into()))
    } else {
        commands::parse(&text)
    };
    if !matches!(command, Ok(Parsed::Tools | Parsed::Copy)) {
        let suggestions = commands::suggestions(&text);
        if !suggestions.is_empty() {
            choices(
                suggestions
                    .into_iter()
                    .map(|(hint, completion)| (hint.into(), Action::Complete(completion)))
                    .collect(),
            );
            return;
        }
    }
    match command {
        Ok(Parsed::Help) => {
            choices(vec![]);
            if text.trim().eq_ignore_ascii_case("/help") {
                note("/t /d /r /f /w /copy /tools");
            }
        }
        Ok(Parsed::Find(q)) => {
            change(|b| b.finder = true);
            if q.chars().count() >= 2 {
                search(q.to_lowercase());
            } else {
                choices(vec![]);
                note("Type at least 2 characters");
            }
        }
        Ok(Parsed::Workspaces(q)) => {
            let Some(app) = get(|b| b.app.clone()) else {
                return;
            };
            let mut result = Vec::new();
            for p in projects(&app) {
                if p.title.to_lowercase().contains(&q.to_lowercase()) {
                        result.push((
                            format!("{} — open in editor", p.title),
                            Action::Editor(p.path.clone()),
                        ));
                        result.push((format!("{} — open folder", p.title), Action::Open(p.path)));
                }
                if result.len() >= 20 {
                    break;
                }
            }
            let empty = result.is_empty();
            choices(result);
            if empty {
                note("No match in saved projects");
            }
        }
        Ok(Parsed::Tools) => choices(vec![(String::new(), Action::Tools)]),
        Ok(Parsed::Copy) => choices(vec![(String::new(), Action::Copy)]),
        Ok(Parsed::Timer { .. } | Parsed::Reminder { .. } | Parsed::Todo(_)) => {
            choices(vec![(String::new(), Action::Input(text))])
        }
        Err(_) => {
            choices(vec![]);
            if let Some((verb, args)) = text.trim_start().split_once(char::is_whitespace) {
                if args.trim().is_empty() {
                    if let Some((hint, _)) = commands::suggestions(verb).first() {
                        note(hint);
                    }
                }
            }
        }
    }
}
pub fn dismiss() {
    unsafe {
        hide();
    }
}
unsafe fn hide() {
    if let Some((h, search)) = get(|b| (b.hwnd, b.search.clone())) {
        search.generation.fetch_add(1, Ordering::Relaxed);
        let _ = ShowWindow(h, SW_HIDE);
        if let Some(app) = get(|b| b.app.clone()) {
            crate::pet::bar_visibility(&app, false);
        }
    }
}
fn open_tools(app: tauri::AppHandle) {
    let h = app.clone();
    let _ = app.run_on_main_thread(move || show_panel(&h));
}
fn open_editor(path: String) -> Result<(), String> {
    if !Path::new(&path).is_absolute() || !Path::new(&path).is_dir() {
        return Err("Workspace folder no longer exists.".into());
    }
    let native = resolve_cli("cursor").filter(|p| p.extension().is_some_and(|e| e == "exe"));
    let local = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .map(|p| p.join("Programs/cursor/Cursor.exe"))
        .filter(|p| p.is_file());
    let exe=native.or(local).ok_or("Cursor IDE was not found. Choose the Open folder result, or install/configure Cursor first.")?;
    use std::os::windows::process::CommandExt;
    std::process::Command::new(exe)
        .arg("--new-window")
        .arg(path)
        .creation_flags(0x08000000)
        .spawn()
        .map_err(err)?;
    Ok(())
}
unsafe fn execute() {
    let Some((app, action)) = get(|b| {
        (
            b.app.clone(),
            b.choices.get(b.selected).map(|row| row.1.clone()),
        )
    }) else {
        return;
    };
    let Some(action) = action else {
        if let Err(error) = commands::parse(&contents()) {
            note(&error);
        }
        return;
    };
    let result = match action {
        Action::Complete(value) => {
            fill(value);
            return;
        }
        Action::Input(value) => match commands::parse(&value) {
            Ok(Parsed::Timer { minutes, title }) => start_timer(
                app.clone(),
                app.state::<Shared>(),
                title,
                minutes,
                String::new(),
            )
            .map(|_| "Timer started.".to_string()),
            Ok(Parsed::Todo(title)) => save_item(
                app.clone(),
                app.state::<Shared>(),
                Item {
                    kind: "todo".into(),
                    title,
                    ..Default::default()
                },
            )
            .map(|_| "Todo saved.".to_string()),
            Ok(Parsed::Reminder { minutes, title }) => save_item(
                app.clone(),
                app.state::<Shared>(),
                Item {
                    kind: "reminder".into(),
                    title,
                    due_at: now() + minutes as i64 * 60000,
                    ..Default::default()
                },
            )
            .map(|_| "Reminder saved.".to_string()),
            _ => Err("Complete the command first.".into()),
        },
        Action::Tools => {
            hide();
            open_tools(app);
            return;
        }
        Action::Open(path) => open_path(path).map(|_| "Folder opened.".to_string()),
        Action::OpenFile(path) => {
            let path = wide(&path);
            let result = windows::Win32::UI::Shell::ShellExecuteW(
                None, w!("open"), PCWSTR(path.as_ptr()), None, None, SW_SHOWNORMAL,
            );
            if result.0 as isize > 32 {
                Ok("File opened.".into())
            } else {
                Err("Windows could not open this file.".into())
            }
        }
        Action::Editor(path) => open_editor(path).map(|_| "Workspace opened.".to_string()),
        Action::Copy => {
            let app2 = app.clone();
            let Some(shared) = get(|b| b.search.clone()) else {
                return;
            };
            let generation = shared.generation.fetch_add(1, Ordering::Relaxed) + 1;
            choices(vec![]);
            tauri::async_runtime::spawn(async move {
                let result = crate::pet::pet_copy(app2).await;
                if shared.generation.load(Ordering::Relaxed) == generation {
                    *shared.copy_result.lock().unwrap() = Some((generation, result));
                    let _ = PostMessageW(
                        Some(HWND(shared.hwnd.load(Ordering::Relaxed) as _)),
                        COPIED,
                        WPARAM(0),
                        LPARAM(0),
                    );
                }
            });
            note("Copying…");
            return;
        }
    };
    match result {
        Ok(message) => {
            choices(vec![]);
            note(&message);
        }
        Err(e) => note(&e),
    }
}
unsafe fn fill(value: &str) {
    if let Some(edit) = get(|b| b.edit) {
        let text = wide(value);
        let end = text.len() - 1;
        let _ = SetWindowTextW(edit, PCWSTR(text.as_ptr()));
        SendMessageW(
            edit,
            EM_SETSEL,
            Some(WPARAM(end)),
            Some(LPARAM(end as isize)),
        );
    }
}

fn search(query: String) {
    let Some((shared, app)) = get(|b| (b.search.clone(), b.app.clone())) else {
        return;
    };
    let generation = shared.generation.fetch_add(1, Ordering::Relaxed) + 1;
    unsafe {
        choices(tool_matches(&query));
        note("Searching…");
    }
    {
        let mut pending = shared.pending.lock().unwrap();
        *pending = Some(Request {
            app,
            query,
            generation,
        });
        if shared.running.swap(true, Ordering::Relaxed) {
            return;
        }
    }
    std::thread::spawn(move || loop {
        let request = {
            let mut pending = shared.pending.lock().unwrap();
            match pending.take() {
                Some(r) => r,
                None => {
                    shared.running.store(false, Ordering::Relaxed);
                    break;
                }
            }
        };
        std::thread::sleep(Duration::from_millis(150));
        let current = || shared.generation.load(Ordering::Relaxed) == request.generation;
        if !current() {
            continue;
        }
        let projects = projects(&request.app);
        let saved = projects.iter().map(|project| PathBuf::from(&project.path));
        let roots = finder::roots(&request.app, saved);
        let (results, limited) = finder::find(roots, &request.query, current);
        if current() {
            *shared.result.lock().unwrap() = Some(Packet {
                generation: request.generation,
                results,
                projects,
                limited,
            });
            unsafe {
                let _ = PostMessageW(
                    Some(HWND(shared.hwnd.load(Ordering::Relaxed) as _)),
                    DONE,
                    WPARAM(0),
                    LPARAM(0),
                );
            }
        }
    });
}

fn tool_matches(query: &str) -> Vec<(String, Action)> {
    commands::definitions()
        .into_iter()
        .filter(|(aliases, label, _)| {
            finder::match_rank(label, query).is_some()
                || aliases.iter().any(|alias| finder::match_rank(alias.trim_start_matches('/'), query).is_some())
        })
        .map(|(_, label, completion)| {
            let action = match completion {
                "/tools" => Action::Tools,
                "/copy" => Action::Copy,
                _ => Action::Complete(completion),
            };
            (label.into(), action)
        })
        .collect()
}
unsafe fn layout(hwnd: HWND) {
    let dpi = GetDpiForWindow(hwnd).max(96);
    let s = |v: i32| v * dpi as i32 / 96;
    let Some((edit, status, pet, rows)) = get(|b| {
        (b.edit, b.status, b.pet, if b.finder { b.choices.len().min(VISIBLE_ROWS) } else { 0 })
    }) else {
        return;
    };
    let has_note = GetWindowTextLengthW(status) > 0;
    let mut rect = RECT::default();
    let _ = GetClientRect(hwnd, &mut rect);
    let height = HEIGHT + rows as i32 * ROW_HEIGHT;
    let pet_dpi = GetDpiForWindow(pet).max(96) as i32;
    if rect.bottom != height * pet_dpi / 96 {
        position(hwnd, pet, height);
        let _ = GetClientRect(hwnd, &mut rect);
        let _ = InvalidateRect(Some(hwnd), None, true);
    }
    let width = rect.right;
    let start = s(43);
    let end = width - s(14);
    let divider = (width * 48 / 100).max(start + s(65));
    let edit_end = if has_note { divider - s(8) } else { end };
    let _ = SetWindowPos(
        edit,
        None,
        start,
        s(13),
        (edit_end - start).max(1),
        s(22),
        SWP_NOZORDER,
    );
    let _ = SetWindowPos(
        status,
        None,
        divider,
        s(13),
        (end - divider).max(1),
        s(22),
        SWP_NOZORDER,
    );
    if get(|b| b.dpi) != Some(dpi) {
        let font = CreateFontW(
            -s(14),
            0,
            0,
            0,
            400,
            0,
            0,
            0,
            DEFAULT_CHARSET,
            OUT_DEFAULT_PRECIS,
            CLIP_DEFAULT_PRECIS,
            DEFAULT_QUALITY,
            DEFAULT_PITCH.0 as u32,
            w!("Segoe UI"),
        );
        let old = change(|b| {
            let old = b.font;
            b.font = font;
            b.dpi = dpi;
            old
        });
        for h in [edit, status] {
            SendMessageW(
                h,
                WM_SETFONT,
                Some(WPARAM(font.0 as usize)),
                Some(LPARAM(1)),
            );
        }
        if let Some(old) = old {
            if !old.is_invalid() {
                let _ = DeleteObject(old.into());
            }
        }
    }
}
unsafe extern "system" fn input_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if msg == WM_IME_STARTCOMPOSITION {
        change(|b| b.composing = true);
    }
    if msg == WM_IME_ENDCOMPOSITION {
        change(|b| b.composing = false);
    }
    let composing = get(|b| b.composing).unwrap_or(false);
    if msg == WM_CHAR && !composing && matches!(wp.0, 9 | 13 | 27) {
        return LRESULT(0);
    }
    if msg == WM_KEYDOWN && !composing {
        match wp.0 as u16 {
            x if x == VK_ESCAPE.0 => {
                hide();
                return LRESULT(0);
            }
            x if x == VK_RETURN.0 => {
                execute();
                return LRESULT(0);
            }
            x if x == VK_TAB.0 => {
                if let Some(Some(Action::Complete(value))) =
                    get(|b| b.choices.get(b.selected).map(|row| row.1.clone()))
                {
                    fill(value);
                }
                return LRESULT(0);
            }
            x if x == VK_DOWN.0 || x == VK_UP.0 => {
                let changed = change(|b| {
                    let count = b.choices.len();
                    if count < 2 {
                        return false;
                    }
                    let step = if x == VK_DOWN.0 { 1 } else { count - 1 };
                    b.selected = (b.selected + step) % count;
                    true
                })
                .unwrap_or(false);
                if changed {
                    show_choice();
                    return LRESULT(0);
                }
            }
            _ => {}
        }
    }
    let original = get(|b| b.old_edit).flatten();
    CallWindowProcW(original, hwnd, msg, wp, lp)
}
unsafe fn paint(hwnd: HWND) {
    let mut paint = PAINTSTRUCT::default();
    let dc = BeginPaint(hwnd, &mut paint);
    let dpi = GetDpiForWindow(hwnd).max(96) as i32;
    let scale = |value| value * dpi / 96;
    let mut rect = RECT::default();
    let _ = GetClientRect(hwnd, &mut rect);
    let surface = CreateSolidBrush(SURFACE);
    let outline = CreatePen(PS_SOLID, 1, COLORREF(0x006a4e68));
    let old_brush = SelectObject(dc, surface.into());
    let old_pen = SelectObject(dc, outline.into());
    let _ = RoundRect(dc, 0, 0, rect.right, rect.bottom, scale(22), scale(22));
    let paw = CreateSolidBrush(ACCENT);
    SelectObject(dc, paw.into());
    SelectObject(dc, GetStockObject(NULL_PEN));
    for (left, top, right, bottom) in [
        (19, 22, 31, 32),
        (14, 18, 19, 24),
        (20, 14, 25, 21),
        (27, 14, 32, 21),
        (33, 18, 38, 24),
    ] {
        let _ = Ellipse(dc, scale(left), scale(top), scale(right), scale(bottom));
    }
    SelectObject(dc, old_brush);
    SelectObject(dc, old_pen);
    get(|bar| {
        if !bar.finder {
            return;
        }
        let previous_font = SelectObject(dc, bar.font.into());
        SetBkMode(dc, TRANSPARENT);
        let first = bar.selected / VISIBLE_ROWS * VISIBLE_ROWS;
        for (index, (label, _)) in bar.choices.iter().enumerate().skip(first).take(VISIBLE_ROWS) {
            let top = HEIGHT + (index - first) as i32 * ROW_HEIGHT;
            let mut bounds = RECT {
                left: scale(12),
                top: scale(top),
                right: rect.right - scale(12),
                bottom: scale(top + ROW_HEIGHT),
            };
            if index == bar.selected {
                let highlight = CreateSolidBrush(COLORREF(0x0056404e));
                let _ = FillRect(dc, &bounds, highlight);
                let _ = DeleteObject(highlight.into());
            }
            SetTextColor(dc, if index == bar.selected { ACCENT } else { INK });
            bounds.left += scale(8);
            bounds.right -= scale(8);
            let mut label: Vec<_> = label.encode_utf16().collect();
            DrawTextW(dc, &mut label, &mut bounds, DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS | DT_NOPREFIX);
        }
        SelectObject(dc, previous_font);
    });
    let _ = DeleteObject(paw.into());
    let _ = DeleteObject(outline.into());
    let _ = DeleteObject(surface.into());
    let _ = EndPaint(hwnd, &paint);
}

unsafe extern "system" fn proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        WM_PAINT => {
            paint(hwnd);
            LRESULT(0)
        }
        WM_LBUTTONUP => {
            let y = (lp.0 >> 16) as i16 as i32 * 96 / GetDpiForWindow(hwnd).max(96) as i32;
            if y >= HEIGHT && change(|bar| {
                let first = bar.selected / VISIBLE_ROWS * VISIBLE_ROWS;
                let index = first + ((y - HEIGHT) / ROW_HEIGHT) as usize;
                if bar.finder && index < bar.choices.len() {
                    bar.selected = index;
                    true
                } else {
                    false
                }
            }).unwrap_or(false) {
                execute();
            }
            LRESULT(0)
        }
        COPIED => {
            let result = get(|b| b.search.clone()).and_then(|s| {
                let (generation, result) = s.copy_result.lock().ok()?.take()?;
                if generation == s.generation.load(Ordering::Relaxed) {
                    Some(result)
                } else {
                    None
                }
            });
            if let Some(result) = result {
                match result {
                    Ok(()) => note("Copied. Ready to paste."),
                    Err(e) => note(&e),
                }
            }
            LRESULT(0)
        }
        WM_CTLCOLORSTATIC | WM_CTLCOLOREDIT => {
            let dc = HDC(wp.0 as _);
            SetTextColor(
                dc,
                if msg == WM_CTLCOLORSTATIC {
                    ACCENT
                } else {
                    INK
                },
            );
            SetBkColor(dc, SURFACE);
            LRESULT(GetClassLongPtrW(hwnd, GCLP_HBRBACKGROUND) as isize)
        }
        WM_COMMAND => {
            let id = wp.0 & 0xffff;
            let code = (wp.0 >> 16) & 0xffff;
            if id == 1 && code == EN_CHANGE as usize {
                refresh();
            }
            LRESULT(0)
        }
        DONE => {
            let packet = get(|b| b.search.clone()).and_then(|s| {
                let packet = s.result.lock().ok()?.take()?;
                if packet.generation == s.generation.load(Ordering::Relaxed) {
                    Some(packet)
                } else {
                    None
                }
            });
            if let Some(packet) = packet {
                let text = contents();
                let query = match commands::parse(&text) {
                    Ok(Parsed::Find(query)) => query,
                    _ => text.trim().into(),
                };
                let mut rows = tool_matches(&query.to_lowercase());
                let query = query.to_lowercase();
                let mut project_paths = std::collections::HashSet::new();
                for project in packet.projects {
                    if rows.len() >= finder::MAX_RESULTS {
                        break;
                    }
                    if finder::match_rank(&project.title, &query).is_some()
                        && project_paths.insert(project.path.to_lowercase())
                    {
                        rows.push((
                            format!("{}  ·  Project  ·  {}", project.title, project.path),
                            Action::Open(project.path),
                        ));
                    }
                }
                rows.extend(packet.results.into_iter().filter(|result| {
                    !project_paths.contains(&result.path.to_lowercase())
                }).map(|result| {
                    let parent = Path::new(&result.path).parent()
                        .map(|path| path.display().to_string()).unwrap_or_default();
                    let kind = if result.folder { "Folder" } else { "File" };
                    let label = format!("{}  ·  {kind}  ·  {parent}", result.name);
                    let action = if result.folder { Action::Open(result.path) } else { Action::OpenFile(result.path) };
                    (label, action)
                }));
                let empty = rows.is_empty();
                choices(rows);
                if packet.limited {
                    note("Partial results");
                } else if empty {
                    note("No matches");
                }
            }
            LRESULT(0)
        }
        WM_ACTIVATE if wp.0 & 0xffff == WA_INACTIVE as usize => {
            hide();
            LRESULT(0)
        }
        WM_SIZE => {
            let dpi = GetDpiForWindow(hwnd).max(96) as i32;
            let radius = 22 * dpi / 96;
            let mut rect = RECT::default();
            let _ = GetClientRect(hwnd, &mut rect);
            let region = CreateRoundRectRgn(0, 0, rect.right, rect.bottom, radius, radius);
            if SetWindowRgn(hwnd, Some(region), true) == 0 {
                let _ = DeleteObject(region.into());
            }
            layout(hwnd);
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
            layout(hwnd);
            LRESULT(0)
        }
        WM_CLOSE => {
            hide();
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wp, lp),
    }
}
pub unsafe fn toggle(app: &tauri::AppHandle, pet: HWND) {
    if let Some(h) = get(|b| b.hwnd) {
        if IsWindowVisible(h).as_bool() {
            hide();
            return;
        }
    }
    if get(|b| b.hwnd).is_none() {
        let instance = match GetModuleHandleW(None) {
            Ok(v) => v,
            Err(_) => return,
        };
        let class = w!("DesktopBuddyCommandBar");
        let bg = CreateSolidBrush(SURFACE);
        RegisterClassW(&WNDCLASSW {
            lpfnWndProc: Some(proc),
            hInstance: instance.into(),
            lpszClassName: class,
            hbrBackground: bg,
            ..Default::default()
        });
        let Ok(hwnd) = CreateWindowExW(
            WS_EX_TOOLWINDOW | WS_EX_TOPMOST,
            class,
            w!("Desktop Buddy commands"),
            WS_POPUP,
            0,
            0,
            WIDTH,
            HEIGHT,
            Some(pet),
            None,
            Some(instance.into()),
            None,
        ) else {
            return;
        };
        let create = |class: PCWSTR, style: WINDOW_STYLE, id: isize| {
            CreateWindowExW(
                WINDOW_EX_STYLE(0),
                class,
                w!(""),
                WS_CHILD | WS_VISIBLE | WS_TABSTOP | style,
                0,
                0,
                1,
                1,
                Some(hwnd),
                Some(HMENU(id as _)),
                Some(instance.into()),
                None,
            )
        };
        let (Ok(edit), Ok(status)) = (
            create(w!("EDIT"), WINDOW_STYLE(ES_AUTOHSCROLL as u32), 1),
            create(
                w!("STATIC"),
                WINDOW_STYLE(SS_ENDELLIPSIS.0 | SS_NOPREFIX.0),
                3,
            ),
        ) else {
            let _ = DestroyWindow(hwnd);
            return;
        };
        let old_edit = std::mem::transmute::<isize, WNDPROC>(SetWindowLongPtrW(
            edit,
            GWLP_WNDPROC,
            input_proc as *const () as isize,
        ));
        let shared = Arc::new(Search {
            hwnd: AtomicIsize::new(hwnd.0 as isize),
            generation: AtomicU64::new(0),
            running: AtomicBool::new(false),
            pending: Mutex::new(None),
            result: Mutex::new(None),
            copy_result: Mutex::new(None),
        });
        BAR.with(|s| {
            *s.borrow_mut() = Some(Bar {
                app: app.clone(),
                pet,
                hwnd,
                edit,
                status,
                old_edit,
                choices: vec![],
                selected: 0,
                search: shared,
                font: HFONT::default(),
                dpi: 0,
                composing: false,
                finder: false,
            })
        });
        SendMessageW(edit, EM_SETLIMITTEXT, Some(WPARAM(1024)), None);
        let cue = w!("Search files or browse commands");
        SendMessageW(edit, 0x1501, Some(WPARAM(1)), Some(LPARAM(cue.0 as isize)));
    }
    let Some((hwnd, edit)) = get(|b| (b.hwnd, b.edit)) else {
        return;
    };
    position(hwnd, pet, HEIGHT);
    layout(hwnd);
    refresh();
    let _ = ShowWindow(hwnd, SW_SHOW);
    let _ = SetForegroundWindow(hwnd);
    let _ = SetFocus(Some(edit));
    crate::pet::bar_visibility(app, IsWindowVisible(hwnd).as_bool());
}

unsafe fn position(hwnd: HWND, pet: HWND, height: i32) {
    let mut anchor = RECT::default();
    let _ = GetWindowRect(pet, &mut anchor);
    let dpi = GetDpiForWindow(pet).max(96) as i32;
    let (width, height) = (WIDTH * dpi / 96, height * dpi / 96);
    let monitor = MonitorFromWindow(pet, MONITOR_DEFAULTTONEAREST);
    let mut info = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    let _ = GetMonitorInfoW(monitor, &mut info);
    let r = info.rcWork;
    let x = ((anchor.left + anchor.right - width) / 2).clamp(r.left, (r.right - width).max(r.left));
    let y = if anchor.bottom + 6 + height <= r.bottom {
        anchor.bottom + 6
    } else {
        (anchor.top - height - 6).max(r.top)
    };
    let _ = SetWindowPos(
        hwnd,
        Some(HWND_TOPMOST),
        x,
        y,
        width,
        height,
        SWP_NOACTIVATE,
    );
}
