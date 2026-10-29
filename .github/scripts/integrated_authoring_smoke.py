"""QOL08 all-in-one, external-authored game acceptance in real Bevy/Xvfb.

Six windows, notebook and banner skins, dynamic word effects, three ambient
motions, reader-owned installed-font override, auto delay, F5/F9, high contrast,
and reduced motion. No game-specific engine code or manual application state.
"""
from __future__ import annotations

import json
import os
from pathlib import Path
import re
import subprocess
import tempfile
import time

ROOT = Path(__file__).resolve().parents[2]
GAME = ROOT / "examples/living-journal"
PLAYER = ROOT / "target/debug/vesper-player"
OUT = Path("/tmp/vesper-integrated-qa")
SIZES = ((946, 511), (955, 520), (800, 600), (900, 700),
         (1280, 720), (1920, 1080))


def call(*args, check=True):
    return subprocess.run(args, capture_output=True, text=True,
                          check=check, timeout=30)


def wait(log, fragment, previous=0, limit=30):
    deadline = time.monotonic() + limit
    while time.monotonic() < deadline:
        if log.exists() and log.read_text(errors="replace").count(fragment) > previous:
            return
        time.sleep(.15)
    raise AssertionError(f"missing {fragment}: {log.read_text(errors='replace')[-2000:]}")


def key(name, delay=.3):
    call("xdotool", "key", "--clearmodifiers", name)
    time.sleep(delay)


def image(win, target, width=1280, height=720):
    end = time.monotonic() + 15
    while time.monotonic() < end:
        time.sleep(.35)
        call("import", "-window", win, str(target))
        actual = call("identify", "-format", "%w %h", str(target)).stdout
        assert actual == f"{width} {height}", (target, actual)
        # Not merely a UI outline rendered before the async background image.
        top_colors = int(call("convert", str(target),
                              "-crop", f"{width//2}x{max(60,height//4)}+{width//4}+0",
                              "+repage", "-format", "%k", "info:").stdout)
        if top_colors > 10:
            return target
    raise AssertionError(f"unready picture: {target}")


def crop(source, target, region):
    call("convert", str(source), "-crop", region, "+repage", str(target))
    return target


def different(left, right):
    result = call("compare", "-metric", "AE", str(left), str(right),
                  "null:", check=False)
    match = re.match(r"^\d+", result.stderr.strip())
    assert match, result.stderr
    return int(match.group())


