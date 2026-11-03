# Vesper exploration authoring validation fixture

**Status:** Schema-only. The player does not yet move through these locations.
This small external package proves that an optional [exploration] section can
declare two bounded locations, spawns, an NPC, a prop and cross-location exits
with validated story scene links. It deliberately reuses existing tiny source
art instead of representing itself as a production urban anime example.

```sh
cargo run --locked -p vesper-cli -- check-game examples/exploration-schema
```

All original VN-only examples remain valid with no [exploration] section. The
separate [roadmap](../../docs/narrative-adventure-roadmap.md) tracks the future
walk controller, interaction-to-dialogue events, animations and save migration.
