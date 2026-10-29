# Vesper — Reader QoL & Expressive Authoring Roadmap

**Status: 86 / 100 verified points — QOL01–QOL06 VERIFIED on merged main; QOL07/QOL08 ACTIVE.**

The preceding three phases (original MVP, Presentation Polish, Visual Experience) are independently **100 / 100 complete**. This is a new phase based on main commit `d90766f34a4d81be3064b1da5ee9ee4410aefca5`, created 2026-10-29.

## Current implementation and certification

**QOL01 (15 points) and QOL02 (10 points) are verified complete.** The installed-font browser and configurable auto-advance controls were integrated in [PR #16](https://github.com/sguzman/vesper/pull/16), merged at `6e936f036300aefa9602d974813db092bd54a925`. The [integration run](https://github.com/sguzman/vesper/actions/runs/40940541014) and [merged-main run](https://github.com/sguzman/vesper/actions/runs/40943723370) both passed all narrative and Bevy jobs. CI includes native graphical font search/preview/persistence, timing and settings smoke tests, regression screenshots, the complete Dawn branches, and Linux packaging.

**QOL03 (17 points) is verified complete.** [PR #17](https://github.com/sguzman/vesper/pull/17) merged at `fe11d1cf83558fcdf63412f125ac414d3aa0b4d0`; [PR CI](https://github.com/sguzman/vesper/actions/runs/40948246555) and [merged-main CI](https://github.com/sguzman/vesper/actions/runs/40951501022) both passed narrative and full Bevy graphical suites. The independent Field Notebook example verifies authored nine-slice notebook/banner art, protected writing margins, six window sizes, high contrast, scene switching, and F5/F9 panel recovery.

**QOL04 (17 points) is verified complete.** [PR #19](https://github.com/sguzman/vesper/pull/19) merged at `36d3a3d0e16ae4864fa12dd850a1621b332ba09d`. Both the [integration CI](https://github.com/sguzman/vesper/actions/runs/40953872164) and [merged-main CI](https://github.com/sguzman/vesper/actions/runs/40957118616) passed narrative and the full Bevy regression suite, including six-size styled word/first-word palette and high-contrast comparisons, Unicode safety, F5/F9 restore, all original stories and Linux packaging.

**QOL06 (13 points) is verified complete.** [PR #20](https://github.com/sguzman/vesper/pull/20) merged at `550b03ef9532c1a30ff29f23741301a9ade3092c`. Both [full PR CI](https://github.com/sguzman/vesper/actions/runs/40958095908) and [merged-main CI](https://github.com/sguzman/vesper/actions/runs/40961574567) passed narrative and Bevy tests: responsive ambient actor placement, show/hide and scene scope, F5/F9 visibility restoration, hot reload add/remove/invalid-source protection, original stories and compiler-free Linux packaging.

**QOL05 (14 points) is verified complete.** [PR #21](https://github.com/sguzman/vesper/pull/21) merged at `6516702317a6569f068d5c79a442e33206a2f2b0`. Both [full integration CI](https://github.com/sguzman/vesper/actions/runs/40965094969) and [merged-main CI](https://github.com/sguzman/vesper/actions/runs/40968256027) passed narrative and Bevy regression suites, including real per-word ember/shadow/shimmer colors, reduced-motion freezes, high-contrast overrides, all legacy stories, F5/F9 and Linux packaging.

**QOL07 (9 points) is active, not verified.** [PR #22](https://github.com/sguzman/vesper/pull/22) adds autonomous paths, flipbook frames and pause/save/reload tests; final branch and merged-main acceptance remain outstanding.

## Purpose

Make Vesper friendlier to **readers** and more expressive for **authors** in exactly the five requested areas: browse installed fonts, adjust auto-advance timing, put dialogue on a custom image/notebook/diary, apply dynamic flair to individual words, and add self-moving background characters, birds, and planes.

The per-word aesthetic is inspired conceptually by *Kentucky Route Zero*, *Celeste*, and *Hollow Knight*. No copyrighted game assets or code should be reused.

## Weighted milestone plan

| Milestone | Points | Dependencies | Status |
| --- | ---: | --- | --- |
| **QOL01 — 🔤 Searchable installed-font browser** | 15 | — | **Verified complete** |
| **QOL02 — ⏱️ Player-controlled auto-advance wait** | 10 | — | **Verified complete** |
| **QOL03 — 📓 Custom image-backed dialogue panels** | 17 | — | **Verified complete** |
| **QOL04 — ✍️ Inline style-span authoring foundation** | 17 | — | **Verified complete** |
| **QOL05 — 🔥 Per-word dynamic flair library** | 14 | QOL04 | **Verified complete** |
| **QOL06 — 🕊️ Independent ambient-scene actor infrastructure** | 13 | — | **Verified complete** |
| **QOL07 — 🚶 Autonomous ambient motion and flipbook frames** | 9 | QOL06 | Active — certification pending |
| **QOL08 — 🧪 Integration, authoring examples, final certification** | 5 | QOL01, QOL02, QOL03, QOL04, QOL05, QOL06, QOL07 | Active — certification pending |
| **Total** | **100** | | **86 verified** |

### Delivery order

**First: reader QoL (QOL01–QOL02, 25 points).** These can be implemented and dogfooded independently of the advanced authoring work. System fonts should be discoverable by search, not by holding arrow keys; the auto-advance setting must apply only after reveal completes.

**Second: illustrated dialogue surfaces (QOL03, 17 points).** Treat the art as an actual UI surface with a safe writing area and a fallback, not a stretched texture behind arbitrary text.

**Third: expressive typography (QOL04–QOL05, 31 points).** Separate robust text-span semantics from animation. Preserve sentence layout/reveal/accessibility before implementing fiery and shadowy word effects.

**Fourth: living backgrounds (QOL06–QOL07, 22 points).** Establish independent ambient actors first; then implement authored automatic paths and sprites. No general AI/pathfinding engine is needed.

**Last: integrated regression and example (QOL08, 5 points).** Combine the features, document authoring, rerun legacy stories and verify final main CI.

## Milestones and acceptance criteria

### QOL01 — 🔤 Searchable installed-font browser (15 points)

Replace the hard-coded family cycle with a searchable, previewable browser of truly installed system fonts.

- Appearance Settings contains a focused search input, case-insensitive incremental filtering, scrollable installed-family list, selected/current indicators, live specimen, and keyboard/mouse activation.
- Enumerate Bevy 0.20 system-font families using a cached discovery catalog; sort/deduplicate and never scan font directories or resolve large catalogs every frame. Zero/one/many-font hosts are handled.
- Persist chosen installed family through existing validated atomic player preferences; missing/uninstalled fonts fall back to the author's font and generic fallback, including multilingual glyph coverage; no arbitrary filesystem paths.
- Bevy input/screenshot QA proves search, preview, selection, restart persistence, missing-font fallback, long/Unicode names and unchanged old-game behavior.

**Reader/author example:** Search 'DejaVu', preview matching fonts immediately, select one, and keep it on restart.

### QOL02 — ⏱️ Player-controlled auto-advance wait (10 points)

Replace the current hard-coded 1.5-second hold with a persistent reader setting.

- Appearance offers Auto-advance delay: 0.5–20.0 seconds in 0.5-second increments, with a visible value and Reset; auto-advance remains separately opt-in and the old default stays 1.5s.
- Countdown begins only when the final grapheme has appeared; user interaction/scene change restarts it; pause, settings, history, saves and manual interaction suspend/reset it safely.
- Choices, unanswered prompts and endings never auto-select; no double advance when user presses Enter during or at timer expiration.
- New optional preference validates finite bounded values; legacy preferences and v1 story saves still load; deterministic clock tests plus GUI and cross-restart regressions pass.

**Reader/author example:** With auto advance enabled and delay 4.5s, a line remains fully visible for 4.5s before moving on.

### QOL03 — 📓 Custom image-backed dialogue panels (17 points)

Enable author-made notebook, diary, letter, parchment and banner dialogue surfaces instead of only flat colored rectangles.

- External theme/manifest supports optional game-relative panel image with transparency plus validated content insets/safe text region, padding and aspect-preserving or nine-slice border behavior.
- An authored default and bounded scene-selectable panel variant can place speaker, choices, dialogue and hints inside the artwork; missing assets fall back to a readable solid panel.
- Player text-scale, opacity and high-contrast override are honored; reduced motion, save/load, theme hot reload and scene switching are correct; legacy panels retain exact old behavior.
- Reject traversal/symlinks/invalid images and unsafe sizing; ship both notebook and banner examples; test wrapped dialogue and nine choices at all six certified window sizes.

**Reader/author example:** A diary's illustrated spine and page edge stay intact while the text sits inside its ruled writing area.

### QOL04 — ✍️ Inline style-span authoring foundation (17 points)

Let authors target individual words/phrases without changing or styling an entire line.

- Specify an optional escaped inline markup grammar in quoted .vn dialogue, with named styles, clear nesting/adjacency rules and file/line diagnostics for bad or unknown tags.
- Keep a clear plain-text representation for accessibility/history and structured span data for presentation; preserve Unicode grapheme-safe reveal, wrapping, measured glyph bounds, source fingerprint/hot reload and legacy save compatibility.
- Use Bevy TextSpan for supported per-span font/color; explicitly isolate effects needing per-glyph offset because TextSpan does not itself solve animated displacement.
- Unit tests cover quoted escapes, malformed/nested markup, ZWJ emoji, combining marks, multilingual dialogue, parser validation and six-size visual layout; unmarked stories render unchanged.

**Reader/author example:** Illustrative, not finalized syntax: "The [[fx=ember]]fire[[/fx]] is waking."

### QOL05 — 🔥 Per-word dynamic flair library (14 points)

Provide reusable theme-defined fiery, dark, wavering and shimmering words, inspired conceptually by expressive game typography.

- Implement static color/emphasis and at least three bounded dynamic effects such as ember flicker, shadow/dim, and gentle wave/tremble, with per-theme validated color, amplitude, rate and timing settings.
- Different marked words can coexist on a line without text reflow, glyph splitting, clipping or broken reveal/choices; expensive shaping and parsing are not repeated each frame.
- High-contrast mode and reduced motion override authored effects with readable static equivalents; pause, skip, hot reload and settings changes have deterministic behavior.
- Set finite active span/glyph caps and measure the worst authored case; visual masks and unit tests cover multibyte glyphs, long wraps, color/contrast, animation frames and no per-frame I/O.

**Reader/author example:** A word glows/flickers like ember; a separate word looks darkened; reduced motion shows both clearly as static text.

### QOL06 — 🕊️ Independent ambient-scene actor infrastructure (13 points)

Put birds, background NPCs, vehicles and other scenery actors on stage independently of dialogue characters or motes.

- Author-side manifest declares ambient IDs, sprite assets, scale, anchor, depth/layer, opacity, optional scene association, show/hide and bounded start phase; engine stays game-neutral.
- Loading, spawning, scene switching, hiding, restoring, save/reload and hot reload of ambient entities are deterministic and independent of foreground speaking actors.
- Validate secure root-contained image paths, finite scale/coordinates, capped active actors (target 24) and missing assets; old games that declare none render unchanged.
- Bevy smoke screenshots verify correct layer order, lifecycle, scene switching, responsive cover/crop alignment, pause/reduced-motion behavior and no blocking update-thread I/O.

**Reader/author example:** Declare a bird flock as background scenery, not as a story character who speaks.

### QOL07 — 🚶 Autonomous ambient motion and flipbook frames (9 points)

Let an NPC patrol, birds fly, and a plane cross the scene on their own with no per-frame story scripting.

- Provide finite authored waypoint/linear paths, loops/ping-pong/one-shot modes, speed or period, staggered repeat phases, facing/flip and optional bounded sprite-frame animation.
- Ship three test motions: walking background NPC, looping bird flight, and one-shot plane traverse; all operate independently of input and coexist with characters and particles.
- Time-based updates are framerate-independent and bounded; pause freezes motion; reduced motion freezes a meaningful pose or hides decoration; changes survive resize, save/load and hot reload predictably.
- Limit number of actors, path points, frames, speeds and texture usage; exclude pathfinding, collision physics, unlimited spawns or runtime image generation; screenshots capture start/middle/end and pause.

**Reader/author example:** An NPC crosses a bridge while a bird arcs overhead and a plane moves behind distant clouds.

### QOL08 — 🧪 Integration, authoring examples, final certification (5 points)

Show all five requested capabilities working together, with documentation and reliable regressions.

- At least one self-contained external authoring example demonstrates system-font selection, auto timing, diary panel, ember/dark word effects and NPC/bird/plane ambient movement; no game-specific engine branches.
- Document actual validated syntax, author-vs-player style precedence, image safe regions, accessibility and default behavior; do not present proposed syntax as implemented.
- Full narrative/Bevy CI passes on merged main; preserve screenshots across 946×511, 955×520, 800×600, 900×700, 1280×720 and 1920×1080 plus old three-game playthroughs, save/load, hot reload and Linux archive.
- Audit render-loop limits (no blocking scans/I/O, unbounded allocations or mandatory shaders); record physical Hyprland versus Xvfb limits; only then mark 100/100 verified.

**Reader/author example:** Run a combined scene with a paper notebook, fiery word, autonomous walkers, birds, and a plane.


## Illustrative authoring direction (not existing syntax)

The markup and TOML below show the desired author experience only. The parser/validation milestones must finalize names, escaping, format migration and boundary rules before these examples become executable.

```vn
@scene sky_garden
background garden
"The [[fx=ember]]fire[[/fx]] reveals a [[fx=shadow]]secret[[/fx]]."
```

```toml
# Example concepts, not recognized by the current engine.
[panel_variants.notebook]
image = "assets/ui/notebook.png"
scaling = "nine_slice"
content_insets = [68, 44, 64, 48]

[text_effects.ember]
kind = "flicker"
color = [1.0, 0.48, 0.12, 1.0]
intensity = 0.20

[ambients.bird]
image = "assets/ambient/bird.png"
motion = "loop_path"
layer = "behind_characters"
# Finite path points, animation frames and duration to be defined.
```

## Cross-cutting nonnegotiables

- **Generic Bevy engine:** all stories, effects, panel image assets and ambient behavior come from validated, relative, external game files. No Dawn-specific or source-specific branches in Rust.
- **Player precedence:** user-selected installed font, text scaling, high contrast and reduced motion must work even when an author provides complex effects or an illustrated panel. Never make text unreadable to preserve an effect.
- **Performance:** do not scan fonts, parse text, decode/generate art, block on files, or allocate unbounded geometry in the update/render loop. Prefer bounded cached structures, finite paths and background workers.
- **Safe authoring:** reject missing/unsafe asset references, bad effect tags and invalid motion parameters with useful file/line diagnostics and safe fallback. Existing saves, themes and all three example games continue to work.
- **Scope:** no WYSIWYG editor, runtime AI, physics/pathfinding, skeletal rigging, 3D expansion, mandatory shaders, paid service or Mac hardware.

## Verification / stewardship protocol

Only award points when a milestone meets its acceptance criteria on a **passing** source commit, including visual screenshots for UI changes and timing/geometry regression tests where needed. Log evidence and update `verified_weight`, `last_verified_commit`, milestone `status` and `next_milestone` in the machine-readable ledger. Report the total percentage and each completed round's percentage-point delta.

The final gate includes Xvfb renders at **946×511, 955×520, 800×600, 900×700, 1280×720 and 1920×1080**; image-panel/word-effect contrast and clipping; animated ambient start/mid/end; settings and keyboard/pointer interactions; reduced motion and high contrast; preferences migration; original three complete games; save/load, hot reload and compiler-free Linux bundling. Full narrative and Bevy CI must be green on final **main** for 100/100 verification. Physical Hyprland/Wayland GPU behavior remains a separate manual QA check; headless CI does not prove it.

**Source of truth:** [qol-roadmap.json](qol-roadmap.json). Prior completed phases: [MVP](roadmap.json) · [Polish](polish-roadmap.json) · [Visual Experience](visual-experience-roadmap.json).

**Next:** Complete QOL07 [motion PR #22](https://github.com/sguzman/vesper/pull/22) and certify on merged main. Finish QOL08 [Living Journal PR #23](https://github.com/sguzman/vesper/pull/23) and full-game graphical audit, then perform final merged-main 100-point certification.
