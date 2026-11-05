# Desktop Buddy — utility-first PRD v1.1 — council reviewed

Date: 5 November 2026 · Owner: Dharu · Status: council-selected proposed implementation baseline; feasibility gates outstanding

Companions: [Original research](desktop-buddy-research-v1.0.md) and [council decision, verified additions, and revision record](desktop-buddy-council-review-v1.1.md). V1.0 is preserved unchanged.

## 1. Product decision

Build a Windows desktop utility for seeing agent work, acting on items that need attention, finding files, and managing small personal tasks. Use a tray icon, a configurable keyboard shortcut, and one on-demand panel. A pet, floating character, custom cat artwork, and animation are deferred. The product must work completely without them.

Recommended stack: Tauri 2, Rust, React, TypeScript, Vite, and SQLite. This is an architecture recommendation based on the discussion, subject to an early release-build feasibility and resource gate. Do not interpret choosing Tauri as proof of low RAM use.

No application has been implemented or benchmarked for this PRD. Numeric budgets below are proposed engineering acceptance targets, not measurements or vendor guarantees.

## 2. Baseline and scope provenance

Confirmed in the chat: Spotify lyrics, folder search, current AI tasks and progress, timers, todos, reminders, and potentially sending requests directly to agents or other services. Primary agents: Codex and Antigravity, with the ability to change agents later. The laptop has 12 GB RAM.

Explicitly accepted additions: Needs me inbox, project shortcuts, quick capture, agent-to-todo actions, focus sessions, drag-and-drop attachments, resume notes, completion receipts, and a temporary shelf.

Latest correction: utility first; defer cat/pet artwork and animation. A default Codex pet was suggested only if useful. It is not a requirement or dependency. Use a neutral static icon initially; any later third-party asset needs a confirmed supported import/reuse path.

Suggested but not accepted additions remain outside the committed release: follow-up queues, project scratchpads beyond resume notes, arbitrary build/download monitoring, snippets, a daily briefing, feeding/petting, accessories, and automatic cross-agent handoffs. Presentation/privacy controls are proposed quality requirements rather than a new product module.

Windows 11 x64 is the proposed first supported target. Exact OS, processor, SSD, installed agent versions, and normal background workload have not been confirmed. Windows 10, ARM64, macOS, Linux, mobile, team accounts, and cloud sync are excluded from the first supported release.

## 3. Problem, user, and outcomes

The initial user is a developer switching between agents, projects, Explorer, music, and personal tasks. The working hypothesis is that manual status checking and scattered follow-ups cause avoidable context switching. No user interviews or measured baseline have been collected.

The tool should answer four questions quickly: What needs me? What is still running? Where is my project or file? What do I need to do next?

Success means the user can identify a supported agent's state without opening every agent app; reach a project in a few actions; capture a todo or reminder without navigating away; and leave the tool resident without noticeable sustained CPU load or memory growth. It must remain useful when agents or the network are unavailable.

## 4. Product principles

1. Resource use is a release criterion. A feature is not complete until its background cost is measured.
2. Prefer events, native services, and existing indexes over polling or rebuilding infrastructure.
3. State is authoritative in Rust and SQLite. The UI can disappear without losing timers or task state.
4. Report evidence, not invented certainty. Unknown, disconnected, and unsupported are real states.
5. AI is optional infrastructure. No model call for timers, filename search, reminder scheduling, or ordinary UI actions.
6. Keep agent behavior under the original agent's permissions and approval policies.
7. Load a module when needed; disable it without breaking unrelated modules.

## 5. Experience and information architecture

Entry points: tray click, Start menu, notification click, and a user-configurable global shortcut. Handle shortcut conflicts with an explanation and a replacement choice; do not silently replace another app's shortcut. Suggested initial shortcut: Ctrl+Alt+Space, subject to conflict detection.

The main panel contains a command/search input and four destinations: Activity, Tasks, Projects, and Music. Activity defaults to Needs me followed by running work. Tasks contains todos, timers, reminders, and focus sessions. Projects contains shortcuts, resume notes, and the temporary shelf. Music contains current playback and lyrics. Settings lives behind a standard menu.

Do not keep a dashboard, settings window, command palette, and music view alive as separate webviews. Reuse the panel. A separately pinned lyric strip is an optional user-opened surface with its own resource cost, not a startup default.

Close normally returns to the tray. Quit is a distinct tray/menu action and explains which background features stop. The first close explains tray behavior once. Fullscreen overlay and always-on-top are off by default. There is no persistent desktop mascot in v1.

Use the saved warm-charcoal reference as a starting visual direction: restrained neutral surfaces, strong text hierarchy, compact lists, a single accent. Interpret it for a utility rather than copying a marketing layout. Use system fonts, readable tracking, semantic status text, and accessible contrast; do not inherit the reference's tiny labels or decorative gradient. Support system/light/dark themes.

## 6. Core journeys

### J1: Track agent work

Connect a supported provider → see a capability summary → select a project/session → receive source events → inspect activity → act on a question or open the original provider. A dropped connection changes connection health, not the task's outcome. Reconnection reconciles state before issuing more actions.

### J2: Start a request

