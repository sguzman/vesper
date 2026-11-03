#!/usr/bin/env python3
"""Native Bevy speech-bubble smoke: old visual novel vs bubble-only fixture.

Uses the actual player and real window pixels, not a mocked layout. The two
externally authored games share a story/art package; only the optional
[dialogue] surface setting differs. No arbitrary scripted game logic.
"""
from __future__ import annotations

import os
import json
from pathlib import Path
import re
import shutil
import subprocess
import tempfile
import time

ROOT = Path(__file__).resolve().parents[2]
PLAYER = ROOT / "target/debug/vesper-player"
OUTPUT = Path("/tmp/vesper-speech-bubbles-qa")
OUTPUT.mkdir(parents=True, exist_ok=True)


def call(*args: str, check: bool = True) -> subprocess.CompletedProcess[str]:
    return subprocess.run(args, capture_output=True, text=True, check=check)


def wait_window(title: str) -> str:
    deadline = time.monotonic() + 30
    while time.monotonic() < deadline:
        result = call("xdotool", "search", "--name", title, check=False)
        if result.returncode == 0 and result.stdout.strip():
            return result.stdout.splitlines()[0]
        time.sleep(0.2)
    raise AssertionError(f"Window never opened: {title}")


def grab(win: str, name: str, size: tuple[int, int]) -> Path:
    out = OUTPUT / name
    for _ in range(30):
        call("import", "-window", win, str(out))
        result = call("identify", "-format", "%w %h %k", str(out))
        width, height, colors = [int(n) for n in result.stdout.split()]
        if (width, height) == size and colors > 80:
            return out
        time.sleep(0.2)
    raise AssertionError(f"No rendered {size} frame for {name}")


def diff(a: Path, b: Path) -> int:
    compare = call("compare", "-metric", "AE", str(a), str(b), "null:", check=False)
    match = re.search(r"^\s*(\d+)", compare.stderr)
    assert match, compare.stderr
    return int(match[1])


def wait_for_phrase(path: Path, phrase: str, after: int = 0) -> None:
    """Wait for a new event, not an old matching log line."""
    deadline = time.monotonic() + 12
    while time.monotonic() < deadline:
        if path.is_file() and path.read_text(
            encoding="utf-8", errors="replace"
        ).count(phrase) > after:
            return
        time.sleep(0.1)
    raise AssertionError(f"Missing new {phrase!r}: {path}")


def press(win: str, key: str) -> None:
    call("xdotool", "windowfocus", "--sync", win)
    call("xdotool", "key", "--clearmodifiers", key)
    time.sleep(0.6)


