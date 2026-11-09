# Optional visual elevation and prop scale

An exploration NPC or prop may now be drawn above or below the area ground
baseline without changing its authored horizontal `x` or interaction circle.
Every new field is **optional**, so existing games and saves work unchanged.

| Field | Available on | Default | Meaning |
| --- | --- | --- | --- |
| `elevation` | NPC and prop | `0.0` | Signed vertical offset in logical world pixels. Positive raises the sprite |
| `scale` | NPC and prop | `1.0` | Relative to the existing 85px NPC or 46px prop nominal height |

Both `elevation` values must be finite and their magnitude must not exceed
the authored location height. `scale` must be finite within `0.35..=3.5`.
These use the existing camera scale so that 946×511 and 1280×720 windows
show the same relative authored placement.

Example of a **hypothetical** original shop after painted art is independently
reconstructed and a cat sprite has been validated (not applied to the game):

```toml
[exploration.locations.store.props.cat]
x = 302.0
image = "assets/props/cat-sleeping-isolated.png"
scale = 2.0
elevation = 170.0
interact_scene = "shop_cat"
interaction_radius = 85.0
```

This places the artwork near the countertop instead of visually grounding
it on the tiled floor, while its existing x-based interaction remains at
`x = 302.0`. For an actually seated NPC, an elevation may also be used.
Author the art in the same logical 1280×720 scene coordinate system.

**Scope boundary:** these properties do not modify walk physics, camera
framing, interaction reach, game variables, save format or dialogue behavior.
They do not add automatic collision or dynamic perspective sorting. Use the
separate optional foreground/overlay plane contract to mask characters
behind shelves or counters; test the final pixels in real Bevy screenshots.

The current *Una Tarde en el Barrio* artwork is still a flattened work
in progress. Do not install a placeholder cat or change the existing
invisible Mateo/cat interaction sprites before issue #51's coordinated
background and matched character exports are ready. Owner art and hardware
acceptance remain separate.