Choose provider and workspace → write prompt → optionally attach files → review exactly what will be sent → submit once → see submitted/running/error state. The interface distinguishes a new buddy-managed session from work observed in another app. Retrying an ambiguous submission must not silently duplicate a task.

### J3: Capture and focus

Open shortcut → type a todo or reminder → confirm parsed date/time → save → start a linked focus session if desired. After restart or sleep, show the correct remaining time or overdue state without replaying every missed tick.

### J4: Resume a project

Select project → see the last manually saved next step and source session links → open configured folder/editor/terminal actions individually or as an explicitly configured bundle.

### J5: Find and attach

Search filenames/folders → open or reveal a result → drag a file into the panel or use a picker → add it to the shelf or request. A referenced folder is not automatically uploaded recursively.

### J6: Follow lyrics

Enable Music → select the Spotify media session when multiple sessions exist → look up a matching track → display synchronized lines if available → handle pause/seek/track changes. With no match, offer plain lyrics, manual LRC, or no-lyrics state. Never invent missing lyrics.

## 7. Functional requirements and acceptance

Phase labels: A = feasibility; B = core usable release; C = accepted utility additions; D = music/full agreed scope; Later = deferred. Phase placement changes delivery order, not the user's accepted scope.

### FR-01 — Shell and lifecycle [B]

Provide one app instance, tray menu, shortcut, open/close/quit, and recoverable window position. Start at login is an explicit setting, initially off. On monitor removal or DPI change, restore the panel into a visible work area. Acceptance: second launch focuses the existing instance; repeated open/close does not duplicate handlers or windows.

### FR-02 — Provider setup and capability reporting [A/B]

Show installed provider/version, authentication state without exposing secrets, supported operations, and setup failures. Each connector independently declares observe, start, reply, cancel, attach, resume, list, openOriginal, approvals, and receipts. Acceptance: unsupported actions are disabled with a reason; a missing agent does not block local tasks.

### FR-03 — Task activity [B]

Display provider, project, title, lifecycle state, latest activity, age of evidence, and elapsed time. Supported task states: queued, starting, running, awaiting_input, awaiting_approval, completed, failed, cancelled, unknown. Connection health is separate: connected, reconnecting, disconnected, incompatible. A silent stream alone does not establish failure or a request for input. Acceptance: duplicate/out-of-order events cannot regress a terminal execution attempt or inflate counters. A session can accept new turns; the old completed turn remains completed while the new turn runs. Distinguish session, turn, attempt, and parent/child task identities.

### FR-04 — Honest progress [B]

Use activity descriptions and state transitions by default. Percentage is nullable and requires provider-supplied measurable work; show its basis. Planned steps are not a completion percentage unless the source explicitly treats them that way. Tool counts and elapsed time are descriptive. Acceptance: no fabricated percentage, ETA, test result, or cost.

### FR-05 — Needs me and notifications [B]

Collect actual questions, approvals, and actionable failures. Allow open, mark seen, and snooze. Deduplicate by provider/session/turn/event identity. Notify on meaningful transitions with per-category controls; retain items in the inbox when notifications are suppressed. No repeated alert solely because the panel reopens.

### FR-06 — Direct agent requests [B for verified routes]

Start a new managed task through a supported protocol with explicit workspace, provider, and prompt. Reply/cancel only where proven. Make submission progress and ambiguous delivery visible. Preserve provider approval requirements; offer open-original fallback when the buddy cannot safely complete an interaction. Never inject keystrokes into another app as the standard transport.

### FR-07 — Completion receipts [C]

Persist task outcome, source session reference, final message excerpt, available artifacts, reported changes, and reported tests. Each field identifies its source; missing tests display as not reported, not passed. Generate the receipt from structured output where possible without another AI call. A provider's completion signal does not prove that all background jobs or tests succeeded.

### FR-08 — Todos and conversion [B/C]

Create/edit/complete/reopen/delete with undo, optional due date and project. Convert a receipt or message into an editable todo with a backlink; conversion is user initiated. Acceptance: survive restart, preserve provenance, and do not create duplicates on repeated clicks.

### FR-09 — Timers [B]

Provide start/pause/resume/reset and notification on completion. Persist state transitions and deadline, not every second. Use a Windows clock abstraction with explicitly sleep-inclusive elapsed duration within a boot; do not assume generic Rust Instant defines suspend accounting. Running timers continue through sleep, while paused timers remain paused. Persist remaining duration, UTC checkpoint/deadline, and a validated boot/clock identity. Within the same boot, wall-clock changes do not alter duration timers. After reboot, use the persisted UTC deadline as the proposed fallback and flag detected clock inconsistencies instead of silently reconstructing exact elapsed duration. Arbitrary clock changes across reboot can make exact elapsed duration unknowable; expose that limit and permit correction. Calendar reminders follow their UTC instant, independently of duration timers. Validate the chosen Windows clock API before implementation; see the council evidence record.

### FR-10 — Reminders [B]

Create, edit, cancel, snooze, and complete one-off reminders. Store UTC instant, original timezone, and user input. Local calendar dates on todos remain dates rather than midnight-UTC instants. Show the resolved date/time before saving ambiguous phrases. Reconcile overdue reminders on app start and resume, coalescing multiple missed notices into one inbox summary. Closed-app OS scheduling is gated on the installed-package spike; do not promise delivery while powered off. Recurring reminders are a later proposal.

