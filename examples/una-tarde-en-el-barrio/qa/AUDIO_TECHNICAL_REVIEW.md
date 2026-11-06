# Technical audio check — v0.7.3

`./qa/check-audio.sh` decodes every manifest audio cue with `ffprobe` and
`ffmpeg`, checks PCM format, reasonable length and peak headroom, and checks
that the edited paper rustle begins at the zero sample. The unmodified
original source is kept in `sources/audio/paper-rustle-raw.wav`; the exact
12 ms fade is rebuilt with `tools/rebuild-paper-rustle.sh`.

All four tracks are mono PCM 16-bit. Golden-hour and shop-hush ambiences are
24-second generated loops at 22050 Hz; the two one-shots are at 44100 Hz.
The technical check does **not** prove a pleasant physical mix, sample-loop
seams in Bevy's runtime, speaker output levels, or hardware playback.
Real listening on the user's hardware remains unverified.
