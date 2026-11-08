#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod agents;
mod bar;
mod commands;
mod drop_data;
mod finder;
mod model;
mod navigation;
mod browser_protocol;
mod browser_fill;
mod screen_helper;
mod menu_theme;
mod pet;
mod pet_art;
use model::*;
use rusqlite::{params, Connection};
use serde::Serialize;
use std::{
    io::{BufRead, BufReader, Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{Arc, Condvar, Mutex},
    time::Duration,
};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager,
};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};
use tauri_plugin_notification::NotificationExt;

struct Core {
    db: Mutex<Connection>,
    revision: Mutex<u64>,
    wake: Condvar,
    run: String,
    data_path: PathBuf,
    warning: Mutex<Option<String>>,
    workers: Mutex<Vec<Arc<Mutex<std::process::Child>>>>,
    agents: Mutex<std::collections::HashMap<String, Arc<agents::Session>>>,
}
type Shared = Arc<Core>;
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Snapshot {
    items: Vec<Item>,
    theme: String,
    data_path: String,
    warning: Option<String>,
    native: bool,
    agent_attention: usize,
    pet_appearance: String,
}

fn rows(db: &Connection) -> Result<Vec<Item>, String> {
    let mut st = db
        .prepare("SELECT payload FROM items ORDER BY created DESC")
        .map_err(err)?;
    let r = st.query_map([], |r| r.get::<_, String>(0)).map_err(err)?;
    r.map(|s| serde_json::from_str(&s.map_err(err)?).map_err(err))
        .collect()
}
fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}
fn put(db: &Connection, item: &Item) -> Result<(), String> {
    let payload = serde_json::to_string(item).map_err(err)?;
    let (count,bytes):(i64,i64)=db.query_row("SELECT COUNT(*),COALESCE(SUM(length(CAST(payload AS BLOB))),0) FROM items WHERE id!=?1",[&item.id],|r|Ok((r.get(0)?,r.get(1)?))).map_err(err)?;
    if count >= 2000 || bytes + payload.len() as i64 > 3 * 1024 * 1024 {
        return Err(
            "Local storage capacity reached. Export a backup and clear old entries.".into(),
        );
    }
    db.execute("INSERT INTO items(id,kind,created,payload) VALUES(?1,?2,?3,?4) ON CONFLICT(id) DO UPDATE SET kind=excluded.kind,payload=excluded.payload",params![item.id,item.kind,item.created_at,payload]).map_err(err)?;
    Ok(())
}
fn get(db: &Connection, id: &str) -> Result<Item, String> {
    let raw: String = db
        .query_row("SELECT payload FROM items WHERE id=?1", [id], |r| r.get(0))
        .map_err(err)?;
    serde_json::from_str(&raw).map_err(err)
}
fn changed(app: &tauri::AppHandle, c: &Core) {
    if let Ok(mut r) = c.revision.lock() {
        *r += 1;
        c.wake.notify_all();
    }
    let _ = app.emit("data-changed", ());
    pet::refresh_indicators(app);
}
#[tauri::command]
fn snapshot(core: tauri::State<Shared>) -> Result<Snapshot, String> {
    let db = core.db.lock().map_err(err)?;
    let mut items = rows(&db)?;
    for i in &mut items {
        if i.kind == "timer" && i.state == "running" {
            i.remaining_ms = remaining(i, now(), ticks(), &core.run);
        }
    }
    let theme = db
        .query_row("SELECT value FROM settings WHERE key='theme'", [], |r| {
            r.get(0)
        })
        .unwrap_or_else(|_| "dark".to_string());
    Ok(Snapshot {
        items,
        theme,
        data_path: core.data_path.display().to_string(),
        warning: core.warning.lock().map_err(err)?.clone(),
        native: true,
        agent_attention: agents::attention_count(&db)?,
        pet_appearance: db.query_row("SELECT value FROM settings WHERE key='pet_appearance'", [], |row| row.get(0)).unwrap_or_else(|_| "calico".into()),
    })
}
#[tauri::command]
fn save_item(
    app: tauri::AppHandle,
    core: tauri::State<Shared>,
    mut item: Item,
) -> Result<(), String> {
    validate(&item)?;
    if item.kind == "timer" || item.kind == "attention" || item.kind == "receipt" {
        return Err("Use the dedicated action for this item.".into());
    }
    let db = core.db.lock().map_err(err)?;
    let mut saved_cue = None;
    if let Ok(old) = get(&db, &item.id) {
        if old.kind != item.kind {
            return Err("An item's type cannot change.".into());
        }
        item.created_at = old.created_at;
    } else {
        let all = rows(&db)?;
        if all.len() >= 1900 {
            return Err("Local item limit reached. Export and clear old entries first.".into());
        }
        if item.kind == "shelf" && all.iter().filter(|i| i.kind == "shelf").count() >= 50 {
            return Err("Shelf limit is 50 entries.".into());
        }
        saved_cue = match item.kind.as_str() {
            "todo" => Some(pet_art::Cue::TodoSaved),
            "reminder" => Some(pet_art::Cue::ReminderSaved),
            _ => None,
        };
        item.id = new_id();
        item.created_at = now();
    }
    if item.kind == "reminder" && !item.done && item.due_at <= now() {
        return Err("Choose a reminder time in the future.".into());
    }
    put(&db, &item)?;
    drop(db);
    changed(&app, &core);
    if let Some(cue) = saved_cue {
        pet::cue(&app, cue);
    }
    Ok(())
}
#[tauri::command]
fn remove_item(
    app: tauri::AppHandle,
    core: tauri::State<Shared>,
    id: String,
) -> Result<(), String> {
    let db = core.db.lock().map_err(err)?;
    let i = get(&db, &id)?;
    if i.kind == "timer" && i.state == "running" {
        return Err("Pause the timer before removing it.".into());
    }
    db.execute("DELETE FROM items WHERE id=?1", [id])
        .map_err(err)?;
    drop(db);
    changed(&app, &core);
    Ok(())
}
#[tauri::command]
fn toggle_item(
    app: tauri::AppHandle,
    core: tauri::State<Shared>,
    id: String,
) -> Result<(), String> {
    let db = core.db.lock().map_err(err)?;
    let mut i = get(&db, &id)?;
    if !["todo", "attention"].contains(&i.kind.as_str()) {
        return Err("This item cannot be checked off.".into());
    }
    i.done = !i.done;
    put(&db, &i)?;
    drop(db);
    changed(&app, &core);
    if i.kind == "todo" && i.done {
        pet::cue(&app, pet_art::Cue::TodoDone);
    }
    Ok(())
}