### FR-11 — Quick capture [C]

Create todo, reminder, timer, or resume note from explicit commands such as `todo review tests`, `timer 25m`, and `remind tomorrow 10:00 review PR`. Deterministic parsing first; preview assumptions, including AM/PM and timezone. Ordinary text becomes a draft, not an unexpected external action. No required cloud call.

### FR-12 — Project shortcuts [C]

Save named workspace folders and configured editor/terminal/session links. Launch executable plus argument array; do not concatenate a shell command from a search result. Check missing paths before opening. Configure each action independently, with no automatic installs or elevated launches.

### FR-13 — Resume notes [C]

Store a manually written next step per project, optional task/session link, and save time. Do not automatically track the user's screen, browsing, or clipboard. Last-opened state is optional local metadata and can be disabled.

### FR-14 — Focus sessions [C]

Link a todo and duration; start/pause/end with persistent history. Quiet mode suppresses the buddy's optional notifications while retaining inbox items; reminder and approval notification policies remain explicit settings. Music control is opt-in and capability dependent. Never change system-wide Do Not Disturb without a supported, explicitly selected integration.

### FR-15 — File/folder search [C]

Use Everything SDK/IPC when available; include dependency health and indexing limitations. Search result count is bounded, stale responses discarded, and query input debounced. Filter folder-only results; provide open/reveal/copy path. Without Everything, search saved projects and explicitly selected directories with a bounded scan and cancellation. No whole-disk recursive fallback or new content index. Document-content search is excluded.

### FR-16 — Attachments [C]

Support panel drag-and-drop and file picker. Stage explicit files, show name/size/path, allow remove, and disclose what the connector sends. Respect each provider's actual file/image support. Directories require explicit scope review; never expand all contents automatically. Initial per-request staging limits: 20 entries and 25 MiB of readable file payload, configurable downward per provider. Unsupported types or sizes remain visible with an explanation.

### FR-17 — Temporary shelf [C]

Hold up to 50 file references and explicitly pasted text items. Files remain where they are; removing a shelf entry never deletes a source file. Mark missing or moved files. Proposed initial text cap: 64 KiB per item. Retain until manually cleared or optional expiry; show retention clearly. No clipboard watcher. Drag-out behavior requires native Windows validation; use open/reveal/copy as baseline fallback.

### FR-18 — Playback and lyrics [D]

Read eligible Windows media sessions and prefer selected Spotify session. Lyrics provider is separate from Spotify; show provider attribution, matching ambiguity, unavailable/instrumental/offline states, and manual sync offset. Fetch on track change, not each lyric line. Track identity includes artist/title and duration when exposed. Resynchronize after seeking. Do not depend on private Spotify endpoints, extracted cookies, or account scraping.

### FR-19 — Settings and recovery [B-D]

Enable/disable each connector and Music independently; configure shortcut, theme, notification privacy, retention, and autostart. Export local data and import a validated export with a preview. Include diagnostics for versions and last connector errors with redaction. Full reset affects app-owned data only and requires confirmation.

### FR-20 — Future pet presentation [Later]

Domain events may later drive a replaceable presentation layer. V1 requires only static app/tray icons and textual status; no animation engine, sprite generation, pet plugin, or extraction of Codex assets. Pet availability must never determine whether commands or notifications work.

## 8. Integration feasibility and contracts

### Codex

