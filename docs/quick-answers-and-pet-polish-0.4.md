# Quick answers and pet polish

## Default command-bar typing

Type a plain question in the pet's native command bar and press Enter. Buddy opens its small answer window, connects through the installed signed-in Codex CLI, and uses its default model. No manual model choice is required for this text-only flow. The window shows the question, progress, cancellation, the answer or a connection error. It reuses the existing ChatGPT account sign-in available to Codex; this is not a connection to an open ChatGPT browser tab or support for arbitrary third-party provider sessions.

Slash commands retain their explicit behavior: `/d task` or `/todo task` creates a Todo; `/f name` finds files; `/w name` finds workspaces; `/sc` captures and fills; `/exp` captures and explains. Unknown slash commands stay errors/suggestions and are not silently sent to AI. `/s` is not added. The separate detailed-tools quick-capture field still creates local Todos as its label describes.

Plain typing no longer searches files automatically in the native bar. Only pressing Enter submits a question; merely editing text makes no AI request. Text questions do not attach a screen capture, held files or a workspace. The helper's tools and browsing are disabled, and it cannot perform local actions. Answers requiring live facts can be unverified. Generation is cancellable with a 90-second timeout; sign-in, network or allowance failures are surfaced. Temporary prompt-output files are cleaned after normal completion/failure/cancellation.

## Pet and Todo artwork

The fish, bone and bamboo remain borderless, but now use the matching pet's illustrated style. The cat's fish uses warm cream and calico orange with a brown outline; the dog's bone uses cream with warm shading and its brown outline; the panda's bamboo uses the same muted greens as its bolster. Due state lightens the fill without adding a surrounding circle.

Pet artwork and held-object poses render at 90% of their previous size. Toolbar, file cards, Todo controls and hit targets retain their existing sizes. No animation loop, dependency or asset download was added.

## Verification

Production frontend build, seven frontend unit tests, 54 native checks and 16 isolated UI workflows passed. Native checks cover plain-question routing versus local slash actions and offscreen idle/held artwork. UI checks cover quick-answer progress, cancellation and the existing tool surfaces. Native before/after renders were inspected; neither pet selection nor the user's open applications were changed during QA.

A real installed-CLI text-only smoke request using the signed-in account returned `2 + 2 = 4`. This verifies account/model connectivity and structured text output, not broad answer accuracy or the native Enter-to-popup interaction on the user's desktop. Live menu z-order, cross-app drag/drop and screen input remain separately documented device checks.
