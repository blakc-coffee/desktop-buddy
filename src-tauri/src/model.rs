use serde::{Deserialize, Serialize};
use std::{
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};
static IDS: AtomicU64 = AtomicU64::new(0);
pub fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}
pub fn new_id() -> String {
    format!(
        "{}-{}-{}",
        now(),
        std::process::id(),
        IDS.fetch_add(1, Ordering::Relaxed)
    )
}
pub fn ticks() -> u64 {
    #[cfg(windows)]
    {
        unsafe { windows_sys::Win32::System::SystemInformation::GetTickCount64() }
    }
    #[cfg(not(windows))]
    {
        now() as u64
    }
}
#[derive(Default, Clone, Debug, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Item {
    pub id: String,
    pub kind: String,
    pub title: String,
    pub done: bool,
    pub created_at: i64,
    pub due_at: i64,
    pub path: String,
    pub note: String,
    pub source: String,
    pub project_id: String,
    pub state: String,
    pub duration_ms: i64,
    pub remaining_ms: i64,
    pub started_tick: u64,
    pub run: String,
}
pub fn validate(i: &Item) -> Result<(), String> {
    if ![
        "todo",
        "reminder",
        "timer",
        "project",
        "shelf",
        "attention",
        "receipt",
    ]
    .contains(&i.kind.as_str())
    {
        return Err("Unsupported item type.".into());
    }
    if i.title.trim().is_empty()
        || i.title.len() > 240
        || i.note.len() > 65536
        || i.path.len() > 32768
        || i.source.len() > 1024
        || i.project_id.len() > 128
        || i.run.len() > 128
        || i.state.len() > 64
    {
        return Err("Item text is empty or exceeds its size limit.".into());
    }
    if !(0..=8_640_000_000_000_000).contains(&i.created_at)
        || !(0..=8_640_000_000_000_000).contains(&i.due_at)
    {
        return Err("Item date is outside the supported calendar range.".into());
    }
    if i.kind == "timer"
        && (!(0..=86_400_000).contains(&i.duration_ms)
            || !(0..=86_400_000).contains(&i.remaining_ms)
            || !["running", "paused", "finished"].contains(&i.state.as_str()))
    {
        return Err("Invalid timer in backup.".into());
    }
    Ok(())
}
pub fn clip_title(s: &str) -> String {
    if s.len() <= 240 {
        return s.to_string();
    }
    let mut end = 237;
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}…", &s[..end])
}
pub fn remaining(i: &Item, wall: i64, tick: u64, run: &str) -> i64 {
    if i.state != "running" {
        return i.remaining_ms.max(0);
    }
    if i.run == run && tick >= i.started_tick {
        i.remaining_ms
            .saturating_sub((tick - i.started_tick).min(i64::MAX as u64) as i64)
            .max(0)
    } else {
        i.due_at.saturating_sub(wall).clamp(0, i.duration_ms)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn long_file_labels_stay_valid_without_breaking_utf8() {
        let name = "猫".repeat(100);
        let title = clip_title(&name);
        assert!(title.len() <= 240);
        let i = Item {
            kind: "shelf".into(),
            title,
            ..Default::default()
        };
        assert!(validate(&i).is_ok());
    }
    #[test]
    fn wall_clock_change_does_not_extend_same_run_timer() {
        let i = Item {
            state: "running".into(),
            run: "one".into(),
            remaining_ms: 60000,
            started_tick: 1000,
            ..Default::default()
        };
        assert_eq!(remaining(&i, 999999999, 31000, "one"), 30000);
        assert_eq!(remaining(&i, 0, 31000, "one"), 30000);
    }
    #[test]
    fn restart_uses_bounded_wall_deadline() {
        let i = Item {
            state: "running".into(),
            run: "old".into(),
            due_at: 100000,
            duration_ms: 60000,
            ..Default::default()
        };
        assert_eq!(remaining(&i, 70000, 1000, "new"), 30000);
        assert_eq!(remaining(&i, 0, 1000, "new"), 60000);
        assert_eq!(remaining(&i, 200000, 1000, "new"), 0);
    }
    #[test]
    fn paused_timer_does_not_elapse() {
        let i = Item {
            state: "paused".into(),
            remaining_ms: 1234,
            ..Default::default()
        };
        assert_eq!(remaining(&i, i64::MAX, u64::MAX, "other"), 1234);
    }
    #[test]
    fn oversized_or_executable_item_type_rejected() {
        let mut i = Item {
            title: "A".into(),
            kind: "shell".into(),
            ..Default::default()
        };
        assert!(validate(&i).is_err());
        i.kind = "todo".into();
        i.note = "x".repeat(65537);
        assert!(validate(&i).is_err());
    }
}
