# Claude chat as the default, with reminders and timers from plain language

## Intent
Typing in the bar should feel like chatting with Kitty. Plain text goes to Claude, and asking
for a reminder or timer in ordinary words ("remind me to call Saran at 5", "25 min timer for
reading") creates it, so the `/r` and `/t` syntax is no longer needed. Latency matters most.

## Decisions
- **Transport (user picked A):** a warm Claude Code CLI child process
  (`claude -p --input-format stream-json --output-format stream-json --include-partial-messages`).
  It uses the user's existing Claude sign-in, adds no new crate and stores no API key.
- **Safety:** no tools (`--tools ""`), no MCP, no setting sources, no slash commands, no saved
  sessions, and an empty working directory. Nothing is sent before Enter; warming up only
  starts the process.
- **Warm-up:** the bar opening starts the process. One process is one conversation, so bubble
  follow-ups continue the chat. Dismissing the bubble or 90 s idle ends it (bounded RAM,
  roughly 150–250 MB while alive).
- **Model:** `haiku`, chosen for first-token latency. Measured on this PC (Haiku, trivial
  prompt): cold 2.4 s to first token, 1.1 s after a 3 s pre-warm, 0.9 s on a warm second turn.
- **Fallback:** if the Claude CLI is missing or fails before anything is sent, the existing
  Codex path still answers.

## Reminders and timers
1. **Deterministic first (0 ms, no AI):** `commands::natural` recognises
   "remind me … in N min/hours", "remind me … at 5 / 5pm / 17:30", "timer/focus N min …"
   and "N min timer …".
2. **Claude second:** for anything else, Claude may reply with a single line
   `ACTION {"kind":"reminder"|"timer","title":…,"in_minutes":N}` or
   `{"kind":"reminder","title":…,"at":"HH:MM"}`. Rust validates the action
   (1–1440 minutes, title ≤ 240 characters, valid clock time) and computes the due time from
   the local clock. The model never does the date arithmetic. The raw ACTION line is never
   shown to the user.
3. The bubble confirms what was created, e.g. "Reminder set for 5:00 pm · call Saran".

## Non-goals
- No API-key mode.
- No tool use by Claude.
- No conversation history saved to disk.
