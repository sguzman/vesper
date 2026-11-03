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

**Controls:** Left/Right arrows or A/D walk; **E** interacts with the
nearest in-range NPC/object; **Q** cancels a conversation. P pauses, M
opens the menu. The readable focus cue names the nearest target.

The .vn fixture exercises `surface bubble`, an authored choice followed by
`explore return`, a `mode visual_novel` alternative, and an illustrated
`surface panel` noticeboard interaction. Location exits, conditional NPC
visibility and world-aware saves are not implemented yet (NA06–NA07).
F5/F9/F6 are disabled during exploration and returnable conversations.

Native CI exercises Bevy movement, an NPC interaction, Q cancellation,
an authored return to the prior X, paused input, resizing, and the
incomplete-save guard. Legacy VNs remain unaffected.
See [schema authoring](../../docs/exploration-schema.md) and the
[expansion roadmap](../../docs/narrative-adventure-roadmap.md).
