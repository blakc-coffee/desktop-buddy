Update: held files now remain until cleared or handed off, with multiple-file labels around the pet. See [current behavior](file-cluster-and-screen-polish-0.4.md).

# Toolbar, Todo and page helper — 8 November 2026

Update: the primary helper is now screen-based. `/sc` captures and types a text answer; `/exp` captures and explains. `/fill` aliases `/sc`. The extension workflow described below remains optional inside Screen helper. See [current screen-helper instructions](screen-helper-0.4.md).

The native toolbar remains empty on opening, with arrow/wheel command browsing in the input placeholder, Escape/outside-click dismissal, native editing/IME behavior, and expanded file results. It has no Actions button. F4 retains the optional grouped tools menu. The toolbar uses Segoe UI, consistent drawn result icons, larger result targets and work-area-aware row limits. Existing aliases remain; `/s` is not implemented.

Direct destinations include `/ask`, `/resume`, `/fill`, `/todos`, `/timers`, `/reminders`, `/projects`, `/shelf`, `/held`, `/imports`, `/settings`, `/music`, and `/codex`. `/held` displays the current temporary pouch contents; `/shelf` opens persistent references; `/imports` opens the retained imported-file folder. Typed navigation requests survive first WebView startup. Generation-matched acknowledgements prevent an older request from consuming a newer one. Opening Ask never submits a prompt or loads model catalogs.

The pet's right-click menu contains exactly Open tools, Hide pet, Clear held references, Choose pet, and Quit. Motion previews, show/hide, reset and other utilities remain in Actions. The paw opens a small Todo window containing a Todo heading, add input, editable titles and checkboxes. Enter saves a title, Escape cancels its edit, blur also saves. A dedicated rename action preserves the latest completion/project/note fields when another window changes the same item. The window follows the selected pet palette on reopening.

## Browser page helper

The optional browser-fill section opens the Chrome/Edge extension/native-host integration. The extension explicitly connects a chosen page for five minutes, without restarting the browser or using a debugging port. A page-specific permission and the extension/native host must be installed once. The release includes both executables, the extension and a per-user registration script; setup is described in `browser-helper/README.md`.

Workflow: connect page → capture questions → select fields/model → generate → review → fill. Generation uses the installed Codex CLI in ephemeral, read-only mode, with shell, browser, image, plugin and agent tools disabled. The answer JSON is validated against captured field identities and choices. Application is deterministic, checks original control/model values and document fingerprint, reads back results, and never clicks Submit or Run. DOM updates are not transactional: interference stops remaining writes and reports the verified count. Unknown Apply outcomes are never automatically replayed.

Supported: standard text/textarea, single selects, checkbox and radio groups including accessible role controls. CodeMirror 5 and Ace use exposed editor instances. Monaco is limited to exactly one exposed public model and one visible editor. Private/bundled Monaco, CodeMirror 6, cross-origin frames and closed shadow roots remain unsupported. No claim of universal website compatibility is made.

The native messaging transport uses bounded, expiring, correlated messages under current-user local application data. Connection directories reject reparse points; page data cannot nominate filesystem paths or arbitrary scripts. Short-lived response files are removed on consumption, expiry or disconnect. Model output files are removed after generation. The bridge accepts only capture/apply and heartbeat messages. Browser tab closure/navigation ends the connection. A tab switch never silently retargets an existing capture.

## Expanded file holding

The drop target accepts local shell paths, virtual attachment streams, Chromium editor/resource formats, transferable PNG/data URLs/DIB images and DownloadURL payloads. Materialized copies are imported in a background worker and then use the existing copy-only outgoing handoff. A generation ticket prevents a late import from replacing a newer hold or reopening cleared references. Failed imports preserve the previous batch and expose a clickable error badge.

Imported content is limited to 16 MiB/file, 32 MiB/drop, 20 files/drop and a 128 MiB store. Local path references retain their existing behavior. Imported copies remain available after the two-minute hold expires, preserving pinned references; clearing/expiry does not delete originals. Manage retained copies through `/imports`. Browser-only blob URLs and protected/authenticated downloads without transferable bytes require the source's Save/Download action. Live Antigravity attachment and browser-image dragging remains unverified; COM format fixtures and outgoing handoff tests pass.

## Design evidence and decision

Primary references: [Raycast compact search](https://manual.raycast.com/search-bar), [PowerToys compact Command Palette](https://learn.microsoft.com/en-us/windows/powertoys/command-palette/overview), and [Fluent Searchbox](https://fluent2.microsoft.design/components/web/react/core/searchbox/usage). Their useful principles were a compact input, expansion for actual results, and deliberate command access. The existing pet palettes and the user's blank-input/no-help-row requirements take precedence over copied launcher layouts.

Three independent proposals plus one peer-review round favored an explicit Actions control and durable destination handoff. Meaningful dissent concerned a redundant paw icon and status text stealing query width; the implementation omits the header paw and limits feedback to actual state rather than typed-command browsing. Browser planning favored extension/native messaging over clipboard-only or desktop accessibility approaches. Named pipes were preferred by one reviewer; a bounded, expiring mailbox was selected for implementation simplicity. DOM preflight is strict, but partial event-driven writes are reported rather than described as atomic.

Browser integration follows [Chrome native messaging](https://developer.chrome.com/docs/extensions/develop/concepts/native-messaging) and [activeTab's user-gesture boundary](https://developer.chrome.com/docs/extensions/develop/concepts/activeTab). Chrome's [default-profile debugging restriction](https://developer.chrome.com/blog/remote-debugging-port) is one reason the implementation preserves the existing profile through an extension. Codex structured output and noninteractive flags were checked against the installed CLI and [official noninteractive documentation](https://learn.chatgpt.com/docs/non-interactive-mode).

## Verification

- Rust tests cover drag formats, actual imported bytes, virtual-file indices/streams, bounds, stale imports, copy-only outgoing handoff, command destinations, navigation acknowledgement races, answer-plan validation and bounded native framing. The local HTTP attachment fixture passes outside the sandbox, whose loopback restrictions otherwise block it.
- Native toolbar rendering inspected for all three pet palettes at 96/144/192 DPI without desktop input. Native menu theme tests retain standard menu semantics.
- Isolated headless UI checks passed Todo add/edit/check/reopen persistence, 300/360px layout, cold Ask focus/ack, warm destinations, newest request winning, no automatic model load/submission, and page-helper capture/generate/review/apply UI at 960/380px with mocked IPC.
- Synthetic browser DOM tests passed standard control filling/readback, no submit, idempotent page receipt, user-edit and changed-question rejection. A reproducible optional Playwright check is in `scripts/check-browser-helper.mjs`.
- Actual CodeMirror 5 and Ace distributions passed editor capture, model update/readback and edited-code rejection in an isolated profile. Monaco adapter behavior remains unverified against a real platform.
- Native host subprocess fixture passed framing, armed connection, capture relay and matching response. One real default-model Codex request returned the expected schema-constrained answer for a synthetic 2+2 quiz with restricted tools.

The individual boundaries were tested; the extension has not been installed into the user's browser, so a full live-site connect/generate/apply acceptance run remains outstanding. Live keyboard/IME, mixed-monitor placement and screen-reader checks were not performed to avoid disturbing the active desktop session. No performance, universal compatibility, signing or commercial-readiness guarantee follows from these checks.
