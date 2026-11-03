# Vesper — Urban Anime Slice-of-Life Example (Creative Brief)

**Status:** **Provisional creative direction, not a finished game and not yet authorized canon.** This is a separate **external game directory** to be created when story specifics are decided: proposed `examples/urban-slice-of-life/`. The final title, city, characters, premise, duration and endings remain deliberately open for the owner to define.

## Core pitch

A **grounded, contemporary, anime-inspired urban slice-of-life narrative adventure**. Ordinary streets, small cafés and shops, transit stops, apartments and parks feel familiar and inhabited. Walking creates a sense of place; dialogue and small interpersonal decisions carry the story. The overall feeling is **casual, warm, emotionally observant and lived-in**, not high fantasy, superhero action or a system-heavy RPG.

**Reference principle:** `Afterlove EP` demonstrates a possible rhythm of short city walks, in-world dialogue bubbles and intimate narrative moments. Draw inspiration only from those **general mechanics**. All art, characters, streets, dialogue, soundtrack, story arcs and identity must be original, not copied.

## Visual identity to aim for

- Grounded anime illustration: stable architecture/perspective, believable shop fronts and street furniture, consistent character proportions and lighting, expressive yet restrained faces. Casual contemporary clothing, textured ordinary spaces rather than abstract symbolic backgrounds.
- A coherent art bible: character model/expression sheets, idle/walk turnaround and cycle sheets, perspective/camera guides, palette for morning/day/evening/night, signage/props and clear layout of authored walkable zones.
- Parallax or a few depth layers may enrich a location, but no uncontrolled camera warp or excessive overlays. Keep readability first; avoid the previously rejected translucent abstract menu decoration motif.
- Native readable UI with a designed speech-bubble style and optional **custom illustrated VN dialogue panel plus independent nameplate artwork**. Both can appear during the same conversation if the author chooses.
- Ambient sound can include gentle traffic, distant voices, footsteps, shop door bells and subtle music. Auditory cues should be pleasant and optional, not mandatory to understand a choice.
- All assets live inside the example directory; document provenance, licensing, generation/production workflow and reproducibility. No runtime art generation or paid service dependency.

## Player experience pillars

**1. Everyday movement, never busywork.** A handful of compact, distinct neighborhoods/interiors; approachable people; interaction prompts; location shortcuts for repeat visits. A trip should have emotional or observational value, not consume time merely to cross the map. No health bars, combat, jumping, platforming or grinding.

**2. Conversations in context.** Casual sidewalk/café interactions use character-anchored bubbles when appropriate. Important intimate conversations can temporarily use Vesper's full VN composition (characters, backdrops, speaker nameplate, reveal pacing). The author can switch the surface within one scene without changing narrative behavior.

**3. Small but meaningful decisions.** Remembering details, choosing what to say, attending or declining meetings, making time for friends and seeing changes in later conversations. Avoid shallow "collect X items" gates. Conditions and persistent relationships derive from authored deterministic story variables.

**4. A real, complete narrative.** Not just a showcase of buttons, art or engine systems. Deliver a beginning, escalating interpersonal stakes, a satisfying middle and ending(s), consistent character voices and meaningful story branches. The final length and depth are **TBD**, not an arbitrary 10-minute tech demo.

## Proposed environment/cast scaffold — all adjustable

**Places (illustrative, not fixed):** apartment/home, neighborhood street, café/shop, bus/train stop, and evening park or small public plaza. These suggest **4–6 distinct zones**, some potentially single-screen. Add only the zones that the eventual narrative actually uses; don't fabricate locations simply to hit a count.

**Cast (illustrative, not fixed):** a player character with expressible emotions, a few recurring friends/neighbors/coworkers, plus background pedestrians. Roughly **4–6 recurring characters** is a starting production estimate rather than a mandated quantity. Choose actual ages, names, city and social context with the owner.

**Story seed (illustrative, not canon):** an ordinary stretch of days in which the protagonist's daily routes intersect with friends' changing plans and neighborhood life. A small unresolved interpersonal matter connects otherwise casual conversations. Stakes are human and particular; there is no default supernatural secret, disaster countdown or obligatory romance.

**Creative decisions reserved for later:** final title, whether setting is fictional or inspired by a real city, protagonist's life stage/relationships, plot/central conflict, relationship mechanics, any dating content, final scope/duration, art technique, soundtrack and language(s).

## Showcase flow (demonstration of engine capabilities, not imposed plot)

1. Begin with an **ordinary authored VN scene** that establishes a character and tone using the usual illustrated panel and custom speaker name art.
2. Transition into a compact **walkable neighborhood** with restrained avatar movement; optional pedestrians add life without blocking progress.
3. Approach and speak to an NPC. The first exchange appears as a **speech bubble anchored above the talking character**; the bubble's tail tracks an animated speaker.
4. For a more intimate/important beat, switch to a **custom VN dialogue frame** and staged portraits; retain the same dialogue and choice logic.
5. Make a real choice that changes a later line/NPC availability. Return to the neighborhood at the **same authored position**, then reach another location via a convenient, brief transition.
6. Demonstrate a **bubble exchange in a purely VN scene** (no movement); the same game can demonstrate both modes without conflating them.
7. Save, exit and load in exploration and mid-conversation; verify stable positions, settings, choices and speaker presentation, then complete an actual story ending.

## First prototypes and delivery criteria

**Prototype A (UI):** existing standalone VN switches panel → bubble → panel around a staged speaker. Prove text wrapping, nameplates and choice focus. No exploration requirement.

**Prototype B (world):** one short side-on street, controllable avatar, NPC, bubble dialogue, custom-panel dialogue, explicit return to position and a single meaningful branch. No filler activity.

**Complete example:** the original artistic and written game is fully playable, authored outside the engine, demonstrates both modes organically, has its own README, controls, credits/source provenance and a full end-to-end CI/Wayland acceptance pass. Unlike a disposable test fixture, its scenes must have coherent pacing, complete character arcs and satisfying endings.

## Acceptance with the owner

A scene is acceptable when it looks like a believable **casual anime urban space**, movement feels direct rather than obstructive, NPC talks can be initiated without fuss, bubbles and portrait panels both look polished, no story character flashes through menu transitions, and important dialogue remains accessible at compact and fullscreen sizes. **Visual approval is required; automated tests alone do not certify aesthetic quality.**

This brief is intentionally editable. The owner's forthcoming story specifics supersede provisional scaffolding without changing the generic engine requirements in the [roadmap](narrative-adventure-roadmap.md).
