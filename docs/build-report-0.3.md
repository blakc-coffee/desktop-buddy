# Desktop Buddy 0.3 — native pet checkpoint

5 November 2026. Unsigned Windows x64 portable build. Normal launch shows a small native floating pet; click to open the existing tools. Closing tools releases WebView2. Tray, Ctrl+Alt+Space and Settings provide recovery access. Custom artwork/animation is deferred.

The pet accepts a temporary batch of local file references and implements native copy-only drag-out. It holds up to 20 paths for 60 seconds, supports cancelling and replacing the batch, and discards references on expiry or exit. Alt-drag moves a loaded pet. No file content is cached/uploaded; hold expiry never deletes originals. Live Explorer/app drag-in and drag-out remains unverified and needs manual acceptance. The automation tool explicitly rejected cross-window drag endpoints.

Verified: production TypeScript build; 6 TypeScript tests; 19 optimized Rust tests; native SQLite/timer/reminder/search/backup/panel/single-instance regressions; installed Codex CLI read-only discovery (25 stored sessions); managed-task protocol fixture regressions. New native tests cover shell IDataObject/CF_HDROP roundtrip for multiple Unicode paths, original content preservation, unsupported batches, expired deadlines and drag cancellation/shutdown. Native pet rendering was visually inspected. No live AI inference occurred.

Memory results, when available, are in memory-summary.json. Each phase uses 10 seconds warmup and 15 one-second samples, with Rust plus WebView2 descendants measured using Windows private working set/commit. Pet-only excludes active Codex runtime. This is a short idle smoke test, not an eight-hour/battery/pressure certification. Do not reuse 0.2 numbers as 0.3 measurements.

Remaining: live transfer/60-second UI expiry, multi-monitor/DPI/sleep testing, real-provider inference/approvals, automatic Spotify, Everything, Antigravity, snooze/autostart/undo. Earlier verification and limitations remain in build-report-0.2.md. See pet-council-0.3.md for decision evidence and manual acceptance steps. Test data is isolated from the normal application database.

Latest pre-packaging usage: 67% weekly remaining, about 23 account-wide points used from the 90% build baseline. No reset/paid credits used. The five-hour window was nearly exhausted; this checkpoint preserves the requested weekly reserve.
