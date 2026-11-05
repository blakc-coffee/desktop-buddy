# Desktop Buddy 0.4

A standalone Windows utility with a native mini pet and native command bar. Codex does not need to be open or installed for local tools. Tauri/React supplies the optional detailed tools window; Rust/SQLite runs the core. Custom pet artwork and animations remain deferred.

## Run

Normal launch shows the pet. Click it or press **Ctrl+Alt+Space** to open a small command bar near it. The bar opens below when space permits, otherwise above; it stays inside the monitor work area. Enter executes and Escape closes. Folder and workspace matches appear one at a time beside the input. Use Up and Down to cycle them. `/tools` opens detailed management. The pet and command bar use no WebView2 process. Right-click the pet for its menu. The tray and Settings provide show/hide recovery; the pet menu can reset position. Quit an older Buddy version before launching this build.

## Quick commands

Use Up and Down to browse commands in an empty bar or cycle matches as you type. Tab fills the highlighted command. Enter fills a suggestion or runs a complete command.

| Command | Action |
| --- | --- |
| `/t 25m review notes` | Start a custom whole-minute timer |
| `/d buy milk` | Save a local todo |
| `/r 10m stretch` | Save a reminder for ten minutes from now |
| `/f folder name` | Choose a saved project root and find matching immediate child folders |
| `/w project name` | Find saved workspaces; open in Cursor when detected, or choose Explorer |
| `/copy` | Copy currently held file references for pasting |
| `/tools` or `/settings` | Open full tools/settings |
| `/help` | Show command choices |

`/timer`, `/todo`, `/remind`, `/find`, `/workspace` and `/focus` (timer only) are aliases. Timer/reminder durations are 1–1440 whole minutes. No command is evaluated as a shell script. Folder search is bounded to the selected root's immediate children, not a whole-disk index. Workspace lookup uses folders saved in Buddy; it does not read Cursor's private history. A missing Cursor executable produces an error and an Explorer option remains available.

## Temporary file handoff

Drop one batch of up to 20 local files/folders onto the pet. While loaded, the pouch shows a count and **two-minute countdown**. Drag a loaded pet into a compatible folder/app to request a copy; Alt-drag moves the pet itself. A successful drag or clipboard copy empties the pouch. Cancelled drags retain references until expiry, and a new batch replaces the previous batch. Expiry, clearing, exit and restart discard references only. Original files are never deleted or moved by Buddy. No file contents are cached or uploaded. Virtual attachments and network-share paths are unsupported. File entries on Shelf also offer **Hold with pet**. The pet menu, `/copy`, and Settings offer a keyboard copy alternative; after an explicit copy, clipboard references can outlive the pet's hold.

Native pet appearance was inspected. The final build passes native bar open/close, actual two-minute hold expiry, copy-only clipboard handoff, Unicode shell-data roundtrip, original-content preservation and copy-after-expiry rejection. Keyboard/IME interaction, actual cross-window dragging and Explorer paste remain manual acceptance checks. The UI tool rejected cross-window drag endpoints and Explorer app-access approval timed out; no successful Explorer paste is claimed.

## Tools and lifecycle

Detailed tools require Windows WebView2. Closing that window releases its WebView and leaves the native pet, bar, tray and reminder scheduler running. Quit from the tray to exit. An attached managed Codex connection must be ended before quitting, including after source-reported completion. `--panel` explicitly opens full tools; `--commands` opens/toggles the native bar; `--close-commands` closes it; `--tray` starts without showing either surface.

This is a local unsigned build, not a signed installer. No startup entry or Windows service is installed. It makes no AI inference requests automatically.

## Included

- Local todos with completion and editing.
- Named focus timers with pause/resume/reset and durable state.
- One-off reminders, persistent Needs me alerts, and attempted Windows notifications.
- Quick capture: ordinary text or `todo review tests`; `timer 25m review tests`; `remind tomorrow 10:00 review tests` (24-hour local time). Reminder capture opens a review form before saving.
- Saved project folders and manually written resume notes.
- Bounded immediate-directory search within a saved project. It is not a whole-disk or document-content search.
- Temporary text shelf and native file/folder drops. These store references; they do not move or upload files.
- JSON backup export and merging import. Existing IDs are kept. Running timers import paused. Export before deleting important notes.
- Dark, light and system themes, keyboard capture with Ctrl+K, compact layouts.
- On-demand, read-only Codex stored-session discovery through the installed native CLI. It sends no prompt, resumes no session, and does not establish live desktop status.
- Explicitly submitted managed Codex tasks, with manual approval/question handling, source-reported receipts and editable follow-up todos. See the connection lifecycle below.
- Local manual LRC reader. It does not synchronize to Spotify playback.

