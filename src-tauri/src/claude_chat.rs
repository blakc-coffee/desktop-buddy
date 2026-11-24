//! Default chat through a warm Claude Code CLI. Opening the bar starts the process; nothing is
//! sent before the user submits. One process holds one conversation; dismissing the bubble or
//! 90 s idle ends it, which bounds RAM. Claude gets no tools, MCP servers, settings or sessions.
use super::*;
use crate::warm_ai::Failure;
use serde_json::{json, Value};
use std::{
    io::{BufRead, Write},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, RecvTimeoutError, Sender},
    },
    time::Instant,
};
const IDLE: Duration = Duration::from_secs(90);
const TURN_LIMIT: Duration = Duration::from_secs(90);
const RETRY_AFTER: Duration = Duration::from_secs(30);
const MAX_LINE: u64 = 1024 * 1024;
const MAX_TEXT: usize = 40_000;
const SYSTEM: &str = "You are Kitty, a desktop pet's quick chat assistant. Answer directly and briefly: one to three short sentences unless the user asks for more. Plain text; fenced code only when useful. You have no tools and cannot read files, browse or run anything. If the user asks to be reminded of something or to start a timer, reply with exactly one line and nothing else: ACTION {\"kind\":\"reminder\",\"title\":\"<short title>\",\"in_minutes\":<integer>} or ACTION {\"kind\":\"reminder\",\"title\":\"<short title>\",\"at\":\"HH:MM\"} (24-hour local time) or ACTION {\"kind\":\"timer\",\"title\":\"<short title>\",\"in_minutes\":<integer>}. Use the local time given with each message. Never say you set something without the ACTION line.";

