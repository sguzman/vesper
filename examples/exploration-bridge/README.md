# Mixed-mode VN-to-exploration transition fixture

This external game starts **as a pure visual novel** with no
`[exploration.start]`. After its opening dialogue, the author explicitly
runs `explore street at home`; a later cinematic path uses
`explore cafe at entry`. The existing dialogue/choice interpreter and panel
art remain independent from the optional world renderer.

The external manifest shares deliberately small placeholder scenery with
`exploration-schema`. It is **not** the finished anime urban example.

```sh
cargo run --locked -p vesper-cli -- check-game examples/exploration-bridge
cargo run --locked -p vesper-player -- examples/exploration-bridge
```

All location IDs and spawn references are checked against the manifest,
with the offending .vn line reported before launch. Returning to pure
VN never requires movement; authored transfers are explicit.