def run_game(directory: str, title: str, filename: str,
             prefs: dict[str, object] | None = None,
             full_route: bool = True) -> tuple[Path, Path, Path | None]:
    with tempfile.TemporaryDirectory(prefix="vesper-bubble-") as folder:
        if prefs is not None:
            (Path(folder) / "player-preferences.json").write_text(
                json.dumps({"format_version": 1, **prefs}), encoding="utf-8"
            )
        env = dict(os.environ, LIBGL_ALWAYS_SOFTWARE="1",
                   VESPER_PREFERENCES_DIR=folder,
                   VESPER_SAVE_DIR=str(Path(folder) / "saves"))
        logfile = OUTPUT / (filename + ".log")
        with logfile.open("w", encoding="utf-8") as stream:
            process = subprocess.Popen([str(PLAYER), str(ROOT / directory)],
                                       stdout=stream, stderr=subprocess.STDOUT,
                                       env=env)
            try:
                win = wait_window(title)
                time.sleep(2.0)
                initial = grab(win, filename + "-initial.png", (1280, 720))
                if directory == "examples/bubble-showcase" and prefs is None:
                    assert "Vesper bubble scroll hint visible" not in logfile.read_text(
                        encoding="utf-8", errors="replace"
                    ), "Default-size short dialogue should not show a paging hint"
                # Text at 160% scale may genuinely fill the capped safe
                # region even for a short line. That is valid scrolling,
                # not a decorative cue regression.
                if directory != "examples/bubble-showcase" or not full_route:
                    return initial, logfile, None
                # A left click in the story window is a native dialogue
                # advance, independent of keyboard control and movement.
                # Menus, history and choices retain higher input priority.
                call("xdotool", "windowfocus", "--sync", win)
                call("xdotool", "mousemove", "--window", win, "940", "610")
                call("xdotool", "click", "--clearmodifiers", "1")
                time.sleep(.65)
                panel = grab(win, filename + "-panel.png", (1280, 720))
                assert diff(initial, panel) > 3000, (
                    "Per-line 'surface panel' did not switch to a traditional box"
                )
                # Save the explicit 'surface panel' override and verify that
                # quick-loading it restores both story position and dialog UI.
                press(win, "F5")
                saved = Path(folder) / "saves" / "bubble-showcase.json"
                deadline = time.monotonic() + 12
                while not saved.exists() and time.monotonic() < deadline:
                    time.sleep(0.1)
                assert saved.exists(), "Bubble Showcase F5 save was not written"
                state = json.loads(saved.read_text(encoding="utf-8"))
                assert state["presentation"]["dialogue_surface"] == "panel", (
                    "Explicit panel dialogue surface missing from save"
                )
                press(win, "Return")
                speaker = grab(win, filename + "-speaker.png", (1280, 720))
                assert diff(panel, speaker) > 3000, (
                    "Per-line 'surface bubble' did not return to anchored dialogue"
                )
                previous_loads = logfile.read_text(
                    encoding="utf-8", errors="replace"
                ).count("Vesper save restored")
                press(win, "F9")
                wait_for_phrase(logfile, "Vesper save restored", after=previous_loads)
                time.sleep(0.8)
                restored_panel = grab(win, filename + "-restored-panel.png", (1280, 720))
                assert diff(restored_panel, panel) < 15000, (
                    "F9 restored the surface but changed its original panel geometry"
                )
                # Once restored, advancing should honor the next authored
                # surface command rather than keeping a stale restored panel.
                press(win, "Return")
                resumed = grab(win, filename + "-resumed-bubble.png", (1280, 720))
                assert diff(resumed, restored_panel) > 3000, (
                    "Dialogue remained stuck on saved surface after F9"
                )
                call("xdotool", "windowsize", "--sync", win, "946", "511")
                time.sleep(0.8)
                compact = grab(win, filename + "-compact.png", (946, 511))
                assert compact.exists()
                call("xdotool", "windowsize", "--sync", win, "1280", "720")
                time.sleep(0.7)
                press(win, "Return")  # a choice scene uses the standard option panel
                choices = grab(win, filename + "-choices.png", (1280, 720))
                assert diff(speaker, choices) > 3000, "Bubble did not yield to choices"
                log = logfile.read_text(encoding="utf-8", errors="replace")
                assert "Vesper dialogue surface: Panel" in log
                assert "Vesper dialogue surface: Bubble" in log
                return initial, logfile, choices
            finally:
                process.terminate()
                try:
                    process.wait(timeout=6)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait(timeout=3)