def run_case(name, *, font=None, contrast=False, reduced=False, auto=False):
    directory = OUT / name
    directory.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="vesper-integrated-") as tmp:
        home = Path(tmp)
        prefs = home / "prefs"
        prefs.mkdir()
        settings = {"format_version": 1, "reveal_speed": "instant",
                    "auto_advance": auto, "auto_advance_delay": .8,
                    "high_contrast": contrast, "reduced_motion": reduced}
        if font is not None:
            settings["font_family"] = font
        (prefs / "player-preferences.json").write_text(json.dumps(settings))
        log = directory / "player.log"
        env = dict(os.environ, LIBGL_ALWAYS_SOFTWARE="1",
                   VESPER_TRACE_SCENES="1",
                   VESPER_PREFERENCES_DIR=str(prefs),
                   VESPER_SAVE_DIR=str(home / "saves"))
        with log.open("w") as stream:
            process = subprocess.Popen([str(PLAYER), str(GAME)], env=env,
                                       stdout=stream, stderr=subprocess.STDOUT)
            try:
                win = None
                until = time.monotonic() + 25
                while time.monotonic() < until:
                    result = call("xdotool", "search", "--name",
                                  "The Living Journal", check=False)
                    if result.stdout.strip():
                        win = result.stdout.splitlines()[0]
                        break
                    time.sleep(.2)
                assert win, "Living Journal window unavailable"
                call("xdotool", "windowfocus", "--sync", win)
                wait(log, "Vesper ambient actors spawned: 3")
                wait(log, "Vesper inline text spans mounted")
                wait(log, 'Vesper dialogue panel: Some("notebook")')
                call("xdotool", "windowsize", "--sync", win, "1280", "720")
                first = image(win, directory / "opening.png")
                if name == "normal":
                    time.sleep(.7)
                    later = image(win, directory / "later.png")
                    stage_a = crop(first, directory / "stage-start.png", "1280x170+0+0")
                    stage_b = crop(later, directory / "stage-later.png", "1280x170+0+0")
                    assert different(stage_a, stage_b) > 60, "bird and plane never moved"
                    # The authored walker belongs in the visible meadow above
                    # the notebook, not entirely behind its decorative PNG.
                    walker_a = crop(first, directory / "walker-start.png", "1280x175+0+260")
                    walker_b = crop(later, directory / "walker-later.png", "1280x175+0+260")
                    assert different(walker_a, walker_b) > 80, (
                        "walker was obscured by the illustrated dialogue panel")
                    word_a = crop(first, directory / "words-start.png", "1000x165+115+430")
                    word_b = crop(later, directory / "words-later.png", "1000x165+115+430")
                    assert different(word_a, word_b) > 20, "authored word effect did not animate"
                    for width, height in SIZES:
                        call("xdotool", "windowsize", "--sync",
                             win, str(width), str(height))
                        image(win, directory / f"{width}x{height}.png", width, height)
                    call("xdotool", "windowsize", "--sync", win, "1280", "720")
                    key("Return")
                    key("Return")
                    wait(log, "Vesper ambient actor bird: visible=false")
                    key("Return")
                    wait(log, "Vesper ambient actor bird: visible=true")
                    key("Return")
                    wait(log, "Vesper choice UI mounted 2 independent rows")
                    key("Return")
                    wait(log, 'Vesper dialogue panel: Some("banner")')
                    banner = image(win, directory / "banner.png")
                    assert different(first, banner) > 3000, "journal panel did not change"
                    key("F5")
                    wait(log, "Vesper save written")
                    key("Return")
                    key("F9")
                    wait(log, "Vesper save restored")
                    restored = image(win, directory / "banner-after-F9.png")
                    # Actor motion continues, but notebook/banner choice must
                    # persist and be visually different from the first page.
                    assert different(first, restored) > 3000, "F9 lost the banner panel"
                elif auto:
                    # No key presses: the reader's configured 0.8 s delays
                    # must progress through the plain second line and trigger
                    # the authored ambient-hide event in the third line.
                    # A later unstyled dialogue does not remount TextSpans.
                    wait(log, "Vesper ambient actor bird: visible=false", limit=12)
                    assert log.read_text(errors="replace").count(
                        "Vesper auto-advance ready") >= 2, (
                        "auto delay never armed for successive dialogue lines")
                    image(win, directory / "auto-advanced.png")
                else:
                    time.sleep(1.1)
                    second = image(win, directory / "still.png")
                    if reduced:
                        assert different(first, second) < 50, (
                            "reduced-motion composition changed")
                assert process.poll() is None, "integrated player crashed"
                print(f"QOL08 {name}: graphical integration passed", flush=True)
                return first
            finally:
                if process.poll() is None:
                    process.terminate()
                    process.wait(timeout=10)


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    normal = run_case("normal")
    # Real installed families are required for visual discrimination.
    # Generic aliases may resolve to the same fallback on headless runners.
    serif = run_case("serif", font="DejaVu Serif", reduced=True)
    mono = run_case("monospace", font="DejaVu Sans Mono", reduced=True)
    # Compare only the notebook dialogue region while both animations freeze.
    serif_words = crop(serif, OUT / "serif-body.png", "930x145+170+438")
    mono_words = crop(mono, OUT / "monospace-body.png", "930x145+170+438")
    assert different(serif_words, mono_words) > 150, (
        "installed player font preferences did not affect rendered text")
    # The font picker also advertises generic serif/monospace choices. Their
    # aliases must resolve to different installed fallback families.
    alias_serif = run_case("generic-serif", font="serif", reduced=True)
    alias_mono = run_case("generic-mono", font="monospace", reduced=True)
    generic_a = crop(alias_serif, OUT / "generic-serif-body.png", "930x145+170+438")
    generic_b = crop(alias_mono, OUT / "generic-mono-body.png", "930x145+170+438")
    assert different(generic_a, generic_b) > 150, (
        "generic font picker aliases resolved to identical fallback glyphs")
    contrast = run_case("high-contrast", contrast=True, reduced=True)
    assert different(serif, contrast) > 2000, (
        "high contrast failed to override decorative notebook styling")
    run_case("auto-delay", auto=True, reduced=True)
    print("QOL08: all-five combined example, fonts, auto delay, effects,"
          " actors, panels and accessibility PASS", flush=True)


if __name__ == "__main__":
    main()
