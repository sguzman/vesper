# Vesper Action: Signal Depot

**Active experiment, not an accepted game.** The visual-novel code is preserved in its
existing crates, but this is an independent action-first Bevy executable.

This first room is deliberately small: a maintenance worker can move around original
pixel-art machinery, strike a hostile drone, dodge its telegraphed attacks, restore
power at a terminal and reach the door. It has no story dialogue or borrowed music.

## Run

From a Vesper checkout:

```fish
cargo run --locked -p vesper-action-demo -- examples/action-adventure
```

With the Nix build (after fetching the branch and building once):

```fish
nix run .#action-demo
nix run .#action-demo-host-nvidia  # opt-in Arch/EndeavourOS host NVIDIA
```

The Nix launcher packages the room's art as external assets. The host-NVIDIA
launcher does **not** install drivers or change global configuration.

## Controls

| Key | Action |
| --- | --- |
| WASD / arrow keys | Move (diagonals normalized) |
| J / Enter | Directional melee strike |
| Space | Short dodge with a limited invulnerable window |
| E | Operate power terminal / use exit |
| R | Replay after victory or defeat |

Defeat the three-hit security drone, activate the power console and exit. The
drone has an orange attack windup; dodging rather than tanking its contact is intended.

## Art and acceptance boundary

The original pixel art is directly editable in `art/*.px`.
Each source records dimensions, a named RGBA palette and exact rows of colored
pixels. The action crate loads them as RGBA texture assets at startup. No art or
map data from Eastward or earlier Vesper novels is reused.

The layout is a provisional 320x180 authored scene displayed at 2x in a
640x360 action space, with aspect-preserving camera fitting for odd windows.
The current content/character design, game feel and art quality are **unapproved**.
A successful build or an Xvfb screenshot only proves functionality, not that
combat feels good or the art meets the owner's standards.

**Next review gate:** actual play + visual evidence on 960x540 and narrow/portrait
windows; then fix controls, telegraphs, readability, room layout and animation
before introducing extra mechanics or a larger world.

No music is implemented. This avoids recycling the rejected visual-novel track.
