# The Field Notebook — QOL03 visual authoring fixture

A standalone, fully external game demonstrating **two independently authored image-backed dialogue panels**. No custom engine code, paid services, fonts, cloud dependencies, characters, or background art.

- `notebook`: textured journal paper, ruled writing region, protected 160-pixel left writing inset, and retained spine, all with nine-slice scaling.
- `banner`: high-contrast dark dispatch card with edge treatment.
- `panel default`: removes the scene override, returning to the ordinary solid theme panel.
- Nine lengthy choices stress layout and scrolling at compact windows; all paths exercise banner and solid-panel fallback.

Run with `cargo run --locked -p vesper-player -- examples/panel-showcase`. The skins are indexed-color PNGs with transparent exterior corners; the underlying solid panel guarantees a readable base. Their edges are intentionally simple original pixel patterns, not third-party artwork.

Theme `[panel] skin = "notebook"` can set a default. A scene-level `panel banner` or `panel default` overrides it. `[panel_skins.<id>]` in the manifest defines image, writing region insets `[left,right,top,bottom]`, and nine-slice border in source pixels. Both samples keep all required text legible against a dark interior.
