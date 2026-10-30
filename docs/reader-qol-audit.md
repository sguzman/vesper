# Reader QoL QOL08 — Render-loop and regression audit

This audit is scoped to the five Reader QoL capabilities and describes the
*committed implementation*, not a general guarantee about every Bevy system.
The source-of-truth weighted roadmap is [qol-roadmap.json](qol-roadmap.json).
Certified source for this phase: merged main `84d0ddf6b1c9a7d0c07fc19d4ff5292b655def0c` ([final CI](https://github.com/sguzman/vesper/actions/runs/40974030361)).

## Renderer safety rule

**Never run heavy work on the render/UI thread.** The authoring inputs remain
outside Rust; the engine validates/loading them outside each displayed frame.

| Concern | Current implementation | Bounded or deferred work |
| --- | --- | --- |
| System font discovery | `font_browser::discover` in `crates/player/src/font_browser.rs` runs at startup using Bevy's font collection, caches a sorted bounded family list for lookup | No OS font enumeration in its Update handlers; user-selected family resolves from a cached catalog |
| Auto-advance | Reader-only optional delay in `crates/game/src/preferences.rs`; `readout::tick` advances a bounded clock | Valid delay 0.5–20 s, independent of authored game; player preferences persist through asynchronous writes |
| Notebook/banner | `Game::load` validates source-relative PNG and safe nine-slice geometry; `panel_art::sync` maintains UI sprite handles | No synchronous image read or external path canonicalization inside the frame loop; high contrast disables decorative panel art |
| Styled words | `crates/story/src/rich_text.rs` parses markup at load; `readout::sync_spans` builds child text spans on line changes | Maximum 64 authored spans and 32 KiB per source line; no markup parser on each rendered frame |
| Dynamic flair | `readout::animate_inline_effects` changes only `TextColor` of existing spans based on bounded time/parameters | Maximum 32 external inline styles; amplitude 0–0.35, frequency 0–6 Hz; no reflow, glyph movement, image decode or file I/O |
| Ambient sprites | Manifest validation in `crates/game/src/lib.rs`; `ambient::sync` reconciles changes and `ambient::tick_motion` samples a pure path | Maximum 32 actors, 2–16 waypoints per motion, maximum 16 flipbook frames; image handles are cached and reused. No physics/pathfinder or filesystem scans per frame |
| Saves and hot reload | Save/restore uses dedicated `IoWorkers`; source reload verifies authored files on a worker before swapping active game state | Only bounded ECS snapshots are gathered on a save request; invalid edited files do not replace live game state |

**Qualifiers.** The ambient reconciliation system may allocate its bounded
temporary actor-ID set when a resize, restored save, or game-state change
requests synchronization. That work is not per-frame disk or image decoding;
it is not a formal allocation-free guarantee. Ordinary Bevy UI shaping,
asset scheduling and GPU driver work still occur in the engine, so this audit
does not claim the entire runtime allocates nothing or never hitches.

## Behavioral and compatibility certification

- **QOL01/QOL02:** installed font picker, cached search and persisted restart;
  author theme vs player font precedence; optional auto-advance delay with bounds.
- **QOL03/QOL04:** six-size art safe regions and word-span layout; panel and
  typography F5/F9 persistence; Unicode grapheme-safe reveal and high contrast.
- **QOL05:** dynamic ember/shadow/shimmer word colors, high-contrast suppression
  and reduced-motion frozen colors, screenshot pixel differences.
- **QOL06/QOL07:** static + moving ambient actors, show/hide and scene scope,
  ping-pong walk/looping bird/one-shot plane; paused/reduced-motion freeze,
  mid-flight F5/F9 clock restoration and live authoring rejection/refresh.
- **QOL08:** [The Living Journal](../examples/living-journal/) combines all
  five requested capabilities into an external game with no game-specific
  renderer. The dedicated `.github/scripts/integrated_authoring_smoke.py`
  passed six viewport sizes and reader-preference overrides on merged main.
  Both narrative and Bevy jobs passed in [run 40974030361](https://github.com/sguzman/vesper/actions/runs/40974030361);
  screenshot evidence is archived as `vesper-integrated-qa` (artifact 12941227870).

## Limits and physical QA

GitHub Actions uses Linux Xvfb software rendering. Passing screenshots
establish visible native Bevy behavior on those virtual displays, **not**
exclusive fullscreen, exact installed-font availability, GPU frame pacing,
actual speaker/audio output, or compositor behavior on the user's
EndeavourOS/Hyprland/Wayland installation. Physical Wayland inspection remains
a separate optional dogfooding test and is not asserted as already performed.

This audit does not promise collision, skeletal rigs, pathfinding, dynamic
raster generated art, 3D scenery, per-glyph geometric wave distortion, or an
authoring GUI. QOL05's shimmer is deliberately color-based. Source files,
player preferences, and version-one save snapshots remain separate concerns.
