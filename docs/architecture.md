# Vesper — Architecture decisions

Baseline: 2026-10-27. License: MIT.

**Purpose.** A small reusable Rust/Bevy runtime for complete authored visual novels. Art and code can be produced with AI assistance, but narrative is authored by the game creator and no AI system is required at runtime.

**Firm decisions.** Pin Bevy to 0.20.0; use idiomatic ECS and built-in 2D, text, UI, audio, input and assets. No Lua. Rich typography and Unicode are first-class acceptance requirements. Prefer native Bevy facilities and free Linux-first workflows.

**Engine/data boundary.** Runtime logic belongs in Rust; game manifests, story `.vn` files, character definitions, theme files, fonts, images, audio and saves live outside it. They must be loadable by an unchanged executable, using explicit game-directory paths. Loading must be asynchronous where relevant; never parse or load large files on the render thread. Prevent directory traversal and preserve previous good content on hot-reload errors.

**BSN.** The `bsn!` macro is permitted for compiled, reusable, engine-owned presentation templates. Do not rely on external `.bsn` loading in Bevy 0.20. Runtime-authored UI themes and compositions require external data, probably TOML initially.

**Narrative behavior.** A headless interpreter is separate for deterministic verification. Bevy ECS is used idiomatically for presentation and input. Scene identifiers are stable; malformed stories give line-numbered diagnostics. Narrative execution must not depend on frame rate. No general-purpose user code execution.

**1.0 definition.** A complete externally authored reference VN runs in an unchanged executable with backgrounds, characters, fonts, dialogue, choices, variables, conditions, audio, persistence, safe dev hot reload, accessible UI, validation, tests and Linux packaging. No arbitrary other-genre functionality, visual editor or remote backend.

**Current state.** Headless parser and runtime have been authored but must pass CI. The Bevy player loads external game manifests, visual commands, text dialogue and keyboard choices. Linux build and initial startup smoke pass. Keyboard choice interactions and sample artwork pixels passed X11 screenshot verification (CI run 40558460155). External TTF/system font styling, Unicode screenshot rendering and UI scaling passed CI 40563141018. Versioned F5/F9 save/load with worker-thread I/O and visual restoration passed CI 40563153534. Native audio asset decoding, timed fade transitions, safely rejected invalid hot reload edits, and Linux packaging have passed CI. Inline mixed-style text markup and advanced variable-font/OpenType authoring controls are not provided by this minimal MVP. See `roadmap.json` for verified state.

2026-10-27: P03 verified on Ubuntu virtual display (CI 40565981048): PCM audio decoded through Bevy's asset server, native playback entities queued, sprite fade timed and completed, arrow/Enter/digit controls tested. Runner has no actual sound device, so audible output still needs separate hardware QA.

**Verified developer iteration and packaging.** CI 40566533674 exercises safe script/theme hot reload, rejection of invalid source and external image reloading. CI 40566657220 verifies a relocatable Linux archive and compiler-free execution. Full integrated regression and both branches of the reference VN passed CI run 40571573177.

**Scoped MVP completion (2026-10-27): 100/100 roadmap points verified.** Source commit `e9a9a9c` passed GitHub Actions run 40571573177. This is completion of the deliberately bounded Vesper runtime objective, not a claim that every conceivable visual novel authoring feature exists. Hardware speaker-output testing remains a documented limitation of headless CI. Future features require a separate, explicitly authorized scope rather than invented continuation tasks.
