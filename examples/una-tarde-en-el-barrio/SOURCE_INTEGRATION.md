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
playthrough. The third starts the actual game. It has now passed native Linux software-renderer Xvfb tests and all three ending routes, as verified in merged PRs #45–#47. This does **not** replace physical Wayland/RTX/audio/owner acceptance.

The CI-generated `una-tarde-linux-development-preview` artifact contains a compressed development game archive: `Una-Tarde-en-el-Barrio-Linux-x86_64.tar.gz`. Extract it and use `./play.sh` (plus `sha256sum -c SHA256SUMS`) from the extracted directory—no Cargo or repository checkout required for this preview. A debug-stripped copy of the native executable is packaged; the original development build is untouched. The archive is not a final game release.

## Git/source policy

Text stories, themes, project notes, character-art source, generated image
and sound files are intentional committed assets. Do not check in personal
save files, readout/preferences data, local parser logs, generated screenshot
captures, development ZIPs or machine-specific build outputs. The external
QA evidence remains separately archived in GitHub Actions. The newer v0.7.8
native CI playthroughs are documented in `QA_STATUS.md`; final physical
presentation/audio and owner acceptance remain pending.
