"""Exercise Atlas in the unchanged Bevy player across all four endings.

GitHub CI runs this with Xvfb, xdotool, and ImageMagick. Story dialogue traces
allow choices to be selected only after their corresponding dialogue pages.
The script does not modify the game, save format, or Rust runtime.
"""
from __future__ import annotations

from collections import Counter
from collections.abc import Callable
import json
import os
from pathlib import Path
import re
import subprocess
import sys
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
EXPECTED_ROUTES = {
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

TRACE = re.compile(r"Vesper scene dialogue: ([a-z0-9_]+)")
FAILURE = re.compile(r"thread .* panicked|Story asset error|Story error:|asset failed", re.I)
PANEL_WIDTH = re.compile(r"Vesper panel resolved width: ([0-9]+(?:\.[0-9]+)?)")


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
            prose = line.split(": ", 1)[1] if not line.startswith('"') else line
            assert len(json.loads(prose)) <= 120, (current, "oversized dialogue page")
            scenes[current]["dialogue"] += 1
        elif current and line.startswith("* "):
            scenes[current]["choices"] += 1
    assert len(scenes) == 61, f"Unexpected Atlas story scene count: {len(scenes)}"
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


def screenshot(window: str, name: str,
               expected_size: tuple[int, int] = (1280, 720)) -> Path:
    path = OUTPUT / name
    temporary = path.with_name(f".{path.stem}.capturing.png")
    # Software rendering can present a black first frame during transitions.
    # ImageMagick can also leave a partial PNG if capture fails. Never let
    # such an incomplete screenshot masquerade as a verified QA artifact.
    # Validate the actual decoded image before atomically publishing it.
    last_error = "capture did not start"
    try:
        for _ in range(25):
            temporary.unlink(missing_ok=True)
            captured = run("import", "-window", window, str(temporary), check=False)
            if captured.returncode != 0:
                last_error = f"import exited {captured.returncode}: {captured.stderr[-500:]}"
            else:
                identified = run(
                    "identify", "-format", "%w %h %k", str(temporary), check=False
                )
                if identified.returncode != 0:
                    last_error = (
                        f"corrupt/incomplete PNG: {identified.stderr[-500:]}"
                    )
                else:
                    try:
                        width, height, colors = map(int, identified.stdout.split())
                    except ValueError:
                        last_error = f"unexpected identify output: {identified.stdout!r}"
                    else:
                        if (width, height) != expected_size:
                            last_error = (
                                f"wrong screenshot size {(width, height)}, "
                                f"expected {expected_size}"
                            )
                        elif colors <= 80:
                            last_error = f"render not ready ({colors} colors)"
                        else:
                            temporary.replace(path)
                            return path
            time.sleep(0.3)
    finally:
        temporary.unlink(missing_ok=True)
    raise AssertionError(
        f"No valid rendered Atlas frame for {name}: {last_error}"
    )


def changed(first: Path, last: Path) -> int:
    result = run("compare", "-metric", "AE", str(first), str(last),
                 "null:", check=False)
    match = re.match(r"\s*(\d+)", result.stderr)
    assert match, result.stderr
    return int(match.group(1))


def screenshot_after_transition(window: str, previous: Path,
                                name: str, min_difference: int = 2500) -> Path:
    """Wait for the GPU-rendered new menu, not just the ECS/log transition.

    Bevy can log a cached overlay reopening before the display server presents
    its new frame. Compare real window pixels with the prior menu, retrying
    with a bounded deadline rather than assuming a fixed sleep is sufficient.
    """
    deadline = time.monotonic() + 12
    last_difference = 0
    while time.monotonic() < deadline:
        path = screenshot(window, name)
        last_difference = changed(previous, path)
        if last_difference > min_difference:
            return path
        time.sleep(.25)
    raise AssertionError(
        f"Menu image did not change after transition to {name}: "
        f"{last_difference} pixels (required > {min_difference})"
    )


def read_log(path: Path) -> str:
    return path.read_text(encoding="utf-8", errors="replace")


def measured_panel_width(log: Path) -> float:
    matches = PANEL_WIDTH.findall(read_log(log))
    assert matches, "Native renderer never reported its computed panel width"
    return float(matches[-1])


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
    # End-to-end story traversal exercises each page and choice, while the
    # separate responsive smoke and text-reveal unit tests cover animation.
    # Instant reader preference keeps expanded pages from doubling CI time.
    prefs = root / f"prefs-{option}"
    prefs.mkdir()
    (prefs / "player-preferences.json").write_text(
        json.dumps({"format_version": 1, "reveal_speed": "instant"})
    )
    env = dict(os.environ, VESPER_SAVE_DIR=str(save_root),
               VESPER_PREFERENCES_DIR=str(prefs),
               VESPER_TRACE_SCENES="1", LIBGL_ALWAYS_SOFTWARE="1")
    with log.open("w", encoding="utf-8") as output:
        process = subprocess.Popen([str(PLAYER), str(GAME)],
                                   stdout=output, stderr=subprocess.STDOUT, env=env)
        try:
            win = window_id()
            time.sleep(2.5)
            opening = screenshot(win, f"{option}-opening.png")
            # Capture consecutive fully visible pages in instant-reader mode.
            # Separate responsive QA validates the animated reveal path.
            press(win, "Return", 0.35)
            readable = screenshot(win, f"{option}-opening-readable.png")
            assert changed(opening, readable) > 100, (
                option, "Text reveal/advance did not change the opening frame")
            if option == 1:
                # Preserve the same fully revealed opening prose at compact and
                # spacious window sizes for human review of the ornate skin.
                # The engine's general resize tests use other external games;
                # these screenshots specifically cover Atlas artwork and text.
                for width, height, label in (
                    (946, 511, "compact"), (1900, 1035, "large")
                ):
                    run("xdotool", "windowsize", "--sync", win,
                        str(width), str(height))
                    time.sleep(0.6)
                    screenshot(win, f"opening-readable-{label}.png",
                               (width, height))
                run("xdotool", "windowsize", "--sync", win, "1280", "720")
                time.sleep(0.6)
                screenshot(win, "opening-readable-restored.png")
            illustrated = set()
            # Count UI mount events already serviced. A late Return on a
            # mounted choice screen selects the default first option; digit
            # input must be delivered without that intervening Return.
            handled_choice_mounts = 0
            for step in range(12000):
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
                                     "the_garden_where_bells_were_planted",
                                     "an_inventory_of_small_mercies",
                                     "the_quay_of_unreturned_things",
                                     "the_hall_of_open_questions",
                                     "the_station_for_things_not_yet_lost",
                                     "assembly", "return_to_lens", "final_engine",
                                     ending):
                        assert required in traces, (option, required)
                    assert len(set(traces)) == 41, (
                        option, "Unexpected number of scenes in ending route",
                        sorted(set(traces)))
                    for branch in EXPECTED_ROUTES[option]:
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
                               "ending_covenant", "ending_unbound",
                               "the_garden_where_bells_were_planted",
                               "an_inventory_of_small_mercies",
                               "the_quay_of_unreturned_things",
                               "the_hall_of_open_questions",
                               "the_station_for_things_not_yet_lost") and current not in illustrated:
                    # Capture different branch illustrations alongside shared
                    # narrative stages in the unchanged Bevy renderer.
                    time.sleep(0.4)
                    screenshot(win, f"{option}-{current}.png")
                    illustrated.add(current)
                dialogue_seen = Counter(traces)[current]
                scene = spec[current]
                # Never send Return to a mounted choice screen: it picks
                # the default row before Digit2/3/4 can choose the route.
                # Wait for the native mount event, then use only the intended
                # numeric choice until a new scene is actually observed.
                if scene["choices"] and dialogue_seen >= scene["dialogue"]:
                    marker = "Vesper choice UI mounted"
                    mounted = transcript.count(marker)
                    if mounted <= handled_choice_mounts:
                        # A preceding Return may still be in the Bevy event
                        # queue when the final dialogue appears. Give that
                        # event an opportunity to mount its choice UI before
                        # issuing any extra Return that might pick row one.
                        settle_until = time.monotonic() + .65
                        while time.monotonic() < settle_until:
                            mounted = read_log(log).count(marker)
                            if mounted > handled_choice_mounts:
                                break
                            time.sleep(.04)
                    if mounted <= handled_choice_mounts:
                        press(win, "Return", 0.09)
                        deadline = time.monotonic() + 8
                        while time.monotonic() < deadline:
                            mounted = read_log(log).count(marker)
                            if mounted > handled_choice_mounts:
                                break
                            time.sleep(.05)
                        else:
                            raise AssertionError(
                                f"Choice UI never mounted in {current} (route {option})"
                            )
                    handled_choice_mounts = mounted
                    route_option = option if option <= scene["choices"] else 1
                    # Verify the actual authored branch *at every decision*,
                    # not only once the ending is reached many minutes later.
                    # A skipped digit can otherwise select the default row.
                    choice_index = handled_choice_mounts - 1
                    if choice_index < len(EXPECTED_ROUTES[option]):
                        expected_branch = EXPECTED_ROUTES[option][choice_index]
                    else:
                        assert choice_index == len(EXPECTED_ROUTES[option]), (
                            option, "Unexpected extra Atlas choice", choice_index,
                        )
                        expected_branch = {
                            1: "ending_open", 2: "ending_witness",
                            3: "ending_covenant", 4: "ending_unbound",
                        }[option]
                    deadline = time.monotonic() + 8
                    while time.monotonic() < deadline:
                        press(win, str(route_option), 0.12)
                        latest = TRACE.findall(read_log(log))
                        if latest and latest[-1] != current:
                            assert latest[-1] == expected_branch, (
                                f"Atlas route {option} wrong choice at {current}: "
                                f"expected {expected_branch}, got {latest[-1]}; "
                                "the runner must not silently accept default choice",
                            )
                            break
                    else:
                        raise AssertionError(
                            f"Choice {route_option} in {current} never changed scene: "
                            + read_log(log)[-2500:]
                        )
                    continue
                # Exactly one Return per observed story/UI transition. Fast
                # unacknowledged key bursts can leave a Return queued when
                # the next choice row mounts and silently choose its default.
                previous_lines = len(traces)
                previous_mounts = transcript.count("Vesper choice UI mounted")
                press(win, "Return", 0.09)
                settle_until = time.monotonic() + 1.2
                while time.monotonic() < settle_until:
                    observed = read_log(log)
                    if (len(TRACE.findall(observed)) > previous_lines
                            or observed.count("Vesper choice UI mounted") > previous_mounts
                            or f"Vesper story reached ending scene: {ending}" in observed):
                        break
                    time.sleep(.04)
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


