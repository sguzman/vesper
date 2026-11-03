#!/usr/bin/env python3
"""NA07: live native walking/bubble/art hot reload and reject unsafe world edits.

Runs the unchanged Bevy binary with --watch against a temporary copy of an
external fixture. Screenshots are real rendered frames, never mocks.
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
OUT = Path("/tmp/vesper-exploration-reload-qa")
OUT.mkdir(parents=True, exist_ok=True)
qa.OUTPUT = OUT


def await_difference(window: str, reference: Path, name: str, minimum: int) -> Path:
    """Wait for Bevy to render the asynchronously loaded new artwork."""
    target = OUT / name
    deadline = time.monotonic() + 15
    difference = 0
    while time.monotonic() < deadline:
        qa.frame(window, name, 1280, 720)
        difference = qa.changed(reference, target)
        if difference >= minimum:
            return target
        time.sleep(.35)
    raise AssertionError(
        f"Native image did not change after valid reload: {name} ({difference} pixels)"
    )


def unchanged(window: str, before: Path, name: str) -> None:
    time.sleep(.6)
    after = qa.frame(window, name, 1280, 720)
    difference = qa.changed(before, after)
    assert difference < 60, f"Rejected world edit changed {difference} rendered pixels"


def main() -> None:
    with tempfile.TemporaryDirectory(prefix="vesper-exploration-reload-") as folder:
        root = Path(folder)
        game = root / "game"
        shutil.copytree(SOURCE, game)
        preferences = root / "prefs"
        preferences.mkdir()
        (preferences / "player-preferences.json").write_text(
            json.dumps({"format_version": 1, "reveal_speed": "instant"}),
            encoding="utf-8",
        )
        env = dict(
            os.environ, LIBGL_ALWAYS_SOFTWARE="1",
            VESPER_PREFERENCES_DIR=str(preferences),
            VESPER_SAVE_DIR=str(root / "saves"),
        )
        manifest = game / "game.toml"
        original = manifest.read_text(encoding="utf-8")
        assert original.count('x = 435.0\ninteract_scene = "neighbor_chat"') == 1
        moved = original.replace(
            'x = 435.0\ninteract_scene = "neighbor_chat"',
            'x = 795.0\ninteract_scene = "neighbor_chat"',
        )
        log = OUT / "reload.log"
        with log.open("w", encoding="utf-8") as stream:
            player = subprocess.Popen(
                [str(PLAYER), str(game), "--watch"], env=env,
                stdout=stream, stderr=subprocess.STDOUT,
            )
            try:
                window = qa.window_id()
                qa.wait_for_log(log, "Vesper exploration preview entered: street")
                first = qa.scenery_frame(
                    window, "reload-before.png", 1280, 720, log
                )

                old_applied = qa.log_text(log).count("Vesper hot reload applied")
                old_areas = qa.log_text(log).count("Vesper exploration area loaded: street")
                manifest.write_text(moved, encoding="utf-8")
                qa.wait_for_log(log, "Vesper hot reload applied", old_applied)
                qa.wait_for_log(log, "Vesper exploration area loaded: street", old_areas)
                shifted = await_difference(
                    window, first, "reload-npc-moved.png", 700
                )
                assert player.poll() is None, "Valid NPC reload stopped player"

                # Change to a different, authored local PNG; source art is
                # recolored without losing its multicolor backdrop structure.
                original_art = game / "assets/backgrounds/sunrise.png"
                alternate = game / "assets/backgrounds/reloaded.png"
                qa.call("convert", str(original_art), "-fill", "#3457a0",
                        "-colorize", "45%", str(alternate))
                assert moved.count('background = "assets/backgrounds/sunrise.png"') == 2
                art_changed = moved.replace(
                    'background = "assets/backgrounds/sunrise.png"',
                    'background = "assets/backgrounds/reloaded.png"', 1,
                )
                old_applied = qa.log_text(log).count("Vesper hot reload applied")
                manifest.write_text(art_changed, encoding="utf-8")
                qa.wait_for_log(log, "Vesper hot reload applied", old_applied)
                art = await_difference(
                    window, shifted, "reload-art-changed.png", 3000
                )

                # Approach the relocated NPC and verify a bubble style change
                # while its modal narrative conversation is already active.
                old_stops = qa.log_text(log).count("Vesper exploration stopped:")
                qa.walk(window, "Right", 2.4)
                reached = qa.position_after_stop(log, old_stops)
                assert 610 < reached < 900, reached
                qa.key(window, "e")
                qa.wait_for_log(
                    log, "Vesper exploration interacting: neighbor -> neighbor_chat"
                )
                time.sleep(.8)
                bubble_before = qa.frame(
                    window, "reload-bubble-before.png", 1280, 720
                )
                bubble_style = (
                    art_changed + "\n[dialogue.bubble]\n"
                    "background = [0.15, 0.28, 0.68, 0.98]\n"
                )
                old_applied = qa.log_text(log).count("Vesper hot reload applied")
                manifest.write_text(bubble_style, encoding="utf-8")
                qa.wait_for_log(log, "Vesper hot reload applied", old_applied)
                bubble_after = await_difference(
                    window, bubble_before, "reload-bubble-after.png", 300
                )

                # Invalid scene target is rejected before any world or bubble
                # state is replaced, even during an in-progress conversation.
                rejected = bubble_style.replace(
                    'interact_scene = "neighbor_chat"',
                    'interact_scene = "missing_scene"', 1,
                )
                old_rejected = qa.log_text(log).count("Vesper hot reload rejected:")
                manifest.write_text(rejected, encoding="utf-8")
                qa.wait_for_log(log, "Vesper hot reload rejected:", old_rejected)
                unchanged(window, bubble_after, "reload-invalid-scene.png")
                qa.key(window, "q")
                qa.wait_for_log(log, "Vesper exploration conversation cancelled")

                # Author schema remains valid, but removing the portion of the
                # street where the player is standing must be rejected by the
                # *live state* validator, not accepted as an implicit teleport.
                old_stops = qa.log_text(log).count("Vesper exploration stopped:")
                qa.walk(window, "Right", 4.5)
                current = qa.position_after_stop(log, old_stops)
                assert current > 950, current
                at_edge = qa.frame(window, "reload-before-bounds.png", 1280, 720)
                shortened = bubble_style.replace(
                    "width = 1280.0", "width = 900.0", 1,
                ).replace("x = 1170.0", "x = 850.0", 1)
                assert shortened != bubble_style
                old_rejected = qa.log_text(log).count(
                    "Vesper hot reload rejected: saved exploration position"
                )
                manifest.write_text(shortened, encoding="utf-8")
                qa.wait_for_log(
                    log, "Vesper hot reload rejected: saved exploration position",
                    old_rejected,
                )
                unchanged(window, at_edge, "reload-invalid-bounds.png")

                # Recover from both invalid edits, keeping the avatar's
                # current X and allowing subsequent movement normally.
                old_applied = qa.log_text(log).count("Vesper hot reload applied")
                manifest.write_text(bubble_style, encoding="utf-8")
                qa.wait_for_log(log, "Vesper hot reload applied", old_applied)
                old_stops = qa.log_text(log).count("Vesper exploration stopped:")
                qa.walk(window, "Left", .4)
                resumed = qa.position_after_stop(log, old_stops)
                assert resumed < current - 5 and resumed > current - 100, (
                    current, resumed
                )
                qa.frame(window, "reload-recovered.png", 1280, 720)
                assert player.poll() is None, "Exploration hot reload crashed"
                print(
                    "NA07 native reload: NPC, background, bubble, invalid scene, "
                    "active-world bounds rejection, state-preserving recovery PASS",
                    flush=True,
                )
            finally:
                player.terminate()
                try:
                    player.wait(timeout=6)
                except subprocess.TimeoutExpired:
                    player.kill()
                    player.wait(timeout=3)


if __name__ == "__main__":
    main()
