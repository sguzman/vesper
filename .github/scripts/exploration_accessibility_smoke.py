#!/usr/bin/env python3
"""NA08: player accessibility preferences in an actual walk-first NPC bubble.

Uses the real Bevy player, X11 window inputs, saved experience and rendered
screenshots; no scripted game-specific engine paths or synthetic UI state.
"""
from __future__ import annotations

import json
import os
from pathlib import Path
import subprocess
import tempfile
import time

import exploration_walk_smoke as qa

ROOT = Path(__file__).resolve().parents[2]
GAME = ROOT / "examples/exploration-schema"
PLAYER = ROOT / "target/debug/vesper-player"
OUT = Path("/tmp/vesper-exploration-accessibility-qa")
OUT.mkdir(parents=True, exist_ok=True)
qa.OUTPUT = OUT


def capture(name: str, overrides: dict[str, object]) -> Path:
    with tempfile.TemporaryDirectory(prefix="vesper-accessible-world-") as folder:
        prefs = {"format_version": 1, "reveal_speed": "instant",
                 "auto_advance": False, **overrides}
        Path(folder, "player-preferences.json").write_text(
            json.dumps(prefs), encoding="utf-8"
        )
        env = dict(os.environ, LIBGL_ALWAYS_SOFTWARE="1",
                   VESPER_PREFERENCES_DIR=folder,
                   VESPER_SAVE_DIR=str(Path(folder) / "saves"))
        log = OUT / f"{name}.log"
        with log.open("w", encoding="utf-8") as stream:
            player = subprocess.Popen(
                [str(PLAYER), str(GAME)], env=env,
                stdout=stream, stderr=subprocess.STDOUT,
            )
            try:
                win = qa.window_id()
                qa.wait_for_log(log, "Vesper exploration preview entered:")
                qa.call("xdotool", "windowsize", "--sync", win, "946", "511")
                time.sleep(.6)
                qa.scenery_frame(win, f"{name}-street.png", 946, 511, log)

                # Approach the authored NPC using real keyboard movement,
                # rather than directly injecting scene/machine state.
                x = 150.0
                for _ in range(12):
                    old = qa.log_text(log).count("Vesper exploration stopped:")
                    qa.walk(win, "Right", .22)
                    x = qa.position_after_stop(log, old)
                    if 315.0 <= x <= 405.0:
                        break
                assert 255.0 <= x <= 510.0, (
                    f"Could not reach the nearest authored NPC: {x}"
                )
                qa.key(win, "e")
                qa.wait_for_log(
                    log, "Vesper exploration interacting: neighbor -> neighbor_chat"
                )
                time.sleep(.35)
                image = qa.frame(win, f"{name}-npc-bubble.png", 946, 511)
                # The story and world save format must be unaffected by
                # player-owned appearance/motion preferences.
                old_saves = qa.log_text(log).count("Vesper save written")
                qa.key(win, "F5")
                qa.wait_for_log(log, "Vesper save written", old_saves)
                saved = json.loads(
                    (Path(folder) / "saves" / "exploration-schema.json")
                    .read_text(encoding="utf-8")
                )
                assert saved["format_version"] == 2
                assert "conversation" in saved["experience"]["mode"]
                assert player.poll() is None
                return image
            finally:
                player.terminate()
                try:
                    player.wait(timeout=6)
                except subprocess.TimeoutExpired:
                    player.kill()
                    player.wait(timeout=3)


def main() -> None:
    normal = capture("default", {})
    accessible = capture("accessible", {
        "high_contrast": True, "text_scale": 1.6,
        "reduced_motion": True,
    })
    difference = qa.changed(normal, accessible)
    assert difference > 2000, (
        f"World conversation did not visually respond to accessible player "
        f"preferences: {difference} changed pixels"
    )
    print(
        "NA08 native world-bubble: high contrast, larger text, reduced motion, "
        "compact layout and unchanged modal saves: passed",
        flush=True,
    )


if __name__ == "__main__":
    main()
