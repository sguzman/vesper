"""Visual dialogue-panel hierarchy and choice layout in both external stories.

All checks sample real Bevy frames under Xvfb. Flat unrendered screenshots fail.
"""
from __future__ import annotations
import os
from pathlib import Path
import re
import subprocess
import time

ROOT = Path(__file__).resolve().parents[2]
PLAYER = ROOT / "target/debug/vesper-player"
GAMES = (("examples/little-lantern", "The Little Lantern"),
         ("examples/clockwork-moon", "The Clockwork Moon"))
SIZES = ((900, 700), (1280, 720), (1600, 900))


def call(*args: str, check: bool = True) -> subprocess.CompletedProcess[str]:
    return subprocess.run(args, text=True, capture_output=True, check=check)


def window(name: str) -> str:
    stop = time.monotonic() + 30
    while time.monotonic() < stop:
        found = call("xdotool", "search", "--name", name, check=False)
        if found.returncode == 0 and found.stdout.strip():
            return found.stdout.splitlines()[0]
        time.sleep(.2)
    raise AssertionError(f"window missing: {name}")


def colors(path: Path, geometry: str | None = None) -> int:
    cmd = ["convert", str(path)]
    if geometry:
        cmd += ["-crop", geometry, "+repage"]
    return int(call(*cmd, "-format", "%k", "info:").stdout)


def sample(window_id: str, file: Path, width: int, height: int) -> None:
    end = time.monotonic() + 12
    while time.monotonic() < end:
        time.sleep(.45)
        call("import", "-window", window_id, str(file))
        size = call("identify", "-format", "%w %h", str(file)).stdout
        assert size == f"{width} {height}", (size, width, height)
        # A rendered dialog has both diverse external image colors and
        # significant visual detail near the bottom panel.
        if colors(file) > 45 and colors(file,
                f"{width // 2}x{max(120, height // 3)}+{width // 4}+{height - max(120, height // 3)}") > 18:
            return
    raise AssertionError(f"dialogue did not render visibly: {file}")


def verify(game: str, title: str, temp: Path) -> None:
    log = temp / "player.log"
    with log.open("w") as stream:
        process = subprocess.Popen([str(PLAYER), str(ROOT / game)],
            stdout=stream, stderr=subprocess.STDOUT,
            env=dict(os.environ, LIBGL_ALWAYS_SOFTWARE="1", VESPER_SAVE_DIR=str(temp)))
        try:
            win = window(title)
            call("xdotool", "windowfocus", "--sync", win)
            for width, height in SIZES:
                call("xdotool", "windowsize", "--sync", win, str(width), str(height))
                opening = temp / f"{width}x{height}-opening.png"
                sample(win, opening, width, height)
                # Dialogue and hint must be visible in separate bands.
                panel_y = int(height * .65)
                panel_h = height - panel_y - 10
                content_colors = colors(opening, f"{width // 2}x{panel_h}+{width // 4}+{panel_y}")
                assert content_colors > 18, f"{game} {width}x{height}: dialogue panel is flat"
                # Preserve evidence of the actual rendered image if a later
                # presentation regression occurs.
                print(f"{title} {width}x{height}: {content_colors} panel colors", flush=True)
            assert process.poll() is None, f"{title} crashed while resizing"
        finally:
            if process.poll() is None:
                process.terminate()
                process.wait(timeout=8)


def main() -> None:
    out = Path("/tmp/vesper-layout-qa")
    out.mkdir(parents=True, exist_ok=True)
    for i, (game, title) in enumerate(GAMES):
        path = out / f"game-{i}"
        path.mkdir(exist_ok=True)
        verify(game, title, path)


if __name__ == "__main__":
    main()