def long_dialogue_scroll() -> None:
    """One real Bevy process exercises a lengthy multilingual bubble."""
    with tempfile.TemporaryDirectory(prefix="vesper-long-bubble-") as folder:
        game = Path(folder) / "game"
        shutil.copytree(ROOT / "examples/bubble-showcase", game)
        story = game / "scripts/story.vn"
        contents = story.read_text(encoding="utf-8")
        original = (
            "At the far end of a quiet valley, a little fox named Pip "
            "carried a lantern no bigger than an apple."
        )
        sample = (
            "En la estación había una pequeña lámpara. "
            "Árboles y corazones, mañana y café; "
            "日本語の文字と絵文字 🌙✨, 그리고 안녕하세요. "
            "The road kept winding gently through the valley. "
        ) * 12
        assert contents.count(original) == 1
        # Consecutive identical, overlong bubble lines must not inherit
        # each other's scroll offset. This also tests position, not text,
        # as the reader's progression identity.
        story.write_text(
            contents.replace(f'"{original}"', f'"{sample}"\n"{sample}"'),
            encoding="utf-8",
        )
        preferences = Path(folder) / "prefs"
        preferences.mkdir()
        (preferences / "player-preferences.json").write_text(
            json.dumps({
                "format_version": 1, "high_contrast": True,
                "text_scale": 1.6, "reduced_motion": True,
                "reveal_speed": "instant",
            }), encoding="utf-8",
        )
        env = dict(os.environ, LIBGL_ALWAYS_SOFTWARE="1",
                   VESPER_PREFERENCES_DIR=str(preferences),
                   VESPER_SAVE_DIR=str(Path(folder) / "saves"))
        log = OUTPUT / "long-multilingual.log"
        with log.open("w", encoding="utf-8") as stream:
            player = subprocess.Popen(
                [str(PLAYER), str(game)],
                stdout=stream, stderr=subprocess.STDOUT, env=env,
            )
            try:
                win = wait_window("Speech Bubble Rendering Fixture")
                call("xdotool", "windowsize", "--sync", win, "946", "511")
                time.sleep(1.7)
                top = grab(win, "long-compact-top.png", (946, 511))
                # Long text may be much taller than its native scroll area,
                # but the bubble itself must never blanket Pip's face. An
                # 8x8 patch of the meadow at x=730,y=325 is covered by the
                # old fullscreen-height black bubble; it is visible when
                # the scroll area respects the upper-scene safe region.
                patch = call(
                    "convert", str(top), "-crop", "8x8+726+325", "+repage",
                    "-colorspace", "Gray", "-format", "%[fx:mean]", "info:",
                )
                assert float(patch.stdout.strip()) > 0.07, (
                    "Long bubble obscures the central scene and speaker face",
                    patch.stdout, patch.stderr,
                )
                wait_for_phrase(log, "Vesper bubble scroll hint visible")
                press(win, "Next")  # PageDown
                wait_for_phrase(log, "Vesper bubble scroll:")
                lower = grab(win, "long-compact-scrolled.png", (946, 511))
                assert diff(top, lower) > 200, (
                    "Long multilingual bubble did not show scrolling content"
                )
                # A second *identical* dialogue line is a distinct authored
                # event; it must start unscrolled, even though the text is
                # byte-for-byte identical to the previous line.
                press(win, "Return")
                reset = grab(win, "long-duplicate-line-reset.png", (946, 511))
                assert diff(top, reset) < 800, (
                    "An identical second line inherited the first line's scroll"
                )
                previous_scrolls = log.read_text(encoding="utf-8").count(
                    "Vesper bubble scroll:"
                )
                press(win, "Next")
                wait_for_phrase(log, "Vesper bubble scroll:", previous_scrolls)
                # Saving while the second identical line is scrolled and
                # restoring it must reset to the first visible paragraph,
                # even when scene/cursor/text are identical after the load.
                press(win, "F5")
                wait_for_phrase(log, "Vesper save written")
                press(win, "F9")
                wait_for_phrase(log, "Vesper save restored")
                time.sleep(.5)
                reloaded = grab(win, "long-compact-reloaded-top.png", (946, 511))
                assert diff(top, reloaded) < 800, (
                    "F9 restored bubble narrative position but leaked old scroll offset"
                )
                press(win, "Home")
                returned = grab(win, "long-compact-back-top.png", (946, 511))
                assert diff(top, returned) < 800, (
                    "Bubble did not return to top of viewport after Home"
                )
                assert player.poll() is None
                print("Native long multilingual bubble remains scrollable at 946x511: passed",
                      flush=True)
            finally:
                player.terminate()
                try:
                    player.wait(timeout=6)
                except subprocess.TimeoutExpired:
                    player.kill()
                    player.wait(timeout=3)


