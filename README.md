# Vesper

Vesper is a Rust + Bevy 0.20 visual novel runtime. The compiled engine will contain behavior; authored stories, characters, artwork, fonts, themes and audio will remain **external files**.

**Status:** scoped Vesper MVP verified complete (100/100 weighted roadmap points; GitHub Actions run 40571573177). Both demonstration branches, presentation, hot reload, sound asset decoding, save/load and compiler-free Linux packaging passed automated tests. Hardware audio output and exhaustive coverage of every possible Linux configuration were not directly tested.

## Two standalone fairytale examples

- **[The Little Lantern](examples/little-lantern/)** — the smallest complete visual novel: a little fox, one background, two choices and a shared ending.
- **[The Clockwork Moon](examples/clockwork-moon/)** — an extended, fully branched children's adventure with 18 scenes, 36 possible choice sequences, four endings, seven locations, music, audio cues, sprite expressions, story variables, save/load, and externally editable themes.

These are **separate game directories**, never compiled into the engine. Run them with:

```sh
cargo run --locked -p vesper-player -- examples/little-lantern
cargo run --locked -p vesper-player -- examples/clockwork-moon
```

Or download the compiled **vesper-linux-fairytale-showcase** artifact from [verified Linux CI](https://github.com/sguzman/vesper/actions/runs/40579833495). Unpack its inner `vesper-linux-x86_64.tar.gz`, then use `./vesper/lantern` or `./vesper/moon` without Rust or Cargo. [Example guide, controls, and visual QA](examples/README.md).

## Presentation polish (next phase)

The original MVP remains verified 100%. The separate [weighted presentation-polish roadmap](docs/polish-roadmap.json) is also **100/100 verified**, on [passing main-branch graphical and narrative CI](https://github.com/sguzman/vesper/actions/runs/40617751211); both milestone ledgers remain independent. **P** opens a keyboard-navigable pause menu (P again resumes), **M** opens the main menu during play, and **R** restarts after an ending. Main-menu commands start a new playthrough, continue the in-memory session, or quit. Pause-menu commands resume, open the main menu, or quit. Both menus use dedicated Bevy UI option rows with independently highlighted selection (no string-generated menu); Up/Down moves focus without respawning the overlay, and Enter or number keys 1–3 activate an action. Ending actions use the same row-based selection system. External games still launch directly into gameplay. Completed games now display an explicit, keyboard-selectable **Play again / Main menu / Quit** ending screen; Up/Down, Enter, 1–3, R and M work without a dead-end. The Little Lantern intentionally contains no audio; The Clockwork Moon uses external WAV assets.

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

The current `.vn` language is intentionally minimal and remains extensible. It supports `@scene`, named dialogue, narration, adjacent `* "text" -> scene` choices, `jump`, `set`, `if/else/end`, `background`, `show`, `hide`, `music` and `sound`. Variables are signed integers, booleans or quoted strings. The Rust interpreter yields commands directly to the Bevy ECS presentation layer. Scripts, art, audio, themes and fonts all remain external runtime data.

## Architectural commitments

- Bevy 0.20 with idiomatic ECS, 2D, UI, audio, and rich typography.
- Pure Rust narrative language, no Lua and no runtime AI.
- Game directories loaded at runtime, never built into the Rust executable.
- Developer hot reloading and safe state recovery before 1.0.
- Bevy's `bsn!` may define engine-owned UI; no dependency on external BSN loading in 0.20.
- No speculative implementation for other genres. Complete a working VN player first.

See `docs/architecture.md` and `docs/roadmap.json` for technical and completion policies.

## Responsive story presentation

The engine now scales panel margins, padding and minimum height with the window, while preserving each external theme's authored panel width, text sizes, contrast, and font. A subtle separator and shaped-glyph-height reservation keep wrapped dialogue/choices separate from the keyboard hints, even at fractional UI scales. The graphical regression checks actual rendered glyph bounds at 946×511 and 955×520 (the reported Wayland window sizes) as well as 800×600 and 900×700; the old implementation overlapped by 5 px at 946×511, while the verified measured-height implementation leaves 13 px at 946×511, 12 px at 955×520, 6 px at 800×600 and 12 px at 900×700. The responsive CI suite waits for actual rendered pixels after resize and fails with screenshot evidence if the art never reaches the viewport edges.

## Differentiated external fairytale themes

The Little Lantern supplies `examples/little-lantern/styles/lantern.toml` with warm sunrise amber, cocoa and parchment tones; The Clockwork Moon supplies `examples/clockwork-moon/styles/moonlight.toml` with midnight-blue, silver and cyan. Both manifests load their own external theme, and the common engine renders the theme-owned `[panel] background`, `accent`, and `divider` colors. All four text categories retain independently authored type sizes and colors. The graphical theme smoke test validates text/background contrast, screenshots, and a live accent change for each game.

## External typography

Set `theme = "styles/dialogue.toml"` in `game.toml`. Themes configure `ui_scale`, panel dimensions, RGBA colors (including optional `[panel] accent` and `divider` tokens), and separate speaker/dialogue/choice/hint sizes and colors. Select `[font] family = "DejaVu Sans"` for system font discovery or `[font] file = "assets/fonts/YourFont.ttf"` to load an external TTF/OTF. Font paths stay inside the game directory and no game font is compiled into Vesper. Unicode wrapping uses native Bevy text layout. Authors must license their font files.

## Saving and resuming

Press **F5** to save the current story, presentation state and pending choices; **F9** restores the slot. Save/load file I/O runs on background threads and never blocks Bevy's update loop. Versioned JSON saves are stored under `$XDG_DATA_HOME/vesper/saves/` or `~/.local/share/vesper/saves/` and are bound to the game id and exact story source fingerprint. `VESPER_SAVE_DIR` overrides the location for tests. Story changes safely invalidate incompatible old snapshots rather than restoring unsafe offsets. Clean exit with **Escape** completes outstanding save/load workers after the window stops updating, and atomic save writes are flushed to disk before replacement. Saves immediately before quitting are covered by the reference playthrough test.

## Audio and visual transitions

In the game's `game.toml`, declare `[music]` and `[sounds]` asset identifiers pointing to game-relative WAV or OGG files. Narrative commands `music evening` and `sound greeting` queue native Bevy audio playback; music loops and replaces the previous track, effects play once. The sample files in `game/assets/audio/` are small original PCM tones. Bevy silently handles systems without an audio output device (such as headless CI), but audible playback still needs physical QA. Themes can set `fade_seconds` (0–3) for sprite/background fade-ins. During a choice, Up/Down cycles the highlighted selection, Enter/Space confirms it, and digit keys 1–9 continue to work. Choice highlight and active music are stored in saves.

## Linux packaging

Build on Linux and run `bash packaging/linux.sh target/release/vesper-player dist/vesper-linux-x86_64.tar.gz`. The archive contains the native player, a relocatable `vesper` launcher, the license and all **external** `game/` scripts/assets. Users can unpack it anywhere and run `./vesper/vesper` without Cargo or Rust. This archive requires compatible host Linux graphics/windowing/audio runtime libraries; it is not statically linked.

## Live game editing

Launch `vesper-player game --watch` during development. Vesper reloads valid external `.vn` and `.toml` changes on a worker thread, applying compatible paused dialogue/choice and theme updates without recompiling. Invalid scripts or incompatible changes leave the last good state in place with a diagnostic. Bevy's file watcher independently refreshes modified images and fonts. Watching is disabled by default for published games.

## Verification

The test suite covers both endings of the external reference story, replaying the fully rendered game under Linux/Xvfb and asserting durable F5 saves on clean exit. `cargo test --locked --workspace` validates the Rust narrative and manifest code. GitHub Actions also runs screenshot checks for dialogue choices, transitions, font styling, hot reload, packaging, save/load and malformed input. **Escape** exits cleanly; live edits are opt-in with `--watch`.

## Verified MVP completion

The agreed weighted roadmap is **100% verified** by [GitHub Actions run 40571573177](https://github.com/sguzman/vesper/actions/runs/40571573177), on source commit `e9a9a9c`. The two reference VN branches reached their authored endings in graphical CI; versioned saves survived both restart and immediate clean shutdown. Validation covers malicious paths, missing references and rejected live edits. The Linux archive launches from an unrelated directory without Rust/Cargo on `PATH`. CI is headless, so it verifies native audio decoding and playback entity integration, **not speaker output on physical hardware**. The available demo uses minimal placeholder art/sounds, not final commissioned content.