#[tauri::command]
fn rename_todo(app: tauri::AppHandle, core: tauri::State<Shared>, id: String, title: String) -> Result<(), String> {
    let db = core.db.lock().map_err(err)?;
    rename_todo_item(&db, &id, &title)?;
    drop(db);
    changed(&app, &core);
    Ok(())
}
fn rename_todo_item(db: &Connection, id: &str, title: &str) -> Result<(), String> {
    let mut item = get(db, id)?;
    if item.kind != "todo" { return Err("This item is no longer a todo.".into()); }
    item.title = title.trim().into();
    validate(&item)?;
    put(db, &item)
}
#[cfg(test)]
fn complete_todo(db: &Connection, id: &str) -> Result<bool, String> {
    let mut item = get(db, id)?;
    if item.kind != "todo" {
        return Err("This item is no longer a to-do.".into());
    }
    if item.done {
        return Ok(false);
    }
    item.done = true;
    put(db, &item)?;
    Ok(true)
}

#[tauri::command]
fn start_timer(
    app: tauri::AppHandle,
    core: tauri::State<Shared>,
    title: String,
    minutes: u32,
    project_id: String,
) -> Result<(), String> {
    if minutes == 0 || minutes > 1440 || title.len() > 240 {
        return Err("Timer duration must be 1–1440 minutes.".into());
    }
    let db = core.db.lock().map_err(err)?;
    let all = rows(&db)?;
    if all.len() >= 1900 {
        return Err("Local item limit reached. Export and clear old entries first.".into());
    }
    if all
        .iter()
        .filter(|i| i.kind == "timer" && ["running", "paused"].contains(&i.state.as_str()))
        .count()
        >= 10
    {
        return Err("Finish a timer before adding another (limit 10).".into());
    }
    let duration = minutes as i64 * 60_000;
    let i = Item {
        id: new_id(),
        kind: "timer".into(),
        title: if title.trim().is_empty() {
            "Focus session".into()
        } else {
            title
        },
        created_at: now(),
        duration_ms: duration,
        remaining_ms: duration,
        due_at: now() + duration,
        started_tick: ticks(),
        run: core.run.clone(),
        state: "running".into(),
        project_id,
        ..Default::default()
    };
    put(&db, &i)?;
    drop(db);
    changed(&app, &core);
    pet::cue(&app, pet_art::Cue::FocusStart);
    Ok(())
}
#[tauri::command]
fn timer_action(
    app: tauri::AppHandle,
    core: tauri::State<Shared>,
    id: String,
    action: String,
) -> Result<(), String> {
    let db = core.db.lock().map_err(err)?;
    let mut i = get(&db, &id)?;
    if i.kind != "timer" {
        return Err("Not a timer.".into());
    }
    let cue = match action.as_str() {
        "pause" if i.state == "running" => {
            i.remaining_ms = remaining(&i, now(), ticks(), &core.run);
            i.state = "paused".into();
            pet_art::Cue::FocusPause
        }
        "resume" if i.state == "paused" => {
            i.state = "running".into();
            i.due_at = now() + i.remaining_ms;
            i.started_tick = ticks();
            i.run = core.run.clone();
            pet_art::Cue::FocusResume
        }
        "reset" => {
            i.state = "paused".into();
            i.remaining_ms = i.duration_ms;
            pet_art::Cue::FocusPause
        }
        _ => return Err("Timer action is not valid for its current state.".into()),
    };
    put(&db, &i)?;
    drop(db);
    changed(&app, &core);
    pet::cue(&app, cue);
    Ok(())
}
#[tauri::command]
fn set_theme(
    app: tauri::AppHandle,
    core: tauri::State<Shared>,
    theme: String,
) -> Result<(), String> {
    if !["dark", "light", "system"].contains(&theme.as_str()) {
        return Err("Unknown theme".into());
    }
    core.db.lock().map_err(err)?.execute("INSERT INTO settings(key,value) VALUES('theme',?1) ON CONFLICT(key) DO UPDATE SET value=excluded.value",[theme]).map_err(err)?;
    changed(&app, &core);
    Ok(())
}
#[tauri::command]
fn open_path(path: String) -> Result<(), String> {
    let p = Path::new(&path);
    if !p.is_absolute() || !p.exists() {
        return Err("This local path does not exist. Update it in Projects.".into());
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        Command::new("explorer.exe")
            .arg(p)
            .creation_flags(0x08000000)
            .spawn()
            .map_err(err)?;
    }
    Ok(())
}
#[derive(Serialize)]
struct SearchResult {
    name: String,
    path: String,
    folder: bool,
}
#[tauri::command]
async fn search_folder(root: String, query: String) -> Result<Vec<SearchResult>, String> {
    tauri::async_runtime::spawn_blocking(move || search_directory(root, query))
        .await
        .map_err(err)?
}
fn search_directory(root: String, query: String) -> Result<Vec<SearchResult>, String> {
    search_directory_kind(root, query, false)
}
fn search_directory_kind(
    root: String,
    query: String,
    folders_only: bool,
) -> Result<Vec<SearchResult>, String> {
    if query.chars().count() < 2 {
        return Ok(vec![]);
    }
    let p = Path::new(&root);
    if !p.is_absolute() || !p.is_dir() {
        return Err("Select an existing project directory first.".into());
    }
    let q = query.to_lowercase();
    let mut out = vec![];
    // Deliberately one directory only: no invisible recursive disk scan.
    for entry in std::fs::read_dir(p).map_err(err)?.take(2000) {
        let e = entry.map_err(err)?;
        let name = e.file_name().to_string_lossy().to_string();
        if name.to_lowercase().contains(&q) {
            let folder = e.file_type().map_err(err)?.is_dir();
            if folders_only && !folder {
                continue;
            }
            out.push(SearchResult {
                name,
                path: e.path().display().to_string(),
                folder,
            });
            if out.len() == 50 {
                break;
            }
        }
    }
    Ok(out)
}
#[tauri::command]
fn stage_paths(
    app: tauri::AppHandle,
    core: tauri::State<Shared>,
    paths: Vec<String>,
) -> Result<(), String> {
    if paths.len() > 20 {
        return Err("Drop at most 20 paths at once.".into());
    }
    let mut db = core.db.lock().map_err(err)?;
    let all = rows(&db)?;
    if all.len() + paths.len() > 1900 {
        return Err("Local item limit reached.".into());
    }
    if all.iter().filter(|i| i.kind == "shelf").count() + paths.len() > 50 {
        return Err("Shelf limit is 50 entries.".into());
    }
    let mut staged = vec![];
    for path in paths {
        let p = Path::new(&path);
        if !p.is_absolute() || !p.exists() {
            return Err("A dropped path no longer exists.".into());
        }
        staged.push(Item {
            id: new_id(),
            kind: "shelf".into(),
            title: clip_title(
                &p.file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| p.display().to_string()),
            ),
            path,
            created_at: now(),
            ..Default::default()
        });
    }
    let tx = db.transaction().map_err(err)?;
    for i in staged {
        put(&tx, &i)?;
    }
    tx.commit().map_err(err)?;
    drop(db);
    changed(&app, &core);
    Ok(())
}
#[tauri::command]
fn export_backup(core: tauri::State<Shared>) -> Result<String, String> {
    let db = core.db.lock().map_err(err)?;
    let mut items = rows(&db)?;
    for i in &mut items {
        if i.kind == "timer" && i.state == "running" {
            i.remaining_ms = remaining(i, now(), ticks(), &core.run);
            i.started_tick = ticks();
        }
    }
    serde_json::to_string(
        &serde_json::json!({"version":2,"items":items,"agentTasks":agents::backup(&db)?}),
    )
    .map_err(err)
}
#[tauri::command]
fn import_backup(
    app: tauri::AppHandle,
    core: tauri::State<Shared>,
    data: String,
) -> Result<usize, String> {
    if data.len() > 5 * 1024 * 1024 {
        return Err("Backup exceeds 5 MiB.".into());
    }
    let v: serde_json::Value = serde_json::from_str(&data).map_err(err)?;
    if v["version"] != 1 && v["version"] != 2 {
        return Err("Unsupported backup version.".into());
    }
    let items: Vec<Item> = serde_json::from_value(v["items"].clone()).map_err(err)?;
    if items.len() > 2000 {
        return Err("Backup item limit exceeded.".into());
    }
    let mut db = core.db.lock().map_err(err)?;
    let existing = rows(&db)?;
    let missing = items
        .iter()
        .filter(|i| !existing.iter().any(|e| e.id == i.id))
        .count();
    if existing.len() + missing > 2000 {
        return Err("Import would exceed the local item limit.".into());
    }
    for i in &items {
        validate(i)?;
        if i.id.is_empty() || i.id.len() > 128 {
            return Err("Invalid backup item identity.".into());
        }
    }
    let tx = db.transaction().map_err(err)?;
    let mut n = 0;
    for mut i in items {
        if existing.iter().any(|e| e.id == i.id) {
            continue;
        }
        if i.kind == "timer" && i.state == "running" {
            i.state = "paused".into();
            i.run.clear();
        }
        put(&tx, &i)?;
        n += 1;
    }
    n += agents::restore(&tx, &v["agentTasks"])?;
    tx.commit().map_err(err)?;
    drop(db);
    changed(&app, &core);
    let _ = app.emit("agent-changed", ());
    Ok(n)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Provider {
    id: String,
    installed: bool,
    path: String,
    description: String,
}
fn resolve_cli(name: &str) -> Option<PathBuf> {
    let env = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&env) {
        for suffix in [".exe", ".cmd", ""] {
            let p = dir.join(format!("{name}{suffix}"));
            if p.is_file() {
                if name == "codex" && suffix == ".cmd" {
                    let binary=dir.join("node_modules/@openai/codex/node_modules/@openai/codex-win32-x64/vendor/x86_64-pc-windows-msvc/bin/codex.exe");
                    if binary.is_file() {
                        return Some(binary);
                    }
                }
                return Some(p);
            }
        }
    }
    None
}
#[tauri::command]
fn providers() -> Vec<Provider> {
    ["codex", "agy"]
        .iter()
        .map(|name| {
            let path = resolve_cli(name);
            Provider {
                id: name.to_string(),
                installed: path.is_some(),
                path: path.map(|p| p.display().to_string()).unwrap_or_default(),
                description: if *name == "codex" {
                    "Read-only stored-session discovery; live desktop monitoring not verified."
                } else {
                    "ACP connection not implemented in this milestone."
                }
                .into(),
            }
        })
        .collect()
}

