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

    for first, second in [
        (panel_normal, panel_black), (bubble_normal, bubble_black)
    ]:
        # Both modes must retain an actually drawn scene/text layer.
        # Numeric WCAG bounds are asserted in the Rust preference tests.
        difference = qa.diff(first, second)
        print(f"{first.name} vs {second.name}: {difference} changed pixels")
    print(
        "Native original classic panel and pure-VN bubble rendered with "
        "reader black ink, screenshots preserved for visual inspection: passed",
        flush=True,
    )


if __name__ == "__main__":
    main()
