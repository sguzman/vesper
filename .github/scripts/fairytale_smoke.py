"""CI-only pixel and keyboard checks for both independent Vesper fairy tales.

Captures Bevy-rendered PNGs as CI artifacts for optional human art review.
Exhaustive dialogue paths are tested by the Rust headless tests separately.
"""
from __future__ import annotations
import os
from pathlib import Path
import re
import subprocess
import tempfile
import time

ROOT = Path(__file__).resolve().parents[2]
OUT = Path("/tmp/vesper-fairytale-shots")


def invoke(*args: str, checked: bool = True) -> subprocess.CompletedProcess:
    return subprocess.run(args, capture_output=True, text=True, check=checked)


def window_for(title: str) -> str:
    until = time.monotonic() + 30
    while time.monotonic() < until:
        result = invoke("xdotool", "search", "--name", title, checked=False)
        if result.returncode == 0 and result.stdout.strip():
            return result.stdout.strip().splitlines()[0]
        time.sleep(0.4)
    raise AssertionError(f"Window did not appear: {title}")


def image_difference(first: Path, second: Path) -> int:
    result = invoke("compare", "-metric", "AE", str(first), str(second), "null:", checked=False)
    found = re.match(r"\s*(\d+)", result.stderr)
    assert found, result.stderr
    return int(found.group(1))


def exercise(name: str, title: str, advances: int) -> None:
    root = ROOT / "examples" / name
    with tempfile.TemporaryDirectory(prefix="vesper-fairytale-save-") as save:
        log = OUT / f"{name}-player.log"
        env = dict(os.environ, LIBGL_ALWAYS_SOFTWARE="1", VESPER_SAVE_DIR=save)
        with log.open("w") as stream:
            proc = subprocess.Popen(
                [str(ROOT / "target/debug/vesper-player"), str(root)],
                env=env, stdout=stream, stderr=subprocess.STDOUT,
            )
            try:
                win = window_for(title)
                time.sleep(4)
                first = OUT / f"{name}-opening.png"
                invoke("import", "-window", win, str(first))
                colors = int(invoke("identify", "-format", "%k", str(first)).stdout)
                assert colors >= 32, f"{name}: suspiciously blank image with {colors} colors"
                invoke("xdotool", "windowfocus", "--sync", win)
                for _ in range(advances):
                    invoke("xdotool", "key", "--clearmodifiers", "space")
                    time.sleep(0.5)
                choices = OUT / f"{name}-choices.png"
                invoke("import", "-window", win, str(choices))
                assert image_difference(first, choices) > 100
                invoke("xdotool", "key", "--clearmodifiers", "Down")
                time.sleep(0.5)
                selected = OUT / f"{name}-selected.png"
                invoke("import", "-window", win, str(selected))
                assert image_difference(choices, selected) > 20, "arrow should change selection"
                invoke("xdotool", "key", "--clearmodifiers", "Return")
                time.sleep(2)
                following = OUT / f"{name}-next-scene.png"
                invoke("import", "-window", win, str(following))
                changed = image_difference(selected, following)
                assert changed > 200, f"{name}: selection did not advance scene"
                assert proc.poll() is None, "game unexpectedly exited"
                print(f"{name}: {colors} colors, selection changed {changed} pixels", flush=True)
                invoke("xdotool", "key", "--clearmodifiers", "Escape")
                proc.wait(timeout=12)
                assert proc.returncode == 0, log.read_text(errors="replace")[-4000:]
            finally:
                if proc.poll() is None:
                    proc.terminate()
                    proc.wait(timeout=8)


if __name__ == "__main__":
    OUT.mkdir(parents=True, exist_ok=True)
    exercise("little-lantern", "The Little Lantern", 3)
    exercise("clockwork-moon", "The Clockwork Moon", 6)
