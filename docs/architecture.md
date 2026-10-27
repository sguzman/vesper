# Vesper — Architecture decisions

Baseline: 2026-10-27. License: MIT.

**Purpose.** A small reusable Rust/Bevy runtime for complete authored visual novels. Art and code can be produced with AI assistance, but narrative is authored by the game creator and no AI system is required at runtime.

**Firm decisions.** Pin Bevy to 0.20.0; use idiomatic ECS and built-in 2D, text, UI, audio, input and assets. No Lua. Rich typography and Unicode are first-class acceptance requirements. Prefer native Bevy facilities and free Linux-first workflows.

**Engine/data boundary.** Runtime logic belongs in Rust; game manifests, story `.vn` files, character definitions, theme files, fonts, images, audio and saves live outside it. They must be loadable by an unchanged executable, using explicit game-directory paths. Loading must be asynchronous where relevant; never parse or load large files on the render thread. Prevent directory traversal and preserve previous good content on hot-reload errors.

**BSN.** The `bsn!` macro is permitted for compiled, reusable, engine-owned presentation templates. Do not rely on external `.bsn` loading in Bevy 0.20. Runtime-authored UI themes and compositions require external data, probably TOML initially.

**Narrative behavior.** A headless interpreter is separate for deterministic verification. Bevy ECS is used idiomatically for presentation and input. Scene identifiers are stable; malformed stories give line-numbered diagnostics. Narrative execution must not depend on frame rate. No general-purpose user code execution.

**1.0 definition.** A complete externally authored reference VN runs in an unchanged executable with backgrounds, characters, fonts, dialogue, choices, variables, conditions, audio, persistence, safe dev hot reload, accessible UI, validation, tests and Linux packaging. No arbitrary other-genre functionality, visual editor or remote backend.

**Current state.** Headless parser and runtime have been authored but must pass CI. The Bevy player has external text dialogue, keyboard input and choices, but its build remains unverified and it does not yet render assets. See `roadmap.json` for verified state.
