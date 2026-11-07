# Desktop Buddy UI/UX review

## Problem and chosen direction

The previous Activity screen mixed local tasks, full project resumption, CLI diagnostics and managed AI work. This made simple actions harder to find. The redesign gives each major task a literal destination and keeps everyday tools first. The native command bar, five-entry pet menu and compact editable Todo remain separate focused surfaces.

Three independent read-only proposals and peer reviews favored separating Overview, Projects and Codex. The selected direction combines that separation with progressive disclosure for creation forms, stored history and diagnostics. Meaningful dissent: another destination adds navigation cost. We accept that cost because managed requests have their own decisions and lifecycle, and test native routing, draft preservation and receipt navigation explicitly.

## Research and its limits

These are established usability principles and design hypotheses, not a survey of all users or evidence of universal visual preferences.

- [Nielsen Norman Group heuristics](https://www.nngroup.com/articles/ten-usability-heuristics/): recognition, visible status, user control and reduced irrelevant information support literal labels, clear feedback and quieter secondary controls.
- [Windows navigation guidance](https://learn.microsoft.com/en-us/windows/apps/design/basics/navigation-basics): organize navigation around meaningful destinations and keep its structure predictable.
- [WCAG target size guidance](https://www.w3.org/WAI/WCAG22/Understanding/target-size-minimum.html): small controls need sufficient target size or spacing. Checklist labels and control targets were enlarged; automated checks supplement keyboard inspection.
- [NN/g dark-mode review](https://www.nngroup.com/articles/dark-mode/): theme suitability depends on context. Preserve light, dark and system choices rather than assume one preference.

No runtime component library was added. Pinned development tools [Playwright](https://github.com/microsoft/playwright) and [axe-core](https://github.com/dequelabs/axe-core) provide repeatable interaction and accessibility checks.

## Implementation

- Overview prioritizes attention, checklist items and project continuation.
- Projects places saved workspaces beside resumable context; full Codex work has its own destination.
- Drafts persist across navigation and do not leak between project, task and shelf forms. Project next-step drafts remain separate per project.
- Completed todos, project creation, connection details and stored history use disclosures. Pending decisions and uncertain outcomes stay visible.
- Cat icon and pet-aware accent colors connect the tool window to the companion. Narrow layouts use an explicit Tools menu.
- Browser helper shows capture, review and fill stages while retaining manual review and no automatic submission.

## Validation

Production frontend build, seven frontend unit tests and 45 native tests passed. Eleven browser workflow tests passed, covering eight destinations across dark/light themes and 1100/380px viewports, checklist editing and cancellation, draft preservation, native request focus and acknowledgements, keyboard entry, pending decisions, receipt navigation the 300px compact Todo, empty data, loading recovery, system theme and reduced motion.

Final screenshot audit captured 32 view/theme/width combinations with zero detected axe WCAG A/AA violations and no horizontal document overflow. Inspected Overview and Projects on desktop and Tasks on the narrow light layout; fixed cramped project-card actions before the final confirmation. Test data is synthetic and all browser contexts are isolated from the user's session.

Automated checks do not prove full accessibility or product-market fit. A screen-reader study, representative user sessions and long-term native WebView use remain future validation. The browser helper still requires manual extension/native-host setup and supported page controls; no live page was filled during this redesign. Playback sync remains explicitly unavailable.
