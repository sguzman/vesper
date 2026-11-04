# Una Tarde en el Barrio

**v0.7.7 development game · original external content for Vesper (Rust / Bevy 0.20)**

A short, Spanish-language urban slice-of-life story set around a neighborhood shop in Guadalajara. Explore an illustrated street and Mateo's store, speak with Elena, Rosa and Mateo, inspect small details, then decide what the neighborhood noticeboard should say. Three story endings reflect decisions and optional conversations. Walking is deliberately optional in some conversations via Vesper's author-defined `T` shortcut.

This is a **playable development package, not an accepted final game**. It is external data, not part of the Vesper binary or engine test fixtures. The story premise and art remain provisional until the owner approves them.

## Requirements and launch

Use a Vesper player built from merged `main` commit [`1752d51d`](https://github.com/sguzman/vesper/commit/1752d51d346f86378fdb7b6eeca9dbcf4f79603b) or a compatible descendant. The package uses Spanish exploration labels and the now-merged `[reader_hints]` theme keys. Older binaries from before PR #31 reject these fields. The game is an external directory and **does not contain the player executable**. Use the same commands with an absolute path if you are running an extracted development ZIP outside the Vesper repository.

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
- **31 story scenes, 22 choice links** and 11 reproducible conditional-route tests
- **Character art:** Elena and Rosa now use original detailed, transparent full-body sprites; Elena's portrait derives from her same source. The protagonist now uses one original five-pose sprite sheet, exported as a coherent idle frame and four distinct walking frames. Mateo and the cat are painted into the shop background with invisible interaction hitspots; they still need independent production compositing
- **Audio:** two original synthesized ambient beds and two original one-shots. The paper-rustle onset has a reproducible 12 ms fade; physical playback and final mix approval remain open
- Art sources are preserved in `sources/`; generated-asset provenance and reproducible transforms are recorded in `ASSET_PROVENANCE.md` and `tools/`

`qa/street-static-art-composite.png` is an **asset composite at intended sprite heights**, not a frame captured from Bevy. Do not mistake it for visual/gameplay QA.

## Verification and limitations

Run the complete, dependency-aware source checks with:

```sh
./qa/check-package.sh
```

The suite checks story connectivity, asset references/decoding, world links, author-provided Spanish hints, audio formats/levels, Elena/Rosa source derivations and all five protagonist sprite exports, and all eleven conditional routes. The original compiled Vesper Rust CLI also passed `check-game` on v0.7.7 (see `qa/real-parser-v077.log`). Its static scenario simulator is **not** the runtime interpreter; neither it nor the authoring CLI constitutes a graphical playthrough.

**Actual Bevy/Xvfb software-rendered game playthrough evidence applies only to v0.7.1.** This version has not been played end-to-end in the Bevy player, nor on the owner's physical Hyprland/RTX setup. The localization PR's engine regression CI is independent of this game's acceptance. Final interactive, aesthetic, physical-audio, accessibility and release tests remain undone. Until the owner approves its appearance and gameplay, do **not** request final human QA.

The separately tracked Vesper narrative-adventure roadmap remains **54/100 formally verified** (engine 54/60; example 0/40), unchanged by provisional game art and source-only work.

See `CHANGELOG.md` for the history, `QA_STATUS.md` for evidence boundaries and `PRODUCTION_STATUS.md` for remaining work.
