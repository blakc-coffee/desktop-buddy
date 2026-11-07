# Buddy page helper

This optional Chrome/Edge companion connects the page you explicitly choose to Desktop Buddy's `/fill` command. It preserves your existing browser session.

1. Keep the entire release folder together. It contains `desktop-buddy.exe`, `buddy-browser-host.exe`, and this `browser-helper` folder.
2. In Chrome's or Edge's extension management page, enable developer mode and choose **Load unpacked**, selecting this folder.
3. Copy the extension ID shown there (also shown in the helper popup).
4. Run `Install-Browser-Helper.ps1 -ExtensionId YOUR_EXTENSION_ID` from this folder. It registers only the current user's native messaging host; administrator access is unnecessary. Moving the release folder requires rerunning this step. If Chrome and Edge assign different extension IDs, run it separately with `-Browser Chrome` and `-Browser Edge`, using each browser's ID.
5. On the question page, click the helper and **Connect this page**. Access is requested only for that website. The connection expires after five minutes, on navigation, or when disconnected.
6. Use `/fill` in Buddy. Capture the questions, choose answer fields and a Codex model, generate, review, then fill. Buddy does not submit, run code, or change the selected coding language.

Supported controls: ordinary text inputs/textareas, single-select dropdowns, checkbox controls, and radio groups, including accessible radio/checkbox roles. Password/personal-information controls and hidden/disabled controls are excluded. CodeMirror 5 and Ace are supported when the page exposes their existing editor instance; Monaco is supported when the page exposes exactly one visible editor and its one model through the public Monaco API. Each adapter reads the editor model back after filling. CodeMirror 6, private/bundled Monaco instances, cross-origin frames, and closed shadow roots remain unsupported. Buddy reports unsupported editors rather than editing their hidden textareas. Adapter fixtures pass; compatibility with a particular live coding platform still requires verification.

Answer generation sends only the captured question text and selected answer controls to the chosen Codex model, using its account allowance. The CLI runs an ephemeral, read-only request with browser, shell, image, plugin, and agent tools disabled. Responses must match known field identities and allowed choices. Every answer is checked again against the original page before filling. If framework events interfere, filling stops and reports the verified count. An uncertain Apply outcome is never automatically retried.

Page snapshots and answer plans live in Buddy memory. Bounded native-message replies briefly pass through the current user's local application data and expire after five minutes. Generated-answer files are removed after generation. Disconnect closes the bridge and removes its connection files. Closing Buddy during generation stops its owned CLI worker.

To remove: remove the extension in browser settings and delete the `local.dharu.desktopbuddy.page_helper` native messaging host entry under the current user's `Software/Google/Chrome/NativeMessagingHosts` and/or `Software/Microsoft/Edge/NativeMessagingHosts`. No browser restart, debugging port, or access to cookies/passwords is needed.
