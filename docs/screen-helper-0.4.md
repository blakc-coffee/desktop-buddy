# Screen helper commands

Update: [capture scope, recoverable answers and compact explanations](file-cluster-and-screen-polish-0.4.md) are now implemented. `/exp` results open beside the pet.

- `/exp`: capture one display and explain its visible content in Buddy.
- `/sc`: capture one display, generate one text answer, return focus to the captured app and send Unicode text to its focused field.
- `/screen` and `/fill` alias `/sc`; `/explain` aliases `/exp`.

Buddy hides its own tools window for three seconds. During the countdown, show the question and focus its answer field, with the pointer on that display. Capture covers the display under the pointer, including other visible windows. It works with browsers, editors, documents and ordinary desktop apps without an extension. Protected video and restricted desktops may be blank or unavailable.

Choose a model once in Screen helper. The installed, signed-in Codex CLI and a model accepting image inputs are required. A saved model lets commands proceed automatically. The ordinary Capture screen button previews locally first and only sends when Explain capture is pressed. Command invocations authorize sending the capture and optional question; `/sc` additionally authorizes typing.

The capture is held in memory, expires for requests after ten minutes, and is replaced or cleared explicitly. Generation creates fixed temporary PNG/schema/result files in a unique application-owned folder and removes them when the request ends normally, fails or is cancelled. Abrupt process termination may leave temporary files; no capture is added to project history or backups. Provider-side data handling follows the signed-in account.

Automatic typing checks the original window handle, process ID, title and foreground focus. It sends Unicode text, with no virtual Enter key, shortcuts, clipboard changes, click or submit action. Terminal top-level windows are rejected. It cannot identify or click arbitrary radio/checkbox choices; ambiguous questions produce no text answer. Windows elevation, custom editors and protected input can block typing. A successful SendInput means text events were sent, not that every target accepted them. Partial or blocked input must be inspected before retrying. Changing a field inside an unchanged window cannot always be detected. Optional browser filling retains its extension-based selected-field validation and readback.

## Verification

- Production frontend build and seven frontend unit tests passed.
- 47 native tests passed, including PNG channel orientation, capture identity, busy exclusion, command aliases and Unicode event construction without Enter/shortcut keys.
- 13 isolated browser workflow tests passed, including command-specific explain/fill routing once only, no browser-connection requirement, local-only manual preview, capture failure recovery, narrow view accessibility and the existing tools in dark/light layouts.
- A real installed-CLI vision run on a synthetic 2+2 PNG returned answer 4 and an explanation. This verifies image ingestion, not broad solution accuracy.
- Desktop and narrow screenshots were generated using synthetic data. No live user screen was captured and no text was injected into the user's desktop during QA. End-to-end capture/focus/typing in arbitrary native apps remains unverified to preserve the user's active session.

The CLI image attachment flags were checked against the installed CLI and [official command-line reference](https://learn.chatgpt.com/docs/cli/reference). A direct dependency on the already locked png 0.18.1 encoder creates lossless screenshots without a new screenshot framework.
