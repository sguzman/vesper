# Una Tarde en el Barrio — Development history

## v0.7.4 — 2026-11-04 (Elena art consistency)

- Added detailed original full-body Elena character artwork, keeping the
  established outfit and hairstyle while replacing the simplified vector sprite.
- Derived the VN portrait from the same transparent production art to prevent
  abrupt face-design changes between walking and full-screen dialogue.
- Preserved source PNG and deterministic ImageMagick export script; added QA
  coverage of transparency, source resolution, and alpha silhouette.
- No narrative, world path, asset ID, gameplay logic, or verified-roadmap
  progress changes. Rosa/protagonist art and runtime verification remain open.

## v0.7.3 — 2026-11-04 (optional reader locale and audio cue edge)

- Authored Spanish narrative control legends for story text, choices, endings,
  world dialogue and its choice prompts. These require Vesper PR #31; the
  feature is backwards compatible with older themes but native CI/QA is pending.
- Fixed a nonzero first sample in the original paper-rustle cue with a small,
  reproducible 12 ms fade-in; retained the unmodified source file.
- Added automated technical audio checks for codec, channel count, duration,
  peak headroom and the adjusted cue start; **not** physical listening.
- Unchanged story graph, scenes, painted backgrounds, sprites and game routes.
  No change to weighted acceptance; character art and native QA are blockers.

## v0.7.2 — 2026-11-04 (source-only continuity pass)

- Align the arrival narration with the already-open illustrated storefront.
- Make the quiet ending consistent with the decision *not* to write a note.
- Rephrase the ledger privacy decision so the player's action and its consequence agree.
- Carry privacy and Rosa's previously heard concerns into the community-plan outcome; recall Mateo's autonomy in the quiet ending when the player actually spoke to him.
- Add `qa/check-package.sh`, a repeatable static check for local asset references, image/audio headers, scene graph connectivity, choice targets, conditional nesting, ending liveness, and world exit/spawn references. Negative QA probes confirmed missing targets and broken exit spawns are rejected.
- No new game locations or scenes; no engine changes; no replacement art. Full native and visual acceptance is still pending.

## v0.7.1 — 2026-11-04 (native-validated development build)

- Introduced the original nine-slice barrio dialogue panel and corrected Rosa's character-text continuity.
- Real Vesper parser and Bevy/Xvfb player exercised the opening, movement, interaction, saves, all three noticeboard endings and the postpone/revisit flow.
- Kept production character art, compositing, audio listening, physical Hyprland and final QA explicitly unfinished.

**v0.7.3 verification addendum:** The current Rust `vesper check-game` CLI
accepted v0.7.3 and the previous version, with negative localized-hint
validation probes; native player testing remains pending.

## v0.7.5 — 2026-11-04 (authored story branch coverage)

- Added an `awk` scenario interpreter for the exact external `.vn` source and
  eleven reproducible authored sequences including branch-sensitive dialogue,
  repeated conversations, note privacy and all three endings.
- Required complete exercise of all 31 scenes, all 22 choice edges, and both
  outcomes of all 24 conditional lines. Added two deliberate mutation probes
  to ensure that lost consequences and inverted flags fail the suite.
- Integrated the new suite into `qa/check-package.sh` and preserved the
  independent real Rust CLI authoring validation; no new engine feature and
  **no claim of a v0.7.5 Bevy playthrough**.
- No replacement for Rosa's or the avatar's simplified sprites; no final
  in-engine shop composition, hardware audio approval, or owner QA. Roadmap
  formally stays at 54/100 until actual milestone acceptance.

**v0.7.5 audio-check fix:** The previous first-sample probe could cause
`ffmpeg` to exit with SIGPIPE (224) under strict shell `pipefail` because
`od -N2` prematurely closed the decoder's pipe. The checker now decodes a
bounded 20 ms segment to a cleaned-up temporary file, checks the actual
`ffmpeg` exit status and still verifies the first PCM sample is zero. The
sound asset itself is unchanged.

## v0.7.6 — 2026-11-04 (Rosa NPC artwork)

- Replaced Rosa's flat-v06 NPC figure with a high-resolution, transparent,
  anime-style full-body illustration retaining her braided silver hair,
  purple floral blouse and cream apron. The installed sprite uses the same
  existing manifest path and authored gameplay scale; no world/story behavior
  was changed.
- Preserved the PNG source under `sources/characters/` and added a deterministic
  ImageMagick rebuild script, source-to-sprite pixel check and alpha coverage QA.
- Regenerated the explicitly static street composition from the **actual**
  installed NPC assets. This preview is not a Bevy frame and has no final owner
  acceptance; the avatar and shop compositing remain open production issues.
- The engine's weighted acceptance ledger remains 54/100 until its existing
  criteria are verified, including in-engine art acceptance and physical QA.

## v0.7.7 — 2026-11-04 (full avatar idle/walk art)

- Added a five-pose original anime-style protagonist sheet that retains the
  established dark outerwear, canvas backpack, dark trousers, and pale shoes.
- Exported five frame-distinct transparent PNGs to the **existing** idle and
  four walking asset paths, using equal canvases, consistent anchoring and
  original sheet pose cuts. No changes to exploration speed or movement logic.
- Added a reproducible shell exporter and a fail-closed test that compares
  every rendered frame with a clean, isolated regeneration from its original
  source. Also checks transparency, dimensions, and all four walk frames are
  different. Updated the **static** in-game-scale street composition.
- All characters have materially improved street art, but owner acceptance,
  Bevy rendered animation and native hardware checks remain outstanding.
  Formal roadmap acceptance remains unchanged at 54/100.

## v0.7.8 — 2026-11-06 (Spanish engine-menu integration)

- Apply the generic Vesper `[menu_labels]` authoring feature merged in `562cfd43` to all three original-game menus: main, pause, and ending titles, action labels and controls.
- Add `qa/check-menu-labels.sh` and include it in both the package QA and the repository narrative CI gate. Existing engine menu actions and keyboard mappings are unchanged.
- Update native verification records: v0.7.7's full three-ending Bevy playthrough passed in run `41941262151`, whereas this new v0.7.8 theme revision has not yet passed its own native run.
- Remaining release blockers: shop artwork layering, temporal transition presentation, physical Hyprland/audio and owner aesthetic review. No formal roadmap points awarded.

## Unreleased — persistent privacy after repeat ledger interaction

- Reading Mateo's private ledger records an irreversible privacy breach, even if a later visit leaves the book closed. An earlier respectful choice is correctly revoked by a later intrusion.
- Cover both interaction orders with two additional source-level scenarios: 13/13 scenario routes, 31/31 scenes, 22/22 choice links, 25/25 conditional true/false pairs, and a third negative mutation probe.
- Preserve all existing game endings, authored artwork, and engine behavior. Native Bevy and owner QA remain separate release gates.
