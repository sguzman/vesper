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
`explore return`, a `mode visual_novel` alternative and an illustrated
`surface panel` noticeboard interaction. E also activates the author-defined
street-to-café exit and reverse exit, each using the named destination spawn.
A new NPC, `friend`, only becomes visible and interactable when the
persistent story variable `met_neighbor` becomes true after dialogue.
No visit is required to advance the original visual novel.

World-aware saves and authored VN-to-exploration commands remain future work
(NA06–NA07). F5/F9/F6 are disabled in exploration and modal conversations.

Native CI exercises Bevy movement, NPC interactions, Q cancellation, exact
return position, the conditional friend, two E-only location transfers,
old-area sprite teardown, pause and resize, and incomplete-save guards. Legacy VNs remain unaffected.
See [schema authoring](../../docs/exploration-schema.md) and the
[expansion roadmap](../../docs/narrative-adventure-roadmap.md).
