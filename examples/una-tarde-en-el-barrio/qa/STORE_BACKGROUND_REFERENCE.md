# Original shop continuity reference (art-production gate)

The reference for the proposed clean store is the **actual unchanged**
`assets/backgrounds/store-illustration-study.png` from *Una Tarde en el Barrio*,
not a newly imagined grocery shop or a UI mockup.

- Exact original Git blob: `26145dcd11106d47b712188e6558e882d7edf789`
- SHA-256: `fb52bfdf44e6a64027b5e46a04c823ac2cc1362adefb755796a4b606c9030ea7`
- Image: RGB PNG, **1280 × 720**. Captured byte-for-byte from the original
  package's CI development-preview archive and verified against the Git blob.
- Reference composition: blackboard with *Abarrotes San Marcos* at left,
  vintage register around x=80–250, curled white/brown sleeping cat around
  x=190–435 on the counter, blue-patterned white mug around x=450–530,
  notebook/ledger around x=540–615, dark-haired/bearded Mateo in a cream
  rolled-sleeve shirt and black apron across x=610–1040, glass candy jars
  around x=1060–1270, merchandise/shelf stock, wood/glass counter and
  warm terracotta tiled floor.

**Required change:** Remove only baked-in Mateo and the cat, convincingly
reconstructing the shelves/stock/counter/lighting formerly concealed by them.
Supply actual separate, provenance-preserved transparent RGBA character art
that matches their pose, markings, facial identity and rendering. Provide a
separate physically plausible transparent countertop strip/occlusion layer.
No character, shelf, sign, mug, ledger or candy-jar displacement just to
accommodate an easier generated output. All character/prop placement must
be reviewed against the game's existing interaction x coordinates.

## Machine-verifiable unchanged-landmark controls

`qa/check-store-background-continuity.sh ORIGINAL.png CANDIDATE.png` checks
that both images are 1280×720 and measures normalized ImageMagick 7 mean
absolute color difference across the following **unoccluded** rectangles:

| Landmark | Pixel crop (width × height + left + top) |
| --- | --- |
| Left chalkboard | `185x260+176+6` |
| Vintage register | `90x135+86+303` |
| Ceramic mug | `70x72+454+392` |
| Notebook | `55x15+540+450` |
| Candy jars | `174x160+1060+305` |
| Counter facade | `850x49+16+510` |
| Floor tiles | `1150x102+40+605` |

Each protected region allows at most **0.04 normalized MAE** to permit minor
color cleanup, while disallowing displaced fixture/composition geometry.
Test-only localized actor-area edits may pass this *continuity-only* gate:
that is **not proof** of genuinely painted clean scenery or alpha-separated
characters. `qa/test-store-background-continuity.sh` exercises unchanged
pixels and synthetic actor-area edits, and rejects wrong canvas size,
shifted store and erased mug landmarks. Test images are temporary and
never installed in the game.

When a real complete art candidate exists, the atomic transition gate
`qa/check-world-composition.sh` additionally requires this continuity
check **and** `qa/check-store-layer-art.sh` (source/alpha and deterministic
rebuild provenance) as well as all four manifest references migrating
together. Neither static check alone certifies artistic quality, actual
native Bevy occlusion, legibility or physical owner acceptance.

## Rejected generation attempts

On 2026-11-09, two exploratory image generations failed source-continuity
review: one replaced the original with a new shop layout (and was not
1280×720); another generated an unrelated project dashboard.
**Neither was committed, copied to runtime assets, or approved as art.**
Do not disguise their shortcomings with cropping or resampling.

See [issue #51](https://github.com/sguzman/vesper/issues/51) for the
required complete character/source/foreground/native acceptance bundle.
Formal roadmap credit remains unchanged until its explicit acceptance gate.
