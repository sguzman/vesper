# Vesper

Vesper is a Rust + Bevy 0.20 visual novel runtime. The compiled engine will contain behavior; authored stories, characters, artwork, fonts, themes and audio will remain **external files**.

**Status:** early implementation. The narrative core and external manifest validation pass CI. Bevy dialogue, sprite rendering, branching keyboard input and Linux startup passed automated screenshot tests; this is **not yet a complete VN engine**.

## Workspace

- `crates/story`: graphics-independent `.vn` parser, validator and deterministic interpreter.
- `crates/cli`: `vesper check <story.vn>` validates an external story file.
- `crates/player`: Bevy 0.20 player reading external game manifests, stories, images and keyboard choices; CI-verified graphical player with external artwork and keyboard branching.
- `game/scripts/demo.vn`: editable sample external story.
- `docs/roadmap.json`: durable weighted roadmap and acceptance criteria.

## Development

Requires stable Rust and Bevy's Linux development libraries.

```sh
cargo test -p vesper-story -p vesper-cli
cargo run -p vesper-cli -- check game/scripts/demo.vn
cargo run -p vesper-cli -- check-game game
cargo run -p vesper-player -- game
```

Independent `game/` and `game-alternate/` folders load without recompiling. The `vesper-game` crate verifies versioned manifests and rejects absolute, parent-traversal and external symlink paths. Small test PNGs are placeholders, not production artwork.

The `.vn` language is provisional. It currently supports `@scene`, named dialogue, narration, adjacent `* "text" -> scene` choices, `jump`, `set`, `if/else/end`, `background`, `show`, `hide`, `music` and `sound`. Variables are signed integers, booleans or quoted strings. The interpreter yields presentation instructions to a future Bevy bridge. This is not a complete production authoring language yet.

## Architectural commitments

- Bevy 0.20 with idiomatic ECS, 2D, UI, audio, and rich typography.
- Pure Rust narrative language, no Lua and no runtime AI.
- Game directories loaded at runtime, never built into the Rust executable.
- Developer hot reloading and safe state recovery before 1.0.
- Bevy's `bsn!` may define engine-owned UI; no dependency on external BSN loading in 0.20.
- No speculative implementation for other genres. Complete a working VN player first.

See `docs/architecture.md` and `docs/roadmap.json` for technical and completion policies.

## External typography

Set `theme = "styles/dialogue.toml"` in `game.toml`. Themes configure `ui_scale`, panel dimensions, RGBA colors, and separate speaker/dialogue/choice/hint sizes and colors. Select `[font] family = "DejaVu Sans"` for system font discovery or `[font] file = "assets/fonts/YourFont.ttf"` to load an external TTF/OTF. Font paths stay inside the game directory and no game font is compiled into Vesper. Unicode wrapping uses native Bevy text layout. Authors must license their font files.
