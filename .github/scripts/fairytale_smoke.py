"""Graphical smoke tests of two standalone external Vesper fairytales.

Run with xvfb-run after building the Bevy player. Captures real rendered
frames, verifies choice movement, image swaps, audio decoding, F5/F9 and the
ambitious game's live theme updates. Artifacts are uploaded for human review.
"""
from __future__ import annotations

import os
import shutil
import subprocess
import tempfile
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUTPUT = Path("/tmp/vesper-fairytale-qa")
PLAYER = ROOT / "target/debug/vesper-player"


def execute(*args: str, check: bool = True) -> subprocess.CompletedProcess:
    return subprocess.run(args, capture_output=True, text=True, check=check)


def window_id(title: str) -> str:
    limit = time.monotonic() + 25
    while time.monotonic() < limit:
        found = execute("xdotool", "search", "--name", title, check=False)
        if found.returncode == 0 and found.stdout.strip():
            return found.stdout.strip().splitlines()[0]
        time.sleep(.25)
    raise AssertionError(f"No player window appeared with title {title!r}")


def press(win: str, key: str, delay: float = .38) -> None:
    execute("xdotool", "windowfocus", "--sync", win)
    execute("xdotool", "key", "--clearmodifiers", key)
    time.sleep(delay)


def screen(win: str, name: str) -> Path:
    path = OUTPUT / f"{name}.png"
    execute("import", "-window", win, str(path))
    unique = int(execute("identify", "-format", "%k", str(path)).stdout)
    assert unique > 30, f"Rendered frame {name} had only {unique} colors"
    print(f"Screenshot {path.name}: {unique} colors", flush=True)
    return path


def differences(a: Path, b: Path) -> int:
    res = execute("compare", "-metric", "AE", str(a), str(b), "null:", check=False)
    import re
    matched = re.match(r"\s*(\d+)", res.stderr)
    assert matched, f"Image comparison failed: {res.stderr}"
    return int(matched.group(1))


def wait_log(path: Path, expected: str, timeout: float = 12.0) -> None:
    cutoff = time.monotonic() + timeout
    while time.monotonic() < cutoff:
        if expected in path.read_text(encoding="utf-8", errors="replace"):
            return
        time.sleep(.2)
    raise AssertionError(f"Missing {expected!r} in {path}:\n{path.read_text()[-4000:]}")


def wait_file(path: Path) -> None:
    deadline = time.monotonic() + 8
    while time.monotonic() < deadline:
        if path.exists() and path.stat().st_size > 100:
            return
        time.sleep(.2)
    raise AssertionError(f"Expected save file {path} was not written")


def start_game(path: Path, title: str, save_root: Path, log: Path, watch: bool = False):
    env = dict(os.environ, LIBGL_ALWAYS_SOFTWARE="1",
               VESPER_SAVE_DIR=str(save_root), VESPER_TRACE_SCENES="1")
    log_stream = log.open("w", encoding="utf-8")
    args = [str(PLAYER), str(path)] + (["--watch"] if watch else [])
    player = subprocess.Popen(args, stdout=log_stream, stderr=subprocess.STDOUT, env=env)
    try:
        win = window_id(title)
        time.sleep(2.0)
        return player, log_stream, win
    except Exception:
        player.terminate()
        player.wait(timeout=8)
        log_stream.close()
        raise


def stop_game(process: subprocess.Popen, handle, win: str) -> None:
    try:
        press(win, "Escape", .3)
        process.wait(timeout=8)
        assert process.returncode == 0, f"Player exited with code {process.returncode}"
    finally:
        if process.poll() is None:
            process.terminate()
            process.wait(timeout=8)
        handle.close()


