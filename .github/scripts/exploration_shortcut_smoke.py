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
                # Discover both actions at a second authored NPC. The
                # first neighbor conversation set met_neighbor=true, so
                # E can focus the now-visible friend. The authored T
                # target remains neighbor and must never be overwritten
                # by the nearest candidate or teleport the player.
                old = qa.log_text(log).count("Vesper exploration stopped:")
                qa.walk(window, "Right", 1.55)
                x = qa.position_after_stop(log, old)
                for _ in range(10):
                    if 575.0 <= x <= 620.0:
                        break
                    direction = "Left" if x > 620.0 else "Right"
                    duration = max(.12, min(.36, abs(600.0 - x) / 230.0))
                    old = qa.log_text(log).count("Vesper exploration stopped:")
                    qa.walk(window, direction, duration)
                    next_x = qa.position_after_stop(log, old)
                    assert (next_x < x if direction == "Left" else next_x > x), (
                        f"Could not adjust dual-action focus: {x} -> {next_x}"
                    )
                    x = next_x
                assert 575.0 <= x <= 620.0, (
                    f"Could not reach both active interaction choices: {x}"
                )
                qa.scenery_frame(window, "shortcut-dual-prompt.png", 1280, 720, log)
                qa.key(window, "e")
                qa.wait_for_log(
                    log, "Vesper exploration interacting: friend -> friend_chat"
                )
                friend = qa.frame(window, "shortcut-friend-via-e.png", 1280, 720)
                old_cancels = qa.log_text(log).count(
                    "Vesper exploration conversation cancelled"
                )
                qa.key(window, "q")
                qa.wait_for_log(
                    log, "Vesper exploration conversation cancelled", old_cancels
                )

                previous_direct = qa.log_text(log).count(
                    "Vesper exploration direct interaction: neighbor -> neighbor_chat"
                )
                qa.key(window, "t")
                qa.wait_for_log(
                    log, "Vesper exploration direct interaction: neighbor -> neighbor_chat",
                    previous_direct,
                )
                neighbor = qa.frame(window, "shortcut-neighbor-via-t.png", 1280, 720)
                assert qa.changed(friend, neighbor) > 100, (
                    "E and T targeted the same conversation or failed to render"
                )
                old_saves = qa.log_text(log).count("Vesper save written")
                qa.key(window, "F5")
                qa.wait_for_log(log, "Vesper save written", old_saves)
                saved = json.loads(
                    (folder / "saves" / "exploration-schema.json").read_text(
                        encoding="utf-8"
                    )
                )
                return_to = saved["experience"]["mode"]["conversation"]["return_to"]
                assert abs(return_to["x"] - x) < 0.1, (
                    f"Direct shortcut teleported the world: {x} -> {return_to}"
                )
                old_cancels = qa.log_text(log).count(
                    "Vesper exploration conversation cancelled"
                )
                qa.key(window, "q")
                qa.wait_for_log(
                    log, "Vesper exploration conversation cancelled", old_cancels
                )
                assert proc.poll() is None
                print(
                    "NA08 native T direct NPC, E proximity, E+T simultaneous "
                    "distinct conversations, modal saves, Q cancellation "
                    "and preserved return X: passed",
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
