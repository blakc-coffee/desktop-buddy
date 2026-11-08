# Desktop Buddy

## Platform
Windows desktop with a native mini pet and native command bar; an on-demand web-rendered panel handles detailed management. No mobile product.

## Stack
User approved the council-reviewed Rust/native Windows primary surface with Tauri/React detailed tools and SQLite. All application state belongs to Rust; browser preview is separate test storage. Local slash commands do not require Codex, a CLI, a browser runtime or model inference. Plain native command-bar text is an explicit AI question on Enter and uses the signed-in Codex CLI.

## User and task
Dharu wants a standalone mini pet: click for a compact command bar, find folders, open Cursor workspaces, customise timers, manage todos/reminders, hold files until cleared or handed off, and eventually show Spotify lyrics and optionally limit distracting apps/sites during focus. Existing and managed AI sessions remain separate capabilities. Custom artwork and animations are deferred. Clearing a hold never deletes originals. See docs/desktop-buddy-PRD-v1.2.md and its council review for the revised scope.

## Constraints
12 GB laptop; measure native app plus WebView2 and any additional workers. No model calls for local tools. No provider prompts without explicit user submission. No credentials read from private app stores. Preserve at least 35% weekly usage; current build may consume at most 50 percentage points from its starting 90% remaining. Practical soft stop: 45% remaining; absolute build stop: 40% remaining. Check between milestones. No repeated council for settled implementation.

## Evidence
Council PRD and saved herding warm-charcoal reference. No performance claims until measured. No fabricated agent tasks or progress.

## 0.2 managed Codex tasks
New task is an explicit submission flow: choose a saved project, load the installed CLI's model catalog, select a model and submit a prompt. Read-only is the default; workspace-write requires opt-in. The app attaches one app-server runtime at a time. Existing Codex desktop work remains stored history only, without verified live attachment or monitoring.

Approval requests require manual approve-once or decline decisions; questions require manual answers. Cancellation waits for a source-reported interrupted outcome. A stopped connection without an authoritative outcome means unknown. Completion retains the connection until explicit End connection so background work is not implicitly killed; quit is blocked until the connection ends.

Local receipts record source-reported outcomes, not independent verification of task correctness. Receipts can create editable follow-up todos. Version 2 backups include receipts; previously active records restore as unknown with no automatic replay or resume.

No real inference has been tested for 0.2. Hermetic protocol fixture QA and native local-tool checks pass; see docs/build-report-0.2.md for idle resource measurements and acceptance limits. Active Codex runtime memory remains unmeasured. Antigravity, Everything and automatic Spotify integrations remain pending.

