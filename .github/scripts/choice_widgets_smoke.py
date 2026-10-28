"""VX03 graphical choice-row acceptance checks.

Uses both authored examples and a temporary, external nine-option variant.
No edits are made to either source game, the saved-game format or Rust code.
"""
from __future__ import annotations

import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile
import time

ROOT = Path(__file__).resolve().parents[2]
PLAYER = ROOT / "target/debug/vesper-player"
OUT = Path("/tmp/vesper-choice-widgets-qa")


def call(*args: str, check: bool = True):
    return subprocess.run(args, check=check, capture_output=True,
                          text=True, timeout=20)


def wait(log: Path, marker: str, timeout: float = 15.0) -> None:
    end = time.monotonic() + timeout
    while time.monotonic() < end:
        if marker in log.read_text(errors="replace"):
            return
        time.sleep(.15)
    raise AssertionError(f"{marker} missing from {log}: {log.read_text(errors='replace')[-2000:]}")


def window(title: str) -> str:
    end = time.monotonic() + 20
    while time.monotonic() < end:
        found = call("xdotool", "search", "--name", title, check=False)
        if found.returncode == 0 and found.stdout.strip():
            return found.stdout.splitlines()[0]
        time.sleep(.2)
    raise AssertionError(f"Window absent: {title}")


def press(key: str, delay: float = .25):
    call("xdotool", "key", "--clearmodifiers", key)
    time.sleep(delay)


def frame(window_id: str, path: Path, width=1280, height=720):
    call("import", "-window", window_id, str(path))
    size = call("identify", "-format", "%w %h", str(path)).stdout.strip()
    assert size == f"{width} {height}", (path, size)


def changed(a: Path, b: Path) -> int:
    result = call("compare", "-metric", "AE", str(a), str(b), "null:", check=False)
    found = re.match(r"^\d+", result.stderr.strip())
    assert found, result.stderr
    return int(found.group())


def launch(game: Path, out: Path, log: Path):
    stream = log.open("w")
    env = dict(os.environ, LIBGL_ALWAYS_SOFTWARE="1",
               VESPER_TRACE_SCENES="1",
               VESPER_SAVE_DIR=str(out / "saves"),
               VESPER_PREFERENCES_DIR=str(out / "preferences"))
    player = subprocess.Popen([str(PLAYER), str(game)], env=env,
                              stdout=stream, stderr=subprocess.STDOUT)
    try:
        win = window("The Little Lantern" if "lantern" in game.name
                     else "The Clockwork Moon")
        call("xdotool", "windowfocus", "--sync", win)
        time.sleep(1.8)
        return player, stream, win
    except BaseException:
        player.terminate()
        player.wait(timeout=10)
        stream.close()
        raise


def stop(player, stream):
    if player.poll() is None:
        player.terminate()
        player.wait(timeout=10)
    stream.close()


def verify_two_choices(base: Path):
    folder = OUT / "little-lantern"
    folder.mkdir(parents=True, exist_ok=True)
    log = folder / "player.log"
    player, stream, win = launch(base, folder, log)
    try:
        for _ in range(3):
            press("Return")
        wait(log, "Vesper choice UI mounted 2 independent rows")
        time.sleep(.35)
        first = folder / "choice-first.png"
        second = folder / "choice-second.png"
        frame(win, first)
        press("Down")
        frame(win, second)
        delta = changed(first, second)
        assert 40 < delta < 250000, f"focus changed unexpected pixels: {delta}"

        press("F5")
        wait(log, "Vesper save written")
        press("F9")
        wait(log, "Vesper save restored")
        time.sleep(.35)
        restored = folder / "choice-restored.png"
        frame(win, restored)
        assert changed(second, restored) < 5000, "F5/F9 changed selected row or game scene"
        # Pointer activation must use the same validated narrative branch,
        # without requiring an Enter key or triggering two choices at once.
        call("xdotool", "mousemove", "--window", win, "550", "575")
        time.sleep(.35)
        call("xdotool", "click", "1")
        wait(log, "Vesper scene dialogue: walk_together")
        assert log.read_text(errors="replace").count("Vesper scene dialogue: walk_together") == 1
        print("Lantern: separate rows, focus, F5/F9 and pointer activation passed", flush=True)
    finally:
        stop(player, stream)


def verify_nine_options(base: Path, tmp: Path):
    game = tmp / "little-lantern"
    shutil.copytree(base, game)
    script = game / "scripts" / "story.vn"
    content = script.read_text()
    old = (
        '* "Hold the lantern while Pip opens the gate." -> hold_light\n'
        '* "Walk beside Pip and watch the stars." -> walk_together'
    )
    generated = "\n".join(
        f'* "Choice {index}: We can travel beside the lantern and watch the starlight'
        f' gather over this very quiet valley together. Buenas noches, mañana, étoile." -> '
        f'{"walk_together" if index == 9 else "hold_light"}'
        for index in range(1, 10)
    )
    assert old in content
    script.write_text(content.replace(old, generated))
    folder = OUT / "nine-long-choices"
    folder.mkdir(parents=True, exist_ok=True)
    log = folder / "player.log"
    player, stream, win = launch(game, folder, log)
    try:
        call("xdotool", "windowsize", win, "946", "511")
        time.sleep(.45)
        for _ in range(3):
            press("Return")
        wait(log, "Vesper choice UI mounted 9 independent rows")
        time.sleep(.5)
        first = folder / "nine-946x511-selected-first.png"
        frame(win, first, 946, 511)
        for _ in range(8):
            press("Down", .14)
        last = folder / "nine-946x511-selected-last.png"
        frame(win, last, 946, 511)
        assert changed(first, last) > 100, "last choice was not visually revealed"
        for width, height in ((955, 520), (800, 600), (900, 700), (1920, 1080)):
            call("xdotool", "windowsize", win, str(width), str(height))
            time.sleep(.65)
            picture = folder / f"nine-{width}x{height}-selected-last.png"
            frame(win, picture, width, height)
        press("9")
        wait(log, "Vesper scene dialogue: walk_together")
        assert player.poll() is None, "choosing ninth option crashed"
        print("External nine-option fixture: wrap, scroll, narrow resize and digit 9 passed", flush=True)
    finally:
        stop(player, stream)


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    verify_two_choices(ROOT / "examples" / "little-lantern")
    with tempfile.TemporaryDirectory(prefix="vesper-choices-fixture-") as tmp:
        verify_nine_options(ROOT / "examples" / "little-lantern", Path(tmp))


if __name__ == "__main__":
    main()
