"""QOL05: genuine Bevy screenshots verify time-varying text colors.

No geometry changes; normal-mode screenshot differences must occur inside the
dialogue body. In high contrast and reduced motion the same crop must be static.
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
GAME = ROOT / "examples/word-effects"
PLAYER = ROOT / "target/debug/vesper-player"
OUT = Path("/tmp/vesper-word-effects-qa")
SIZES = ((946, 511), (955, 520), (800, 600), (900, 700),
         (1280, 720), (1920, 1080))


def call(*args, check=True):
    return subprocess.run(args, text=True, capture_output=True,
                          check=check, timeout=30)


def wait(log, needle):
    deadline = time.monotonic() + 25
    while time.monotonic() < deadline:
        if log.exists() and needle in log.read_text(errors="replace"):
            return
        time.sleep(.12)
    raise AssertionError(f"missing {needle}: {log.read_text(errors='replace')[-1200:]}")


def shot(win, path, width=1280, height=720):
    deadline = time.monotonic() + 12
    while time.monotonic() < deadline:
        time.sleep(.2)
        call("import", "-window", win, str(path))
        size = call("identify", "-format", "%w %h", str(path)).stdout
        assert size == f"{width} {height}", (size, width, height)
        if int(call("identify", "-format", "%k", str(path)).stdout) > 40:
            return path
    raise AssertionError(f"blank effect frame: {path}")


def crop(path, destination):
    # Exclude the continue cue and window border. The first line is always
    # inside the dialogue region on the 1280x720 reference window.
    call("convert", str(path), "-crop", "1030x180+110+430", "+repage", str(destination))
    return destination


def different(a, b):
    comparison = call("compare", "-metric", "AE", str(a), str(b),
                      "null:", check=False)
    match = re.match(r"^\d+", comparison.stderr.strip())
    assert match, comparison.stderr
    return int(match.group())


def verify(mode, contrast=False, reduced=False):
    out = OUT / mode
    out.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="vesper-word-fx-") as temp:
        home = Path(temp)
        preferences = home / "prefs"
        preferences.mkdir()
        (preferences / "player-preferences.json").write_text(json.dumps({
            "format_version": 1, "reveal_speed": "instant",
            "high_contrast": contrast, "reduced_motion": reduced,
        }))
        log = out / "player.log"
        env = dict(os.environ, LIBGL_ALWAYS_SOFTWARE="1",
                   VESPER_PREFERENCES_DIR=str(preferences),
                   VESPER_SAVE_DIR=str(home / "save"))
        with log.open("w") as stream:
            proc = subprocess.Popen([str(PLAYER), str(GAME)], env=env,
                                    stdout=stream, stderr=subprocess.STDOUT)
            try:
                win = None
                until = time.monotonic() + 25
                while time.monotonic() < until:
                    r = call("xdotool", "search", "--name",
                             "The Luminous Page", check=False)
                    if r.stdout.strip():
                        win = r.stdout.splitlines()[0]
                        break
                    time.sleep(.2)
                assert win, "missing word effects game window"
                call("xdotool", "windowfocus", "--sync", win)
                wait(log, "Vesper inline text spans mounted")
                for width, height in SIZES if mode == "animated" else ((1280, 720),):
                    call("xdotool", "windowsize", "--sync", win,
                         str(width), str(height))
                    shot(win, out / f"{width}x{height}.png", width, height)
                call("xdotool", "windowsize", "--sync", win, "1280", "720")
                time.sleep(.35)
                first = crop(shot(win, out / "before.png"), out / "before-body.png")
                time.sleep(.61)
                later = crop(shot(win, out / "after.png"), out / "after-body.png")
                delta = different(first, later)
                if mode == "animated":
                    assert delta > 20, f"author word effects did not animate: {delta}"
                else:
                    assert delta < 20, f"{mode} must suppress changes: {delta}"
                assert proc.poll() is None, f"{mode} player exited unexpectedly"
                print(f"QOL05 {mode}: body pixel delta {delta}", flush=True)
            finally:
                if proc.poll() is None:
                    proc.terminate()
                    proc.wait(timeout=10)


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    verify("animated")
    verify("high-contrast", contrast=True)
    verify("reduced-motion", reduced=True)
    print("QOL05: three animated effects and accessibility fallback PASS",
          flush=True)


if __name__ == "__main__":
    main()
