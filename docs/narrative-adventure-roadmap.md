# Vesper — Optional Narrative Adventure Roadmap

**Status:** **0 / 100 verified points** (active development; completed
acceptance gates alone earn weighted points). The earlier Vesper MVP, visual experience and human-approved Atlas presentation remain independently complete; **do not reset their ledgers**.

**Goal:** Extend the reusable Rust/Bevy visual novel engine with optional compact 2D exploration and **an independent choice of dialogue surfaces**: traditional illustrated boxes with authored image and nameplate, anchored speech bubbles, or both within a single story. Then ship an original grounded anime-urban slice-of-life example. The game brief is [separate](urban-slice-of-life-brief.md), so the engine remains genre-neutral.

## Non-negotiable product model

**Three valid authored experiences:** (1) pure visual novel with no movement, (2) exploration-led narrative adventure using existing deterministic dialogue, (3) a game that transitions freely between these two. **Dialogue style is orthogonal**: a pure VN may use a speech bubble, and an explorable scene may use the illustrated VN panel or change styles per spoken line.

- **VN panels:** retain existing externally authored artwork, safe writing region, character-name plaque and player settings; no regression to Atlas or older games.
- **Bubbles:** native Bevy UI text with an optional author-provided bubble/frame image, tail pointing to the speaker, wrapping, reveal pacing, choice focus, readable contrast and resize/reduced-motion fallbacks. Supported for static VN portraits *and* moving NPCs.
- **Exploration:** deliberate 2D horizontal motion, constrained authored routes, approachable NPCs and props, side-scrolling camera only when needed. The goal is *presence in the world*, not platforming.
- **Story:** the current .vn interpreter remains the sole deterministic authority for dialogue, choices and variables. World interactions route to authored narrative entry points and return cleanly to exploration by an explicit authored transition.
- **No busywork:** short distances, meaningful destinations, optional navigation shortcuts/quick travel, no forced errands, wandering or repeated treks. No jumping, combat, inventory economy, general physics, rhythm game or giant open-world engine in this phase.
- **Ownership:** games fully external: manifests, maps/locations, characters, sprites, animations, conversation scripts, music, custom panels and bubbles; no example names/asset paths in Rust, no runtime AI or remote service.
- **Backward compatibility:** all current VN-only examples and old saves keep working without new config fields. Use versioned migration for any added save data. Old-style reader does not gain an unwanted avatar, bubble or exploration control.

### Intended presentation flow

```text
Pure VN ────────── [illustrated dialogue panel or speech bubble] ──> next VN scene
                          ↑                            ↑
Explorable area ── interact with NPC/object ──> scripted talk/choice
      ↑                                                   │
      └──── explicit return to area/position/state ───────┘

Either mode may transition to the other through authored narrative commands.
```

**Mode** (VN versus explore) and **surface** (panel versus bubble) are *separate settings*, not one combined type. A scene may override the game's default; a line may override the scene; absent authoring keeps the legacy VN box. Player accessibility controls take precedence over decorative styling.

**Architecture approach:** specify and validate a bounded external location schema (world/area bounds, visual layers, spawn points, avatar, NPCs, props, trigger actions and links); use a small engine-owned locomotion/controller ECS module and a separate speech-bubble UI module; let the story machine issue/receive typed deterministic entry/return events. All format syntax is **design-stage**, not a promised final grammar. Keep meaningful responsibility-based modules, never a god file; there are **no arbitrary line-count limits**.

## Weighted delivery plan

The ledger [narrative-adventure-roadmap.json](narrative-adventure-roadmap.json) is the source of truth for dependencies, exact evidence and verification status. The [NA01 architecture and authoring contract](narrative-adventure-architecture.md) defines the two independent modes, explicit transitions and backwards-compatibility requirements. The separate [NA04 exploration schema](exploration-schema.md) specifies the game-relative bounded location/NPC/exit format (validation only until movement work begins). **Engine: 60 points; example: 40 points.** Planning creates no verified points.

