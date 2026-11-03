# Vesper engineering contract

This project is stewarded toward a **verified, shippable Rust/Bevy visual novel runtime**. The owner retains final authority. Repository state, tests and architecture decisions are primary evidence. A request to **"continue"** means autonomously select, implement, test, commit and push the highest-priority in-scope work, not merely discuss a plan.

## Architectural commitments

- Rust stable, Bevy **0.20.x**, idiomatic ECS, native rendering, UI, typography, input, assets and audio when appropriate.
- Custom Ink-inspired .vn narrative language in Rust. No Lua, general-purpose embedded scripting or runtime generative AI.
- Reusable executable. All game-specific scripts, character definitions, manifests, themes, artwork, audio and fonts are external runtime files. The same binary must run distinct games.
- BSN may describe engine-owned UI; standalone external .bsn loading is not available in Bevy 0.20 and must not be a game-data prerequisite.
- Rich typography, keyboard accessibility, error diagnostics and secure game-relative asset paths are core requirements.
- Do not perform heavy parsing, loading or unbounded story execution on the Bevy update/render thread.
- Prefer free, Linux-first tools. No unnecessary paid services, npm, Mac-only tooling or third-party plugins. Protect credentials and private content.

## Scope and work protocol

The weighted, evidence-backed acceptance ledger is [docs/roadmap.json](docs/roadmap.json); design decisions are in [docs/architecture.md](docs/architecture.md).

1. On every round inspect the current branch, CI, tests, ledger and relevant history; do not guess.
2. Independently choose the highest-value actionable work inside the agreed objective. Implement through available tools, minimize user chores.
3. Test and repair. Never report an untested feature as verified.
4. Commit and push verified changes when permitted, update task statuses and stable weighted progress.
5. Report an overall progress bar, prior percentage, delta (in percentage points), milestone, newly verified tasks, tests, commit, blockers and next task. Report **+0 pp** if no task satisfies its acceptance criteria.
6. "Continue" never just means planning. Work through more than one sensible task when possible. Do not claim to work between user messages.
7. If the objective has genuinely reached 100%, stop. Never invent feature creep, random cleanup or speculative other-genre systems.
8. Keep a small, durable repo ledger rather than a large parallel project-management system.

Do not ask the owner to download generated YAML/scripts/specifications or manually execute work supported by available tools. Request participation only when a genuine access or hardware boundary makes automation impossible.

## New authorized roadmap (planning only)

The owner approved **planning** a separate optional narrative-adventure phase:
[docs/narrative-adventure-roadmap.md](docs/narrative-adventure-roadmap.md) and the
[original urban slice-of-life example brief](docs/urban-slice-of-life-brief.md).
Its independent 0/100 baseline is [docs/narrative-adventure-roadmap.json](docs/narrative-adventure-roadmap.json).
Do not change the completed MVP/QoL/visual/Atlas acceptance weights.
The new scope is **not implemented** by adding its plan; begin work when the
owner explicitly authorizes starting the phase (or continuing it after start).
Speech bubbles and exploration must remain independent, optional systems;
pure VNs and existing saves remain compatible. No mandatory travel filler or
new genre-wide physics/scripting scope.

## Additional stewardship controls for the authorized expansion

- The newly authorized narrative-adventure roadmap is **independent** of the already verified Vesper VN milestones. Reuse their evidence; do not restart the old scopes or reclassify completed work simply because this is a new phase.
- Adopt the applicable owner-supplied existing-Rust-project protocol (D V5 / A V3). Calibrate the full 100-point lifecycle against actual integration, save migrations, UI QA, example production, packaging and release work; do not hide hard work in the last ten points. Rebaseline transparently if truly necessary.
- Keep source modules cohesive by responsibility across the codebase; actively refactor mixed-purpose god files when relevant to authorized work. **No hard or soft source-line limits.**
- Maintain an approachable README, detailed architecture/authoring guides under `docs/`, and a separate chronological changelog if the project uses one. Do not turn the README into an internal worklog or publicize private reasoning/context.
- Apply the relevant provisional Nomos profile requirements by actual preconditions, not as permission for unrelated changes, new project registration or rebranding. Preserve existing Rust/Bevy toolchain, MIT license, project identity, dependencies and Linux/Wayland assumptions unless a justified approved decision changes them. Never invoke npm.
- For new work distinguish **planned, written, compiled, CI tested, visually verified and accepted**. Only credit weighted roadmap points after every associated acceptance condition is supported by passing evidence.
- Report prior/current/Δ percentages beside visual bars for the *new* active scope and only the milestones touched in the session; include concrete CI state and actionable next step. Do not repeatedly announce previously completed scopes.
- End a substantial development report with **User action:** either specific genuinely required approvals/QA or **None required**. Never delegate available repository work or imply mandatory owner action when no blocker exists.

## 100% definition

A complete externally authored reference visual novel runs in an unchanged compiled executable with dialogue, branching, variables, backgrounds, character expressions, fonts, themes, audio, saves, safe development hot reload and Linux packaging. All relevant checks pass, release-blocking defects are resolved, and the final state is committed and recoverable. Do not treat compilation or a starting window as equivalent to end-to-end release readiness.
