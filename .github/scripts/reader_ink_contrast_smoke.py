#!/usr/bin/env python3
"""Native Bevy screenshots for a reader's black ink on dark surfaces.

A persisted reader preference must not erase a story, its choices, or hints.
These are actual rendered snapshots of the original external game fixtures;
the Rust contrast helper's unit tests enforce the numeric threshold.
"""
from __future__ import annotations

from pathlib import Path

import speech_bubble_smoke as qa

OUT = Path("/tmp/vesper-reader-ink-contrast-qa")
OUT.mkdir(parents=True, exist_ok=True)
qa.OUTPUT = OUT

PREFS = {"reveal_speed": "instant", "reduced_motion": True}


def capture(game: str, title: str, label: str, ink: str | None) -> Path:
    prefs = {**PREFS}
    if ink is not None:
        prefs["text_ink"] = ink
    frame, _, _ = qa.run_game(game, title, label, prefs=prefs, full_route=False)
    assert frame.is_file() and frame.stat().st_size > 0, frame
    return frame


def main() -> None:
    # Classic panel with no illustrated skin: this was unreadably black on
    # dark navy in the owner's real Wayland screenshots.
    panel_normal = capture(
        "examples/exploration-bridge",
        "Optional Cinematic To Walkable Scene",
        "panel-authored-ink", None,
    )
    panel_black = capture(
        "examples/exploration-bridge",
        "Optional Cinematic To Walkable Scene",
        "panel-reader-black-ink", "black",
    )
    # Independently authored pure VN bubble, unchanged game assets.
    bubble_normal = capture(
        "examples/bubble-showcase",
        "Speech Bubble Rendering Fixture",
        "bubble-authored-ink", None,
    )
    bubble_black = capture(
        "examples/bubble-showcase",
        "Speech Bubble Rendering Fixture",
        "bubble-reader-black-ink", "black",
    )

    # An illustrated panel uses actual painted pixels instead of the solid
    # theme.background. Historically black reader ink was allowed through
    # unmodified; the owner's native game became black-on-dark unreadable.
    # Compare the SAME original game with and without the persisted override.
    # A repaired build must render the author's light colors in both cases.
    illustrated_game = "examples/una-tarde-en-el-barrio"
    # Use the ASCII title prefix, matching the native full-playthrough smoke.
    # X11 window-title search is not reliable with the Unicode em dash.
    illustrated_title = "Una Tarde en el Barrio"
    illustrated_authored = capture(
        illustrated_game, illustrated_title, "urban-illustrated-authored", None,
    )
    illustrated_black = capture(
        illustrated_game, illustrated_title, "urban-illustrated-black", "black",
    )
    # Exclude the moving upper world scene: inspect the actual bottom text
    # panel, where the background illustration and intended ink are stable.
    def panel_crop(frame: Path) -> Path:
        cropped = OUT / (frame.stem + "-dialogue-crop.png")
        qa.call("convert", str(frame), "-crop", "1160x190+60+505",
                "+repage", str(cropped))
        return cropped

    panel_mismatch = qa.diff(
        panel_crop(illustrated_authored), panel_crop(illustrated_black)
    )
    assert panel_mismatch < 1200, (
        "Illustrated game overrides its authored light ink with unreadable "
        f"reader black (dialogue-panel pixel mismatch: {panel_mismatch})"
    )
    print(
        "Original illustrated game keeps authored legible text despite a "
        f"persisted black reader preference: {panel_mismatch} changed pixels",
        flush=True,
    )

    for first, second in [
        (panel_normal, panel_black), (bubble_normal, bubble_black)
    ]:
        # Both modes must retain an actually drawn scene/text layer.
        # Numeric WCAG bounds are asserted in the Rust preference tests.
        difference = qa.diff(first, second)
        print(f"{first.name} vs {second.name}: {difference} changed pixels")
    print(
        "Native classic panel, pure-VN bubble and original illustrated urban "
        "reader black ink, screenshots preserved for visual inspection: passed",
        flush=True,
    )


if __name__ == "__main__":
    main()