def minimal_novel(root: Path) -> None:
    game = ROOT / "examples/little-lantern"
    process, handle, win = start_game(
        game, "The Little Lantern", root / "saves-simple", OUTPUT / "tiny.log"
    )
    try:
        opening = screen(win, "01-lantern-opening")
        for _ in range(3):
            press(win, "Return")
        choices = screen(win, "02-lantern-choices")
        assert differences(opening, choices) > 100
        press(win, "Down")
        alternative = screen(win, "03-lantern-choice-selected")
        assert differences(choices, alternative) > 30
        press(win, "Return")
        wait_log(OUTPUT / "tiny.log", "Vesper scene dialogue: walk_together")
        for _ in range(5):
            press(win, "Return")
        wait_log(OUTPUT / "tiny.log", "Vesper story reached ending scene: goodnight")
        ending = screen(win, "04-lantern-ending")
        assert differences(opening, ending) > 100
    finally:
        stop_game(process, handle, win)


def ambitious_novel(root: Path) -> None:
    game = root / "clockwork-moon"
    shutil.copytree(ROOT / "examples/clockwork-moon", game)
    saves = root / "saves-moon"
    process, handle, win = start_game(
        game, "The Clockwork Moon", saves, OUTPUT / "moon.log", watch=True
    )
    try:
        start = screen(win, "05-moon-cottage")
        wait_log(OUTPUT / "moon.log", "Vesper decoded audio asset: music:hearth")
        for _ in range(6):
            press(win, "Return")
        first_choice = screen(win, "06-moon-first-choice")
        assert differences(start, first_choice) > 100
        press(win, "2")
        wait_log(OUTPUT / "moon.log", "Vesper scene dialogue: river_trail")
        time.sleep(1)
        river = screen(win, "07-moon-river")
        assert differences(first_choice, river) > 1000

        for _ in range(5):
            press(win, "Return")
        wait_log(OUTPUT / "moon.log", "Vesper scene dialogue: clearing")
        glade = screen(win, "08-moon-clearing")
        assert differences(river, glade) > 1000
        for _ in range(4):
            press(win, "Return")
        second_choice = screen(win, "09-moon-four-choices")
        press(win, "2")
        wait_log(OUTPUT / "moon.log", "Vesper scene dialogue: tower_trial")
        tower = screen(win, "10-moon-tower")
        assert differences(glade, tower) > 1000

        for _ in range(5):
            press(win, "Return")
        wait_log(OUTPUT / "moon.log", "Vesper scene dialogue: moon_gate")
        time.sleep(1)
        gate = screen(win, "11-moon-gate")
        assert differences(tower, gate) > 1000

        # Save and load a complex game with branching variables and artwork.
        press(win, "F5")
        saved = saves / "clockwork-moon.json"
        wait_file(saved)
        press(win, "Return")
        changed = screen(win, "12-moon-after-advance")
        assert differences(gate, changed) > 20
        press(win, "F9")
        wait_log(OUTPUT / "moon.log", "Vesper save restored")
        time.sleep(1)
        restored = screen(win, "13-moon-restored")
        restored_pixels = differences(gate, restored)
        print(f"Advanced game's saved/restored image diff: {restored_pixels}", flush=True)
        assert restored_pixels < 2000

        # Live theme edits should update a running, paused story.
        theme = game / "styles/moonlight.toml"
        source = theme.read_text(encoding="utf-8")
        theme.write_text(source.replace("width_percent = 91.0", "width_percent = 68.0"),
                         encoding="utf-8")
        wait_log(OUTPUT / "moon.log", "Vesper hot reload applied")
        time.sleep(1)
        restyled = screen(win, "14-moon-live-theme")
        assert differences(restored, restyled) > 1000

        # Every WAV, sprite and background must be decodable, not merely named.
        for source_png in game.rglob("*.png"):
            execute("convert", str(source_png), "-resize", "1x1!", "null:")
        import wave
        for filename in (game / "assets/audio").glob("*.wav"):
            with wave.open(str(filename), "rb") as wav:
                assert wav.getnframes() > 1000
                assert wav.getsampwidth() == 2
    finally:
        stop_game(process, handle, win)


def main() -> None:
    OUTPUT.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="vesper-fairy-") as directory:
        root = Path(directory)
        minimal_novel(root)
        ambitious_novel(root)
    print("Both fairytales passed rendered integration QA", flush=True)


if __name__ == "__main__":
    main()
