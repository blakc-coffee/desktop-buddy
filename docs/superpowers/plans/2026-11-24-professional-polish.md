# Professional polish — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close the gaps between Buddy and a professional desktop app, using only free and local means.

**Architecture:** Each item is small and independent. Fullscreen awareness hooks foreground changes, so nothing polls. Diagnostics are a capped local log file. The hotkey becomes a setting chosen from presets. Accessibility adds names, not new UI. The CI and budget scripts are repo files only.

**Tech Stack:** Rust (Tauri 2, `windows` crate), React 19, PowerShell, GitHub Actions YAML (kept local, not pushed).

**Spec:** the polish list approved in chat on 2026-11-24 ("proceed with the polishing stuff except the paid one"). Items as listed there:
1. Fullscreen awareness.
2. First-run tour. This is folded into the greeting plan's first-run line.
3. Signed installer. Excluded: it is the paid one.
4. Update checks. Deferred: it needs releases published to GitHub, and the user said not to push.
5. Crash and error log.
6. Settings polish (hotkey, Start with Windows; pet size deferred as large).
7. Automated checks.
8. Performance budget.
9. Accessibility.

## Global Constraints
- No network calls, no telemetry and no uploads (AGENTS.md, CLAUDE.md).
- Zero idle CPU: no polling timers are added. Use event hooks.
- RAM stays under 40 MB working set at idle, and `scripts/measure.ps1` enforces it.
- Never push. Repo files such as CI YAML are committed locally only.
- Every task finishes with the native and UI suites passing and a local commit.

