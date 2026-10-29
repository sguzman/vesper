"""Regressions for body text approaching the separator or hint footer.

Run the real Lantern scene with identical geometry/art and variants hiding
dialogue, hints, choices or the separator. Difference masks isolate real glyph
bounds and the physical divider; assertions catch even near-misses. No OCR,
layout guesses or test-only UI code are needed in the engine.
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
PLAYER = ROOT / "target/debug/vesper-player"
SOURCE = ROOT / "examples/little-lantern"
OUT = Path("/tmp/vesper-hint-overlap-qa")
SIZES = ((946, 511), (955, 520), (800, 600), (900, 700),
         (1280, 720), (1920, 1080))


def run(*args: str, check: bool = True) -> subprocess.CompletedProcess[str]:
    return subprocess.run(args, capture_output=True, text=True, check=check)


def window() -> str:
    deadline = time.monotonic() + 25
    while time.monotonic() < deadline:
        result = run("xdotool", "search", "--name", "The Little Lantern", check=False)
        if result.returncode == 0 and result.stdout.strip():
            return result.stdout.splitlines()[0]
        time.sleep(.25)
    raise AssertionError("Little Lantern window did not appear")


def sample(game: Path, size: tuple[int, int], path: Path, choices: Path) -> None:
    log = path.with_suffix(".log")
    env = dict(os.environ, LIBGL_ALWAYS_SOFTWARE="1",
               VESPER_SAVE_DIR=str(game.parent / "saves"))
    with log.open("w") as stream:
        process = subprocess.Popen([str(PLAYER), str(game)], stdout=stream,
                                   stderr=subprocess.STDOUT, env=env)
        try:
            win = window()
            run("xdotool", "windowfocus", "--sync", win)
            # Reproduce the actual user's sequence: compositor resizes the
            # window at launch, THEN dialogue advances at the narrow aspect
            # ratio. Resizing after advancing can accidentally repair the
            # text measure, making the earlier regression a false negative.
            run("xdotool", "windowsize", "--sync", win, str(size[0]), str(size[1]))
            time.sleep(1.5)
            for key in ("Return", "Return", "Return"):
                run("xdotool", "key", "--clearmodifiers", key)
                time.sleep(.35)
            # Take *all four variant masks* from the same deterministic
            # initial selection and scroll offset. Previously the rapid eight
            # Down presses occasionally landed on different rows under slow
            # software rendering; comparing those offset screenshots created
            # fake glyph collisions (e.g. 1920x1080 on main).
            time.sleep(.65)
            run("import", "-window", win, str(choices))
            # Verify scrolling separately, without subtracting screenshots
            # whose selected row may differ across independent launches.
            for _ in range(8):
                run("xdotool", "key", "--clearmodifiers", "Down")
                time.sleep(.20)
            time.sleep(.75)
            selected_last = choices.with_name(
                f"{choices.stem}-selected-last.png"
            )
            run("import", "-window", win, str(selected_last))
            scroll_pixels = mask(
                choices, selected_last,
                choices.with_name(f"{choices.stem}-scroll-difference.png"),
            )
            assert scroll_pixels > 100, (
                f"{size}: navigating nine long choices did not move focus/scroll"
            )
            for key in ("1", "Return"):
                run("xdotool", "key", "--clearmodifiers", key)
                time.sleep(.35)
            time.sleep(1.1)
            run("import", "-window", win, str(path))
            dimensions = run("identify", "-format", "%w %h", str(path)).stdout
            assert dimensions == f"{size[0]} {size[1]}", dimensions
            errors = log.read_text(errors="replace")
            assert "panicked at" not in errors and "Story asset error" not in errors
            assert process.poll() is None
        finally:
            if process.poll() is None:
                process.terminate()
                process.wait(timeout=8)


def long_choices(game: Path) -> None:
    """Keep all authored scenes, but exercise nine long wrapped choices."""
    script = game / "scripts/story.vn"
    source = script.read_text()
    old = (
        '* "Hold the lantern while Pip opens the gate." -> hold_light\\n'
        '* "Walk beside Pip and watch the stars." -> walk_together'
    )
    # A literal newline separates the two existing authored choices.
    old = old.replace("\\n", "\n")
    assert old in source
    options = "\n".join(
        f'* "Option {i}: The lantern and the evening sky carry the old '
        f'promise across the quiet valley, even beyond the gate." -> '
        f'{"walk_together" if i == 9 else "hold_light"}'
        for i in range(1, 10)
    )
    script.write_text(source.replace(old, options))


def invisible_theme(game: Path, section: str) -> None:
    path = game / "styles/lantern.toml"
    source = path.read_text()
    block = re.search(rf"(?ms)^\[{re.escape(section)}\]\n.*?(?=^\[|\Z)", source)
    assert block, f"missing [{section}] in {path}"
    changed, count = re.subn(
        r"(?m)^color = \[[^\]]+\]$",
        "color = [1.0, 1.0, 1.0, 0.0]",
        block.group(),
    )
    assert count == 1, (section, count)
    path.write_text(source[:block.start()] + changed + source[block.end():])


def invisible_divider(game: Path) -> None:
    """Hide only the 1px rule without changing flex geometry."""
    path = game / "styles/lantern.toml"
    source = path.read_text()
    changed, count = re.subn(
        r"(?m)^(divider = \[[^\]\n]*,\s*)[^\],\n]+(\])$",
        r"\g<1>0.0\2",
        source,
    )
    assert count == 1, "missing [panel] divider color"
    path.write_text(changed)


def mask(base: Path, invisible: Path, output: Path) -> int:
    run("convert", str(base), str(invisible), "-compose", "difference",
        "-composite", "-colorspace", "gray", "-threshold", "12%", str(output))
    return int(round(float(run("convert", str(output),
        "-format", "%[fx:mean*w*h]", "info:").stdout)))


def check(size: tuple[int, int], base: Path, no_divider: Path) -> None:
    body = OUT / f"{size[0]}x{size[1]}-body-mask.png"
    hint = OUT / f"{size[0]}x{size[1]}-hint-mask.png"
    pixels_body = mask(base, OUT / f"{size[0]}x{size[1]}-no-body.png", body)
    pixels_hint = mask(base, OUT / f"{size[0]}x{size[1]}-no-hint.png", hint)
    assert pixels_body > 120, f"Body text not visible: {pixels_body}"
    assert pixels_hint > 35, f"Hint text not visible: {pixels_hint}"
    # Detect visual collision by vertical extents, not merely intersecting
    # white pixels: different letters can overlap without sharing a pixel.
    def extent(path: Path) -> tuple[int, int]:
        # %@ computes the non-background bounding box *in the original
        # image*. Running -trim first resets its page offset to (0, 0)
        # and silently invalidates the vertical-overlap measurement.
        bbox = run("convert", str(path), "-format", "%@", "info:").stdout
        match = re.fullmatch(r"(\d+)x(\d+)\+(\d+)\+(\d+)", bbox)
        assert match, (path, bbox)
        _, h, _, y = map(int, match.groups())
        return y, y + h
    top_hint, _ = extent(hint)
    _, bottom_body = extent(body)
    gap = top_hint - bottom_body
    assert gap >= 5, (
        f"{size}: dialogue ends at {bottom_body} but hint starts at "
        f"{top_hint} (gap {gap}); text is overlaid"
    )
    overlap = OUT / f"{size[0]}x{size[1]}-overlap.png"
    run("convert", str(body), str(hint), "-compose", "multiply",
        "-composite", str(overlap))
    shared = int(round(float(run("convert", str(overlap),
        "-format", "%[fx:mean*w*h]", "info:").stdout)))
    print(f"{size[0]}x{size[1]}: {pixels_body} body pixels, "
          f"{pixels_hint} hint pixels, {shared} overlap, {gap} px vertical gap", flush=True)
    assert shared <= 4, f"Wrapped text overlaps keyboard hint by {shared} pixels"

    # The previous glyph/hint test misses the user's actual failure: the
    # dialogue descenders can touch the horizontal rule while staying clear
    # of the hint letters below it.
    divider = OUT / f"{size[0]}x{size[1]}-divider-mask.png"
    divider_pixels = mask(base, no_divider, divider)
    assert divider_pixels > 100, f"{size}: separator wasn't rendered"
    divider_top, _ = extent(divider)
    divider_gap = divider_top - bottom_body
    assert divider_gap >= 8, (
        f"{size}: last dialogue glyph ends at {bottom_body}, "
        f"divider starts at {divider_top} (gap {divider_gap}px)"
    )
    print(f"{size[0]}x{size[1]}: dialogue/divider real-pixel gap "
          f"{divider_gap}px", flush=True)


def check_choices(size: tuple[int, int], images: dict[str, Path]) -> None:
    """Use isolated option/hint glyph bounds, not shared individual pixels."""
    glyph = OUT / f"{size[0]}x{size[1]}-choice-glyph-mask.png"
    hint = OUT / f"{size[0]}x{size[1]}-choice-hint-mask.png"
    option_pixels = mask(images["normal"], images["no-choices"], glyph)
    hint_pixels = mask(images["normal"], images["no-hint"], hint)
    assert option_pixels > 140, f"{size}: no real choice glyphs"
    assert hint_pixels > 35, f"{size}: no choice hint glyphs"

    def vertical_bounds(path: Path) -> tuple[int, int]:
        bbox = run("convert", str(path), "-format", "%@", "info:").stdout
        m = re.fullmatch(r"(\d+)x(\d+)\+(\d+)\+(\d+)", bbox)
        assert m, (path, bbox)
        _, height, _, y = map(int, m.groups())
        return y, y + height

    top_hint, _ = vertical_bounds(hint)
    _, bottom_choice = vertical_bounds(glyph)
    # If choice text leaks into the hint mask, fail with a precise diagnostic
    # rather than reporting a bogus overlap. The hint is a footer in this UI.
    assert top_hint >= int(size[1] * .55), (
        f"{size}: hint difference mask starts at y={top_hint}; "
        "captured variant states differ outside the footer"
    )
    gap = top_hint - bottom_choice
    assert gap >= 5, (
        f"{size}: option glyphs end at {bottom_choice}; "
        f"hint glyphs start at {top_hint} (gap {gap})"
    )
    print(f"{size[0]}x{size[1]}: option/hint actual glyph gap {gap}px, "
          f"{option_pixels} option pixels, {hint_pixels} hint pixels", flush=True)


def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="vesper-hint-qa-") as temp:
        for width, height in SIZES:
            size = width, height
            images = {}
            choice_images = {}
            for variant, hidden in (("normal", None), ("no-body", "dialogue"),
                                    ("no-hint", "hint"), ("no-choices", "choices"),
                                    ("no-divider", "divider")):
                folder = Path(temp) / f"{width}x{height}-{variant}"
                shutil.copytree(SOURCE, folder)
                long_choices(folder)
                if hidden == "divider":
                    invisible_divider(folder)
                elif hidden:
                    invisible_theme(folder, hidden)
                image = OUT / f"{width}x{height}-{variant}.png"
                choice = OUT / f"{width}x{height}-{variant}-choices.png"
                sample(folder, size, image, choice)
                images[variant] = image
                choice_images[variant] = choice
            check(size, images["normal"], images["no-divider"])
            check_choices(size, choice_images)


if __name__ == "__main__":
    main()
