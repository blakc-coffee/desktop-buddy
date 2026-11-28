//! Chat through a warm stream-json CLI: Claude Code or Antigravity (`agy`), whichever the user
//! connected. Opening the bar starts the process; nothing is sent before the user submits. One
//! process holds one conversation; dismissing the bubble or 90 s idle ends it, which bounds RAM.
//! Claude gets no tools, MCP servers, settings or sessions; agy runs sandboxed in an empty folder
//! and is never granted permission approvals, so its tools cannot act.
use super::*;
use crate::chat_agent::Agent;
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
    Prepare(Agent),
    Run(Agent, Turn, Sender<Result<Reply, Failure>>),
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
    /// Starts the agent's CLI ahead of a question; sends nothing.
    pub fn prepare_for(&self, agent: Agent) {
        self.send(Job::Prepare(agent));
    }
    /// Ends the conversation; the next question starts a fresh one.
    pub fn reset(&self) {
        self.send(Job::Reset);
    }
    /// True while a conversation is open, so a follow-up can be sent as-is.
    pub fn live(&self) -> bool {
        self.live.load(Ordering::SeqCst)
    }
    pub fn run(&self, agent: Agent, turn: Turn) -> Result<Reply, Failure> {
        let (tx, rx) = mpsc::channel();
        if !self.send(Job::Run(agent, turn, tx)) {
            return Err(unavailable("Chat worker stopped."));
        }
        rx.recv()
            .map_err(|_| unavailable("Chat worker stopped."))?
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
            Ok(Job::Prepare(agent)) => {
                if worker.as_ref().is_some_and(|w| w.agent != agent) {
                    worker = None;
                    blocked = None;
                }
                if worker.is_none() && blocked.is_none_or(|until| Instant::now() > until) {
                    match Worker::spawn(agent) {
                        Ok(value) => worker = Some(value),
                        Err(_) => blocked = Some(Instant::now() + RETRY_AFTER),
                    }
                }
                last = Instant::now();
            }
            Ok(Job::Run(agent, turn, reply)) => {
                if worker.as_ref().is_some_and(|w| w.agent != agent) {
                    worker = None;
                }
                if worker.is_none() {
                    match Worker::spawn(agent) {
                        Ok(value) => worker = Some(value),
                        Err(message) => {
                            let _ = reply.send(Err(unavailable(&message)));
                            continue;
                        }
                    }
                }
                let result = worker.as_mut().map_or_else(
                    || Err(unavailable("The agent is unavailable.")),
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
    agent: Agent,
    /// agy has no system-prompt flag, so its first message carries the instructions.
    greeted: bool,
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
    fn spawn(agent: Agent) -> Result<Self, String> {
        let path = resolve_cli(agent.cli())
            .filter(|p| p.extension().is_some_and(|e| e == "exe"))
            .ok_or_else(|| format!("Install {} and sign in to use it for chat.", agent.label()))?;
        // An empty working folder keeps project instruction files and git state out of the chat.
        let folder = std::env::temp_dir().join("desktop-buddy-claude");
        std::fs::create_dir_all(&folder).map_err(err)?;
        let mut command = std::process::Command::new(path);
        let args: &[&str] = match agent {
            Agent::Antigravity => &[
                "--input-format",
                "stream-json",
                "--output-format",
                "stream-json",
                "--disable-slash-commands",
                "--sandbox",
                // agy reads "-p <next arg>" as the prompt; an empty attached value keeps stdin as input.
                "-p=",
            ],
            _ => &[
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
            ],
        };
        command
            .args(args)
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
        let input = child.stdin.take().ok_or("No agent input")?;
        let output = child.stdout.take().ok_or("No agent output")?;
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
        Ok(Self { agent, greeted: false, child, input, rx })
    }
    fn turn(&mut self, mut turn: Turn) -> Result<Reply, Failure> {
        let label = self.agent.label();
        let message = match self.agent {
            Agent::Antigravity => {
                let content = if self.greeted { turn.prompt.clone() } else { format!("{SYSTEM}\n\n{}", turn.prompt) };
                json!({"event":"user","message":{"role":"user","content":content}})
            }
            _ => json!({"type":"user","message":{"role":"user","content":turn.prompt}}),
        };
        writeln!(self.input, "{message}")
            .and_then(|_| self.input.flush())
            .map_err(|_| unavailable(&format!("{label} closed before the question was sent.")))?;
        self.greeted = true;
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
                    message: format!("{label} took too long. Try again."),
                });
            }
            let value = match self.rx.recv_timeout(Duration::from_millis(100)) {
                Ok(value) => value,
                Err(RecvTimeoutError::Timeout) => continue,
                Err(RecvTimeoutError::Disconnected) => {
                    return Err(Failure {
                        submitted: shown,
                        message: format!("{label} stopped responding. Check that it is signed in."),
                    })
                }
            };
            match event_text(self.agent, &value) {
                Event::Delta(delta) => {
                    if text.len() + delta.len() <= MAX_TEXT {
                        text.push_str(&delta);
                    }
                    // Never stream the ACTION protocol line; it is applied, not shown.
                    if !maybe_action(&text) {
                        (turn.publish)(&text, !shown);
                        shown = true;
                    }
                }
                Event::Done(Err(reason)) => {
                    return Err(Failure {
                        submitted: shown,
                        message: reason,
                    });
                }
                Event::Done(Ok(result)) => {
                    if text.trim().is_empty() {
                        text = result;
                    }
                    return Ok(reply(&text));
                }
                Event::Other => {}
            }
        }
    }
}
#[derive(Debug, PartialEq)]
enum Event {
    Delta(String),
    Done(Result<String, String>),
    Other,
}
/// Maps one output line of either CLI to a text delta or the turn's final result.
fn event_text(agent: Agent, value: &Value) -> Event {
    let clip = |text: &str| text.chars().take(300).collect::<String>();
    match agent {
        Agent::Antigravity => match value["event"].as_str() {
            Some("step_update") => {
                let step = &value["step_update"];
                match step["text_delta"].as_str() {
                    Some(delta) if step["step_type"] == "agent_response" => Event::Delta(delta.into()),
                    _ => Event::Other,
                }
            }
            Some("result") => {
                let result = &value["result"];
                if result["status"] == "SUCCESS" {
                    Event::Done(Ok(result["response"].as_str().unwrap_or_default().into()))
                } else {
                    Event::Done(Err(clip(result["error"].as_str().unwrap_or("Antigravity returned an error."))))
                }
            }
            _ => Event::Other,
        },
        _ => match value["type"].as_str() {
            Some("stream_event") => {
                let event = &value["event"];
                match event["delta"]["text"].as_str() {
                    Some(delta) if event["type"] == "content_block_delta" && event["delta"]["type"] == "text_delta" => {
                        Event::Delta(delta.into())
                    }
                    _ => Event::Other,
                }
            }
            Some("result") => {
                let result = value["result"].as_str().unwrap_or_default();
                if value["is_error"].as_bool() == Some(true) {
                    Event::Done(Err(clip(if result.is_empty() { "Claude returned an error." } else { result })))
                } else {
                    Event::Done(Ok(result.into()))
                }
            }
            _ => Event::Other,
        },
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
    #[test]
    fn agy_fixture_yields_text() {
        let mut text = String::new();
        let mut done = None;
        for line in include_str!("../tests/fixtures/agy-stream.ndjson").lines() {
            match event_text(Agent::Antigravity, &serde_json::from_str(line).unwrap()) {
                Event::Delta(delta) => text.push_str(&delta),
                Event::Done(result) => done = Some(result),
                Event::Other => {}
            }
        }
        assert!(text.to_lowercase().contains("ok"));
        assert!(done.unwrap().unwrap().to_lowercase().contains("ok"));
        let failed = json!({"event":"result","result":{"status":"ERROR","error":"not authenticated"}});
        assert_eq!(event_text(Agent::Antigravity, &failed), Event::Done(Err("not authenticated".into())));
    }
    #[test]
    fn claude_events_still_parse() {
        let delta = json!({"type":"stream_event","event":{"type":"content_block_delta","delta":{"type":"text_delta","text":"Hi"}}});
        assert_eq!(event_text(Agent::Claude, &delta), Event::Delta("Hi".into()));
        let error = json!({"type":"result","is_error":true,"result":"Invalid API key · Please run /login"});
        assert_eq!(event_text(Agent::Claude, &error), Event::Done(Err("Invalid API key · Please run /login".into())));
    }
    #[test]
    fn auth_failure_is_flagged() {
        assert!(crate::chat_agent::is_auth_error("Invalid API key · Please run /login"));
        assert!(crate::chat_agent::is_auth_error("not authenticated"));
        assert!(!crate::chat_agent::is_auth_error("Claude took too long. Try again."));
    }
}
