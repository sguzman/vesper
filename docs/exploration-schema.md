# NA04 — Optional external explorable-location data

**Implementation status:** Versioned parsing, validation, walking, NPC/prop
dialogue, typed story-variable visibility conditions, and E-triggered authored
exits are implemented. This is not the final narrative adventure:
VN-to-location story commands, exploration persistence and safe reload remain
unfinished. This is not the original anime urban slice-of-life game.

## Separation from visual novels

The `format_version = 1` game manifest is unchanged. Games without
`[exploration]` remain pure visual novels: no location fields, world or
avatar assets, walking behavior or interaction prompts are required. The
optional `[exploration]` table has its **own** `schema_version = 1`.
All authored content remains inside the external game directory.

A minimal example:

```toml
[exploration]
schema_version = 1

# Optional, explicit walk-first opening; omit for an ordinary VN.
[exploration.start]
location = "street"
spawn = "home"

[exploration.avatar]
speed = 230.0 # logical px / second; bounded walking speed
# idle = "assets/avatar/idle.png"
# walk = ["assets/avatar/walk-1.png", "assets/avatar/walk-2.png"]
# walk_fps = 8.0

[exploration.locations.street]
background = "assets/backgrounds/street.png"
width = 1280.0
height = 720.0

[exploration.locations.street.camera]
zoom = 1.0
horizon = 0.55
follow_x = true

[exploration.locations.street.spawns.home]
x = 160.0
facing = "right"

[exploration.locations.street.npcs.neighbor]
x = 425.0
interact_scene = "neighbor_chat"
interaction_radius = 80.0
# Optional (absent means always visible):
# visible_if = { variable = "met_neighbor", equals = true }

[exploration.locations.street.props.poster]
x = 720.0
interact_scene = "read_poster"

[exploration.locations.street.exits.cafe]
x = 1170.0
to_location = "coffee_shop"
to_spawn = "front"

[exploration.locations.coffee_shop]
background = "assets/backgrounds/cafe.png"
width = 640.0
height = 720.0

[exploration.locations.coffee_shop.spawns.front]
x = 48.0
facing = "left"
```

`[exploration.locations.<id>.layers.<name>]` optionally supplies
`image = "assets/foreground.png"`, `parallax = 1.0` and `depth = 0.0`
to prepare for future parallax renderer composition. NPC and prop `image`
fields are optional valid local PNG references; they may be absent while
the author is building the map. Locations can fit within one screen: the
initial implementation is intentionally a **bounded side-on walk**, not
a general 2D/3D physics engine.

## Schema validation contract

`vesper-game::Game::load` parses and validates these values **before**
running the Bevy player. Failed hot reload should preserve the prior validated
game. Errors include the game manifest path and precise exploration field.

- Up to 24 locations with logical dimensions width 320–8192 and height
  360–2160. Every location needs a PNG background and 1–64 named spawns.
  Optional `[exploration.locations.<id>.camera]` provides bounded
  `zoom = 1.0` (0.5–2.0), `horizon = 0.5` (0–1) and `follow_x = true`;
  omission preserves these defaults. Single-screen locations need no camera
  scrolling.
- All spawn, NPC, prop and exit X positions are finite, nonnegative and within
  the authored location width. Facing is `left` or `right`.
- Up to 32 combined NPC/prop/exit interactions and 12 visual layers per area.
  An interaction radius must be a finite 20–200 logical pixels.
- Every NPC's `interact_scene` must name an existing authored `.vn` scene.
  Props may omit `interact_scene`. An exit's destination location and named
  spawn must both exist, with no implicit teleport to an unknown ID.
- NPCs and props may declare `visible_if = { variable = "met_neighbor",
  equals = true }`. The expected value must be a boolean, integer or
  quoted string, and the variable identifier is validated. Missing
  variables and mismatched types make the actor unavailable, not an error.
- Images are game-local, checked for path traversal/symlinks and require
  `.png` extension. Maximum 16 optional walk animation frames, with avatar
  speeds, scale, frame rate, parallax and depth constrained to bounded ranges.
- Duplicate TOML keys, unknown schema fields, non-finite coordinates, missing
  scene targets, bad destination names and missing images fail authoring
  validation. For now, passable 1D paths are assumed because obstacles/collision
  are explicitly outside the initial phase.

**Named namespaces** are separate for location, spawn, NPC, prop and exit.
The same spelling in two distinct namespaces is not an engine collision.

## Ownership / transition boundary

The contract deliberately **does not** add `explore ...` commands to the
existing story parser yet. A game with validated `[exploration.start]`
opens directly into a bounded side-on world instead of advancing the hidden
VN script. Author-controlled keyboard walking, viewport projection, optional
PNG sprites and simple stand-ins run in the isolated player module.
If `start` is missing, the player remains a traditional VN; merely declaring
locations never starts walking.

**Current preview scope:** Left/Right or A/D walking with a capped frame step,
camera following only when necessary, optional walk/idle frame animation,
and pause/menu/focus input ownership. Background PNGs retain aspect through
uniform cover scaling. The completed VN UI and story camera remain separate.
NPCs and props respond to **E** inside their declared interaction radii,
using nearest-distance selection and deterministic ID tie-breaks. A visible
keyboard focus cue names the target. The original .vn interpreter provides
variables, choices, history, and `surface` switches without moving the
avatar. `explore return` resumes the exact prior X/facing/location;
`mode visual_novel` explicitly drops the return. **Q** cancels modal
conversation without selecting an option; already-run story assignments
are not rolled back. World art stays visible under modal dialogue.
Bubbles may anchor to an authored NPC ID.

Authored exits appear in the same nearest-focus cue as NPCs and props.
**E** transfers only within the exit's author-defined interaction radius,
instantly to its validated named destination spawn and facing. There is no
forced backtracking or automatic walking. Changing area despawns only
exploration-owned sprites (background, NPCs, props, avatar and layers) before
building the new area; the VN UI and menu entities are preserved. NPCs/props
can become visible or disappear on story-variable changes without world
rebuilding or a second quest system.

F5/F9/F6 saves remain unavailable in exploration and modal conversation
until NA07 persists complete world state; no incomplete VN-only save is written.

The `examples/exploration-schema` walk-first fixture uses reused test
background art. It must not be reported as the original playable urban
slice-of-life example.

## Test coverage

- Pure Rust unit tests in `crates/game/src/exploration.rs`: valid two-location
  world, NPC+prop+exit and sprite defaults, invalid dimensions, missing scenes
  and target spawns, bad radius/coordinate, path traversal and unknown fields.
- `cargo run --locked -p vesper-cli -- check-game examples/exploration-schema`
  validates a complete externally authored package in CI.
- Pure locomotion and camera tests under `crates/game/src/walk.rs` enforce
  frame-time caps, level bounds, single-screen framing and resize independence.
- The native `.github/scripts/exploration_walk_smoke.py` drives actual
  Bevy movement, NPC conversation, cancellation, authored return, conditional
  friend, street/café roundtrip, old-area entity removal, pause, sprite
  rendering and compact-window resize, archiving graphical frames.
- Existing VN-only games and all original `.vn` commands remain unchanged and
  continue running under the regular CLI/Bevy regressions.

The NA04 milestone remains **active**, not verified, until integration and all
specified acceptance checks are green on the committed code.
