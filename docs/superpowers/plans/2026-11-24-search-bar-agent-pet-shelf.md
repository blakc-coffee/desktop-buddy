# Search bar: agent picker, pet picker, recent shelf — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** New users can connect one agent (Claude Code, Codex or Antigravity) from the search bar. `/pets` shows a live pet preview. The shelf keeps recently used files for one hour beside pinned items. `/music` leaves the bar.

**Architecture:** A small `chat_agent.rs` module owns the agent catalog and the `chat_agent` setting. `claude_chat.rs` keeps its warm stream-json actor but spawns whichever stream-json CLI is chosen (`claude` or `agy`). Codex keeps its existing path. `bar.rs` gains picker rows and a preview cell, both drawn in GDI. Recent files are `Item`s with kind `recent`, pruned lazily in `main.rs`.

**Tech Stack:** Rust (Tauri 2, `windows` 0.62 GDI, rusqlite), React 19 + Vite, Playwright UI tests.

**Spec:** `docs/superpowers/specs/2026-11-24-agent-picker-design.md`

## Global Constraints
- **Wait for the other session:** do not edit `src-tauri/src/pet.rs` or `src-tauri/src/bar.rs` until the floor-fuse session has committed its changes to them (`git status` shows them clean).
- **No early prompts:** never send a prompt before the user presses Enter on a question (AGENTS.md).
- **Warm one agent only:** only the chosen agent may be warmed. Opening the bar warms that one only.
- **No credentials:** Buddy never stores, reads or types credentials. Sign-in runs in the CLI's own console.
- **Agent strings:** setting key `chat_agent`, with values `claude` | `codex` | `antigravity`.
- **Recent limits:** recent files expire after 1 hour (3_600_000 ms). At most 20 recent items. The pinned `shelf` limit stays 50.
- **Paths only:** recent items store paths only and never read file contents.
- **Local commands:** timers, reminders and todos stay local (`commands::natural`), with or without an agent.
- **After each task:** run native tests with `export PATH="$HOME/.cargo/bin:$PATH"; cargo test --release --manifest-path src-tauri/Cargo.toml -j 2` from the repo root.
- **Commits:** commit locally after each task. Push at most 5 commits a day.

## Review Focus
1. **The chosen agent is uninstalled later:** the bar shows that agent's row as `not installed` and a question gives "Connect an agent: type /agent". No panic. Test in Task 3.
2. **A saved `chat_agent` value is unknown or misspelled:** treat it as no agent chosen. Test in Task 1.
3. **A recent file is deleted or moved on disk:** it stays listed until it expires. Open reports the existing "no longer exists" error, and Pin is refused. Test in Task 6.
4. **The same path is used repeatedly:** exactly one `recent` row, with its time refreshed. Test in Task 6.
5. **A path is both pinned and recent:** it shows only in Pinned. Pinning a recent item deletes the recent row. Test in Task 6.

---

### Task 1: Agent catalog and setting

**Files:**
- Create: `src-tauri/src/chat_agent.rs`
- Modify: `src-tauri/src/main.rs` (add `mod chat_agent;`; make `resolve_cli` `pub(crate)`)

**Interfaces:**
- Produces:
  - `#[derive(Clone, Copy, PartialEq, Eq, Debug)] pub enum Agent { Claude, Codex, Antigravity }`
  - `pub const AGENTS: [Agent; 3]`
  - `impl Agent`:
    - `fn key(self) -> &'static str` returns `"claude"` | `"codex"` | `"antigravity"`.
    - `fn label(self) -> &'static str` returns `"Claude Code"` | `"Codex"` | `"Antigravity"`.
    - `fn cli(self) -> &'static str` returns `"claude"` | `"codex"` | `"agy"`.
    - `fn from_key(&str) -> Option<Agent>`
    - `fn install_url(self) -> &'static str` returns `https://claude.com/product/claude-code`, `https://developers.openai.com/codex/cli` or `https://antigravity.google/download`.
    - `fn login_command(self) -> &'static [&'static str]` returns `["claude"]`, `["codex","login"]` or `["agy"]`.
    - `fn installed(self) -> bool`: `resolve_cli(self.cli())` is some and the extension is `exe` (except `codex`, where `resolve_cli` already maps the shim to the exe).
  - `pub fn chosen(app: &tauri::AppHandle) -> Option<Agent>`, read from the settings table.
  - `pub fn choose(app: &tauri::AppHandle, agent: Agent)`, which writes the setting.
  - `pub fn open_sign_in(agent: Agent) -> Result<(), String>`: spawns `cmd /c start "" <cli> <args…>` so the login runs in a visible console.

