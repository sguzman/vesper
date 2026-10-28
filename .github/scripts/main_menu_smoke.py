"""Exercise the main menu without changing the two external game files."""
from __future__ import annotations
import os
from pathlib import Path
import re
import subprocess
import tempfile
import time

ROOT = Path(__file__).resolve().parents[2]
GAMES = (("examples/little-lantern", "The Little Lantern"),
         ("examples/clockwork-moon", "The Clockwork Moon"))

def run(*args: str) -> None:
    subprocess.run(args, check=True, capture_output=True, text=True)

def change(a: Path, b: Path) -> int:
    result = subprocess.run(["compare", "-metric", "AE", str(a), str(b), "null:"],
                            capture_output=True, text=True)
    metric = re.match(r"^\d+", result.stderr.strip())
    assert metric is not None, result.stderr
    return int(metric.group())

def key(value: str) -> None:
    run("xdotool", "key", "--clearmodifiers", value)
    time.sleep(.5)

def verify(game: str, title: str, folder: Path) -> None:
    log = folder / "session.log"
    with log.open("w") as output:
        process = subprocess.Popen([str(ROOT / "target/debug/vesper-player"), str(ROOT / game)],
            stdout=output, stderr=subprocess.STDOUT,
            env=dict(os.environ, LIBGL_ALWAYS_SOFTWARE="1", VESPER_SAVE_DIR=str(folder)))
        try:
            window = None
            for _ in range(60):
                found = subprocess.run(["xdotool", "search", "--name", title],
                                       capture_output=True, text=True)
                if found.returncode == 0 and found.stdout.strip():
                    window = found.stdout.strip().splitlines()[0]
                    break
                time.sleep(.25)
            assert window, f"{title}: window missing"
            time.sleep(4)
            run("xdotool", "windowfocus", "--sync", window)
            before, menu, after = (folder / n for n in ("before.png", "menu.png", "after.png"))
            run("import", "-window", window, str(before))
            key("m")
            run("import", "-window", window, str(menu))
            assert change(before, menu) > 5000, f"{title}: no visible menu"
            key("Down")
            key("Return")  # Continue must leave the current frame unchanged
            run("import", "-window", window, str(after))
            assert change(before, after) < 3000, f"{title}: Continue changed the story"
            key("space")  # advance story before restarting
            key("m")
            key("Return")  # Start resets in-memory story
            assert "restarted from its beginning via menu" in log.read_text(errors="replace")
            key("p")
            key("Down")
            key("Return")  # Pause option 2 -> Main menu
            key("Up")  # Start -> Quit
            key("Return")
            process.wait(timeout=12)
            assert process.returncode == 0
            assert "Vesper quitting through menu" in log.read_text(errors="replace")
            print(f"{title}: menu start, continue, pause navigation and quit passed", flush=True)
        finally:
            if process.poll() is None:
                process.terminate()
                process.wait(timeout=10)

for game, title in GAMES:
    with tempfile.TemporaryDirectory(prefix="vesper-menu-") as tmp:
        verify(game, title, Path(tmp))