def width_preference(prefs: Path) -> float | None:
    filename = prefs / "player-preferences.json"
    if not filename.is_file():
        return None
    return json.loads(filename.read_text()).get("panel_width_percent")


def select_width_setting(win: str, log: Path) -> None:
    """Walk only after each selection event has actually reached the renderer.

    llvmpipe can drop rapid xdotool keypresses under a heavy screenshot load.
    """
    # A resize and a reused pause menu can drop the immediately following
    # '4' shortcut under llvmpipe. Checking an *old* "opened" log marker
    # silently sends Right to a closed overlay and fails 30 seconds later.
    # Count opens versus closes to establish the CURRENT overlay state.
    deadline = time.monotonic() + 35
    while time.monotonic() < deadline:
        transcript = read_log(log)
        opens = transcript.count("Vesper appearance settings opened")
        closes = transcript.count("Vesper appearance settings closed")
        if opens > closes:
            break
        press(win, "4", 0.22)
    else:
        raise AssertionError(
            "Settings did not reopen after pause/resize: " + read_log(log)[-1600:]
        )
    while time.monotonic() < deadline:
        section = read_log(log).rsplit("Vesper appearance settings opened", 1)[-1]
        positions = re.findall(r"Vesper appearance selection: (\d+)", section)
        selection = int(positions[-1]) if positions else 0
        if selection == 8:
            return
        press(win, "Down" if selection < 8 else "Up", 0.19)
    raise AssertionError("Settings never focused Dialogue width: " + read_log(log)[-1800:])