fn spawn_cli(path: &Path, args: &[&str]) -> Result<std::process::Child, String> {
    if path.extension().is_some_and(|s| s == "cmd") {
        return Err("Found a CLI shim, but no directly manageable native executable. Install a supported native CLI to avoid orphan workers.".into());
    }
    let mut c = Command::new(path);
    c.args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        c.creation_flags(0x08000000);
    }
    c.spawn().map_err(err)
}
#[tauri::command]
async fn codex_sessions(core: tauri::State<'_, Shared>) -> Result<serde_json::Value, String> {
    let owned = core.inner().clone();
    codex_read(owned,"thread/list",serde_json::json!({"limit":25,"useStateDbOnly":true,"sourceKinds":["cli","vscode","appServer"],"sortKey":"updated_at"})).await
}
#[tauri::command]
async fn codex_models(core: tauri::State<'_, Shared>) -> Result<serde_json::Value, String> {
    codex_read(
        core.inner().clone(),
        "model/list",
        serde_json::json!({"limit":100,"includeHidden":false}),
    )
    .await
}
async fn codex_read(
    owned: Shared,
    method: &'static str,
    params: serde_json::Value,
) -> Result<serde_json::Value, String> {
    tauri::async_runtime::spawn_blocking(move||{
        let path=resolve_cli("codex").ok_or("Codex CLI was not found in PATH.")?;
        let mut workers=owned.workers.lock().map_err(err)?;if !workers.is_empty(){return Err("A session read is already running.".into())}
        let child=Arc::new(Mutex::new(spawn_cli(&path,&["app-server"])?));workers.push(child.clone());drop(workers);
        let result=(||->Result<serde_json::Value,String>{
            let mut process=child.lock().map_err(err)?;let mut input=process.stdin.take().ok_or("Cannot open Codex input")?;let output=process.stdout.take().ok_or("Cannot open Codex output")?;drop(process);
            let (tx,rx)=std::sync::mpsc::sync_channel(20);
            std::thread::spawn(move||{let mut reader=BufReader::new(output);loop{let mut bytes=vec![];match reader.by_ref().take(1024*1024+1).read_until(b'\n',&mut bytes){Ok(0)|Err(_)=>break,Ok(_)=>{if bytes.len()>1024*1024{let _=tx.try_send(Err("Codex frame exceeded 1 MiB.".to_string()));break}if tx.send(serde_json::from_slice::<serde_json::Value>(&bytes).map_err(err)).is_err(){break}}}}});
            let send=|input:&mut std::process::ChildStdin,v:serde_json::Value|->Result<(),String>{writeln!(input,"{}",v).map_err(err)?;input.flush().map_err(err)};
            let receive=|id:i64|->Result<serde_json::Value,String>{let deadline=std::time::Instant::now()+Duration::from_secs(12);for _ in 0..50{let left=deadline.saturating_duration_since(std::time::Instant::now());let v=rx.recv_timeout(left).map_err(|_|"Codex did not respond within 12 seconds.".to_string())??;if v["id"].as_i64()==Some(id){if !v["error"].is_null(){return Err(format!("Codex protocol: {}",v["error"]))}return Ok(v["result"].clone())}}Err("Too many unexpected Codex messages.".into())};
            send(&mut input,serde_json::json!({"id":1,"method":"initialize","params":{"clientInfo":{"name":"desktop_buddy","version":"0.1.0"},"capabilities":{"experimentalApi":false}}}))?;receive(1)?;
            send(&mut input,serde_json::json!({"method":"initialized"}))?;
            send(&mut input,serde_json::json!({"id":2,"method":method,"params":params}))?;
            receive(2)
        })();if let Ok(mut p)=child.lock(){let _=p.kill();let _=p.wait();}if let Ok(mut workers)=owned.workers.lock(){workers.retain(|w|!Arc::ptr_eq(w,&child));}result
    }).await.map_err(err)?
}

