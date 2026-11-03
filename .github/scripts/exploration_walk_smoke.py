#!/usr/bin/env python3
"""Run an actual opt-in walk-first Vesper location in the Bevy player.

The fixture only supplies external authoring data. No engine source-path
hardcoding, simulated keyboard state, fake screenshots or parsed game code.
"""
from __future__ import annotations

import json
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


def scenery_frame(window: str, filename: str, width: int,
                  height: int, log: Path) -> Path:
    """Wait for asynchronous Bevy texture readiness, not just an open window.

    The HUD may be ready while scenery is still a flat background. Preserve
    the last captured frame and fail with the actual render log if scenery
    never appears; never accept a single-color frame as a passing test.
    """
    deadline = time.monotonic() + 15
    colors = 0
    while time.monotonic() < deadline:
        image = frame(window, filename, width, height)
        colors = int(call(
            "convert", str(image), "-crop", "480x300+200+120",
            "+repage", "-format", "%k", "info:",
        ).stdout.strip())
        # Reused Little Lantern scenery is deliberately low-palette (~34
        # colors in this area), whereas the failed frame had 1.
        if colors > 12:
            return image
        time.sleep(.3)
    raise AssertionError(
        f"World backdrop never appeared ({colors} colors); "
        f"see {OUTPUT / filename} and {log}:\\n{log_text(log)[-3000:]}"
    )


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
                initial = scenery_frame(
                    window, "walk-initial.png", 1280, 720, log
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
                wait_for_log(log, "Vesper save written")
                saved_path = Path(folder) / "saves" / "exploration-schema.json"
                saved = json.loads(saved_path.read_text(encoding="utf-8"))
                assert saved["format_version"] == 2
                assert "conversation" in saved["experience"]["mode"], (
                    "Modal save omitted the return-to-world position"
                )
                key(window, "F9")
                wait_for_log(log, "Vesper save restored: Conversation")
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

                # The friend is gated behind met_neighbor=true, set by
                # the first real conversation (not by a separate quest store).
                old = log_text(log).count("Vesper exploration stopped:")
                walk(window, "Right", .90)
                x4 = position_after_stop(log, old)
                assert 520.0 < x4 < 710.0, (
                    f"Could not reach the conditional friend: {x4}"
                )
                key(window, "e")
                wait_for_log(log, "Vesper exploration interacting: friend -> friend_chat")
                frame(window, "conditional-friend-dialogue.png", 946, 511)
                old_returns = log_text(log).count("Vesper exploration resumed")
                for _ in range(4):
                    key(window, "Return")
                    if log_text(log).count("Vesper exploration resumed") > old_returns:
                        break
                wait_for_log(log, "Vesper exploration resumed", old_returns)

                # An authored E-only exit replaces street sprites with the
                # café's area and lands at its named spawn, not at a random X.
                old = log_text(log).count("Vesper exploration stopped:")
                walk(window, "Right", 2.35)
                x5 = position_after_stop(log, old)
                assert 1090.0 < x5 <= 1240.0, (
                    f"Could not reach the café exit: {x5}"
                )
                key(window, "e")
                wait_for_log(log, "Vesper exploration exit: street/cafe_door -> cafe at x=65.0")
                wait_for_log(log, "Vesper exploration area loaded: cafe; replaced 5 entities")
                time.sleep(.5)
                cafe = scenery_frame(
                    window, "cafe-after-exit.png", 946, 511, log
                )
                assert changed(cafe, resumed) > 800, (
                    "Entering café failed to reframe/rebuild the authored location"
                )
                key(window, "e")
                wait_for_log(log, "Vesper exploration exit: cafe/street_door -> street at x=150.0")
                wait_for_log(log, "Vesper exploration area loaded: street; replaced 2 entities")
                time.sleep(.5)
                scenery_frame(
                    window, "street-after-return-exit.png", 946, 511, log
                )

                # F9 restores a walking position, not merely the VN machine.
                old_saves = log_text(log).count("Vesper save written")
                key(window, "F5")
                wait_for_log(log, "Vesper save written", old_saves)
                saved = json.loads(saved_path.read_text(encoding="utf-8"))
                assert saved["format_version"] == 2
                point = saved["experience"]["mode"]["exploring"]
                assert point["location"] == "street"
                assert abs(point["x"] - 150.0) < 1.0, point
                walk(window, "Right", .4)
                old_restores = log_text(log).count("Vesper save restored: Exploring")
                key(window, "F9")
                wait_for_log(log, "Vesper save restored: Exploring", old_restores)
                old_stops = log_text(log).count("Vesper exploration stopped:")
                walk(window, "Right", .15)
                position = position_after_stop(log, old_stops)
                assert 152.0 < position < 218.0, (
                    f"F9 did not restore saved street position: {position}"
                )

                # Save a distinct from-start walking X for cold-process QA.
                old_saves = log_text(log).count("Vesper save written")
                key(window, "F5")
                wait_for_log(log, "Vesper save written", old_saves)
                persisted = json.loads(saved_path.read_text(encoding="utf-8"))
                saved_x = persisted["experience"]["mode"]["exploring"]["x"]
                assert 152.0 < saved_x < 218.0, saved_x

                # The same v2 world snapshot is supported in manual F6 slots.
                key(window, "F6")
                wait_for_log(log, "Vesper save slot metadata scanned")
                old_saves = log_text(log).count("Vesper save written")
                key(window, "Return")
                wait_for_log(log, "Vesper save written", old_saves)
                manual_path = Path(folder) / "saves" / "exploration-schema.slot1.json"
                manual = json.loads(manual_path.read_text(encoding="utf-8"))
                assert manual["format_version"] == 2
                assert abs(manual["experience"]["mode"]["exploring"]["x"] - saved_x) < 0.1
                key(window, "F6")
                print(f"Exploration movement x={x1:.1f}->{x2:.1f}->{x3:.1f}->{x4:.1f}->{x5:.1f}",
                      flush=True)
                print("Native NPC/cancellation, gated friend, E-only café roundtrip, entity cleanup, explicit return and versioned world saves: passed",
                      flush=True)
            finally:
                proc.terminate()
                try:
                    proc.wait(timeout=6)
                except subprocess.TimeoutExpired:
                    proc.kill()
                    proc.wait(timeout=3)

        # A fresh executable must restore the saved position and manual slot
        # from disk, not from an in-memory Session or ECS entity handle.
        restarted_log = OUTPUT / "walk-restarted.log"
        with restarted_log.open("w", encoding="utf-8") as stream:
            restarted = subprocess.Popen(
                [str(PLAYER), str(GAME)],
                env=env, stdout=stream, stderr=subprocess.STDOUT,
            )
            try:
                window = window_id()
                wait_for_log(restarted_log, "Vesper exploration preview entered:")
                key(window, "F9")
                wait_for_log(restarted_log, "Vesper save restored: Exploring")
                old_stops = log_text(restarted_log).count("Vesper exploration stopped:")
                walk(window, "Right", .15)
                cold_x = position_after_stop(restarted_log, old_stops)
                assert saved_x + 2.0 < cold_x < saved_x + 70.0, (
                    f"Cold-start F9 lost world X: {saved_x} -> {cold_x}"
                )
                frame(window, "walk-cold-restored.png", 1280, 720)

                key(window, "F6")
                wait_for_log(restarted_log, "Vesper save slot metadata scanned")
                key(window, "Right")
                old_loads = log_text(restarted_log).count("Vesper save restored:")
                key(window, "Return")
                wait_for_log(restarted_log, "Vesper save restored:", old_loads)
                old_stops = log_text(restarted_log).count("Vesper exploration stopped:")
                walk(window, "Right", .15)
                slot_x = position_after_stop(restarted_log, old_stops)
                assert saved_x + 2.0 < slot_x < saved_x + 70.0, (
                    f"Cold-start slot load lost world X: {saved_x} -> {slot_x}"
                )
                print("Cold-process quicksave and manual slot world restore: passed",
                      flush=True)
            finally:
                restarted.terminate()
                try:
                    restarted.wait(timeout=6)
                except subprocess.TimeoutExpired:
                    restarted.kill()
                    restarted.wait(timeout=3)


if __name__ == "__main__":
    main()
