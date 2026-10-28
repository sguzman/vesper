"""Visual regression and live-edit verification for external fairytale palettes.

Capture screenshots under Xvfb; do not change the repository's authored art.
Contrast checks are computed against each authored panel background.
"""
from __future__ import annotations
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import time
import tomllib

ROOT = Path(__file__).resolve().parents[2]
OUT = Path("/tmp/vesper-theme-qa")
SHOWCASES = (
    ("little-lantern", "The Little Lantern", "styles/lantern.toml"),
    ("clockwork-moon", "The Clockwork Moon", "styles/moonlight.toml"),
)

def run(*args: str, check: bool = True):
    return subprocess.run(args, capture_output=True, text=True, check=check)

def luminance(rgb):
    def linear(c):
        return c / 12.92 if c <= .04045 else ((c + .055) / 1.055) ** 2.4
    r, g, b = (linear(v) for v in rgb[:3])
    return .2126 * r + .7152 * g + .0722 * b

def ratio(a, b):
    hi, lo = sorted((luminance(a), luminance(b)), reverse=True)
    return (hi + .05) / (lo + .05)

def pixels(a: Path, b: Path) -> int:
    import re
    cmp = run("compare", "-metric", "AE", str(a), str(b), "null:", check=False)
    match = re.match(r"^\d+", cmp.stderr.strip())
    assert match, cmp.stderr
    return int(match.group())

def capture(win: str, img: Path):
    deadline = time.monotonic() + 12
    while time.monotonic() < deadline:
        run("import", "-window", win, str(img))
        count = int(run("identify", "-format", "%k", str(img)).stdout)
        if count > 50:
            return
        time.sleep(.35)
    raise AssertionError(f"Rendered screenshot too flat: {img}")

def await_log(log: Path, phrase: str):
    deadline = time.monotonic() + 15
    while time.monotonic() < deadline:
        if phrase in log.read_text(errors="replace"):
            return
        time.sleep(.2)
    raise AssertionError(f"Missing {phrase}: {log.read_text(errors='replace')[-4000:]}")

def window(name: str):
    deadline = time.monotonic() + 30
    while time.monotonic() < deadline:
        found = run("xdotool", "search", "--name", name, check=False)
        if found.returncode == 0 and found.stdout.strip():
            return found.stdout.splitlines()[0]
        time.sleep(.3)
    raise AssertionError(f"Missing window {name}")

def main():
    OUT.mkdir(parents=True, exist_ok=True)
    backgrounds = []
    accents = []
    with tempfile.TemporaryDirectory(prefix="vesper-theme-") as temp:
        for slug, title, relative in SHOWCASES:
            source = ROOT / "examples" / slug
            game = Path(temp) / slug
            shutil.copytree(source, game)
            path = game / relative
            style = tomllib.loads(path.read_text())
            panel = style["panel"]
            backgrounds.append(panel["background"][:3])
            accents.append(panel["accent"][:3])
            for name in ("speaker", "dialogue", "choices", "hint"):
                contrast = ratio(panel["background"], style[name]["color"])
                assert contrast >= 4.5, f"{slug}: {name} contrast {contrast:.2f} below 4.5"
                print(f"{slug} {name} contrast {contrast:.2f}:1", flush=True)
            assert panel["accent"][:3] != panel["divider"][:3]

            log = OUT / f"{slug}.log"
            with log.open("w") as output:
                process = subprocess.Popen(
                    [str(ROOT / "target/debug/vesper-player"), str(game), "--watch"],
                    stdout=output, stderr=subprocess.STDOUT,
                    env=dict(os.environ, LIBGL_ALWAYS_SOFTWARE="1",
                             VESPER_SAVE_DIR=str(game / "saves")))
                try:
                    win = window(title)
                    run("xdotool", "windowfocus", "--sync", win)
                    await_log(log, "Vesper viewport resized: 1280x720")
                    time.sleep(2)
                    before = OUT / f"{slug}-original.png"
                    capture(win, before)
                    original = path.read_text()
                    # Only alter a presentation token; hot reload must update
                    # its actual rendered accent and panel color without restart.
                    import re
                    changed, replaced = re.subn(
                        r"(?m)^accent = \[[^\]]+\]$",
                        "accent = [1.0, 0.0, 0.8, 1.0]",
                        original,
                    )
                    assert replaced == 1, f"expected one accent token in {slug}"
                    assert changed != original, f"accent token missing in {slug}"
                    path.write_text(changed)
                    await_log(log, "Vesper hot reload applied")
                    time.sleep(1)
                    after = OUT / f"{slug}-reloaded.png"
                    capture(win, after)
                    diff = pixels(before, after)
                    assert diff > 500, f"{slug}: accent hot reload invisible: {diff} pixels"
                    print(f"{slug}: rendered accent hot reload changed {diff} pixels", flush=True)
                    assert process.poll() is None
                finally:
                    process.terminate()
                    process.wait(timeout=10)
    assert backgrounds[0] != backgrounds[1], "Showcases must have different panel palettes"
    assert accents[0] != accents[1], "Showcases must have different accent identities"

if __name__ == "__main__":
    main()
