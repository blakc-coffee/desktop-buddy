# Additive holds, square glasses, previews and quiet mode

Implemented November 10, 2026 under the user's existing usage-guard override and explicit selection of glasses option 2. No additional AI inference or usage reset credit was used for these features/tests.

## Changes

- Oversized chocolate-brown rounded-square nerd glasses replace the small round accessory. Native GDI paths follow each species' eye anchors; panda frames use a narrower fit. The accessory still appears during thinking/explanation. No image loader, resident animation service or runtime dependency was added.
- Successive file drops merge into the held pile. Existing paths remain, duplicates are ignored, different files sharing a name remain distinct. The total hold is bounded to 20 references. A full batch is rejected without replacing the previous pile.
- Virtual-file imports use a FIFO queue (four waiting batches plus one active import) and completion queue. A clear epoch changes only on explicit Clear, so local additions no longer invalidate an unfinished import. Clearing also drains queued jobs; late results cannot restore cleared items. Individual drag-out keeps its existing stale-generation protection and never deletes original files.
- Selected text and HTTP/HTTPS links become locally cached references with text/link labels and icons. Plain-text whitespace and Unicode are preserved. A dropped link is not fetched. Existing explicit DownloadURL/image drags still import file contents. Each reference is limited to 32 KiB; metadata registry and disk cache are bounded. Generated copies remain in the existing Imported files cache so delayed drag receivers retain access.
- Individual text/link drag and copy exposes native CF_UNICODETEXT; links also expose UniformResourceLocatorW. File-compatible fallback copies are available. Bulk copy of only text/link cards joins their text; a mixed batch retains file-transfer semantics.
- Clicking a file card opens a separate small preview; dragging still transfers the card. Text/code previews read at most 64 KiB and render as escaped text. PNG/JPEG/BMP previews accept at most 8 MiB and four million pixels, with dimension limits; animated PNG is excluded. PDF/Office/GIF/WebP and oversized files show details and an explicit Open in app action. Folder contents are not scanned. Links open only after an explicit click.
- Preview work runs outside the UI thread. A generation check rejects stale reads after a newer selection or Close. The dedicated frontend entry excludes the settings dashboard, and hidden windows reuse the existing 45-second reclamation policy. Closing clears preview payloads.
- Quiet mode is a persistent Settings toggle. It suppresses decorative motion/loading bob and Windows notifications while timers/reminders and due badges continue. It does not disable direct commands, file transfers, or explicit previews. No extra slash command, context-menu item, or shortcut was added. Quiet mode does not automatically detect fullscreen applications.

## Validation

Native fixture tests cover consecutive/duplicate/case-alias drops, bounded capacity, queued completion/clear identity, Unicode native transfer, link versus download classification, safe text preview and image header bounds. No real user screen or clipboard was accessed. Live browser/editor drag handling was not exercised against the user's session; native transfer formats were verified with real Windows data objects.

Frontend checks cover the quiet toggle, unchanged timers, text rendering without HTML execution, narrow-window overflow, accessibility, explicit link opening, and standalone bundle isolation. Offscreen pet renders verify the glasses on cat, dog and panda. The release build uses Tauri's packaged protocol. Full PDF/Office previews and automatic fullscreen quiet mode remain outside this batch.

Final checks: 74 native tests passed (two inference probes skipped), eight TypeScript tests passed, and 19 UI workflows passed (four archived workflows skipped). Small and desktop synthetic previews were inspected. Native case aliases are tested using a real temporary Windows file.

## Learn while building

A merge preserves existing collection entries; assignment replaces them. The import FIFO prevents one completion from overwriting another. A separate clear epoch makes delayed work valid until the user clears it, instead of treating every added file as cancellation. The payload enum separates text/link data from file downloads so a link need not trigger a network request. Optional exercise: drop A, then B, then A again; predict the count. Then clear while an import is pending and explain why its completion must be ignored.