def adjust_width(win: str, prefs: Path, direction: str, accept: Callable[[float], bool]) -> float:
    """Verify the serialized setting itself; image pixel changes are insufficient."""
    deadline = time.monotonic() + 30
    while time.monotonic() < deadline:
        current = width_preference(prefs)
        if current is not None and accept(current):
            return current
        press(win, direction, 0.18)
        time.sleep(0.13)
    raise AssertionError("Reader width setting did not persist: " + str(width_preference(prefs)))


def assert_settings_frame_fits(path: Path, width: int, height: int) -> None:
    """Both solid gold vertical frame sides must be visible in the image.

    Earlier smoke only checked that screenshots existed; a 1050px Settings
    frame could be truncated at 946px yet still pass. The Atlas skin's exact
    border RGB is (232, 178, 94), and Bevy renders the three-pixel side as a
    continuous solid column. Require real pixels on both sides of the viewport.
    """
    result = subprocess.run(
        ["convert", str(path), "-depth", "8", "rgb:-"],
        capture_output=True, check=True,
    )
    pixels = result.stdout
    assert len(pixels) == width * height * 3, "Unexpected screenshot pixel data"
    top, bottom = 65, min(400, height - 15)
    threshold = int((bottom - top) * 0.86)

    def framed_column(x: int) -> bool:
        total = 0
        for y in range(top, bottom):
            index = (y * width + x) * 3
            rgb = pixels[index:index + 3]
            if all(abs(actual - expected) <= 3 for actual, expected in
                   zip(rgb, (232, 178, 94))):
                total += 1
        return total >= threshold

    assert any(framed_column(x) for x in range(20, width // 2)), (
        "Settings left frame missing", path,
    )
    assert any(framed_column(x) for x in range(width // 2, width - 20)), (
        "Settings right frame clipped outside the compact window", path,
    )


def assert_settings_preview_reflows(path: Path, width: int, height: int,
                                     compact: bool) -> None:
    """Pixel-level contract: the actual parchment sample moves columns.

    Merely capturing a screenshot can miss a stuck desktop-sized grid. The
    Atlas live preview's opaque parchment swatch has an exact authored RGB,
    independent of scene artwork, text rasterization or keyboard focus.
    Its median position distinguishes a stacked compact viewport from the
    native desktop two-column composition.
    """
    result = subprocess.run(
        ["convert", str(path), "-depth", "8", "rgb:-"],
        capture_output=True, check=True,
    )
    pixels = result.stdout
    assert len(pixels) == width * height * 3, (path, "unexpected dimensions")
    paper = bytes((223, 208, 175))
    positions = [index // 3 for index in range(0, len(pixels), 3)
                 if pixels[index:index + 3] == paper]
    assert len(positions) > 10000, (
        "Atlas Settings preview paper did not render", path, len(positions),
    )
    xs = sorted(position % width for position in positions)
    ys = sorted(position // width for position in positions)
    center_x = xs[len(xs) // 2] / width
    center_y = ys[len(ys) // 2] / height
    if compact:
        assert 0.42 < center_x < 0.60 and center_y > 0.69, (
            "Compact Settings preview did not stack beneath controls",
            path, center_x, center_y,
        )
    else:
        assert center_x > 0.60 and center_y < 0.70, (
            "Desktop Settings preview did not occupy right-hand column",
            path, center_x, center_y,
        )


def assert_native_opacity_slider_drag(window: str, prefs_dir: Path) -> None:
    """Exercise actual Bevy slider pointer events, not only keyboard arrows.

    At the 1280x720 illustrated Settings view, the opacity rail occupies
    x=397..567, y=326. First click near its left end, then drag its native
    thumb toward the right and require two separately persisted model values.
    The temporary preferences directory is isolated to this QA process.
    """
    path = prefs_dir / "player-preferences.json"

    def value_after(predicate) -> float:
        end = time.monotonic() + 10
        while time.monotonic() < end:
            try:
                value = json.loads(path.read_text()).get("panel_opacity")
                if isinstance(value, (float, int)) and predicate(float(value)):
                    return float(value)
            except (OSError, json.JSONDecodeError):
                pass
            time.sleep(0.12)
        current = path.read_text(errors="replace") if path.exists() else "(no preference file)"
        raise AssertionError(
            "Native Settings opacity slider did not persist pointer edit: " + current
        )

    run("xdotool", "windowfocus", "--sync", window)
    run("xdotool", "mousemove", "--window", window, "420", "326")
    run("xdotool", "click", "1")
    left = value_after(lambda x: 0.60 <= x < 0.78)
    run("xdotool", "mousemove", "--window", window, "420", "326")
    run("xdotool", "mousedown", "1")
    time.sleep(0.16)
    # Cross the drag threshold in a distinct rendered frame, then generate
    # a second move; the first motion starts a Bevy drag, while subsequent
    # motion actually produces the ValueChange used by the preference model.
    run("xdotool", "mousemove", "--window", window, "460", "326")
    time.sleep(0.24)
    run("xdotool", "mousemove", "--window", window, "533", "326")
    time.sleep(0.30)
    run("xdotool", "mouseup", "1")
    right = value_after(lambda x: x > left + 0.12)
    assert right <= 1.0, (left, right)
    print(f"Atlas native opacity slider click/drag: {left:.2f} -> {right:.2f}", flush=True)


def screenshot_settled_settings(window: str, path: Path,
                                width: int, height: int,
                                compact: bool) -> None:
    """Wait for a *new* native frame after resizing, not a cropped stale frame.

    Xvfb/llvmpipe can report the new window size before Bevy's next swapchain
    presentation. Check actual in-frame parchment placement and, at compact
    sizes, both physical gold borders. Retries are bounded: a genuinely
    broken layout still fails rather than silently passing a screenshot.
    """
    deadline = time.monotonic() + 12
    last_error = None
    while time.monotonic() < deadline:
        screenshot(window, path, (width, height))
        try:
            if compact:
                assert_settings_frame_fits(path, width, height)
            assert_settings_preview_reflows(path, width, height, compact)
            return
        except AssertionError as error:
            last_error = error
            time.sleep(.35)
    raise AssertionError(("Settings never produced a settled responsive frame",
                          path, last_error))


def assert_pause_click_activates_on_mouse_down(window: str, log: Path) -> None:
    """Resume must execute before mouse-up, not after Activate on release.

    Keep the primary button held while observing the game log. An ordinary
    release-driven button would fail, even if its visual transition is quick.
    Coordinates are the center of the real Pause 'Resume story' plaque at
    1280x720, after the viewport smoke restores its original dimensions.
    """
    run("xdotool", "windowfocus", "--sync", window)
    before = read_log(log).count("Vesper story resumed")
    run("xdotool", "mousemove", "--window", window, "640", "285")
    begin = time.monotonic()
    run("xdotool", "mousedown", "1")
    try:
        deadline = time.monotonic() + 0.8
        while time.monotonic() < deadline:
            if read_log(log).count("Vesper story resumed") > before:
                elapsed = (time.monotonic() - begin) * 1000
                print(f"Native Pause button resumed on mouse-down in {elapsed:.1f} ms", flush=True)
                return
            time.sleep(.02)
        raise AssertionError(
            "Resume did not activate while primary mouse button was held "
            "(waiting for release is not acceptable for menu responsiveness)"
        )
    finally:
        run("xdotool", "mouseup", "1")


def responsive_layout() -> None:
    """Quick visual regression before the much longer four-ending playthrough."""
    OUTPUT.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="vesper-atlas-panel-") as tmp:
        root = Path(tmp)
        prefs = root / "prefs"
        prefs.mkdir()
        log = OUTPUT / "panel-responsive.log"
        env = dict(os.environ, VESPER_PREFERENCES_DIR=str(prefs),
                   VESPER_SAVE_DIR=str(root / "saves"),
                   LIBGL_ALWAYS_SOFTWARE="1",
                   VESPER_TRACE_SCENES="1")
        with log.open("w", encoding="utf-8") as output:
            process = subprocess.Popen([str(PLAYER), str(GAME)],
                                       stdout=output, stderr=subprocess.STDOUT, env=env)
            try:
                win = window_id()
                time.sleep(2.5)
                # Atlas exercises three independent, optional menu illustrations.
                # Existing games without menu_backgrounds retain plain overlays.
                screenshot(win, "panel-before-illustrated-menus.png")
                wait_for_log(log, "Vesper hint diagnostic:")
                diagnosis = re.search(
                    r"Vesper hint diagnostic: anchors=(\d+) anchor_height=([-.\d]+) text_nodes=(\d+) chars=(\d+) glyph_height=([-.\d]+) box_height=([-.\d]+)",
                    read_log(log),
                )
                assert diagnosis, "Missing parsed hint diagnostic"
                assert int(diagnosis[1]) == 1 and int(diagnosis[3]) == 1 and int(diagnosis[4]) > 0, diagnosis.groups()
                assert float(diagnosis[5]) > 0 and float(diagnosis[6]) > 0, (
                    "Hint exists but has no rendered geometry", diagnosis.groups()
                )
                def capture_menu_viewports(kind: str) -> None:
                    # Protect diegetic button-label readability at narrow and
                    # spacious sizes, not only the default 1280x720 window.
                    for width, height, suffix in (
                        (946, 511, "compact"),
                        (1900, 1035, "large"),
                    ):
                        run("xdotool", "windowsize", "--sync", win,
                            str(width), str(height))
                        time.sleep(0.55)
                        path = f"menu-{kind}-{suffix}.png"
                        if kind == "settings":
                            screenshot_settled_settings(
                                win, OUTPUT / path, width, height,
                                suffix == "compact",
                            )
                        else:
                            screenshot(win, path, (width, height))
                    run("xdotool", "windowsize", "--sync", win,
                        "1280", "720")
                    time.sleep(0.55)

                press(win, "p", 0.35)
                time.sleep(0.65)
                screenshot(win, "menu-pause-illustrated.png")
                capture_menu_viewports("pause")
                assert_pause_click_activates_on_mouse_down(win, log)
                # The pointer test resumes the story. Reopen Pause for the
                # existing Settings, responsive and preference tests.
                press(win, "p", 0.35)
                settings_before = read_log(log).count("Vesper appearance settings opened")
                press(win, "4", 0.35)
                # Backdrops are prewarmed at startup. Their logged load is
                # NOT evidence the Settings UI actually opened. Guard the
                # mouse-release ghost action after clicking Resume above.
                deadline = time.monotonic() + 10
                while read_log(log).count("Vesper appearance settings opened") <= settings_before:
                    assert time.monotonic() < deadline, (
                        "Pause digit 4 was hijacked by a stale pointer release"
                    )
                    time.sleep(.05)
                time.sleep(0.65)
                screenshot_settled_settings(win,
                    OUTPUT / "menu-settings-illustrated.png", 1280, 720, False)
                capture_menu_viewports("settings")
                assert_settings_frame_fits(
                    OUTPUT / "menu-settings-compact.png", 946, 511,
                )
                for name, width, height, compact in (
                    ("menu-settings-compact.png", 946, 511, True),
                    ("menu-settings-illustrated.png", 1280, 720, False),
                    ("menu-settings-large.png", 1900, 1035, False),
                ):
                    assert_settings_preview_reflows(
                        OUTPUT / name, width, height, compact,
                    )
                assert_native_opacity_slider_drag(win, prefs)
                press(win, "Escape", 0.30)
                press(win, "p", 0.30)
                press(win, "m", 0.35)
                time.sleep(0.65)
                screenshot(win, "menu-main-illustrated.png")
                capture_menu_viewports("main")
                # Regression for Main -> Settings -> Main. Both UI trees
                # already exist and must be reused without exposing the
                # underlying story or creating a new set of image controls.
                settings_before = read_log(log).count("Vesper appearance settings opened")
                main_before = read_log(log).count("Vesper main menu opened")
                main_reuse_before = read_log(log).count("Vesper Main menu UI reused")
                press(win, "4", 0.35)
                deadline = time.monotonic() + 10
                while read_log(log).count("Vesper appearance settings opened") <= settings_before:
                    assert time.monotonic() < deadline, "Main -> Settings transition stalled"
                    time.sleep(.05)
                screenshot_after_transition(
                    win, OUTPUT / "menu-main-illustrated.png",
                    "menu-settings-from-main.png",
                )
                press(win, "Escape", 0.35)
                deadline = time.monotonic() + 10
                while read_log(log).count("Vesper main menu opened") <= main_before:
                    assert time.monotonic() < deadline, "Settings -> Main transition stalled"
                    time.sleep(.05)
                assert read_log(log).count("Vesper Main menu UI reused") > main_reuse_before
                screenshot_after_transition(
                    win, OUTPUT / "menu-settings-from-main.png",
                    "menu-main-returned.png",
                )
                assert "Vesper Pause menu backdrop loaded: assets/backgrounds/observatory_evening.png" in read_log(log)
                assert "Vesper Main menu backdrop loaded: assets/backgrounds/observatory_evening.png" in read_log(log)
                assert "Vesper settings menu backdrop loaded: assets/backgrounds/observatory_evening.png" in read_log(log)
                # All three illustrated menus are prebuilt and reused: no
                # expensive UI subtree construction on transition frames.
                assert "Vesper Settings menu UI reused" in read_log(log)
                assert "Vesper Pause menu UI reused" in read_log(log)
                assert "Vesper Main menu UI reused" in read_log(log)
                # The legacy translucent abstract menu art was removed.
                # Only the painterly observatory backdrop and native controls
                # should remain for all three menus.
                for kind in ("Pause", "Main", "settings"):
                    assert f"Vesper {kind} menu decoration loaded:" not in read_log(log), (
                        kind, "obsolete low-opacity menu overlay is still loading"
                    )
                for previous_name, current_name in [
                    ("panel-before-illustrated-menus.png", "menu-pause-illustrated.png"),
                    ("menu-pause-illustrated.png", "menu-settings-illustrated.png"),
                    ("menu-settings-illustrated.png", "menu-main-illustrated.png"),
                ]:
                    assert changed(OUTPUT / previous_name, OUTPUT / current_name) > 2500, (
                        "Custom menu state did not render distinct pixels",
                        previous_name, current_name,
                    )
                press(win, "m", 0.35)
                # Capture fully revealed prose rather than timing-dependent
                # partial glyph output while the first line is animating.
                press(win, "Return", 0.35)
                # A one-step debug rewind must visually restore the previous
                # page without a disk save and without restarting the novel.
                press(win, "Return", 0.35)
                screenshot(win, "panel-debug-next.png")
                staged_before = read_log(log).count("Vesper character staged:")
                music_before = read_log(log).count("Vesper queued music:")
                press(win, "BackSpace", 0.40)
                wait_for_log(log, "Vesper debug rewind restored")
                assert read_log(log).count("Vesper character staged:") == staged_before, (
                    "Debug rewind replayed a character entrance instead of restoring instantly"
                )
                assert read_log(log).count("Vesper queued music:") == music_before, (
                    "Debug rewind restarted the same music track"
                )
                screenshot(win, "panel-debug-rewound.png")
                assert changed(
                    OUTPUT / "panel-debug-next.png",
                    OUTPUT / "panel-debug-rewound.png"
                ) > 250, "Backspace rewind did not change the visible dialogue"
                for width, height, name in [
                    (1280, 720, "standard"),
                    (946, 511, "compact"),
                    (810, 450, "windowed"),
                    (1900, 1035, "ultrawide"),
                ]:
                    run("xdotool", "windowsize", "--sync", win, str(width), str(height))
                    time.sleep(0.6)
                    screenshot(win, f"panel-authored-{name}.png", (width, height))
                run("xdotool", "windowsize", "--sync", win, "1280", "720")
                time.sleep(0.6)

                # P -> Settings (pause menu option 4); track selection logs
                # rather than assuming every event was processed by llvmpipe.
                press(win, "p", 0.24)
                press(win, "4", 0.24)
                select_width_setting(win, log)
                narrow = adjust_width(win, prefs, "Left", lambda width: width <= 60.0)
                press(win, "Escape", 0.24)
                press(win, "p", 0.40)
                screenshot(win, "panel-reader-narrow.png")

                press(win, "p", 0.24)
                press(win, "4", 0.24)
                select_width_setting(win, log)
                wide = adjust_width(win, prefs, "Right", lambda width: width >= 85.0)
                press(win, "Escape", 0.24)
                press(win, "p", 0.40)
                screenshot(win, "panel-reader-wide.png")
                assert wide - narrow >= 25.0, (narrow, wide)
                assert changed(
                    OUTPUT / "panel-reader-narrow.png",
                    OUTPUT / "panel-reader-wide.png"
                ) > 1000, "Reader width control did not change the rendered panel"

                # Confirm width changes in an ordinary compact WINDOW,
                # not only at desktop/fullscreen resolution. Old hard clamps
                # made several slider values indistinguishable here.
                run("xdotool", "windowsize", "--sync", win, "946", "511")
                time.sleep(0.4)
                screenshot(win, "panel-windowed-wide.png", (946, 511))
                windowed_wide_width = measured_panel_width(log)
                press(win, "p", 0.24)
                press(win, "4", 0.24)
                select_width_setting(win, log)
                adjusted = adjust_width(win, prefs, "Left", lambda width: width <= 60.0)
                press(win, "Escape", 0.24)
                press(win, "p", 0.40)
                screenshot(win, "panel-windowed-narrow.png", (946, 511))
                windowed_narrow_width = measured_panel_width(log)
                assert windowed_wide_width - windowed_narrow_width > 20.0, (
                    "946px window panel did not shrink", windowed_wide_width, windowed_narrow_width
                )
                assert adjusted <= 60.0
                assert changed(
                    OUTPUT / "panel-windowed-wide.png",
                    OUTPUT / "panel-windowed-narrow.png"
                ) > 500, "Width control remained clamped in windowed mode"
                # A window narrower than the authored minimum must still
                # have distinct slider endpoints, not a single clamped width.
                run("xdotool", "windowsize", "--sync", win, "810", "450")
                time.sleep(0.4)
                press(win, "p", 0.24)
                press(win, "4", 0.24)
                select_width_setting(win, log)
                adjust_width(win, prefs, "Right", lambda width: width >= 85.0)
                press(win, "Escape", 0.24)
                press(win, "p", 0.40)
                screenshot(win, "panel-small-wide.png", (810, 450))
                small_wide_width = measured_panel_width(log)
                press(win, "p", 0.24)
                press(win, "4", 0.24)
                select_width_setting(win, log)
                adjust_width(win, prefs, "Left", lambda width: width <= 60.0)
                press(win, "Escape", 0.24)
                press(win, "p", 0.40)
                screenshot(win, "panel-small-narrow.png", (810, 450))
                small_narrow_width = measured_panel_width(log)
                assert small_wide_width - small_narrow_width > 20.0, (
                    "810px window panel did not shrink", small_wide_width, small_narrow_width,
                    [line for line in read_log(log).splitlines()
                     if "Vesper panel target width:" in line or
                        "Vesper panel resolved width:" in line][-24:]
                )
                assert changed(
                    OUTPUT / "panel-small-wide.png",
                    OUTPUT / "panel-small-narrow.png"
                ) > 500, "Width setting is still clamped below authored min-width"
                # Restore normal resolution and prior wide preference.
                run("xdotool", "windowsize", "--sync", win, "1280", "720")
                time.sleep(0.4)
                press(win, "p", 0.24)
                press(win, "4", 0.24)
                select_width_setting(win, log)
                adjust_width(win, prefs, "Right", lambda width: width >= 85.0)
                press(win, "Escape", 0.24)
                press(win, "p", 0.40)

                # Live reader control smoke: the bottom offset and ink
                # palette must persist independently of authored game data.
                press(win, "p", 0.25)
                press(win, "4", 0.25)
                select_width_setting(win, log)  # row 8, width
                press(win, "Down", 0.22)       # row 9, bottom position
                press(win, "Right", 0.35)
                press(win, "Down", 0.22)       # row 10, ink preset
                for _ in range(3):
                    press(win, "Right", 0.35)  # authored -> black -> brown -> navy
                wait_for_log(log, "Vesper settings applied: Text ink")
                deadline = time.monotonic() + 15
                while time.monotonic() < deadline:
                    try:
                        actual = json.loads((prefs / "player-preferences.json").read_text())
                        if actual.get("text_ink") == "navy" and actual.get("panel_bottom_percent", 0) >= 1.0:
                            break
                    except (OSError, ValueError):
                        pass
                    time.sleep(0.16)
                else:
                    raise AssertionError("Ink and bottom settings did not persist")
                press(win, "Escape", 0.25)
                press(win, "p", 0.35)
                screenshot(win, "panel-reader-bottom-ink.png")
                assert changed(
                    OUTPUT / "panel-reader-wide.png",
                    OUTPUT / "panel-reader-bottom-ink.png"
                ) > 200, "Live bottom/ink settings produced no visible change"

                # Compare ink alone on the *same* scene, unchanged width and
                # position, so a colour setting cannot silently do nothing.
                press(win, "p", 0.25)
                press(win, "4", 0.25)
                select_width_setting(win, log)
                press(win, "Down", 0.22)
                press(win, "Down", 0.22)  # ink row 10
                press(win, "Right", 0.35)  # navy -> sepia
                deadline = time.monotonic() + 15
                while time.monotonic() < deadline:
                    if json.loads((prefs / "player-preferences.json").read_text()).get("text_ink") == "sepia":
                        break
                    time.sleep(0.15)
                else:
                    raise AssertionError("Sepia ink preset did not persist")
                press(win, "Escape", 0.25)
                press(win, "p", 0.35)
                screenshot(win, "panel-reader-sepia-ink.png")
                assert changed(
                    OUTPUT / "panel-reader-bottom-ink.png",
                    OUTPUT / "panel-reader-sepia-ink.png"
                ) > 100, "Ink palette change did not affect the rendered story"

                # Include a multi-line revealed page, not only the title frame.
                for _ in range(4):
                    press(win, "Return", 0.25)
                screenshot(win, "panel-later-dialogue.png")
                # Advance to a naturally longer page and reveal all its words.
                # This catches text/footer overlap that title or short-line
                # screenshots cannot expose.
                press(win, "Return", 0.30)
                press(win, "Return", 0.45)
                screenshot(win, "panel-long-dialogue.png")

                # Confirm the gold speaker label on the actual painted titleplate.
                # First named Mira passage follows the anonymous prologue.
                for _ in range(140):
                    if len(TRACE.findall(read_log(log))) >= 38:
                        break
                    press(win, "Return", 0.16)
                else:
                    raise AssertionError("Did not reach named Mira dialogue")
                screenshot(win, "panel-mira-titleplate.png")
                assert changed(
                    OUTPUT / "panel-long-dialogue.png",
                    OUTPUT / "panel-mira-titleplate.png"
                ) > 500, "Named dialogue screen did not render"
                assert changed(
                    OUTPUT / "panel-later-dialogue.png",
                    OUTPUT / "panel-long-dialogue.png"
                ) > 800, "Long dialogue did not render a distinct page"
                assert not FAILURE.search(read_log(log)), read_log(log)[-3000:]
                press(win, "Escape", 0.2)
                process.wait(timeout=15)
                assert process.returncode == 0
                stored = json.loads((prefs / "player-preferences.json").read_text())
                assert 85.0 <= stored["panel_width_percent"] <= 100.0, stored
                print("Atlas panel: compact, standard, ultrawide, reader width "
                      "and persisted preference passed", flush=True)
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
    if "--layout-only" in sys.argv:
        responsive_layout()
    else:
        main()
