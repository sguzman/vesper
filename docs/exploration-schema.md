# NA04 — Optional external explorable-location data

**Implementation status:** Versioned parsing, validation and standalone
authoring fixture are implemented. There is **no player movement or world
rendering yet**; those belong to NA05 and NA06. Do not confuse this with a
playable map or the forthcoming original anime urban example.

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

[exploration.avatar]
speed = 230.0 # logical px / second; future movement speed
# idle = "assets/avatar/idle.png"
# walk = ["assets/avatar/walk-1.png", "assets/avatar/walk-2.png"]
# walk_fps = 8.0

[exploration.locations.street]
background = "assets/backgrounds/street.png"
width = 1280.0
height = 720.0

[exploration.locations.street.spawns.home]
x = 160.0
facing = "right"

[exploration.locations.street.npcs.neighbor]
x = 425.0
interact_scene = "neighbor_chat"
interaction_radius = 80.0

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
  360–2160. Every location needs a PNG background and at least one spawn.
- All spawn, NPC, prop and exit X positions are finite, nonnegative and within
  the authored location width. Facing is `left` or `right`.
- Up to 32 combined NPC/prop/exit interactions and 12 visual layers per area.
  An interaction radius must be a finite 20–200 logical pixels.
- Every NPC's `interact_scene` must name an existing authored `.vn` scene.
  Props may omit `interact_scene`. An exit's destination location and named
  spawn must both exist, with no implicit teleport to an unknown ID.
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
existing story parser yet. NA05 will consume a validated location with a
bounded avatar/camera controller; NA06 will connect interaction to the
deterministic `.vn` interpreter, including explicit return-to-position
events. NA07 will persist the logical area ID and position, never Bevy
entity handles.

The fixture `examples/exploration-schema` is authoring-only and uses reused
test background art. It must not be reported as the actual playable
slice-of-life example.

## Test coverage

- Pure Rust unit tests in `crates/game/src/exploration.rs`: valid two-location
  world, NPC+prop+exit and sprite defaults, invalid dimensions, missing scenes
  and target spawns, bad radius/coordinate, path traversal and unknown fields.
- `cargo run --locked -p vesper-cli -- check-game examples/exploration-schema`
  validates a complete externally authored package in CI.
- Existing VN-only games and all original `.vn` commands remain unchanged and
  continue running under the regular CLI/Bevy regressions.

The NA04 milestone remains **active**, not verified, until integration and all
specified acceptance checks are green on the committed code.
