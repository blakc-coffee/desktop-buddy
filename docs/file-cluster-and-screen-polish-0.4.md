# File cluster and screen helper polish

Updated: [expandable, individually draggable file cards](expandable-file-cards-0.4.md) replace the original click-through labels described below.

## Held files

Two or more held files create native click-through labels beside the pet. Every label shows its filename and a drawn type icon: folder, PDF, text/document, spreadsheet, image, code, archive or generic file. Long filenames ellipsize; the existing held-file command provides full names. Labels follow the pet and clamp to the monitor work area. They have no idle animation or independent pointer interaction.

The existing pickup, held and release poses remain. The pet continues holding its pouch/document while multiple labels are shown. File references no longer have a countdown or automatic expiry. They last until clear, successful drag/copy handoff, replacement or app exit. This does not change focus timers and never deletes source files. A hold generation counter preserves newer file batches across OLE's nested drag loop.

## Screen helper

Capture scope can be full display, the visible active-window area, or a rectangle selected on the display under the pointer. The preference is reused by /sc and /exp. Area selection is an explicit foreground interaction; Escape, focus loss and a two-minute picker timeout cancel it. It does not run in the background. Active-window capture reads visible pixels inside its bounds; occluding windows remain visible.

Generated answer text stays available with Copy answer when automatic typing is blocked, partial or ambiguous. No automatic retry is added. The original focused native control is checked after restoring the original window, in addition to window/process/title checks. Browser fields and custom editors can share one native control, so this is not universal field identity verification. Successful SendInput remains unverified delivery.

/exp opens a compact themed explanation window near the pet after generation. It shows readable paragraphs, fenced code, answer text and separate copy actions. The main tools window remains available through Open tools. Clearing the capture clears the retained result and closes its explanation window.

## Validation and limits

The production frontend build, seven frontend unit tests, 50 native tests and 15 isolated UI workflow tests passed. The native suite includes file type classification, 20-item placement without overlaps at desktop edges across three scales, offscreen cluster rendering with the held pose, screen-region geometry and existing transfer/input tests. Offscreen rendering opens no desktop windows. Browser fixtures cover scope persistence, blocked-input answer recovery and the compact explanation surface as well as prior workflows.

Real desktop typing, area selection and native overlay hit testing were not exercised against the user's active session. No user screen was captured, no file batch was loaded into the live pet, and no input was injected during QA. Full compatibility still needs an isolated native-app acceptance run. The Windows focus/control APIs cannot prove which HTML field accepted text.