- [ ] **Step 1: Write the failing test** `agent_keys_roundtrip_and_reject_unknown` in `chat_agent.rs`:
  ```rust
  for a in AGENTS { assert_eq!(Agent::from_key(a.key()), Some(a)); }
  assert_eq!(Agent::from_key("Claude"), None);
  assert_eq!(Agent::from_key(""), None);
  assert_eq!(Agent::Antigravity.cli(), "agy");
  ```
- [ ] **Step 2: Run** `cargo test … agent_keys`. Expected: FAIL, with `chat_agent` not found.
- [ ] **Step 3: Implement** the interfaces above. `chosen` and `choose` use the same SQL as `pet::setting` and `pet::save`. Move those two into `main.rs` as `pub(crate) fn setting` and `pub(crate) fn save_setting` only if `pet.rs` is clean (Global Constraints). Otherwise duplicate the two short queries in `chat_agent.rs`.
- [ ] **Step 4: Run** the test. Expected: PASS.
- [ ] **Step 5: Commit** with message "Add agent catalog and chat_agent setting".

### Task 2: Warm actor drives `claude` or `agy`

**Files:**
- Modify: `src-tauri/src/claude_chat.rs` (`Worker::spawn`, event parsing, `ClaudeState`)
- Test fixture: `src-tauri/tests/fixtures/agy-stream.ndjson` (captured)

**Interfaces:**
- Consumes: `chat_agent::Agent`.
- Produces:
  - `ClaudeState::prepare_for(&self, agent: Agent)` warms the given agent. If a worker for a different agent is live, it is dropped first.
  - `ClaudeState::run(&self, agent: Agent, turn: Turn) -> Result<Reply, Failure>`.
  - `prepare()` with no argument is removed. Callers move to `prepare_for`.
  - `Failure` gets `pub auth: bool`, set when stderr or the result text matches a sign-in error (`/login|sign in|not authenticated|unauthorized|401/i`).

- [ ] **Step 1: Capture agy's stream format.**
  - Run `echo '{"type":"user","message":{"role":"user","content":"Say ok"}}' | agy -p --input-format stream-json --output-format stream-json > src-tauri/tests/fixtures/agy-stream.ndjson` once, by hand.
  - This is the developer's own test prompt, not user data.
  - Note which line types carry text deltas and which line carries the final result.
