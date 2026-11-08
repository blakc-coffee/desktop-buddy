# Kitty speech bubbles and direct commands

The native command bar now suggests 10 everyday actions. /tools opens management/settings. Existing hidden aliases remain compatible.

- Plain questions: Codex quick answer in a frameless, scrollable speech bubble.
- /exp question: capture the current display after three seconds; answer the supplied question in the bubble. No tools window is opened or restored.
- /sc question: capture the current display, identify a supported accessible text field, radio option or checkbox, and fill/select one answer. Submit buttons are excluded. No suitable control means an attention animation and no window. A failed action retains an explanation. Custom canvas controls and apps without Windows accessibility support remain unsupported.
- /codex: scrollable recent local chat history beside the pet. Refresh reads source status; notLoaded is shown as unavailable, never idle. A live shared app-server proxy enables explicit follow-ups: active turns use turn/steer with an expected turn ID; idle loaded threads use resume/start without changing their settings. Missing acknowledgement never triggers replay. The installed desktop runtime did not expose that shared proxy during a read-only probe, so messaging stays disabled here. There is no claim that standalone history reads know desktop task status.

Explanation bubbles show loading dots, Cancel, Close/Escape, readable code, copy actions and the original question. Pet glasses follow the illustrated eye positions and pose transform. File holds remain independent and never expire.

Connection: native Codex CLI login status was verified as ChatGPT authentication. Quick answers and screen explanations consume the signed-in account allowance. ChatGPT Work and Codex share usage; API-key pricing is separate. No credentials are read into Buddy and no reset/credits were purchased.

Sources checked November 8, 2026:
- https://learn.chatgpt.com/docs/app-server
- https://learn.chatgpt.com/docs/pricing

Validation excludes capturing or filling the user's active apps. UI tests use synthetic data in a separate headless browser. Native controls compile against Windows UI Automation; broad per-app fill acceptance remains device testing. Live follow-up delivery is tested with fixtures only and requires a source runtime exposing the proxy.

Verified: frontend build and 7 unit tests, 55 native tests, 18 UI/accessibility workflows, and a final 4-test bubble/chat regression pass. Offscreen native render checked glasses on Calico, Dog and Panda; separate headless previews checked dark/light bubbles and scrolling chat history.
