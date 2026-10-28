# Player preferences — VX01 foundation

Player-owned appearance overrides are distinct from external game themes and story saves. The file location on Linux is `$XDG_CONFIG_HOME/vesper/player-preferences.json` (or `~/.config/vesper/player-preferences.json`). For a portable/testing profile, `VESPER_PREFERENCES_DIR` points at the directory containing `player-preferences.json`.

The JSON format is strictly validated and versioned. All fields other than `format_version` are optional; their absence means **follow the game's externally authored theme**:

```json
{
  "format_version": 1,
  "panel_opacity": 0.8,
  "text_scale": 1.2,
  "font_family": "monospace"
}
```

Opacity is constrained to 0.60–1.00, text scale to 0.80–1.60. `font_family` must be a safe, nonempty, bounded family name; for the initial foundation, generic `sans-serif`, `serif`, `monospace` and the game's authored family are accepted at runtime. Unavailable user fonts fall back to the author's selection. VX02 will expose the font picker and settings menu, with additional installed choices.

Preference load happens **before the Bevy frame loop**. Future settings edits will use the provided bounded atomic JSON writer from a worker, never from a Bevy UI/render system. No preferences are written merely by launching a game. Missing/corrupt files give safe defaults without overwriting the original; legacy pre-versioned JSON (V0) is read as V1 and upgraded on the next explicit write. The `reset()` API clears overrides to author defaults without editing `game.toml` or a theme file.

The engine computes effective styling using **player override > authored theme > engine default**, with clamped font sizes for legibility. The underlying `Game.theme` remains unchanged, so compatible `--watch` theme updates still affect all unoverridden fields. No preference values enter narrative `SaveData`.

`cargo test --locked -p vesper-game` covers atomic round-trips, validation/fallback, migration, game switching, font fallback, reset, precedence and failure safety. GitHub Actions runs `.github/scripts/preferences_smoke.py` with both fairy tales under Xvfb, comparing actual rendered baseline/override/corrupt/missing-font/legacy frames. The settings controls and user-initiated save operation belong to **VX02**, not VX01.
