#!/usr/bin/env python3
"""Native Bevy speech-bubble smoke: old visual novel vs bubble-only fixture.

Uses the actual player and real window pixels, not a mocked layout. The two
externally authored games share a story/art package; only the optional
[dialogue] surface setting differs. No arbitrary scripted game logic.
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
OUTPUT = Path("/tmp/vesper-speech-bubbles-qa")
OUTPUT.mkdir(parents=True, exist_ok=True)


def call(*args: str, check: bool = True) -> subprocess.CompletedProcess[str]:
    return subprocess.run(args, capture_output=True, text=True, check=check)


def wait_window(title: str) -> str:
    deadline = time.monotonic() + 30
    while time.monotonic() < deadline:
        result = call("xdotool", "search", "--name", title, check=False)
        if result.returncode == 0 and result.stdout.strip():
            return result.stdout.splitlines()[0]
        time.sleep(0.2)
    raise AssertionError(f"Window never opened: {title}")


def grab(win: str, name: str, size: tuple[int, int]) -> Path:
    out = OUTPUT / name
    for _ in range(30):
        call("import", "-window", win, str(out))
        result = call("identify", "-format", "%w %h %k", str(out))
        width, height, colors = [int(n) for n in result.stdout.split()]
        if (width, height) == size and colors > 80:
            return out
        time.sleep(0.2)
    raise AssertionError(f"No rendered {size} frame for {name}")


def diff(a: Path, b: Path) -> int:
    compare = call("compare", "-metric", "AE", str(a), str(b), "null:", check=False)
    match = re.search(r"^\s*(\d+)", compare.stderr)
    assert match, compare.stderr
    return int(match[1])


def press(win: str, key: str) -> None:
    call("xdotool", "windowfocus", "--sync", win)
    call("xdotool", "key", "--clearmodifiers", key)
    time.sleep(0.6)


def run_game(directory: str, title: str, filename: str) -> tuple[Path, Path, Path | None]:
    with tempfile.TemporaryDirectory(prefix="vesper-bubble-") as folder:
        env = dict(os.environ, LIBGL_ALWAYS_SOFTWARE="1",
                   VESPER_PREFERENCES_DIR=folder,
                   VESPER_SAVE_DIR=str(Path(folder) / "saves"))
        logfile = OUTPUT / (filename + ".log")
        with logfile.open("w", encoding="utf-8") as stream:
            process = subprocess.Popen([str(PLAYER), str(ROOT / directory)],
                                       stdout=stream, stderr=subprocess.STDOUT,
                                       env=env)
            try:
                win = wait_window(title)
                time.sleep(2.0)
                initial = grab(win, filename + "-initial.png", (1280, 720))
                if directory != "examples/bubble-showcase":
                    return initial, logfile, None
                press(win, "Return")
                press(win, "Return")
                speaker = grab(win, filename + "-speaker.png", (1280, 720))
                assert diff(initial, speaker) > 3000, "Speaker bubble never advanced"
                call("xdotool", "windowsize", "--sync", win, "946", "511")
                time.sleep(0.8)
                compact = grab(win, filename + "-compact.png", (946, 511))
                assert compact.exists()
                call("xdotool", "windowsize", "--sync", win, "1280", "720")
                time.sleep(0.7)
                press(win, "Return")  # a choice scene uses the standard option panel
                choices = grab(win, filename + "-choices.png", (1280, 720))
                assert diff(speaker, choices) > 3000, "Bubble did not yield to choices"
                return initial, logfile, choices
            finally:
                process.terminate()
                try:
                    process.wait(timeout=6)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait(timeout=3)


def main() -> None:
    bubble, _, choices = run_game(
        "examples/bubble-showcase", "Speech Bubble Rendering Fixture", "bubble"
    )
    classic, _, _ = run_game(
        "examples/little-lantern", "The Little Lantern", "classic"
    )
    difference = diff(bubble, classic)
    assert difference > 4000, (
        "A pure VN bubble default looks the same as the traditional dialogue panel",
        difference,
    )
    assert choices is not None
    print(f"Native optional bubble vs legacy panel: {difference} different pixels", flush=True)
    print("Speech bubbles, staged-speaker advance, choice fallback, compact layout: passed",
          flush=True)


if __name__ == "__main__":
    main()
