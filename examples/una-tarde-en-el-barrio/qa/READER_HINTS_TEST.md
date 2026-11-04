# Spanish reader hints — v0.7.3

The new `[reader_hints]` external theme fields require the opt-in generic
Vesper reader implementation in PR #31 (`feat/reader-hint-localization`),
not the v0.7.2 main-branch player. The game does not modify engine code.

The authored fields cover pure VN narration, choices, endings and modal
world conversations/choices, including Q cancellation. The original English
fallback remains mandatory for all games without the section.

`check-reader-hints.sh` verifies authoring presence and field naming only;
the PR's Rust unit tests verify parser bounds and rendering selection.
Actual native Bevy screenshots for these new labels, compact layout and
full hardware acceptance have **not** been run yet.
