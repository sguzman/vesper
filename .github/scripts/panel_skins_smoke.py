"""QOL03: real Bevy screenshots for two externally authored panel skins.

Checks a nine-choice notebook at every certified window size, a scene-switched
banner, the return to the solid panel, and high-contrast skin suppression.
Runs under Xvfb with all state isolated in a temporary directory.
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
GAME = ROOT / "examples/panel-showcase"
OUT = Path("/tmp/vesper-panel-skins-qa")
SIZES = ((946, 511), (955, 520), (800, 600), (900, 700),
         (1280, 720), (1920, 1080))


def call(*args: str, check: bool = True):
    return subprocess.run(args, text=True, capture_output=True,
                          check=check, timeout=25)


def wait(log: Path, needle: str, timeout: float = 20.0) -> None:
    limit = time.monotonic() + timeout
    while time.monotonic() < limit:
        if log.exists() and needle in log.read_text(errors="replace"):
            return
        time.sleep(.15)
    raise AssertionError(f"did not observe {needle!r}: "
                         f"{log.read_text(errors='replace')[-1800:]}")


def key(name: str, delay: float = .25) -> None:
    call("xdotool", "key", "--clearmodifiers", name)
    time.sleep(delay)


def shot(win: str, path: Path, width: int, height: int) -> None:
    time.sleep(.5)
    call("import", "-window", win, str(path))
    got = call("identify", "-format", "%w %h", str(path)).stdout
    assert got == f"{width} {height}", (path, got)


def changed(before: Path, after: Path) -> int:
    result = call("compare", "-metric", "AE", str(before), str(after),
                  "null:", check=False)
    match = re.match(r"^\d+", result.stderr.strip())
    assert match, result.stderr
    return int(match.group())


def launch(root: Path, contrast: bool):
    prefix = root / ("contrast" if contrast else "normal")
    prefix.mkdir()
    prefs = prefix / "prefs"
    prefs.mkdir()
    (prefs / "player-preferences.json").write_text(
        json.dumps({"format_version": 1, "high_contrast": contrast}))
    log = prefix / "player.log"
    stream = log.open("w")
    env = dict(os.environ, LIBGL_ALWAYS_SOFTWARE="1",
               VESPER_TRACE_SCENES="1",
               VESPER_SAVE_DIR=str(prefix / "saves"),
               VESPER_PREFERENCES_DIR=str(prefs))
    process = subprocess.Popen([str(PLAYER), str(GAME)], env=env,
                               stdout=stream, stderr=subprocess.STDOUT)
    try:
        limit = time.monotonic() + 25
        win = None
        while time.monotonic() < limit:
            result = call("xdotool", "search", "--name", "The Field Notebook",
                          check=False)
            if result.stdout.strip():
                win = result.stdout.splitlines()[0]
                break
            time.sleep(.2)
        assert win, "field notebook window missing"
        call("xdotool", "windowfocus", "--sync", win)
        wait(log, 'Vesper dialogue panel: Some("notebook")')
        return process, stream, win, log, prefix
    except BaseException:
        process.terminate()
        process.wait(timeout=10)
        stream.close()
        raise


def stop(process, stream):
    if process.poll() is None:
        process.terminate()
        process.wait(timeout=10)
    stream.close()


def verify(root: Path, contrast: bool):
    process, stream, win, log, _ = launch(root, contrast)
    out = OUT / ("contrast" if contrast else "normal")
    out.mkdir(parents=True, exist_ok=True)
    try:
        call("xdotool", "windowsize", "--sync", win, "1280", "720")
        opening = out / "notebook-opening.png"
        shot(win, opening, 1280, 720)
        # Two authored dialogue lines before a genuine nine-row choice list.
        key("Return")
        key("Return")
        wait(log, "Vesper choice UI mounted 9 independent rows")
        for width, height in SIZES:
            call("xdotool", "windowsize", "--sync",
                 win, str(width), str(height))
            shot(win, out / f"notebook-choices-{width}x{height}.png",
                 width, height)
        call("xdotool", "windowsize", "--sync", win, "1280", "720")
        for _ in range(8):
            key("Down", .13)
        last = out / "notebook-ninth-selected.png"
        shot(win, last, 1280, 720)
        assert changed(out / "notebook-choices-1280x720.png", last) > 150, (
            "scroll to ninth notebook choice was not visible")
        key("Return")
        wait(log, 'Vesper dialogue panel: Some("banner")')
        banner = out / "banner.png"
        shot(win, banner, 1280, 720)
        key("F5")
        wait(log, "Vesper save written")
        key("Return")
        wait(log, "Vesper dialogue panel: None")
        solid = out / "solid-default.png"
        shot(win, solid, 1280, 720)
        assert changed(banner, solid) > 2500, (
            "switching from banner image to solid panel did not redraw")
        key("F9")
        wait(log, "Vesper save restored")
        restored = out / "banner-after-quickload.png"
        shot(win, restored, 1280, 720)
        assert changed(banner, restored) < 5000, (
            "quickload did not restore the author-selected banner panel")
        assert process.poll() is None, "panel-showcase player exited unexpectedly"
        print(f"QOL03 high_contrast={contrast}: six sizes, nine choices,"
              " banner scene and default restored", flush=True)
    finally:
        stop(process, stream)


def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="vesper-panel-skins-") as tmp:
        root = Path(tmp)
        verify(root, False)
        verify(root, True)
        normal = OUT / "normal" / "notebook-opening.png"
        contrast = OUT / "contrast" / "notebook-opening.png"
        # If the authored art is accidentally always hidden, this cannot pass
        # on otherwise identical geometry; changing accessibility styling
        # also deliberately changes pixels.
        assert changed(normal, contrast) > 2500, (
            "high-contrast skin suppression did not change the screenshot")
    print("QOL03 image panel graphical regression: passed", flush=True)


if __name__ == "__main__":
    main()
