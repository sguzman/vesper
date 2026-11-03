# Vesper optional walk-first preview

This external example exercises a **bounded side-on walking controller**
with an authored 1280×720 street, one neighboring café, a placeholder avatar,
a visible NPC/prop and a location camera. The small art assets are reused
from Little Lantern for validation; **this is not** the forthcoming original
anime urban slice-of-life novel.

Only games with an explicit `[exploration.start]` enter walking mode.
Remove this subsection to launch the same package as a pure visual novel.

```sh
cargo run --locked -p vesper-cli -- check-game examples/exploration-schema
cargo run --locked -p vesper-player -- examples/exploration-schema
```

**Controls:** Left/Right arrows or A/D walk; P pauses, M opens the menu.
This checkpoint intentionally does **not** make NPCs/exits interactive: the
deterministic conversation bridge and explicit location transitions are
NA06. F5/F9/F6 exploration saves are deliberately disabled until NA07
can persist logical world positions and restore them safely.

Native CI exercises actual Bevy movement, paused input, resizing, art
rendering and the incomplete-save guard. Legacy VNs remain unaffected.
See [schema authoring](../../docs/exploration-schema.md) and the
[expansion roadmap](../../docs/narrative-adventure-roadmap.md).
