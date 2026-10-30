# The Atlas of Unwritten Skies

**An original, complete, branching, long-form visual novella for Vesper.**

A city is disappearing from every map. Mira Vale, the archivist of a cloudborne observatory, and Cael Vey, a courier who has been made to forget his own family, must decide what a machine built to save people is allowed to take from them. The story is about power, consent, memory, public history, love, accountability, and the ordinary work of constructing a future.

The game is **entirely external to Vesper's Rust player**. Its `.vn` source, images, character staging, original soundtrack, and dialogue panel reside under this folder; no engine changes or internet access are needed at runtime.

## What is included

- **39 scenes**, **25 choice options**, four complete endings and many persistent variable-dependent paragraphs. A graph enumerator verifies **8,748** possible walks through named branches; these are combinations of authored choices, *not* 8,748 separately authored narratives.
- About **20,000 words across all route variants**, with roughly **11,000 words per representative playthrough**. This is a complete novella-length visual novel rather than a conventional full-length print novel.
- **14 1280×720 scene treatments** of the bespoke high-resolution painted observatory original: warm sunset, night, storm, library close-up, distant coast, memory-city, altered lens, hidden market, first dawn, and more. They are **graded/cropped variants of one original illustration**, not 14 independent image generations.
- **Two original AI-assisted painterly character illustrations**, each with three lighting/expression treatments on transparent PNG, and a bespoke gold/navy illuminated-manuscript dialogue frame.
- **Seven original, procedurally synthesized ambient music cues and two original sound effects**, stored as uncompressed 16 kHz, mono PCM WAV. No pre-existing tracks or paid APIs used.
- Live ambient light motion, character staging/idle/crossfades, cinematic transitions, reader font/auto-advance controls, save/restore, reduced-motion and high-contrast handling.
- Four optional editorial reading editions in `reader_editions/` and all game-authoring source files.

## Run from the Vesper repository

This complete game is versioned at `examples/atlas-unwritten-skies/`. No separate ZIP extraction is needed after checking out the repository.

```fish
cargo run --locked -p vesper-cli -- check-game examples/atlas-unwritten-skies
cargo run --locked -p vesper-player -- examples/atlas-unwritten-skies
```

The long-form text is authored as `scripts/novel.source` and compiled to `scripts/story.vn`. The game uses the existing Vesper player, not a game-specific executable.

## Controls

Enter/Space progresses; Up/Down or number keys choose; P pauses; P then 4 opens Settings including font browser and automatic advance delay. H opens reading history, F5/F9 quick saves/loads, F6 manual slots, K skips skippable staging, M opens the main menu. Fullscreen and window resizing follow the same Vesper engine rules as the original examples.

## Production media

`assets/source/` preserves the original four generated image sources from this work. The **finished** game uses pre-rendered files under `assets/backgrounds/`, `assets/characters/` and `assets/panels/` and does **not** run any media generator. To reproduce the graded/cropped variants or deterministic audio offline, install Pillow and NumPy and run `python tools/build_media.py` in this game directory. This is production tooling, not a player requirement. A font file is **not** included or distributed; Vesper uses the reader's installed fonts and fallback.

The audio is intentionally gentle and synthetic rather than recorded orchestral music. It showcases playable audio pipelines but should not be mistaken for a professionally recorded score. The artwork is high-resolution and painterly; the scene variety is achieved primarily through composition/lighting treatments of a single generated location.

## Verification status

The original offline authoring validator previously passed structural, asset and branching checks. The repository CI also invokes Vesper's native `check-game` on this directory; consult the CI result for actual engine validation. Full graphical playthrough, four-ending navigation and physical speaker QA are separate acceptance gates, not yet claimed here.

See `QA_CHECKLIST.md` for manual acceptance. The game's sources and media are committed on the Atlas integration branch; do not treat that as a merged release until CI and integration pass.

## Four endings (titles only; no spoilers)

- **The Open Sky** — reunion without guarantees.
- **The Living Witness** — sacrifice and the right to return.
- **The Solstice Covenant** — an agreement with a deadline.
- **The Unbound World** — abolition and the work that follows.

The canonical path in `reader_editions/open_sky.md` offers one continuous reading experience, but the game itself supports the other paths, earlier evidence choices, relationship changes and multiple conclusions.
