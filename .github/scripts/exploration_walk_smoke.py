#!/usr/bin/env python3
"""Run an actual opt-in walk-first Vesper location in the Bevy player.

The fixture only supplies external authoring data. No engine source-path
hardcoding, simulated keyboard state, fake screenshots or parsed game code.
"""
from __future__ import annotations

import os
from pathlib import Path
import re
import subprocess
import tempfile
import time

ROOT = Path(__file__).resolve().parents[2]
PLAYER = ROOT / "target/debug/vesper-player"
GAME = ROOT / "examples/exploration-schema"
OUTPUT = Path("/tmp/vesper-exploration-walk-qa")
OUTPUT.mkdir(parents=True, exist_ok=True)
START = re.compile(r"Vesper exploration preview entered: street at x=([\d.]+)")
STOP = re.compile(r"Vesper exploration stopped: street at x=([\d.]+)")


def call(*args: str, check: bool = True) -> subprocess.CompletedProcess[str]:
    return subprocess.run(args, text=True, capture_output=True, check=check)


def log_text(log: Path) -> str:
    return log.read_text(encoding="utf-8", errors="replace")


def wait_for_log(log: Path, phrase: str, previous: int = 0) -> None:
    deadline = time.monotonic() + 15
    while time.monotonic() < deadline:
        if log.exists() and log_text(log).count(phrase) > previous:
            return
        time.sleep(.1)
    raise AssertionError(f"Expected {phrase!r} after {previous} occurrences:\n{log_text(log)[-3500:]}")


def window_id() -> str:
    deadline = time.monotonic() + 35
    while time.monotonic() < deadline:
        result = call("xdotool", "search", "--name",
                      "Optional Walkable Location Schema", check=False)
        if result.returncode == 0 and result.stdout.strip():
            return result.stdout.splitlines()[0]
        time.sleep(.2)
    raise AssertionError("Walk-first fixture window never opened")


def frame(window: str, filename: str, width: int, height: int) -> Path:
    out = OUTPUT / filename
    for _ in range(30):
        call("import", "-window", window, str(out))
        dims = call("identify", "-format", "%w %h %k", str(out)).stdout
        w, h, colors = [int(value) for value in dims.split()]
        if (w, h) == (width, height) and colors > 80:
            return out
        time.sleep(.25)
    raise AssertionError(f"Rendered frame has wrong dimensions for {filename}")


def changed(left: Path, right: Path) -> int:
    out = call("compare", "-metric", "AE", str(left), str(right),
               "null:", check=False)
    match = re.search(r"^\s*(\d+)", out.stderr)
    assert match, out.stderr
    return int(match[1])


def key(window: str, button: str) -> None:
    call("xdotool", "windowfocus", "--sync", window)
    call("xdotool", "key", "--clearmodifiers", button)
    time.sleep(.45)


def walk(window: str, button: str, seconds: float) -> None:
    call("xdotool", "windowfocus", "--sync", window)
    call("xdotool", "keydown", "--clearmodifiers", button)
    try:
        time.sleep(seconds)
    finally:
        call("xdotool", "keyup", "--clearmodifiers", button)


def position_after_stop(log: Path, old_count: int) -> float:
    wait_for_log(log, "Vesper exploration stopped:", old_count)
    values = [float(x) for x in STOP.findall(log_text(log))]
    assert values, "Missing walk-position log"
    return values[-1]


