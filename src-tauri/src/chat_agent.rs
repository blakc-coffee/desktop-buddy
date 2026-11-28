//! The one AI agent the user connected for questions in the search bar.

use crate::resolve_cli;

const SETTING: &str = "chat_agent";

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Agent {
    Claude,
    Codex,
    Antigravity,
}

pub const AGENTS: [Agent; 3] = [Agent::Claude, Agent::Codex, Agent::Antigravity];

impl Agent {
    pub fn key(self) -> &'static str {
        match self {
            Agent::Claude => "claude",
            Agent::Codex => "codex",
            Agent::Antigravity => "antigravity",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Agent::Claude => "Claude Code",
            Agent::Codex => "Codex",
            Agent::Antigravity => "Antigravity",
        }
    }
    pub fn cli(self) -> &'static str {
        match self {
            Agent::Claude => "claude",
            Agent::Codex => "codex",
            Agent::Antigravity => "agy",
        }
    }
    pub fn from_key(key: &str) -> Option<Agent> {
        AGENTS.into_iter().find(|agent| agent.key() == key)
    }
    pub fn install_url(self) -> &'static str {
        match self {
            Agent::Claude => "https://claude.com/product/claude-code",
            Agent::Codex => "https://developers.openai.com/codex/cli",
            Agent::Antigravity => "https://antigravity.google/download",
        }
    }
    /// The CLI's own sign-in; Buddy only opens it and never sees credentials.
    pub fn login_command(self) -> &'static [&'static str] {
        match self {
            Agent::Claude => &["claude"],
            Agent::Codex => &["codex", "login"],
            Agent::Antigravity => &["agy"],
        }
    }
    /// Only native executables count: shims cannot be supervised without orphan workers.
    pub fn installed(self) -> bool {
        resolve_cli(self.cli()).is_some_and(|path| path.extension().is_some_and(|e| e == "exe"))
    }
}

pub fn chosen(app: &tauri::AppHandle) -> Option<Agent> {
    crate::pet::setting(app, SETTING).as_deref().and_then(Agent::from_key)
}

pub fn choose(app: &tauri::AppHandle, agent: Agent) {
    crate::pet::save(app, SETTING, agent.key());
}

/// Shown, and matched by the bar, when no usable agent is connected.
pub const NO_AGENT: &str = "Connect an agent: type /agent";

/// The one agent a question goes to; there is no fallback to another agent.
pub fn route(chosen: Option<Agent>, installed: impl Fn(Agent) -> bool) -> Result<Agent, &'static str> {
    chosen.filter(|agent| installed(*agent)).ok_or(NO_AGENT)
}

pub fn sign_in_message(agent: Agent) -> String {
    format!("Sign in to {}", agent.label())
}

/// The agent whose last answer failed for lack of sign-in; the bar offers its sign-in row.
static SIGN_IN: std::sync::Mutex<Option<Agent>> = std::sync::Mutex::new(None);

pub fn flag_sign_in(agent: Option<Agent>) {
    *SIGN_IN.lock().unwrap_or_else(|e| e.into_inner()) = agent;
}

pub fn sign_in_pending() -> Option<Agent> {
    *SIGN_IN.lock().unwrap_or_else(|e| e.into_inner())
}

/// The muted footer label naming the connected agent.
pub fn status_label(chosen: Option<Agent>, installed: impl Fn(Agent) -> bool) -> String {
    match chosen {
        Some(agent) if installed(agent) => format!("{} · ready", agent.key()),
        Some(agent) => format!("{} · not installed", agent.key()),
        None => "no agent · /agent".into(),
    }
}

/// Errors that mean the CLI is installed but not signed in.
pub fn is_auth_error(message: &str) -> bool {
    let m = message.to_lowercase();
    ["/login", "sign in", "log in", "not authenticated", "unauthorized", "invalid api key", "401"]
        .iter()
        .any(|needle| m.contains(needle))
}

/// Opens the agent's own sign-in in a visible console window.
pub fn open_sign_in(agent: Agent) -> Result<(), String> {
    let path = resolve_cli(agent.cli()).ok_or_else(|| format!("{} is not installed.", agent.label()))?;
    let mut command = std::process::Command::new("cmd");
    command.args(["/c", "start", "", &path.display().to_string()]);
    command.args(&agent.login_command()[1..]);
    command.spawn().map(drop).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_keys_roundtrip_and_reject_unknown() {
        for a in AGENTS {
            assert_eq!(Agent::from_key(a.key()), Some(a));
        }
        assert_eq!(Agent::from_key("Claude"), None);
        assert_eq!(Agent::from_key(""), None);
        assert_eq!(Agent::Antigravity.cli(), "agy");
    }
    #[test]
    fn routing_target_follows_setting() {
        assert_eq!(route(None, |_| true), Err(NO_AGENT));
        assert_eq!(route(Some(Agent::Codex), |_| false), Err(NO_AGENT));
        assert_eq!(route(Some(Agent::Antigravity), |_| true), Ok(Agent::Antigravity));
        assert_eq!(sign_in_message(Agent::Codex), "Sign in to Codex");
        assert_eq!(status_label(Some(Agent::Claude), |_| true), "claude · ready");
        assert_eq!(status_label(Some(Agent::Codex), |_| false), "codex · not installed");
        assert_eq!(status_label(None, |_| true), "no agent · /agent");
    }
}
