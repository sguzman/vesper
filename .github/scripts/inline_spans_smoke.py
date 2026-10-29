"""QOL04: authored inline colors visibly render in native Bevy TextSpans.

Comparing only the two declared style colors proves a normal-mode difference;
high-contrast override suppresses the author palette in both variations.
Six real captured viewports test wrapping and stable text layout.
"""
from __future__ import annotations
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile
import time

ROOT = Path(__file__).resolve().parents[2]
GAME = ROOT / "examples/inline-words"
PLAYER = ROOT / "target/debug/vesper-player"
OUT = Path("/tmp/vesper-inline-spans-qa")
SIZES = ((946, 511), (955, 520), (800, 600), (900, 700),
         (1280, 720), (1920, 1080))


def call(*args: str, check: bool = True):
    return subprocess.run(args, text=True, capture_output=True,
                          check=check, timeout=30)


def wait(log: Path, needle: str) -> None:
    deadline = time.monotonic() + 25
    while time.monotonic() < deadline:
        if log.exists() and needle in log.read_text(errors="replace"):
            return
        time.sleep(.15)
    raise AssertionError(f"missing {needle}: {log.read_text(errors='replace')[-1500:]}")


def capture(win: str, path: Path, width: int, height: int) -> None:
    deadline = time.monotonic() + 12
    while time.monotonic() < deadline:
        time.sleep(.3)
        call("import", "-window", win, str(path))
        actual = call("identify", "-format", "%w %h", str(path)).stdout
        assert actual == f"{width} {height}", (path, actual)
        if int(call("identify", "-format", "%k", str(path)).stdout) > 40:
            return
    raise AssertionError(f"blank inline text frame: {path}")


def difference(left: Path, right: Path) -> int:
    result = call("compare", "-metric", "AE", str(left), str(right),
                  "null:", check=False)
    digits = re.match(r"^\d+", result.stderr.strip())
    assert digits, result.stderr
    return int(digits.group())


def verify(root: Path, label: str, recolor: bool, contrast: bool) -> Path:
    game = root / label
    shutil.copytree(GAME, game)
    if recolor:
        theme = game / "styles/words.toml"
        old = theme.read_text()
        assert "color = [1.0, 0.43, 0.12, 1.0]" in old
        assert "color = [0.34, 0.64, 0.94, 1.0]" in old
        theme.write_text(old.replace(
            "color = [1.0, 0.43, 0.12, 1.0]",
            "color = [0.2, 1.0, 0.3, 1.0]"
        ).replace(
            "color = [0.34, 0.64, 0.94, 1.0]",
            "color = [1.0, 0.9, 0.1, 1.0]"
        ))
    directory = OUT / label
    directory.mkdir(parents=True, exist_ok=True)
    preferences = game / "prefs"
    preferences.mkdir()
    (preferences / "player-preferences.json").write_text(json.dumps({
        "format_version": 1,
        "reveal_speed": "instant",
        "high_contrast": contrast
    }))
    log = directory / "player.log"
    with log.open("w") as output:
        env = dict(os.environ, LIBGL_ALWAYS_SOFTWARE="1",
                   VESPER_PREFERENCES_DIR=str(preferences),
                   VESPER_SAVE_DIR=str(game / "saves"),
                   VESPER_TRACE_SCENES="1")
        process = subprocess.Popen([str(PLAYER), str(game)], env=env,
                                   stdout=output, stderr=subprocess.STDOUT)
        try:
            win = None
            deadline = time.monotonic() + 25
            while time.monotonic() < deadline:
                result = call("xdotool", "search", "--name",
                              "The Ember Letter", check=False)
                if result.stdout.strip():
                    win = result.stdout.splitlines()[0]
                    break
                time.sleep(.2)
            assert win, "missing styled text demo window"
            call("xdotool", "windowfocus", "--sync", win)
            wait(log, "Vesper inline text spans mounted")
            for width, height in SIZES if label == "original" else ((1280,720),):
                call("xdotool", "windowsize", "--sync",
                     win, str(width), str(height))
                capture(win, directory / f"{width}x{height}.png", width, height)
            # A new line begins with an authored style, without an unstyled
            # prefix. This catches zero-height Text roots hiding every span.
            call("xdotool", "windowsize", "--sync", win, "1280", "720")
            previous = log.read_text(errors="replace").count(
                "Vesper inline text spans mounted")
            call("xdotool", "key", "--clearmodifiers", "Return")
            deadline = time.monotonic() + 15
            while time.monotonic() < deadline:
                if log.read_text(errors="replace").count(
                        "Vesper inline text spans mounted") > previous:
                    break
                time.sleep(.1)
            else:
                raise AssertionError("styled-leading scene never mounted")
            capture(win, directory / "leading-1280x720.png", 1280, 720)
            assert process.poll() is None, "styled dialogue player crashed"
            print(f"{label}: styled text rendered; high contrast={contrast}", flush=True)
        finally:
            if process.poll() is None:
                process.terminate()
                process.wait(timeout=10)
    return directory / "1280x720.png"


def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="vesper-inline-spans-") as tmp:
        root = Path(tmp)
        a = verify(root, "original", False, False)
        b = verify(root, "alternative", True, False)
        # Same story, same sizes and identical panel: changing only the two
        # authored inline colors must change the actual text pixels.
        assert difference(a, b) > 50, (
            "style colors did not modify rendered word pixels")
        assert difference(
            OUT / "original/leading-1280x720.png",
            OUT / "alternative/leading-1280x720.png"
        ) > 50, "leading styled words did not render tinted pixels"
        c = verify(root, "contrast-original", False, True)
        d = verify(root, "contrast-alternative", True, True)
        assert difference(c, d) < 150, (
            "high contrast did not override externally authored text colors")
        assert difference(
            OUT / "contrast-original/leading-1280x720.png",
            OUT / "contrast-alternative/leading-1280x720.png"
        ) < 150, "leading styled words ignored high contrast"
    print("QOL04: named inline colors, six sizes, high-contrast precedence PASSED",
          flush=True)


if __name__ == "__main__":
    main()
