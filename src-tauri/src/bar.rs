//! Native command bar; detailed management windows are opened only on request.
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
            Controls::*,
            HiDpi::GetDpiForWindow,
            Input::KeyboardAndMouse::*,
            WindowsAndMessaging::*,
        },
    },
};
const DONE: u32 = WM_APP + 30;
const COPIED: u32 = WM_APP + 31;
const OUTSIDE_CLICK: u32 = WM_APP + 32;
const WIDTH: i32 = 500;
const FILE_WIDTH: i32 = 680;
const HEIGHT: i32 = 56;
const ROW_HEIGHT: i32 = 48;
const VISIBLE_ROWS: usize = 5;
#[derive(Clone)]
enum Action {
    Input(String),
    Complete(&'static str),
    Tools,
    Panel(commands::Destination),
    Todos,
    Held,
    Imports,
    Feedback,
    Preview(usize),
    ShowPet,
    HidePet,
    ResetPet,
    Copy,
    Pet(crate::pet_art::Appearance),
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
    click_hook: Option<HHOOK>,
    old_edit: WNDPROC,
    choices: Vec<(String, Action)>,
    selected: usize,
    search: Arc<Search>,
    font: HFONT,
    dpi: u32,
    composing: bool,
    finder: bool,
    file_search: bool,
    menu_active: bool,
    menu_theme: Option<crate::menu_theme::MenuTheme>,
}
impl Bar {
    fn visible_rows(&self) -> usize {
        let limit = if self.file_search { 8 } else { VISIBLE_ROWS };
        let area = unsafe { work_area(self.pet) };
        let scale = unsafe { GetDpiForWindow(self.pet).max(96) as i32 };
        let available = (area.bottom - area.top) * 96 / scale;
        limit.min(((available - HEIGHT - 24) / ROW_HEIGHT).max(1) as usize)
    }
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
    if let Some((hwnd, edit, status)) = get(|b| (b.hwnd, b.edit, b.status)) {
        let value = wide(text);
        let empty = GetWindowTextLengthW(edit) == 0;
        let cue = wide(if empty { text } else { "" });
        SendMessageW(
            edit,
            0x1501,
            Some(WPARAM(1)),
            Some(LPARAM(cue.as_ptr() as isize)),
        );
        let inline = !empty && !text.is_empty();
        let _ = SetWindowTextW(
            status,
            if inline {
                PCWSTR(value.as_ptr())
            } else {
                w!("")
            },
        );
        let _ = ShowWindow(status, if inline { SW_SHOWNOACTIVATE } else { SW_HIDE });
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
    if GetWindowTextLengthW(get(|bar| bar.edit).unwrap_or_default()) > 0 {
        note("");
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
    change(|b| {
        b.finder = false;
        b.file_search = false;
    });
    if text.is_empty() {
        choices(
            commands::suggestions("")
                .into_iter()
                .map(|(hint, completion)| (hint.into(), Action::Complete(completion)))
                .collect(),
        );
        change(|b| b.selected = usize::MAX);
        note("");
        return;
    }
    let command = if !text.trim().is_empty() && !text.trim_start().starts_with('/') {
        Ok(Parsed::Find(text.trim().into()))
    } else {
        commands::parse(&text)
    };
    if let Some((_, hint, completion)) =
        commands::definitions()
            .into_iter()
            .find(|(aliases, _, completion)| {
                *completion != "/f "
                    && completion.ends_with(' ')
                    && aliases
                        .iter()
                        .any(|alias| text.trim().eq_ignore_ascii_case(alias))
            })
    {
        choices(vec![(String::new(), Action::Complete(completion))]);
        note(hint);
        return;
    }
    if !matches!(
        command,
        Ok(Parsed::Tools
            | Parsed::Panel(_)
            | Parsed::Todos
            | Parsed::Held
            | Parsed::Imports
            | Parsed::Copy
            | Parsed::Pets
            | Parsed::Find(_))
    ) {
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
                note("/t /d /r /f /w /pet");
            }
        }
        Ok(Parsed::Find(q)) => {
            change(|b| {
                b.finder = true;
                b.file_search = true;
            });
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
        Ok(Parsed::Panel(destination)) => {
            choices(vec![(String::new(), Action::Panel(destination))])
        }
        Ok(Parsed::Todos) => choices(vec![(String::new(), Action::Todos)]),
        Ok(Parsed::Imports) => choices(vec![(String::new(), Action::Imports)]),
        Ok(Parsed::Held) => {
            change(|bar| bar.finder = true);
            let files = pet::held_paths();
            choices(
                files
                    .into_iter()
                    .map(|path| {
                        let name = path.file_name().unwrap_or_default().to_string_lossy();
                        (
                            format!("{}  \u{b7}  {}", name, path.display()),
                            if path.is_dir() {
                                Action::Open(path.display().to_string())
                            } else {
                                Action::OpenFile(path.display().to_string())
                            },
                        )
                    })
                    .collect(),
            );
            if get(|bar| bar.choices.is_empty()).unwrap_or(true) {
                note("The pouch is empty. Drop files onto your pet.");
            }
        }
        Ok(Parsed::Copy) => choices(vec![(String::new(), Action::Copy)]),
        Ok(Parsed::Pets) => {
            use crate::pet_art::Appearance::*;
            let current = pet::appearance();
            change(|bar| bar.finder = true);
            choices(
                [(Calico, "Calico"), (Dog, "Shiba Inu"), (Panda, "Panda")]
                    .into_iter()
                    .map(|(pet, label)| {
                        let label = if pet == current {
                            format!("{label} · Current")
                        } else {
                            label.into()
                        };
                        (label, Action::Pet(pet))
                    })
                    .collect(),
            );
        }
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
pub fn repaint_theme() {
    if let Some(hwnd) = get(|bar| bar.hwnd) {
        unsafe {
            let _ = RedrawWindow(Some(hwnd), None, None, RDW_INVALIDATE | RDW_ALLCHILDREN);
        }
    }
}
unsafe fn hide() {
    if let Some((h, hook, search)) = change(|b| (b.hwnd, b.click_hook.take(), b.search.clone())) {
        if let Some(hook) = hook {
            let _ = UnhookWindowsHookEx(hook);
        }
        search.generation.fetch_add(1, Ordering::Relaxed);
        let _ = ShowWindow(h, SW_HIDE);
        if let Some(app) = get(|b| b.app.clone()) {
            crate::pet::bar_visibility(&app, false);
        }
    }
}

unsafe extern "system" fn outside_mouse(code: i32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if code >= 0
        && matches!(
            wp.0 as u32,
            WM_LBUTTONDOWN | WM_RBUTTONDOWN | WM_MBUTTONDOWN | WM_XBUTTONDOWN
        )
    {
        let point = (*(lp.0 as *const MSLLHOOKSTRUCT)).pt;
        BAR.with(|state| {
            if let Ok(state) = state.try_borrow() {
                if let Some(bar) = state.as_ref() {
                    let hit = WindowFromPoint(point);
                    let menu_click = if bar.menu_active {
                        let mut class = [0u16; 32];
                        let length = GetClassNameW(hit, &mut class).max(0) as usize;
                        String::from_utf16_lossy(&class[..length]) == "#32768"
                    } else {
                        false
                    };
                    let inside = menu_click
                        || hit == bar.hwnd
                        || (hit == bar.pet && wp.0 as u32 == WM_LBUTTONDOWN);
                    if !inside && !IsChild(bar.hwnd, hit).as_bool() {
                        let _ = PostMessageW(Some(bar.hwnd), OUTSIDE_CLICK, WPARAM(0), LPARAM(0));
                    }
                }
            }
        });
    }
    CallNextHookEx(None, code, wp, lp)
}
fn open_tools(app: tauri::AppHandle) {
    crate::navigation::open(app, commands::Destination::Activity);
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
    let opens_path = matches!(
        &action,
        Action::Open(_) | Action::OpenFile(_) | Action::Editor(_)
    );
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
        Action::Panel(destination) => {
            hide();
            crate::navigation::open(app, destination);
            return;
        }
        Action::Todos => {
            hide();
            crate::navigation::todo(app, pet::appearance().palette());
            return;
        }
        Action::Held => {
            fill("/held");
            return;
        }
        Action::Imports => {
            let path = app.state::<Shared>().data_path.join("drag-cache");
            std::fs::create_dir_all(&path)
                .map_err(err)
                .and_then(|_| open_path(path.display().to_string()))
                .map(|_| "Imported files folder opened.".to_string())
        }
        Action::Feedback => return,
        Action::Preview(index) => {
            hide();
            pet::preview(&app, index);
            return;
        }
        Action::ShowPet => pet::pet_show(app.clone()).map(|_| "Pet shown.".to_string()),
        Action::HidePet => {
            hide();
            let _ = pet::pet_hide(app);
            return;
        }
        Action::ResetPet => pet::reset(&app).map(|_| "Pet position reset.".to_string()),
        Action::Pet(appearance) => {
            pet::set_appearance(&app, appearance);
            fill("");
            note("Pet selected");
            return;
        }
        Action::Open(path) => open_path(path).map(|_| "Folder opened.".to_string()),
        Action::OpenFile(path) => {
            let path = wide(&path);
            let result = windows::Win32::UI::Shell::ShellExecuteW(
                None,
                w!("open"),
                PCWSTR(path.as_ptr()),
                None,
                None,
                SW_SHOWNORMAL,
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
            if opens_path {
                pet::cue(&app, crate::pet_art::Cue::FileOpened);
            }
            fill("");
            choices(vec![]);
            note(&message);
        }
        Err(e) => {
            change(|bar| bar.finder = true);
            choices(vec![(e, Action::Feedback)]);
        }
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
                || aliases
                    .iter()
                    .any(|alias| finder::match_rank(alias.trim_start_matches('/'), query).is_some())
        })
        .map(|(_, label, completion)| {
            let action = match completion {
                "/tools" | "/actions" => Action::Tools,
                value => match commands::parse(value) {
                    Ok(Parsed::Panel(destination)) => Action::Panel(destination),
                    Ok(Parsed::Todos) => Action::Todos,
                    Ok(Parsed::Held) => Action::Held,
                    Ok(Parsed::Imports) => Action::Imports,
                    _ => Action::Complete(completion),
                },
            };
            (label.into(), action)
        })
        .collect()
}
unsafe fn show_actions() {
    let Some(hwnd) = get(|bar| bar.hwnd) else {
        return;
    };
    let Ok(menu) = CreatePopupMenu() else {
        return;
    };
    let mut actions = Vec::new();
    unsafe fn add(menu: HMENU, actions: &mut Vec<Action>, label: &str, action: Action) {
        actions.push(action);
        let text = wide(label);
        let _ = AppendMenuW(menu, MF_STRING, actions.len(), PCWSTR(text.as_ptr()));
    }
    unsafe fn command(menu: HMENU, actions: &mut Vec<Action>, completion: &str) {
        let Some((_, label, _)) = commands::definitions()
            .into_iter()
            .find(|(_, _, value)| *value == completion)
        else {
            return;
        };
        let title = label
            .split_once(" \u{b7} ")
            .map(|(_, title)| title)
            .unwrap_or(label);
        let action = match commands::parse(completion) {
            Ok(Parsed::Panel(destination)) => Action::Panel(destination),
            Ok(Parsed::Todos) => Action::Todos,
            Ok(Parsed::Held) => Action::Held,
            Ok(Parsed::Imports) => Action::Imports,
            _ => Action::Complete(
                commands::definitions()
                    .into_iter()
                    .find(|(_, _, value)| *value == completion)
                    .unwrap()
                    .2,
            ),
        };
        add(
            menu,
            actions,
            &format!("{}\t{}", title, completion.trim()),
            action,
        );
    }
    unsafe fn attach(menu: HMENU, child: HMENU, label: &str) {
        let text = wide(label);
        if AppendMenuW(menu, MF_POPUP, child.0 as usize, PCWSTR(text.as_ptr())).is_err() {
            let _ = DestroyMenu(child);
        }
    }
    for completion in ["/todos", "/ask", "/resume", "/fill"] {
        command(menu, &mut actions, completion);
    }
    let _ = AppendMenuW(menu, MF_SEPARATOR, 0, None);
    for (label, completions) in [
        (
            "Tasks and focus",
            &["/t ", "/d ", "/r ", "/timers", "/reminders"][..],
        ),
        (
            "Files and projects",
            &["/f ", "/w ", "/projects", "/shelf", "/held", "/imports"][..],
        ),
    ] {
        if let Ok(child) = CreatePopupMenu() {
            for completion in completions {
                command(child, &mut actions, completion);
            }
            if label == "Files and projects" {
                add(child, &mut actions, "Copy held files", Action::Copy);
            }
            attach(menu, child, label);
        }
    }
    if let Ok(child) = CreatePopupMenu() {
        command(child, &mut actions, "/pet");
        add(child, &mut actions, "Show pet", Action::ShowPet);
        add(child, &mut actions, "Hide pet", Action::HidePet);
        add(child, &mut actions, "Reset pet position", Action::ResetPet);
        if let Ok(motion) = CreatePopupMenu() {
            for (index, set) in pet::appearance().animation_sets().iter().enumerate() {
                add(motion, &mut actions, set.name, Action::Preview(index));
            }
            attach(child, motion, "Try a motion");
        }
        command(child, &mut actions, "/settings");
        attach(menu, child, "Pet and appearance");
    }
    for completion in ["/music", "/codex"] {
        command(menu, &mut actions, completion);
    }
    let _ = AppendMenuW(menu, MF_SEPARATOR, 0, None);
    add(menu, &mut actions, "Open tools", Action::Tools);
    let mut rect = RECT::default();
    let _ = GetWindowRect(hwnd, &mut rect);
    change(|bar| bar.menu_active = true);
    let selected = crate::menu_theme::track(
        menu,
        hwnd,
        rect.left,
        rect.bottom,
        pet::appearance().palette(),
    );
    change(|bar| bar.menu_active = false);
    if let Some(action) = selected
        .checked_sub(1)
        .and_then(|index| actions.get(index as usize))
        .cloned()
    {
        choices(vec![(String::new(), action)]);
        execute();
    }
}
unsafe fn draw_result_icon(
    dc: HDC,
    left: i32,
    top: i32,
    dpi: u32,
    action: &Action,
    color: COLORREF,
) {
    let s = |value: i32| value * dpi as i32 / 96;
    let pen = CreatePen(PS_SOLID, s(1).max(1), color);
    let old_pen = SelectObject(dc, pen.into());
    let old_brush = SelectObject(dc, GetStockObject(NULL_BRUSH));
    let line = |x1, y1, x2, y2| {
        let _ = MoveToEx(dc, left + s(x1), top + s(y1), None);
        let _ = LineTo(dc, left + s(x2), top + s(y2));
    };
    match action {
        Action::Open(_) => {
            line(2, 7, 2, 19);
            line(2, 19, 23, 19);
            line(23, 19, 23, 7);
            line(23, 7, 11, 7);
            line(11, 7, 8, 4);
            line(8, 4, 2, 4);
            line(2, 4, 2, 7);
        }
        Action::OpenFile(_) => {
            line(5, 2, 16, 2);
            line(16, 2, 22, 8);
            line(22, 8, 22, 22);
            line(22, 22, 5, 22);
            line(5, 22, 5, 2);
            line(16, 2, 16, 8);
            line(16, 8, 22, 8);
            line(9, 13, 18, 13);
            line(9, 17, 18, 17);
        }
        Action::Pet(_) => {
            let _ = Ellipse(dc, left + s(7), top + s(11), left + s(19), top + s(21));
            for (x, y) in [(3, 7), (9, 2), (15, 2), (21, 7)] {
                let _ = Ellipse(dc, left + s(x), top + s(y), left + s(x + 4), top + s(y + 5));
            }
        }
        _ => {
            line(5, 18, 19, 4);
            line(9, 4, 19, 4);
            line(19, 4, 19, 14);
        }
    }
    SelectObject(dc, old_brush);
    SelectObject(dc, old_pen);
    let _ = DeleteObject(pen.into());
}
unsafe fn layout(hwnd: HWND) {
    let dpi = GetDpiForWindow(hwnd).max(96);
    let s = |v: i32| v * dpi as i32 / 96;
    let Some((edit, status, pet, rows, file_search)) = get(|b| {
        (
            b.edit,
            b.status,
            b.pet,
            if b.finder {
                b.choices.len().min(b.visible_rows())
            } else {
                0
            },
            b.file_search,
        )
    }) else {
        return;
    };
    let has_note = GetWindowTextLengthW(status) > 0;
    let mut rect = RECT::default();
    let _ = GetClientRect(hwnd, &mut rect);
    let height = HEIGHT + rows as i32 * ROW_HEIGHT + if rows > 0 { 8 } else { 0 };
    let pet_dpi = GetDpiForWindow(pet).max(96) as i32;
    let width = if file_search { FILE_WIDTH } else { WIDTH };
    if rect.bottom != height * pet_dpi / 96 || rect.right != fitted_width(pet, width) {
        position(hwnd, pet, width, height);
        let _ = GetClientRect(hwnd, &mut rect);
        let _ = InvalidateRect(Some(hwnd), None, true);
    }
    let width = rect.right;
    let start = s(16);
    let end = width - s(16);
    let status_width = if has_note { s(130).min(width / 3) } else { 0 };
    let _ = SetWindowPos(
        edit,
        None,
        start,
        s(18),
        (end - start - status_width).max(1),
        s(22),
        SWP_NOZORDER,
    );
    let _ = SetWindowPos(
        status,
        None,
        end - status_width + s(8),
        s(18),
        (status_width - s(8)).max(1),
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
unsafe fn browse(forward: bool) -> bool {
    let changed = change(|bar| {
        let count = bar.choices.len();
        if count == 0 {
            return false;
        }
        bar.selected = if bar.selected >= count {
            if forward {
                0
            } else {
                count - 1
            }
        } else {
            (bar.selected + if forward { 1 } else { count - 1 }) % count
        };
        true
    })
    .unwrap_or(false);
    if changed {
        show_choice();
    }
    changed
}

unsafe fn theme_edit_menu(menu: HMENU) {
    change(|bar| {
        if !bar.menu_active {
            return;
        }
        let palette = pet::appearance().palette();
        if let Some(theme) = &mut bar.menu_theme {
            theme.apply(menu, bar.edit, palette, GetDpiForWindow(bar.edit).max(96));
        } else {
            bar.menu_theme = crate::menu_theme::prepare(menu, bar.edit, palette);
        }
    });
}

unsafe extern "system" fn input_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if let Some(result) = crate::menu_theme::handle_message(msg, wp, lp) {
        return result;
    }
    if msg == WM_CONTEXTMENU {
        change(|bar| bar.menu_active = true);
        let original = get(|bar| bar.old_edit).flatten();
        let result = CallWindowProcW(original, hwnd, msg, wp, lp);
        change(|bar| {
            bar.menu_active = false;
            bar.menu_theme = None;
        });
        return result;
    }
    if msg == WM_INITMENUPOPUP {
        theme_edit_menu(HMENU(wp.0 as _));
    }
    if msg == WM_IME_STARTCOMPOSITION {
        change(|b| b.composing = true);
    }
    if msg == WM_IME_ENDCOMPOSITION {
        change(|b| b.composing = false);
    }
    let composing = get(|b| b.composing).unwrap_or(false);
    if msg == WM_MOUSEWHEEL && !composing && browse(((wp.0 >> 16) as i16) < 0) {
        return LRESULT(0);
    }
    if msg == WM_KEYDOWN && !composing && wp.0 as u16 == VK_F4.0 {
        show_actions();
        return LRESULT(0);
    }
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
                if browse(x == VK_DOWN.0) {
                    return LRESULT(0);
                }
            }
            _ => {}
        }
    }
    let original = get(|b| b.old_edit).flatten();
    let result = CallWindowProcW(original, hwnd, msg, wp, lp);
    if msg == WM_PAINT && GetWindowTextLengthW(hwnd) == 0 {
        let mut cue = [0u16; 1025];
        SendMessageW(
            hwnd,
            0x1502,
            Some(WPARAM(cue.as_mut_ptr() as usize)),
            Some(LPARAM(cue.len() as isize)),
        );
        let length = cue
            .iter()
            .position(|value| *value == 0)
            .unwrap_or(cue.len());
        if length > 0 {
            let dc = GetDC(Some(hwnd));
            let palette = pet::appearance().palette();
            let mut bounds = RECT::default();
            let _ = GetClientRect(hwnd, &mut bounds);
            let brush = CreateSolidBrush(palette.surface);
            let _ = FillRect(dc, &bounds, brush);
            SetBkMode(dc, TRANSPARENT);
            SetTextColor(dc, palette.accent);
            let font = get(|bar| bar.font).unwrap_or_default();
            let old = SelectObject(dc, font.into());
            DrawTextW(
                dc,
                &mut cue[..length],
                &mut bounds,
                DT_SINGLELINE | DT_END_ELLIPSIS | DT_NOPREFIX,
            );
            SelectObject(dc, old);
            let _ = DeleteObject(brush.into());
            ReleaseDC(Some(hwnd), dc);
        }
    }
    result
}
unsafe fn paint(hwnd: HWND) {
    let mut paint = PAINTSTRUCT::default();
    let dc = BeginPaint(hwnd, &mut paint);
    let dpi = GetDpiForWindow(hwnd).max(96) as i32;
    let mut rect = RECT::default();
    let _ = GetClientRect(hwnd, &mut rect);
    let palette = pet::appearance().palette();
    get(|bar| {
        render_bar(
            dc,
            rect,
            dpi as u32,
            palette,
            bar.font,
            &bar.choices,
            bar.selected,
            bar.visible_rows(),
            bar.finder,
        )
    });
    let _ = EndPaint(hwnd, &paint);
}
unsafe fn render_bar(
    dc: HDC,
    rect: RECT,
    dpi: u32,
    palette: crate::pet_art::Palette,
    font: HFONT,
    rows: &[(String, Action)],
    selected: usize,
    visible_rows: usize,
    finder: bool,
) {
    let scale = |value: i32| value * dpi as i32 / 96;
    let surface = CreateSolidBrush(palette.surface);
    let outline = CreatePen(PS_SOLID, 1, palette.outline);
    let _ = FillRect(dc, &rect, surface);
    let old_brush = SelectObject(dc, surface.into());
    let old_pen = SelectObject(dc, outline.into());
    let _ = RoundRect(dc, 0, 0, rect.right, rect.bottom, scale(22), scale(22));
    if rect.bottom > scale(HEIGHT) {
        SelectObject(dc, outline.into());
        let _ = MoveToEx(dc, scale(12), scale(HEIGHT), None);
        let _ = LineTo(dc, rect.right - scale(12), scale(HEIGHT));
    }
    SelectObject(dc, old_brush);
    SelectObject(dc, old_pen);
    {
        let previous_font = SelectObject(dc, font.into());
        SetBkMode(dc, TRANSPARENT);
        if finder {
            let first = selected / visible_rows * visible_rows;
            for (index, (label, action)) in rows.iter().enumerate().skip(first).take(visible_rows) {
                let top = HEIGHT + (index - first) as i32 * ROW_HEIGHT;
                let mut bounds = RECT {
                    left: scale(12),
                    top: scale(top + 3),
                    right: rect.right - scale(12),
                    bottom: scale(top + ROW_HEIGHT - 2),
                };
                if index == selected {
                    let highlight = CreateSolidBrush(palette.selection);
                    let old = SelectObject(dc, highlight.into());
                    let pen = SelectObject(dc, GetStockObject(NULL_PEN));
                    let _ = RoundRect(
                        dc,
                        bounds.left,
                        bounds.top,
                        bounds.right,
                        bounds.bottom,
                        scale(9),
                        scale(9),
                    );
                    SelectObject(dc, pen);
                    SelectObject(dc, old);
                    let _ = DeleteObject(highlight.into());
                }
                SetTextColor(
                    dc,
                    if index == selected {
                        palette.accent
                    } else {
                        palette.ink
                    },
                );
                bounds.left += scale(8);
                bounds.right -= scale(8);
                draw_result_icon(
                    dc,
                    bounds.left,
                    (bounds.top + bounds.bottom - scale(22)) / 2,
                    dpi as u32,
                    action,
                    palette.accent,
                );
                bounds.left += scale(40);
                if let Some((name, detail)) = label.split_once("  ·  ") {
                    let mut title_rect = RECT {
                        bottom: bounds.top + scale(16),
                        ..bounds
                    };
                    let mut title: Vec<_> = name.encode_utf16().collect();
                    DrawTextW(
                        dc,
                        &mut title,
                        &mut title_rect,
                        DT_SINGLELINE | DT_END_ELLIPSIS | DT_NOPREFIX,
                    );
                    bounds.top += scale(15);
                    SetTextColor(dc, palette.accent);
                    let mut detail: Vec<_> = detail.encode_utf16().collect();
                    DrawTextW(
                        dc,
                        &mut detail,
                        &mut bounds,
                        DT_SINGLELINE | DT_END_ELLIPSIS | DT_NOPREFIX,
                    );
                } else {
                    let mut label: Vec<_> = label.encode_utf16().collect();
                    DrawTextW(
                        dc,
                        &mut label,
                        &mut bounds,
                        DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS | DT_NOPREFIX,
                    );
                }
            }
        }
        SelectObject(dc, previous_font);
    }
    let _ = DeleteObject(outline.into());
    let _ = DeleteObject(surface.into());
}

unsafe extern "system" fn proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if let Some(result) = crate::menu_theme::handle_message(msg, wp, lp) {
        return result;
    }
    if msg == WM_INITMENUPOPUP {
        theme_edit_menu(HMENU(wp.0 as _));
    }
    match msg {
        OUTSIDE_CLICK => {
            hide();
            LRESULT(0)
        }
        WM_MOUSEWHEEL => {
            browse(((wp.0 >> 16) as i16) < 0);
            LRESULT(0)
        }
        WM_PAINT => {
            paint(hwnd);
            LRESULT(0)
        }
        WM_LBUTTONUP => {
            let dpi = GetDpiForWindow(hwnd).max(96) as i32;
            let y = (lp.0 >> 16) as i16 as i32 * 96 / dpi;
            if y >= HEIGHT
                && change(|bar| {
                    let visible_rows = bar.visible_rows();
                    let first = bar.selected / visible_rows * visible_rows;
                    let index = first + ((y - HEIGHT) / ROW_HEIGHT) as usize;
                    if bar.finder && index < bar.choices.len() {
                        bar.selected = index;
                        true
                    } else {
                        false
                    }
                })
                .unwrap_or(false)
            {
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
            let palette = pet::appearance().palette();
            SetTextColor(
                dc,
                if msg == WM_CTLCOLORSTATIC {
                    palette.accent
                } else {
                    palette.ink
                },
            );
            SetBkColor(dc, palette.surface);
            SetDCBrushColor(dc, palette.surface);
            LRESULT(GetStockObject(DC_BRUSH).0 as isize)
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
                rows.extend(
                    packet
                        .results
                        .into_iter()
                        .filter(|result| !project_paths.contains(&result.path.to_lowercase()))
                        .map(|result| {
                            let parent = Path::new(&result.path)
                                .parent()
                                .map(|path| path.display().to_string())
                                .unwrap_or_default();
                            let kind = if result.folder {
                                "Folder".to_owned()
                            } else {
                                Path::new(&result.path)
                                    .extension()
                                    .and_then(|value| value.to_str())
                                    .filter(|value| !value.is_empty())
                                    .map(|value| format!("{} file", value.to_ascii_uppercase()))
                                    .unwrap_or_else(|| "File".into())
                            };
                            let label = format!("{}  ·  {kind}  ·  {parent}", result.name);
                            let action = if result.folder {
                                Action::Open(result.path)
                            } else {
                                Action::OpenFile(result.path)
                            };
                            (label, action)
                        }),
                );
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
        RegisterClassW(&WNDCLASSW {
            lpfnWndProc: Some(proc),
            hInstance: instance.into(),
            lpszClassName: class,
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
            None,
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
                click_hook: None,
                old_edit,
                choices: vec![],
                selected: 0,
                search: shared,
                font: HFONT::default(),
                dpi: 0,
                composing: false,
                finder: false,
                file_search: false,
                menu_active: false,
                menu_theme: None,
            })
        });
        SendMessageW(edit, EM_SETLIMITTEXT, Some(WPARAM(1024)), None);
    }
    let Some((hwnd, edit)) = get(|b| (b.hwnd, b.edit)) else {
        return;
    };
    fill("");
    position(hwnd, pet, WIDTH, HEIGHT);
    layout(hwnd);
    refresh();
    let _ = ShowWindow(hwnd, SW_SHOW);
    let _ = SetForegroundWindow(hwnd);
    let _ = SetFocus(Some(edit));
    let module = GetModuleHandleW(None).ok().map(Into::into);
    let hook = SetWindowsHookExW(WH_MOUSE_LL, Some(outside_mouse), module, 0).ok();
    change(|bar| bar.click_hook = hook);
    crate::pet::bar_visibility(app, IsWindowVisible(hwnd).as_bool());
}

