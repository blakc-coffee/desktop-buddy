use crate::commands::Destination;
pub const CODEX_CHATS: bool = false;
pub const WORKSPACES: bool = false;
pub fn destination_enabled(destination: Destination) -> bool {
    match destination {
        Destination::Codex | Destination::Ask => CODEX_CHATS,
        Destination::Projects | Destination::Resume => WORKSPACES,
        _ => true,
    }
}
pub fn require_codex_chats() -> Result<(), String> {
    if CODEX_CHATS { Ok(()) } else { Err("Codex chats are archived for now.".into()) }
}
#[cfg(test)] mod tests { use super::*; #[test] fn archived_routes_are_inaccessible_without_disabling_screen_tools() {
    for destination in [Destination::Codex,Destination::Ask,Destination::Projects,Destination::Resume] { assert!(!destination_enabled(destination)); }
    assert!(destination_enabled(Destination::ScreenExplain));assert!(destination_enabled(Destination::ScreenCapture));assert!(destination_enabled(Destination::Settings));
} }
