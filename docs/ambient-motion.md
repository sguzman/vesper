# QOL07 — Autonomous ambient sprite motion

QOL07 extends [QOL06 ambient actors](ambient-actors.md). Motion is **optional**: existing static actors and ordinary character dialogue behave exactly as before.

## Authored motion

Inside a game's `game.toml`:

```toml
[ambient_actors.bird]
image = "assets/bird.png"
anchor = [0.12, 0.22]
scale = 1.4
opacity = 1.0
depth = 2.0
start_phase_seconds = 1.2

[ambient_actors.bird.motion]
waypoints = [[0.12, 0.22], [0.50, 0.14], [0.88, 0.30]]
period_seconds = 5.0
mode = "loop"
flip_on_reverse = true
# Optional image sequence already validated and preloaded:
# frames = ["assets/bird-1.png", "assets/bird-2.png"]
# frame_fps = 4.0
```

Modes are `loop`, `ping_pong`, and `once`; the latter remains at the final waypoint rather than abruptly teleporting or despawning. `period_seconds` measures one traversal; the round-trip ping-pong period is twice that duration.

Motion takes 2–16 **normalized** `[x, y]` waypoints, where both coordinates are finite and in `0..=1`. Each consecutive segment receives an equal share of the traversal period, independent of its geometric length. `period_seconds` must be finite and within `0.5..=120.0`. The initial phase is the actor's existing `start_phase_seconds`, bounded to `0..=120.0`.

An optional flipbook contains at most 16 safe, game-relative PNG paths and plays at a finite `1.0..=24.0` frames per second. All handles are created at spawn/reload and reused during frame updates; nothing parses markup or decodes an image on the UI thread.

## Behavior and accessibility

- Animations use a monotonic finite clock and a pure interpolator. The same author data and clock produce the same anchor and frame selection; no physics drift, dynamic memory allocation or PRNG is necessary.
- `flip_on_reverse` mirrors only the sprite image horizontally when its current segment heads left, without changing layout or the actor's anchor.
- The player pauses motion during pause/menu/history and after story completion. **Reduced motion** freezes both position and flipbook at their initial path state.
- Scene restrictions and `ambient show/hide` from QOL06 still apply. The active clock is copied into optional version-one save data on explicit saves, and validated on load; previous saves remain compatible. No per-frame save writes.
- Externally edited motions and frame lists are validated off the render thread; the existing actor is reconciled without recompiling the engine.
- Ambient actors remain behind speaking actors and in front of backgrounds; body/choice layout is never changed by them.

## Runnable example and acceptance

[The Moving Meadow](../examples/moving-meadow/) contains three simultaneous actors: ping-pong walking character with flipbook frames, looping bird flight, and one-shot plane traverse. Run:

```sh
cargo run --locked -p vesper-cli -- check-game examples/moving-meadow
cargo run --locked -p vesper-player -- examples/moving-meadow
```

The CI test `.github/scripts/ambient_motion_smoke.py` captures genuine native Bevy start/mid/end frames, verifies all three independent movements, measures six reference window sizes and confirms reduced-motion freeze. This is not a physics engine, a full character rig, or a claim of physical Wayland GPU acceptance.
