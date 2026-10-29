# VX09 — Accessibility and Linux experience certification

**Scope:** Final non-showcase accessibility and regression gate for the Visual Experience Roadmap. The milestone remains pending until the *current PR head* passes both CI jobs and the evidence is added to the durable roadmap. The original MVP and prior presentation polish phases remain independently complete.

## Readable defaults and player controls

The player maintains separate authored styling and optional *player-owned* preferences. In **P**ause or **M**ain menu, choose **Appearance settings** and use Up/Down to focus rows, Left/Right or Enter to change a control, and Escape to close without exiting. Real Bevy buttons support pointer activation. The existing persistent controls include opacity (60–100%), font size scaling (80–160%), generic font fallback, Instant/Slow/Normal/Fast reveal, reduced motion, and auto-advance opt-in. VX09 adds **High contrast**, which overrides the game panel with fully opaque dark navy and all four text styles with opaque white; author TOML remains unchanged.

The high-contrast base panel is `[0.015, 0.020, 0.035, 1.0]` against white text. Their nominal WCAG relative-luminance contrast is **20.37:1** before compositing; rendered screenshots were inspected separately because screenshots and antialiasing can differ from color-token calculations. The preference's optional JSON field is backward compatible with existing format-version-1 preference files and survives atomic save, restart and Reset. Unavailable player font overrides fall back to the author's font or system family rather than crashing. Reduced motion disables optional moving particles, background and character motion, while immediately completing text reveal.

## Actual rendered glyph bounds

Under Linux CI's **Xvfb / X11 software renderer**, the six requested window dimensions are tested by four independent launches of a temporary copied fairy-tale fixture at each size. The normal image, no-body, no-choice, and no-hint renders produce isolated difference masks. The check uses ImageMagick's non-background bounding box `%@` **without resetting page origin**, so reported Y extents refer to actual screenshot coordinates. A minimum 5-pixel vertical gap and nonempty glyph masks are required for both dialogue–hint and choice–hint regions. This avoids the earlier false negative where letters occupied the same line area without sharing the same white pixels.

| Window (logical pixels) | Dialogue/hint gap | Choice/hint glyph gap | Pixel collision |
| --- | ---: | ---: | ---: |
| 946×511 | 13 px | 82 px | 0 |
| 955×520 | 12 px | 82 px | 0 |
| 800×600 | 6 px | 62 px | 0 |
| 900×700 | 12 px | 78 px | 0 |
| 1280×720 | 43 px | 157 px | 0 |
| 1920×1080 | 115 px | 325 px | 0 |

The values above were captured in **CI run 40791505152**, with its `vesper-hint-overlap` artifact (ID `12862037969`). The glyph masks and settings images from both external games were also visually inspected. The narrowest-window presentation is compact but the body, selectable choices and hint remain visually distinct. The test selects the ninth of nine lengthy options and confirms it is independently scrollable.

## Full functional regression coverage

The full CI pipeline covers Rust narrative validation, Bevy build/unit tests, all VX01–VX08 graphical suites, keyboard and pointer selection/focus, live Settings, preferences persistence and Reset, pause, history, manual slots with overwrite confirmation and F5/F9 quick saves, interrupted cinematic/character transitions, atmosphere on/off and reduced motion, both external fairy-tale playthroughs, authored fonts/themes/art hot reload, external audio decoding, version-1 save compatibility and Linux packaging that does not require a Rust compiler on the destination system.

## Render-thread and resource-bound review

This source audit is distinct from a runtime profiler:

- **Game and save I/O:** atomic player preferences, save-slot scans, quick/manual save/load and watch-directory scanning run through workers. No new synchronous file operations were introduced into the Bevy update/render loops.
- **Atmosphere:** at most 32 ECS sprite entities across all presets. The 128×128 (64 KiB RGBA) feathered glow texture is generated once at startup, shared by glow layers, and not recreated per frame; theme reload rebuilds only on a changed preset.
- **History and saves:** current-session history holds at most 80 entries with 2048 Unicode graphemes per field; only three manual slots and one legacy quick slot exist. History does not scan future branches or enter the durable save schema.
- **Layout:** VX09 removes a full `Theme` clone/resolve from the per-frame viewport system, borrowing unchanged author-owned panel geometry directly. Existing geometry and user override semantics are preserved. Character transition scratch storage is reused across frames with a Bevy `Local<Vec<Entity>>`, rather than allocating a new vector every frame.
- **Animation:** motes and actors have bounded update clocks, normalized positions, and reusable ECS entities. Choice rows rebuild only when the story's actual options change. The audit found no mandatory render-thread image generation, file parsing, or unlimited per-frame spawning path.

The last two layout/scratch optimizations were committed **after** run `40791505152`; their exact head requires a separate green full CI run before final VX09 certification.

## What CI does *not* certify

Xvfb is an X11 virtual display, **not** a native Wayland compositor. These results validate six requested logical sizes and software-rendered visual behavior, but do not prove compositor fractional scaling, monitor DPI, touch behavior, specific GPU drivers, actual audio device routing, or latency/power on a user's physical Linux machine. Free/Linux-only tools and compiler-free package tests remain intact. Physical Wayland, audio hardware and input-device compatibility need separate real-hardware evidence if later requested. No unsupported blanket accessibility/WCAG conformance claim is made.

## Evidence links

- [Latest pre-audit full green CI (40791505152)](https://github.com/sguzman/vesper/actions/runs/40791505152)
- [Visual Experience Roadmap](visual-experience-roadmap.md)
- [Player-owned preferences](../crates/game/src/preferences.rs)
- [Six-size glyph-mask regression](../.github/scripts/hint_overlap_smoke.py)
- [Live settings regression](../.github/scripts/settings_smoke.py)

Only after the final head's full CI passes should VX09's eight points be awarded, and only after merge should `main` display **82/100** verified.
