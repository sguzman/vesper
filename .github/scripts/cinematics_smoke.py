"""VX05: verify authored background transitions with REAL rendered frames.

Both source games are copied to a temporary directory; all scripts/theme
changes are in the clone. No source .vn, artwork or saves are modified.
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
BIN = ROOT / "target/debug/vesper-player"
OUT = Path("/tmp/vesper-cinematics-qa")
MODES = ("crossfade", "fade_through_color", "soft_slide")
GAMES = (
    ("little-lantern", "The Little Lantern", "first_light", "sunrise", "sunset", "styles/lantern.toml"),
    ("clockwork-moon", "The Clockwork Moon", "threshold", "cottage", "forest", "styles/moonlight.toml"),
)


def run(*args, check=True):
    return subprocess.run(args, text=True, capture_output=True, check=check, timeout=25)


def wait(log: Path, needle: str, count: int = 1, timeout=20.):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        if log.exists() and log.read_text(errors="replace").count(needle) >= count:
            return
        time.sleep(.12)
    raise AssertionError(f"{needle!r} missing from {log}: {log.read_text(errors='replace')[-3000:]}")


def window_id(title):
    deadline = time.monotonic() + 20
    while time.monotonic() < deadline:
        result = run("xdotool", "search", "--name", title, check=False)
        if result.returncode == 0 and result.stdout.strip():
            win = result.stdout.splitlines()[0]
            run("xdotool", "windowfocus", "--sync", win)
            return win
        time.sleep(.2)
    raise AssertionError(f"no player window titled {title}")


def key(name, delay=.2):
    run("xdotool", "key", "--clearmodifiers", name)
    time.sleep(delay)


def shot(win, image):
    run("import", "-window", win, str(image))


def pixels(a,b):
    comparison = run("compare", "-metric", "AE", str(a), str(b), "null:", check=False)
    match = re.match(r"^\d+", comparison.stderr.strip())
    assert match, comparison.stderr
    return int(match.group())


def clone_fixture(tmp, game_name, scene, first, second, theme_name, kind):
    copy = tmp / f"{game_name}-{kind}"
    shutil.copytree(ROOT / "examples" / game_name, copy)
    theme = copy / theme_name
    theme.write_text(theme.read_text() + (
        "\n[cinematic]\n"
        f'transition = "{kind}"\n'
        "seconds = 1.4\n"
        "through_color = [0.26, 0.09, 0.32, 1.0]\n"
        "slide_pixels = 35.0\n"
        "motion_seconds = 12.0\n"
        "motion_zoom = 1.12\n"
        "motion_pan = [0.02, -0.01]\n"
    ))
    if game_name == "little-lantern":
        # A second differently colored image is derived inside this temporary
        # fixture, never added to the authored repository.
        run("convert", str(copy/"assets/backgrounds/sunrise.png"),
            "-modulate", "100,125,175", str(copy/"assets/backgrounds/sunset.png"))
        manifest = copy/"game.toml"
        manifest.write_text(manifest.read_text().replace(
            'sunrise = "assets/backgrounds/sunrise.png"',
            'sunrise = "assets/backgrounds/sunrise.png"\n'
            'sunset = "assets/backgrounds/sunset.png"'
        ))
    manifest = copy/"game.toml"
    line = next(line.strip() for line in manifest.read_text().splitlines()
                if line.startswith('story = '))
    story = copy / line.split('"')[1]
    story.write_text(
        f"@scene {scene}\n"
        f"background {first}\n"
        '"First panorama: stars are just beginning to glow above the village."\n'
        f"background {second}\n"
        '"Second panorama: the other world becomes visible behind the story."\n'
        f"background {first}\n"
        '"Third panorama: the original lights return across the valley."\n'
        '"The End."\n'
    )
    return copy, theme


def scenario(tmp, game_name, title, scene, first, second, theme_name, kind):
    game, theme = clone_fixture(tmp, game_name, scene, first, second, theme_name, kind)
    out = OUT/f"{game_name}-{kind}"
    out.mkdir(parents=True, exist_ok=True)
    log = out/"playback.log"
    env = dict(os.environ, LIBGL_ALWAYS_SOFTWARE="1",
               VESPER_TRACE_SCENES="1",
               VESPER_SAVE_DIR=str(out/"saves"),
               VESPER_PREFERENCES_DIR=str(out/"preferences"))
    with log.open("w") as stream:
        proc = subprocess.Popen([str(BIN), str(game), "--watch"],
                                env=env, stdout=stream, stderr=subprocess.STDOUT)
        try:
            win = window_id(title)
            wait(log, f"Vesper scene dialogue: {scene}")
            time.sleep(1.2)
            first_image = out/"first-frame.png"
            shot(win,first_image)
            key("Return", .15)
            wait(log,f"Vesper cinematic started: {kind.replace('_',' ').title().replace(' ','')}")
            initial = out/"transition-start.png"
            shot(win,initial)
            if kind == "crossfade" and game_name == "little-lantern":
                key("p")
                wait(log,"Vesper story paused")
                time.sleep(.25)
                paused = out/"paused.png"
                shot(win,paused)
                time.sleep(.5)
                still = out/"paused-still.png"
                shot(win,still)
                assert pixels(paused,still) == 0,"pause did not freeze the cinematic"
                key("p")
            time.sleep(.55)
            halfway=out/"transition-middle.png"
            shot(win,halfway)
            wait(log,f"Vesper cinematic complete: {kind.replace('_',' ').title().replace(' ','')}",timeout=15)
            time.sleep(.15)
            end=out/"transition-end.png"
            shot(win,end)
            assert pixels(first_image,end)>2000, "background did not change"
            assert pixels(initial,end)>500, "no transition visible across frames"
            assert pixels(halfway,end)>250, "transition lacked a distinct midpoint"
            assert proc.poll() is None,"player crashed"
            # A second rapid transition is explicitly skipped with K.
            key("Return",.16)
            wait(log,"Vesper cinematic started:",2)
            key("k",.10)
            wait(log,"Vesper cinematic skipped")
            wait(log,"Vesper cinematic complete:",2)
            restored=out/"skipped-back-to-first.png"
            shot(win,restored)
            assert pixels(restored,end)>2000, "skip left previous art on screen"
            if kind == "soft_slide" and game_name == "clockwork-moon":
                key("F5",.5)
                wait(log,"Vesper save written")
                key("F9",.4)
                wait(log,"Vesper save restored")
                assert proc.poll() is None
            if kind == "fade_through_color" and game_name == "clockwork-moon":
                # A live external theme edit must not reset narrative state or
                # retain the old transition curtain.
                source = theme.read_text()
                theme.write_text(source.replace("motion_zoom = 1.12","motion_zoom = 1.14"))
                wait(log,"Vesper hot reload applied")
                assert proc.poll() is None
            print(f"VX05 {game_name}/{kind}: start/mid/end, pause/skip, restore and hot reload pass",flush=True)
        finally:
            if proc.poll() is None:
                proc.terminate()
                proc.wait(timeout=10)


def main():
    OUT.mkdir(parents=True,exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="vesper-cinematics-") as tmp:
        for case in GAMES:
            for kind in MODES:
                scenario(Path(tmp), *case, kind)


if __name__=="__main__":
    main()
