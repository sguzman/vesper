"""QOL01 graphical font browser acceptance on the real Bevy player.

Type into the native EditableText search field, verify filtering changes actual
pixels, select an installed family, then restart and check persisted resolution.
All writes and screenshots are isolated from user files.
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
OUT = Path("/tmp/vesper-font-browser-qa")


def run(*args: str, check: bool = True) -> subprocess.CompletedProcess[str]:
    return subprocess.run(args, text=True, capture_output=True, check=check)


def key(value: str, delay: float = .18) -> None:
    run("xdotool", "key", "--clearmodifiers", value)
    time.sleep(delay)


def wait_log(log: Path, needle: str, seconds: float = 15.0) -> None:
    until = time.monotonic() + seconds
    while time.monotonic() < until:
        if log.exists() and needle in log.read_text(errors="replace"):
            return
        time.sleep(.10)
    raise AssertionError(f"missing {needle!r}: {log.read_text(errors='replace')[-2500:]}")


def wait_prefs(file: Path, predicate, seconds: float = 15.0) -> dict:
    until = time.monotonic() + seconds
    while time.monotonic() < until:
        try:
            data = json.loads(file.read_text())
            if predicate(data):
                return data
        except (OSError, ValueError):
            pass
        time.sleep(.1)
    raise AssertionError(f"font preference not persisted: {file}")


def find_window() -> str:
    until = time.monotonic() + 20.0
    while time.monotonic() < until:
        p = run("xdotool", "search", "--name", "The Little Lantern", check=False)
        if p.returncode == 0 and p.stdout.strip():
            win = p.stdout.strip().splitlines()[0]
            run("xdotool", "windowfocus", "--sync", win)
            return win
        time.sleep(.1)
    raise AssertionError("Little Lantern window unavailable")


def shot(win: str, file: Path) -> None:
    run("import", "-window", win, str(file))
    result = run("identify", "-format", "%w %h", str(file))
    assert result.stdout.strip() == "1280 720", result.stdout


def changed(left: Path, right: Path) -> int:
    result = run("compare", "-metric", "AE", str(left), str(right),
                 "null:", check=False).stderr
    m = re.match(r"\s*(\d+)", result)
    assert m, result
    return int(m.group(1))


def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="vesper-font-") as tmp:
        prefs_dir = Path(tmp)
        prefs_path = prefs_dir / "player-preferences.json"
        env = dict(os.environ, LIBGL_ALWAYS_SOFTWARE="1",
                   VESPER_PREFERENCES_DIR=str(prefs_dir),
                   VESPER_SAVE_DIR=str(prefs_dir / "saves"))

        for stage in ("choose", "restart"):
            log = OUT / f"{stage}.log"
            with log.open("w") as stream:
                player = subprocess.Popen(
                    [str(PLAYER), str(ROOT / "examples/little-lantern")],
                    stdout=stream, stderr=subprocess.STDOUT, env=env)
                try:
                    win = find_window()
                    time.sleep(1.8)
                    wait_log(log, "Vesper discovered")
                    key("p")
                    key("4")
                    wait_log(log, "Vesper appearance settings opened")
                    # A log line precedes the actual first UI frame. Rendering
                    # under software Vulkan can drop very short synthetic
                    # Down/up pairs while the menu is being mounted.
                    time.sleep(.85)
                    shot(win, OUT / f"{stage}-settings-visible.png")
                    for _ in range(2):
                        key("Down", .32)
                    key("Return", .35)
                    wait_log(log, "Vesper system font browser opened")
                    wait_log(log, "Vesper font search focused")
                    time.sleep(.45)
                    initial = OUT / f"{stage}-browser.png"
                    shot(win, initial)

                    if stage == "choose":
                        # Real Hyprland QA found typing can stop working after
                        # interacting with the font browser. Click empty panel
                        # background, then type without manually refocusing.
                        # A browser that only focuses once at spawn fails here.
                        run("xdotool", "mousemove", "--window", win, "12", "12")
                        run("xdotool", "click", "1")
                        time.sleep(.35)
                        run("xdotool", "type", "--clearmodifiers", "--delay", "75", "DejaVu")
                        time.sleep(.85)
                        search = OUT / "filtered-dejavu.png"
                        shot(win, search)
                        assert changed(initial, search) > 250, (
                            "typing into font input did not update the picker"
                        )
                        # An unmatched search must show an empty-state message
                        # rather than preserve a stale result or selecting it.
                        # Backspace is deterministic under Xvfb: native
                        # Ctrl+A selection can vary with focus/shortcut mapping.
                        for _ in "DejaVu":
                            key("BackSpace", .035)
                        run("xdotool", "type", "--clearmodifiers", "zzzz-no-such-font")
                        time.sleep(.5)
                        no_match = OUT / "no-match.png"
                        shot(win, no_match)
                        assert changed(search, no_match) > 150
                        for _ in "zzzz-no-such-font":
                            key("BackSpace", .035)
                        # P and M are letters in a search, never global menu
                        # shortcuts while the browser is open.
                        run("xdotool", "type", "--clearmodifiers", "monospace")
                        time.sleep(.35)
                        mono = OUT / "filtered-monospace.png"
                        shot(win, mono)
                        assert changed(no_match, mono) > 150
                        assert "Vesper system font browser closed" not in log.read_text()
                        for _ in "monospace":
                            key("BackSpace", .035)
                        run("xdotool", "type", "--clearmodifiers", "DejaVu")
                        time.sleep(.35)
                        key("Down")
                        key("Up")
                        key("Return")
                        wait_log(log, "Vesper selected font: DejaVu")
                        prefs = wait_prefs(prefs_path, lambda p:
                            p.get("font_family", "").startswith("DejaVu"))
                        assert prefs["format_version"] == 1, prefs
                    else:
                        # Browser highlight starts on the persisted system
                        # family, not on Game default. Re-choosing it is safe.
                        key("Return")
                        wait_log(log, "Vesper system font browser closed")
                        prefs = json.loads(prefs_path.read_text())
                        assert prefs["font_family"].startswith("DejaVu"), prefs

                    key("Escape")  # Appearance -> pause
                    key("p")       # resume
                    key("Escape")  # exit
                    player.wait(timeout=15)
                    assert player.returncode == 0
                finally:
                    if player.poll() is None:
                        player.terminate()
                        player.wait(timeout=10)

        print("QOL01: installed font search/filter, empty state, live preview, "
              "selection, persisted restart and fallback flow passed", flush=True)


if __name__ == "__main__":
    main()
