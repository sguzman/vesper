"""Exercise Atlas in the unchanged Bevy player across all four endings.

GitHub CI runs this with Xvfb, xdotool, and ImageMagick. Story dialogue traces
allow choices to be selected only after their corresponding dialogue pages.
The script does not modify the game, save format, or Rust runtime.
"""
from __future__ import annotations

from collections import Counter
import json
import os
from pathlib import Path
import re
import subprocess
import tempfile
import time

ROOT = Path(__file__).resolve().parents[2]
PLAYER = ROOT / "target/debug/vesper-player"
GAME = ROOT / "examples/atlas-unwritten-skies"
OUTPUT = Path("/tmp/vesper-atlas-qa")
TITLE = "The Atlas of Unwritten Skies"
ENDINGS = {
    1: "epilogue_open",
    2: "epilogue_witness",
    3: "epilogue_covenant",
    4: "epilogue_unbound",
}
TRACE = re.compile(r"Vesper scene dialogue: ([a-z0-9_]+)")
FAILURE = re.compile(r"thread .* panicked|Story asset error|Story error:|asset failed", re.I)


def run(*argv: str, check: bool = True) -> subprocess.CompletedProcess[str]:
    return subprocess.run(argv, capture_output=True, text=True, check=check)


def scene_spec() -> dict[str, dict[str, int]]:
    """Count presented dialogue screens, not raw prose words."""
    scenes: dict[str, dict[str, int]] = {}
    current = None
    for raw in (GAME / "scripts/story.vn").read_text(encoding="utf-8").splitlines():
        line = raw.strip()
        if line.startswith("@scene "):
            current = line[7:]
            scenes[current] = {"dialogue": 0, "choices": 0}
        elif current and (line.startswith('"') or
                          re.match(r"^[a-z][a-z0-9_]*: \".*\"$", line)):
            scenes[current]["dialogue"] += 1
        elif current and line.startswith("* "):
            scenes[current]["choices"] += 1
    assert len(scenes) == 53, f"Unexpected Atlas story scene count: {len(scenes)}"
    assert scenes["final_engine"]["choices"] == 4
    return scenes


def window_id() -> str:
    until = time.monotonic() + 40
    while time.monotonic() < until:
        r = run("xdotool", "search", "--name", TITLE, check=False)
        if r.returncode == 0 and r.stdout.strip():
            return r.stdout.splitlines()[0]
        time.sleep(0.2)
    raise AssertionError("Atlas Bevy player never opened a window")


def press(window: str, key: str, delay: float = 0.09) -> None:
    run("xdotool", "windowfocus", "--sync", window)
    run("xdotool", "key", "--clearmodifiers", key)
    time.sleep(delay)


def screenshot(window: str, name: str) -> Path:
    path = OUTPUT / name
    # Software Vulkan may present a completely black first frame immediately
    # after a scene/save restoration. Wait for the actual composed frame, not
    # just the X11 window's existence. Real persistent black output still fails.
    for attempt in range(25):
        run("import", "-window", window, str(path))
        stats = run("identify", "-format", "%w %h %k", str(path)).stdout
        width, height, colors = map(int, stats.split())
        assert (width, height) == (1280, 720), (name, stats)
        if colors > 80:
            return path
        time.sleep(0.3)
    raise AssertionError(f"No rendered Atlas frame for {name}: {stats}")


def changed(first: Path, last: Path) -> int:
    result = run("compare", "-metric", "AE", str(first), str(last),
                 "null:", check=False)
    match = re.match(r"\s*(\d+)", result.stderr)
    assert match, result.stderr
    return int(match.group(1))


def read_log(path: Path) -> str:
    return path.read_text(encoding="utf-8", errors="replace")


def wait_for_log(path: Path, phrase: str, seconds: float = 10) -> None:
    deadline = time.monotonic() + seconds
    while time.monotonic() < deadline:
        if phrase in read_log(path):
            return
        time.sleep(0.1)
    raise AssertionError(f"Missing {phrase!r} in {path}:\n{read_log(path)[-4000:]}")


def test_save_restore(root: Path) -> None:
    save_root = root / "saves-check"
    save_root.mkdir()
    log = OUTPUT / "save-restore.log"
    env = dict(os.environ, VESPER_SAVE_DIR=str(save_root),
               VESPER_TRACE_SCENES="1", LIBGL_ALWAYS_SOFTWARE="1")
    with log.open("w", encoding="utf-8") as output:
        process = subprocess.Popen([str(PLAYER), str(GAME)],
                                   stdout=output, stderr=subprocess.STDOUT, env=env)
        try:
            win = window_id()
            wait_for_log(log, "Vesper scene dialogue: prologue")
            time.sleep(1.8)
            press(win, "F5")
            saved = save_root / "atlas-unwritten-skies.json"
            until = time.monotonic() + 10
            while time.monotonic() < until and not saved.is_file():
                time.sleep(0.1)
            assert saved.is_file(), f"Atlas F5 save not created at {saved}"
            snapshot = json.loads(saved.read_text(encoding="utf-8"))
            assert snapshot["game_id"] == "atlas-unwritten-skies"
            press(win, "Return")
            press(win, "F9")
            wait_for_log(log, "Vesper save restored")
            assert not FAILURE.search(read_log(log)), read_log(log)[-3000:]
            screenshot(win, "save-restore.png")
            print("Atlas: F5 save JSON and F9 restoration passed", flush=True)
            press(win, "Escape", 0.15)
            process.wait(timeout=15)
            assert process.returncode == 0
        finally:
            if process.poll() is None:
                process.terminate()
                process.wait(timeout=10)


