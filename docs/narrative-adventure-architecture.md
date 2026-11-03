# NA01 — Optional exploration and dialogue-surface architecture

**Status:** Decision contract for the new narrative-adventure phase. This document specifies interfaces to implement, **not** a claim that exploration or speech bubbles are already rendered. See the [separate 100-point ledger](narrative-adventure-roadmap.json).

## Existing engine boundary (verified before the extension)

- The `vesper-story` Rust crate owns deterministic `.vn` parsing, script choices, variables and machine snapshots. A scene currently yields dialogue, choices, visual instructions or the ending; it never polls the world or the render frame.
- The `vesper-game` crate owns game-local `game.toml`, theme and asset path validation, as well as the existing backwards-compatible version-1 save wrapper.
- The Bevy `vesper-player` crate owns ECS entities, camera, reading state, modal menus, text layout, artwork, audio and input. The existing author-specified `panel <skin>` command controls **art on a traditional dialogue panel**, not whether a bubble or a panel is used.
- Pure visual novels already launch directly into authored narrative scenes. Old games are not required to declare locations, input mappings, avatar sprites, bubble art or additional fields.

This is an **additive** extension. The existing version-1 script grammar and panel art semantics must continue to work. Use responsibility-oriented modules (not mixed-purpose large files), and keep the deterministic story core free of Bevy dependencies.

## Two independent axes

**Experience mode**, owned by an engine state machine:

| Mode | Owner | Authoritative position | Dialogue behavior |
| --- | --- | --- | --- |
| `visual_novel` | existing story machine and player | staged character slots / backgrounds | standard VN dialogue, no movement input |
| `exploring` | bounded walk-first exploration controller | current named external location + logical X + facing | movement and interaction prompts, no automatic dialogue advance |
| `conversation` | story machine while exploration is suspended | an explicit saved return point | modal dialogue/choices; player movement disabled until explicit outcome |

**Dialogue surface**, separate from mode: `panel` (default; existing artwork and speaker nameplate) or `bubble` (anchored text and optional artist-supplied bubble frame). An ordinary VN may use a bubble; an exploration conversation may use the full illustrated panel; either may switch within one conversation.

`crates/game/src/experience.rs` provides the pure typed model and transition guards. The walk-first controller, native bubbles, and first NPC/prop interaction
bridge now use this model. E-only named exits and typed variable-gated
NPC/prop availability and typed VN-to-location transitions are implemented.
Version-two world persistence and live authored hot reload are implemented
and passed full merged-main CI 41696094392; NA07 formal weighted acceptance
remains dependency-gated behind the unverified NA02/NA03/NA05/NA06 milestones.

## Explicit transitions and invariants

- **VN → exploration:** the author names a validated location and spawn point. The player loads its bounded world and changes to exploration. The story interpreter must yield rather than executing an unbounded loop.
- **Exploration → conversation:** a focused, reachable NPC/object links to an authored `.vn` scene. Interact snapshots its world return point (location, logical X and facing), freezes locomotion, and enters a modal conversation. The narrative machine retains story variables and state.
- **Conversation → exploration:** an explicit author-directed return resumes that **exact** world location and position. Repeating an interaction doesn't double-fire a choice or move the protagonist.
- **Conversation → cinematic VN:** an explicit authored transfer drops the pending world return; the story enters ordinary VN staging, including any `panel` or `bubble` selection. No automatic walk back is imposed.
- **VN → VN:** unchanged. If no exploration instructions or manifest sections are present, legacy games never construct exploration controllers, walkable scenes or interaction prompts.
- **Presentation changes:** switching the surface **never** triggers an experience-mode change, story branch, camera move, save or relocation. The author controls dialogue surface independently; the reader's font size, text speed, accessibility and high-contrast preferences retain precedence.
- A live menu/pause/settings overlay always suspends movement and consumes input first; focus loss suspends walking; no double activation through press and release. A headless story machine cannot observe camera positions, pointer events, or the frame clock.
- No silent fallback that changes story progression. Invalid IDs, missing assets, bad location bounds and out-of-range spawn positions receive game-file/line diagnostics and reject the invalid load/hot reload while preserving the last valid world.

## External authoring contract to implement

**New authoring is additive and versioned.** The optional `[dialogue]`
surface authoring, `[exploration]` location data, and the opt-in
`[exploration.start]` walk-first preview are now implemented; the
`explore return`, `mode visual_novel` and
`explore <location> at <spawn>` commands are implemented. The latter
validates locations and named spawns before starting the player.
Existing `format_version = 1` manifests remain valid. Use an optional isolated `[exploration]` subsection with its own `schema_version = 1` and referenced locations; validate that optional section only when present. Preserve the existing game-relative secure path policy.

Example optional authoring shape (supported; individual example assets must exist):

