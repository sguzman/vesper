# Validation status — v0.7.8 (current native game and distribution verified; owner acceptance pending)

## Current accepted development evidence (2026-11-08)

- **PR #45 — complete narrative + Bevy CI [42262341196](https://github.com/sguzman/vesper/actions/runs/42262341196): green.** The original v0.7.8 game actually played through both locations, privacy-preserving ledger read/close and F5/F9 restoration, rendered four-choice noticeboard, postpone/revisit and all three endings. Xvfb screenshot evidence confirmed settled **946×511, 1280×720 and 1600×900** Bevy frames; the earlier stale background/resize snapshot was an automated-capture timing bug.
- **PR #46 — complete CI [42265390551](https://github.com/sguzman/vesper/actions/runs/42265390551): green.** The optimized x86_64 Linux development archive was extracted, checked using SHA-256 and booted in a fresh virtual display, reaching real original-game narration. The optimized `.tar.gz` is approximately 67 MiB, includes executable/game/assets/license/launcher and is not a final release.
- **PR #47 — complete CI [42267394865](https://github.com/sguzman/vesper/actions/runs/42267394865): green.** Native game player walked to and completed the old doorway, cat, merchandise display and Rosa authored conversations, then verified `remembered_door`, `stroked_cat`, `read_display` and `heard_rosa` in F5 save state; retained ledger/privacy and three-ending checks.
- **Merged main:** #45 `672dbb15`, #46 `430b31f5`, #47 `c508246e`. Post-merge main CI [42490557066](https://github.com/sguzman/vesper/actions/runs/42490557066) was still running when this entry was authored. Do not substitute PR success for a pending post-merge result.
- **Not accepted:** independently layered/repainted Mateo and cat, physical Hyprland/RTX 3090 rendering/fullscreen/keyboard feel, physical audio balance, coherent final anime art polish and owner subjective approval. The project's weighted narrative-adventure progress is **54/100** (engine 54/60, original-game 0/40); passing the development preview tests alone does not satisfy the game's entire acceptance track.

## Historical native and authoring evidence (older checkpoints retained below)