Official documentation describes app-server as a custom-client interface with version-generated schemas, stdio transport, streamed events, and approval interactions. Use the documented protocol from Rust rather than adding a permanent Node bridge solely for convenience. [S1](desktop-buddy-research-v1.0.md#s1-codex-app-server)

Two different promises must be tested: managing work launched by this tool, and observing/control of pre-existing desktop-app sessions. Documentation of the first is not proof of the second. Installed external-session discovery, ownership, live freshness, concurrent control, account behavior, cloud visibility, and original-session links remain unverified. Start with supported metadata APIs, not private stores: current documentation describes thread/list and thread/read without resume, pagination, explicit sourceKinds filters, and useStateDbOnly to avoid scan-and-repair. Default source filters are not a complete inventory. Stored history and this server's loaded/runtime status do not prove live visibility into another desktop process. Do not resume merely to observe. Each distinct capability needs its own result on an independently started session.

### Antigravity

Official ACP documentation provides a subprocess/stdin/stdout route. Treat it as the preferred candidate for managed work, with protocol/version/authentication testing. [S2](desktop-buddy-research-v1.0.md#s2-antigravity-acp)

Hooks are a separate candidate for externally started activity. They distinguish invocation/tool lifecycle and stopping; a stop may leave background work active. Hook telemetry must be observational and must not alter agent decisions. Use supported payload identifiers and completion fields, not transcript-path assumptions. If fullyIdle is false and no later authoritative idle event exists, show background work last observed with evidence age; do not guess completion. Official Antigravity Remote Control is a possible open-original destination, not evidence of a public third-party control API. Do not enable it, inspect its transport, or add a permanent remote-control daemon merely for this utility. [S3](desktop-buddy-research-v1.0.md#s3-antigravity-hooks)

### Future agents and direction of integration

A connector normalizes observations and exposes explicitly supported commands. ACP is reusable where supported, not a guarantee of identical features. MCP could later expose buddy actions to agents, such as create-todo; it is not a universal mechanism to control every agent app. No third-party connector marketplace in v1.

### Media, search, and reminders

Windows media sessions expose information only from participating applications. Spotify desktop/browser behavior and timeline quality need a real-device spike. LRCLIB is a candidate, with service/usage terms, coverage, attribution, and distribution suitability checked before release. Everything must be installed/running in an IPC-capable edition; do not embed its index. Scheduled Windows notifications can operate without a running app but have delivery limits; package identity, activation, cancellation, and Tauri integration must be proven. [S7-S11](desktop-buddy-research-v1.0.md)

## 9. Proposed architecture

Single Rust application process owns SQLite, scheduler, provider connections, tray, and commands. React renders local state snapshots and incremental updates through Tauri IPC. Provider subprocesses start only when a connector/session actually needs them. Avoid an always-running Python server, Node server, local web server, extra database service, or an embedded agent browser.

Suggested modules: app-lifecycle, task-registry, provider-adapters, notification-router, reminder-scheduler, local-storage, project-launcher, search-provider, media-provider, and attachment-staging. Keep them inside one app/crate initially; do not create microservices.

For hook ingestion, prefer a current-user-restricted named pipe and a small native relay invocation. If a connector cannot use that route, evaluate a bounded user-owned spool with filesystem notification or authenticated loopback fallback. Local service failures must not block an external agent's work. Never expose an unauthenticated network-wide endpoint.

UI lifecycle proposal: launch the Rust tray without a webview; create one panel on demand; after close, compare immediate destruction with a short retention policy (30 seconds is one candidate), then select the measured policy and detach listeners before teardown. A fast-wake setting may retain it longer only if the budget permits. Timers and agent activity remain native. This lifecycle is a hypothesis until Tauri/WebView2 process cleanup and reopening are measured; destruction does not guarantee every browser process exits immediately.

Keep connector state machines independent of renderer lifecycle. Reopening requests a snapshot plus revision cursor, then subscribes; it must not miss events between these steps. Unsubscribe/cleanup is mandatory at every close and route teardown.

## 10. Task model, reliability, and bounded data

Identity: provider instance/connection + session + turn + execution attempt, plus parent/child references where exposed. Session runtime state is separate from the immutable outcome of each finished turn/attempt. For sources lacking turn IDs, define a documented adapter-owned attempt boundary; do not deduplicate separate user requests into one task. Store source event ID or a stable dedupe key, occurredAt, receivedAt, schemaVersion, localRevision, source type, state evidence, nullable progress, and pending action IDs. Keep connection health and most-recent authoritative task state separate.

Normalize raw events into lifecycle, activity, input request, approval request, artifact, result, and diagnostic. Enforce per-source ordering when sequence numbers exist. Without ordering, reconcile against authoritative snapshots before changing terminal states. Historical timestamps alone do not prove ordering across machines.

For submitted work, persist request intent before sending. If the provider lacks idempotency and the connection drops before acknowledgement, mark delivery unknown and offer reconciliation before retry. Do not automatically replay prompts. Cancellation requests become cancelled only after a source acknowledgement; otherwise show cancellation requested/unknown.

Proposed bounds: UI receives coalesced activity updates at no more than 4 Hz per visible active task; state changes, errors, and approvals bypass cosmetic coalescing. Use both count and byte limits: proposed total in-memory ingress ≤1,000 normalized events and ≤4 MiB; raw frames ≤1 MiB unless a negotiated provider contract requires a documented bounded exception. Global visible cosmetic updates ≤20/s in addition to the per-task limit. Persist critical transitions/pending interaction metadata before acknowledging application ingestion; coalesce verbose output separately. Bound a current-user hook spool at 10 MiB with a short delivery deadline. Under critical-event flood or disk failure, emit explicit degraded-telemetry state, reject new managed submissions as appropriate, and reconcile through supported source APIs; never imply lossless delivery. External hooks fail independently rather than blocking agent execution. Provider streams must have bounded processing/backpressure, not indefinite in-memory spill. Critical queues and byte limits are validated by overload tests. Store at most 64 KiB text excerpt per receipt and 200 KiB per task activity history, with export/open-original for full results. Never put unbounded token streams in React state or SQLite.

Core entities: projects, provider_connections, tasks, task_events, attention_items, receipts, todos, reminders, timer_sessions, focus_sessions, resume_notes, shelf_items, attachments, notification_deliveries, settings, schema_migrations. Index task state/project/time, reminder due time, and dedupe keys. Store file references, not entire repositories. SQLite WAL and migration strategy must be validated against backup and crash recovery requirements.

## 11. Performance budgets — proposed release gates

All memory figures are MiB. Measure release builds with devtools/dev servers disabled, on the actual 12 GB machine. Idle means five minutes after startup or close, with no incoming task events, music disabled, and no search running. Do not use memory trimming to disguise commit growth.

### Memory

- Tray-only app group, no active provider worker: target private working set ≤60 MiB and private commit ≤100 MiB.
- One open panel with 100 visible/current task summaries: target private working set ≤180 MiB and private commit ≤250 MiB.
- Panel plus active lyric strip: target private working set ≤250 MiB and private commit ≤350 MiB.
- Eight-hour repeated-use soak: return to within 20 MiB of the same warmed baseline, with no upward trend across repeated cycles.

The app group includes the Rust executable, attributed WebView2 browser/renderer/GPU/utility processes, and any buddy-specific relay or bridge. Independently report agent subprocesses, editor apps, Everything, and total session footprint. Also compare the identical normal workload without Buddy against tray/open/active Buddy. Every duplicate agent worker or helper caused by Buddy counts in the incremental product cost, even if excluded from the shell-only budget. Excluding those from the shell budget does not exclude them from the user's real RAM cost. Start no agent worker merely because Windows logged in. Avoid claiming browser-process sharing eliminates RAM overhead. [S4, S12](desktop-buddy-research-v1.0.md)

### CPU, latency, disk, and network

- Tray-only mean CPU ≤0.2% of whole-machine capacity over 10 minutes; one static open panel ≤0.5%. Compute CPU as summed process CPU-time delta / (wall time × logical processor count) ×100.
- Source event received to visible activity: p95 ≤250 ms; actionable inbox update p95 ≤500 ms, excluding Windows notification delivery.
- Warm panel usable p95 ≤300 ms; recreated panel usable p95 ≤1,000 ms. First cold launch ≤2 seconds is a target, subject to measured machine startup/runtime cost.
- Local search first page p95 ≤250 ms after debounce with an already-ready Everything index; provider latency reported separately. Query debounce 150–200 ms; page size 50.
- No fixed-rate background repaint or one-second persistent countdown tick. Use next-deadline wakeups, events, and visible-only display ticks.
- No music/network calls with Music disabled; lyric request at track identity change, cached with bounded retention. One fetch at a time, 8-second timeout, bounded retries/backoff, and negative cache.
- App-owned cache target ≤100 MiB including ≤25 MiB logs and ≤20 MiB lyric cache; warn before database metadata passes 100 MiB. Default event retention 14 days, receipts 90 days; user-authored todos/notes do not expire automatically.
- Initial compressed application JavaScript target ≤250 KiB, no remote fonts, no heavy animation/chart/editor library, and route-level lazy loading. This is a build check, not a claimed starting size.

Budget failure blocks the affected milestone. Investigate attribution, worker duplication, listener leaks, cached views, and repeated scans before weakening targets. Record measured reasons and a decision if a budget changes; do not silently redefine idle.

## 12. Optimization rules

Prefer Tauri/Rust direct OS and subprocess APIs. Reuse mature libraries only where they reduce complexity; assess binary cost, process cost, maintenance, license, and update impact. No local LLM, embedding database, screen recorder, always-listening microphone, clipboard monitor, or whole-drive watcher in this release.

Use native deadline scheduling. Persist timer state on actions and lifecycle boundaries rather than every tick. Maintain only the nearest deadline wakeup; reconcile wall-clock reminders on resume/timezone/clock changes. Pause music UI rendering when hidden while preserving only necessary source subscriptions.

Bound logs and event queues; batch database writes without delaying durable user actions or pending approvals. Backpressure verbose agent streams instead of causing unbounded RAM. Load only the selected task's details; virtualize long lists where measured benefit warrants it. Never prefetch full histories for every session.

Power saver: disable optional lyric strip and animations, use tray-only lifecycle, and avoid warming connectors speculatively. Resource diagnostics are sampled on demand or in a deliberate benchmark, not by a permanent high-frequency monitor.

## 13. Reminder and timer reliability

Use SQLite as the durable source; reconcile native scheduled notifications against it using stable reminder/revision IDs. Editing/cancelling removes the prior scheduled notification. Record attempted/delivered-in-app/acknowledged separately: a native schedule call succeeding does not prove the user saw a banner.

For v1, background reminders work while the tray is running. Closed-app delivery is conditional until the packaged-build native scheduling spike passes; show that capability accurately in Settings. If unavailable, Quit explains the limitation. No Windows service or wake-from-sleep timer is proposed by default.

Define and test: sleep across deadline, app crash, Windows restart, daylight saving changes, timezone travel, manual clock adjustment, duplicate native/in-app delivery, notifications disabled, Do Not Disturb, and stale notification click. Past-due items remain discoverable even when an OS delivery window expired. Use a notification outbox/dedupe record and overdue digest.

## 14. Privacy, permissions, and security

Local-first storage and no analytics upload by default. Provider requests leave the device only through the selected integration, with its own terms/account. A ChatGPT or other subscription is not assumed to grant every external API or integration entitlement.

Use Tauri capabilities by surface, explicit backend validation for custom commands, bundled local UI, and a restrictive CSP. Do not grant arbitrary shell/fs access to rendered agent output. Render receipts/lyrics as untrusted text or sanitized markup; block executable HTML and unapproved URL schemes. File content and agent messages never become application instructions. [S13](desktop-buddy-research-v1.0.md#s13-tauri-capabilities)

Prefer provider-native authentication. If app-owned credentials are unavoidable, store in Windows Credential Manager or a reviewed OS-protected secret store, not settings or diagnostic exports. Do not extract cookies or tokens from installed app internals. A hook gets only telemetry permissions; approvals are acted on only through a validated supported provider interaction and explicit user action.

Preserve workspace and provider permission boundaries. Validate canonical paths, executable paths, arguments, output links, IPC peer identity, message sizes, and attachment limits. No elevation for normal app use. Use current-user ACLs for local IPC. Redact prompts, file paths, and tokens from routine logs; offer a local privacy mode that hides notification content.

Backups/export are opt-in and explain that notes and paths can be sensitive. Full-disk encryption remains an OS setting; local SQLite is not claimed to be encrypted. Release packaging includes dependency/license review and update integrity; code-signing availability/cost is an open release decision.

## 15. Accessibility and platform quality

Complete operation by keyboard with clear focus, Escape behavior, and predictable focus return. No task status communicated by color alone. Labels for icon buttons; readable error recovery; screen-reader announcements for important state changes without announcing every token. Target WCAG AA text contrast and respect Windows reduced motion/high contrast where supported.

Validate at 1366×768 and 1920×1080, Windows scaling at 100/150/200%, small panel widths around 360 logical px, long filenames/prompts, and multiple monitors. This is a desktop product; mobile browser support is not a release requirement. Avoid layout jumps, excessive card decoration, custom drag regions that swallow controls, or auto-focus stealing when an agent finishes.

## 16. Alternatives and build-versus-reuse

Criteria: total resident cost on 12 GB RAM, supported agent access, keyboard speed, accepted feature coverage, operational reliability, and implementation effort. These are qualitative design judgments, not benchmark rankings.

1. Extend PowerToys Command Palette or Flow Launcher. Reuses discovery/launch UI and is the simplest experiment if a launcher already meets most needs. Still requires agent connectors and persistent task/reminder state; a dedicated lyrics surface and a future pet are awkward. Host and extension processes count in the total cost. Candidate if custom-shell feasibility fails. [S14-S15](desktop-buddy-research-v1.0.md)
2. Dedicated Tauri tray utility. Best proposed fit for one local Rust core, an on-demand web UI, and future replaceable presentation. Requires Rust/native integration work and explicit WebView measurement. Recommended pending Phase A.
3. Electron utility. Node ecosystem can simplify some connectors; Chromium-based architecture and per-window renderers add runtime considerations. Viable if integration delivery is materially easier, but there is no evidence that it meets this project's budget better. [S16](desktop-buddy-research-v1.0.md#s16-electron)
4. Native Windows utility. A .NET/WinUI or Rust-native shell could avoid a web frontend and deserves a small comparison if the Tauri shell misses the budget. It introduces a different UI implementation and does not solve provider access. No native-RAM superiority is claimed without measurement.

Reuse Everything, Windows media sessions and notifications, SQLite, official agent protocols, and small vetted UI primitives. Build the unifying activity model, inbox, local task workflows, request staging, and resource policy. Do not fork a full launcher or agent harness just to obtain an icon or text field.

## 17. Delivery plan and exit criteria

### Phase A — Evidence before implementation commitment

Run technical spikes only after implementation authorization. Split this phase into independent gates rather than one promise to resolve everything in 2–4 days.

A1 — Integration verdict: identify exact installed products/versions and separate rows for Codex Desktop, Codex CLI/app-server managed, Antigravity App, Antigravity IDE/hooks, and Antigravity CLI/ACP managed. Assess stored discovery, live observation, actual input questions, approvals, start, reply, cancel, attachments, reconnect, and open-original individually. Result vocabulary: not tested, documented candidate, verified on version X, unsupported on version X, or unresolved. Test independently initiated work, not only Buddy-owned tasks. Every named primary provider receives an explicit existing-session verdict. Prefer official non-resuming APIs; no private-store edits, token extraction, or UI automation substitute.

A2 — Shell/resource verdict: minimal packaged tray, shortcut, SQLite, synthetic attention stream, durable timer and notification. Measure zero-startup-webview lifecycle, retention versus destruction, 100 reopen cycles, actual process group, and incremental normal-workload cost. Do not build competing complete shells; native Windows is an evidence-triggered challenger only if bounded Tauri tuning fails.

A3 — Later module gates: media timeline/lyrics provider, Everything edition/IPC, and closed-app notification activation are separate probes scheduled before those modules. Native basic notification delivery must be tested in A2. No pet work.

Investigation policy: allocate one focused day per primary-provider existing-session route initially, extend once only when concrete evidence identifies a promising next check; otherwise report unresolved and present the tradeoff. These are investigation limits, not guarantees of support. The shell gate is independently estimated at 1–2 focused days after toolchain availability. Stop speculative expansion; re-estimate from results.

Exit: installed-version capability matrix with explicit existing-session verdicts for both named providers; initial shell and incremental workload benchmark; one real managed task represented correctly. Demonstrate an external attention event and its resolution if the supported route exists. Unsupported/unresolved original scope remains pending. A managed-only pilot may proceed as an explicitly labelled partial release; claiming complete agent support requires both provider-specific gates or an explicit user scope change. Changing frameworks cannot solve absent provider access.

### Phase B — Core usable release

Deliver FR-01–06, FR-08–10, basic export/backup/restore and settings, and one complete verified agent route. Move a minimal completion receipt → editable todo conversion and one project shortcut/resume note into this first vertical slice; full module polish remains in C. Choose the first route by demonstrated capability, preserving the second named provider as a separate committed gate. Exit: real request → actual attention item → explicit action or open-original → receipt → todo; timer/reminder restart recovery; project resume; resource/permission checks. Only a real input/approval event satisfies the attention criterion; synthetic fixtures are separate reliability evidence. A partial pilot must state which external and second-provider capabilities remain pending.

### Phase C — Accepted productivity additions

C1 completes receipts/conversion, project/resume workflows, quick capture, and focus sessions. C2 adds bounded search, explicit provider-specific attachments, and shelf. Exit: journeys J2–J5 with keyboard/recovery paths and per-module incremental resource results. Early backup/export is required before storing important authored notes; it is not postponed to music polish.

### Phase D — Music and complete agreed scope

Deliver playback/lyrics and optional pinned strip, capability-dependent direct actions, data export/recovery polish, packaged reminder behavior, and provider setup documentation. Exit: full agreed feature checklist or explicit recorded exclusions for unsupported provider functionality; no hidden placeholder controls; eight-hour soak passed.

### Later — Pet and optional expansions

Add visual presentation only after the utility meets its budgets. No schedule or effort allowance for custom artwork now. Follow-up queue, voice, synchronization, extra agents, and recurring reminders require separate scope decisions.

Planning estimate, not a commitment: the prior 15–26-day range is withdrawn as a delivery forecast because it bundled unresolved provider, clock, and Windows packaging investigations. Use the timeboxed A1 and A2 probes to estimate remaining work separately. Relative effort: local task UI medium; timer/notification recovery medium-high; each managed connector medium-high; external desktop observation high/unknown; search medium; lyrics medium-high. Re-estimate after the integration and shell verdicts, including packaging, backup, and dependency setup. This estimate is judgment, not researched productivity data.

## 18. Validation protocol

Record release commit, OS build, WebView2 version, CPU/logical cores, RAM, storage, power mode, connected providers, and benchmark inputs. Never compare a dev build to a release build as an architecture result.

Run five-minute warmup then ten-minute idle samples at one-second intervals; record mean/p95/max CPU, private working set, private commit, handles/threads, disk I/O, and attributed WebView2 PID membership. Shared working sets can double count memory; report total working set separately, not as unique resident RAM. Capture external worker totals alongside shell totals. Repeat on battery and AC under the user's normal editor/browser workload.

Scenarios: tray-only; open panel; one active agent; both primary agents; five simulated concurrent event streams; lyrics strip; repeated search; 100 panel open/close cycles; 10,000 historical task rows with paginated display; offline/reconnect; eight-hour normal-use soak. Synthetic streams test capacity, not provider compatibility. Active agents doing real work get separate resource figures.

Latency: at least 30 warm/recreated panel openings, 100 synthetic normalized event deliveries, and 50 ready-index searches; report p50/p95, not only the best run. Use screen/input timing for shortcut-to-usable UI. OS notification delivery latency is a separate observed metric.

Meaningful automated tests: event normalization/deduplication/reordering, ambiguous submission handling, durable scheduling/time changes, migration/backup recovery, attachment and path validation, renderer snapshot/event race, provider capability negotiation, multi-turn identity, stale approval ownership, worker shutdown/restart, attachment mutation, byte/count overflow, and notification dedupe. Contract tests use captured redacted versioned provider fixtures; live integration smoke checks run against supported installed versions with deliberate authorization for any prompts. UI tests cover keyboard and failure recovery rather than mirroring every component implementation.

No benchmarks or live provider tests were executed while drafting this document. Documentation availability does not establish account entitlement or installed compatibility.

## 19. Risks, improvements, and decision gates

R1 — Scope can overwhelm the lightweight goal. Improvement: keep all accepted features but ship in measured modules. Benefit: useful early release. Effort: small planning discipline, ongoing budget checks. Priority: critical; proposed baseline.

R2 — External agent monitoring may have weaker support than custom-client sessions. Improvement: capability-based setup and a dedicated external-session spike. Benefit: accurate promises and usable fallback. Effort: medium/high; depends on exact provider versions. Priority: critical; no full-access claim approved.

R3 — WebView residency can erase expected savings. Improvement: native tray, one recreated panel, bounded data, explicit attributed-process measurement. Benefit: lower background footprint if verified. Effort: medium; browser cleanup and latency tradeoff. Priority: critical; target requires Phase A evidence.

R4 — Missed or duplicated reminders reduce trust. Improvement: durable outbox, native scheduling where proven, overdue digest, clear Quit semantics. Benefit: recoverable reminders. Effort: medium; Windows identity/activation integration. Priority: high.

R5 — Lyrics provider availability and redistribution fit are uncertain. Improvement: isolate Music, permit manual LRC, do not make Spotify OAuth a core prerequisite. Benefit: rest of product works independently. Effort: medium; rights/coverage validation before distribution. Priority: medium; provider choice proposed.

R6 — Native dependencies and Rust tooling slow delivery. Improvement: validate toolchain and installers first, use minimal maintained libraries, avoid additional language runtimes. Benefit: fewer processes and packaging paths. Effort: medium; if delivery cost becomes disproportionate, revisit alternatives explicitly.

R7 — Broad filesystem/provider access creates avoidable exposure. Improvement: explicit staging, scoped commands, current-user IPC, provider-native approvals, sanitized output. Benefit: preserves user intent without intrusive background monitoring. Effort: medium; release gate.

## 20. Open decisions and safe defaults

- Exact Windows/agent versions and full laptop specs: collect during implementation setup; do not inspect unrelated private data.
- Existing desktop sessions versus buddy-launched work: both desired; feasibility is unresolved, with no substitution hidden from the user.
- Original app deep links, cloud tasks, cost reporting, and rich attachments: provider-dependent; hide unsupported capabilities with explanation.
- Icons: neutral static app icon now; Codex pet reuse is optional and unresolved, not blocking.
- Platform: Windows 11 x64 proposed first; no cross-platform promise.
- Startup: user-selected opt-in; no Windows service by default.
- Account/fees: local tools work without model usage; agent/provider costs and quotas remain separate and unknown until selected.
- Closed-app reminder delivery: native scheduling spike decides support; tray-running guarantee comes first.
- Performance budgets: retain as provisional gates until first measured baseline; document any revision.

## 21. Change record

v1.0, 5 November 2026: converted chat requirements into a testable utility-first baseline; removed the cat/animation dependency; preserved all explicitly accepted productivity features; separated managed and external agent sessions; replaced rough RAM reassurance with measurable budgets and explicit accounting; added notification recovery, attachment boundaries, evidence statuses, alternatives, milestones, and risk gates. No implementation, installation, account changes, or provider requests were performed.


## 22. Council-required execution contracts

### Ownership and lifecycle

Every connection/session records origin (externally observed or Buddy-managed), process ownership, owning interaction channel, and supported operations. Closing the panel stops no work. Disabling observation detaches only that observer and leaves original-app work alone. For a managed task, disable/quit offers a clear active-work choice: keep the background app running, request supported cancellation and await outcome, or explicitly terminate the Buddy-owned runtime after warning of unresolved work. Do not silently kill a task or assume detached stdio workers remain operable. Provider-specific crash/disconnect behavior must be measured and disclosed. Original-app processes are never terminated by Buddy.

On restart, recover intent and reconcile source state before any new prompt, answer, approval, or cancellation. Never replay approvals automatically. Cancellation requested, acknowledged cancellation, and runtime process termination are distinct outcomes. Installer/update must defer or explicitly handle active managed work rather than killing workers as housekeeping.

### Pending interactions and telemetry

An attention item records original provider/session/turn/request ID, connection owner, action authority, observed/pending/resolved state, and source evidence age. Revalidate the pending interaction immediately before answering; an original app or another client may have resolved it. Observation does not grant control. If revalidation is unsupported, open the original interface instead of blindly sending a stale approval.

Hooks have a short bounded relay timeout (initial target ≤100 ms) and event-specific no-op output that cannot continue/stop/modify agent work. Test buddy absent, blocked pipe, oversized payload, malformed message, full spool, crash and duplicate delivery. Failure is visible in Buddy's connection health when detectable; no claim of guaranteed telemetry under storage failure. Preserve source event IDs and reconcile after loss. Global normalized event/frame limits may not truncate protocol structures blindly; reject/summarize with explicit diagnostics and connector-specific handling.

### Attachment contents

Each connector declares whether it sends a byte snapshot, image payload, or path reference that the agent reads later. Staging records size/mtime and checks for changed or missing files before send. Changed files require a refreshed preview; copied content has a bounded encrypted-or-OS-protected temporary staging policy if implemented. Do not universally copy files merely to simplify the model. Path references are labelled as live references and cannot promise immutable contents or that file bytes were uploaded. Deleting a draft/staged copy never deletes the source.

### User-value and total-cost gate

In five representative work sessions, compare the existing workflow with Buddy: finding a real attention item, reopening the next project step, and capturing/recovering a local task. Record action counts, elapsed time, stale/missed items, original-app visits, and perceived interruption. Establish improvement thresholds after observing a baseline; never invent measured savings or assume fewer clicks always means better work.

For the same normal browser/editor/agent workload, compare Buddy absent, tray-only, open, and active with both providers. Record incremental process-group private commit/residency, total available physical memory, sustained CPU, reopening p95 and responsiveness under actual memory pressure. Include duplicate agent workers and search-helper costs. If the shell fits its budget while total workflow cost degrades the laptop, the product does not pass optimization. Agree a total incremental budget from Phase A measurements before expanding modules; no universal safe headroom is claimed from 12 GB installed RAM alone.

### Council revision record

V1.1: selected Plan B's narrow useful vertical slice, incorporating A's explicit external-session/worker/lifecycle gates and C's clock, ownership, attachment and recovery contracts. Corrected discovery nuance, made 30-second retention experimental, protected critical ingress with byte/global limits, brought backup and minimal project/receipt workflows forward, and withdrew an unsupported delivery forecast. All accepted features remain in staged scope; pet work stays deferred. The council decision record preserves rankings, meaningful dissent, evidence and remaining uncertainties. No code or benchmarks were produced by this review.
