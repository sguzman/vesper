"""Startup and game-switch verification of player preferences (real Bevy pixels).

No player settings UI is expected in VX01. Prepare external config files,
launch the unchanged game and compare rendered screens without touching assets.
"""
from __future__ import annotations
import os
from pathlib import Path
import re
import subprocess
import tempfile
import time
import json

ROOT = Path(__file__).resolve().parents[2]
PLAYER = ROOT / "target/debug/vesper-player"
OUT = Path("/tmp/vesper-preferences-qa")
GAMES = (
    ("little-lantern", "The Little Lantern"),
    ("clockwork-moon", "The Clockwork Moon"),
)

def run(*args: str, check: bool = True):
    return subprocess.run(args, check=check, capture_output=True, text=True)

def window(name: str) -> str:
    end = time.monotonic() + 18
    while time.monotonic() < end:
        match = run("xdotool", "search", "--name", name, check=False)
        if match.returncode == 0 and match.stdout.strip():
            return match.stdout.splitlines()[0]
        time.sleep(.2)
    raise AssertionError(f"Window did not appear: {name}")

def frame(game: str, title: str, folder: Path, variant: str,
          content: object | None) -> tuple[Path, str]:
    prefs = folder / f"{game}-{variant}"
    prefs.mkdir(parents=True)
    path = prefs / "player-preferences.json"
    if content is not None:
        path.write_text(json.dumps(content) if not isinstance(content, str) else content)
    output = OUT / f"{game}-{variant}.png"
    log = OUT / f"{game}-{variant}.log"
    env = dict(os.environ, LIBGL_ALWAYS_SOFTWARE="1",
               VESPER_PREFERENCES_DIR=str(prefs),
               VESPER_SAVE_DIR=str(folder / "save"))
    with log.open("w") as stdout:
        process = subprocess.Popen(
            [str(PLAYER), str(ROOT / "examples" / game)],
            stdout=stdout, stderr=subprocess.STDOUT, env=env)
        try:
            win = window(title)
            run("xdotool", "windowfocus", "--sync", win)
            # Allow animated initial fade to complete before comparison.
            time.sleep(2.1)
            run("import", "-window", win, str(output))
            assert run("identify", "-format", "%w %h", str(output)).stdout == "1280 720"
            assert process.poll() is None
        finally:
            if process.poll() is None:
                process.terminate()
                process.wait(timeout=8)
    return output, log.read_text(errors="replace")

def diff(first: Path, second: Path) -> int:
    comparison = run("compare", "-metric", "AE",
                     str(first), str(second), "null:", check=False)
    match = re.match(r"^\d+", comparison.stderr.strip())
    assert match, comparison.stderr
    return int(match.group())

def main():
    OUT.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="vesper-prefs-smoke-") as tmp:
        folder = Path(tmp)
        for game, title in GAMES:
            base, _ = frame(game, title, folder, "baseline", None)
            modified, _ = frame(game, title, folder, "overrides", {
                "format_version": 1, "panel_opacity": .62,
                "text_scale": 1.5, "font_family": "monospace"
            })
            assert diff(base, modified) > 1200, f"{game}: player overrides invisible"
            corrupt, corrupt_log = frame(game, title, folder, "corrupt", "{bad json")
            assert "preferences ignored" in corrupt_log.lower()
            assert diff(base, corrupt) < 250, f"{game}: corrupt config changed UI"
            missing, _ = frame(game, title, folder, "missingfont", {
                "format_version": 1, "font_family": "This Font Cannot Exist 999"
            })
            assert diff(base, missing) < 250, f"{game}: missing font did not fall back"
            legacy, _ = frame(game, title, folder, "legacy", {"panel_opacity": .62})
            assert diff(base, legacy) > 500, f"{game}: V0 preferences migration invisible"
            print(f"{game}: V1 overrides, legacy migration, missing and invalid fallbacks pass", flush=True)

if __name__ == "__main__":
    main()
