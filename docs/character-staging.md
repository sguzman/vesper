# VX06 — Authored character staging and expression motion

The engine interprets ordinary external story commands (show tess curious at left; show tess determined at center; hide tess). The story language and version-1 save format remain unchanged. Stage appearance is optional **game-owned** data in game.toml:

```toml
[characters.tess.expressions]
curious = "assets/characters/tess-curious.png"
determined = "assets/characters/tess-determined.png"

[characters.tess.stage]
offset_x = 24.0          # logical pixels relative to left/center/right
offset_y = 12.0
scale = 0.90             # 0.5..1.5
depth = 3.0              # 0..8 (larger draws in front)
crossfade_seconds = 0.7  # 0..2
enter_seconds = 0.5      # 0..2
exit_seconds = 0.4       # 0..2
slide_pixels = 20.0      # 0..60 vertical travel
idle_pixels = 2.5        # 0..8 vertical bobbing
idle_period_seconds = 4  # 0.5..10
```

The original three canonical slots still exist. Stage offsets are bounded and rendered positions are clamped inside compact/wide windows; sprites remain correctly centered and are ordered by author-specified depth. Images retain the existing 320×500 logical canvas and are streamed through the Bevy asset loader.

Expression changes can crossfade while retaining the outgoing expression until the incoming asset is ready. A fresh appearance can slide in; hide can slide/fade away; and optional idle bobbing uses a bounded periodic clock. These are **presentation-only ECS states**: no animation executes another story command, serializes entity IDs, or mutates narrative saves. The player **Reduced motion** override skips animation, including in-flight effects. Pausing or opening menus freezes motion; **K** completes an active entry/exit.

Without per-character stage data, all values take legacy-compatible defaults: existing left/center/right geometry, original size/depth, no idle movement or staged transitions, and original theme fade behavior. Validation rejects non-finite values, unsupported properties, excessively large offsets and invalid durations at game load and hot reload. Existing authored games and save files require no conversion.

## Graphical verification

The actors_smoke.py script clones Clockwork Moon to a temporary directory and synthesizes flat-color overlapping character fixtures for precise pixel assertions. It verifies depth/z-order, distinct expression crossfade frames, frozen pause snapshots, exit transitions, live manifest stage reordering, F5/F9 reconstruction, reduced-motion immediacy and compact/wide layouts. Original authored source and artwork are untouched. CI preserves screenshots in the vesper-actor-staging artifact.
