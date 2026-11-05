# Desktop Buddy — research and decision record v1.0

Research date: 5 November 2026, Asia/Kolkata. Sources accessed on that date. This is a documentation review, not a benchmark, security audit, or validation of installed integrations.

Read alongside [PRD v1.0](desktop-buddy-PRD-v1.0.md). Public documentation changes; Phase A must record exact installed versions and recheck protocol compatibility.

## Evidence terminology

Verified means an opened primary source documents the capability or limitation. It does not mean the application has been implemented. Inferred means a project-specific architectural judgment drawn from those facts. Unverified means an entitlement, compatibility, performance, or product-behavior question that needs a focused test. Performance targets and delivery estimates are proposals, never facts about a finished product.

## Main findings

The main integration uncertainty is access to work already running in another agent application. Custom-client protocols are documented; arbitrary desktop-session monitoring/control is not established by that alone. The main performance opportunity is eliminating a permanent floating window, then measuring an on-demand UI with a native background core. The product should be useful without model calls, a pet, or a network connection.

## Source register

### S1: Codex app-server

Source: [official Codex App Server documentation](https://learn.chatgpt.com/docs/app-server). Accessed 5 November 2026; no stable publication date established.

Verified: custom clients can use JSON-RPC, start/resume threads, initiate turns, consume lifecycle events, and handle interactions. stdio is documented; the page flags WebSocket transport as experimental/unsupported. Schemas can be generated for a specific CLI version.

Inference: a Rust protocol client avoids a Node-only bridge for basic orchestration. Unverified: safe attachment to existing desktop-app sessions, account access, cloud task discovery, cross-process ownership, and native Windows behavior for this installation. Smallest test: one managed task with event, approval, reconnect, and cancellation handling; separately test an externally started session without modifying private stores.

### S2: Antigravity ACP

Source: [official Antigravity ACP guide](https://antigravity.google/docs/cli/acp). Accessed 5 November 2026; publication date not shown.

Verified: the guide describes `agy acp`, a subprocess JSON-RPC connection over standard input/output, version capability checking, and authentication using CLI credentials.

Inference: this is a better candidate for buddy-launched work than UI automation or an unofficial adapter. Unverified: supported session operations, Windows packaging, attachment behavior, and compatibility with the user's exact Antigravity product/version. Test its advertised capabilities and a real task before exposing controls. Do not generalize CLI support into IDE-session takeover.

### S3: Antigravity hooks

Source: [official hooks documentation](https://antigravity.google/docs/hooks?tab=ide). Accessed 5 November 2026; publication date not shown.

Verified: documentation distinguishes App, CLI, and IDE configuration and lists session, invocation, tool, and stop events. Stop payloads include a termination reason and a `fullyIdle` field distinguishing remaining background activity.

Inference: hooks can feed lightweight external activity observations. A completed invocation alone is not a finished user task. Unverified: exact installed payload/schema and reliable attention detection. Test normal finish, error, background command, and missing-buddy scenarios. A telemetry hook must never inject continuation instructions or turn a logging failure into an agent failure.

### S4: WebView2 performance

Source: [Microsoft WebView2 performance best practices](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/performance). Accessed 5 November 2026; page currently indexed as updated in 2026.

Verified: WebView2 uses multiple browser-engine processes and adds startup/memory overhead. Microsoft discusses avoiding redundant instances, inactive-view memory policies, best-effort suspension, caching, communication volume, and profiling.

Inference: Tauri's use of a system runtime does not establish a fixed RAM budget. Avoid unnecessary windows and measure real browser processes. Unverified: whether selected memory-management APIs are exposed cleanly through the chosen Tauri version. Do not require custom WebView internals until ordinary lifecycle cleanup is measured.

### S5: Tauri tray

Source: [Tauri system tray guide](https://v2.tauri.app/learn/system-tray/). Accessed 5 November 2026; publication date not shown.

Verified: native tray icons, menus, and event handling are supported.

Inference: the first usable product can start with a conventional icon instead of a pet. Unverified: the exact no-startup-webview and final-window-close lifecycle in the selected application configuration. Smallest test: start without panel, open/destroy/reopen it repeatedly, and keep a native timer/event connection operating throughout.

### S6: Tauri architecture

Source: [What is Tauri?](https://v2.tauri.app/start/) and [Windows prerequisites](https://v2.tauri.app/start/prerequisites/). Accessed 5 November 2026; publication dates not shown.

Verified: web frontend frameworks can be combined with native backend logic; Windows development requires the documented native toolchain and WebView2 prerequisites.

Inference: React/TypeScript plus Rust is a reasonable fit for this proposed product. No benchmark establishes it as the lowest-memory option. Test setup/build/install early; a smaller installer is not a memory measurement.

### S7: Everything search

Source: [Everything SDK](https://voidtools.com/support/everything/sdk/) and [installation/edition guidance](https://www.voidtools.com/support/everything/installing_everything/). Accessed 5 November 2026; publication dates not shown.

Verified: SDK DLL/library interfaces use IPC; Everything runs separately. The documented Lite edition omits IPC support.

Inference: reuse an installed IPC-capable instance and its existing index instead of writing a new indexer. Account for its independent memory and indexing work. Unverified: local installation, edition, database readiness, architecture compatibility, and actual indexed paths. Test query cancellation, bounded results, folder filter, and missing-index behavior; no invisible whole-disk fallback.

### S8: Windows media sessions

Source: [GlobalSystemMediaTransportControlsSessionManager](https://learn.microsoft.com/en-us/uwp/api/windows.media.control.globalsystemmediatransportcontrolssessionmanager?view=winrt-26100). Accessed 5 November 2026; publication date not shown.

Verified: provides access to playback sessions from applications participating in system media transport controls.

Inference: it is a candidate for obtaining current playback without requiring Spotify account OAuth for the basic local feature. Unverified: track metadata completeness, playback timeline reliability, and desktop/browser session selection on the user's machine. Validate pause, seek, adverts, multiple players, and missing duration. Do not treat participation by every application as guaranteed.

### S9: Spotify developer constraints

Sources: [Spotify developer response on lyrics](https://community.spotify.com/t5/Spotify-for-Developers/Update-endpoint-for-lyrics/td-p/5355493), older official community response; [2026 Development Mode migration guide](https://developer.spotify.com/documentation/web-api/tutorials/february-2026-migration-guide), effective timeline in 2026. Accessed 5 November 2026.

Verified: Spotify's developer response says it does not offer a public lyrics API; current developer documentation includes changed Development Mode constraints. The older response must not be presented as a newly announced 2026 policy.

Inference: use an independent lyrics integration rather than a private Spotify endpoint. Recheck current API/reference and applicable account requirements before adding remote Spotify controls. This review does not establish the user's developer account eligibility or entitlement.

### S10: LRCLIB candidate

Source: [LRCLIB documentation](https://lrclib.net/docs). Accessed 5 November 2026; publication date not established.

Verified at limited depth: the site identifies itself as a lyrics database and points to API documentation. Indexed official documentation describes track metadata and plain/synchronized lyric fields. The current interactive page returned only its documentation wrapper; its Markdown response could not be rendered by the research browser.

Unverified: current detailed API contract, caching/usage expectations, rights suitable for this distribution, catalog coverage, and reliability. Candidate only. Retrieve current machine-readable docs and test a small track sample before implementing; retain manual-LRC/no-lyrics fallbacks. Public API access does not by itself establish redistribution permission.

### S11: Scheduled Windows notifications

Source: [Microsoft scheduled app notifications](https://learn.microsoft.com/en-us/windows/apps/develop/notifications/app-notifications/app-notifications-scheduled). Accessed 5 November 2026; publication date not shown.

Verified: scheduled notifications may appear while an app is not running. The documented delivery window is five minutes; a computer remaining off beyond it can cause the notification to be dropped. Scheduling and cancellation APIs are described.

Inference: native scheduling improves background reminders but does not replace durable overdue recovery. Unverified: selected Tauri package identity, scheduling integration, activation/click routing, Do Not Disturb, and sleep outcomes. Test packaged release builds rather than treating a dev-server notification as evidence of closed-app delivery.

### S12: Windows memory accounting

Source: [Microsoft Process Working Set](https://learn.microsoft.com/en-us/windows/win32/procthread/process-working-set), last updated 14 July 2025; accessed 5 November 2026.

Verified: working sets contain both private and shared pages; the page identifies APIs for obtaining process memory information.

Inference: sum private metrics for attributed app processes, report shared/total working sets separately, and include separate external-agent totals. A small number for the Rust executable alone is insufficient. Memory budgets in this project are design targets; the source provides no numeric budget for this application.

### S13: Tauri capabilities

Source: [Tauri capabilities](https://v2.tauri.app/security/capabilities/). Accessed 5 November 2026; publication date not shown.

Verified: permissions can be assigned to windows/webviews; overlapping capabilities combine permissions.

Inference: minimize the local frontend's allowed operations and validate backend commands explicitly. Capabilities are not proof that arbitrary custom command implementations are safe. Review path handling, command arguments, output rendering, and named-pipe identity separately.

### S14: PowerToys alternative

Source: [Microsoft Command Palette extension model](https://learn.microsoft.com/en-us/windows/powertoys/command-palette/extensibility-overview). Accessed 5 November 2026; current documentation.

Verified: extensions are separate .NET processes communicating through WinRT, with commands and several page types.

Inference: it can reuse a launcher surface but still needs backend workflows and contributes host/extension cost. It is a plausible simpler mechanism, especially if already installed. No comparative benchmark or proof that it covers every agreed feature was conducted.

### S15: Flow Launcher alternative

Source: [official plugin catalog](https://www.flowlauncher.com/plugins/). Accessed 5 November 2026; live catalog, no publication date.

Verified: the project offers a plugin catalog with varied launcher workflows.

Inference: evaluate reuse of an existing launcher before building overlapping command UI. A catalog listing is not compatibility, maintenance, security, or performance validation of each plugin. No plugin installation or code audit was performed.

### S16: Electron

Source: [official Electron process model](https://www.electronjs.org/docs/latest/tutorial/process-model). Accessed 5 November 2026; publication date not shown.

Verified: Chromium-style multiprocess architecture, Node.js main process, and separate window renderer processes are documented.

Inference: JavaScript connector development may be simpler, but installer/runtime implications must be weighed against the always-on utility goal. Do not assert fixed RAM differences or automatic Tauri superiority. Measure comparable release builds if the framework decision becomes contested.

### S17: Default pet availability

Source: [official Pets documentation](https://learn.chatgpt.com/docs/pets). Accessed 5 November 2026; publication date not shown.

Verified: product documentation exists for built-in/custom pet experiences. This review did not establish a supported embeddable pet runtime or reusable-art license for this independent application.

Decision: defer pet integration; use a neutral static icon. No pet plugin, generation, extraction, download, or republishing was performed or required for the PRD.

## Reusable design reference

Local source inspected: `C:/Users/dharu/Documents/Codex/design-references/herding-warm-charcoal.md`. Existing saved reference, read-only. No app repository, implemented design system, or screenshots exist in this workspace. Use its neutral/warm palette as an initial proposal while prioritizing Windows utility conventions, accessible text, and reduced visual cost. No original reference file was changed.

Authoritative interfaces also reviewed: Tauri tray documentation and Microsoft's Command Palette extension model. They establish useful entry-point and command/page concepts; no claim of visual screenshot inspection is made. Detailed visual design and rendered QA belong to implementation.

## Three critique passes

### Pass 1 — Problem and alternatives

Concern: a pet plus music plus search plus tasks can become an unfocused dashboard. Response: foreground Needs me and quick actions; accepted modules remain available without all being resident. Compare launcher extension, Tauri, Electron, and native Windows UI before claiming an architecture winner. Value is the shared workflow and reliable attention state, not presumed product novelty.

### Pass 2 — Architecture and privacy

Concern: private app data and UI scraping can make cross-agent integration brittle and intrusive. Response: supported protocols/hooks, per-capability disclosure, bounded payloads, explicit attachments, and provider-owned approvals. Avoid a universal-control promise. Concern: hidden browser windows can still cost resources. Response: native tray and lifecycle spike with actual PID attribution.

### Pass 3 — Delivery and reliability

Concern: restarting, sleep, disconnects, and upgrades can cause missed reminders or duplicate requests. Response: persistent deadlines/outbox, unknown-delivery state, source reconciliation, versioned adapters, and bounded crash/reconnect tests. Stop expanding design until the first utility/agent slice meets its budgets.

## Decisions and status

- Utility first, no pet dependency: confirmed latest user direction.
- All explicitly accepted productivity extras: retained; staged delivery is proposed.
- Windows-first and 12 GB optimization: discussion baseline; exact OS/hardware unverified.
- Tauri/React/Rust/SQLite: recommended and previously discussed; feasibility gate remains mandatory.
- Native tray plus on-demand renderer: research-backed proposal, not yet tested.
- ACP/app-server/hooks: documented integration candidates, installed compatibility unverified.
- Everything and LRCLIB: proposed reuse, optional dependencies with fallbacks.
- Numeric budgets, retention, limits, theme interpretation, milestone order, and effort estimate: explicit planning proposals.
- No new feature silently added from the unaccepted suggestion list.

## Research limits and next evidence

No account sign-in, software installation, provider prompt, repository change, live agent control, notification scheduling, or laptop benchmark was performed. Primary sources document mechanisms, not this project's achieved performance. Some pages carry an authorization banner while exposing public documentation text; inaccessible details were not inferred.

The next work is the bounded Phase A feasibility suite defined in the PRD. Its output should be an installed-version capability report and release-build resource baseline, then a focused update to this PRD. It should not start by implementing artwork or every accepted module at once.
