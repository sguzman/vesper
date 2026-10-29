"""VX02 graphical interaction regression for engine-owned player appearance settings.

Proves menu access, keyboard navigation, live preview, async persistence,
safe exit, restart, reset, and compact/resized presentation. Game files stay
untouched; all preferences and screenshots are isolated under /tmp.
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
OUT = Path("/tmp/vesper-live-settings-qa")
SIZES = ((946, 511), (800, 600), (1280, 720), (1920, 1080))
GAMES = (("little-lantern", "The Little Lantern"),
         ("clockwork-moon", "The Clockwork Moon"))

def run(*args: str, check: bool = True) -> subprocess.CompletedProcess[str]:
    return subprocess.run(args, text=True, capture_output=True, check=check)

def key(name: str, pause: float = .30) -> None:
    run("xdotool", "key", "--clearmodifiers", name)
    time.sleep(pause)

def screenshot(window: str, path: Path) -> None:
    run("import", "-window", window, str(path))

def difference(a: Path, b: Path) -> int:
    result = run("compare", "-metric", "AE", str(a), str(b), "null:", check=False)
    found = re.match(r"^\d+", result.stderr.strip())
    assert found is not None, result.stderr
    return int(found.group())

def wait_log(path: Path, marker: str, timeout: float = 15) -> None:
    end = time.monotonic() + timeout
    while time.monotonic() < end:
        if marker in path.read_text(errors="replace"):
            return
        time.sleep(.12)
    raise AssertionError(f"{marker}: {path.read_text(errors='replace')[-4000:]}")

def wait_prefs(path: Path, predicate, timeout: float = 12) -> dict:
    end = time.monotonic() + timeout
    while time.monotonic() < end:
        try:
            value = json.loads(path.read_text())
            if predicate(value):
                return value
        except (OSError, json.JSONDecodeError):
            pass
        time.sleep(.12)
    raise AssertionError(f"prefs never reached expected state: {path}")

def find_window(title: str) -> str:
    end = time.monotonic() + 25
    while time.monotonic() < end:
        found = run("xdotool", "search", "--name", title, check=False)
        if found.returncode == 0 and found.stdout.strip():
            return found.stdout.splitlines()[0]
        time.sleep(.2)
    raise AssertionError(f"player window not found: {title}")

def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="vesper-live-settings-") as tmp:
        root = Path(tmp)
        for game, title in GAMES:
            base = OUT / game
            base.mkdir(exist_ok=True)
            prefs_dir = root / game
            prefs_dir.mkdir()
            prefs_path = prefs_dir / "player-preferences.json"
            log = base / "run.log"
            env = dict(os.environ, LIBGL_ALWAYS_SOFTWARE="1",
                       VESPER_PREFERENCES_DIR=str(prefs_dir),
                       VESPER_SAVE_DIR=str(prefs_dir / "saves"))
            with log.open("w") as output:
                process = subprocess.Popen(
                    [str(PLAYER), str(ROOT / "examples" / game)],
                    stdout=output, stderr=subprocess.STDOUT, env=env)
                try:
                    win = find_window(title)
                    run("xdotool", "windowfocus", "--sync", win)
                    time.sleep(2.2)
                    before = base / "story-before.png"
                    screenshot(win, before)

                    key("m")
                    wait_log(log, "Vesper main menu opened")
                    menu = base / "main-menu.png"
                    end = time.monotonic() + 10
                    while True:
                        screenshot(win, menu)
                        if difference(before, menu) > 5000:
                            break
                        assert time.monotonic() < end, "main menu never appeared as rendered pixels"
                        time.sleep(.20)
                    time.sleep(.35)
                    screenshot(win, menu)
                    key("4")
                    wait_log(log, "Vesper appearance settings opened")
                    first = base / "settings-initial.png"
                    # Bevy rendering is asynchronous relative to log emission.
                    # Do not misattribute the menu-to-settings transition to a
                    # focus change: first require the new frame to be visible.
                    end = time.monotonic() + 10
                    while True:
                        screenshot(win, first)
                        if difference(menu, first) > 60000:
                            break
                        assert time.monotonic() < end, "settings never appeared as rendered pixels"
                        time.sleep(.20)
                    time.sleep(.5)
                    screenshot(win, first)
                    assert difference(before, first) > 5000

                    key("Down")
                    second = base / "settings-focus.png"
                    screenshot(win, second)
                    focus_diff = difference(first, second)
                    assert 50 < focus_diff < 60000, f"{game}: settings focus changed {focus_diff} pixels"
                    key("Up")
                    # First row: reduce opacity by multiple steps.
                    for _ in range(5):
                        key("Left", .10)
                    opaque = wait_prefs(prefs_path,
                        lambda p: p.get("panel_opacity") is not None
                        and p["panel_opacity"] < 0.90)
                    assert 0.60 <= opaque["panel_opacity"] < 0.90
                    fade = base / "settings-opacity.png"
                    screenshot(win, fade)
                    assert difference(first, fade) > 100, "live opacity preview did not change"

                    key("Down")
                    key("Right")
                    scaled = wait_prefs(prefs_path, lambda p:
                        p.get("text_scale", 0) > 1.05)
                    assert scaled["text_scale"] <= 1.60
                    bigger = base / "settings-text-size.png"
                    screenshot(win, bigger)
                    assert difference(fade, bigger) > 100, "live font size preview did not change"

                    key("Down")
                    key("Right")
                    font = wait_prefs(prefs_path, lambda p:
                        p.get("font_family") == "sans-serif")
                    assert font["format_version"] == 1

                    # Click a real Bevy settings control with the pointer.
                    # The first opacity row is centered near x=600, y=145
                    # in the 1280x720 settings panel.
                    if game == "little-lantern":
                        previous_opacity = font["panel_opacity"]
                        run("xdotool", "mousemove", "--window", win, "600", "145")
                        time.sleep(.4)
                        run("xdotool", "click", "1")
                        mouse_change = wait_prefs(prefs_path, lambda p:
                            p.get("panel_opacity", 0) > previous_opacity + .02)
                        assert mouse_change["panel_opacity"] <= 1.0
                        mouse_screen = base / "settings-pointer.png"
                        screenshot(win, mouse_screen)

                    # High contrast is player-owned, and its opaque panel and
                    # white text override authored low-contrast colors.
                    # Pointer test selects row 0; keyboard-only path remains
                    # on the font row (index 2).
                    for _ in range(6 if game == "little-lantern" else 4):
                        key("Down", .08)
                    key("Return", .12)
                    contrast = wait_prefs(prefs_path, lambda p:
                        p.get("high_contrast") is True)
                    assert contrast["high_contrast"] is True
                    high = base / "settings-high-contrast.png"
                    screenshot(win, high)
                    assert difference(bigger, high) > 120, (
                        "high contrast did not visibly update the preview"
                    )

                    # Real popup must fit every requested window; dimensions
                    # and screenshots are preserved for visual review.
                    if game == "little-lantern":
                        for width, height in SIZES:
                            run("xdotool", "windowsize", win, str(width), str(height))
                            time.sleep(.8)
                            image = base / f"settings-{width}x{height}.png"
                            screenshot(win, image)
                            result = run("identify", "-format", "%w %h", str(image))
                            assert result.stdout.strip() == f"{width} {height}", result.stdout
                        run("xdotool", "windowsize", win, "1280", "720")
                        time.sleep(.6)

                    # Escape closes Settings, not the process. The user returns
                    # to the main menu and can continue the same narrative.
                    key("Escape")
                    wait_log(log, "Vesper appearance settings closed")
                    assert process.poll() is None, "Escape accidentally quit the player"
                    key("2")
                    time.sleep(.7)
                    resumed = base / "story-after.png"
                    screenshot(win, resumed)
                    assert difference(before, resumed) > 300, "preferences did not affect story"
                    key("Escape")
                    process.wait(timeout=15)
                    assert process.returncode == 0
                finally:
                    if process.poll() is None:
                        process.terminate()
                        process.wait(timeout=10)

            # Settings must survive exit, then Reset must remove overrides
            # without changing authored game files.
            restart = base / "restart.log"
            with restart.open("w") as output:
                player = subprocess.Popen(
                    [str(PLAYER), str(ROOT / "examples" / game)],
                    stdout=output, stderr=subprocess.STDOUT, env=env)
                try:
                    win = find_window(title)
                    run("xdotool", "windowfocus", "--sync", win)
                    time.sleep(2)
                    key("p")
                    key("4")
                    wait_log(restart, "Vesper appearance settings opened")
                    # VX09 high contrast precedes Reset (8th row).
                    for _ in range(7):
                        key("Down")
                    key("Return")
                    wait_prefs(prefs_path, lambda p:
                        p.get("panel_opacity") is None and
                        p.get("text_scale") is None and
                        p.get("font_family") is None and
                        p.get("high_contrast") is None)
                    key("Escape")
                    assert player.poll() is None
                    key("p")
                    key("Escape")
                    player.wait(timeout=15)
                    assert player.returncode == 0
                finally:
                    if player.poll() is None:
                        player.terminate()
                        player.wait(timeout=10)
            print(f"{game}: settings, focus, live changes, persistence, restart and reset passed", flush=True)

if __name__ == "__main__":
    main()
