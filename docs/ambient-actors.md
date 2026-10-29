# QOL06 — External ambient scene actors

Ambient actors are **background artwork**, independent of the named, speaking-character expression slots. The game supplies images and placement; the Bevy player supplies bounded spawning, scene visibility, persistence, and reconciling hot reload. Moving along a path or displaying animated frames is not part of QOL06.

## Manifest

Add a static actor to your game's `game.toml`:

```toml
[ambient_actors.bird]
image = "assets/bird.png"
anchor = [0.75, 0.28]
scale = 2.0
opacity = 1.0
depth = 1.0
scene = "dawn"
initially_visible = true
start_phase_seconds = 0.0
```

The image must be a game-root-relative, existing PNG; normal containment/path checks reject absolute paths, parent traversal and external symlinks. Each manifest permits at most 32 named actors. IDs must satisfy Vesper's identifier grammar. A scene association must name an existing scene; omitting `scene` makes the actor available in all scenes.

The `anchor` uses normalized 1280×720 canvas coordinates, `[x, y]`: 0 is the left/top edge and 1 is the right/bottom edge. Scale must be 0.1–3, opacity 0–1, depth -8–8, and each anchor coordinate 0–1; all numeric inputs must be finite. Render depth maps to a background layer above the background image but below speaking characters. `start_phase_seconds` is restricted to 0–120 and reserved for QOL07; **it does not animate the sprite yet**.

## Story commands and visibility

```text
@scene dawn
ambient hide bird
"The bird disappears behind the cloud."
ambient show bird
"There it is again."
```

These events affect ambient visibility only. They do not alter character expressions, background transitions, or audio. The current scene still constrains a scene-associated actor: `ambient show bird` cannot display it in an unrelated scene. If no explicit command is present, the manifest's `initially_visible` value applies.

Version-1 save data includes optional `presentation.ambient_visibility` overrides, keyed by actor ID, with an empty/absent map valid for older saves. Save restoration rejects unknown ambient actor keys, and F5/F9 preserve the existing overrides. The player resolves declared assets via Bevy's AssetServer rather than reading files in a frame update.

## Live authoring and limits

In `--watch` mode, Vesper's existing worker loads and validates changed `.vn`/TOML sources. Once accepted, the ambient reconciliation system can change an actor's image, placement, visibility, or alpha; it can also add and remove actors without restarting. Invalid scene references or unsafe image paths reject the new source and preserve the last valid scene. Asset hot reload remains subject to Bevy's image decoder and source availability.

Run the independent fixture:

```sh
cargo run --locked -p vesper-cli -- check-game examples/ambient-showcase
cargo run --locked -p vesper-player -- examples/ambient-showcase
```

The project's graphical acceptance scripts are `.github/scripts/ambient_actors_smoke.py` and `.github/scripts/ambient_reload_smoke.py`. They run the real Bevy player under Xvfb, test six viewport sizes, show/hide, scene restrictions, F5/F9, live scale changes, added/removed entities, and rejection of invalid edits. CI saves screenshots to `vesper-ambient-actors` and `vesper-ambient-reload`. Xvfb does not independently certify physical Wayland/Hyprland displays.

## Not included yet

**QOL07** will add finite authored paths, loops/ping-pong/one-shot travel, phase and facing controls, sprite frames, a walking NPC, a bird in flight and a traversing plane. QOL06 is static scene composition, not any of those motions.
