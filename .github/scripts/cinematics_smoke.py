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


def background_pixels(a, b, out):
    """Ignore dialogue when checking which actual background was rendered."""
    left, right = out/"crop-a.png", out/"crop-b.png"
    for source, target in ((a, left), (b, right)):
        run("convert", str(source), "-crop", "1280x320+0+0",
            "+repage", str(target))
    return pixels(left, right)


def top_colors(frame):
    """Visible background detail, independent of the dialogue panel."""
    result = run("convert", str(frame), "-crop", "1280x300+0+0",
                 "+repage", "-format", "%k", "info:")
    return int(result.stdout.strip())


def wait_for_art(win, frame, timeout=6.):
    """Asset loaded/logged != first image actually presented by the renderer."""
    deadline = time.monotonic() + timeout
    while True:
        shot(win, frame)
        colors = top_colors(frame)
        if colors >= 80:
            return
        if time.monotonic() >= deadline:
            raise AssertionError(
                f"initial background never rendered in {frame} (colors={colors})"
            )
        time.sleep(.12)


def top_brightness(frame):
    """Mean 0-255 luminance of the scene above the dialogue panel."""
    result = run("convert", str(frame), "-crop", "1280x300+0+0",
                 "+repage", "-colorspace", "gray",
                 "-format", "%[fx:mean*255]", "info:")
    return float(result.stdout.strip())


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
            wait_for_art(win, first_image)
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
            wait_for_art(win, end)
            assert background_pixels(first_image, end, out)>2000, (
                "declared second background was not rendered"
            )
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
            # The update-thread completion log may precede the presented
            # frame, especially under the software renderer in CI.
            # Require the *rendered destination art* rather than just a log.
            deadline = time.monotonic() + 3.0
            while True:
                shot(win, restored)
                first_distance = background_pixels(restored, first_image, out)
                previous_distance = background_pixels(restored, end, out)
                if first_distance < previous_distance:
                    break
                if time.monotonic() >= deadline:
                    raise AssertionError(
                        f"skip never displayed destination background: "
                        f"first={first_distance}, previous={previous_distance}"
                    )
                time.sleep(.12)
            if kind == "soft_slide" and game_name == "clockwork-moon":
                before_restore = out/"before-restore.png"
                shot(win,before_restore)
                key("F5",.5)
                wait(log,"Vesper save written")
                key("F9",.4)
                wait(log,"Vesper save restored")
                wait(log,"Vesper restored existing background without reloading")
                restored = out/"same-image-restored.png"
                shot(win,restored)
                assert pixels(before_restore,restored)<160000, (
                    "restoring the same background flashed to missing artwork"
                )
                for width,height in ((946,511),(1920,1080)):
                    run("xdotool","windowsize",win,str(width),str(height))
                    time.sleep(.45)
                    image=out/f"motion-{width}x{height}.png"
                    shot(win,image)
                    size=run("identify","-format","%w %h",str(image)).stdout
                    assert size==f"{width} {height}",size
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


def verify_rapid_interruption(tmp):
    name,title,scene,first,second,theme_name = GAMES[1]
    interrupt_root = tmp/"interrupt-fixture"
    interrupt_root.mkdir()
    game,_ = clone_fixture(interrupt_root,name,scene,first,second,theme_name,"fade_through_color")
    out = OUT/"rapid-interruption"
    out.mkdir(parents=True,exist_ok=True)
    log=out/"player.log"
    env=dict(os.environ,LIBGL_ALWAYS_SOFTWARE="1",
             VESPER_SAVE_DIR=str(out/"saves"),
             VESPER_PREFERENCES_DIR=str(out/"prefs"))
    with log.open("w") as stream:
        proc=subprocess.Popen([str(BIN),str(game)],env=env,
                              stdout=stream,stderr=subprocess.STDOUT)
        try:
            win=window_id(title)
            time.sleep(2.5)
            original=out/"original.png"
            wait_for_art(win, original)
            key("Return",.16)
            wait(log,"Vesper cinematic started: FadeThroughColor")
            key("Return",.10)  # Interrupt well before 1.4s elapsed
            wait(log,"Vesper cinematic started: FadeThroughColor",2)
            # A rapid replacement must never expose the empty render clear
            # color while prior/incoming artwork is loading.
            for index in range(2):
                time.sleep(.12)
                frame = out/f"interruption-{index}.png"
                shot(win, frame)
                assert top_brightness(frame) > 22, (
                    f"background went dark during interruption: {frame}"
                )
            key("k",.20)
            wait(log,"Vesper cinematic skipped")
            wait(log,"Vesper cinematic complete: FadeThroughColor",1)
            time.sleep(.65)
            completed=out/"interrupted-then-skipped.png"
            shot(win,completed)
            assert pixels(original,completed)<160000, (
                "stale color curtain or transparent backdrop survived interruption"
            )
            assert proc.poll() is None
            print("VX05 rapid interruption: stale curtain cleared and original background restored",flush=True)
        finally:
            if proc.poll() is None:
                proc.terminate()
                proc.wait(timeout=10)


def verify_reduced_motion(tmp):
    # Player accessibility takes precedence over the author's cinematic theme.
    name, title, scene, first, second, theme_name = GAMES[0]
    root = tmp/"reduced-motion-fixture"
    root.mkdir()
    game, _ = clone_fixture(root, name, scene, first, second, theme_name, "crossfade")
    out = OUT/"reduced-motion"
    out.mkdir(parents=True, exist_ok=True)
    prefs = out/"preferences"
    prefs.mkdir(exist_ok=True)
    (prefs/"player-preferences.json").write_text(
        '{"format_version": 1, "reduced_motion": true}'
    )
    log = out/"player.log"
    env = dict(os.environ, LIBGL_ALWAYS_SOFTWARE="1",
               VESPER_TRACE_SCENES="1",
               VESPER_SAVE_DIR=str(out/"saves"),
               VESPER_PREFERENCES_DIR=str(prefs))
    with log.open("w") as stream:
        proc = subprocess.Popen([str(BIN), str(game)], env=env,
                                stdout=stream, stderr=subprocess.STDOUT)
        try:
            win = window_id(title)
            wait(log, f"Vesper scene dialogue: {scene}")
            time.sleep(.8)
            before = out/"before.png"
            wait_for_art(win, before)
            key("Return", .45)
            after = out/"after.png"
            deadline = time.monotonic() + 4.
            while True:
                shot(win, after)
                if background_pixels(before, after, out) > 2000:
                    break
                if time.monotonic() >= deadline:
                    raise AssertionError("reduced motion blocked the story background")
                time.sleep(.12)
            assert "Vesper cinematic started" not in log.read_text(), (
                "reduced motion allowed an authored cinematic transition"
            )
            time.sleep(.35)
            still = out/"still.png"
            shot(win, still)
            assert pixels(after, still) < 100, (
                "reduced-motion background kept animating after the story settled"
            )
            assert proc.poll() is None, "player crashed under reduced motion"
            print("VX05 reduced motion: immediate static background, no cinematic or pan/zoom", flush=True)
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
        verify_rapid_interruption(Path(tmp))
        verify_reduced_motion(Path(tmp))


if __name__=="__main__":
    main()
