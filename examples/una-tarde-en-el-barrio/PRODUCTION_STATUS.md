# Production status — Una Tarde en el Barrio v0.7.8

**ACTIVE DEVELOPMENT. NOT READY FOR HUMAN WHOLE-GAME QA.**

## Historical v0.7.3 development state and inherited native evidence

- 31 scene identifiers, 22 choice edges, 2 explorable locations, 2 reciprocal exits, 10 individually reachable interaction targets, 3 ending scenes.
- **Inherited v0.7.1 evidence only:** independent Rust `vesper check-game` accepted the preceding external package. v0.7.3 has not been checked with that binary.
- **Inherited v0.7.1 evidence only:** actual Bevy player successfully loaded the narrative panel, entered world mode, walked, began Elena's T interaction, saved during dialogue and returned to exploration.
- Real native 1280×720 and 946×511 screenshots verify the new dialogue skin's framing and visible text.
- **Inherited v0.7.1 evidence only:** native tests of the skin build completed the three ending routes (`ending_listen`, `ending_open`, `ending_quiet`) and the postpone/return/re-enter flow with F9 reload of world state.
- **Current v0.7.3:** `qa/check-package.sh` passes story graph, conditional nesting and external resource checks; new narrative edits leave branch destinations unchanged. No new native execution has been claimed.

## Remaining development and acceptance work (original checklist; see updates below)

1. The character sprites and portrait are still simple relative to the richly painted backgrounds. A full matching anime art and animation pass remains essential.
2. Shop art still contains baked-in Mateo; his interaction uses a transparent hitspot. Replace it with production compositing before final visual QA.
3. General gameplay route coverage is incomplete for combinations of Elena/Rosa/Mateo, the old door, ledger, cat and environment stories, despite all major endings passing.
4. The native Spanish walking UI is localized, but core narrative control hints are still in English; address localization before final UI acceptance.
5. Audio assets exist and decode, but physical playback levels, audio-device switching and mix have not been approved.
6. Real RTX/Wayland/Hyprland input, fullscreen and scaling, player comfort, final visual review and user approval remain undone.
7. Game binaries and binary assets are not source-controlled together in GitHub yet; the source package is locally versioned and separately archived.

**Roadmap accounting:** The engine's separately verified 54/100 points are unchanged. They are not a measure of this game's content completion. Original-game milestone points remain unawarded.

## Verified Rust parser (2026-11-04, v0.7.3)

A PR #31 Linux Rust CLI binary accepted the real v0.7.3 external game.
Negative authoring cases failed as expected; see `qa/real-parser-v073.log`.
Native Bevy/Wayland playthrough and appearance approval are still open.

## v0.7.4 art focus

Elena's standing/walking conversation sprite and calm VN portrait now share
one transparent painted character source. Rosa and the playable avatar still
use the prior flat vector assets. Elena's new look has not yet been accepted
by the owner in real gameplay. All milestone acceptance points remain unchanged.

## v0.7.5 source-level QA improvement

Eleven reproducible scenarios now exercise all scene IDs, all authored
choice links, and both true/false paths of every conditional statement in
`story.vn`. A real Bevy run of that v0.7.5 game was not recorded; inherited native game
frames are from v0.7.1. At the time, Rosa, the protagonist and shop composition
remained visually inconsistent with Elena. Later art revisions address the
first two placeholders; this historical paragraph is not current acceptance.
No scope weights or acceptance statuses have changed.

## v0.7.6 Rosa NPC art

Rosa now has a source-preserved detailed anime-style standing sprite with
physical appearance and outfit carried over from her original design; a
source-to-raster verification and static in-game-size composite passed.
**At that v0.7.6 checkpoint the original flat avatar remained**, Mateo's shop background
still bakes in Mateo and the cat, and new Bevy/physical Wayland QA remains
necessary. Existing release weights are unchanged (54/100).

## v0.7.7 playable avatar art

The protagonist's runtime idle and four walking assets now derive from one
original consistent anime-style five-pose sheet and use the established paths
and animation frame rate. Elena/Rosa were already upgraded in earlier rounds.
The street static preview uses current assets, but has **no runtime-verified
animation**. Native Bevy/physical display checks, shop compositing, a full art
pass and owner's acceptance are still release blockers. No new weighted
milestone acceptance points have been awarded.

## Current v0.7.7 blockers

Street art now has compatible illustrated assets for Elena, Rosa and the protagonist. The native visual and physical animation results are not yet proven by in-engine tests; Mateo/cat shop compositing and UI polish are unresolved. The authoring parser and source tests are green but final whole-game owner QA is not authorized. Vesper localization PR #31 merged after full CI at `1752d51d`. The external game package is still outside the GitHub repository and must be integrated before a release. Overall 54/100 formally verified with no points newly awarded.
