# Optional exploration layer planes

Vesper's external version-one exploration schema now accepts an optional
`plane` on each authored `[exploration.locations.<place>.layers.<name>]`
table. **Omitting it behaves exactly as before**; old pure visual novels and
old exploration packages need no migration.

| `plane` | Draw order | Intended use |
| --- | --- | --- |
| `"background"` (default) | With authored scenery, behind NPCs, props and the player | Distant scenery, parallax backgrounds |
| `"foreground"` | **Above NPCs and props, below the player avatar** | Transparent counter fronts, railings, vegetation overlapping actors |
| `"overlay"` | Above NPCs, props **and** the player avatar | Translucent rain, windows, overhead visual effects |

Each plane retains the existing bounded `depth = -12.0..12.0` sorting
preference within that plane. `parallax = 0.0..2.0` and the authored
full-scene transparent PNG path behave as before. Engine-owned UI remains
outside exploration world layers, so overlays must not become a faux dialog
box. Foreground/overlay do not grant new interaction, collision, or movement
behavior.

Illustrative, **not installed in any example**, for a shop whose actual art
pipeline eventually supplies a transparent foreground countertop strip:

```toml
[exploration.locations.store.layers.counter_front]
image = "assets/backgrounds/shop-counter-front.png"
plane = "foreground"
depth = 0.0
parallax = 1.0
```

The transparent image should use the same full-scene canvas and viewpoint
as the world background. Its nontransparent pixels should appear only where
the foreground element physically overlaps world actors; a large opaque
rectangle would hide the shop and characters. For the existing original
*Una Tarde en el Barrio*, producing that art is a separate blocker
([issue #51](https://github.com/sguzman/vesper/issues/51)); **the current
flattened shop image and invisible Mateo/cat hitspots are unchanged**.

## Implementation safety

- `background` preserves legacy `z = -5.5 + 0.08 * depth`.
- `foreground` uses a distinct actor-safe band around `z = 3` (NPCs
  at `z = 2`, props at `z = 1.5`, avatar at `z = 4`).
- `overlay` uses a separate band around `z = 5`, above the avatar.
- All three bands stay disjoint for the existing validated depth range.
  The numeric scale is an internal implementation detail, not a game
  authoring requirement.
- The schema rejects unknown plane names and types. Image paths retain the
  existing game-relative validation, optional layers remain optional, and
  ordinary visual novels never spawn exploration world sprites.
- Rust parser and Bevy renderer unit tests check defaults, new values,
  incorrect inputs, ordering and cross-band separation. Native visual
  compositing and physical owner QA must be performed with real finished art
  before any art/acceptance points are earned.
