# VX08 — Reading history and manual save slots

The engine records only dialogue emitted by the interpreter as the player actually progresses. It never reads future scenes or attempts to predict branches. Press **H** to open the session backlog, and **H** or **Escape** to close. Up/Down and PageUp/PageDown scroll; the panel also has pointer-accessible Older, Newer and Close controls. Reading history never executes a story command or changes the selected choice. While open, dialogue reveal, cinematic backgrounds, character motion and atmosphere are paused.

## Memory bound and rewind semantics

History is strictly ephemeral and never added to the save schema. It retains the newest **80 entries**, with scene markers and individual dialogue entries; a dialogue's speaker and text are each capped at **2,048 Unicode grapheme clusters** to avoid splitting emoji. On restart or successful load, entries are cleared, and only a currently displayed saved dialogue line may be seeded back into the backlog. This prevents a player loading an earlier branch from seeing dialogue encountered later in another branch. A save's scene name comes from the restored interpreter, not from an authoring file scan.

## Manual save slots

Press **F6** to open the structured Save/Load overlay. It provides three named slots distinct from the legacy quick file, displays the saved scene and recent modified age, and has keyboard and pointer controls. Use Up/Down to select; Left/Right to switch Save/Load; Enter to use the selected slot; Escape or F6 to close. The panel requires **an explicit second confirmation before overwriting an occupied slot**; Escape cancels without mutating the file.

Slot filenames reside beside the game-bound quick save:

- **F5 / F9**: `<game-id>.json` (unchanged, identical version-1 format)
- **Manual slot 1..3**: `<game-id>.slot1.json`, `<game-id>.slot2.json`, `<game-id>.slot3.json`

All slot scanning, JSON decode, metadata reads, writes, and load validation use the existing background worker, bounded by the original 8 MiB decoder, game ID checks and story fingerprint matching. Saving still writes a temporary file, flushes it, atomically renames it and synchronizes the parent directory on Unix. Invalid, empty, and incompatible slots are shown rather than silently loaded. Loading a slot restores a valid saved story and closes the overlay only after validation completes.

The save format remains **version 1** and existing quick saves remain loadable without migration. The additional UI never changes saved character entity IDs, scripts, audio, preferences or externally authored assets.

## Verification

`.github/scripts/history_saves_smoke.py` runs a copied Clockwork Moon fixture through a real Bevy window, recording scenes, opening/scrolling/closing history without advancing dialogue, testing an occupied-slot overwrite cancellation and byte preservation, manually saving/loading a pending choice, validating that F5/F9 still use the old quick path and that loading earlier history clears later-branch dialogue. CI captures screenshots and logs in `vesper-history-saves`.

VX08's seven roadmap points remain unverified until this acceptance gate and all previous full graphical regressions pass.
