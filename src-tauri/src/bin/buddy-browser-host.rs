#[path = "../browser_protocol.rs"]
mod protocol;
use serde_json::json;
use std::{io, sync::mpsc, time::Duration};
fn main() {
    if let Err(error) = run() {
        eprintln!("Buddy page helper: {error}");
    }
}
fn run() -> Result<(), String> {
    let origin = std::env::args()
        .nth(1)
        .ok_or("The browser must start this helper.")?;
    let id = origin
        .strip_prefix("chrome-extension://")
        .and_then(|value| value.strip_suffix('/'))
        .ok_or("Invalid extension origin.")?;
    if id.len() != 32 || !id.bytes().all(|byte| (b'a'..=b'p').contains(&byte)) {
        return Err("Invalid extension identity.".into());
    }
    let root = protocol::root()?;
    std::fs::create_dir_all(&root).map_err(|error| error.to_string())?;
    let session = format!("session-{}-{}", protocol::now(), std::process::id());
    let folder = root.join(&session);
    std::fs::create_dir(&folder).map_err(|error| error.to_string())?;
    let (tx, rx) = mpsc::sync_channel(8);
    std::thread::spawn(move || {
        let mut input = io::stdin().lock();
        loop {
            match protocol::read_frame(&mut input) {
                Ok(value) => {
                    if tx.send(Some(value)).is_err() {
                        break;
                    }
                }
                Err(_) => {
                    let _ = tx.send(None);
                    break;
                }
            }
        }
    });
    let mut output = io::stdout().lock();
    let mut last_request = String::new();
    let mut armed_until = 0;
    let mut title = String::new();
    let mut page_origin = String::new();
    let mut ping = 0;
    let mut pending: Option<(String, u64)> = None;
    let result = (|| {
        loop {
            let time = protocol::now();
            if time - ping > 1000 {
                protocol::write_frame(&mut output, &json!({"action":"ping"}))?;
                ping = time;
                protocol::atomic(
                    &folder.join("status.json"),
                    &json!({"id":session,"updatedAt":time,"until":armed_until,"title":title,"origin":page_origin}),
                )?;
            }
            match rx.recv_timeout(Duration::from_millis(200)) {
                Ok(None) | Err(mpsc::RecvTimeoutError::Disconnected) => break,
                Ok(Some(message)) => match message["type"].as_str() {
                    Some("ready") => {
                        let until = message["until"].as_u64().unwrap_or(0);
                        if until > time + 301000 {
                            return Err("Invalid page lifetime.".into());
                        }
                        armed_until = until;
                        title = message["title"]
                            .as_str()
                            .unwrap_or("")
                            .chars()
                            .take(240)
                            .collect();
                        page_origin = message["origin"]
                            .as_str()
                            .unwrap_or("")
                            .chars()
                            .take(500)
                            .collect();
                    }
                    Some("response") => {
                        if let Some((id, expires)) = pending.take() {
                            if message["id"] == id && time < expires {
                                protocol::atomic(
                                    &folder.join(format!("response-{id}.json")),
                                    &message,
                                )?;
                            }
                        }
                    }
                    _ => return Err("Unsupported browser message.".into()),
                },
                Err(mpsc::RecvTimeoutError::Timeout) => {}
            }
            if let Ok(command) = protocol::read(&folder.join("command.json")) {
                let id = command["id"].as_str().unwrap_or("");
                if protocol::identifier(id) && id != last_request {
                    last_request = id.into();
                    let expires = command["expires"].as_u64().unwrap_or(0);
                    if time < armed_until
                        && time < expires
                        && expires <= time + 20000
                        && matches!(command["action"].as_str(), Some("capture" | "apply"))
                    {
                        pending = Some((id.into(), expires));
                        protocol::write_frame(&mut output, &command)?;
                    } else {
                        protocol::atomic(
                            &folder.join(format!("response-{id}.json")),
                            &json!({"id":id,"ok":false,"error":"Reconnect the page in the Buddy extension."}),
                        )?;
                    }
                }
            }
            // Expire page content even if the app disconnected before reading its reply.
            if let Ok(files) = std::fs::read_dir(&folder) {
                for file in files.take(100).flatten() {
                    let name = file.file_name().to_string_lossy().into_owned();
                    if name.starts_with("response-")
                        && name.ends_with(".json")
                        && file
                            .metadata()
                            .ok()
                            .and_then(|metadata| metadata.modified().ok())
                            .and_then(|modified| modified.elapsed().ok())
                            .is_some_and(|age| age > Duration::from_secs(300))
                    {
                        let _ = std::fs::remove_file(file.path());
                    }
                }
            }
            if armed_until != 0 && time > armed_until {
                break;
            }
        }
        Ok(())
    })();
    if let Ok(files) = std::fs::read_dir(&folder) {
        for file in files.take(100).flatten() {
            if file.file_type().is_ok_and(|kind| kind.is_file()) {
                let _ = std::fs::remove_file(file.path());
            }
        }
    }
    let _ = std::fs::remove_dir(&folder);
    result
}