def play_ending(option: int, spec: dict[str, dict[str, int]], root: Path) -> None:
    ending = ENDINGS[option]
    save_root = root / f"saves-{option}"
    save_root.mkdir()
    log = OUTPUT / f"ending-{option}.log"
    env = dict(os.environ, VESPER_SAVE_DIR=str(save_root),
               VESPER_TRACE_SCENES="1", LIBGL_ALWAYS_SOFTWARE="1")
    with log.open("w", encoding="utf-8") as output:
        process = subprocess.Popen([str(PLAYER), str(GAME)],
                                   stdout=output, stderr=subprocess.STDOUT, env=env)
        try:
            win = window_id()
            time.sleep(2.5)
            opening = screenshot(win, f"{option}-opening.png")
            illustrated = set()
            for step in range(5000):
                transcript = read_log(log)
                assert not FAILURE.search(transcript), (
                    option, step, transcript[-5000:])
                if f"Vesper story reached ending scene: {ending}" in transcript:
                    final = screenshot(win, f"{option}-{ending}.png")
                    assert changed(opening, final) > 3000, (
                        option, "Opening and ending scene did not visibly differ")
                    traces = TRACE.findall(transcript)
                    for required in ("prologue", "courier", "atlas_wakes",
                                     "second_chapter", "third_chapter",
                                     "fourth_chapter", "threshold_market",
                                     "assembly", "return_to_lens", "final_engine",
                                     ending):
                        assert required in traces, (option, required)
                    assert len(set(traces)) == 33, (
                        option, "Unexpected number of scenes in ending route",
                        sorted(set(traces)))
                    expected_routes = {
                        1: ("letter_open", "east_route", "trust_courier",
                            "record_contract", "negotiate_route",
                            "families_heard", "face_orsol"),
                        2: ("courier_questions", "memory_route",
                            "suspect_courier", "record_names",
                            "consent_route", "chamber_heard", "fetch_delegates"),
                        3: ("protective_choice", "people_route",
                            "examine_scar", "record_witness",
                            "expose_route", "storm_heard", "write_clause"),
                        4: ("letter_open", "east_route", "trust_courier",
                            "record_contract", "negotiate_route",
                            "families_heard", "face_orsol"),
                    }
                    for branch in expected_routes[option]:
                        assert branch in traces, (option, branch)
                    assert "Vesper decoded audio asset: music:archive" in transcript, (
                        option, "Opening music was not decoded")
                    assert "Vesper decoded audio asset: sound:bell" in transcript, (
                        option, "Opening bell sound was not decoded")
                    print(f"Atlas: option {option} reached {ending}, "
                          f"{len(set(traces))} scenes; rendered ending and "
                          f"audio decoding passed", flush=True)
                    break
                assert process.poll() is None, (
                    option, process.returncode, transcript[-5000:])
                traces = TRACE.findall(transcript)
                if not traces:
                    time.sleep(0.1)
                    continue
                current = traces[-1]
                assert current in spec, (option, current)
                if current in ("atlas_wakes", "threshold_market", "final_engine",
                               "east_route", "memory_route", "people_route",
                               "record_contract", "record_names", "record_witness",
                               "face_orsol", "fetch_delegates", "write_clause",
                               "ending_open", "ending_witness",
                               "ending_covenant", "ending_unbound") and current not in illustrated:
                    # Capture different branch illustrations alongside shared
                    # narrative stages in the unchanged Bevy renderer.
                    time.sleep(0.4)
                    screenshot(win, f"{option}-{current}.png")
                    illustrated.add(current)
                dialogue_seen = Counter(traces)[current]
                scene = spec[current]
                # Each Enter either finishes text reveal or advances a dialogue.
                # Once the scene's final dialogue is visible, pair Enter with a
                # numeric choice. The number is inert during text reveal and
                # selects the intended option only when the choice UI appears.
                press(win, "Return", 0.09)
                if scene["choices"] and dialogue_seen >= scene["dialogue"]:
                    route_option = option if option <= scene["choices"] else 1
                    press(win, str(route_option), 0.09)
            else:
                raise AssertionError(f"Atlas route {option} never reached {ending}; "
                                     f"last log:\n{read_log(log)[-5000:]}")
            press(win, "Escape", 0.2)
            process.wait(timeout=15)
            assert process.returncode == 0, (option, process.returncode)
        finally:
            if process.poll() is None:
                process.terminate()
                process.wait(timeout=10)


def main() -> None:
    OUTPUT.mkdir(parents=True, exist_ok=True)
    spec = scene_spec()
    with tempfile.TemporaryDirectory(prefix="vesper-atlas-routes-") as tmp:
        root = Path(tmp)
        test_save_restore(root)
        for option in ENDINGS:
            play_ending(option, spec, root)
    print("Atlas: all four endings exercised in native Bevy", flush=True)


if __name__ == "__main__":
    main()
