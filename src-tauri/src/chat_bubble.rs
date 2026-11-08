//! Recent Codex chats. Only a live shared app-server may receive follow-ups.
use super::*;
use serde_json::{json, Value};
use std::time::Instant;
struct Rpc {
    child: std::process::Child,
    input: std::process::ChildStdin,
    rx: std::sync::mpsc::Receiver<Value>,
    next: i64,
}
impl Drop for Rpc {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
impl Rpc {
    fn connect() -> Result<Self, String> {
        let path = resolve_cli("codex").ok_or("Codex CLI is unavailable")?;
        let mut child = spawn_cli(&path, &["app-server", "proxy"])?;
        let input = child.stdin.take().ok_or("No Codex input")?;
        let output = child.stdout.take().ok_or("No Codex output")?;
        let (tx, rx) = std::sync::mpsc::sync_channel(32);
        std::thread::spawn(move || {
            let mut reader = BufReader::new(output);
            loop {
                let mut bytes = Vec::new();
                match reader
                    .by_ref()
                    .take(1024 * 1024 + 1)
                    .read_until(b'\n', &mut bytes)
                {
                    Ok(0) | Err(_) => break,
                    Ok(_) => {
                        if bytes.len() > 1024 * 1024 {
                            break;
                        }
                        if let Ok(value) = serde_json::from_slice(&bytes) {
                            if tx.send(value).is_err() {
                                break;
                            }
                        }
                    }
                }
            }
        });
        let mut rpc = Self {
            child,
            input,
            rx,
            next: 0,
        };
        rpc.request("initialize",json!({"clientInfo":{"name":"desktop_buddy_chats","version":"0.4.0"},"capabilities":{"experimentalApi":false}}),3)?;
        writeln!(rpc.input, "{}", json!({"method":"initialized"})).map_err(err)?;
        rpc.input.flush().map_err(err)?;
        Ok(rpc)
    }
    fn request(&mut self, method: &str, params: Value, seconds: u64) -> Result<Value, String> {
        self.next += 1;
        let id = self.next;
        writeln!(
            self.input,
            "{}",
            json!({"id":id,"method":method,"params":params})
        )
        .map_err(err)?;
        self.input.flush().map_err(err)?;
        let until = Instant::now() + Duration::from_secs(seconds);
        for _ in 0..256 {
            let value=self.rx.recv_timeout(until.saturating_duration_since(Instant::now())).map_err(|_|"Codex connection did not acknowledge the request. Do not resend until you check the source chat.".to_string())?;
            if value["id"].as_i64() == Some(id) && value["method"].is_null() {
                if !value["error"].is_null() {
                    return Err(format!("Codex: {}", value["error"]["message"]));
                }
                return Ok(value["result"].clone());
            }
            if !value["id"].is_null() && value["method"].is_string() {
                writeln!(self.input,"{}",json!({"id":value["id"],"error":{"code":-32601,"message":"Review this request in Codex."}})).map_err(err)?;
                self.input.flush().map_err(err)?;
            }
        }
        Err("Codex sent too many events. Check the source chat before retrying.".into())
    }
}
#[tauri::command]
pub async fn kitty_chats(
    core: tauri::State<'_, Shared>,
    cursor: Option<String>,
) -> Result<Value, String> {
    crate::features::require_codex_chats()?;
    let params = json!({"limit":25,"cursor":cursor,"sortKey":"recency_at","useStateDbOnly":true,"sourceKinds":["cli","vscode","appServer"]});
    let probe_params = params.clone();
    let shared = tauri::async_runtime::spawn_blocking(move || {
        let mut rpc = Rpc::connect()?;
        rpc.request("thread/list", probe_params, 8)
    })
    .await
    .map_err(err)?;
    match shared {
        Ok(mut result) => {
            result["live"] = true.into();
            Ok(result)
        }
        Err(_) => {
            let mut result = codex_read(core.inner().clone(), "thread/list", params).await?;
            result["live"] = false.into();
            Ok(result)
        }
    }
}
fn can_message(status: &Value) -> bool {
    matches!(status["type"].as_str(), Some("idle" | "active"))
}
#[tauri::command]
pub async fn kitty_chat_send(thread_id: String, message: String) -> Result<Value, String> {
    crate::features::require_codex_chats()?;
    if thread_id.is_empty()
        || thread_id.len() > 128
        || message.trim().is_empty()
        || message.len() > 8192
    {
        return Err("Choose a chat and type a message under 8,192 characters.".into());
    }
    tauri::async_runtime::spawn_blocking(move || {
        let mut rpc=Rpc::connect().map_err(|_|"Live Codex connection unavailable. Open this chat in Codex to continue it.".to_string())?;
        let current=rpc.request("thread/read",json!({"threadId":thread_id,"includeTurns":true}),10)?;
        if !can_message(&current["thread"]["status"]) {return Err("This chat has no live runtime status. Continue it in Codex; Kitty did not send anything.".into());}
        let input=json!([{"type":"text","text":message,"text_elements":[]}]);
        if current["thread"]["status"]["type"]=="active" {
            let turn=current["thread"]["turns"].as_array().and_then(|turns| turns.iter().rev().find(|turn|turn["status"]=="inProgress")).and_then(|turn|turn["id"].as_str()).ok_or("Running turn identity is unavailable; nothing was sent.")?;
            rpc.request("turn/steer",json!({"threadId":thread_id,"expectedTurnId":turn,"input":input}),12)?;
        } else {
            // Rejoin the loaded thread; preserve its model, workspace, permissions and settings.
            let resumed=rpc.request("thread/resume",json!({"threadId":thread_id,"excludeTurns":true}),10)?;
            if resumed["thread"]["status"]["type"] != "idle" { return Err("The chat status changed. Refresh before sending; nothing was sent.".into()); }
            rpc.request("turn/start",json!({"threadId":thread_id,"input":input}),12)?;
        }
        Ok(json!({"accepted":true}))
    }).await.map_err(err)?
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unloaded_is_never_inferred_idle() {
        assert!(!can_message(&json!({"type":"notLoaded"})));
        assert!(!can_message(&Value::Null));
        assert!(can_message(&json!({"type":"idle"})));
        assert!(can_message(&json!({"type":"active"})));
    }
}
