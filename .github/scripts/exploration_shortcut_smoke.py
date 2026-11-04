#!/usr/bin/env python3
"""NA08 native opt-in direct interaction without compulsory walking.

Uses the unchanged Bevy player with a temporary author-owned manifest:
press T while outside any proximity radius, open the real NPC's .vn scene,
save the modal world position, cancel, and continue ordinary walking.
"""
from __future__ import annotations

import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import time

import exploration_walk_smoke as qa

ROOT = Path(__file__).resolve().parents[2]
PLAYER = ROOT / "target/debug/vesper-player"
SOURCE = ROOT / "examples/exploration-schema"
OUT = Path("/tmp/vesper-exploration-shortcut-qa")
OUT.mkdir(parents=True, exist_ok=True)
qa.OUTPUT = OUT


def main() -> None:
    with tempfile.TemporaryDirectory(prefix="vesper-shortcut-") as directory:
        folder = Path(directory)
        game = folder / "game"
        shutil.copytree(SOURCE, game)
        manifest = game / "game.toml"
        original = manifest.read_text(encoding="utf-8")
        anchor = "[exploration.locations.street]\n"
        assert original.count(anchor) == 1
        manifest.write_text(original.replace(
            anchor, anchor + 'quick_interact = "neighbor"\n', 1,
        ), encoding="utf-8")

        env = dict(os.environ, LIBGL_ALWAYS_SOFTWARE="1",
                   VESPER_PREFERENCES_DIR=str(folder / "preferences"),
                   VESPER_SAVE_DIR=str(folder / "saves"))
        log = OUT / "shortcut.log"
        with log.open("w", encoding="utf-8") as stream:
            proc = subprocess.Popen(
                [str(PLAYER), str(game)], env=env,
                stdout=stream, stderr=subprocess.STDOUT,
            )
            try:
                window = qa.window_id()
                qa.wait_for_log(log, "Vesper exploration preview entered: street")
                before = qa.scenery_frame(window, "shortcut-initial.png", 1280, 720, log)
                # Starting X=150; named neighbor X=435, radius=180.
                # E must remain proximity-gated, but the author's T option
                # permits exactly this otherwise unreachable conversation.
                assert "Vesper exploration interacting:" not in qa.log_text(log)
                qa.key(window, "e")
                assert "Vesper exploration interacting:" not in qa.log_text(log), (
                    "Nearby E ignored the authored interaction radius"
                )
                qa.key(window, "t")
                qa.wait_for_log(
                    log, "Vesper exploration direct interaction: neighbor -> neighbor_chat"
                )
                bubble = qa.frame(window, "shortcut-npc-bubble.png", 1280, 720)
                assert qa.changed(before, bubble) > 800, (
                    "Opt-in T shortcut did not render the real scene"
                )
                old = qa.log_text(log).count("Vesper save written")
                qa.key(window, "F5")
                qa.wait_for_log(log, "Vesper save written", old)
                saved = json.loads(
                    (folder / "saves" / "exploration-schema.json").read_text(
                        encoding="utf-8"
                    )
                )
                assert saved["format_version"] == 2
                world = saved["experience"]["mode"]["conversation"]["return_to"]
                assert world["location"] == "street"
                assert abs(world["x"] - 150.0) < 1.0, world
                qa.key(window, "q")
                qa.wait_for_log(log, "Vesper exploration conversation cancelled")
                old = qa.log_text(log).count("Vesper exploration stopped:")
                qa.walk(window, "Right", .15)
                x = qa.position_after_stop(log, old)
                assert 152.0 < x < 220.0, (
                    f"Direct conversation changed authored return position: {x}"
                )
                assert proc.poll() is None
                print(
                    "NA08 native T direct NPC conversation, E proximity, "
                    "modal save, Q cancellation and preserved return X: passed",
                    flush=True,
                )
            finally:
                proc.terminate()
                try:
                    proc.wait(timeout=6)
                except subprocess.TimeoutExpired:
                    proc.kill()
                    proc.wait(timeout=3)


if __name__ == "__main__":
    main()
