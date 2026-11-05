# Desktop Buddy PRD v1.2 — standalone pet and command bar

6 November 2026. This revision changes the primary interaction and file-hold duration following the user's latest description. v1.1 remains the baseline for the broader utility and agent scope; this document supersedes its primary-surface and temporary-handoff requirements. It does not claim the entire product is implemented.

## Product and constraints

A small desktop companion that runs independently of Codex. Clicking the pet opens a compact input bar near it, with modes/explicit shortcuts for local actions. The full tools window is secondary. No animation or custom pet art in this stage. Target host has 12 GB RAM. Native idle surfaces must not require WebView2, a local AI model, an agent worker, or a continuously running browser.

Keep the user’s weekly reserve: build expenditure at most 50 percentage points from the starting 90% remaining; conclude at a verified checkpoint as weekly remaining approaches 40%. No reset or paid credits without a separate explicit instruction.

## Primary journey

1. Launch the portable executable: native pet appears, without opening Codex or full tools.
2. Click the pet or use Ctrl+Alt+Space: show a small native command bar below it where space permits; use a clamped position above at screen edges.
3. Type a command or choose a mode. Enter executes, arrows choose results, Tab moves between input/results, Escape dismisses. IME composition must not execute a command accidentally.
4. Keep local everyday operations native. Open the detailed Tauri tools only through an explicit action.

## Command contract in 0.4

| Mode | Example | Behavior and scope |
| --- | --- | --- |
| Timer | `/t 25m review notes` | Whole-minute duration 1–1440; shared native timer service |
| Todo | `/d buy milk` | Local todo, no AI call |
| Reminder | `/r 10m stretch` | Relative whole-minute reminder; requires tray/core running |
| Folder | `/f assets` | Select a saved project root; bounded immediate-child folder lookup |
| Workspace | `/w project` | Match saved project folders; open in detected Cursor executable or Explorer |
| File copy | `/copy` | Copy held file references for pasting; report failures |
| Detailed tools | `/tools` | Open management/settings and optional managed AI tasks |

Aliases and help are documented in README. Unknown text must not become shell execution or an implicit AI request. Arguments are passed structurally to a validated executable/path. Cursor means Cursor IDE here; automatic discovery of its private workspace history is not implemented. Whole-disk indexing is a future adapter, not an implied property of `/f`.

## Temporary file handoff

One batch, up to 20 existing local files/folders, held as in-memory references for **120 seconds** from intake. Display count and remaining hold time. A new batch replaces the previous batch. A drag-out requests COPY only. Alt-drag moves a loaded pet. Cancelled transfers retain references until expiry. Expiry, clear, exit and restart discard references; they never delete/move originals. A transfer snapshot remains valid if its drag spans expiry. No file contents are cached/uploaded. Virtual attachments and network-share paths are unsupported in this stage.

Keyboard-accessible alternatives: Shelf file entries can be sent to the pet; the pet menu, command bar and Settings offer copy. Clipboard references created by the user's explicit copy action may outlive the pet hold. This must be clear rather than silently clearing the user's clipboard at expiry.

## Spotify lyrics: staged, not connected in 0.4

Preferred low-resource architecture: native Windows media-session metadata/timeline, native lyrics display, user-selected local LRC first, optional online provider after configuration and runtime verification. No Spotify SDK, embedded browser, private Spotify endpoints, credential-store inspection or local AI model. Media work should run only while the lyrics surface is enabled/visible; do not block its UI thread.

Candidate provider LRCLIB supports anonymous API access but requires an application identity containing name/version and a real project URL/contact. No such Buddy identity has been supplied yet. Do not invent it. Before enabling online lookup, explain that track metadata and ordinary network information reach the provider. Proposed bounds: one request per track signature, one in flight, five-second timeout, 256 KiB response limit, 16-entry/1 MiB cache, stale-result rejection, negative caching and Retry-After handling. These are proposed limits, not implemented features or permission to redistribute lyrics.

Runtime acceptance must establish real Spotify session visibility, metadata matching, pause/seek/track transitions and timeline behavior. Missing/incorrect timing must fall back to unsynchronised text or unavailable status. Current 0.4 still has the earlier manual LRC tool, not a Spotify connection.

## Focus restrictions: planned optional mode

Timers are implemented; `/focus` is currently a timer alias, **not app/site enforcement**. Evaluate an event-driven native foreground guard for a user-selected app list and browser-native rules for a selected site list. Require an obvious stop/emergency-off path and dependable cleanup at session end/restart. Do not kill apps, alter global host/firewall rules or claim security-grade enforcement. Browser integration/installation and its recovery model need separate verification; none is installed by 0.4.

## Acceptance and current evidence

- Native pet and bar open without Codex, WebView2 or model inference. Detailed tools remain explicit.
- Command parsing, duration bounds, literal argument handling and shared storage/timer behavior have automated coverage.
- Native bar programmatic open/close and actual two-minute holder expiry pass in the release executable. Shell-data roundtrip, copy-only metadata, original-content preservation and copy-after-expiry rejection pass.
- Actual click/typing/keyboard/IME, mixed-DPI edges and cross-window drag/Explorer paste still require manual UI acceptance. The automation tool blocked cross-window endpoints; Explorer app-access approval timed out. Existing destination files were excluded as evidence of a new paste.
- Measure release process-tree private working set/commit for tray, pet, native bar, tools and tools teardown. A short smoke test does not certify battery, eight-hour leaks, sleep/hibernate or pressure latency. Active AI runtime cost remains separate.

See the council review and build report for decision evidence, actual checks and remaining limits.
