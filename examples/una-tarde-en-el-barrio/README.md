# Una Tarde en el Barrio

**v0.7.8 development game · original external content for Vesper (Rust / Bevy 0.20)**

A short, Spanish-language urban slice-of-life story set around a neighborhood shop in Guadalajara. Explore an illustrated street and Mateo's store, speak with Elena, Rosa and Mateo, inspect small details, then decide what the neighborhood noticeboard should say. Three story endings reflect decisions and optional conversations. Walking is deliberately optional in some conversations via Vesper's author-defined `T` shortcut.

This is a **playable development package, not an accepted final game**. It is external data, not part of the Vesper binary or engine test fixtures. The story premise and art remain provisional until the owner approves them.

## Requirements and launch

Use a Vesper player built from merged `main` commit [`562cfd43`](https://github.com/sguzman/vesper/commit/562cfd43a1aa72d879eeabbfa88828a7683e337a) or a compatible descendant. The game opts into Spanish `[reader_hints]` and `[menu_labels]`; older player builds do not recognize all these theme keys. The source-checkout game remains external data, but the **Linux x86_64 development-preview archive now includes the actual native player**. The `play.sh` launcher resolves the packaged game path even when launched from another working directory.

From a compatible Vesper workspace, after building using the machine's normal Rust build coordinator:

```sh
./target/debug/vesper-player examples/una-tarde-en-el-barrio
./target/debug/vesper check-game examples/una-tarde-en-el-barrio
```

The second command validates external game authoring; it does not run a playable window. Alternatively, CI now publishes `una-tarde-linux-development-preview`, containing `Una-Tarde-en-el-Barrio-Linux-x86_64.tar.gz` (the executable, complete game, assets, sound, story, MIT license and SHA256 manifest). Extract the tarball and launch `./play.sh` from the extracted directory; neither Cargo nor a source checkout is required. Verify `SHA256SUMS` before running. This is a **development preview**, not a finished or owner-accepted build.

## Controls

Use **A/D** or **Left/Right** to walk; **E** interacts with a nearby person, object or exit. **T** directly opens the designated NPC's conversation without walking to them. **Q** leaves an exploration conversation, **F5/F9** quick-saves/loads, **F6** opens manual slots, **H** opens history, **P** pauses and **M** opens the menu. The fourth noticeboard choice deliberately postpones the decision, allowing you to return later. The Spanish footer instructions in `styles/urban.toml` belong to the game; key bindings belong to Vesper.

## Content and art

- **Two locations:** the street and the illustrated shop; **three endings** and **four noticeboard outcomes** (including postpone)
- **31 story scenes, 22 choice links** and 13 reproducible conditional-route tests
- **Character art:** Elena and Rosa now use original detailed, transparent full-body sprites; Elena's portrait derives from her same source. The protagonist now uses one original five-pose sprite sheet, exported as a coherent idle frame and four distinct walking frames. Mateo and the cat are painted into the shop background with invisible interaction hitspots; they still need independent production compositing
- **Audio:** two original synthesized ambient beds and two original one-shots. The paper-rustle onset has a reproducible 12 ms fade; physical playback and final mix approval remain open
- Art sources are preserved in `sources/`; generated-asset provenance and reproducible transforms are recorded in `ASSET_PROVENANCE.md` and `tools/`

Earlier development packages supplied a **static art composite** for inspection, not a Bevy frame. The real Bevy screenshots from the v0.7.7 native CI run are retained separately as GitHub Actions evidence.

## Verification and limitations

Run the complete, dependency-aware source checks with:

```sh
./qa/check-package.sh
```

The source QA checks story connectivity, asset references/decoding, world links, Spanish reader and menu labels, audio formats/levels, original character source derivations, five protagonist animation frames and thirteen conditional routes. The current v0.7.8 original game passed the Rust `vesper check-game`, narrative authoring checks and actual native Bevy gameplay in CI. Static scenario tests do **not** replace a real gameplay run.

**Current merged code:** CI [42262341196](https://github.com/sguzman/vesper/actions/runs/42262341196) passed the complete narrative + Bevy suite for the ledger/privacy, noticeboard-postpone and responsive frame integration (merged PR #45). CI [42265390551](https://github.com/sguzman/vesper/actions/runs/42265390551) passed the native three endings, stripped Linux archive boot and full Bevy suite (merged PR #46). CI [42267394865](https://github.com/sguzman/vesper/actions/runs/42267394865) passed the full suite and native optional Rosa/old-door/cat/display interactions with real F5 state checks (merged PR #47). The earlier transient store-background screenshot was a premature capture and was fixed in QA by waiting for the actual painted frame; this does not certify all hardware transitions. The new post-merge `main` run [42490557066](https://github.com/sguzman/vesper/actions/runs/42490557066) is underway at this checkpoint, so do not call that run green yet. Independent Mateo/cat shop art, physical Hyprland/RTX testing, real audio listening, final visual polish and owner approval remain open. **Do not request final whole-game QA yet.**

The separately tracked Vesper narrative-adventure roadmap remains **54/100 formally verified** (engine 54/60; example 0/40), unchanged by provisional game art and source-only work.

See `CHANGELOG.md` for the history, `QA_STATUS.md` for evidence boundaries and `PRODUCTION_STATUS.md` for remaining work.
