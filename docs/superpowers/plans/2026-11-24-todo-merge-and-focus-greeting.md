# One todo list and a focus greeting — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the Todo pop-up the single list for todos, timers and reminders. After Buddy starts, the pet asks "What are we working on?", and the answer becomes today's pinned focus with an optional linked 25-minute timer.

**Architecture:** `Item` gains `parent_id`, which links a timer or reminder to a todo. Today's focus is a setting holding `"{todo_id}|{YYYY-MM-DD}"`, resolved by pure Rust helpers. The pet shows a speech line drawn in GDI that never takes focus. Clicking it opens the bar in a one-shot "focus answer" mode. The Tasks page leaves the Tools window, and `src/Todo.tsx` renders every task kind.

**Tech Stack:** Rust (Tauri 2, GDI, rusqlite), React 19, Playwright.

**Spec:** `docs/superpowers/specs/2026-11-24-todo-merge-and-focus-greeting-design.md`

## Global Constraints
- **No focus stealing:** the greeting never opens a window or steals focus on its own.
- **Once a day:** the greeting is skipped when a focus is pinned for today, when the setting `focus_asked` equals today, or when Quiet mode is on.
- **Expiry:** the greeting line hides after 60 s and Esc dismisses it. Either sets `focus_asked` = today.
- **Dates:** they come from the local clock (`GetLocalTime`) as `YYYY-MM-DD`. No model is involved.
- **Linked timer:** it is 25 minutes, created through the existing `start_timer` checks.
- **Old data:** it must still load. `parent_id` uses the serde default `""`.
- **Every task:** finishes with `cargo test --release` (native) and `npx playwright test --timeout=120000` (UI) passing, and a local commit. Never push.
- **First run:** the greeting also tells new users the hotkey once. On first-ever launch the line reads "Hi, I'm Kitty. Ctrl+Alt+Space talks to me." before the question.

## Review Focus
1. **Focus todo deleted:** the pin points at nothing, so the pop-up shows no focus row and nothing crashes. Test in Task 1 (`focus_for_today` returns None for an unknown id).
2. **Midnight:** a focus pinned yesterday is not today's focus. Test in Task 1.
3. **Linked timer's todo deleted:** the timer shows as an unlinked row instead of vanishing. Test in Task 3 (UI fixture with an orphan `parentId`).
4. **Greeting and a held-files badge or notice at once:** the speech line sits above the pet and never overlaps `NOTICE_BOUNDS`. Test in Task 2 (bounds do not intersect).
5. **Bar opened from the hotkey, not the greeting:** it does not treat Enter as a focus answer. Test in Task 2 (`focus_prompt` is false after a plain open).

---

### Task 1: Data — `parent_id`, today's focus, linked timers

**Files:**
- Modify: `src-tauri/src/model.rs` (`Item.parent_id`).
- Modify: `src-tauri/src/main.rs` (`start_timer` gains `parent_id: Option<String>`; new focus helpers; new commands `set_focus`, `start_focus`).
- Modify: `src/domain.ts` (`parentId: string` in `Item` and `emptyItem`).

**Interfaces:**
- Produces, in `main.rs`:
  - `pub(crate) fn today() -> String`, the local date as `YYYY-MM-DD`.
  - `fn focus_for_today(setting: Option<&str>, today: &str, items: &[Item]) -> Option<Item>` returns the open todo whose id matches when the date matches.
  - `#[tauri::command] fn set_focus(app, core, id: String) -> Result<(), String>` writes `focus`. An empty id clears it.
  - `#[tauri::command] fn start_focus(app, core, id: String) -> Result<(), String>` starts a 25-minute timer titled like the todo, with `parent_id = id`.
  - The snapshot gains `focus_id: String`, empty when none, serialized as `focusId`.

- [ ] **Step 1:** Write failing tests in `storage_tests`:
  - `focus_matches_only_today_and_open_todos`:
    - `focus_for_today(Some("t1|2026-11-24"), "2026-11-24", &[todo t1])` → `Some(t1)`.
    - Same with today `"2026-11-25"` → `None`.
    - Unknown id → `None`.
    - Done t1 → `None`.
    - `None` setting → `None`.
  - `start_focus_links_the_timer`: in-memory DB with todo t1. Calling the inner `start_timer_in(&db, title, 25, "", Some("t1"))` creates a running timer with `parent_id == "t1"` and `duration_ms == 1_500_000`.
- [ ] **Step 2:** Run `cargo test --release focus`. Expected: FAIL to compile.
- [ ] **Step 3:** Implement.
  - Extract `fn start_timer_in(db: &Connection, title: String, minutes: u32, project_id: String, parent_id: Option<String>) -> Result<(), String>` from `start_timer`. The command keeps its checks and the `changed()` call.
  - `today()` uses `GetLocalTime`, like `local_ms_of_day`.
- [ ] **Step 4:** Run the native suite. Expected: PASS.
- [ ] **Step 5:** Commit with message "Link timers to todos and keep today's focus".

### Task 2: Pet greeting and the bar's focus answer

**Files:**
- Modify: `src-tauri/src/pet.rs` (greeting state, `GREETING_BOUNDS`, paint, click, 60 s expiry via the existing notice timer pattern).
- Modify: `src-tauri/src/bar.rs` (`Bar.focus_prompt`, `open_for_focus`, Enter handling, "Start 25 min focus" row).
- Modify: `src-tauri/src/main.rs` (startup schedules the greeting 3 s after the pet shows).

