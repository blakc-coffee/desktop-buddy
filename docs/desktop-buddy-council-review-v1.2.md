# Council review v1.2

6 November 2026. Three independent planners compared native Win32, an on-demand Tauri command bar and reuse of the large tools window against correctness, requested interaction, RAM, verification, feasibility and maintainability. Proposals were presented under neutral labels for one peer round; this is not guaranteed blind review or model diversity.

All selected native Win32. All peer rankings were B, C, A. Parent selected B's native owned popup, bounded asynchronous search and shared Rust services, with C's explicit editor detection and A's `/tools` escape hatch. Focus/activation, stale asynchronous results and IME were the main risks. Native resource savings were a hypothesis until measured.

Implemented scope: native pet and command bar, explicit local commands, saved-root folder/workspace lookup, 120-second file references, copy-only handoff and keyboard copy alternative. Detailed tools remain lazy. Review found and corrected hidden copy errors, folder filtering after a result cap, IME Enter interception and absent Tab traversal. Source review does not replace native interaction acceptance.

A targeted three-agent music round compared native media metadata plus online LRCLIB with native metadata/local LRC first. Two peer rankings favoured the local-first plan; one favoured the online-capable architecture while accepting its gates. Parent chose local-first staging because actual Spotify timeline behavior and valid client identification are unresolved. The compatible future architecture remains native metadata + optional LRCLIB + local fallback, without Spotify SDK/browser workers.

Decisive primary evidence:

- [Microsoft media session manager](https://learn.microsoft.com/en-us/uwp/api/windows.media.control.globalsystemmediatransportcontrolssessionmanager): APIs for participating media sessions. This does not prove Spotify metadata/timeline works on this host.
- [Microsoft WinRT concurrency](https://learn.microsoft.com/en-us/windows/apps/develop/cpp-winrt/concurrency): blocking async `.get()` belongs off the UI thread. Exact Rust bindings still require runtime checks for a future adapter.
- [Spotify developer response](https://community.spotify.com/t5/Spotify-for-Developers/Update-endpoint-for-lyrics/td-p/5355493): public lyrics availability is not supplied by the reviewed developer interface; do not build against private endpoints.
- [LRCLIB current API docs](https://lrclib.net/docs.txt): anonymous access, required identifying headers, sequential requests and Retry-After. Approximate rate figures may change and are not a contract. A real Buddy project/contact value is missing; online lookup remains unimplemented. API access is not a blanket lyrics-content redistribution licence.

Focus enforcement remains a separate optional design, not a claim made by the timer alias. Preserve the weekly budget and stop expansion before adding speculative integrations to an unaccepted primary UI.

Next action: manually accept pet click → bar → timer/folder/workspace and file drag-in/out; then verify real Spotify sessions and configure provider identity before online lyrics work.
