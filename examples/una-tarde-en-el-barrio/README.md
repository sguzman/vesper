# Una Tarde en el Barrio

**v0.7.8 development game · original external content for Vesper (Rust / Bevy 0.20)**

A short, Spanish-language urban slice-of-life story set around a neighborhood shop in Guadalajara. Explore an illustrated street and Mateo's store, speak with Elena, Rosa and Mateo, inspect small details, then decide what the neighborhood noticeboard should say. Three story endings reflect decisions and optional conversations. Walking is deliberately optional in some conversations via Vesper's author-defined `T` shortcut.

This is a **playable development package, not an accepted final game**. It is external data, not part of the Vesper binary or engine test fixtures. The story premise and art remain provisional until the owner approves them.

## Requirements and launch

Use a Vesper player built from merged `main` commit [`562cfd43`](https://github.com/sguzman/vesper/commit/562cfd43a1aa72d879eeabbfa88828a7683e337a) or a compatible descendant. The game opts into Spanish `[reader_hints]` and `[menu_labels]`; older player builds do not recognize all these theme keys. The game is an external directory and **does not contain the player executable**. Use the same commands with an absolute path if you are running an extracted development ZIP outside the Vesper repository.

From a compatible Vesper workspace, after building using the machine's normal Rust build coordinator:

```sh
./target/debug/vesper-player examples/una-tarde-en-el-barrio
./target/debug/vesper check-game examples/una-tarde-en-el-barrio
```

The second command validates external game authoring; it does not run a playable window. A compiler-free compatible Vesper player can instead be used with this directory.

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

The source QA checks story connectivity, asset references/decoding, world links, Spanish reader and menu labels, audio formats/levels, original character source derivations, five protagonist animation frames and thirteen conditional routes. The real Rust `vesper check-game` also ran successfully against v0.7.7 in CI; it must be rerun for v0.7.8. Static scenario tests do **not** replace a real gameplay run.

**The v0.7.7 game passed a new real Bevy/Xvfb full-three-ending CI playthrough** (run [41941262151](https://github.com/sguzman/vesper/actions/runs/41941262151), native artifact `vesper-urban-original-gameplay`). It covered actual movement, two locations, both direct conversations, F5/F9 and all three endings. The v0.7.8 Spanish-menu revision is **not yet runtime verified** until its own CI completes. The previous store-background frame transient, shop compositing, owner Hyprland/RTX testing, physical audio, visual polish and owner acceptance remain open. **Do not request final human QA yet.**

The separately tracked Vesper narrative-adventure roadmap remains **54/100 formally verified** (engine 54/60; example 0/40), unchanged by provisional game art and source-only work.

See `CHANGELOG.md` for the history, `QA_STATUS.md` for evidence boundaries and `PRODUCTION_STATUS.md` for remaining work.
