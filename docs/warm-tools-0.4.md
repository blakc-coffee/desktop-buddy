# Faster tools and bounded popup reuse

Implemented November 10, 2026. The user explicitly overrode the usage guard for this batch; the standing usage policy remains unchanged.

## Runtime behavior

- Todo and explanation have dedicated Vite entry pages. Their initial imports exclude the settings dashboard; shared React/bridge assets remain shared. Other tools load their heavier components on demand.
- Todo is prepared hidden at startup and when the native command bar opens. Preparation does not take focus. The current pet palette is reapplied when opening.
- Settings/tools, Todo, and explanation windows hide on close and are reusable for approximately 45 seconds. A single five-second sweep destroys expired hidden windows. Visible windows are never reclaimed. No mutex is held while native destruction can invoke window callbacks.
- Compact Todo now exposes the existing Remove action for every checklist item.
- Opening the native command bar prepares one dedicated local Codex app-server process. It initializes and verifies configuration only: no screenshot, thread, turn, or prompt before Enter. Each submitted question starts a fresh ephemeral read-only thread, with tools/callbacks rejected and default service tier. It does not attach to the user's running Codex chats.
- The installed app-server cannot use exec's ignore-user-config/ignore-rules flags. Preparation discovers configured MCP server names, restarts with each disabled, and verifies effective capabilities. Global AGENTS.md remains an installed CLI behavior; only that known global source and Buddy's own instructions are accepted, with no arbitrary project instruction sources. Buddy never copies authentication credentials or changes account/billing settings.
- Stream events must match thread, turn, and assistant item identities. The first complete sentence opens the bubble; updates are throttled to 80 ms. Dismissal/clear invalidate presentation so late results cannot reopen it. Only the pet animates while waiting.
- One successful exact screenshot/question/model/effort/instruction result is retained in memory. PNG key limit 2 MiB; global instruction content is bounded and part of the key. Changed images/questions/profiles/instructions miss the cache. Partial/error/cancelled results and /sc actions are never cached.
- The AI process and cached answer expire after 45 seconds idle; an active request is never interrupted by that timer. Failure before submission can use the previous exec path; after turn/start is attempted, automatic replay is forbidden.
- /sc retains the existing silent, atomic UI Automation path. No cached or streamed UI actions were introduced.

## Verification and limits

- Frontend build and eight TypeScript tests pass.
- 68 native tests pass; live inference probes are explicitly ignored by the default suite.
- 17 headless UI workflows pass; four previously archived workflows remain skipped. Coverage includes desktop/narrow layouts, light/dark themes, keyboard navigation, accessibility, Remove, streaming, dismissal, follow-ups, and dedicated bundle isolation.
- Synthetic Todo (300 px), tools (1100 px), and scrollable answer (380 px) previews were inspected. No user screen was captured or operated.
- Two synthetic plain questions on the prepared process: 4024/3925 ms to first sentence, 4025/3925 ms total. One synthetic screenshot request including cold worker preparation: 5031 ms; identical repeat: <1 ms. These small samples are not guarantees or an apples-to-apples speed benchmark against different previous prompts.
- Prepared worker working set: 117,841,920 bytes (112.4 MiB), excluding Buddy and WebView2. Idle cleanup was verified after 47 seconds. Native popup cold/reopen timing and the complete app process-tree memory budget were not measured in this batch; WebView startup and provider latency remain variable.

## Decision record

Three independent read-only council proposals agreed on an initialization-only dedicated process, fresh threads, strict event identity checks, first-sentence streaming, bounded idle cleanup, and one exact screenshot cache. Peer review favored the plan with explicit no-replay and dismissal rules. Installed-CLI probes required replacing unsupported exec flags with effective configuration checks and a narrowly allowed global instruction source. This is a compatibility limitation, not a promise of configuration-free execution.

## Learn while building

The critical path is the chain of work that must finish before something becomes usable. A lightweight entry removes unnecessary dashboard parsing; preparation overlaps process startup with the time spent typing; caching avoids repeated inference. Warm reuse trades temporary RAM for lower startup cost, so both resources need a bound. Trace `bar::show -> WarmState::prepare -> Job::Run -> publish -> bubble_present` and `navigation::todo -> retained WebView -> idle destroy` to see that flow. Optional exercise: explain why a cancelled answer must not be cached, even if its model request finishes successfully.

Official documentation checked:
- [App-server protocol](https://learn.chatgpt.com/docs/app-server)
- [Configuration schema](https://learn.chatgpt.com/docs/config-schema.json)
