# Silent screen commands and archived features

/sc dismisses the native command bar on the first Enter even when a completion row is selected. It opens no bubble for success or failure. Kitty uses glasses while working and a completion/attention reaction afterward. Existing focus, supported-control and no-submit checks remain in place.

/exp uses a 100 ms focus-settle wait instead of the three-second manual capture countdown. Command captures omit the base64 preview and explanations skip Windows accessibility enumeration. Explicit manual captures retain their countdown, review preview and scope choices. Model requests use low reasoning effort, and /exp defaults to 1–3 short sentences directly addressing the question. Longer explanations remain available when asked. Bubble loading uses accessible dots with Cancel, without provider/connection text or repeated questions.

Codex chat and workspace implementations and saved data are retained but archived. Native aliases /codex, /ask, /w, /workspace, /resume and /projects are rejected. Native destinations and chat/start RPC entry points are gated, the chat surface is removed from routing, and Codex/Projects navigation plus project resume and Codex attention cards are hidden. Screen/quick answers continue using the signed-in Codex CLI; codex_models remains available to the screen-helper settings. Release gates live in src/features.ts and src-tauri/src/features.rs. Old feature-specific UI tests are retained as skipped archive scenarios.

Validation: 8 frontend unit tests and 59 native tests passed. A synthetic question PNG using the screen command’s actual CLI flags and JSON schema returned {explanation: '4.', answer: '', controlId: null} in approximately 7.05 seconds. This measures one model request, not guaranteed production latency; account/model/network conditions vary. No user screen was captured or filled during verification.

Official configuration reference checked: https://learn.chatgpt.com/docs/config-file/config-reference (model_reasoning_effort; supported levels depend on the model).

UI validation: 15 active workflows passed across dark/light themes and desktop/narrow windows, including keyboard/focus, accessibility, archived entry-point blocking and no connection chatter. Four archived feature scenarios remain skipped for later restoration.

The explanation Copy action is a single icon button in the bubble header beside Close, with a keyboard focus outline and accessible label. The duplicate bottom action is removed.
