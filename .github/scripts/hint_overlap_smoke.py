"""Regression for body text wrapping into the keyboard-hint footer.

Run the real Lantern scene at a compact viewport three times with identical
geometry/art: normal, dialogue invisible, and hint invisible. Difference masks
isolate body and footer glyphs. Their intersection must be empty: no OCR, no
guesswork about line positions, and no test-only UI code in the engine.
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
SIZES = ((946, 511), (955, 520), (800, 600), (900, 700))


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


def sample(game: Path, size: tuple[int, int], path: Path) -> None:
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
            for key in ("Return", "Return", "Return", "1", "Return"):
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


def mask(base: Path, invisible: Path, output: Path) -> int:
    run("convert", str(base), str(invisible), "-compose", "difference",
        "-composite", "-colorspace", "gray", "-threshold", "12%", str(output))
    return int(round(float(run("convert", str(output),
        "-format", "%[fx:mean*w*h]", "info:").stdout)))


def check(size: tuple[int, int], base: Path) -> None:
    body = OUT / f"{size[0]}x{size[1]}-body-mask.png"
    hint = OUT / f"{size[0]}x{size[1]}-hint-mask.png"
    pixels_body = mask(base, OUT / f"{size[0]}x{size[1]}-no-body.png", body)
    pixels_hint = mask(base, OUT / f"{size[0]}x{size[1]}-no-hint.png", hint)
    assert pixels_body > 120, f"Body text not visible: {pixels_body}"
    assert pixels_hint > 35, f"Hint text not visible: {pixels_hint}"
    # Detect visual collision by vertical extents, not merely intersecting
    # white pixels: different letters can overlap without sharing a pixel.
    def extent(path: Path) -> tuple[int, int]:
        bbox = run("convert", str(path), "-trim", "-format", "%@", "info:").stdout
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


def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="vesper-hint-qa-") as temp:
        for width, height in SIZES:
            size = width, height
            images = {}
            for variant, hidden in (("normal", None), ("no-body", "dialogue"),
                                    ("no-hint", "hint")):
                folder = Path(temp) / f"{width}x{height}-{variant}"
                shutil.copytree(SOURCE, folder)
                if hidden:
                    invisible_theme(folder, hidden)
                image = OUT / f"{width}x{height}-{variant}.png"
                sample(folder, size, image)
                images[variant] = image
            check(size, images["normal"])


if __name__ == "__main__":
    main()