## Managed Codex tasks

Choose **New task**, select a saved project, load the installed CLI's model catalog and choose a model, then explicitly submit your prompt. Read-only access is the default; workspace-write access requires opting in. Loading a catalog is distinct from submitting a task. Everyday local tools do not make model calls, and no provider prompt is submitted automatically.

The app attaches one app-server runtime at a time. This managed connection is separate from existing Codex desktop work: existing desktop sessions are stored history only, with no verified live monitoring or attachment.

Approval requests require a manual approve-once or decline decision; questions require a manual answer. Requesting cancellation does not establish cancellation: the app waits for a source-reported interrupted outcome. If the connection stops without an authoritative outcome, the task outcome is unknown.

A source-reported completion retains the connection until you choose **End connection**. Completion alone must not implicitly kill background work. Ending a connection is an explicit lifecycle action, not evidence that the task completed or was interrupted. Quit remains blocked until the connection ends.

Receipts are stored locally and describe source-reported outcomes, rather than independently verified correctness. A receipt can become an editable follow-up todo. Version 2 backups include receipts; records that were active when backed up restore as unknown and are not automatically replayed or resumed.

## Pending from the full PRD

Live monitoring of existing Codex desktop work, Antigravity integration, todo-linked focus history, Everything integration, automatic Spotify playback/lyrics, snoozing, autostart controls, and undo for deletion remain pending. Antigravity CLI was not detected in the initial toolchain inventory. Managed Codex tasks do not establish those integrations; manual LRC reading and stored history remain separate capabilities.

## Verification status

The build passes 21 Rust tests, 6 TypeScript tests and native local-tool/protocol-fixture checks. The installed Codex CLI returned 25 stored sessions through read-only discovery. No real inference task or live-provider approval round trip was tested. See `docs/build-report-0.4.md` and accompanying JSON for exact checks, native interaction limits and measured resources. Active Codex runtime cost is separate.

## Data and reminder behavior

The native database is `buddy.sqlite3` in the application-local data folder (shown in Settings). Browser preview uses separate localStorage and cannot operate when the preview is closed. The native scheduler works while the tray app is running. Quit stops it; overdue items are recovered next launch. Closed-app OS scheduling is not implemented. Windows may suppress notification banners; the durable alert remains in Needs me.

Running native timers use Windows uptime within the current app run and include sleep. A low-frequency reconciliation check while deadlines exist bounds delayed evaluation after resume; actual Windows sleep/hibernate behavior still needs a dedicated device test. After app restart, remaining time falls back to the saved UTC deadline and is capped at the original duration. Arbitrary clock changes across restart can affect that fallback. Paused timers do not count down.

Normal creation stops at 1,900 records; 100 records are reserved for recovery/alerts. All producers stop at 2,000 records or 3 MiB total serialized record payload. Capacity failure is explicit rather than hiding older records. A full store can prevent alert persistence until space is freed. The shelf has 50 entries; individual notes are limited to 64 KiB. Backups accept at most 5 MiB and 2,000 records. Backups are private plaintext; keep them somewhere appropriate.

## Development

Requirements: Node 22+ (Node 24 used here), Rust stable, Visual C++ build tools and WebView2. From this directory:

```powershell
npm ci
npm run dev
npm run build
npm test
cargo test --release --manifest-path src-tauri/Cargo.toml -j 2
./scripts/build.ps1
```

`build.ps1` builds an optimized executable with two compiler jobs. It can reuse the isolated sibling `tools/cargo` and `tools/rustup` directories in the original workspace; otherwise it uses an installed Rust toolchain. Runtime dependencies are locked in package-lock.json and Cargo.lock. Prettier is development-only.

Optional browser/native QA uses Playwright with installed Edge. Install it separately for development or set `BUDDY_PLAYWRIGHT_PATH` to an existing Playwright module. Run a local preview on port 1421 before `node scripts/browser-qa.cjs`. The native QA script requires the release executable, uses its own workspace data folder, and temporarily enables a loopback WebView2 debugging endpoint for testing only. `scripts/measure.ps1` runs a short release-build tray/panel/teardown smoke benchmark; it is not the full eight-hour resource acceptance suite.

## Native switches

`--tray` starts without a panel. A second launch with `--close-panel` closes only the panel. A second launch with `--quit` exits this app. `BUDDY_DATA_DIR` can select an absolute isolated data directory for testing. These do not affect other applications.

See the separate build report for actual completed checks and measured resource results. Targets in the PRD are not achieved-performance claims.
