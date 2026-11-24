# One todo list, and a "what are we working on?" greeting

Status: draft, awaiting approval.

## Why
Todos live in two places: the Tasks page in the Tools window (tabs for Todos, Timers and
Reminders) and the small Todo pop-up opened from the pet's token. The user wants one place,
and wants Buddy to feel interactive after a restart by asking what they are working on.
Other focus-pet apps (Cat On Chair, TimoPet) link each focus session to a todo; the user
picked that pattern.

## Decisions made by the user
- The Todo pop-up becomes the single place. The Tasks page leaves the Tools window.
- The greeting's answer becomes a todo pinned as today's focus. Enter on it can start a
  25-minute timer linked to it.

## Goals
- **One list:** the Todo pop-up shows todos, plus running timers and pending reminders.
- **Linked tags:** a timer or reminder started from a todo shows as a small tag on that row,
  for example `⏳ 15:30` or `🔔 5:00 pm`.
- **Unlinked rows:** timers and reminders made from the bar or chat (`/t`, "remind me…") have
  no todo, so they show as their own rows at the top with a clock icon.
- **Redirects:** `/timers`, `/reminders`, `/todos` and the pet's token all open the pop-up. The
  Overview links that pointed at Tasks open the pop-up too.
- **Greeting:**
  - About 3 s after Buddy starts, the pet shows a small speech bubble: "What are we working
    on?".
  - It does not steal focus. Clicking the bubble or the pet opens the search bar with the
    placeholder "What are we working on?".
  - Enter saves the answer as a todo and pins it as today's focus.
  - The bar then offers one row, "Start 25 min focus", which starts a timer linked to the
    focus.
- **Today's focus:** it sits at the top of the pop-up with an accent left rule.
  - Ticking it done clears the pin.
  - A new day clears the pin, but not the todo.

## Non-goals
- No AI involved in the greeting.
- No calendar or history view in this change.
- No change to how timers count or how reminders fire.

## Anti-goals
- **No focus stealing:** the greeting never steals focus or opens a window on its own. That
  matters with "Start with Windows", where it would interrupt login.
- **Asked once a day:** the greeting is skipped if a focus is already pinned for today, if
  Quiet mode is on, or if it was dismissed today. One ask per day at most.

## Approaches considered
| | Approach | Trade-off |
|---|---|---|
| **Picked** | Todo pop-up is the single list; timers and reminders become tags or rows in it | Small window, one place. The Tools window loses its biggest page. |
| Rejected | Tasks page is the single list; the pop-up is removed | Roomier, but every todo check opens the big window |
| **Picked** | Greeting bubble opens the bar on click | No focus steal, and reuses the bar's input |
| Rejected | Greeting opens the bar by itself on start | Steals focus at login |

## Design
- **Data:**
  - `Item` gains `parent_id: String` (serde default empty). A timer or reminder started from
    a todo stores that todo's id there. Old data stays valid.
  - The settings key `focus` holds `"{todo_id}|{YYYY-MM-DD}"`.
  - The date comes from the local clock (`GetLocalTime`), never from a model.
- **Pop-up (`src/Todo.tsx`):** reads todos, running or paused timers, and pending reminders
  from the existing snapshot. Order:
  1. Unlinked timer and reminder rows.
  2. The focus row.
  3. Open todos.
  4. A collapsed "Done" section.

  Each todo row gets a small menu with "Start 25 min focus" and "Remind me in…", which
  create linked items.
- **Tools window:** the Tasks entry and its page leave `uiNavigation.ts` and `App.tsx`. The
  Overview cards for timers and reminders call the native `todo` window instead.
- **Greeting:**
  - The pet shows a short-lived notice in the existing `NOTICE_BOUNDS` badge style, placed
    as a speech line above the pet, for 60 s.
  - Clicking it calls `bar::open_with(app, pet, "")` with a one-shot placeholder.
  - The bar marks the next Enter as a focus answer (a `focus_prompt` flag on `Bar`).
  - That Enter saves a todo, writes `focus`, and shows the "Start 25 min focus" row.
  - Esc or 60 s with no click dismisses it for the day (setting `focus_asked` = today).

## Left to the builder
- Exact tag glyphs and colours in the pop-up, from the existing ledger tokens.
- Where the greeting line sits relative to the todo token, so they don't overlap.
- Migrating the Playwright tests that drive the Tasks page over to the pop-up.