unsafe fn work_area(pet: HWND) -> RECT {
    let monitor = MonitorFromWindow(pet, MONITOR_DEFAULTTONEAREST);
    let mut info = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    let _ = GetMonitorInfoW(monitor, &mut info);
    info.rcWork
}

unsafe fn fitted_width(pet: HWND, width: i32) -> i32 {
    let area = work_area(pet);
    (width * GetDpiForWindow(pet).max(96) as i32 / 96).min((area.right - area.left).max(1))
}

unsafe fn position(hwnd: HWND, pet: HWND, width: i32, height: i32) {
    let mut anchor = RECT::default();
    let _ = GetWindowRect(pet, &mut anchor);
    let dpi = GetDpiForWindow(pet).max(96) as i32;
    let height = height * dpi / 96;
    let r = work_area(pet);
    let width = fitted_width(pet, width);
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn render_toolbar_palettes_and_scaled_results_without_desktop_input() {
        unsafe {
            let width = 2280i32;
            let height = 930i32;
            let dc = CreateCompatibleDC(None);
            let info = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: 40,
                    biWidth: width,
                    biHeight: -height,
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0,
                    ..Default::default()
                },
                ..Default::default()
            };
            let mut bits = std::ptr::null_mut();
            let bitmap =
                CreateDIBSection(Some(dc), &info, DIB_RGB_COLORS, &mut bits, None, 0).unwrap();
            let old = SelectObject(dc, bitmap.into());
            let mut x = 0;
            for dpi in [96, 144, 192] {
                let s = |value: i32| value * dpi as i32 / 96;
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
                for (row, pet) in [
                    crate::pet_art::Appearance::Calico,
                    crate::pet_art::Appearance::Dog,
                    crate::pet_art::Appearance::Panda,
                ]
                .into_iter()
                .enumerate()
                {
                    let palette = pet.palette();
                    let _ = SetViewportOrgEx(dc, x, row as i32 * 310, None);
                    let rows = vec![
                        (
                            "Project notes.pdf  ·  PDF file  ·  C:\\Projects\\Study".into(),
                            Action::OpenFile("notes.pdf".into()),
                        ),
                        (
                            "Algorithms  ·  Project  ·  C:\\Projects\\Algorithms".into(),
                            Action::Open("C:\\Projects\\Algorithms".into()),
                        ),
                    ];
                    render_bar(
                        dc,
                        RECT {
                            left: 0,
                            top: 0,
                            right: s(WIDTH),
                            bottom: s(HEIGHT + ROW_HEIGHT * 2 + 8),
                        },
                        dpi,
                        palette,
                        font,
                        &rows,
                        0,
                        5,
                        true,
                    );
                    let previous_font = SelectObject(dc, font.into());
                    let mut input = RECT {
                        left: s(16),
                        top: s(18),
                        right: s(WIDTH - 16),
                        bottom: s(42),
                    };
                    let mut text: Vec<_> = "notes".encode_utf16().collect();
                    SetTextColor(dc, palette.ink);
                    DrawTextW(dc, &mut text, &mut input, DT_SINGLELINE | DT_NOPREFIX);
                    SelectObject(dc, previous_font);
                }
                let _ = DeleteObject(font.into());
                x += s(WIDTH) + 10;
            }
            if let Some(path) = std::env::var_os("BUDDY_TOOLBAR_PREVIEW") {
                let bytes =
                    std::slice::from_raw_parts(bits as *const u8, (width * height * 4) as usize);
                let mut bmp = Vec::new();
                bmp.extend_from_slice(b"BM");
                bmp.extend_from_slice(&((54 + bytes.len()) as u32).to_le_bytes());
                bmp.extend_from_slice(&[0; 4]);
                bmp.extend_from_slice(&54u32.to_le_bytes());
                bmp.extend_from_slice(&40u32.to_le_bytes());
                bmp.extend_from_slice(&width.to_le_bytes());
                bmp.extend_from_slice(&(-height).to_le_bytes());
                bmp.extend_from_slice(&1u16.to_le_bytes());
                bmp.extend_from_slice(&32u16.to_le_bytes());
                bmp.extend_from_slice(&[0; 24]);
                bmp.extend_from_slice(bytes);
                std::fs::write(path, bmp).unwrap();
            }
            SelectObject(dc, old);
            let _ = DeleteObject(bitmap.into());
            let _ = DeleteDC(dc);
        }
    }
}
