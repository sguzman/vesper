"""Regression: Vesper's sprites and UI must reflow when the OS resizes the window.

Runs under Linux Xvfb with the normal external fairy-tale assets. Does not
require the user to run a script or manually resize a window.
"""
from __future__ import annotations
import os
from pathlib import Path
import subprocess
import tempfile
import time

ROOT = Path(__file__).resolve().parents[2]
PLAYER = ROOT / "target/debug/vesper-player"


def call(*args: str, check: bool = True) -> subprocess.CompletedProcess:
    return subprocess.run(args, text=True, capture_output=True, check=check)


def find_window() -> str:
    end = time.monotonic() + 30
    while time.monotonic() < end:
        match = call("xdotool", "search", "--name", "The Little Lantern", check=False)
        if match.returncode == 0 and match.stdout.strip():
            return match.stdout.splitlines()[0]
        time.sleep(0.3)
    raise AssertionError("Vesper player window did not appear")


def wait_for(log: Path, string: str) -> None:
    end = time.monotonic() + 12
    while time.monotonic() < end:
        if string in log.read_text(errors="replace"):
            return
        time.sleep(0.3)
    raise AssertionError(f"missing resize event {string!r}\n{log.read_text(errors='replace')[-4000:]}")


def image_size(path: Path) -> tuple[int, int]:
    return tuple(map(int, call("identify", "-format", "%w %h", str(path)).stdout.split()))


def unique_in_strip(path: Path, x: int, y: int, width: int, height: int) -> int:
    return int(call(
        "convert", str(path),
        "-crop", f"{width}x{height}+{x}+{y}",
        "+repage", "-format", "%k", "info:"
    ).stdout)


def main() -> None:
    out = Path("/tmp/vesper-responsive-qa")
    out.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="vesper-resize-") as temp:
        log = out / "player.log"
        env = dict(os.environ, LIBGL_ALWAYS_SOFTWARE="1",
                   VESPER_SAVE_DIR=str(Path(temp) / "saves"))
        with log.open("w") as stdout:
            process = subprocess.Popen(
                [str(PLAYER), str(ROOT / "examples/little-lantern")],
                env=env, stdout=stdout, stderr=subprocess.STDOUT,
            )
            try:
                win = find_window()
                wait_for(log, "Vesper viewport resized: 1280x720")
                for width, height in [(1600, 900), (900, 700), (1920, 1080)]:
                    call("xdotool", "windowsize", "--sync", win, str(width), str(height))
                    wait_for(log, f"Vesper viewport resized: {width}x{height}")
                    frame = out / f"frame-{width}x{height}.png"
                    strip_height = min(height - 160, 440)
                    # Window existence and resize logs can precede the first
                    # presented frame on software GPU runners. Poll real pixels,
                    # but fail with the last screenshot if edges stay letterboxed.
                    deadline = time.monotonic() + 12
                    attempt = 0
                    while True:
                        attempt += 1
                        time.sleep(0.4)
                        call("import", "-window", win, str(frame))
                        size = image_size(frame)
                        assert size == (width, height), f"unexpected screenshot size: {size}"
                        left = unique_in_strip(frame, 0, 100, 110, strip_height)
                        right = unique_in_strip(frame, width - 110, 100, strip_height)
                        if left >= 3 and right >= 3:
                            break
                        if time.monotonic() >= deadline:
                            raise AssertionError(
                                f"{width}x{height}: scene never reached both edges "
                                f"after {attempt} captures (left={left}, right={right}); "
                                f"last screenshot retained: {frame}"
                            )
                    left_pixel = call("convert", str(frame), "-format",
                                      "%[pixel:p{30,300}]", "info:").stdout
                    middle_pixel = call("convert", str(frame), "-format",
                                        f"%[pixel:p{{{width // 2},300}}]", "info:").stdout
                    right_pixel = call("convert", str(frame), "-format",
                                       f"%[pixel:p{{{width - 30},300}}]", "info:").stdout
                    print(f"Frame {width}x{height}: colors left={left}, right={right}; "
                          f"pixel left={left_pixel}, center={middle_pixel}, right={right_pixel}",
                          flush=True)
                    # Previously these strips were flat charcoal after resizing
                    # because the background remained hard-coded at 1280x720.
                    assert left >= 3 and right >= 3, (
                        f"{width}x{height}: side edges were letterboxed, "
                        f"left={left} colors, right={right} colors"
                    )
                    print(f"{width}x{height}: art reaches edges "
                          f"(left {left} colors, right {right} colors)", flush=True)
                assert process.poll() is None, "player unexpectedly exited"
                print("Dynamic background, actor and UI resize assertions passed", flush=True)
            finally:
                process.terminate()
                try:
                    process.wait(timeout=6)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait()


if __name__ == "__main__":
    main()