pub struct Turn {
    pub prompt: String,
    pub cancelled: Box<dyn Fn() -> bool + Send>,
    pub publish: Box<dyn FnMut(&str, bool) + Send>,
}
#[derive(Debug, PartialEq)]
pub enum Reply {
    Text(String),
    Action(Value),
}
enum Job {
    Prepare,
    Run(Turn, Sender<Result<Reply, Failure>>),
    Reset,
}
#[derive(Default)]
pub struct ClaudeState {
    tx: Mutex<Option<Sender<Job>>>,
    live: Arc<AtomicBool>,
}
impl ClaudeState {
    fn send(&self, job: Job) -> bool {
        let mut slot = self.tx.lock().unwrap_or_else(|e| e.into_inner());
        let tx = slot.get_or_insert_with(|| {
            let (tx, rx) = mpsc::channel();
            let live = self.live.clone();
            std::thread::spawn(move || actor(rx, live));
            tx
        });
        tx.send(job).is_ok()
    }
    /// Starts the CLI ahead of a question; sends nothing.
    pub fn prepare(&self) {
        self.send(Job::Prepare);
    }
    /// Ends the conversation; the next question starts a fresh one.
    pub fn reset(&self) {
        self.send(Job::Reset);
    }
    /// True while a conversation is open, so a follow-up can be sent as-is.
    pub fn live(&self) -> bool {
        self.live.load(Ordering::SeqCst)
    }
    pub fn run(&self, turn: Turn) -> Result<Reply, Failure> {
        let (tx, rx) = mpsc::channel();
        if !self.send(Job::Run(turn, tx)) {
            return Err(unavailable("Claude worker stopped."));
        }
        rx.recv()
            .map_err(|_| unavailable("Claude worker stopped."))?
    }
}
fn unavailable(message: &str) -> Failure {
    Failure {
        submitted: false,
        message: message.into(),
    }
}
fn actor(rx: Receiver<Job>, live: Arc<AtomicBool>) {
    let mut worker: Option<Worker> = None;
    let mut last = Instant::now();
    let mut blocked: Option<Instant> = None;
    loop {
        let wait = if worker.is_some() {
            IDLE.saturating_sub(last.elapsed()).max(Duration::from_millis(1))
        } else {
            Duration::from_secs(3600)
        };
        match rx.recv_timeout(wait) {
            Err(RecvTimeoutError::Disconnected) => break,
            Err(RecvTimeoutError::Timeout) | Ok(Job::Reset) => worker = None,
            Ok(Job::Prepare) => {
                if worker.is_none() && blocked.is_none_or(|until| Instant::now() > until) {
                    match Worker::spawn() {
                        Ok(value) => worker = Some(value),
                        Err(_) => blocked = Some(Instant::now() + RETRY_AFTER),
                    }
                }
                last = Instant::now();
            }
            Ok(Job::Run(turn, reply)) => {
                if worker.is_none() {
                    match Worker::spawn() {
                        Ok(value) => worker = Some(value),
                        Err(message) => {
                            let _ = reply.send(Err(unavailable(&message)));
                            continue;
                        }
                    }
                }
                let result = worker.as_mut().map_or_else(
                    || Err(unavailable("Claude is unavailable.")),
                    |w| w.turn(turn),
                );
                if result.is_err() {
                    worker = None;
                }
                let _ = reply.send(result);
                last = Instant::now();
            }
        }
        live.store(worker.is_some(), Ordering::SeqCst);
    }
}
struct Worker {
    child: std::process::Child,
    input: std::process::ChildStdin,
    rx: Receiver<Value>,
}
impl Drop for Worker {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
impl Worker {
    fn spawn() -> Result<Self, String> {
        let path = resolve_cli("claude")
            .filter(|p| p.extension().is_some_and(|e| e == "exe"))
            .ok_or("Install Claude Code and sign in to use Claude chat.")?;
        // An empty working folder keeps project CLAUDE.md files and git state out of the chat.
        let folder = std::env::temp_dir().join("desktop-buddy-claude");
        std::fs::create_dir_all(&folder).map_err(err)?;
        let mut command = std::process::Command::new(path);
        command
            .args([
                "-p",
                "--input-format",
                "stream-json",
                "--output-format",
                "stream-json",
                "--verbose",
                "--include-partial-messages",
                "--model",
                "haiku",
                "--tools",
                "",
                "--strict-mcp-config",
                "--setting-sources",
                "",
                "--disable-slash-commands",
                "--no-session-persistence",
                "--system-prompt",
                SYSTEM,
            ])
            .current_dir(&folder)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x08000000);
        }
        let mut child = command.spawn().map_err(err)?;
        let input = child.stdin.take().ok_or("No Claude input")?;
        let output = child.stdout.take().ok_or("No Claude output")?;
        let (tx, rx) = mpsc::sync_channel(256);
        std::thread::spawn(move || {
            let mut reader = std::io::BufReader::new(output);
            loop {
                let mut line = Vec::new();
                match (&mut reader).take(MAX_LINE + 1).read_until(b'\n', &mut line) {
                    Ok(0) | Err(_) => break,
                    Ok(n) if n as u64 > MAX_LINE => break,
                    Ok(_) => {
                        if let Ok(value) = serde_json::from_slice::<Value>(&line) {
                            if tx.send(value).is_err() {
                                break;
                            }
                        }
                    }
                }
            }
        });
        Ok(Self { child, input, rx })
    }
    fn turn(&mut self, mut turn: Turn) -> Result<Reply, Failure> {
        let message = json!({"type":"user","message":{"role":"user","content":turn.prompt}});
        writeln!(self.input, "{message}")
            .and_then(|_| self.input.flush())
            .map_err(|_| unavailable("Claude closed before the question was sent."))?;
        let started = Instant::now();
        let mut text = String::new();
        let mut shown = false;
        loop {
            if (turn.cancelled)() {
                return Err(Failure {
                    submitted: true,
                    message: "Question cancelled".into(),
                });
            }
            if started.elapsed() > TURN_LIMIT {
                return Err(Failure {
                    submitted: shown,
                    message: "Claude took too long. Try again.".into(),
                });
            }
            let value = match self.rx.recv_timeout(Duration::from_millis(100)) {
                Ok(value) => value,
                Err(RecvTimeoutError::Timeout) => continue,
                Err(RecvTimeoutError::Disconnected) => {
                    return Err(Failure {
                        submitted: shown,
                        message: "Claude stopped responding. Check that Claude Code is signed in."
                            .into(),
                    })
                }
            };
            match value["type"].as_str() {
                Some("stream_event") => {
                    let event = &value["event"];
                    if event["type"] == "content_block_delta" && event["delta"]["type"] == "text_delta" {
                        if let Some(delta) = event["delta"]["text"].as_str() {
                            if text.len() + delta.len() <= MAX_TEXT {
                                text.push_str(delta);
                            }
                            // Never stream the ACTION protocol line; it is applied, not shown.
                            if !maybe_action(&text) {
                                (turn.publish)(&text, !shown);
                                shown = true;
                            }
                        }
                    }
                }
                Some("result") => {
                    if value["is_error"].as_bool() == Some(true) {
                        let reason: String = value["result"]
                            .as_str()
                            .unwrap_or("Claude returned an error.")
                            .chars()
                            .take(300)
                            .collect();
                        return Err(Failure {
                            submitted: shown,
                            message: reason,
                        });
                    }
                    if text.is_empty() {
                        text = value["result"].as_str().unwrap_or_default().to_string();
                    }
                    return Ok(reply(&text));
                }
                _ => {}
            }
        }
    }
}
/// Text that is, or may still become, an "ACTION {...}" line.
fn maybe_action(text: &str) -> bool {
    let t = text.trim_start();
    t.starts_with("ACTION") || (t.len() < 6 && "ACTION".starts_with(t))
}
fn reply(text: &str) -> Reply {
    text.trim()
        .strip_prefix("ACTION")
        .and_then(|rest| serde_json::from_str::<Value>(rest.trim()).ok())
        .filter(Value::is_object)
        .map_or_else(|| Reply::Text(text.trim().to_string()), Reply::Action)
}
/// Turns Claude's proposed action into a command only after checking every field.
/// The model never does date arithmetic: clock times are resolved from the local clock.
pub fn action_command(action: &Value) -> Result<commands::Command, String> {
    let title: String = action["title"]
        .as_str()
        .unwrap_or_default()
        .trim()
        .chars()
        .take(240)
        .collect();
    let minutes = || {
        action["in_minutes"]
            .as_u64()
            .filter(|m| (1..=1440).contains(m))
            .map(|m| m as u32)
            .ok_or("Claude suggested a time outside 1–1440 minutes.".to_string())
    };
    let named = |fallback: &str| if title.is_empty() { fallback.to_string() } else { title.clone() };
    match action["kind"].as_str() {
        Some("timer") => Ok(commands::Command::Timer {
            minutes: minutes()?,
            title: named("Focus session"),
        }),
        Some("reminder") => match action["at"].as_str() {
            Some(at) => {
                let (h, m) = at.split_once(':').ok_or("Claude gave an unreadable time.")?;
                let (hour, minute) = (
                    h.trim().parse::<u32>().map_err(|_| "Claude gave an unreadable time.")?,
                    m.trim().parse::<u32>().map_err(|_| "Claude gave an unreadable time.")?,
                );
                if hour > 23 || minute > 59 {
                    return Err("Claude gave an unreadable time.".into());
                }
                Ok(commands::Command::ReminderAt {
                    hour,
                    minute,
                    twelve_hour: false,
                    title: named("Reminder"),
                })
            }
            None => Ok(commands::Command::Reminder {
                minutes: minutes()?,
                title: named("Reminder"),
            }),
        },
        _ => Err("Claude suggested an action Buddy does not support.".into()),
    }
}
/// The user's words plus the local time, which the model needs for "at 5" style requests.
pub fn prompt(question: &str) -> String {
    let minute = crate::local_ms_of_day() / 60_000;
    format!("(Local time {})\n{question}", crate::clock_label(minute as u32))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn action_lines_are_hidden_parsed_and_validated() {
        assert!(maybe_action("ACT"));
        assert!(maybe_action("  ACTION {\"kind\""));
        assert!(!maybe_action("A binary tree"));
        assert!(!maybe_action("Sure"));
        assert_eq!(reply("Hello there."), Reply::Text("Hello there.".into()));
        let Reply::Action(action) =
            reply("ACTION {\"kind\":\"reminder\",\"title\":\"call Saran\",\"at\":\"17:00\"}")
        else {
            panic!("expected an action")
        };
        assert_eq!(
            action_command(&action),
            Ok(commands::Command::ReminderAt {
                hour: 17,
                minute: 0,
                twelve_hour: false,
                title: "call Saran".into()
            })
        );
        assert_eq!(
            action_command(&json!({"kind":"timer","title":"","in_minutes":25})),
            Ok(commands::Command::Timer {
                minutes: 25,
                title: "Focus session".into()
            })
        );
        assert!(action_command(&json!({"kind":"timer","in_minutes":0})).is_err());
        assert!(action_command(&json!({"kind":"reminder","at":"25:00"})).is_err());
        assert!(action_command(&json!({"kind":"delete_files"})).is_err());
        assert_eq!(reply("ACTION not json"), Reply::Text("ACTION not json".into()));
    }
}