- **v0.7.7 was played end-to-end through all three endings** in actual Bevy 0.20 under Xvfb/Mesa (CI [41941262151](https://github.com/sguzman/vesper/actions/runs/41941262151)). Runtime evidence artifact `vesper-urban-original-gameplay` (ID 13377030788) contains real PNG screenshots and logs.
- Actual tests included two exploration locations and their reciprocal exits, Elena/Mateo conversation shortcuts, real F5/F9 choice and modal saves, noticeboard, and reaching `Output::End` for `ending_listen`, `ending_open` and `ending_quiet`.
- **v0.7.8 authoring delta:** Spanish `[menu_labels]` for main, pause, and ending screens, with a required game-owned check. This depends on generic engine menu-label support merged as `562cfd43`; no game-specific Rust code.
- **At that earlier checkpoint:** v0.7.8 CI was still pending. The newer accepted runs above supersede this historical limitation.
- Open visual issues observed in native screenshots: transient old street background immediately after shop entry, English menu labels before v0.7.8, and Mateo/cat baked into the shop background. No owner physical GPU, Wayland, sound or aesthetic approval.
- Older sections below retain their original historical meaning at the time they were written. The newer v0.7.7 test supersedes any prior statement that only v0.7.1 had native game evidence.


**Prior runtime (v0.7.1 only):** Vesper player built from `qa/urban-authored-world-presentation` (`097c00451d0ae549ec1f467933459b542a943d1b`), Linux Xvfb + Mesa llvmpipe Vulkan software rendering. This is **not a real Hyprland/RTX 3090 session**.

- `vesper check-game` — passed on **v0.7.1** manifest with new `barrio` nine-slice panel image.
- Native opening and exploration at 1280×720 — passed; F5 dialogue save and Q cancellation verified.
- Native opening and exploration at 946×511 — passed; F5 dialogue save and Q cancellation verified.
- Native ending `ending_listen` — passed, terminal text “FIN — Una invitación.”
- Native ending `ending_open` — passed, terminal text “FIN — Una posibilidad.”
- Native ending `ending_quiet` — passed, terminal text “FIN — Una puerta abierta.”
- Native postpone option — passed; returned to exploration, F9 restored world coordinate, revisited board and reached the open ending.
- `qa/validate_package.py`, `qa/route_matrix.py`, `qa/world_focus_checks.py` — passed, but they are additional source-level checks, not runtime evidence.

**Evidence:** `qa/native/intro-1.png`, `qa/compact-946/intro-1.png` and `qa/native-endings-evidence/*` contain actual rendered frames (the development ZIP separates screenshot evidence from shipping game assets). `qa/native-endings-evidence/report.json` records each run. A real GPU and physical audio device have not been tested. Do not mark owner QA as completed.

**Coverage caveat:** Native ending frames were captured before Rosa's prose-only consistency edit. The story parser and static route tests were rerun after that edit, and no choices or destinations changed. This does not establish a full rerecording of every character-interaction path after the edit.

## v0.7.2 verification update

- `./qa/check-package.sh` — passed for edited `game.toml`, story graph, scene termination, world exit/spawn links and current asset references; also rejected intentionally broken choice target and spawn in isolated negative QA probes; this is *static authoring QA only*.
- Narrative continuity changes preserve all scene identifiers and choice destinations, but alter conditional lines in two endings.
- The v0.7.1 real parser and Bevy footage do **not** prove the v0.7.2 code was parsed or played. A compatible `vesper check-game` CLI, native rerun, and physical owner acceptance remain explicit gates.
- No percent points awarded. The repository's 54/100 includes 54 engine-track points; the game's separate 40-point track is still 0.

## v0.7.3 authored features (later engine PR merged)

- The reader-hint localization implementation was developed on PR #31 and merged
  into Vesper `main` at `1752d51d` after narrative and Bevy CI passed. No native
  v0.7.7 **game** evidence is established by those engine fixture tests.
- The paper-rustle onset has a 12 ms PCM fade; a technical audio regression
  checks format and peak levels, but cannot certify subjective mix/loudness.
- All inherited native screenshots/ending playthroughs are from v0.7.1 only.
- The automatically generated project-dashboard image from the development
  conversation is **not** game artwork, is excluded from the package, and
  makes no claims about this package's actual content.

## v0.7.3 Rust parser acceptance (new, verified)

The actual current Rust `vesper check-game` compiled from commit
`ffea7f3cf2a97b88f639e30da409e21612ddb1ff` in PR #31 accepted
**v0.7.3**, including the new `[reader_hints]` theme fields. It also accepted
v0.7.2 unchanged. Both negative tests rejected an empty authored label and
an unknown label field. Full command results, artifact ID and executable
SHA-256 are recorded in `qa/real-parser-v073.log`.

This is stronger evidence than static source simulation, but **does not**
certify a Bevy playthrough or an actual graphics/audio session. Physical and
final aesthetic owner acceptance remain pending.

## v0.7.4 provisional Elena sprite integration

- Replaced the previous flat-vector Elena stand-in and her unrelated provisional
  portrait with transparent, consistent artwork derived from a single new
  full-body original source. PNG alpha/decoding/resolution checks pass.
- This revision **has not been tested with a live Bevy 0.20 player**. The existing
  v0.7.1 screenshots are historical evidence only. The 54/100 weighted formal
  acceptance remains unchanged; Rosa, protagonist, shop compositing and native
  hardware QA still block approval.

**v0.7.4 real CLI rerun:** `vesper check-game` completed successfully with
PR #31's compiled Rust CLI, as recorded in `qa/real-parser-v074.log`.
`qa/street-static-art-composite.png` is a hand-composited reference showing
external asset placement and actual relative sprite size; it is **not** a live
Bevy screenshot, does not exercise bubble/panel occlusion, and is not owner QA.

## v0.7.5 independent scenario-coverage pass

`./qa/check-scenarios.sh` passed eleven real-script scenario cases:
**31/31** scenes entered, **22/22** authored choice edges selected,
**24/24** conditional lines exercised in both true and false states, and
**2/2** targeted negative mutation checks rejected as expected. These checks
are now part of `qa/check-package.sh`. They model a deliberately small subset
of the `.vn` grammar and do not constitute Vesper interpreter equivalence,
real save/load, GUI, audio or aesthetic acceptance.

**v0.7.5 Rust CLI:** The PR #31 compiled authoring validator accepted the
complete new archive input; see `qa/real-parser-v075.log`. Native Bevy QA
still pending. A shell pipeline SIGPIPE bug in the paper-rustle sample probe
was repaired, and the comprehensive package checker now exits cleanly.

## v0.7.6 Rosa production-art checkpoint

The new Rosa PNG passes source reproduction, image decoding, transparency,
resolution, and small-size static-composite checks. The updated v0.7.6 game
has **not** been tested with the Bevy player on graphics hardware; do not
reuse v0.7.1 runtime screenshots as evidence for this version. There is no
owner visual approval yet. The executable game schema and story branch graph
remain unchanged; the original game is still not ready for final human QA.

## v0.7.7 protagonist cycle source verification

All five avatar PNGs have common 600x754 geometry, transparent alpha,
identifiable distinct pose pixels, and exact reproducible export from the
preserved five-pose original. Source package tests and real CLI parser checks
are independent of the rendered Bevy animation. **Not yet verified**:
actual left/right flipping in the player's current build, step-frame cadence,
foot alignment in the live world, focus transitions and physical Hyprland QA.

## v0.7.7 verification update

- Updated five original hero sprite frames: all have common geometry, exact source regeneration and distinct pose pixels. Real compiled Rust `vesper check-game` accepts v0.7.7.
- Generic localization PR #31 merged as `1752d51d` after both narrative/Bevy workflow jobs passed in run 41906180461; that is **engine CI**, not a native playthrough of this independent game.
- The 11 real-script scenario routes and both mutation-negative controls still pass.
- The street composition and avatar contact sheet are STATIC ASSET PREVIEWS only. In-game 8 FPS cycling, actual foot alignment/hand motion, loading F5/F9 in v0.7.7, Wayland/GPU and owner visual acceptance are not yet verified.
