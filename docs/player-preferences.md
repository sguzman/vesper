# Player preferences — VX01 foundation, VX02 live Settings and VX04 reading flow

Player-owned appearance overrides are distinct from external game themes and story saves. The file location on Linux is `$XDG_CONFIG_HOME/vesper/player-preferences.json` (or `~/.config/vesper/player-preferences.json`). For a portable/testing profile, `VESPER_PREFERENCES_DIR` points at the directory containing `player-preferences.json`.

The JSON format is strictly validated and versioned. All fields other than `format_version` are optional; their absence means **follow the game's externally authored theme**:

```json
{
  "format_version": 1,
  "panel_opacity": 0.8,
  "text_scale": 1.2,
  "font_family": "monospace",
  "reveal_speed": "normal",
  "reduced_motion": false,
  "auto_advance": false
}
```

Opacity is constrained to 0.60–1.00, text scale to 0.80–1.60. `font_family` must be a safe, nonempty, bounded family name; for the initial foundation, generic `sans-serif`, `serif`, `monospace` and the game's authored family are accepted at runtime. Unavailable user fonts fall back to the author's selection. VX02 provides a font selector for the three generic families and any game-approved authored family, with safe fallback for unavailable fonts.

Preference load happens **before the Bevy frame loop**. User-initiated Settings edits use a sequential background worker and bounded atomic JSON writer, never a Bevy UI/render system. No preferences are written merely by launching a game. Missing/corrupt files give safe defaults without overwriting the original; legacy pre-versioned JSON (V0) is read as V1 and upgraded on the next explicit write. The `reset()` API clears overrides to author defaults without editing `game.toml` or a theme file.

The engine computes effective styling using **player override > authored theme > engine default**, with clamped font sizes for legibility. The underlying `Game.theme` remains unchanged, so compatible `--watch` theme updates still affect all unoverridden fields. No preference values enter narrative `SaveData`.

`cargo test --locked -p vesper-game` covers atomic round-trips, validation/fallback, migration, game switching, font fallback, reset, precedence and failure safety. GitHub Actions runs `.github/scripts/preferences_smoke.py` with both fairy tales under Xvfb, comparing actual rendered baseline/override/corrupt/missing-font/legacy frames. The keyboard-first, pointer-operable Settings screen is implemented in **VX02**, and its real rendered/interaction regression is `.github/scripts/settings_smoke.py` (both games, persistence, restart/reset and four viewport sizes), verified in CI run 40721330853.

## Live Settings controls

While playing, press **P** for the pause menu or **M** for the main menu and select **4. Appearance settings**. Use **Up/Down** to select opacity, size, font, reveal speed, reduced motion, auto-advance, Reset or Back; **Left/Right** changes the selected value and **Enter** activates it. Pointer clicks activate the same validated settings actions. **Escape** returns to the previous menu without quitting the game or changing the story position. Every change immediately updates the on-screen dialogue and live preview, and is persisted asynchronously. Reset clears the player overrides, revealing authored game values again. The settings do not modify any authored theme or narrative save file.

## VX04 reading speed, motion and advancement

The same versioned player-preferences file accepts optional `reveal_speed` (`"instant"`, `"slow"`, `"normal"` or `"fast"`), `reduced_motion` (boolean) and `auto_advance` (boolean). Authors can set `reveal_speed = "normal"` at the top level of their external theme TOML. The engine's default remains **Instant** for legacy themes. Player overrides take priority without modifying authored files; the previous preference format remains readable.

With reveal enabled, dialogue is shown in Unicode grapheme clusters, so combining marks and joined emoji are not split. **Enter/Space while text is revealing completes only the current line**; the following separate press advances. Pause and menus suspend the reveal clock. F9, a valid hot reload and restored scenes show the current line fully instead of replaying a partial reveal; resizing retains its progress. A subtle theme-accent continue cue appears after the full line is visible; motion is suppressed by Reduced Motion, and cues in Instant mode are static for backward compatibility. Auto-advance is **disabled by default** and only runs after the line finishes when the player explicitly enables it. Preference writes remain on the sequential background worker.

Evidence: [full VX04 narrative and Bevy Linux CI run 40745061158](https://github.com/sguzman/vesper/actions/runs/40745061158), including `.github/scripts/readout_smoke.py` (rendered timed reveal, skip-once, settings, compact resize and live theme reload), previous Settings and choice tests, all legacy graphical regressions and durable story saves.
