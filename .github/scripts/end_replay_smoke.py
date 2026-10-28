"""End-of-story keyboard regression: R begins a fresh playthrough."""
from __future__ import annotations
import os
from pathlib import Path
import subprocess
import tempfile
import time

ROOT = Path(__file__).resolve().parents[2]


def cmd(*args: str) -> subprocess.CompletedProcess:
    return subprocess.run(args, text=True, capture_output=True, check=True)


def wait_for(log: Path, message: str, timeout: float = 12) -> None:
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        if message in log.read_text(errors="replace"):
            return
        time.sleep(.2)
    raise AssertionError(f"missing {message}: {log.read_text(errors='replace')[-4000:]}")


def main() -> None:
    with tempfile.TemporaryDirectory(prefix="vesper-replay-") as tmp:
        log = Path(tmp) / "player.log"
        env = dict(os.environ, LIBGL_ALWAYS_SOFTWARE="1", VESPER_SAVE_DIR=tmp)
        with log.open("w") as out:
            process = subprocess.Popen([
                str(ROOT / "target/debug/vesper-player"),
                str(ROOT / "examples/little-lantern"),
            ], stdout=out, stderr=subprocess.STDOUT, env=env)
            try:
                win = ""
                for _ in range(60):
                    found = subprocess.run(["xdotool", "search", "--name", "The Little Lantern"],
                                           capture_output=True, text=True)
                    if found.returncode == 0 and found.stdout.strip():
                        win = found.stdout.strip().splitlines()[0]
                        break
                    time.sleep(.25)
                assert win, "window never appeared"
                time.sleep(2)
                cmd("xdotool", "windowfocus", "--sync", win)
                for _ in range(3):
                    cmd("xdotool", "key", "--clearmodifiers", "space")
                    time.sleep(.25)
                cmd("xdotool", "key", "--clearmodifiers", "1")
                for _ in range(5):
                    time.sleep(.3)
                    cmd("xdotool", "key", "--clearmodifiers", "space")
                wait_for(log, "Vesper story reached ending scene: goodnight")
                cmd("xdotool", "key", "--clearmodifiers", "r")
                wait_for(log, "Vesper story restarted from its beginning")
                time.sleep(.8)
                assert process.poll() is None, "player exited during restart"
                cmd("xdotool", "key", "--clearmodifiers", "Escape")
                process.wait(timeout=10)
                assert process.returncode == 0
                print("End-of-story restart preserved the running Bevy process", flush=True)
            finally:
                if process.poll() is None:
                    process.terminate()
                    process.wait(timeout=6)


if __name__ == "__main__":
    main()
