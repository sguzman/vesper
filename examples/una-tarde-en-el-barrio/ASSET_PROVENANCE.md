# Asset provenance — Una Tarde en el Barrio (current v0.7.7, historical revisions preserved)

All files are produced for this development project; none are taken from a
third-party game. This record does **not** assert final user approval,
commercial licensing review or completed artist attribution.

- `assets/backgrounds/street.png`: original generated anime-urban street
  illustration for this game, no baked HUD. Source lineage retained in the
  earlier v0.3 release. `street-evening.png` is a derived dusk treatment.
- `assets/backgrounds/store-illustration-study.png`: detailed storefront
  interior study composed from **the independently generated concept image**
  `a_highly_detailed_anime_style_visual_novel_explo.png` made during this
  project. The visible UI/portrait was cropped out or replaced, a wood floor
  was synthesized, and a painted shopkeeper remains in the backdrop to avoid
  rough duplicate sprite art. This is not final painter-approved world art.
- `assets/backgrounds/tienda-interior-archive.png`: archived prior geometric
  shop collage; not referenced by the playable manifest.
- Historically, `assets/npcs/elena.png` and `rosa.png` were source-rendered v0.6
  vector sprites; later sections document their current illustrated replacements. The older Mateo drawing is preserved but not currently
  referenced; further anime-quality polish remains outstanding.
- Historically, `assets/avatars/idle.png`, `walk-1.png` through `walk-4.png`
  were provisional vectors. The current five-pose sheet is described below.
- Historically, `assets/npcs/elena-portrait-provisional.png` was a
  hand-masked concept portrait. The current file derives from Elena's matching
  full-body illustration, documented below.
- `assets/props/scene-noticeboard.png` and `invisible-hitspot.png`: invisible
  authored scene anchors, since the real noticeboard was painted into the
  external background. `bulletin-board-art.png` is derived board art embedded
  in the street background.
- `assets/audio/shop-bell.wav` and `paper-rustle.wav`: new project-generated
  one-shot waveforms, not samples taken from an existing copyrighted soundtrack.

This document distinguishes generated *concept artwork* from final production
assets. No screenshots in this package are represented as a successful native
Bevy or Hyprland game run.

## v0.5 original synthetic audio

`assets/audio/golden-hour-ambient.wav` and
`assets/audio/shop-hush-ambient.wav` were authored as original numerical
synthesis within this development session: harmonic pads, deterministic
soft bell/chirp textures, simple envelopes, PCM 16-bit mono 22.05 kHz.
There are no imported recordings, loops or copyrighted compositions.
They have not yet been audited through a real audio output device.

## v0.6 drawable cast sources

The v0.6 game PNGs for `elena`, `rosa`, and all five avatar states were
re-rendered from the new project-authored vector SVG sources stored under
`sources/characters/`. The same SVG figures can be reviewed, adjusted and
regenerated without acquiring external stock character art. Their simplified
chibi execution is a deliberate *interim asset*, not the final anime
character-style signoff. Original rendered landscape, store and incomplete
Elena narrative portrait are unchanged. The generated full-screen UI mockups
seen during planning are **not** part of this game, and no concept preview is
represented as an actual Bevy screenshot.

## v0.7.1 original illustrated dialogue skin

`assets/panels/barrio-panel.png` is rasterized from the authored,
editable `sources/ui/barrio-panel.svg` in this package. It contains no HUD,
printed dialogue, characters or borrowed game artwork. The gold framing,
floral corner ornament, dark-violet fill and nine-slice margins were designed
for a resilient Vesper panel at both normal and compact widths. Actual
software-rendered Bevy screenshots confirm it is used at runtime; their
success does not constitute final visual approval.

## v0.7.3 audio derivative

The current `assets/audio/paper-rustle.wav` is the same original synthetic
one-shot with a deterministic 12 ms `ffmpeg` fade-in. The exact original
waveform remains at `sources/audio/paper-rustle-raw.wav`. Rebuild script:
`tools/rebuild-paper-rustle.sh` (PCM 16-bit), no third-party recording.

## v0.7.4 — Elena character-art revision

The game now uses `sources/characters/elena-fullbody-anime-v074.png` as the
source for Elena's standing full-body sprite and her single calm VN portrait.
It was generated specifically for this original Vesper example during the
2026-11-04 development session, using the established Elena character design
from the original editable SVG: dark ponytail, twin gold hairpins, warm brown
skin/freckles, over-ear headphones at her neck, navy jacket/cream hoodie,
blue jeans and pale sneakers. The source is a high-resolution transparent
RGBA PNG, not a screenshot from Bevy and not extracted from third-party media.

`tools/rebuild-elena.sh` makes the game-consumed PNG files using ImageMagick,
clearing low-alpha generation haze, preserving aspect, and deriving the VN
portrait from the same character artwork. The old `sources/characters/elena-v06.svg`
remains only as provenance/history, not the current production sprite source.

**Not accepted:** this one character revision does not establish an accepted
character model bible, matching Rosa and protagonist art, runtime animation,
production voice/music, full native visual QA or owner aesthetic approval.

The supplemental `qa/street-static-art-composite.png` deliberately combines
**actual packaged** street, avatar, Elena and Rosa assets at their nominal
132-pixel standing heights. Its shell source is
`tools/render-static-street-preview.sh`. It is not a runtime capture and must
not be described as one.

## v0.7.6 — Rosa original character illustration

`sources/characters/rosa-fullbody-anime-v076.png` was created as original
artwork for this game from the established Rosa design: elderly neighbor,
silver braided hair, gold hoop earrings, purple flowered blouse, cream apron,
long dark skirt. It was not copied from a third-party image. It is an RGBA
source, not a Bevy screenshot. The deterministic `tools/rebuild-rosa.sh`
exports the runtime `assets/npcs/rosa.png`; `qa/check-rosa-art.sh` confirms
that the installed pixels still come from the archived source.

This art has not yet been approved on actual physical Hyprland or in a Vesper
screenshot. The original v0.6 SVG is retained for historic provenance only.

## v0.7.7 — Original protagonist five-pose sprite sheet

`sources/characters/protagonist-idle-walk-sheet-v077.png` was generated
specifically for this original game, retaining the established protagonist's
navy hooded jacket, tan backpack, dark trousers and pale shoes. It contains
five continuous, **right-facing** painted poses on transparent RGBA: one idle
and four different phases of walking. It is not from a third-party game.
`tools/rebuild-protagonist.sh` slices this 1983x793 source into five 600x754
RGBA frames, aligning the same logical pivot and baseline on each to avoid
per-frame height changes. `qa/check-protagonist-art.sh` regenerates in an
isolated temp directory and pixel-compares all five game assets against the
source, catching stale or mismatched exports.

The project uses Vesper's normal gameplay-time left/right sprite flipping;
no runtime programmatic motion generation is done. Frame-by-frame Bevy QA and
physical owner approval remain pending. Prior v0.6 SVGs remain in `sources/`
for historical design provenance.
