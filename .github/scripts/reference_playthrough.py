"""Graphical end-to-end reference story coverage (two branches + fast-exit saves)."""
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


def run(*args: str, check: bool = True) -> subprocess.CompletedProcess:
    return subprocess.run(args, check=check, capture_output=True, text=True)


def find_window() -> str:
    deadline = time.monotonic() + 30
    while time.monotonic() < deadline:
        result = run("xdotool", "search", "--name", "Vesper Demonstration", check=False)
        if result.returncode == 0 and result.stdout.strip():
            return result.stdout.splitlines()[0]
        time.sleep(0.3)
    raise AssertionError("Vesper window not found")


def changed(first: Path, second: Path) -> int:
    result = run("compare", "-metric", "AE", str(first), str(second), "null:", check=False)
    match = re.match(r"\s*(\d+)", result.stderr)
    if not match:
        raise AssertionError(f"screenshot comparison failed: {result.stderr}")
    return int(match.group(1))


def play(choice: int, branch: str, steps: int, destination: Path) -> Path:
    save_dir = destination / f"save-{choice}"
    save_dir.mkdir()
    log = destination / f"play-{choice}.log"
    env = dict(os.environ,
               LIBGL_ALWAYS_SOFTWARE="1",
               VESPER_TRACE_SCENES="1",
               VESPER_SAVE_DIR=str(save_dir))
    with log.open("w", encoding="utf-8") as out:
        process = subprocess.Popen([str(PLAYER), str(ROOT / "game")],
                                   stdout=out, stderr=subprocess.STDOUT, env=env)
        try:
            win = find_window()
            time.sleep(3)
            first = destination / f"opening-{choice}.png"
            run("import", "-window", win, str(first))
            run("xdotool", "windowfocus", "--sync", win)
            run("xdotool", "key", "--clearmodifiers", "space")
            time.sleep(1)
            run("xdotool", "key", "--clearmodifiers", str(choice))
            time.sleep(1)
            for _ in range(steps):
                run("xdotool", "key", "--clearmodifiers", "space")
                time.sleep(0.9)
            end = destination / f"ending-{choice}.png"
            run("import", "-window", win, str(end))
            transcript = log.read_text(errors="replace")
            assert f"Vesper scene dialogue: {branch}" in transcript, transcript[-4000:]
            assert "Vesper scene dialogue: ending" in transcript, transcript[-4000:]
            assert "Vesper story reached ending scene: ending" in transcript, transcript[-4000:]
            assert not re.search(r"thread .* panicked|Story asset error", transcript), transcript[-4000:]
            delta = changed(first, end)
            print(f"Branch {choice} reached ending; opening/ending changed pixels: {delta}", flush=True)
            assert delta > 1000

            # Save and exit immediately: the shutdown path must join the
            # background save worker, leaving a complete JSON snapshot.
            run("xdotool", "key", "--clearmodifiers", "F5")
            run("xdotool", "key", "--clearmodifiers", "Escape")
            process.wait(timeout=12)
            assert process.returncode == 0, log.read_text(errors="replace")[-4000:]
            saved = save_dir / "vesper-demo.json"
            snapshot = json.loads(saved.read_text())
            assert snapshot["game_id"] == "vesper-demo"
            assert snapshot["machine"]["scene"] == "ending"
            assert not list(save_dir.glob("*.tmp"))
            return end
        finally:
            if process.poll() is None:
                process.terminate()
                process.wait(timeout=8)


def main() -> None:
    with tempfile.TemporaryDirectory(prefix="vesper-playthrough-") as tmp:
        root = Path(tmp)
        friendly = play(1, "friendly", 3, root)
        confused = play(2, "confused", 2, root)
        print(f"Both branches completed; ending screenshot difference: {changed(friendly, confused)}")


if __name__ == "__main__":
    main()
