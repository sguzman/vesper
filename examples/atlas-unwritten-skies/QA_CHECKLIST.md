# The Atlas of Unwritten Skies — physical QA

Run from the Vesper repository root (the game is checked in under `examples/`):

```fish
cargo run --locked -p vesper-cli -- check-game examples/atlas-unwritten-skies
cargo run --locked -p vesper-player -- examples/atlas-unwritten-skies
```

- [ ] **Load and start.** The observatory appears, two portraits can stage on left/right, the manuscript panel is readable, and narrative progresses with Enter.
- [ ] **Fonts and accessibility.** P → 4 → font browser accepts a query; choose a font; test high contrast and reduced motion. No clipped text or unclickable menu.
- [ ] **Resizing.** At 946×511, 1280×720 and ~1900×1035 the manuscript panel, menu and choice rows remain usable. Prose is paginated; no single dialogue page should occupy half the viewport.
- [ ] **Art and motion.** Observe sunset/storm/night/dawn backgrounds, ambient blue light in the prologue, character entry/exit and smooth crossfade. Reduced motion should freeze optional animation.
- [ ] **Branching and persistence.** Complete one route, save with F5, reload with F9, and inspect H backlog. Choices in the courier, atlas, record, city, and endgame chapters should work.
- [ ] **Audio.** Confirm the atmospheric score plays on your speakers and page/bell sounds fire. Avoid confusing no soundtrack with muted OS audio.
- [ ] **Four conclusions.** Replay the final choice for The Open Sky, The Living Witness, The Solstice Covenant and The Unbound World. Each should lead to a narrative epilogue and game ending, not a missing scene.
- [ ] **No new engine changes.** Run one earlier example (Living Journal or Dawn Post Office) to ensure the packaged game has not modified or replaced any engine files.

Report exact scene, screenshot, console/log fragment and resolution for failures. Do not mark a test passed merely because the offline validator passed.
