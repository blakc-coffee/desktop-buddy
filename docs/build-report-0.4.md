# Desktop Buddy 0.4 — standalone command-bar checkpoint

6 November 2026. Windows x64 portable unsigned executable. No installer, service, autostart entry, local AI model or animation engine. Local functions operate independently of Codex and its CLI.

## Delivered

Native pet → native compact command bar. Explicit timer/todo/reminder, saved-root folder lookup, saved workspace opening in detected Cursor/Explorer, held-file copy and detailed-tools commands. Ctrl+Alt+Space opens the bar. Cursor is detected through native executable paths; its private workspace history is not read. Search is immediate-child lookup within a selected saved root, not whole-disk indexing. `/focus` is currently a timer alias, not app/site enforcement.

Temporary holder: one batch of up to 20 existing local file/folder references, 120-second monotonic deadline, native OLE COPY-only drag source, immutable in-progress transfer snapshot, and keyboard copy alternative with Preferred DropEffect=COPY. Expiry clears references, never original files. Explicit clipboard copies can outlive the hold. Native UI requires Windows device acceptance; no animation/custom art was added.

## Verification

- Production TypeScript build, 6 TypeScript domain tests and 21 optimized Rust tests pass. New tests cover explicit commands, duration/title bounds, shell-data Unicode roundtrip, COPY metadata, unchanged originals, expiry bounds and source cancellation/shutdown.
- Final executable: native bar opens/closes through the application's public command API; two files are held for a real two-minute interval; native clipboard handoff succeeds; expiry clears references; originals match their initial bytes; copying after expiry fails.
- Final native local-tool regressions pass: SQLite, timer pause/resume, durable reminder attention, bounded folder search, backup merge without duplicates, panel teardown/recreation and single instance. Installed Codex CLI read-only discovery returns 25 stored sessions.
- Managed-task fixture checks pass for explicit submission, approvals/questions, receipts, stale requests, authoritative interruption, unknown disconnects, duplicate protection, backup restoration and startup cancellation. These perform no model inference or commands.
- Source review corrected hidden copy errors, result-cap filtering, IME Enter handling, Tab navigation and startup/control lifetime issues. Source review is not GUI acceptance.

The native pet was visually inspected. Automated cross-window drag was rejected by the UI tool; Explorer app-access approval timed out. Click/typing/keyboard/IME behavior, actual drag-in/out, Explorer paste, DPI edges, high contrast and sleep still need manual checks. A previous destination-file existence check was not accepted as evidence of a new Explorer paste; final QA uses a fresh destination and explicitly records that paste remains unverified.

## Short idle measurement

11.70 GiB usable RAM, 8 logical processors, WebView2 156.x. Release build with no dev server, debugger or attached AI runtime. Each phase: 10-second warmup and 15 one-second samples. Windows private working set/commit are attributed to the Rust process and WebView2 descendants.

| Phase | Processes | Mean private working set | Mean private commit |
| --- | ---: | ---: | ---: |
| Tray only | 1 | 2.35 MiB | 3.11 MiB |
| Pet only | 1 | 2.35 MiB | 3.11 MiB |
| Native command bar | 1 | 2.40 MiB | 3.22 MiB |
| Detailed tools | 7 | 86.11 MiB | 172.00 MiB |
| Tools destroyed | 1 | 3.86 MiB | 5.43 MiB |

Normalized idle CPU rounded to 0.000% at sample resolution, not literally zero. This small-state smoke test excludes active editors/AI, large histories, eight-hour leaks, battery impact, memory pressure and latency p95. The native-bar lifecycle was separately confirmed through status reporting; typing was not automated. JSON summaries accompany the checkpoint.

## Deferred scope

The user explicitly deferred online lyrics/client identification. Spotify metadata/synchronization and online lookup are not connected. Focus app/site enforcement remains a separate optional integration; no restrictions were installed or system security/network settings changed. Antigravity, full indexed search, snooze/autostart/undo and the broader v1.1 features remain staged. See PRD v1.2 and council review for the updated plan.

Manual acceptance: click pet; try `/t 1m test`, `/d test task`, `/r 1m test`; configure a project in `/tools`, try `/f` and `/w`; drop disposable files into the pet, copy/drag into a different folder, compare contents and originals, cancel a drag and wait two minutes. Native tray menu provides recovery and Quit.
