# The Living Journal — all five reader QoL capabilities in one game

This self-contained external game combines all earlier milestones without one game-specific Rust code path. It uses the previously authored notebook/banner art, sunrise painting, bird, plane and Tess sprite frames; art and story are in the game directory. Everything is readable and playable without a network connection.

1. **Installed-font browser** (reader-owned): press `P` → `4` (Appearance), navigate to System Font, search/filter installed families, and choose one. Or provide `font_family` in the player preferences JSON. Authors do not override the reader’s chosen system font.
2. **Auto-advance wait time** (reader-owned): set `auto_advance: true` and `auto_advance_delay: 0.8` in `player-preferences.json` (delay valid range 0.5–20 s), or change Auto Advance/Delay in Appearance.
3. **Illustrated notebook/diary** (author-owned): `panel notebook` followed by `panel banner`; safe validated image paths, nine-slice margins, and high-contrast suppression.
4. **Per-word dynamic flair** (author-owned): `[[fx=ember]]`, `[[fx=shadow]]`, and `[[fx=shimmer]]` mapped to bounded color effects in `styles/journal.toml`. Reduced motion freezes their colors; high contrast uses readable text.
5. **Autonomous scenery** (author-owned): a two-frame walker, a looping bird, and a one-shot plane travel independently of dialogue and each other. The bird can be hidden and shown via story visual commands. Reduced motion freezes all three.

Run:

```sh
cargo run --locked -p vesper-cli -- check-game examples/living-journal
cargo run --locked -p vesper-player -- examples/living-journal
```

Temporary isolated player preferences for local QA (saved under the chosen temp directory):

```json
{"format_version":1,"font_family":"monospace","auto_advance":true,"auto_advance_delay":0.8,"reveal_speed":"instant"}
```

Set `VESPER_PREFERENCES_DIR` to the directory containing that `player-preferences.json` and `VESPER_SAVE_DIR` separately to isolate save files. The engine has no new hard-coded font, scene or asset names.

CI verifies this one game at six viewport sizes, actual rendered word/flair/ambient appearance, font precedence, panel switches, auto-advance, high-contrast and reduced-motion variants and F5/F9. Prior independent games and the Linux bundle have their own complete CI suite. Xvfb is not a substitute for physical Hyprland/Wayland GPU QA.
