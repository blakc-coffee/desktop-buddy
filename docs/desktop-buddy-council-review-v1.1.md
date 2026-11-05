# Desktop Buddy — council decision and PRD review

Date: 5 November 2026 · Status: planning review complete; implementation and device verification outstanding.

Deliverables: [Revised PRD v1.1](desktop-buddy-PRD-v1.1.md), [preserved PRD v1.0](desktop-buddy-PRD-v1.0.md), and [original evidence record](desktop-buddy-research-v1.0.md).

## Decision

Select **Plan B: a feasibility-gated Tauri utility with one narrow end-to-end user workflow**, incorporating compatible correctness improvements from Plans A and C. Tauri/Rust/React/SQLite remains a provisional recommendation. Neither council agreement nor vendor documentation proves the app will fit the laptop's resource budget.

The first useful slice is: request → real question/approval → explicit response or original-app action → completion receipt → editable todo, together with project resume and reliable local timer/reminder storage. Keep all previously accepted modules; defer the pet and animations.

Two independent questions come before feature expansion: can supported integrations observe the user's existing work, and can the shell plus any additional workers meet measured responsiveness/resource goals? A framework change can help the second; it cannot manufacture an unavailable provider API.

## Method and criteria

Used the installed llm-council skill: three fresh-context independent planner subagents received the same self-contained brief, file paths, constraints, criteria, and read-only boundaries. They researched and proposed independently before seeing other proposals. Then all received the same neutrally labelled summary packet for one peer-review round. Each reviewer recognized its own proposal; presentation was anonymized, not guaranteed blind.

No model diversity was requested or claimed. This was native subagent deliberation, not an external council service. Planners did not edit shared files, install tools, run provider requests, or spawn other agents. The parent verified decisive source claims and made the final selection. This report records conclusions and objections, not raw reasoning transcripts.

Criteria established before proposals: (1) correctness and user scope; (2) total resident resource cost and responsiveness on 12 GB; (3) supported integration feasibility; (4) reliability, privacy and verification; (5) effort and maintainability. Rankings are advisory and no invented numeric scores were used.

## Alternatives proposed

**A — Existing-session and incremental-cost gates first.** Strongest emphasis on preventing managed-only work from substituting for monitoring existing desktop sessions. Compare teardown against retention; define worker ownership; use official original-app destinations. Main concern: provider-first exploration can consume time before demonstrating user value; initial overflow bounds were less concrete.

**B — Useful vertical slice with global resource bounds.** Test supported discovery, shell lifecycle, then a full attention-to-result workflow. Bring minimal receipt/todo/project resume earlier, protect critical ingress, and measure total incremental cost. Main concerns: must explicitly retain the second named provider and existing-session verdicts; needs A/C's worker, timer and attachment contracts.

**C — Separate gates and precise failure semantics.** Strongest treatment of suspend/reboot clocks, stale interaction ownership, attachment mutation, and close/quit behavior. Main concern: its delivery sequence postpones project/resume utility and lacks B's clear product-value test.

All three independently selected conditional Tauri. Existing-launcher extensions remain a simpler candidate when a host is already installed and adequate; native Windows UI is the first evidence-triggered shell challenger; Electron needs a demonstrated connector-delivery advantage. None provided comparative device benchmarks or advocated building every alternative in advance.

## Peer rankings and dissent

- Reviewer of A ranked **B > C > A**: B best proves cross-tool value; C adds necessary edge-case contracts.
- Reviewer of B ranked **C > A > B**: C best closes correctness gaps; B could underweight existing sessions and the second provider.
- Reviewer of C ranked **B > A > C**: B best balances usefulness, supported observation and bounded cost; A supplies the right shell experiment.

Plan B received two first-place rankings; C received one. There was no unanimous ranking. Chairman selection: B best connects evidence to a useful product outcome, provided its scope/ownership gaps are explicitly corrected. C's dissent is accepted as required engineering contracts rather than dismissed by majority vote.

## Verified additions to the research

Sources accessed 5 November 2026. “Verified” below means documented, not tested on the user's installation.

