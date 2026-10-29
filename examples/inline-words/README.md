# The Ember Letter — QOL04 styled dialogue fixture

A tiny independent external game. Its lines contain plain text interleaved with named inline styles:

```text
"The [[fx=ember]]fire[[/fx]] reveals a [[fx=shadow]]secret[[/fx]]."
```

This markup is interpreted by the graphics-independent story parser once. The runtime and history receive the **plain text** plus bounded style spans. Bevy owns native TextSpan rendering. Unicode extended grapheme clusters cannot be split by a style boundary.

Define each used style as `[inline_styles.ember]` in the game's theme TOML. A style can specify `color = [r, g, b, a]` and optional `font_family`; accessibility high contrast and a reader's font override take priority. No external image, font download, paid dependency, shader or gameplay-specific Rust code is required.

Adjacent tags are permitted, nesting is not. Escape a literal opening marker as `\\[[` in a quoted .vn literal (decoded form: `\[[`). Unknown tags and undeclared style names are rejected with source-line diagnostics during game load. Use `cargo run --locked -p vesper-player -- examples/inline-words`.
