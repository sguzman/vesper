# The Luminous Page

QOL05 static and dynamic, author-controlled **per-word effects**. This independent external story has three effects active in a single line.

- `ember`: gently flickering warm-channel colors.
- `shadow`: rhythmic but bounded dimming that preserves readable text.
- `shimmer`: slow periodic cool-color sheen, not geometric glyph movement.

The effects never change glyph positions, sizes, line wrapping, reveal offsets, or story/snapshot text. Reader high contrast suppresses author colors and all effects; reduced motion freezes animated colors at one stable instant. No user-specific renderer branch and no shaders or paid runtime.

```sh
cargo run --locked -p vesper-cli -- check-game examples/word-effects
cargo run --locked -p vesper-player -- examples/word-effects
```

`docs/inline-styles.md` describes the exact externally authored options and limits.
