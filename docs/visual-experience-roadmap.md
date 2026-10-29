# Vesper — Visual Experience Roadmap

**Status:** in progress, **82 / 100 verified points** (VX01–VX09 verified; VX10 next). This is a **new** phase; the original MVP and prior presentation-polish ledgers remain independently complete at 100 / 100.

**Goal:** A lovely, expressive visual novel reader: readable, configurable dialogue; distinct choices; gentle cinematic staging; history and saves; accessible motion controls; and a complete original game that demonstrates all of them.

## Milestones

| ID | Weight | Milestone | Proof of completion |
| --- | ---: | --- | --- |
| VX01 | 8 | 🧱 Player preferences foundation — verified | Preferences are stored in an atomic, bounded, versioned JSON file under the platform-appropriate user data/config directory; no writes on the render/update thread. |
| VX02 | 14 | 🎛️ Live appearance settings — verified ([CI run 40721330853](https://github.com/sguzman/vesper/actions/runs/40721330853)) | From pause/main menus, players can adjust dialogue panel opacity (bounded 0.60–1.00), dialogue font size multiplier (0.80–1.60), choose an installed or game-approved font with safe fallback, and restore defaults. |
| VX03 | 10 | 💬 Dialogue and choice composition — verified ([CI run 40725819513](https://github.com/sguzman/vesper/actions/runs/40725819513)) | Each choice is a separate, engine-owned UI entity with selection/highlight, focus, mouse target and keyboard action; choice data remains supplied by external .vn scripts. |
| VX04 | 8 | ⌨️ Text pacing and reading cues — verified ([CI run 40745061158](https://github.com/sguzman/vesper/actions/runs/40745061158)) | A reveal effect supports instant/off, slow, normal and fast player choices, with Unicode grapheme-safe pacing and no string corruption. |
| VX05 | 10 | 🎞️ Cinematic background presentation — verified ([CI run 40763057822](https://github.com/sguzman/vesper/actions/runs/40763057822)) | External theme/story data can specify bounded crossfade, fade-through-color and soft-slide background transitions, with compatible defaults for legacy .vn games. |
| VX06 | 10 | 🎭 Character staging and expression polish — verified ([CI run 40775856131](https://github.com/sguzman/vesper/actions/runs/40775856131)) | Externally declared sprite anchors/scale/depth and expression-crossfades work for multiple characters without engine references to specific games. |
| VX07 | 7 | ✨ Atmosphere and micro-interactions — verified ([CI run 40785668645](https://github.com/sguzman/vesper/actions/runs/40785668645)) | Provide a small, bounded data-driven library of 2D atmospheric presets such as drifting motes, gentle glow and vignette, with deterministic seeds and no mandatory shaders. |
| VX08 | 7 | 📖 Reading history and polished saves — verified ([CI run 40789199729](https://github.com/sguzman/vesper/actions/runs/40789199729)) | A readable, scrollable dialogue backlog shows the current session's recent lines with speaker names and scene boundaries, with a documented bounded memory limit and no spoilers from future branches. |
| VX09 | 8 | ♿ Accessibility and experience regression — verified ([CI run 40794656091](https://github.com/sguzman/vesper/actions/runs/40794656091)) | Readable defaults and player controls for high-contrast text, reduced motion, text speed/instant reveal, and safe font fallback are documented and tested. |
| VX10 | 18 | 🌠 Original generated-asset showcase game | Create a self-contained examples/dawn-post-office/ with its own game.toml, authored .vn narrative, theme(s), asset manifest, README, artwork/audio directories and runnable Linux launcher; no game-specific Rust branches. |

## What players will control

Players now have an engine-owned **Settings** screen (option 4 from pause and main menus). Its first-class controls include dialogue-box **opacity (60–100%)**, **text size (80–160% of the game theme)**, **font selection** among approved/installed choices, and **Reset to game defaults**. Changes have a live preview and persist to local preferences, not game source files. Later stages add **text reveal speed, instant text, reduced motion and high contrast**.

There are two distinct authorities: game creators supply a default external TOML theme, and players may override presentation details locally. Theme hot reload still works for fields the player hasn't overridden. If a requested font disappears, the engine uses a readable fallback. Do not hide validation errors or corrupt story saves.

## Presentation quality to build

**Story UI:** real choice-option widgets rather than a giant concatenated Text node; speaker nameplate, themeable panel geometry, pointer/keyboard focus, clear selected state, an unobtrusive continue cue, typographic rhythm and safe wrapping.

**Scene language:** reusable authored fade/crossfade/soft slide, optional slow pan/zoom, art anchors/depth, expression crossfades, entrance and gentle idle motion, lightweight motes/glow/vignette, and coherent focus/confirmation cues. Every effect must be skippable/disableable and behave correctly with pause, save/load and hot reload. Avoid flashy effects that reduce legibility.

**Reading flow:** deliberate text reveal, auto-advance only with opt-in, bounded dialogue history, browseable save/load slots, and keyboard plus mouse navigation. Keep F5/F9 for quick save/load.

## Final showcase: *The Post Office at the Edge of Dawn*

A tiny moth courier is given a letter addressed to a dawn that doesn't exist yet. In a whimsical airborne post office, she visits sky gardens, a teacup observatory and drifting cloud neighborhoods to discover whom the message is really for. Multiple decisions lead to at least **three endings**, using conditionals and meaningfully different scenes. Expect about **12–20 minutes** of reading, with **at least five illustrated locations, three characters, and nine expression images**.

This must be a **real complete game**, not a feature-menu or static mockup. The work at **VX10** includes generating original, visually consistent backgrounds and transparent character art, creating/licensing permitted fonts, composing or procedurally synthesizing original audio, authoring external `.vn` and TOML files, packaging, and visually reviewing rendered story branches. All generation happens during production, not as a runtime service. The engine remains generic.

## Acceptance protocol

Each milestone receives its **entire weight only after verified evidence**. Keep the JSON ledger current with commit SHA, passing run, useful screenshots and any remaining limitation. A completed implementation with red CI is still pending. Visual tests include the previously troublesome **946×511** and **955×520** Wayland window sizes in addition to **800×600**, **900×700**, **1280×720**, and **1920×1080**.

The final game milestone is deliberately **last** and depends on the earlier usability, presentation and regression work. No new mandatory engine scope may be invented merely because development can continue. When all tasks pass, stop.

## Implementation boundaries

Rust + Bevy 0.20, external game directories and TOML themes, free/offline-capable Linux-first workflow. No npm, no paid API, no Mac requirement, no runtime AI, no WYSIWYG editor, no full 3D/genre expansion, and no heavyweight render-thread work. Preserve the original example games and saves. See [the machine-readable source of truth](visual-experience-roadmap.json) for dependencies and testable criteria.