fn show_panel(app: &tauri::AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.set_focus();
        return;
    }
    if let Some(cfg) = app.config().app.windows.first() {
        match tauri::WebviewWindowBuilder::from_config(app, cfg).and_then(|b| b.build()) {
            Ok(w) => {
                let _ = w.set_focus();
            }
            Err(e) => {
                if let Some(c) = app.try_state::<Shared>() {
                    if let Ok(mut warning) = c.warning.lock() {
                        *warning = Some(format!("Could not open panel: {e}"));
                    }
                }
            }
        }
    }
}
fn request_quit(app: &tauri::AppHandle) {
    if let Some(core) = app.try_state::<Shared>() {
        if core.agents.lock().map(|a| !a.is_empty()).unwrap_or(true) {
            if let Ok(mut w) = core.warning.lock() {
                *w=Some("A managed Codex connection is attached. End it in Activity before quitting. Closing the panel keeps the connection running.".into());
            }
            show_panel(app);
            let _ = app.emit("data-changed", ());
            return;
        }
    }
    app.exit(0);
}
fn scheduler(app: tauri::AppHandle, core: Shared) {
    std::thread::spawn(move || loop {
        let generation = core.revision.lock().unwrap_or_else(|p| p.into_inner());
        let mut delay = Duration::from_secs(3600);
        let mut fired = vec![];
        let mut reminder_due = false;
        if let Ok(mut db) = core.db.lock() {
            if let Ok(items) = rows(&db) {
                if let Ok(tx) = db.transaction() {
                    let mut successful = true;
                    for mut i in items {
                        let wait = if i.kind == "timer" && i.state == "running" {
                            Some(remaining(&i, now(), ticks(), &core.run))
                        } else if i.kind == "reminder" && !i.done {
                            Some(i.due_at - now())
                        } else {
                            None
                        };
                        if let Some(ms) = wait {
                            if ms <= 0 {
                                if i.kind == "timer" {
                                    i.state = "finished".into();
                                    i.remaining_ms = 0;
                                } else {
                                    i.done = true;
                                }
                                let a = Item {
                                    id: format!("due-{}", i.id),
                                    kind: "attention".into(),
                                    title: i.title.clone(),
                                    created_at: now(),
                                    source: if i.kind == "timer" {
                                        "Timer finished"
                                    } else {
                                        "Reminder due"
                                    }
                                    .into(),
                                    ..Default::default()
                                };
                                if put(&tx, &i).and_then(|_| put(&tx, &a)).is_err() {
                                    successful = false;
                                    break;
                                }
                                reminder_due |= i.kind == "reminder";
                                fired.push(a.title);
                            } else {
                                delay = delay
                                    .min(Duration::from_millis(ms as u64))
                                    .min(Duration::from_secs(30));
                            }
                        }
                    }
                    if !successful || tx.commit().is_err() {
                        fired.clear();
                        delay = Duration::from_secs(30);
                        if let Ok(mut w) = core.warning.lock() {
                            *w=Some("Could not persist alerts: local storage is full or unavailable. Export and clear old items, and check disk space.".into())
                        }
                    }
                }
            }
        }
        if !fired.is_empty() {
            pet::refresh_indicators(&app);
            let cue = if reminder_due { pet_art::Cue::ReminderDue } else { pet_art::Cue::FocusDone };
            pet::cue(&app, cue);
            let _ = app.emit("data-changed", ());
            let body = if fired.len() == 1 {
                fired[0].clone()
            } else {
                format!(
                    "{} items need your attention. Open Desktop Buddy.",
                    fired.len()
                )
            };
            if app
                .notification()
                .builder()
                .title("Desktop Buddy")
                .body(body)
                .show()
                .is_err()
            {
                if let Ok(mut w) = core.warning.lock() {
                    *w = Some(
                        "Windows notification delivery failed. Items remain in Needs me.".into(),
                    )
                }
            }
        }
        let _guard = core
            .wake
            .wait_timeout(generation, delay)
            .unwrap_or_else(|p| p.into_inner());
    });
}
fn initialize(path: &Path) -> Result<Connection, String> {
    std::fs::create_dir_all(path).map_err(err)?;
    let db = Connection::open(path.join("buddy.sqlite3")).map_err(err)?;
    db.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL; PRAGMA busy_timeout=3000; CREATE TABLE IF NOT EXISTS items(id TEXT PRIMARY KEY,kind TEXT NOT NULL,created INTEGER NOT NULL,payload TEXT NOT NULL); CREATE INDEX IF NOT EXISTS items_kind ON items(kind); CREATE TABLE IF NOT EXISTS settings(key TEXT PRIMARY KEY,value TEXT NOT NULL); PRAGMA user_version=1;").map_err(err)?;
    agents::initialize(&db)?;
    Ok(db)
}
fn main() {
    tauri::Builder::default()
        .manage(navigation::Navigation::default())
        .manage(Arc::new(browser_fill::BrowserState::default()))
        .manage(Arc::new(screen_helper::ScreenState::default()))
        .plugin(tauri_plugin_single_instance::init(|app, args, _| {
            if args.iter().any(|a| a == "--quit") {
                request_quit(app)
            } else if args.iter().any(|a| a == "--close-panel") {
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.close();
                }
            } else if args.iter().any(|a| a == "--commands") {
                let _ = pet::pet_commands(app.clone());
            } else if args.iter().any(|a| a == "--close-commands") {
                let _ = pet::close_commands(app.clone());
            } else {
                if args.iter().any(|a| a == "--panel") {
                    show_panel(app)
                } else {
                    let _ = pet::pet_show(app.clone());
                }
            }
        }))
        .plugin(tauri_plugin_notification::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _, event| {
                    if event.state() == ShortcutState::Pressed {
                        if pet::pet_commands(app.clone()).is_err() {
                            show_panel(app)
                        }
                    }
                })
                .build(),
        )
        .invoke_handler(tauri::generate_handler![
            snapshot,
            screen_helper::screen_capture,
            screen_helper::screen_explain,
            screen_helper::screen_clear,
            screen_helper::screen_cancel,
            browser_fill::browser_helper_files,
            browser_fill::browser_connections,
            browser_fill::browser_capture,
            browser_fill::browser_generate,
            browser_fill::browser_apply,
            browser_fill::browser_cancel,
            navigation::panel_navigation,
            navigation::acknowledge_navigation,
            save_item,
            remove_item,
            toggle_item,
            rename_todo,
            start_timer,
            timer_action,
            set_theme,
            open_path,
            search_folder,
            stage_paths,
            export_backup,
            import_backup,
            providers,
            codex_sessions,
            codex_models,
            agents::agent_snapshot,
            agents::agent_start,
            agents::agent_respond,
            agents::agent_cancel,
            agents::agent_remove,
            agents::agent_seen,
            agents::agent_todo,
            pet::pet_show,
            pet::pet_hide,
            pet::pet_hold,
            pet::pet_copy,
            pet::pet_commands,
            pet::pet_status
        ])
        .setup(|app| {
            if std::env::args()
                .any(|a| a == "--quit" || a == "--close-panel" || a == "--close-commands")
            {
                app.handle().exit(0);
                return Ok(());
            }
            let data = if let Some(p) = std::env::var_os("BUDDY_DATA_DIR") {
                let path = PathBuf::from(p);
                if !path.is_absolute() {
                    return Err("BUDDY_DATA_DIR must be absolute".into());
                }
                path
            } else {
                app.path().app_local_data_dir()?
            };
            let db = initialize(&data).map_err(std::io::Error::other)?;
            let core = Arc::new(Core {
                db: Mutex::new(db),
                revision: Mutex::new(0),
                wake: Condvar::new(),
                run: new_id(),
                data_path: data,
                warning: Mutex::new(None),
                workers: Mutex::new(vec![]),
                agents: Mutex::new(std::collections::HashMap::new()),
            });
            app.manage(core.clone());
            let mut unavailable = Vec::new();
            for shortcut in ["Ctrl+Alt+P", "Ctrl+Alt+Space"] {
                if let Err(e) = app.global_shortcut().register(shortcut) {
                    unavailable.push(format!("{shortcut}: {e}"));
                }
            }
            if !unavailable.is_empty() {
                *core.warning.lock().unwrap() = Some(format!(
                    "Shortcut unavailable ({}). You can open Buddy from the tray.",
                    unavailable.join(", ")
                ));
            }
            let mut pixels = vec![0u8; 32 * 32 * 4];
            for y in 5..27 {
                for x in 5..27 {
                    let n = (y * 32 + x) * 4;
                    let accent = (10..14).contains(&x) || (18..22).contains(&x);
                    pixels[n..n + 4].copy_from_slice(if accent {
                        &[220, 186, 252, 255]
                    } else {
                        &[46, 46, 43, 255]
                    });
                }
            }
            let tray = TrayIconBuilder::new()
                .icon(tauri::image::Image::new_owned(pixels, 32, 32))
                .tooltip("Desktop Buddy · Ctrl+Alt+P")
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "recovery-open" => show_panel(app),
                    "recovery-quit" => request_quit(app),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if matches!(
                        event,
                        TrayIconEvent::Click {
                            button: MouseButton::Left,
                            button_state: MouseButtonState::Up,
                            ..
                        }
                    ) {
                        show_panel(tray.app_handle())
                    } else if matches!(event, TrayIconEvent::Click {
                        button: MouseButton::Right,
                        button_state: MouseButtonState::Up,
                        ..
                    }) && tray.app_handle().try_state::<Arc<pet::Control>>().is_some()
                        && pet::pet_menu(tray.app_handle()).is_err() {
                        show_panel(tray.app_handle())
                    }
                })
                .build(app)?;
            scheduler(app.handle().clone(), core);
            let tray_only = std::env::args().any(|a| a == "--tray");
            match pet::start(app.handle().clone(), !tray_only) {
                Ok(control) => {
                    app.manage(control);
                }
                Err(e) => {
                    let open = MenuItem::with_id(app, "recovery-open", "Open Desktop Buddy", true, None::<&str>)?;
                    let quit = MenuItem::with_id(app, "recovery-quit", "Quit (reminders stop)", true, None::<&str>)?;
                    tray.set_menu(Some(Menu::with_items(app, &[&open, &quit])?))?;
                    if let Some(c) = app.try_state::<Shared>() {
                        *c.warning.lock().unwrap() = Some(format!(
                            "Pet unavailable: {e}. Tools remain available from the tray."
                        ));
                    }
                    if !tray_only {
                        show_panel(app.handle());
                    }
                }
            }
            if std::env::args().any(|a| a == "--panel") {
                show_panel(app.handle());
            }
            if std::env::args().any(|a| a == "--commands") {
                let _ = pet::pet_commands(app.handle().clone());
            }
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("Desktop Buddy could not start")
        .run(|app, event| match event {
            tauri::RunEvent::ExitRequested {
                code: None, api, ..
            } => api.prevent_exit(),
            tauri::RunEvent::Exit => {
                pet::stop(app);
                if let Some(core) = app.try_state::<Shared>() {
                    if let Ok(workers) = core.workers.lock() {
                        for worker in workers.iter() {
                            if let Ok(mut p) = worker.lock() {
                                let _ = p.kill();
                                let _ = p.wait();
                            }
                        }
                    }
                }
            }
            _ => {}
        });
}

