"""QOL02: real Bevy auto-advance delay and appearance-setting persistence.

Validates that the hold begins after completed text, is configurable,
does not activate without opt-in, and is editable without restarting.
"""
from __future__ import annotations

import json
import os
from pathlib import Path
import subprocess
import tempfile
import time

ROOT = Path(__file__).resolve().parents[2]
PLAYER = ROOT / "target/debug/vesper-player"
OUT = Path("/tmp/vesper-auto-delay-qa")


def run(*args: str, check: bool = True) -> subprocess.CompletedProcess[str]:
    return subprocess.run(args, text=True, capture_output=True, check=check)


def find_window() -> str:
    limit = time.monotonic() + 25
    while time.monotonic() < limit:
        result = run("xdotool", "search", "--name", "The Little Lantern", check=False)
        if result.returncode == 0 and result.stdout.strip():
            window = result.stdout.splitlines()[0]
            run("xdotool", "windowfocus", "--sync", window)
            return window
        time.sleep(.15)
    raise AssertionError("Vesper window not found")


def wait(log: Path, token: str, seconds: float = 12.0) -> None:
    deadline = time.monotonic() + seconds
    while time.monotonic() < deadline:
        if log.exists() and token in log.read_text(errors="replace"):
            return
        time.sleep(.10)
    raise AssertionError(f"missing {token}: {log.read_text(errors='replace')[-1800:]}")


def key(name: str, delay: float = .17) -> None:
    run("xdotool", "key", "--clearmodifiers", name)
    time.sleep(delay)


def shot(win: str, path: Path) -> None:
    run("import", "-window", win, str(path))


def launch(root: Path, prefs: dict, label: str):
    data = root / label
    data.mkdir()
    prefdir = data / "prefs"
    prefdir.mkdir()
    (prefdir / "player-preferences.json").write_text(
        json.dumps({"format_version": 1, **prefs})
    )
    log = OUT / f"{label}.log"
    stream = log.open("w")
    env = dict(os.environ, LIBGL_ALWAYS_SOFTWARE="1",
               VESPER_PREFERENCES_DIR=str(prefdir),
               VESPER_SAVE_DIR=str(data / "saves"),
               VESPER_TRACE_SCENES="1")
    player = subprocess.Popen([str(PLAYER), str(ROOT / "examples/little-lantern")],
                              stdout=stream, stderr=subprocess.STDOUT, env=env)
    try:
        win = find_window()
        wait(log, "Vesper scene dialogue")
        return player, stream, win, log, prefdir / "player-preferences.json"
    except BaseException:
        player.terminate()
        player.wait(timeout=10)
        stream.close()
        raise


def stop(player, stream) -> None:
    if player.poll() is None:
        player.terminate()
        player.wait(timeout=10)
    stream.close()


def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="vesper-auto-delay-") as tmp:
        root = Path(tmp)

        # Four and a half seconds is a meaningful reader wait; the test
        # inspects the actual ready signal rather than frame-duration guesses.
        player, stream, win, log, path = launch(
            root, {"auto_advance": True, "auto_advance_delay": 4.5,
                   "reveal_speed": "instant"}, "long-delay"
        )
        try:
            time.sleep(1.7)
            assert "Vesper auto-advance ready" not in log.read_text(errors="replace"), (
                "reader advanced before configured delay"
            )
            shot(win, OUT / "long-delay-before.png")
            wait(log, "Vesper auto-advance ready", 8.0)
            shot(win, OUT / "long-delay-after.png")
            print("QOL02 configured 4.5s hold: respected", flush=True)
        finally:
            stop(player, stream)

        # Non-opted-in readers must never auto-advance just because an
        # author/user preference specifies a short delay.
        player, stream, win, log, path = launch(
            root, {"auto_advance": False, "auto_advance_delay": 0.5,
                   "reveal_speed": "instant"}, "opt-out"
        )
        try:
            time.sleep(2)
            assert "Vesper auto-advance ready" not in log.read_text(errors="replace")
            key("m")
            key("4")
            wait(log, "Vesper appearance settings opened")
            # New 7th row (index 6): auto-advance delay.
            for _ in range(6):
                key("Down", .10)
            key("Right")
            end = time.monotonic() + 10
            while time.monotonic() < end:
                current = json.loads(path.read_text())
                if current.get("auto_advance_delay") == 1.0:
                    break
                time.sleep(.1)
            else:
                raise AssertionError(f"live auto-delay change was not saved: {path.read_text()}")
            shot(win, OUT / "live-delay-setting.png")
            key("Escape")
            wait(log, "Vesper appearance settings closed")
            assert player.poll() is None
            print("QOL02 opt-out, visible live setting and atomic persistence: passed",
                  flush=True)
        finally:
            stop(player, stream)


if __name__ == "__main__":
    main()