def main() -> None:
    with tempfile.TemporaryDirectory(prefix="vesper-walk-") as folder:
        env = dict(os.environ,
                   LIBGL_ALWAYS_SOFTWARE="1",
                   VESPER_PREFERENCES_DIR=folder,
                   VESPER_SAVE_DIR=str(Path(folder) / "saves"))
        log = OUTPUT / "walk.log"
        with log.open("w", encoding="utf-8") as stream:
            proc = subprocess.Popen(
                [str(PLAYER), str(GAME)],
                env=env, stdout=stream, stderr=subprocess.STDOUT,
            )
            try:
                window = window_id()
                wait_for_log(log, "Vesper exploration preview entered:")
                assert START.search(log_text(log)), "Didn't enter the authored street"
                time.sleep(1.5)
                initial = frame(window, "walk-initial.png", 1280, 720)
                # Exclude the text/control HUD. A fullscreen placeholder
                # previously hid the entire location while movement logs
                # still looked correct; require actual scenery texture.
                central_colors = int(call(
                    "convert", str(initial),
                    "-crop", "480x300+200+120", "+repage",
                    "-format", "%k", "info:",
                ).stdout.strip())
                # The reused Little Lantern art is intentionally low-palette
                # (about 34 colors in this crop); a solid fullscreen avatar
                # previously left only one. Require textured scenery, not 50+.
                assert central_colors > 12, (
                    f"World backdrop concealed or missing ({central_colors} colors)"
                )
                old = log_text(log).count("Vesper exploration stopped:")
                walk(window, "Right", .85)
                x1 = position_after_stop(log, old)
                assert 215.0 < x1 < 480.0, f"Keyboard movement didn't advance correctly: {x1}"
                moved = frame(window, "walk-moved.png", 1280, 720)
                assert changed(initial, moved) > 100, (
                    "Exploration avatar did not visibly move in the actual rendered frame"
                )

                # Pause owns controls. Holding the walking key while the pause
                # overlay is up must not move the avatar in world coordinates.
                key(window, "p")
                wait_for_log(log, "Vesper story paused")
                walk(window, "Right", .75)
                key(window, "p")
                old = log_text(log).count("Vesper exploration stopped:")
                walk(window, "Right", .15)
                x2 = position_after_stop(log, old)
                assert 2.0 < x2 - x1 < 70.0, (
                    f"Paused movement leaked into the world: {x1} -> {x2}"
                )

                # Focus/resize state should re-project instead of modifying
                # logical player position or stretching source artwork.
                call("xdotool", "windowsize", "--sync", window, "946", "511")
                time.sleep(.7)
                compact = frame(window, "walk-compact.png", 946, 511)

                # A focused NPC opens a real .vn scene while freezing walking.
                key(window, "e")
                wait_for_log(log, "Vesper exploration interacting: neighbor -> neighbor_chat")
                time.sleep(.4)
                bubble = frame(window, "npc-bubble.png", 946, 511)
                assert changed(compact, bubble) > 800, (
                    "Real NPC dialogue did not appear in the world"
                )
                # World position must not change under dialogue.
                walk(window, "Right", .35)
                key(window, "F5")
                assert not (Path(folder) / "saves" / "exploration-schema.json").exists(), (
                    "Modal NPC conversation wrote a world-incomplete save"
                )
                key(window, "q")
                wait_for_log(log, "Vesper exploration conversation cancelled")
                restored = frame(window, "npc-cancelled.png", 946, 511)
                assert changed(restored, bubble) > 800, (
                    "Cancel did not dismiss the modal dialogue"
                )
                # Cancellation does not select a choice or consume the NPC:
                # a second E opens the authored scene again.
                old_interactions = log_text(log).count("Vesper exploration interacting:")
                key(window, "e")
                wait_for_log(log, "Vesper exploration interacting:", old_interactions)
                old_returns = log_text(log).count("Vesper exploration resumed")
                for button in ("Return", "Return", "Return", "1", "Return", "Return"):
                    key(window, button)
                    if log_text(log).count("Vesper exploration resumed") > old_returns:
                        break
                wait_for_log(log, "Vesper exploration resumed", old_returns)
                resumed = frame(window, "npc-returned.png", 946, 511)
                assert changed(resumed, bubble) > 800, (
                    "Authored explore return did not reveal walking again"
                )
                # Resume at the same X rather than teleporting to a spawn.
                old = log_text(log).count("Vesper exploration stopped:")
                walk(window, "Right", .15)
                x3 = position_after_stop(log, old)
                assert 2.0 < x3 - x2 < 70.0, (
                    f"Modal conversation changed the return X: {x2} -> {x3}"
                )

                # Until NA07 introduces a valid migration that stores world
                # position, F5 deliberately refuses incomplete quick saves.
                key(window, "F5")
                assert not (Path(folder) / "saves" / "exploration-schema.json").exists(), (
                    "Walk preview incorrectly wrote an incomplete VN-only save"
                )
                print(f"Exploration movement x={x1:.1f}->{x2:.1f}->{x3:.1f}; pause and return preserved",
                      flush=True)
                print("Native walking, NPC interaction, cancellation, explicit return, dialogue and incomplete-save guard: passed",
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
