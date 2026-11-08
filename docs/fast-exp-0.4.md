# Faster explanations and square themed scrollbars

Default /exp and its screenshot follow-ups now use GPT-6 Luna with reasoning effort none and a small, fixed screen-reading instruction file. This replaces the coding-agent instruction profile only for explanation requests. Tools remain disabled, user configuration/rules remain ignored, and the request stays ephemeral and read-only. /sc keeps the previous default/low reasoning behavior; explicitly selected models retain low reasoning and are carried into follow-ups. The private instruction file is removed with the other request files. Child completion checks now poll every 40 ms instead of 150 ms.

This mode prioritizes short, quick explanations. A stronger model can still be explicitly selected in the screen helper for more demanding questions. It uses the existing sign-in and standard service; no paid fast tier, API key or billing change was added.

Synthetic 1280x800 study-screen test: the previous GPT-6.1 Sol/low path took 11.33 and 25.76 seconds. GPT-6 Luna/none with the small instruction profile took 5.77 and 6.14 seconds, returning correct O(n) time, O(1) space and input-doubling explanations. These small-sample end-to-end CLI timings are not guaranteed production latency. Network, provider load, question complexity and image content still matter. No active user screen was captured during these comparisons.

The answer and code scrollbars use 4-pixel square thumbs/tracks, the active theme accent for the thumb and surface for the track. Chromium/WebView2 uses explicit scrollbar styling; other engines retain thin themed native styling. Keyboard scrolling and focus remain available.

Verification: 63 native tests and four targeted browser workflows passed, including the scrollbar width/radius contract, accessibility, scrolling, follow-up submission and ready-only presentation.

Official sources checked November 8, 2026:
- https://developers.openai.com/api/docs/models/gpt-6-luna (image input and reasoning effort none; also verified through the installed CLI)
- https://learn.chatgpt.com/docs/config-file/config-reference (model_instructions_file replaces built-in instructions)
