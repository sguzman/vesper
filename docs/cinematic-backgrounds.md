# VX05 — External cinematic backgrounds

Background effects are configured in **external game theme TOML**, not hardcoded in a story or player binary. A story can issue ordinary `background <id>` commands to choose which authored background to show. The existing `fade_seconds` theme field continues to control the legacy single-background fade when no cinematic mode is selected.

```toml
[cinematic]
transition = "crossfade" # legacy | crossfade | fade_through_color | soft_slide
seconds = 1.4
through_color = [0.26, 0.09, 0.32, 1.0]
slide_pixels = 35.0
motion_seconds = 12.0
motion_zoom = 1.12
motion_pan = [0.02, -0.01]
```

**Crossfade** blends the incoming art over the outgoing art. **Fade-through-color** moves through an opaque theme-defined color curtain before revealing the new image. **Soft-slide** uses a short, edge-safe horizontal movement and a gentle opacity ramp. The outgoing asset stays visible while the new image loads; transitions do not block narrative keyboard input. An optional Ken Burns motion profile applies modest continuous pan/zoom to each new background.

Defaults are backward compatible: if `[cinematic]` is absent, the mode is `legacy`, preserving the authored `fade_seconds` behavior. `seconds` must be 0–3; `slide_pixels` 0–80; `motion_seconds` 0–120, where zero turns off optional movement; `motion_zoom` 1.0–1.15; `motion_pan` components -0.04–0.04. The transition color has alpha 1. Invalid or non-finite configuration is rejected before applying a live theme reload.

When a transition is active, **P** pauses it alongside the story, while **K** skips to its completed visual state. Keyboard actions remain available during a transition. After F9 restoration or a valid hot reload, outdated backdrops and curtains must not survive. The engine computes pan against the actual window's aspect-preserving cover rectangle and clamps all offsets to available image overscan, preventing exposed image edges at compact or ultrawide dimensions.

The graphical acceptance gate is `.github/scripts/cinematics_smoke.py`, which duplicates the two externally authored fairy tales into temporary directories, configures each of the three modes, and records start/mid/end frames. It exercises pause, skip, F5/F9 and live theme reload, with unchanged original sources and no test-only narrative commands.
