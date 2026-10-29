# Morning at the Perch — static ambient actor fixture

This is a small external authored game for **QOL06**, not a new core story or the later motion/flipbook milestone QOL07.

- `[ambient_actors.bird]` declares a game-relative PNG, normalized canvas anchor, scale, opacity, background depth, optional scene association, initial visibility and deterministic starting phase.
- `ambient hide bird` and `ambient show bird` are discrete narrative visual events. They do not touch speaking-character slots. A scene-bound bird stays absent outside its selected scene.
- Visibility overrides are saved in the existing version-1 presentation state. Old saves with no ambient key use manifest defaults. Replacing/removing external assets is mediated by worker-validated hot reload.
- Six-size render and bird-only screenshot crops test stable geometry, show/hide, scene restrictions and F5/F9 visibility restoration. Original simple pixel bird asset and the repository's existing sunrise background are local, reusable examples.

To run: `cargo run --locked -p vesper-player -- examples/ambient-showcase`.

No autonomous flight or sprite animation yet: those are **QOL07**.
