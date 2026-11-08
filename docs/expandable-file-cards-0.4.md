# Expandable file cards and menu layering

Up to three held files show individual cards. Four or more start as a compact count card. Click Show to reveal all filenames; click Hide to collapse them. A new hold resets the expansion state. Removing one file from an expanded hold keeps it expanded while more than three remain. The last file retains a draggable card.

Drag an individual card onto a folder or compatible app to hand off only that file. The original source file stays in place: transfers remain copy-only. A successful accepted drop removes that file from the pet's hold; cancelled, rejected and failed drops retain it. Clipboard contents are not changed by card dragging. Dragging the pet still transfers the complete batch; Alt-drag still moves the pet. A generation check prevents an older transfer from consuming files in a replacement hold.

Cards now receive mouse input instead of being click-through labels. The native menu hides cards while open, temporarily allows its normally non-activating owner to take foreground ownership, and raises each popup/submenu into the topmost group. Original owner styles and menu hooks are restored on dismissal. Card repaints also keep an open command bar above the cards. The five-item right-click menu is preserved.

Fish, bone and bamboo Todo symbols now draw directly without circular backgrounds. Due state uses the lighter pet ink color; hit bounds and checklist actions are preserved. File holds still have no timer or automatic expiry, and the existing holding pose remains.

## Validation

The production frontend build and 54 native regression checks passed. Checks cover collapse threshold, expanded identity/order, single-file removal, stale-batch protection, 21-row placement at 1920x1040 and 1366x720 across three scales, normal/held pet renders, and temporary menu-owner styles in a hidden window. The hidden fixture is never shown or made foreground. Offscreen previews compare collapsed/expanded layouts and borderless Todo symbols.

No user's files were loaded or dragged during QA, and no live context menu was opened. Real cross-app drag/drop and visual z-order above other applications remain device acceptance checks. An accepted native copy effect is not independent verification that an asynchronous destination finished copying. Ordinary topmost windows cannot overlay Windows secure desktops.

The menu ownership handling follows Microsoft's [TrackPopupMenu guidance](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-trackpopupmenu). Popup creation hooks are scoped to the current process and owner thread and are removed when tracking ends.
