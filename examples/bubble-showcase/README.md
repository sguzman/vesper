# Optional native speech-bubble fixture

This game is a **test fixture**, not the forthcoming urban slice-of-life story.

It reuses The Little Lantern's original art and branching plot but has a
separate story script with explicit `surface panel`, `surface bubble` and
`surface default` commands. It demonstrates per-line and scene-start style
switching in a **pure visual novel**, without movement or exploration.
Narrator lines use the viewport fallback; Pip's dialogue uses the staged
character's head as its bubble anchor. The standard panel remains available
for choice controls and the ending.

```sh
cargo run --locked -p vesper-cli -- check-game examples/bubble-showcase
cargo run --locked -p vesper-player -- examples/bubble-showcase
```

Omit `[dialogue]` in a game to preserve the original full illustrated VN
dialogue panel. This fixture does not change any pre-existing examples.
