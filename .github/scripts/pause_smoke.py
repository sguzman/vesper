"""Verify engine pause overlay visibility and blocked narrative input."""
from __future__ import annotations

import os
from pathlib import Path
import re
import subprocess
import tempfile
import time

ROOT = Path(__file__).resolve().parents[2]


def run(*args: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(args, check=True, capture_output=True, text=True)


def changed_pixels(a: Path, b: Path) -> int:
    result = subprocess.run(
        ["compare", "-metric", "AE", str(a), str(b), "null:"],
        capture_output=True, text=True
    )
    match = re.match(r"^\d+", result.stderr.strip())
    assert match is not None, f"unrecognized image metric: {result.stderr}"
    return int(match.group())


def main() -> None:
    with tempfile.TemporaryDirectory(prefix="vesper-pause-") as directory:
        temp = Path(directory)
        log = temp / "player.log"
        env = dict(os.environ, LIBGL_ALWAYS_SOFTWARE="1", VESPER_SAVE_DIR=directory)
        with log.open("w") as out:
            process = subprocess.Popen([
                str(ROOT / "target/debug/vesper-player"),
                str(ROOT / "examples/little-lantern")
            ], stdout=out, stderr=subprocess.STDOUT, env=env)
            try:
                window = None
                for _ in range(60):
                    result = subprocess.run(
                        ["xdotool", "search", "--name", "The Little Lantern"],
                        capture_output=True, text=True
                    )
                    if result.returncode == 0 and result.stdout.strip():
                        window = result.stdout.strip().splitlines()[0]
                        break
                    time.sleep(.25)
                assert window, "the Vesper window did not appear"
                time.sleep(4)  # wait for introductory fades to settle
                run("xdotool", "windowfocus", "--sync", window)
                before, paused, after = [temp / name for name in ("before.png", "paused.png", "after.png")]
                run("import", "-window", window, str(before))
                run("xdotool", "key", "--clearmodifiers", "p")
                time.sleep(1)
                run("import", "-window", window, str(paused))
                run("xdotool", "key", "--clearmodifiers", "space")
                time.sleep(.5)
                run("xdotool", "key", "--clearmodifiers", "p")
                time.sleep(1)
                run("import", "-window", window, str(after))
                shown = changed_pixels(before, paused)
                restored = changed_pixels(before, after)
                print(f"Pause overlay changed pixels: {shown}; restored mismatch: {restored}")
                assert shown > 5000, "the pause overlay was not visibly rendered"
                assert restored < 3000, "narrative advanced while paused or overlay failed to close"
                report = log.read_text(errors="replace")
                assert "Vesper story paused" in report, report[-5000:]
                assert "Vesper story resumed" in report, report[-5000:]
                assert process.poll() is None, "player terminated during pause"
                run("xdotool", "key", "--clearmodifiers", "Escape")
                process.wait(timeout=10)
                assert process.returncode == 0, f"unexpected exit code {process.returncode}"
            finally:
                if process.poll() is None:
                    process.terminate()
                    process.wait(timeout=10)


if __name__ == "__main__":
    main()
