# Repository integration — original game

This is a standalone **external** game under `examples/una-tarde-en-el-barrio/`.
The Vesper engine remains unchanged; the existing manifest and `.vn` narrative
are not compiled into Rust. Art sources, sound sources, generated sprites,
scenario checks and their reproducible transforms are kept together in Git.

## Requirements

Use Vesper `main` at or after the merged opt-in reader-hint change
`1752d51d346f86378fdb7b6eeca9dbcf4f79603b` (PR #31), which supports
`[reader_hints]` in the external theme. Install ImageMagick 7 and ffmpeg for
art-source and audio-source consistency checks. The compiled player and CLI
are supplied by the normal Vesper workspace build, not this game.

## Validate from the repository root

```sh
examples/una-tarde-en-el-barrio/qa/check-package.sh
./target/debug/vesper check-game examples/una-tarde-en-el-barrio
./target/debug/vesper-player examples/una-tarde-en-el-barrio
```

The first two are **source and parser checks**, not a native graphical
playthrough. The third starts the real game and still needs native QA.
Do not interpret Git import or passing static checks as owner art approval.

## Git/source policy

Text stories, themes, project notes, character-art source, generated image
and sound files are intentional committed assets. Do not check in personal
save files, readout/preferences data, local parser logs, generated screenshot
captures, development ZIPs or machine-specific build outputs. The external
QA evidence remains separately archived; v0.7.1 is the last game version with
actual Bevy playthrough evidence. Final acceptance remains pending.