1. **Codex discovery nuance.** Official [App Server documentation](https://learn.chatgpt.com/docs/app-server) describes non-resuming thread list/read operations, pagination, source filters, and `useStateDbOnly`. It does not follow that another server can observe/control a desktop process's live sessions. Test documented discovery first; distinguish stored data, server-local runtime state, and live subscription authority. Source defaults and version compatibility require care.

2. **Lazy window mechanism.** [Tauri configuration](https://v2.tauri.app/reference/config/) documents `create:false` and subsequent native creation. That supports delayed panel creation. [Microsoft WebView2 performance guidance](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/performance) documents browser-process overhead and startup tradeoffs. Actual process reclamation and best retention duration remain measurement questions; 30 seconds is a candidate, not a proven optimum.

3. **Timer clock semantics.** Rust's [Instant documentation](https://doc.rust-lang.org/std/time/struct.Instant.html) does not guarantee suspend accounting. Microsoft's [Interrupt Time](https://learn.microsoft.com/en-us/windows/win32/sysinfo/interrupt-time) distinguishes ordinary and unbiased interrupt counts; [QueryInterruptTime](https://learn.microsoft.com/en-us/windows/win32/api/realtimeapiset/nf-realtimeapiset-queryinterrupttime) is unaffected by user/time-service clock adjustments. Proposed Windows clock abstraction must be tested for sleep-inclusive duration semantics and reboot fallback. Exact elapsed duration after reboot plus arbitrary wall-clock change cannot be promised without trustworthy external evidence.

4. **Antigravity boundaries.** Official [ACP](https://antigravity.google/docs/cli/acp) and [hooks](https://antigravity.google/docs/hooks?tab=ide) are distinct managed/observational mechanisms. [Remote Control](https://antigravity.google/docs/remote-control/) documents an official browser interface for desktop/CLI sessions. It is a candidate original-app destination, not proof of a third-party API or permission to reverse-engineer a transport. Installed capability, auth and resource costs are unverified; nothing was enabled.

5. **Notification limits remain.** Microsoft's [scheduled notification documentation](https://learn.microsoft.com/en-us/windows/apps/develop/notifications/app-notifications/app-notifications-scheduled) describes delivery while an app is closed, with a five-minute delivery window. Durable overdue recovery and packaged-build tests remain necessary; no guaranteed delivery during power-off is claimed.

These facts refine the baseline rather than prove all proposed integrations. Original lyrics/provider-rights, attachment, OS-version, account-access and performance gaps remain open.

## Required PRD changes applied in v1.1

- Distinguish session, turn and execution attempt; a new turn can run after a completed turn without corrupting terminal outcomes.
- Produce separate existing-session verdicts for Codex Desktop, Antigravity App/IDE, and managed CLI routes. Leave unsupported accepted requirements pending rather than claiming completion through a fallback.
- Add precise ownership for questions/approvals, revalidation before answering, and no automatic approval replay after reconnect.
- Specify close, disable, quit, crash and upgrade behavior for original-app and Buddy-owned processes. No implicit cancellation or unclaimed orphan workers.
- Add count, byte, spool and global UI-update bounds; persist critical metadata, expose degraded telemetry, and keep hooks bounded and observational.
- Define timer sleep/reboot/clock-change behavior and its unavoidable recovery ambiguity.
- Specify attachment snapshot versus live-reference semantics and changed-file revalidation, without universal copying.
- Compare window retention policies empirically and measure incremental workload cost including duplicate workers.
- Bring basic export/backup and a narrow project/receipt/todo workflow into the first useful release.
- Replace the optimistic total schedule with timeboxed investigations and post-evidence estimation.
- Keep automatic track-to-lyrics lookup as accepted scope. Manual LRC is a fallback, not completion of that requirement.

## Actionable selected plan

1. **Integration gate:** after implementation authorization, inventory exact versions and test one independently started session and one managed route per named provider. Separate discovery, history, live status, attention, start/reply/cancel/attachment and original-app navigation. Write a result per capability; do not modify private provider stores.
2. **Shell gate:** packaged tray, shortcut, SQLite, one panel, timer and notification. Compare immediate teardown against short retention; run 100 reopen cycles. Measure shell group and total incremental normal-workload cost. Do not install every dependency or build multiple shells speculatively.
3. **Narrow usable release:** real attention workflow plus local task recovery, minimal receipt→todo, project resume, and backup. The first provider can be chosen by proved capability; both named providers retain independent scope gates.
4. **Productivity completion:** finish capture/focus/projects/receipts, then search, staged attachments and shelf, measuring each addition.
5. **Music:** validate Windows timeline and provider suitability, then automatic lyrics and optional strip. Pet work follows only after utility budgets pass.

In five representative sessions compare finding an attention item, resuming a project and capturing/recovering a task against the existing workflow. Establish improvement thresholds after collecting a baseline; do not claim time savings without data. Performance includes the same editor/browser/agent workload with Buddy absent, tray-only, open and active on both providers.

Investigation bounds in the PRD are proposed planning limits, not deadlines promising integration success. If the central existing-session workflow lacks a supported route, produce an explicit unresolved verdict. A managed-only pilot may still be useful but must be labelled partial. If Tauri misses measured budgets after bounded tuning, compare the identical minimal native shell slice rather than rewriting all features.

## Persistent preference

Saved the user's instruction in global Codex guidance: use llm-council for future research, PRD work and substantial planning; compare independent proposals, peer-review and select a supported plan; preserve scope and distinguish verified facts from assumptions. If independent agents are unavailable, disclose that rather than simulate a council. Existing guidance was preserved. This preference does not authorize implementation or external mutations.

## Validation and limits

Three independent proposals and three actual peer assessments completed. Decisive documentation was reopened/checked by the parent. V1.0 preserved; v1.1 contains all 20 functional requirement IDs and the council corrections. Local document links and headings checked. No app code, provider prompts, installations, performance runs, notification schedules, or live integration tests were performed. Recommendations and targets remain provisional until the defined gates pass.
