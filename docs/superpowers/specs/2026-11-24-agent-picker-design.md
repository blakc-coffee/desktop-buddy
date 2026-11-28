# Search bar: agent picker, pet picker, recent shelf

Status: draft, awaiting approval.

## Why
A new Buddy user has no way to see which AI answers questions, or how to connect one.
Today the chat silently tries Claude Code, falls back to Codex, and otherwise shows an
"Install … and sign in" error. The user wants new users to connect any one agent
(Claude, Codex or Antigravity) straight from the search bar.

Later asks folded in (same day): drop music from the bar, move the pet chooser into the
bar with a live preview, and have the shelf keep recently used files for about an hour,
beside the pinned ones.

## Goals
- First open with no agent chosen: the bar lists Claude Code, Codex and Antigravity as rows.
- Enter on a row makes it the one agent for questions, saved as setting `chat_agent`.
- `/agent` reopens the same rows to switch later.
- The bar footer names the connected agent in a muted label, e.g. `claude · ready`.
- `/music` leaves the bar's actions menu (the Tools window Music page stays).
- `/pets` rows get a live preview of the highlighted pet; ↑↓ swaps it, Enter switches.
- The shelf gains a "Recent" group of files used in the last hour, beside pinned items.

## Non-goals
- No multi-agent fallback chain once an agent is chosen (one agent, as asked).
- No new window or Tools page.
- Buddy does not store, read or type credentials.

## Anti-goals
- Never sending a prompt before the user presses Enter on a question (AGENTS.md).
- No extra idle RAM: only the chosen agent may be warmed, never all three.

## Approaches considered
| | Approach | Trade-off |
|---|---|---|
| **A (picked)** | Picker rows inside the bar | One step, stays in the bar; footer needs a hint |
| B | "Connect an agent" row opens a Tools page | Roomier, but more clicks and a window hop |
| C | Auto-pick the first installed CLI | Zero clicks, but no choice |

## Design
- **Row status** comes from `resolve_cli` (`claude`, `codex`, `agy`): `ready` (native exe
  found) or `not installed`. Sign-in is not guessed from credential files.
- **Enter on a row:**
  - installed → save `chat_agent` and close the picker.
  - not installed → open the vendor's install page in the default browser.
- **Sign-in:** if a turn fails with an auth error, the bar shows one row,
  "Sign in to {agent}". Enter opens a console running that CLI's own login: `claude`
  (then `/login`), `codex login`, or `agy`. The user signs in there.
- **Chat routing:** `screen_helper::quick_answer` uses only `chat_agent`.
  - Claude: the existing warm actor in `claude_chat.rs`.
  - Antigravity: the same actor with `agy -p --input-format stream-json
    --output-format stream-json`. Its event shape must be verified first, with an
    adapter only if it differs.
  - Codex: the existing `warm_ai`/`codex exec` path.
- **Prewarm:** opening the bar warms only the chosen agent.
- **Deterministic commands:** timers, reminders and todos stay local and never need an agent.

## Pet picker
- When `/pets` is open, the bar widens its right side by a 96×104 preview cell and
  draws the highlighted pet with `pet_art::draw_pose_accessory` (neutral still pose), on the
  bar surface. Nothing is drawn when another list is open, so the normal bar costs nothing.
- The pet's right-click menu keeps one item, "Change pet…", which opens the bar on `/pets`.
  The per-pet entries (menu ids 20..=26) are removed from that menu.

## Recent shelf
- Uses: opening a file from a bar result, dropping files on the pet, and copying held files.
- Each use upserts an Item with kind `recent` (path is the key) and `created_at = now`.
  A reuse refreshes the time instead of adding a duplicate.
- Expiry: any `recent` item older than 1 hour is deleted lazily when items are read or a
  new recent is written. No background timer.
- Cap: 20 recent items; the oldest is dropped first. Pinned shelf items keep their 50 limit.
- Pin: the Shelf page shows "Recent · last hour" above pinned items. Each recent row has
  Pin, which turns it into a normal `shelf` item, plus Open and Hold with pet.
- `/shelf` in the bar lists recent first, then pinned.
- Paths only, local SQLite, same as today. No file contents are read or copied.

## Left to the builder
- Exact install URLs for each vendor.
- The exact auth-error strings that trigger the sign-in row.
- Footer layout next to the existing `n / total`.
- Exact preview-cell placement inside the bar card.
