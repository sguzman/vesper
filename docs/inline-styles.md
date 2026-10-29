# QOL04 — Inline word styling in authored dialogue

This feature belongs to the external **story and theme files**. There is no text-style editor, runtime markup interpretation, per-frame file parsing, or engine-specific story fork.

## Author dialogue

In a quoted `.vn` dialogue line, surround a phrase with a named tag:

```text
@scene opening
"The [[fx=ember]]fire[[/fx]] illuminates a [[fx=shadow]]secret[[/fx]]."
"[[fx=ember]]Warm light[[/fx]] spreads over the hillside."
ella: "Buenas noches. [[fx=shadow]]おやすみ[[/fx]]."
```

Tags may be adjacent but **cannot nest**. Names start with ASCII letters or underscore, followed by letters, digits, underscores, hyphens or dots. `[[/fx]]` must close exactly one open tag; an empty tagged phrase, unmatched delimiters, or an unknown tag is an authoring error.

For a literal `[[` sequence, write `\\[[` (two backslashes) in the quoted story source. The quote decoder yields `\[[` (one backslash), and the inline parser then emits a literal `[[`. Tags are not accepted inside choices; these currently retain their existing native UI presentation.

## Name each style in the external theme

For example, `styles/words.toml`:

```toml
[dialogue]
size = 30.0
color = [0.96, 0.96, 0.96, 1.0]

[inline_styles.ember]
color = [1.0, 0.43, 0.12, 1.0]

[inline_styles.shadow]
color = [0.34, 0.64, 0.94, 1.0]
# Optional installed system font family:
# font_family = "serif"
```

Each color is normalized RGBA, with every component finite and in `0..=1`. A theme may define up to 32 named inline styles; unknown names in scripts fail `check-game` with the original filename and line number. The `font_family` name, if supplied, must be nonempty and at most 128 characters; installed availability depends on the machine. The reader's chosen font takes precedence over the authored style family.

## Data, compatibility and accessibility

- The parser runs during story loading. A decorated line has a separate plain-text string and a bounded set of named, nonoverlapping UTF-8 byte spans. **It does not store raw markup as reader-facing text.**
- All span boundaries must align to extended Unicode grapheme clusters; emoji ZWJ sequences, flags, combining marks and multilingual text cannot be cut in half by a tag. Maximum input is 32 KiB per line and at most 64 marked spans.
- Unmarked stories retain their original `Output::Dialogue` behavior. Tagged stories use a `RichDialogue` variant. The existing save format stays at version 1; prior saves still load and new rich-text saves persist their spans. The backlog and accessibility content use the original readable words, never markup.
- Bevy uses one text block containing native `TextSpan` segments. Styled characters use the same wrapping and grapheme-safe typewriter reveal as plain dialogue. Section boundaries are prepared on line changes instead of rescanning tags every frame.
- Reader high contrast overrides author colors with normal dialogue color, and the reader's selected font overrides optional author font families. Reduced motion does not require an exception for QOL04 because these are static styles. Animated fiery/wavy/dark effects are **future QOL05**, not part of this milestone.
- Reloading game text/themes remains handled by the existing worker-based reload path. Ill-formed styles or removed definitions are rejected before changing the active game; a compatible story edit refreshes the current line without replaying side effects.

## Run and verify

```sh
cargo run --locked -p vesper-cli -- check-game examples/inline-words
cargo test --locked -p vesper-story -p vesper-game
cargo run --locked -p vesper-player -- examples/inline-words
```

The CI Bevy regression `.github/scripts/inline_spans_smoke.py` takes real rendered screenshots at six viewport sizes (946×511, 955×520, 800×600, 900×700, 1280×720, 1920×1080), changes **only authored inline palette colors** between identical runs, verifies styled first words, confirms high-contrast override, and checks F5/F9 graphical restoration of an annotated dialogue. It uploads `vesper-inline-spans` for human review.

This is **not a commitment that every system font exists on every installation** or a replacement for physical Wayland/Hyprland QA; CI uses Xvfb software rendering.