## Review Focus
1. **A borderless-fullscreen browser video, not just games:** the pet still hides. Test in Task 1 (`hide_for(QUNS_BUSY)` is true).
2. **Leaving fullscreen while the pet was hidden by the user:** it stays hidden. Test in Task 1 (`restore` keeps the user's hidden flag).
3. **A log file that grows forever:** it rotates at 256 KB. Test in Task 2.
4. **The chosen hotkey is taken by another app:** keep the old one and show the existing warning. Test in Task 3.
5. **Diagnostics leaking personal data:** the copied text holds only the version, OS build, RAM, settings flags and log lines. Log lines never include item titles or paths. Test in Task 2.

---

### Task 1: Hide the pet while a fullscreen app is in front

**Files:**
- Create: `src-tauri/src/fullscreen.rs`.
- Modify: `src-tauri/src/main.rs` (`mod fullscreen;`, install the hook in `setup`).
- Modify: `src-tauri/src/pet.rs` (`pub fn set_fullscreen_hidden(app, bool)`, which hides or shows without touching the user's own hidden flag).

**Interfaces:**
- `pub fn hide_for(state: i32) -> bool`: true for `QUNS_BUSY` (2), `QUNS_RUNNING_D3D_FULL_SCREEN` (3) and `QUNS_PRESENTATION_MODE` (4).
- `pub fn install(app: tauri::AppHandle)`: `SetWinEventHook(EVENT_SYSTEM_FOREGROUND)` on the main thread. The callback calls `SHQueryUserNotificationState`, then `pet::set_fullscreen_hidden`.

- [ ] **Step 1:** Write the failing test `fullscreen_states_hide_the_pet`: `hide_for(2)`, `hide_for(3)` and `hide_for(4)` are true, while `hide_for(1)` and `hide_for(5)` are false. Also add a pure test in `pet.rs`: `visible_after(user_hidden: bool, fullscreen: bool) -> bool` is true only for `(false, false)`.
- [ ] **Step 2:** Run. Expected: FAIL.
- [ ] **Step 3:** Implement. The pet tracks `fullscreen_hidden` separately from the user's hide, and shows when both are clear.
- [ ] **Step 4:** Run the suites. Expected: PASS.
- [ ] **Step 5:** Commit with message "Hide the pet while a fullscreen app is in front".

### Task 2: Local error log and "Copy diagnostics"

**Files:**
- Create: `src-tauri/src/diag.rs`.
- Modify: `src-tauri/src/main.rs` (install the panic hook in `setup`; command `diagnostics() -> String`).
- Modify: `src/App.tsx` (Settings gets a "Copy diagnostics" button that copies the returned text with `navigator.clipboard.writeText`).

**Interfaces:**
- `pub fn log(path: &Path, line: &str)` appends `"{iso time} {line}\n"`. When the file is over 262_144 bytes, it first renames it to `buddy.log.1`, replacing any old one.
- `pub fn report(version: &str, os: &str, ram_mb: u64, flags: &[(&str, bool)], tail: &str) -> String`.
- The panic hook logs `"panic: {location}"` plus the message, clipped to 300 chars.
- Errors already shown to the user, the `Err(e)` arms in `bar.rs execute`, log their message only.

- [ ] **Step 1:** Write failing tests:
  - `log_rotates_at_256kb`: write 300 KB of lines into a temp dir. `buddy.log` is under 262_144 bytes and `buddy.log.1` exists.
  - `report_has_no_item_data`: `report(...)` contains the version and flag names and nothing else passed.
- [ ] **Step 2:** Run. Expected: FAIL.
- [ ] **Step 3:** Implement. The log lives in `data_path/buddy.log`.
- [ ] **Step 4:** Run the suites, plus a UI test that the "Copy diagnostics" button calls `diagnostics`.
- [ ] **Step 5:** Commit with message "Keep a small local error log and copy diagnostics from Settings".

### Task 3: Choose the hotkey in Settings

**Files:**
- Modify: `src-tauri/src/main.rs` (lines 1054-1060 register from the setting `hotkey`, falling back to `Ctrl+Alt+Space`; command `set_hotkey(keys: String)`).
- Modify: `src/App.tsx` (Settings gets a `<select>` with presets `Ctrl+Alt+Space`, `Ctrl+Shift+Space`, `Alt+Shift+K`).

**Interfaces:**
- `const HOTKEYS: [&str; 3]`.
- `fn valid_hotkey(keys: &str) -> bool` accepts only `HOTKEYS` members.
- `set_hotkey` unregisters the old key and registers the new one. If registration fails, it re-registers the old key and returns `Err("{keys} is already used by another app.")`.
- `Ctrl+Alt+P` stays registered as it is today.

- [ ] **Step 1:** Write the failing tests `hotkey_presets_only` (`valid_hotkey("Ctrl+Alt+Space")` is true and `valid_hotkey("Win+L")` is false) and the UI test `settings hotkey select calls set_hotkey`.
- [ ] **Step 2:** Run. Expected: FAIL.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** Run the suites. Expected: PASS.
- [ ] **Step 5:** Commit with message "Let users pick the hotkey in Settings".

### Task 4: Accessibility names

**Files:**
- Modify: `src-tauri/src/pet.rs` (window title `"Kitty, your desktop buddy"` via `SetWindowTextW` at creation).
- Modify: `src-tauri/src/bar.rs` (bar window title `"Ask Kitty"`; the edit's accessible name via a `SetWindowTextW` on a hidden label sibling, or `EM_SETCUEBANNER` already covers the placeholder).

- [ ] **Step 1:** Write the failing test `windows_have_accessible_names`: the constants `PET_TITLE` and `BAR_TITLE` are non-empty and include "Kitty".
- [ ] **Step 2:** Run. Expected: FAIL.
- [ ] **Step 3:** Implement using the constants at window creation.
- [ ] **Step 4:** Run the suites. Expected: PASS.
- [ ] **Step 5:** Commit with message "Name the pet and bar windows for screen readers".

### Task 5: CI workflow and performance budget (local files)

**Files:**
- Create: `.github/workflows/ci.yml`. On `windows-latest`, for push and pull request, it runs `npm ci`, `npx tsc --noEmit`, `npm test`, `cargo test --release --manifest-path src-tauri/Cargo.toml`, then `npx playwright install msedge` and `npx playwright test --timeout=120000`.
- Create: `scripts/measure.ps1`. It starts the built exe, waits 10 s, and reads the working set. It prints `idle_mb=…` and exits 1 above 40 MB. Then it stops the process.

- [ ] **Step 1:** Run `powershell -File scripts/measure.ps1 -Exe src-tauri/target/release/desktop-buddy.exe`. Expected: `idle_mb=` under 40 and exit 0.
- [ ] **Step 2:** Validate the YAML with `npx --yes yaml-lint .github/workflows/ci.yml`, or a Python `yaml.safe_load` if that is unavailable. Expected: parses.
- [ ] **Step 3:** Commit with message "Add CI workflow and idle memory budget script". Do not push.

### Task 6: Build, relaunch, measure
- [ ] **Step 1:** Build, replace the installed exe and relaunch.
- [ ] **Step 2:** Run `scripts/measure.ps1` against the installed exe, record cold idle MB, then relaunch normally.
- [ ] **Step 3:** By hand, open a fullscreen video. The pet hides, then returns when you leave fullscreen.