```toml
[dialogue]
default_surface = "panel"   # omitted means the existing dialogue box

[bubble_skins.default]
# Optional PNG frame and tail artwork; fallback is a native UI color panel.
# All safe areas, colors, offsets and text sizes must be bounded and validated.
image = "assets/ui/bubble.png"

[exploration]
schema_version = 1
[exploration.start]
location = "street"
spawn = "cafe"

[exploration.locations.street]
background = "assets/streets/street.png"
width = 1920.0
height = 720.0

[exploration.locations.street.camera]
zoom = 1.0
horizon = 0.5
follow_x = true

[exploration.locations.street.spawns.cafe]
x = 640.0
facing = "right"

[exploration.locations.street.npcs.friend]
x = 810.0
interact_scene = "friend_talk"
interaction_radius = 80.0
```

Authoring policy: IDs use the existing constrained scene/asset-identifier family; world coordinates are finite, nonnegative logical pixels with validated viewport/location bounds; paths are game-local. Backgrounds use aspect-preserving display rules. NPC talk targets must exist in the parsed `.vn` story; aliases, scene IDs, spawn IDs, and location IDs are distinct namespaces. The initial controller is **side-on horizontal walking only**; maintain an isolated viewport/locomotion interface so future view types need not rewrite narrative logic.

**Additive `.vn` commands** (explicit, typed, and validated by the story parser and external game loader):

```text
@scene intro
surface bubble
friend: "You're early."
surface panel
friend: "There's something we should talk about."
surface default
explore street at cafe

@scene friend_talk
surface bubble
friend: "Want some coffee?"
* "Sure" -> coffee_yes
* "Later" -> coffee_later

@scene coffee_yes
set friendship += 1
explore return

@scene important
mode visual_novel
panel ornate
friend: "Let's talk somewhere quiet."
```

- `surface panel` selects the existing dialogue box; `surface bubble` selects an anchored bubble; `surface default` restores the author default (and the legacy box if no new section exists). The switch can occur immediately before **any line** and persists until the next explicit switch. Authors may explicitly set the surface at a scene's first command; there is no implicit per-scene reset. This permits per-line and per-scene authoring without embedded markup.
- `explore <location> at <spawn>` explicitly yields into exploration.
- `explore return` is valid only while a modal world conversation is pending; it returns to its snapped location/position instead of automatically walking there.
- `mode visual_novel` explicitly transfers into cinematic staging from any mode and clears the pending exploration return.
- Parser and runtime must add typed commands instead of interpreting TOML or source strings as executable code, while preserving file/line diagnostics. Cross-reference validation must reject missing scenes or locations before starting play.

**Still pending acceptance:** comprehensive bubble/choice geometry and
multi-NPC occlusion across supported sizes, plus full hot-reload and
save-compatibility acceptance. These remain NA02/NA04/NA06–08 scope.

## Persistence and hot reload

Legacy `SaveData` version 1 continues to encode the narrative snapshot plus
`PresentationState` for games with no exploration. Version 2 adds an optional
`ExperienceState` with a typed mode, stable location ID, bounded world X,
avatar facing, and a modal conversation return point. The same `MachineSnapshot`
retains variables and authored dialogue state. Old version-one saves load
without migration; invalid world locations, coordinates, missing scenes, or
inconsistent save versions are rejected before replacing the live session.
No ECS IDs, texture handles or frame-time state are serialized.

Atomic writes and background-worker I/O are retained. Hot reload rejects
edits incompatible with the current authored world position and rebuilds only
exploration-owned sprites after an accepted manifest change. Native hot-reload and cold-process walking, modal conversation and VN-first
restore acceptance passed CI 41696094392. Final human UI quality acceptance
remains a separate NA08/UR04 gate.

## Verification matrix and completion gate

1. **Contract and Rust state model:** normal VN+panel default, bubbles while still in pure VN, modal conversation→exact return, explicit VN exit, invalid location and nested interaction rejected, JSON round trip.
2. **Legacy compatibility:** current CLI validates all existing external game manifests unchanged; parser, game and player unit tests pass, existing menu/Atlas graphical regressions and saves survive.
3. **Future independent fixtures:** (a) pure VN with bubble switching and existing choice panel, (b) one single-screen street with interaction, (c) mixed-mode game with persistent world return. Reuse one compiled player; no hardcoded example logic.
4. **Graphical QA:** capture real Bevy frames at 946×511 and 1280×720 and wide resolution; nameplate and bubble legibility, occlusion/fallback, focus, movement freeze, frame transition, reduced motion and high contrast.
5. **Final manual acceptance:** physical Linux/Hyprland UI quality and actual response latency remain separate from green virtual-display CI.

**NA01 earns its 6 weight only when** this adopted decision is committed, the typed state contract has passing tests and all relevant legacy validation plus full CI are green on the tested code. **NA02 and later earn nothing** merely from this design or the Rust model.
