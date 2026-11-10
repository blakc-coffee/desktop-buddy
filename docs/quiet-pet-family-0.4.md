# Quiet pet family — 0.4

Choose a pet using `/pet` (arrow keys or mouse wheel to browse) or **right-click → Choose pet**. The selection persists and themes the launcher, menus, and tool accents.

The simple native lineup has 11 choices: Calico, Siamese, blue shorthair, Dachshund, Samoyed, Border collie, red panda, otter, capybara, Shiba Inu, and Panda. The existing Calico, Shiba, and Panda remain available.

Todo controls are borderless and retain the same checklist action: cats have fish, dogs have bones, red panda has an apple, otter has a clam, capybara has a leaf, and Panda has bamboo.

The nine approved preview PNGs are embedded unchanged in `assets/pet-reference/`. Their file hashes are recorded in `manifest.json`. The renderer omits only the connected dark presentation background, crops the retained artwork, and keeps its original aspect ratio, silhouette, face, colors, and coat details. It does not recreate the characters from circles and paths. The existing Shiba and Panda remain vector art.

The shared bounded animation states still supply context-specific whole-pet lean, bounce, file pickup, holding and handoff. The original facial expressions stay intact. Square glasses are mapped from the artwork's eye anchors while thinking or explaining; file overlays stay over the body. Quiet mode and Windows animation preferences still govern motion. There is no additional idle animation loop or network fetch.

Each selected reference is decoded once and its cropped pixels are cached. Automated checks cap the total for all nine decoded crops at 8 MiB; the large presentation canvases are discarded after decoding. Original PNGs increase the packaged executable size. This replaces the earlier approximate GDI shapes after a visual fidelity correction.

![Each group shows idle, held-reference, and glasses states](screenshots/quiet-pet-family.png)

Preview order: cats; new dogs; red panda/otter/capybara; original Shiba/Panda. Each group has three rows: idle, holding, glasses. This is an offscreen render by the actual Windows GDI renderer, not an illustration mockup.

Validation: 76 native tests passed (two deliberate live-inference probes ignored); the previous unchanged frontend passed eight TypeScript tests and 19 headless UI tests (four archived-flow tests skipped). Native coverage includes every selectable pet rendered in all three states and appearance-key/token/context compatibility. Frontend production build passed. The packaged release embeds the frontend through Tauri.

CS learning: share the state machine, specialize the renderer. `Appearance::family()` selects a reusable animation/palette family, while `context_pose()` and the approved artwork own species details. One catalogue feeds both pet choosers so their ordering and available choices cannot drift. To practice, trace a Todo-completed event from the existing cue to a dog's completion reaction in `src-tauri/src/pet_quiet.rs`.