#[cfg(test)]
mod storage_tests {
    use super::*;
    fn database() -> Connection {
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch("CREATE TABLE items(id TEXT PRIMARY KEY,kind TEXT NOT NULL,created INTEGER NOT NULL,payload TEXT NOT NULL);").unwrap();
        db
    }
    #[test]
    fn completing_a_stale_todo_keeps_its_latest_data_and_never_reopens_it() {
        let db = database();
        let item = Item {
            id: "todo".into(),
            kind: "todo".into(),
            title: "Updated after the menu opened".into(),
            note: "Keep this note".into(),
            ..Default::default()
        };
        put(&db, &item).unwrap();
        assert!(complete_todo(&db, &item.id).unwrap());
        rename_todo_item(&db, &item.id, "Renamed while checked").unwrap();
        assert!(get(&db, &item.id).unwrap().done);
        rename_todo_item(&db, &item.id, &item.title).unwrap();
        assert!(!complete_todo(&db, &item.id).unwrap());
        let completed = get(&db, &item.id).unwrap();
        assert!(completed.done);
        assert_eq!(completed.title, item.title);
        assert_eq!(completed.note, item.note);
        db.execute("DELETE FROM items WHERE id=?1", [&item.id]).unwrap();
        assert!(complete_todo(&db, &item.id).is_err());
        assert!(rows(&db).unwrap().is_empty());
        put(&db, &Item { kind: "reminder".into(), ..item }).unwrap();
        assert!(complete_todo(&db, "todo").is_err());
        assert!(!get(&db, "todo").unwrap().done);
    }
    #[test]
    fn updates_preserve_one_identity_and_export_roundtrips() {
        let db = database();
        let mut i = Item {
            id: "one".into(),
            kind: "todo".into(),
            title: "before".into(),
            ..Default::default()
        };
        put(&db, &i).unwrap();
        i.title = "after".into();
        put(&db, &i).unwrap();
        let all = rows(&db).unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].title, "after");
        let export = serde_json::to_string(&serde_json::json!({"version":1,"items":all})).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&export).unwrap();
        assert_eq!(parsed["items"][0]["id"], "one");
    }
    #[test]
    fn legacy_rows_are_never_silently_truncated() {
        let db = database();
        let tx = db.unchecked_transaction().unwrap();
        for n in 0..2001 {
            let i = Item {
                id: n.to_string(),
                kind: "todo".into(),
                title: "history".into(),
                ..Default::default()
            };
            tx.execute(
                "INSERT INTO items VALUES(?1,'todo',0,?2)",
                params![i.id, serde_json::to_string(&i).unwrap()],
            )
            .unwrap();
        }
        tx.commit().unwrap();
        assert_eq!(rows(&db).unwrap().len(), 2001);
        let i = Item {
            id: "new".into(),
            kind: "todo".into(),
            title: "overflow".into(),
            ..Default::default()
        };
        assert!(put(&db, &i).is_err());
    }
    #[test]
    fn payload_capacity_failure_does_not_replace_old_data() {
        let db = database();
        let original = Item {
            id: "one".into(),
            kind: "todo".into(),
            title: "keep".into(),
            ..Default::default()
        };
        put(&db, &original).unwrap();
        let oversized = Item {
            note: "a".repeat(3 * 1024 * 1024),
            ..original.clone()
        };
        assert!(put(&db, &oversized).is_err());
        assert_eq!(get(&db, "one").unwrap().title, "keep");
    }
    #[test]
    fn transaction_rolls_back_partial_batch() {
        let mut db = database();
        {
            let tx = db.transaction().unwrap();
            put(
                &tx,
                &Item {
                    id: "one".into(),
                    kind: "todo".into(),
                    title: "temporary".into(),
                    ..Default::default()
                },
            )
            .unwrap();
        }
        assert!(rows(&db).unwrap().is_empty());
    }
}
