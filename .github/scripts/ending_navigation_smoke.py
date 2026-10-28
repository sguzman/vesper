"""End-to-end replay/menu/quit and persisted ending navigation for a real game.

Uses the existing standalone Little Lantern story and Xvfb player; no edits to
authored scripts, assets, or save layout.
"""
from __future__ import annotations

import os
from pathlib import Path
import re
import subprocess
import time

ROOT = Path(__file__).resolve().parents[2]
GAME = ROOT / "examples/little-lantern"
OUT = Path("/tmp/vesper-ending-qa")


def run(*args: str, check=True):
    return subprocess.run(args, check=check, capture_output=True, text=True)


def pressed(key: str, pause: float = .28):
    run("xdotool", "key", "--clearmodifiers", key)
    time.sleep(pause)


def wait(log: Path, marker: str, timeout=12):
    until = time.monotonic() + timeout
    while time.monotonic() < until:
        if marker in log.read_text(errors="replace"):
            return
        time.sleep(.2)
    raise AssertionError(f"Missing {marker}: {log.read_text(errors='replace')[-4500:]}")


def win_id():
    until = time.monotonic() + 25
    while time.monotonic() < until:
        found = run("xdotool", "search", "--name", "The Little Lantern", check=False)
        if found.returncode == 0 and found.stdout.strip():
            return found.stdout.strip().splitlines()[0]
        time.sleep(.25)
    raise AssertionError("Little Lantern window not found")


def diff(a: Path, b: Path) -> int:
    comparison = run("compare", "-metric", "AE", str(a), str(b), "null:", check=False)
    metric = re.match(r"^\d+", comparison.stderr.strip())
    assert metric, comparison.stderr
    return int(metric.group())


def reach_end(log: Path):
    for _ in range(3):
        pressed("space")
    pressed("1")
    for _ in range(5):
        pressed("space")
    wait(log, "Vesper story reached ending scene: goodnight")


def launch(log: Path):
    stream = log.open("w")
    player = subprocess.Popen(
        [str(ROOT / "target/debug/vesper-player"), str(GAME)],
        stdout=stream, stderr=subprocess.STDOUT,
        env=dict(os.environ, LIBGL_ALWAYS_SOFTWARE="1",
                 VESPER_SAVE_DIR=str(OUT / "save")),
    )
    win = win_id()
    run("xdotool", "windowfocus", "--sync", win)
    time.sleep(2)
    return player, stream, win


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    save_file = OUT / "save" / "little-lantern.json"
    log = OUT / "first.log"
    first, stream, win = launch(log)
    try:
        reach_end(log)
        still = OUT / "ending.png"
        selected = OUT / "ending-selected.png"
        run("import", "-window", win, str(still))
        pressed("Down")  # select 2. Main menu
        run("import", "-window", win, str(selected))
        assert diff(still, selected) > 15, "end-screen highlight did not move"
        pressed("Return")
        wait(log, "Vesper main menu opened")
        pressed("2")  # Continue the completed game
        pressed("r")  # Existing replay shortcut still works
        wait(log, "Vesper story restarted from its beginning")
        reach_end(log)
        pressed("F5")
        wait(log, "Vesper save written")
        assert save_file.is_file() and save_file.stat().st_size > 100
        pressed("Escape")
        first.wait(timeout=10)
        assert first.returncode == 0
    finally:
        if first.poll() is None:
            first.terminate()
            first.wait(timeout=10)
        stream.close()

    restored_log = OUT / "restored.log"
    second, stream2, win2 = launch(restored_log)
    try:
        pressed("F9")
        wait(restored_log, "Vesper save restored")
        pressed("Down")
        pressed("Return")
        wait(restored_log, "Vesper main menu opened")
        pressed("3")
        wait(restored_log, "Vesper quitting through menu")
        second.wait(timeout=10)
        assert second.returncode == 0
    finally:
        if second.poll() is None:
            second.terminate()
            second.wait(timeout=10)
        stream2.close()

    last_log = OUT / "last.log"
    third, stream3, win3 = launch(last_log)
    try:
        pressed("F9")
        wait(last_log, "Vesper save restored")
        pressed("3")  # Direct end-screen Quit option
        wait(last_log, "Vesper quitting through ending menu")
        third.wait(timeout=10)
        assert third.returncode == 0
    finally:
        if third.poll() is None:
            third.terminate()
            third.wait(timeout=10)
        stream3.close()

    print("End-screen replay, navigation, quit and cross-process restore passed", flush=True)


if __name__ == "__main__":
    main()
