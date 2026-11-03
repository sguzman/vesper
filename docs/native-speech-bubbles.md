# NA02 — Optional speech-bubble presentation (in progress)

This optional native reader style belongs to Vesper's visual-novel experience.
It does **not** enable walking, exploration, physics, or scripted AI. The same
`.vn` interpreter and existing text widgets are used for either presentation.

## External game manifest

Omit the entire `[dialogue]` table to keep the existing classic dialogue
box and any custom image/nameplate. To select speech bubbles for authored
dialogue in a pure VN, use:

```toml
[dialogue]
default_surface = "bubble" # optional: panel (default) or bubble

[dialogue.bubble]
max_width_percent = 58.0
padding = 16.0
corner_radius = 18.0
background = [0.035, 0.065, 0.12, 0.98]
border_color = [0.86, 0.76, 0.57, 0.96]
# Optional artist-owned nine-slice PNG frame. Omit for native rounded background.
# image = "assets/ui/speech-bubble.png"
# slice_border = 16.0
```

- `default_surface` supports only `panel` and `bubble`; the default is the
  existing illustrated VN panel. The global default is independent of any
  future optional exploration mode.
- **Additive `.vn` commands:** `surface bubble`, `surface panel`,
  `surface default`. Authors may put one at the beginning of a scene or
  between dialogue lines. The command takes effect on the next line without
  changing story variables, movement, saved choice or custom panel artwork.
  `default` clears the active override and restores the manifest default.
  These presentation commands are typed and validated with file/line
  diagnostics, not interpreted as arbitrary scripts.
- `max_width_percent` is finite and between 30 and 80. `padding` is 8–48,
  `corner_radius` is 2–45, and `slice_border` is 2–80 logical/source pixels.
  RGBA fields contain finite values from 0 to 1. Unknown manifest fields fail
  validation rather than being silently ignored.
- Custom `image` must be a validated game-relative PNG. No remote paths or
  assets outside the game directory; Bevy's asynchronous loader owns decoding.
  A solid native fallback remains underneath the image while loading.
- The same authored theme supplies dialogue and speaker fonts, inline styling,
  reveal speed and player-selected font size. The existing player high-contrast
  override suppresses bubble art and uses legible black/white backing.
- If a speaking actor is visible and staged, the engine positions the bubble
  near that actor's head with a small native tail. Narrator/non-staged dialogue
  uses a readable centered fallback with no false tail.
- On a choice screen or the ending, the existing panel / engine-owned choice
  UI is used. This keeps focus, multiple choice targets and save/history
  behavior without introducing new game-specific controls.
- UI positioning is computed from logical Bevy screen coordinates, accounting
  for `UiScale`. Rects and anchor positions stay inside the compact viewport.
  The original `StoryText` entities and `ReadingState` are reused rather
  than re-parsing dialogue or creating another text-reveal pipeline.

## Testing

```sh
cargo test --locked -p vesper-game -p vesper-story
cargo test --locked -p vesper-player --bin vesper-player
cargo run --locked -p vesper-cli -- check-game examples/bubble-showcase
cargo run --locked -p vesper-player -- examples/bubble-showcase
```

The independent `examples/bubble-showcase/` fixture reuses the tiny Little
Lantern story and art, with a different game ID and only the new surface
configuration. This protects the original example as a compatibility control.
GitHub CI launches both **real** Bevy games under a virtual X11 display, checks
distinct bubble/legacy pixel output at 1280×720, renders a 946×511 compact
bubble, advances to a staged character and confirms the unchanged choice-panel
fallback. It also uses F5/F9 to verify that a **per-line panel override survives
a real quick-save/load and returns to subsequent bubble dialogue**. A separate
high-contrast/reduced-motion/large-type launch checks that reader preferences
change the authored bubble safely. Screenshots are retained as the
`vesper-speech-bubbles` artifact.

**Still pending acceptance:** integrated saved override and graphical
round-trip checks for per-line commands (NA03), exploration NPC anchors
(NA04–06), fully authored art for the new slice-of-life title (UR02), and
physical Wayland visual acceptance. This partial implementation is not
automatic completion of NA02 or NA03.
