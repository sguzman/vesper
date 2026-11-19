# Vesper action-adventure: first-playable gate

Owner-authorized direction: 2026-11-18. The old visual-novel engine code is
preserved, but is **not** the active genre and must not be presented as a
successful finished game.

## Only active milestone: feel-first encounter

A single original top-down room, a mechanically readable enemy, directional
attack, dodge, collision, switch and exit. Assets must be standalone, editable,
externally supplied and independent from visual-novel narrative/UI/audio.

**Verification layers (do not conflate):**

- **Written:** code and source-art files exist on the feature branch.
- **Compiled:** `cargo check --locked -p vesper-action-demo` passes.
- **Regressed:** pure mechanics tests and real graphical smoke pass.
- **Visually inspected:** actual screenshots reviewed at wide and tall sizes
  with no stretched art or clipped key information.
- **Accepted:** the owner plays it on physical hardware and says movement,
  dodge, attack feedback and art are promising enough to build on.

Until the owner accepts that first gate, **no multi-room campaign, random
dungeon, talent tree, inventory, RPG progression, elaborate storyline, quest
system or companion mechanics**. Do not create a false 100-point ledger for a
prototype or count legacy narrative milestones as action-game progress.

The transit-depot setting, worker and drone are only provisional test content.
No creative detail becomes final canon by merging tests.
