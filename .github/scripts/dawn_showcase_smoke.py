"""End-to-end graphical Dawn Post Office acceptance under Linux/Xvfb.

Use the *unchanged* Vesper player and the checked-in authored game. Exercise
three independent complete paths, capture real game frames and preserve logs.
No test-only Rust code or game modifications are needed.
"""
from __future__ import annotations

import os
from pathlib import Path
import re
import subprocess
import tempfile
import time

ROOT = Path(__file__).resolve().parents[2]
PLAYER = ROOT / "target/debug/vesper-player"
GAME = ROOT / "examples/dawn-post-office"
OUTPUT = Path("/tmp/vesper-dawn-qa")
TITLE = "The Post Office at the Edge of Dawn"
ENDINGS = {
    1: "ending_homecoming",
    2: "ending_questions",
    3: "ending_lanterns",
}
SCENES = (
    "sorting_room", "sky_garden", "teacup_observatory",
    "cloud_quarter", "bell_roof", "horizon_gate",
)


def command(*args: str, check: bool = True) -> subprocess.CompletedProcess[str]:
    return subprocess.run(args, capture_output=True, text=True, check=check)


def find_window() -> str:
    deadline = time.monotonic() + 30
    while time.monotonic() < deadline:
        found = command("xdotool", "search", "--name", TITLE, check=False)
        if found.returncode == 0 and found.stdout.strip():
            return found.stdout.strip().splitlines()[0]
        time.sleep(.2)
    raise AssertionError("Dawn Post Office window never appeared")


def press(window: str, key: str, delay: float = .13) -> None:
    command("xdotool", "windowfocus", "--sync", window)
    command("xdotool", "key", "--clearmodifiers", key)
    time.sleep(delay)


def screenshot(window: str, path: Path) -> None:
    command("import", "-window", window, str(path))
    result = command("identify", "-format", "%w %h %k", str(path)).stdout
    width, height, unique = map(int, result.split())
    assert (width, height) == (1280, 720), (path, width, height)
    assert unique >= 50, f"Frame is blank or palette degenerate: {path}"


def changed(first: Path, second: Path) -> int:
    metric = command("compare", "-metric", "AE",
                     str(first), str(second), "null:", check=False).stderr
    m = re.match(r"\s*(\d+)", metric)
    assert m, metric
    return int(m.group(1))


def exercise_route(choice: int, root: Path) -> None:
    ending = ENDINGS[choice]
    save_dir = root / f"saves-{choice}"
    save_dir.mkdir()
    log = OUTPUT / f"route-{choice}.log"
    env = dict(os.environ, LIBGL_ALWAYS_SOFTWARE="1",
               VESPER_SAVE_DIR=str(save_dir), VESPER_TRACE_SCENES="1")
    with log.open("w", encoding="utf-8") as stream:
        process = subprocess.Popen([str(PLAYER), str(GAME)], stdout=stream,
                                   stderr=subprocess.STDOUT, env=env)
        try:
            window = find_window()
            time.sleep(2.5)
            first = OUTPUT / f"{choice}-sorting-room.png"
            screenshot(window, first)
            seen = {"sorting_room"}
            scene_frames = {"sorting_room": first}
            # Select a specific option using digit keys as soon as choices
            # appear. Outside a choice, the digit is inert. Return advances
            # dialogues (or finishes the current Unicode text reveal).
            # The interpreter never has to be modified or introspected.
            for step in range(600):
                current = log.read_text(encoding="utf-8", errors="replace")
                assert not re.search(r"panicked at|Story asset error|Story error:", current), (
                    choice, current[-5000:]
                )
                for scene in SCENES:
                    if scene not in seen and f"Vesper scene dialogue: {scene}" in current:
                        time.sleep(1.4)  # capture a settled cinematic frame
                        path = OUTPUT / f"{choice}-{scene}.png"
                        screenshot(window, path)
                        seen.add(scene)
                        scene_frames[scene] = path
                if f"Vesper story reached ending scene: {ending}" in current:
                    time.sleep(.8)
                    last = OUTPUT / f"{choice}-{ending}.png"
                    screenshot(window, last)
                    assert set(SCENES) == seen, (ending, seen)
                    assert changed(first, last) > 3000
                    assert changed(first, scene_frames["sky_garden"]) > 3000
                    assert changed(scene_frames["sky_garden"],
                                   scene_frames["horizon_gate"]) > 3000
                    print(f"Route {choice}: all six journey scenes, {ending}, "
                          f"and real rendered art transitions passed", flush=True)
                    break
                assert process.poll() is None, (choice, process.returncode, current[-5000:])
                press(window, str(choice), .12)
                press(window, "Return", .16)
            else:
                raise AssertionError(
                    f"Route {choice} failed to reach {ending}; "
                    f"log tail:\n{log.read_text(errors='replace')[-5000:]}"
                )
            # This game must not silently mutate its source or corrupt the
            # previous quick-save contract on clean exit.
            press(window, "Escape", .25)
            process.wait(timeout=15)
            assert process.returncode == 0, (choice, process.returncode)
        finally:
            if process.poll() is None:
                process.terminate()
                process.wait(timeout=10)


def main() -> None:
    OUTPUT.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="vesper-dawn-routes-") as directory:
        for selection in ENDINGS:
            exercise_route(selection, Path(directory))
    print("Dawn: all three original endings visually exercised", flush=True)


if __name__ == "__main__":
    main()
