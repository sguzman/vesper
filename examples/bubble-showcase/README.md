# Optional native speech-bubble fixture

This game is a **test fixture**, not the forthcoming urban slice-of-life story.

It shares The Little Lantern's original authorized script and assets, with a
new game ID and an optional `[dialogue]` surface default. It demonstrates
speech bubbles in a **pure visual novel**, without movement or exploration.
Narrator lines use the viewport fallback; Pip's dialogue uses the staged
character's head as its bubble anchor. The standard panel remains available
for choice controls and the ending.

```sh
cargo run --locked -p vesper-cli -- check-game examples/bubble-showcase
cargo run --locked -p vesper-player -- examples/bubble-showcase
```

Omit `[dialogue]` in a game to preserve the original full illustrated VN
dialogue panel. This fixture does not change any pre-existing examples.
