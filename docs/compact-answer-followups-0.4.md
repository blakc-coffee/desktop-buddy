# Compact ready-only answers

Explanation and quick-answer windows remain hidden until the response is ready. The frontend measures loaded content before presenting the native window. Windows are 300–380 logical pixels wide and at most 360 high, with a custom thin scrollbar on the answer region. Header Copy/Close and the follow-up input remain outside the scroll area. The answer region is keyboard focusable.

While a request runs, Kitty wears glasses and gently bobs. The animation layers over the existing pose, preserving held files and motion. Windows reduced-motion settings disable the bob while retaining glasses as static feedback. Completion stops the loading timer. No loading-dot popup is opened.

Outside clicks close the window without consuming the underlying click. Native focus loss and a scoped low-level mouse hook cover normal apps, the desktop and non-activating pet windows. Hooks are removed on bubble close/replacement or pet destruction. Escape and Close remain available.

Follow-ups use the prior question and answer as bounded context. Screen explanations reuse the original in-memory capture while it is current (10-minute existing capture lifetime); they do not capture the user's desktop again. Expired captures require /exp again. Quick-answer follow-ups use text context. This does not restore the archived Codex chats/workspaces features or create a persistent chat database.

Validation uses isolated browser fixtures, native hit-boundary tests, bounded Unicode follow-up context tests and offscreen pet rendering. It does not capture or fill the user's active applications. Native outside-click delivery and pet loading appearance still benefit from real-device use; no broad per-app claims are made.

Checks passed: 61 native tests, 8 frontend unit tests, and the targeted ready-only/scroll/follow-up/outside-click/accessibility workflows after fixing keyboard focus on the scrolling region. Headless previews show a short answer at about 196 logical pixels high and long content capped at 336 inside the 360-pixel native window.
