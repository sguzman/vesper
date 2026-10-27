# Vesper

Vesper is a Rust + Bevy 0.20 visual novel runtime. The compiled engine will contain behavior; authored stories, characters, artwork, fonts, themes and audio will remain **external files**.

**Status:** early implementation. The narrative core is CI-tested; the Bevy text player is implemented but graphical build/run verification is pending.

## Workspace

- `crates/story`: graphics-independent `.vn` parser, validator and deterministic interpreter.
- `crates/cli`: `vesper check <story.vn>` validates an external story file.
- `crates/player`: Bevy 0.20 dialogue-and-keyboard player reading a `.vn` path at runtime; graphic build verification pending.
- `game/scripts/demo.vn`: editable sample external story.
- `docs/roadmap.json`: durable weighted roadmap and acceptance criteria.

## Development

Requires stable Rust and Bevy's Linux development libraries.

```sh
cargo test -p vesper-story -p vesper-cli
cargo run -p vesper-cli -- check game/scripts/demo.vn
cargo run -p vesper-player -- game/scripts/demo.vn
```

The `.vn` language is provisional. It currently supports `@scene`, named dialogue, narration, adjacent `* "text" -> scene` choices, `jump`, `set`, `if/else/end`, `background`, `show`, `hide`, `music` and `sound`. Variables are signed integers, booleans or quoted strings. The interpreter yields presentation instructions to a future Bevy bridge. This is not a complete production authoring language yet.

## Architectural commitments

- Bevy 0.20 with idiomatic ECS, 2D, UI, audio, and rich typography.
- Pure Rust narrative language, no Lua and no runtime AI.
- Game directories loaded at runtime, never built into the Rust executable.
- Developer hot reloading and safe state recovery before 1.0.
- Bevy's `bsn!` may define engine-owned UI; no dependency on external BSN loading in 0.20.
- No speculative implementation for other genres. Complete a working VN player first.

See `docs/architecture.md` and `docs/roadmap.json` for technical and completion policies.
