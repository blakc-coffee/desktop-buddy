# Desktop Buddy 0.2.0 — verified checkpoint

5 November 2026. Windows x64, Tauri 2 / Rust / React / SQLite. Portable unsigned executable: 4,946,944 bytes (4.72 MiB). The distributed SHA256.txt identifies the exact tested executable. No startup entry, service, installer or local AI model is installed.

## Delivered

Local todos, focus timers, one-off reminders and durable alerts; quick capture; project folders/resume notes; bounded immediate-directory search; file-reference drops and text shelf; backup import/export; themes and manual LRC reading remain available.

0.2 adds explicit Buddy-managed Codex requests, user-loaded model selection, read-only default/workspace-write opt-in, source activity, manual approve-once/decline and non-secret questions, acknowledged cancellation, source-reported receipts and editable follow-up todos. Existing Codex desktop sessions remain separate stored history. Completion retains the attached runtime until explicit End connection. Quit is blocked while attached. No prompt is automatically resumed or replayed.

History is bounded to 50 receipts / 1 MiB total serialized payload, with 64 KiB reserved for each active record. Actual capacity depends on record sizes. Telemetry and decision details are bounded; output can be truncated. Export before removing old receipts. Backups are plaintext and version 2 includes receipts; importing active records makes them unknown without reconnecting.

## Verification

- Production TypeScript/Vite build, 6 TypeScript domain tests and 16 optimized Rust tests pass. Coverage includes clock/restart handling, transactional storage, capacity reservation, UTF-8 output bounds, terminal state and approval/question constraints.
- Browser regression checks pass for capture, todo persistence/completion, timers, reminder review, project notes, backup, theme, Ctrl+K and a 390px viewport, with no page errors recorded.
- Final native executable passes SQLite persistence, timer pause/resume, reminder attention, bounded search, backup merging without duplicates, panel destruction/recreation and single-instance reopening. Read-only discovery through installed Codex CLI 0.159.2 returned 25 stored sessions.
- A hermetic CLI fixture passes the native request-to-receipt workflow, manual command approval and answer, removal of answered interactions, late-event/stale-answer rejection, completed-task cancellation protection, source-acknowledged interruption, unknown disconnect outcome, duplicate guard, active backup restoration and stopping startup before prompt delivery.
- Independent source review identified concurrency, stale decision and storage-headroom defects; fixes were followed by the release tests above. The reviewer did not independently run native/provider tests.

Fixture tests execute no commands and make no model calls. Screenshots contain QA data, not preloaded user tasks. All native tests use isolated workspace databases. No real inference task, live-provider approval round trip, or live desktop attachment was tested. Protocol fixture success does not establish full provider compatibility. See managed-codex-contract.md for the contract and unsupported interactions.

## Short idle resource measurement

Host: 11.70 GiB usable RAM, 8 logical processors, installed WebView2 156.x. Release executable with no dev server, debugging endpoint or attached agent. Each phase uses a 10-second warmup and 15 one-second samples. Windows PROCESS_MEMORY_COUNTERS_EX2 measures private working set and private commit for the Rust process plus WebView2 descendants.

| Phase | Processes | Mean private working set | Mean private commit |
| --- | ---: | ---: | ---: |
| Tray only | 1 | 1.81 MiB | 2.40 MiB |
| Panel open | 7 | 83.66 MiB | 172.47 MiB |
| Panel destroyed | 1 | 3.58 MiB | 5.02 MiB |

Normalized mean CPU rounded to 0.000% at this idle sampling resolution. This does not establish zero CPU use. These small-state smoke results exclude active Codex runtime cost and do not establish eight-hour leak behavior, battery impact, memory-pressure response, large-history responsiveness or latency p95.

## Remaining scope and device checks

Automatic Spotify playback/lyrics, Everything indexing, Antigravity/other-agent adapters, live monitoring of existing desktop sessions, todo-linked focus history, snooze, autostart and delete undo remain pending. Antigravity and Everything were not found in the initial known-path/PATH inventory.

Global shortcut focus, actual sleep/hibernate and clock-change recovery, notification banner visibility, file-drop UX, native backup download location and real-provider/background-job behavior still need device acceptance. The durable reminder alert was verified; Windows can suppress banners. Quit stops scheduling, with overdue recovery next launch.

## Usage budget

Build started with 90% weekly remaining. The last pre-packaging meter showed 74% remaining, approximately 16 account-wide percentage points used since that baseline. This stays within the user's maximum 50-point build expenditure and preserves more than the requested 35% weekly reserve. No reset or paid credits were used. No model inference was performed during connector verification.
