# Snooze, undo, autostart — design (0.4.x)

Approved by Dharu on 24 November 2026. Small change; short design.

## Intent
These close three items the 0.4 build report deferred (`docs/build-report-0.4.md:37`):
- a due reminder can be postponed without retyping it,
- a misclick on a hover-trash or "Clear held references" can be undone,
- Buddy can start with Windows.

## Goals / non-goals / anti-goals
- Goals: snooze reminder and timer alerts (+10m, +1h); undo of the most recent removal for 10 s; an opt-in "Start with Windows" setting.
- Non-goals: multi-level undo history, snoozing todos (they have no due time), buttons on the Windows notification (the Tauri notification plugin has no desktop actions).
- Anti-goals: autostart being on by default; a Windows service or admin rights; any new crate, database table or background loop; undo deleting or recreating the user's original files.

## Constraints
RAM must stay low: the native idle state is about 2.4 MiB. Local commands must not use the network or AI. The local-tool latency policy in `AGENTS.md` applies.

## Approaches
| Feature | Chosen | Rejected and why |
|---|---|---|
| Autostart | A per-user registry `Run` value (`HKCU\Software\Microsoft\Windows\CurrentVersion\Run`, name `DesktopBuddy`) written with the `windows` crate. It only needs the `Win32_System_Registry` feature added. | `tauri-plugin-autostart` writes the same key but adds a dependency. A Task Scheduler entry is heavier and gains nothing here. A Windows service is wrong for a per-user app with a UI. |
| Undo | A single in-memory slot: the last removed item, or the last cleared hold, with a timestamp. It is valid for 10 s. | A trash table needs a migration and cleanup rules. A browser-only undo would not cover the native hold menu. |
| Snooze | A new command reopens the original reminder (`done=false`, `due_at=now+N`) and deletes the `due-<id>` alert in one transaction, then wakes the scheduler. | Creating a new reminder item would lose the original id and duplicate items. |

## Decided behaviour
- **Snooze.** Each unhandled alert in Needs attention gets "+10m" and "+1h" buttons. The alert's id is `due-<originalId>`. If the original is a reminder, it is reopened. If it is a finished timer, a new reminder is created with the same title (a timer cannot be re-run in place). The pet shows the `ReminderSaved` cue. Allowed lengths are 1–1440 minutes.
- **Undo, items.** `remove_item` saves the deleted `Item` in `Shared.undo`. `undo_remove` puts it back with the same id if 10 s or less have passed; otherwise it returns "Nothing to undo." After a removal, the Todo window and the tools panel show "Removed · Undo" for 10 s. Ctrl+Z in the Todo window does the same.
- **Undo, holds.** Before "Clear held references" clears anything, the pet saves the held paths and their `held_refs` entries. While the snapshot is 30 s old or less, the right-click menu shows "Undo clear". Restoring uses the normal merge path, so the 20-reference limit and duplicate checks still apply. A new hold discards the snapshot. Clearing never deleted files, so undo only restores the list.
- **Autostart.** A checked item "Start with Windows" in the pet's right-click menu. Turning it on writes the quoted path of the current exe; turning it off deletes the value. The menu check mark comes from the registry itself, which is the source of truth. On startup, if the value exists but points somewhere other than the current exe, it is rewritten, because Buddy is portable.

## Left to the builder
Exact button styling (reuse the existing `.message.notice` and button classes), test names, and how the undo strip sits in the compact 300 px Todo window.