**Interfaces:**
- Consumes: `today()`, `set_focus`'s inner `write focus`, `start_timer_in` from Task 1.
- Produces:
  - `pub fn greeting_line(first_run: bool) -> &'static str` returns `"Hi, I'm Kitty. Ctrl+Alt+Space talks to me."` when first run, otherwise `"What are we working on?"`.
  - `fn should_greet(focus_pinned_today: bool, asked_today: bool, quiet: bool) -> bool`.
  - `const GREETING_BOUNDS: RECT` sits above the pet's head inside the 128×136 window, so it does not intersect `NOTICE_BOUNDS` or `TODO_BOUNDS`.
  - `pub unsafe fn bar::open_for_focus(app, pet)` opens the bar with the placeholder "What are we working on?" and `focus_prompt = true`. Any other open resets `focus_prompt = false`.
  - The setting `first_run_done` is `"1"` after the first greeting.

- [ ] **Step 1:** Write failing tests:
  - `greeting_rules`:
    - `should_greet(false,false,false)` is true.
    - Each true flag alone makes it false.
    - `greeting_line(true)` contains "Ctrl+Alt+Space".
    - `greeting_line(false) == "What are we working on?"`.
  - `greeting_bounds_clear_other_badges`: `GREETING_BOUNDS` intersects neither `NOTICE_BOUNDS` nor `TODO_BOUNDS`.
  - `plain_open_is_not_a_focus_answer`: a pure `fn enter_kind(focus_prompt: bool, text: &str) -> EnterKind` with `enum EnterKind { Focus(String), Normal }`. `(false, "essay")` → `Normal`. `(true, "essay")` → `Focus("essay")`. `(true, "/t 5m")` → `Normal`, so commands still work.
- [ ] **Step 2:** Run. Expected: FAIL.
- [ ] **Step 3:** Implement.
  - The greeting renders as a speech line (badge style, surface fill, ink text) at `GREETING_BOUNDS`.
  - A click inside it, or on the pet while it shows, calls `open_for_focus`. When the line is "Hi…", a click instead shows "What are we working on?" for the remaining time.
  - In the bar, Enter on `Focus(title)` calls `save_item` (todo) then sets `focus`, and shows choices `[("Start 25 min focus", Action::StartFocus(id))]`. Enter on that row runs `start_focus`.
- [ ] **Step 4:** Run the native suite plus the `BUDDY_PET_THEME_PREVIEW` render, with a greeting drawn in one cell. Check the PNG: the line reads fully and doesn't cover the face.
- [ ] **Step 5:** Commit with message "Ask what we're working on after Buddy starts".

### Task 3: One list in the Todo pop-up

**Files:**
- Modify: `src/Todo.tsx` (render timers, reminders, focus row and tags; row menu actions).
- Modify: `src/style.css` (`.todo-focus`, `.todo-tag`, `.todo-clock-row`).
- Modify: `tests/ui/fixture.ts` (a linked timer `parentId: "t1"`, an orphan reminder `parentId: "gone"`, and `focusId: "t1"` in the snapshot).
- Test: `tests/ui/workflows.spec.ts`.

**Interfaces:**
- Consumes: `focusId` and `parentId` from Task 1, and the `start_focus`, `set_focus`, `timer_action` and `remove_item` commands.

- [ ] **Step 1:** Write the failing UI test `todo pop-up holds focus, tags and unlinked clocks`. Open `/todo.html` with the fixture, then expect:
  - A `.todo-focus` row containing the t1 title.
  - A `.todo-tag` on that row with the timer's mm:ss.
  - The orphan reminder as a `.todo-clock-row`.
  - A row menu item "Start 25 min focus" that calls `start_focus` with the row id.
- [ ] **Step 2:** Run. Expected: FAIL.
- [ ] **Step 3:** Implement. Order:
  1. Unlinked clocks.
  2. Focus row.
  3. Open todos.
  4. Done disclosure (existing).

  Tags use `countdown` formatting for timers and `h:mm am` for reminders. Clicking a timer tag pauses or resumes it through `timer_action`.
- [ ] **Step 4:** Run the UI suite and `npx tsc --noEmit`. Expected: pass and clean.
- [ ] **Step 5:** Commit with message "Show timers, reminders and today's focus in the Todo list".

### Task 4: Remove the Tasks page and point every task route at the pop-up

**Files:**
- Modify: `src/uiNavigation.ts`, `src/panelNavigation.ts` and `src/App.tsx` (drop the `Tasks` page and the Overview links calling `setPage("Tasks", …)`; they call `run("todo_open")`).
- Modify: `src-tauri/src/main.rs` (`todo_open` command → `navigation::todo`).
- Modify: `src-tauri/src/navigation.rs` (`Destination::Timers` and `Reminders` open the todo window).
- Modify: tests that navigate to Tasks (`workflows.spec.ts` "checklist edits…", "keyboard can skip chrome…") move to the pop-up, keeping their assertions.

- [ ] **Step 1:** Write the failing test `tools window has no Tasks page`: no `.nav-item` with text "Tasks", and clicking the Overview timers link calls `todo_open`.
- [ ] **Step 2:** Run. Expected: FAIL.
- [ ] **Step 3:** Implement, then migrate the old Tasks tests to `/todo.html`.
- [ ] **Step 4:** Run the full UI and native suites. Expected: pass.
- [ ] **Step 5:** Commit with message "Make the Todo pop-up the only task list".

### Task 5: Build, relaunch, check
- [ ] **Step 1:** Build with `scripts/build.ps1`, replace `C:\Users\dharu\anything\desktop-buddy\desktop-buddy.exe`, and relaunch.
- [ ] **Step 2:** Clear `focus_asked` and `focus` in the settings table, relaunch, and check the greeting appears within about 3 s without taking focus. Then answer it and start the 25-minute focus.
