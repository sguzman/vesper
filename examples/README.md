# Two complete fairytales for Vesper

These are self-contained **external games**, not hard-coded engine demos. Every script, character, image, sound, and theme is loaded from its chosen game directory. No Rust recompilation is needed to switch novels.

## The Little Lantern — beginner example

Path: `examples/little-lantern/`. A fox and a lantern, one background, one sprite, a single two-option choice, two little branches and a shared ending. Minimal `game.toml`, no styling or audio configuration.

```sh
cargo run --locked -p vesper-player -- examples/little-lantern
```

## The Clockwork Moon — feature/stress example

Path: `examples/clockwork-moon/`. A three-act fairytale with **36 choice combinations** (three trails × four trials × three final decisions), four endings, seven backgrounds, five character expressions, background fades, three music cues, five sound effects, conditional storytelling, variables (integer/boolean/string), nested conditions, all comparison operators, arithmetic including subtraction, repositions and hides, long wrapped text, and Spanish/Japanese passages. All effects use existing public Vesper commands.

```sh
cargo run --locked -p vesper-player -- examples/clockwork-moon
# Development-only live editing of .vn, .toml, images and fonts:
cargo run --locked -p vesper-player -- examples/clockwork-moon --watch
```

Controls: **Enter/Space** advance dialogue, **Up/Down** move through choices, **1–9** choose directly, **F5** save, **F9** load, **Escape** quit. Saves use a separate game ID for each example and do not modify authoring files.

## Verification

Run without the graphics stack:

```sh
cargo run --locked -p vesper-cli -- check-game examples/little-lantern
cargo run --locked -p vesper-cli -- check-game examples/clockwork-moon
cargo test --locked -p vesper-game --test fairy_tales
```

CI additionally launches both examples under Linux/Xvfb, captures actual Bevy screenshots, advances the story and checks visual changes and save restoration. The extended novel's graph test explores every reachable combination and rejects crashes, loops and missing endings. This automation checks *functionality*, not subjective illustration quality or audio on real speakers; a short human review on Hyprland is still worthwhile.

### Artwork and audio

Scenes use original procedurally drawn **pixel-style test artwork** (indexed PNG), intended to reveal layer, placement, background-swap and font/UI issues. The audio consists of original synthesized PCM tones and short motifs, for playback/load testing rather than finished orchestral music. Each game is fully portable with its assets. Neither novel embeds game-specific content in Rust.

No new Vesper engine features are required for either story.

## No-compilation Linux showcase

The CI Linux archive now includes both independent games and a separate launcher for each. Download the `vesper-linux-fairytale-showcase` artifact from the passing GitHub Actions run, unpack the `vesper-linux-x86_64.tar.gz` inside it, and run `./vesper/lantern` or `./vesper/moon`. These launchers work from any directory and use the same Vesper executable. As with the normal Linux bundle, compatible graphics/audio system libraries are required; the user does not need Rust or Cargo.