def authored_bubble_frame(reference: Path) -> None:
    """Prove the native player displays a game-local sliced PNG bubble frame.

    The art is generated in this isolated QA fixture; the engine never draws
    or decodes it on the frame thread. Existing VN fixtures stay untouched.
    """
    with tempfile.TemporaryDirectory(prefix="vesper-authored-bubble-") as folder:
        game = Path(folder) / "game"
        shutil.copytree(ROOT / "examples/bubble-showcase", game)
        art = game / "assets/ui/bubble-frame.png"
        art.parent.mkdir(parents=True, exist_ok=True)
        call(
            "convert", "-size", "192x96", "xc:#283f59",
            "-stroke", "#e8ce88", "-strokewidth", "7",
            "-fill", "none", "-draw", "roundrectangle 7,7 185,89 16,16",
            str(art),
        )
        manifest = game / "game.toml"
        authored = manifest.read_text(encoding="utf-8")
        setting = "corner_radius = 18.0"
        assert authored.count(setting) == 1
        manifest.write_text(authored.replace(
            setting,
            setting + '\nimage = "assets/ui/bubble-frame.png"'
                    + '\nslice_border = 16.0',
        ), encoding="utf-8")
        prefs = Path(folder) / "prefs"
        prefs.mkdir()
        (prefs / "player-preferences.json").write_text(
            json.dumps({"format_version": 1, "reveal_speed": "instant"}),
            encoding="utf-8",
        )
        env = dict(
            os.environ, LIBGL_ALWAYS_SOFTWARE="1",
            VESPER_PREFERENCES_DIR=str(prefs),
            VESPER_SAVE_DIR=str(Path(folder) / "saves"),
        )
        log = OUTPUT / "authored-bubble-frame.log"
        with log.open("w", encoding="utf-8") as stream:
            player = subprocess.Popen(
                [str(PLAYER), str(game)],
                stdout=stream, stderr=subprocess.STDOUT, env=env,
            )
            try:
                win = wait_window("Speech Bubble Rendering Fixture")
                time.sleep(2.0)
                frame = grab(win, "authored-frame-1280.png", (1280, 720))
                difference = diff(reference, frame)
                assert difference > 6000, (
                    "Game-authored sliced bubble frame did not render",
                    difference, log.read_text(encoding="utf-8", errors="replace")[-1000:],
                )
                call("xdotool", "windowsize", "--sync", win, "946", "511")
                time.sleep(0.8)
                compact = grab(win, "authored-frame-compact.png", (946, 511))
                assert compact.is_file() and player.poll() is None
                print(
                    "Game-authored optional nine-slice PNG bubble frame at "
                    "1280x720 and 946x511: passed", flush=True,
                )
            finally:
                player.terminate()
                try:
                    player.wait(timeout=6)
                except subprocess.TimeoutExpired:
                    player.kill()
                    player.wait(timeout=3)


def main() -> None:
    bubble, _, choices = run_game(
        "examples/bubble-showcase", "Speech Bubble Rendering Fixture", "bubble"
    )
    classic, _, _ = run_game(
        "examples/little-lantern", "The Little Lantern", "classic"
    )
    difference = diff(bubble, classic)
    assert difference > 4000, (
        "A pure VN bubble default looks the same as the traditional dialogue panel",
        difference,
    )
    assert choices is not None
    # Test actual reader preferences in a second native process. High contrast
    # must override authored colors while reduced motion and increased font
    # scale keep the fallback bubble readable at a compact viewport.
    high_contrast, _, _ = run_game(
        "examples/bubble-showcase", "Speech Bubble Rendering Fixture",
        "bubble-high-contrast",
        prefs={"high_contrast": True, "text_scale": 1.6,
               "reduced_motion": True, "reveal_speed": "instant"},
        full_route=False,
    )
    assert diff(bubble, high_contrast) > 2000, (
        "High contrast/large text preferences were not applied to bubbles"
    )
    long_dialogue_scroll()
    authored_bubble_frame(bubble)
    print(f"Native optional bubble vs legacy panel: {difference} different pixels", flush=True)
    print("Speech bubbles, saved panel restoration, contrast, font scale, choices, compact layout: passed",
          flush=True)


if __name__ == "__main__":
    main()
