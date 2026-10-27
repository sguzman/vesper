"""Automated Linux/Xvfb smoke test of Vesper's external live reload.

Uses the built player binary. No private files, service or test framework needed.
"""
from __future__ import annotations

import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile
import time

ROOT = Path(__file__).resolve().parents[2]


def pixels(before: Path, after: Path) -> int:
    result = subprocess.run(
        ["compare", "-metric", "AE", str(before), str(after), "null:"],
        capture_output=True,
        text=True,
        check=False,
    )
    match = re.match(r"\s*(\d+)", result.stderr)
    if match is None:
        raise AssertionError(f"image comparison failed: {result.stderr}")
    return int(match.group(1))


def window_id() -> str:
    end = time.monotonic() + 30
    while time.monotonic() < end:
        result = subprocess.run(
            ["xdotool", "search", "--name", "Vesper Demonstration"],
            capture_output=True,
            text=True,
            check=False,
        )
        if result.returncode == 0 and result.stdout.strip():
            return result.stdout.strip().splitlines()[0]
        time.sleep(0.4)
    raise AssertionError("player window never appeared")


def capture(window: str, target: Path) -> None:
    subprocess.run(["import", "-window", window, str(target)], check=True)


def await_log(log: Path, phrase: str, number: int = 1, timeout: float = 12) -> None:
    end = time.monotonic() + timeout
    while time.monotonic() < end:
        if log.exists() and log.read_text(errors="replace").count(phrase) >= number:
            return
        time.sleep(0.3)
    raise AssertionError(f"missing log message: {phrase!r}\n{log.read_text(errors='replace')[-5000:]}")


def main() -> None:
    with tempfile.TemporaryDirectory(prefix="vesper-hotreload-") as temp:
        work = Path(temp)
        game = work / "game"
        shutil.copytree(ROOT / "game", game)
        log = work / "player.log"
        theme = game / "styles/dialogue.toml"
        story = game / "scripts/demo.vn"
        background = game / "assets/backgrounds/bedroom.png"
        args = [str(ROOT / "target/debug/vesper-player"), str(game), "--watch"]
        env = dict(os.environ, LIBGL_ALWAYS_SOFTWARE="1",
                   VESPER_SAVE_DIR=str(work / "saves"))
        with log.open("w", encoding="utf-8") as stream:
            player = subprocess.Popen(args, stdout=stream, stderr=subprocess.STDOUT, env=env)
            try:
                window = window_id()
                await_log(log, "Vesper decoded audio asset: music:evening")
                time.sleep(2)
                initial = work / "initial.png"
                capture(window, initial)

                old_theme = theme.read_text(encoding="utf-8")
                theme.write_text(
                    old_theme.replace("width_percent = 90.0", "width_percent = 62.0")
                             .replace("size = 30.0", "size = 38.0"),
                    encoding="utf-8",
                )
                await_log(log, "Vesper hot reload applied", 1)
                time.sleep(1)
                changed_theme = work / "theme.png"
                capture(window, changed_theme)
                theme_difference = pixels(initial, changed_theme)
                print(f"Valid external theme changed {theme_difference} pixels", flush=True)
                assert theme_difference >= 2000

                old_story = story.read_text(encoding="utf-8")
                edited_story = old_story.replace(
                    'elena: "You\'re finally awake."',
                    'elena: "This dialogue was changed at runtime without recompilation."',
                )
                assert edited_story != old_story
                story.write_text(edited_story, encoding="utf-8")
                await_log(log, "Vesper hot reload applied", 2)
                time.sleep(1)
                changed_story = work / "story.png"
                capture(window, changed_story)
                story_difference = pixels(changed_theme, changed_story)
                print(f"Valid dialogue edit changed {story_difference} pixels", flush=True)
                assert story_difference >= 100

                # Invalid source may never overwrite the last working scene.
                story.write_text(edited_story.replace(
                    'elena: "This dialogue was changed at runtime without recompilation."',
                    'elena: "Invalid missing closing quote',
                ), encoding="utf-8")
                await_log(log, "Vesper hot reload rejected", 1)
                time.sleep(1)
                after_invalid = work / "invalid.png"
                capture(window, after_invalid)
                invalid_difference = pixels(changed_story, after_invalid)
                print(f"Rejected invalid edit changed {invalid_difference} pixels", flush=True)
                assert invalid_difference == 0

                story.write_text(edited_story, encoding="utf-8")
                await_log(log, "Vesper hot reload applied", 3)

                # Bevy's native file watcher should update image handles in place.
                subprocess.run(
                    ["convert", "-size", "160x90", "xc:#1133cc", str(background)],
                    check=True,
                )
                time.sleep(4)
                after_art = work / "art.png"
                capture(window, after_art)
                art_difference = pixels(after_invalid, after_art)
                print(f"External image hot reload changed {art_difference} pixels", flush=True)
                assert art_difference >= 5000
                assert player.poll() is None, "player exited unexpectedly"
            finally:
                player.terminate()
                try:
                    player.wait(timeout=6)
                except subprocess.TimeoutExpired:
                    player.kill()
                    player.wait()


if __name__ == "__main__":
    main()