- [ ] **Step 2: Write failing tests:**
  - `agy_fixture_yields_text`: feed each fixture line through the same parser used for Claude (`turn`'s event handling, extracted as `fn event_text(agent: Agent, v: &Value) -> Event` with `enum Event { Delta(String), Done(String), Other }`). Assert that the concatenated deltas, or the `Done` text, contain `"ok"` (case-insensitive).
  - `auth_failure_is_flagged`: `Failure` built from stderr `"Please run /login"` has `auth == true`.
- [ ] **Step 3: Run.** Expected: FAIL.
- [ ] **Step 4: Implement.**
  - `Worker::spawn(agent)` uses `chat_agent::Agent::cli()` and the agent's flags.
    - Claude keeps its current args.
    - agy uses `-p --input-format stream-json --output-format stream-json --disable-slash-commands`. Add `--model` only if Step 1 shows a cheap default is needed.
    - Use the same empty temp folder and `CREATE_NO_WINDOW`.
  - `event_text` maps each CLI's lines.
    - If agy's lines match Claude's shapes, it is a shared match.
    - Otherwise add one `Agent::Antigravity` arm.
  - The actor remembers which agent its worker belongs to.
- [ ] **Step 5: Run** all `claude_chat` tests. Expected: PASS, including the existing ACTION tests.
- [ ] **Step 6: Commit** with message "Let the warm chat actor drive Claude Code or Antigravity".

### Task 3: Route questions to the chosen agent only

**Files:**
- Modify: `src-tauri/src/screen_helper.rs:406-470` (`quick_answer`, `claude_answer`), plus the `bubble_followup` live check (around line 899) and `dismiss_presentation`.

**Interfaces:**
- Consumes: `chat_agent::{chosen, Agent}` and `ClaudeState::{run, prepare_for}`.
- Produces:
  - `quick_answer` error strings, used by Task 4:
    - With no agent chosen: exactly `"Connect an agent: type /agent"`.
    - On an auth failure: exactly `"Sign in to {label}"`.
  - `pub(crate) const NO_AGENT: &str` holds the no-agent string.
  - `pub(crate) fn sign_in_needed(message: &str) -> Option<Agent>` recognises the sign-in string.

- [ ] **Step 1: Write the failing test** `routing_target_follows_setting`, as a pure function `fn route(chosen: Option<Agent>, installed: impl Fn(Agent) -> bool) -> Result<Agent, &'static str>`:
  - `route(None, |_| true) == Err(NO_AGENT)`.
  - `route(Some(Agent::Codex), |_| false) == Err(NO_AGENT)`.
  - `route(Some(Agent::Antigravity), |_| true) == Ok(Agent::Antigravity)`.
- [ ] **Step 2: Run.** Expected: FAIL.
- [ ] **Step 3: Implement** `route` and use it in `quick_answer`. Order of handling:
  1. `natural` runs first, unchanged.
  2. Claude or Antigravity → `ClaudeState::run(agent, …)`.
  3. Codex → the existing `warm_answer`, then `codex exec`.
  4. Remove the Claude→Codex fallback.
  5. An auth failure (`Failure.auth`) returns `"Sign in to {label}"`.
- [ ] **Step 4: Run** the full native suite. Expected: PASS (85+ passed).
- [ ] **Step 5: Commit** with message "Send questions only to the agent the user connected".

### Task 4: Agent picker, footer label and `/music` removal in the bar

**Files:**
- Modify: `src-tauri/src/commands.rs` (`Command::Agent`, parse `"/agent"` with no args; remove `Destination::Music` from the bar only, so the `/music` parse stays for the Tools window).
- Modify: `src-tauri/src/bar.rs`:
  - `Action`: add `Agent(Agent)`, `Install(Agent)` and `SignIn(Agent)`.
  - `refresh()`.
  - The bar-open prewarm (currently `ClaudeState::prepare()`).
  - `execute()`.
  - Footer drawing in `render_bar`.
  - `show_actions` (drop `"/music"` from the `["/music", "/codex"]` loop).

**Interfaces:**
- Consumes: Task 1 catalog, Task 3 strings, `ClaudeState::prepare_for`.
- Produces: `fn agent_rows(chosen: Option<Agent>, installed: impl Fn(Agent) -> bool) -> Vec<(String, Action)>`. Labels are exactly `"{label}      ready"` / `"{label}      not installed"`, with `" · connected"` appended to the chosen agent's row.

- [ ] **Step 1: Write the failing tests:**
  - `agent_command_parses`: `parse("/agent") == Ok(Command::Agent)`.
  - `agent_rows_mark_status`:
    - With chosen `Some(Claude)` and only Codex not installed:
      - Row 0 contains `"Claude Code"` and `"connected"`.
      - Row 1 contains `"not installed"` and its action is `Install(Codex)`.
      - Row 2's action is `Agent(Antigravity)`.
  - `actions_menu_has_no_music`: the completion list used by `show_actions` (extract it as `const MENU_COMPLETIONS: [&str; 1] = ["/codex"]`) does not contain `"/music"`.
- [ ] **Step 2: Run.** Expected: FAIL.
- [ ] **Step 3: Implement.**
  - **Showing the picker:** in `refresh()`, show `agent_rows` when the text is `/agent`, and also when the text is empty and `chosen()` is `None`. A muted header line reads `Connect an agent`.
  - **Enter on a row:**
    - `Agent(a)` → `choose`, `prepare_for(a)`, then the note `"{label} connected"`.
    - `Install(a)` → `open_path(a.install_url())`.
    - `SignIn(a)` → `open_sign_in(a)`.
  - **Answer failures:** when `quick_answer` returns `NO_AGENT`, show the agent rows. When `sign_in_needed` matches, show one row, `Sign in to {label}`, whose action is `SignIn(a)`.
  - **Footer:** draw `"{key} · ready"` (or `"no agent · /agent"`) right-aligned in muted ink, left of `n / total`. Use the existing `mix` color.
  - **Prewarm:** when the bar opens, call `prepare_for` only when `chosen()` is some, the agent is installed, and it isn't Codex. Codex keeps its existing warm path.
- [ ] **Step 4: Run** native tests. Expected: PASS. Then run `BUDDY_TOOLBAR_PREVIEW=<scratch>/bar.png cargo test … render_toolbar -- --nocapture` and check the image: the footer label is visible and doesn't overlap `n / total`.
- [ ] **Step 5: Commit** with message "Connect one agent from the search bar and drop music from its menu".

### Task 5: Pet picker preview and "Change pet…"

**Files:**
- Modify: `src-tauri/src/bar.rs` (the `Parsed::Pets` branch, `render_bar`, and the bar window width while `/pets` is open).
- Modify: `src-tauri/src/pet.rs:1410-1427` (right-click menu).

**Interfaces:**
- Consumes: `pet_art::draw_pose_accessory(dc, appearance, hover=false, dpi, AnimationSet::neutral_still_pose(), None, awake=false, None, nerdy=false)`.
- Produces:
  - `Bar.preview: Option<Appearance>`, set from the selected row whenever the selection changes while `/pets` is listed. `None` otherwise.
  - `pub fn open_with(app: &tauri::AppHandle, anchor: HWND, text: &str)` in `bar.rs`, which opens the bar prefilled.

- [ ] **Step 1: Write the failing test** `pets_list_sets_preview_to_selection`: build the pets choices with the existing `choices` path and select index 2. Assert `preview == Some(APPEARANCES[2])`. After `choices(vec![])`, assert `preview == None`.
- [ ] **Step 2: Run.** Expected: FAIL.
- [ ] **Step 3: Implement.**
  - **Preview cell:** while `preview` is some, `render_bar` draws a 96×104 logical cell inside the card at the right edge of the rows area, on the bar surface. It sets up the 96-DPI mapping like `pet.rs paint`, offsets the viewport origin to the cell, and calls `draw_pose_accessory`. Path text right-alignment ends left of the cell. The bar width does not change.
  - **Selection change:** ↑↓ invalidate the bar so the preview redraws. No timer and no animation.
  - **Right-click menu:** replace the "Choose pet" submenu and the `20..=26` arm with one item, id 20, labelled `Change pet…`, which calls `bar::open_with(&app, hwnd, "/pets")`.
- [ ] **Step 4: Run** native tests. Expected: PASS. Then extend `render_toolbar` with a `/pets` case and check the PNG: the pet is drawn whole inside the card, and no row text runs under it.
- [ ] **Step 5: Commit** with message "Pick pets from the search bar with a live preview".

### Task 6: Recent shelf

**Files:**
- Modify: `src-tauri/src/model.rs` (allow kind `recent` in `validate`).
- Modify: `src-tauri/src/main.rs` (new fns below; call `prune_recent` in `rows`' public readers; new Tauri command `pin_recent`).
- Modify: `src-tauri/src/bar.rs` (`Action::OpenFile` handler, `Parsed::Panel(Shelf)` listing).
- Modify: `src-tauri/src/pet.rs` (`pet_hold` success path, `copy_held` success path).
- Modify: `src/App.tsx` (Shelf page "Recent · last hour" group), `src/domain.ts` if the kind union lives there.
- Test: `tests/ui/*.spec.ts` (Shelf page).

**Interfaces:**
- Produces, in `main.rs`:
  - `pub(crate) fn note_recent(app: &tauri::AppHandle, paths: &[String], now_ms: i64)` upserts by path into kind `recent`. It skips paths already pinned as `shelf`, then prunes.
  - `fn prune_recent(db: &Connection, now_ms: i64) -> Result<(), String>` deletes `recent` items whose `created_at < now_ms - 3_600_000`, then all but the newest 20.
  - `#[tauri::command] fn pin_recent(core, id: String) -> Result<(), String>` turns the item into kind `shelf`, respecting the 50 limit, and refuses if the path no longer exists.

- [ ] **Step 1: Write failing native tests** on an in-memory DB, using the schema setup the existing tests use:
  - `recent_upsert_refreshes_instead_of_duplicating`: note `a` at t=0 and again at t=10. Expect one row with `created_at == 10`.
  - `recent_expires_after_an_hour`: note at t=0, prune at t=3_600_001. Expect zero rows.
  - `recent_caps_at_twenty`: note 25 distinct paths at t=0..24. Expect 20 rows, oldest gone.
  - `pinned_path_is_not_noted_recent`: a `shelf` item with path `a` exists, then note `a`. Expect zero `recent` rows.
  - `pin_recent_moves_to_shelf`: pinning makes kind `shelf`, leaves no `recent` with that path, and fails with `"A dropped path no longer exists."` for a missing path.
- [ ] **Step 2: Run.** Expected: FAIL.
- [ ] **Step 3: Implement** the three functions.
  - Call `note_recent` after a successful bar `OpenFile`, a successful `pet_hold` (dropped paths), and a successful `copy_held`.
  - Bar `/shelf` lists `recent` first, newest first, then pinned `shelf`.
  - Emit the existing items-changed event so the Tools window refreshes.
- [ ] **Step 4: Run** native tests. Expected: PASS.
- [ ] **Step 5: Write the failing UI test** `shelf shows recent above pinned with Pin`:
  - The fixture has one `recent` and one `shelf` item.
  - Expect the heading `Recent · last hour` before the first pinned `.shelf-item`.
  - The recent row has buttons `Pin`, `Open` and `Hold with pet`.
  - Clicking `Pin` invokes `pin_recent` with its id.
- [ ] **Step 6: Implement** the React group, reusing the `.shelf-item` markup. Replace the trash button with `Pin` for recent rows. Run `npx playwright test` and `npx tsc --noEmit`. Expected: pass and clean.
- [ ] **Step 7: Commit** with message "Keep recently used files on the shelf for an hour".

### Task 7: Build, relaunch and hand check

- [ ] **Step 1: Build.** Run `powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/build.ps1` from bash. Expected: `Finished release`.
- [ ] **Step 2: Install and relaunch.** Stop `desktop-buddy.exe`, copy `src-tauri/target/release/desktop-buddy.exe` to `C:\Users\dharu\anything\desktop-buddy\`, and start it.
- [ ] **Step 3: Check the bar by hand:**
  - With `chat_agent` cleared, an empty bar shows the three agent rows.
  - Pick Antigravity and ask "what is 2+2". An answer appears.
  - `/agent`, then pick Claude, and ask again.
  - `/pets` shows the preview, and ↑↓ swap it.
  - Open a file from `/f`, then `/shelf` lists it under recent.
- [ ] **Step 4: Measure.** Record the cold and warm time to first token for the chosen agent, plus Buddy's working set with the agent warm. Report these as measured, not as targets.
