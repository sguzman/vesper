# The Moving Meadow — QOL07 motion fixture

Three independent ambient actors in one external game, **without character dialogue slots** or gameplay-specific Rust branches.

- **Walker:** back-and-forth ping-pong path, sprite-frame animation and directional flipping.
- **Bird:** repeating three-point flight path with a nonzero author-controlled starting phase.
- **Plane:** one-shot traverse along the top of the stage; it stops at its authored terminal waypoint.

Positions use normalized 1280×720 stage coordinates, rendered with responsive fit scaling. QOL07 samples finite paths and cached image handles in a bounded per-frame system; no runtime image scanning or physics integration.

```sh
cargo run --locked -p vesper-cli -- check-game examples/moving-meadow
cargo run --locked -p vesper-player -- examples/moving-meadow
```

The sample uses existing project-owned Tess character frames and bird art, a reused original sunrise backdrop, and a new original simple airplane icon. Its separate Xvfb acceptance captures start/mid/end and reduced-motion freeze. The branch remains unverified until the complete CI suite and merged-main gate succeed.