| ID | Points | Milestone | Required outcome |
| --- | ---: | --- | --- |
| NA01 | 6 | 🧭 Mode architecture and authoring contract | Versioned mode/surface contract, explicit dialogue↔explore transitions, legacy tests |
| NA02 | 10 | 💬 Native speech-bubble renderer | Anchored, responsive, legible and themeable, in ordinary VN scenes too |
| NA03 | 6 | 🎭 Dialogue-surface authoring | Game/scene/line default/override for bubbles and existing illustrated panels |
| NA04 | 7 | 🏙️ External exploration schema | Validated game-local locations, interactive NPCs/objects, exits and asset paths |
| NA05 | 8 | 🚶 Simple 2D walking and camera | Smooth input, idle/walk sprites, bounded camera and modal pause |
| NA06 | 10 | 🤝 Interaction-to-story bridge | NPC/object → authored talk/choice → correct return to location |
| NA07 | 7 | 💾 Save, load and safe hot reload | Round-trip world/narrative state, old saves still load, no blocking I/O |
| NA08 | 6 | ♿ Accessibility and regression | No-busywork controls, screen-size tests, pure-VN and mixed-mode CI |
| UR01 | 5 | 📝 World, story and art bible | Owner-approved original cast, location/story structure, art direction |
| UR02 | 9 | 🎨 Original art and audio | Coherent urban anime environments, sprite cycles, expressions and audio |
| UR03 | 15 | 🌇 Complete slice-of-life example | Original playable branching story demonstrating both presentation modes |
| UR04 | 11 | 🎬 Full QA and packaging | Branch playthroughs, responsive visual evidence, green CI, physical user QA |

## Execution sequence and stop/go gates

1. **Contract first:** NA01 establishes engine/external-authoring boundary, event semantics, and legacy compatibility. An early bubble-only fixture can start while location data is planned.
2. **Speech bubbles independent of movement:** NA02–NA03 first prove a *pure VN* can switch between anchored bubbles and authored panels with correct keyboard controls, custom nameplate and player preferences.
3. **Minimal exploration vertical slice:** NA04–NA06 author a small one-screen street with avatar + one NPC. Interact, finish the .vn conversation, and return to the same position; then add a second area, bounded scrolling and conditional dialogue.
4. **Persistence and comfort:** NA07–NA08 verify replay, save/load/hot reload, accessibility, quick navigation, stable presentation, small-window/fullscreen and performance. Old games and their output remain intact.
5. **Parallel creative preproduction:** UR01 can proceed once owner chooses story specifics. UR02–UR03 build original art/story on the generic engine, not hardcoded exceptions. UR04 is final full-game testing and acceptance.

**First coded vertical slice acceptance:** an unchanged Vesper binary launches a tiny author-provided game with one explorable street, a controlled walker and an NPC; dialogue opens in a bubble, switches to the existing custom panel with nameplate, makes a meaningful choice, and returns to the street. A second pure-VN fixture switches from panel to bubble without movement. Save/load and old-VN compatibility must pass before expansion to the full example.

**Quality bar:** prioritize responsiveness, native presentation quality and legibility. A passing Rust compile is not a passing screenshot test. The verification gate includes CI source/run IDs, real screenshots, bounded performance checks and owner inspection on native Hyprland. Only complete acceptance earns milestone points.

## Scope boundaries

- Retain **Bevy 0.20**, Rust stable and the current Ink-inspired .vn format as the base. New syntax is additive and externally authored, with a clear migration/compatibility plan.
- No game-specific engine branches, developer-only scripts as user prerequisites, external vector-art renderers at runtime, npm, Lua or full engine rewrite.
- No replacement of the completed Atlas visual presentation. New features use separate engine modules and new external example data.
- Keep the future top-down/isometric possibility architectural (mode/controls/maps not hardwired to one view), but implement only **simple side-on 2D** here; don't claim unbuilt camera modes.
- No final cast, city, title or fixed full-story duration is decided until the user's creative brief. The example brief contains **proposals**, not fixed story canon.
- All new tasks start **not started**, verified **0/100**, delta **+0 pp**. Prior project milestone percentages do not change.
