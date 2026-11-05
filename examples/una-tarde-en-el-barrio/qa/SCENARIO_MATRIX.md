# v0.7.5 authoring scenario matrix — SOURCE-LEVEL ONLY

`./qa/check-scenarios.sh` executes the actual `scripts/story.vn` using a
purpose-built, strictly limited `awk` model of scene/choice/boolean-condition
syntax. It does not instantiate the Vesper engine, render Bevy, test audio
output, or serialize/reload a real save. Do not report these as native runs.

Each case begins with the real `intro` flag initialization, performs the
listed authored NPC/prop/noticeboard conversations in order, chooses one
available option for each encounter, and asserts both the lines that must be
present and the lines that must be absent. Scene mode, world return and ending
completion must match the selected path. Repeated conversations retain state.

The checked-in `qa/scenarios.tsv` covers all three ending IDs and both
noticeboard direct/postponed decisions, including:

- Resident responses with and without prior Elena/Rosa/Mateo conversations.
- Planning with Mateo's consent and without it.
- Reading a private ledger versus respectfully leaving it closed.
- Remembering the old door, petting or ignoring the cat, inspecting or
  leaving the display.
- Revisiting Mateo after reading his ledger and revisiting the noticeboard.
- All alternate dialogue choices and returns, not just the happy path.

The checker validates **31/31 authored scenes, 22/22 selected choices and
24/24 true/false outcomes of conditional lines**. It deliberately corrupts
`set saw_ledger = true` and inverts an Elena condition in temporary source
copies, and requires the scenario suite to reject both changes.

This is regression coverage of current *authored source semantics*, not proof
that Bevy's interpreter and save/load matches this small independent model.
The acceptance gate still requires real native playthroughs after every
material source/art change, plus human visual approval.
