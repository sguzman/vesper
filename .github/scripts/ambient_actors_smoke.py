"""QOL06 graphical acceptance of independent scene actors (real Bevy/Xvfb)."""
from __future__ import annotations
import json
import os
from pathlib import Path
import re
import subprocess
import tempfile
import time

ROOT = Path(__file__).resolve().parents[2]
GAME = ROOT / "examples/ambient-showcase"
PLAYER = ROOT / "target/debug/vesper-player"
OUT = Path("/tmp/vesper-ambient-actors-qa")
SIZES = ((946, 511), (955, 520), (800, 600), (900, 700),
         (1280, 720), (1920, 1080))

def call(*args, check=True):
    return subprocess.run(args, text=True, capture_output=True,
                          check=check, timeout=30)

def wait(log, needle):
    limit = time.monotonic() + 25
    while time.monotonic() < limit:
        if needle in log.read_text(errors="replace"):
            return
        time.sleep(.1)
    raise AssertionError(f"missing {needle}: {log.read_text(errors='replace')[-1300:]}")

def key(name):
    call("xdotool", "key", "--clearmodifiers", name)
    time.sleep(.35)

def shot(win, path, width, height):
    limit = time.monotonic() + 15
    while time.monotonic() < limit:
        time.sleep(.35)
        call("import", "-window", win, str(path))
        assert call("identify", "-format", "%w %h", str(path)).stdout == f"{width} {height}"
        if int(call("identify", "-format", "%k", str(path)).stdout) > 40:
            return
    raise AssertionError(f"blank ambient frame: {path}")

def crop(path, output):
    call("convert", str(path), "-crop", "220x150+850+130", "+repage", str(output))
    return output

def difference(left, right):
    r = call("compare", "-metric", "AE", str(left), str(right), "null:", check=False)
    m = re.match(r"^\d+", r.stderr.strip())
    assert m, r.stderr
    return int(m.group())

def main():
    OUT.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="vesper-ambient-qa-") as temp:
        prefs = Path(temp) / "preferences"
        prefs.mkdir()
        (prefs / "player-preferences.json").write_text(
            json.dumps({"format_version": 1, "reveal_speed": "instant"}))
        log = OUT / "run.log"
        env = dict(os.environ, LIBGL_ALWAYS_SOFTWARE="1",
                   VESPER_PREFERENCES_DIR=str(prefs),
                   VESPER_SAVE_DIR=str(Path(temp)/"saves"),
                   VESPER_TRACE_SCENES="1")
        with log.open("w") as output:
            proc = subprocess.Popen([str(PLAYER), str(GAME)], env=env,
                                    stdout=output, stderr=subprocess.STDOUT)
            try:
                win = None
                end = time.monotonic() + 25
                while time.monotonic() < end:
                    find = call("xdotool", "search", "--name",
                                "Morning at the Perch", check=False)
                    if find.stdout.strip():
                        win = find.stdout.splitlines()[0]
                        break
                    time.sleep(.1)
                assert win, "ambient game never appeared"
                call("xdotool", "windowfocus", "--sync", win)
                wait(log, "Vesper ambient actors spawned: 1")
                time.sleep(1)
                for width, height in SIZES:
                    call("xdotool", "windowsize", "--sync", win,
                         str(width), str(height))
                    shot(win, OUT / f"visible-{width}x{height}.png", width, height)
                call("xdotool", "windowsize", "--sync", win, "1280", "720")
                visible = crop(OUT/"visible-1280x720.png", OUT/"bird-visible.png")
                key("Return")
                wait(log, "Vesper ambient actor bird: visible=false")
                shot(win, OUT/"hidden-1280x720.png", 1280, 720)
                hidden = crop(OUT/"hidden-1280x720.png", OUT/"bird-hidden.png")
                assert difference(visible, hidden) > 150, "ambient hide did not remove bird pixels"

                key("F5")
                wait(log, "Vesper save written")
                key("Return")
                wait(log, "Vesper ambient actor bird: visible=true")
                shot(win, OUT/"shown-again-1280x720.png", 1280, 720)
                shown = crop(OUT/"shown-again-1280x720.png", OUT/"bird-shown.png")
                assert difference(visible, shown) < 700, "bird show did not restore initial pixels"

                key("F9")
                wait(log, "Vesper save restored")
                shot(win, OUT/"hidden-after-quickload.png", 1280, 720)
                restored = crop(OUT/"hidden-after-quickload.png", OUT/"bird-restored-hidden.png")
                assert difference(hidden, restored) < 700, "F9 lost saved ambient visibility"

                key("Return")
                key("Return")
                wait(log, "Vesper choice UI mounted 2 independent rows")
                key("1")
                wait(log, "Vesper scene dialogue: indoors")
                shot(win, OUT/"scene-indoors.png", 1280, 720)
                indoors = crop(OUT/"scene-indoors.png", OUT/"bird-scene-absent.png")
                assert difference(hidden, indoors) < 700, "scene-specific bird escaped its scene"
                assert proc.poll() is None, "ambient fixture crashed"
                print("QOL06: six sizes, show/hide, scene scope and F5/F9 PASS", flush=True)
            finally:
                if proc.poll() is None:
                    proc.terminate()
                    proc.wait(timeout=10)

if __name__ == "__main__":
    main()
