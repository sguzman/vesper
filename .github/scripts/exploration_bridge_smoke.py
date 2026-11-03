#!/usr/bin/env python3
"""Exercise explicit author-driven VN -> world -> VN -> world mode changes."""
from __future__ import annotations

import os
from pathlib import Path
import subprocess
import tempfile
import time

import exploration_walk_smoke as qa

ROOT = Path(__file__).resolve().parents[2]
GAME = ROOT / "examples/exploration-bridge"
PLAYER = ROOT / "target/debug/vesper-player"
qa.OUTPUT = Path("/tmp/vesper-exploration-bridge-qa")
qa.OUTPUT.mkdir(parents=True, exist_ok=True)


def window_id() -> str:
    deadline = time.monotonic() + 35
    while time.monotonic() < deadline:
        result = qa.call(
            "xdotool", "search", "--name", "Optional Cinematic To Walkable Scene",
            check=False,
        )
        if result.returncode == 0 and result.stdout.strip():
            return result.stdout.splitlines()[0]
        time.sleep(.2)
    raise AssertionError("The VN-first bridge game never opened")


def main() -> None:
    with tempfile.TemporaryDirectory(prefix="vesper-bridge-") as folder:
        env = dict(os.environ,
                   LIBGL_ALWAYS_SOFTWARE="1",
                   VESPER_PREFERENCES_DIR=folder,
                   VESPER_SAVE_DIR=str(Path(folder) / "saves"))
        log = qa.OUTPUT / "bridge.log"
        with log.open("w", encoding="utf-8") as stream:
            proc = subprocess.Popen(
                [str(PLAYER), str(GAME)],
                env=env, stdout=stream, stderr=subprocess.STDOUT,
            )
            try:
                window = window_id()
                time.sleep(1.1)
                initial_vn = qa.frame(window, "initial-vn.png", 1280, 720)
                assert "Vesper exploration preview entered" not in qa.log_text(log), (
                    "The no-start manifest incorrectly entered walking automatically"
                )

                # A source-authored transition follows the VN opening line.
                for _ in range(5):
                    qa.key(window, "Return")
                    if "Vesper exploration area loaded: street" in qa.log_text(log):
                        break
                qa.wait_for_log(log, "Vesper exploration area loaded: street")
                world = qa.scenery_frame(window, "street-entered.png", 1280, 720, log)
                assert qa.changed(initial_vn, world) > 800, (
                    "Author command failed to replace VN staging with the walkable street"
                )

                # Enter the neighbor's choice-driven scene and choose cinematic
                # VN instead of the direct world-return option.
                qa.walk(window, "Right", .65)
                qa.key(window, "e")
                qa.wait_for_log(log, "Vesper exploration interacting: neighbor -> neighbor_chat")
                qa.key(window, "Return")
                qa.key(window, "Return")
                qa.key(window, "2")
                for _ in range(4):
                    if "Vesper explicitly entered visual novel mode" in qa.log_text(log):
                        break
                    qa.key(window, "Return")
                qa.wait_for_log(log, "Vesper explicitly entered visual novel mode")
                qa.wait_for_log(log, "Vesper exploration area released for cinematic VN")
                cinematic = qa.frame(window, "cinematic-vn.png", 1280, 720)
                assert qa.changed(cinematic, world) > 800, (
                    "Explicit full VN mode did not replace walking presentation"
                )

                # The same .vn script deliberately enters the cafe directly.
                # There is no enforced walk back across the street.
                for _ in range(8):
                    qa.key(window, "Return")
                    if "Vesper exploration area loaded: cafe" in qa.log_text(log):
                        break
                qa.wait_for_log(log, "Vesper exploration area loaded: cafe")
                cafe = qa.scenery_frame(window, "cafe-from-vn.png", 1280, 720, log)
                assert qa.changed(cafe, cinematic) > 800, (
                    "Explicit VN-to-cafe command did not restore world rendering"
                )
                assert "Vesper authored exploration entered at named spawn entry" in qa.log_text(log)
                print("VN-only opening, story-authored walk, cinematic VN and direct cafe entry: passed",
                      flush=True)
            finally:
                proc.terminate()
                try:
                    proc.wait(timeout=6)
                except subprocess.TimeoutExpired:
                    proc.kill()
                    proc.wait(timeout=3)


if __name__ == "__main__":
    main()
