# The Atlas of Unwritten Skies

**An original, complete, branching, long-form visual novella for Vesper.**

A city is disappearing from every map. Mira Vale, the archivist of a cloudborne observatory, and Cael Vey, a courier who has been made to forget his own family, must decide what a machine built to save people is allowed to take from them. The story is about power, consent, memory, public history, love, accountability, and the ordinary work of constructing a future.

The game is **entirely external to Vesper's Rust player**. Its `.vn` source, images, character staging, original soundtrack, and dialogue panel reside under this folder; no engine changes or internet access are needed at runtime.

## What is included

- **61 scenes**, **25 choice options**, four complete endings and many persistent variable-dependent paragraphs. A graph enumerator verifies **8,748** possible walks through named branches; these are combinations of authored choices, *not* 8,748 separately authored narratives.
- About **65,489 words in editable source across all branches**, and **50,607–52,255 words in each complete route**, including four distinct long epilogues. The latest chapters—*The Library of Lost Afternoons*, *The Ferry of Blue Glass*, and *The Room Beneath the Weather*—explore Mira’s mother, Cael’s erased family history, a forbidden passage into AsterVale, and an underwater archive of days removed from memory. The provisional novel-length prose goal has been met across all four routes, but the example's **visual-art acceptance remains in progress**. Do not claim the finished flagship until the newly replaced location art, expression variety, and rendered gameplay receive the same quality check.
- **22 authored 1280×720 backgrounds**: the original observatory grades, eight newly painted native-resolution storybook scenes (lost city, coast, market, name garden, archive, undersea lens, shattered lens and airship port), the Blue Glass Ferry and underwater chamber, and five newly drawn story-specific settings: the bell garden, relief ministry, unreturned quay, public-questions hall, and abandoned station. All fifteen new location illustrations have reproducible build-time drawing source. The eight classic locations are now independently painted full-resolution scenes rather than blurry magnifications of concept thumbnails, though their cleaner storybook look still needs visual harmonization with the painterly character sprites and elaborate panel.
- **Two original AI-assisted painterly character illustrations**, each with three lighting/expression treatments on transparent PNG, and a bespoke gold/navy illuminated-manuscript dialogue frame.
- **Seven original, evolving 68-second ambient music cues and two original sound effects**, with compact Ogg Vorbis tracks for playback and uncompressed WAV effects. No pre-existing tracks or paid APIs used.
- Live ambient light motion, character staging/idle/crossfades, cinematic transitions, reader font/auto-advance controls, save/restore, reduced-motion and high-contrast handling.
- Four optional editorial reading editions in `reader_editions/` and all game-authoring source files.

## Run from the Vesper repository

This complete game is versioned at `examples/atlas-unwritten-skies/`. No separate ZIP extraction is needed after checking out the repository.

```fish
cargo run --locked -p vesper-cli -- check-game examples/atlas-unwritten-skies
cargo run --locked -p vesper-player -- examples/atlas-unwritten-skies
```

The long-form text is authored as `scripts/novel.source` and compiled to `scripts/story.vn`. The game uses the existing Vesper player, not a game-specific executable.

## Dialogue panel and reader width

The illustrated manuscript panel uses **aspect-preserving end caps**: the
celestial globe, books and lantern are never squeezed to fit a window.
Only a narrow undecorated **horizontal parchment strip** expands.
Atlas defaults to a narrower **72% dialogue width**, capped at **1180 logical
pixels** on large displays and with a compact-window readability minimum.
The authored `compact_top_boost` keeps words below the upper ornament on
smaller windows rather than assuming that its border scales like plain UI.

Press **P**, choose **Settings**. **Dialogue width** adjusts the frame
from **50% to 100%** in 5% increments, subject to the authored min/max caps.
**Dialogue position** moves the panel up or down (0–20% above the viewport
bottom). **Text ink** cycles through game default, black, dark brown, navy,
and sepia. Preferences persist without editing the game's TOML, and
**Reset to game defaults** restores the authored look.

Atlas uses a fixed-size illustrated frame and an explicit protected text-safe
rectangle measured against the rendered panel. Speaker and prose remain inside
the parchment, and the hint is anchored separately, regardless of text length.
Longer prose is paginated at sentence and word boundaries into short,
readable pages (at most 22 words / 120 characters each), preserving every
word of the original novel. The engine still supports height easing for
unusual content and respects reduced-motion settings.

**Artwork authoring:** the engine also accepts optional two-axis cuts
(`vertical_slices` and `reference_height`) for **intentionally authored
stretchable-paper skins**. Atlas's original painting is *not* vertically
slice-safe—its books, ivy and lantern cross the potential cut—so Atlas itself
retains the nonwarping, aspect-preserving three-part configuration rather than
distorting its ornaments.

## Controls

Enter/Space progresses; Up/Down or number keys choose; P pauses; P then 4 opens Settings including font browser and automatic advance delay. H opens reading history, F5/F9 quick saves/loads, F6 manual slots, K skips skippable staging, M opens the main menu. Fullscreen and window resizing follow the same Vesper engine rules as the original examples.

## Production media

`assets/source/` preserves the original character, panel and observatory sources plus the losslessly split location concept board. The **finished** game uses pre-rendered files under `assets/backgrounds/`, `assets/characters/` and `assets/panels/` and does **not** run any media generator. The new independent backgrounds can be regenerated with `python tools/paint_blue_glass.py`, `python tools/paint_submerged_memory.py`, and `python tools/paint_story_places.py`, and `python tools/paint_classic_locations.py`; the saved PNGs are loaded directly by the player. The old location concept-board source is preserved in `assets/source/` and `tools/extract_locations.py` is retained for historical reproducibility, but it no longer generates the shipped eight classic-location backdrops. This is production tooling, not a player requirement. A font file is **not** included or distributed; Vesper uses the reader's installed fonts and fallback.

The extended 68-second themes now evolve through original chord changes, plucked motifs and atmospheric textures. `tools/compose_atlas_score.py` creates the original PCM WAV masters offline; the distributed Ogg Vorbis tracks were encoded from these masters using FFmpeg. The game player consumes the resulting audio files without executing the composer. The audio is intentionally gentle and synthetic rather than recorded orchestral music. It showcases playable audio pipelines but should not be mistaken for a professionally recorded score. The original observatory and gold/navy dialogue panel are high-resolution. The location pictures are original full-resolution compositions rather than enlarged concept-board exports; unifying the painterly and graphic storybook art treatments is still an artistic acceptance item.

## Verification status

The original offline authoring validator previously passed structural, asset and branching checks. The repository CI also invokes Vesper's native `check-game` on this directory; consult the CI result for actual engine validation. Full graphical playthrough, four-ending navigation and physical speaker QA are separate acceptance gates, not yet claimed here.

See `QA_CHECKLIST.md` for manual acceptance. The game's sources and media are committed on the Atlas integration branch; do not treat that as a merged release until CI and integration pass.

## Four endings (titles only; no spoilers)

- **The Open Sky** — reunion without guarantees.
- **The Living Witness** — sacrifice and the right to return.
- **The Solstice Covenant** — an agreement with a deadline.
- **The Unbound World** — abolition and the work that follows.

The canonical path in `reader_editions/open_sky.md` offers one continuous reading experience, but the game itself supports the other paths, earlier evidence choices, relationship changes and multiple conclusions.
