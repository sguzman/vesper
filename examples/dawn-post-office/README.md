# The Post Office at the Edge of Dawn

**An original, self-contained Vesper visual novel.** A tiny moth courier named Nima is assigned a letter addressed to a morning that has not happened yet. Its journey crosses a suspended post office, floating sky garden, teacup observatory, cloud neighborhood, bell tower roof and a gate at the edge of the sky.

The story is a cozy, gently mysterious 12–20-minute read. Each full playthrough has approximately 2,375 words before interface and choices (roughly 15–17 minutes at a normal reading pace). Three decisions with three choices each produce **27 complete paths** and **three distinct endings**: homecoming, discovery, and kindness. Earlier decisions alter variables and later lines.

## Run

From the Vesper source tree after compiling the shared player:

~~~sh
cargo run --locked -p vesper-cli -- check-game examples/dawn-post-office
cargo run --locked -p vesper-player -- examples/dawn-post-office
~~~

The game is entirely external; the same executable runs the existing examples. The Linux bundle's dawn launcher opens it without Rust installed. Appearance Settings are available from P or M; F5/F9 quick-save/load, F6 manual slots, H backlog, K skips active staging, and choices use Up/Down, Enter or 1–9. Player preferences override the game-authored theme; reduced motion removes optional animations. Editing preferences does not change game files.

## Original production assets and provenance

All included illustrations and sound cues are original, generated expressly for this project without borrowed game art or recordings. Reproducible generator source is tools/generate_assets.py; it runs **only during production**, not during gameplay. Pillow is needed to regenerate the source art, never to play the finished bundle. The game ships with its rendered files.

- Seven illustrated background PNGs: 960×540, lavender/apricot cut-paper storybook world.
- Nine transparent character PNGs: 320×500 RGBA, three stable expressions for Nima (hopeful/curious/brave), Orin (wry/worried/warm), Kettle (sleepy/patient/sparkle).
- Seven original mono 16-kHz 16-bit PCM WAV files: four musical loops and three effects, synthesized using mathematical oscillators rather than recordings.
- Typography: installed DejaVu Sans with engine font fallback; no font copies redistributed.

To regenerate media for production:

~~~sh
python3 -m venv /tmp/dawn-art-env
/tmp/dawn-art-env/bin/pip install Pillow==12.0.0
/tmp/dawn-art-env/bin/python examples/dawn-post-office/tools/generate_assets.py
~~~

Generated bitmaps and WAVs are included in version control and copied into the Linux package. All file paths are game-relative. Runtime is entirely offline with no image-generation or paid APIs.

## Verification

The Rust game integration test enumerates all 27 paths, checks reachable scenes, loaded expressions and sounds, and distinct endings. The production generator checks canonical image dimensions, transparent characters, and original PCM format. Actual Bevy screenshots and story playthroughs remain the acceptance authority; Xvfb audio decoding does not prove physical speaker output.

The three expression images per character deliberately share body geometry and canvas dimensions, so crossfades do not cause portrait jumps. game.toml owns actor staging; styles/dawn.toml owns typography, transitions, atmosphere and feedback; scripts/story.vn owns branching. There are no Rust branches specific to this game.
