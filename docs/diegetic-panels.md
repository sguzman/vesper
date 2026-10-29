# QOL03 — Image-backed dialogue panels

This is an **external authoring feature**, not an embedded image editor. The engine loads source-validated PNG art through Bevy's asynchronous asset system; a solid dialogue panel always remains underneath. Individual games control their own artwork. The existing three story examples require no migration.

## Manifest and theme configuration

Place author-created PNGs **inside the game directory**, e.g. `assets/panels/notebook.png`. In `game.toml`:

```toml
[panel_skins.notebook]
image = "assets/panels/notebook.png"
insets = [35.0, 35.0, 26.0, 24.0]
slice_border = 26.0

[panel_skins.banner]
image = "assets/panels/banner.png"
insets = [25.0, 25.0, 20.0, 20.0]
slice_border = 22.0
```

`image` is a safe game-relative PNG. `insets` means **[left, right, top, bottom]** in logical UI pixels, *added* to the theme's ordinary panel padding. This reserves space for a journal binding, page ornaments or a banner frame. At compact sizes the extra insets scale down, with each edge limited to 20% of the logical viewport. Text, choices and hint layout share the same content region.

`slice_border` keeps that many **source-image pixels** from each edge when nine-slice scaling. Corners remain intact as the center stretches; use high-resolution original art for highly enlarged windows. This mode is *not* aspect-preserving whole-image scaling.

For a game-wide default, add `skin = "notebook"` under `[panel]` in the external theme TOML. No `skin` means the established solid-color panel. Scene commands can override either:

```text
@scene entry
panel notebook
"An entry written on a ruled page."
panel banner
"An incoming letter, delivered in the same scene."
panel default
"The authored theme default returns."
```

`panel default` clears the **scene override**; it returns to the theme default, which can itself be a skin. In the Field Notebook example there is no theme default skin, so `panel default` returns to plain dark panel styling. The scene command is not dynamic image generation, scripting or animation.

## Safety, compatibility and accessibility

- Panel keys must use the engine's ordinary identifier rules; `default` is reserved. Every referenced skin key must exist, including references from themes and story commands.
- Paths are canonicalized at game load and must be normal components inside the selected game root. Parent traversal, absolute paths and external symlinks are rejected.
- Each skin must name a PNG; its signature/IHDR must be valid, dimensions 32–4096 pixels per axis, total pixels at most 8,388,608, and a border smaller than half the shorter image dimension. Insets are finite 0–160; slice borders finite 2–160. Bevy performs full image decoding off the render thread and failed decoding leaves the opaque/specified solid backing.
- Player high-contrast mode suppresses decorative panel art entirely, preserving readable flat colors. Scaling, text/font preferences, choice selection and pause remain engine-owned. Reduced motion needs no special handling because the skins are static.
- A save's existing version-1 presentation object gains an optional `panel_skin` identifier. Older save JSON (without this field) loads with the normal theme default; F5/F9 restore the scene override without recompilation.
- The development `--watch` mode reloads changed manifests/themes on a worker thread; Bevy's asset watcher refreshes modified PNGs. Invalid authoring changes are rejected in favor of the prior valid state.

## Test and inspect

```sh
cargo run --locked -p vesper-cli -- check-game examples/panel-showcase
cargo test --locked -p vesper-game
cargo run --locked -p vesper-player -- examples/panel-showcase
```

CI adds `.github/scripts/panel_skins_smoke.py`, running the **real Bevy player under Xvfb** at 946×511, 955×520, 800×600, 900×700, 1280×720 and 1920×1080. It screenshots notebook styling, nine-choice navigation, banner-to-solid scene switching, F5/F9 skin recovery, and both regular and high-contrast appearance.

Inspect the `vesper-panel-skins` workflow artifact for captured pixels. Headless X11 is not physical Hyprland/Wayland certification; that hardware check remains separately useful.

## Boundaries

This feature does not introduce a per-panel font palette, an image editor, a general animation timeline, per-choice image styles, dynamically shaped parchment, or unrestricted filesystem image imports. Authored images are backgrounds behind native Bevy UI, and the same compiled engine handles other game folders unchanged.
