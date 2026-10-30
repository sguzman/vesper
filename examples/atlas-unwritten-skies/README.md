# The Atlas of Unwritten Skies

**An original, complete, branching, long-form visual novella for Vesper.**

A city is disappearing from every map. Mira Vale, the archivist of a cloudborne observatory, and Cael Vey, a courier who has been made to forget his own family, must decide what a machine built to save people is allowed to take from them. The story is about power, consent, memory, public history, love, accountability, and the ordinary work of constructing a future.

The game is **entirely external to Vesper's Rust player**. Its `.vn` source, images, character staging, original soundtrack, and dialogue panel reside under this folder; no engine changes or internet access are needed at runtime.

## What is included

- **47 scenes**, **25 choice options**, four complete endings and many persistent variable-dependent paragraphs. A graph enumerator verifies **8,748** possible walks through named branches; these are combinations of authored choices, *not* 8,748 separately authored narratives.
- About **39,000 words in editable source across branches**, and **26,000–27,000 words in each complete route**. The four endings now have extended, distinct epilogues. The full-length novel expansion remains underway. This is a complete novella-length visual novel rather than a conventional full-length print novel.
- **15 authored 1280×720 backgrounds: the original observatory grades plus eight distinct location studies of Bellwether, a moonlit coast, aerial harbor, hidden market, enchanted name garden, archive, memory realm and shattered world. The new studies are extracted from an original concept board and are not yet full-resolution location originals.
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

`assets/source/` preserves the original character, panel and observatory sources plus the losslessly split location concept board. The **finished** game uses pre-rendered files under `assets/backgrounds/`, `assets/characters/` and `assets/panels/` and does **not** run any media generator. To regenerate location studies offline, install Pillow and run `python tools/extract_locations.py`; `tools/build_media.py` also reconstitutes the full set with Pillow and NumPy. This is production tooling, not a player requirement. A font file is **not** included or distributed; Vesper uses the reader's installed fonts and fallback.

The audio is intentionally gentle and synthetic rather than recorded orchestral music. It showcases playable audio pipelines but should not be mistaken for a professionally recorded score. The original observatory and gold/navy dialogue panel are high-resolution. The independent location paintings are first-pass concept-board exports, not yet final native-resolution illustrations.

## Verification status

The original offline authoring validator previously passed structural, asset and branching checks. The repository CI also invokes Vesper's native `check-game` on this directory; consult the CI result for actual engine validation. Full graphical playthrough, four-ending navigation and physical speaker QA are separate acceptance gates, not yet claimed here.

See `QA_CHECKLIST.md` for manual acceptance. The game's sources and media are committed on the Atlas integration branch; do not treat that as a merged release until CI and integration pass.

## Four endings (titles only; no spoilers)

- **The Open Sky** — reunion without guarantees.
- **The Living Witness** — sacrifice and the right to return.
- **The Solstice Covenant** — an agreement with a deadline.
- **The Unbound World** — abolition and the work that follows.

The canonical path in `reader_editions/open_sky.md` offers one continuous reading experience, but the game itself supports the other paths, earlier evidence choices, relationship changes and multiple conclusions.
