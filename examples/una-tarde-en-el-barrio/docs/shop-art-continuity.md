# Shop background continuity contract (work in progress)

**Scope:** repaint the existing `assets/backgrounds/store-illustration-study.png` in *Una Tarde en el Barrio*, not redesign or replace the store. This is an **art production reference and QA guard**, not a clean background or accepted new artwork. The source is an original game-owned 1280×720 RGB painting.

## Source painting, verified landmarks

The painting is a close, warm late-afternoon view across the counter of **Abarrotes San Marcos**. Preserve its perspective, brushwork and physical placements:

| Landmark | Approximate area in source (x/y pixels) | Requirement |
| --- | --- | --- |
| Tall blackboard/sign | x175–365, y0–308 | Keep the original Spanish sign, lettering, warm trim and leafy vine |
| Vintage register | x75–370, y255–460 | Do not move, restyle or cover the painted register |
| Curled calico cat | x188–437, y350–469 | Remove from clean background; later place as **independent source-authored transparent RGBA** on the counter, not the floor |
| Painted ceramic mug | x450–535, y386–469 | Retain blue motif, scale and spacing between cat and ledger |
| Notebook/ledger | x540–619, y450–471 | Keep beside the mug, aligned with gameplay x≈555 |
| Mateo | x600–1040, y0–475 | Remove painted body, arm and head from background and carefully reconstruct occluded shelves/counter |
| Tall candy jars | x1060–1270, y295–480 | Preserve colors, perspective and relation to counter edge |
| Counter and display case | x0–1280, y450–560 | Keep contact surfaces and scene perspective; author a separate transparent foreground-front mask later |
| Wood floor | x0–1280, y555–720 | Keep horizon and tile/wood texture alignment; no abruptly clipped floor or new architecture |

The interim manifest intentionally displays **invisible hitspots** for Mateo (`x=820`, radius 140) and cat (`x=302`, radius 85). Other shop hotspots include ledger (`x=555`) and display (`x=270`). These coordinates are gameplay contracts, not optional decorative suggestions. The current VN `store_interior` alias and exploration background must point to the same painted asset.

## Safeguard added by this QA change

`qa/check-store-background-continuity.sh ORIGINAL.png CANDIDATE.png` rejects malformed/differently sized backgrounds and compares seven **stable unoccluded** original-painting regions with ImageMagick 7 normalized mean absolute pixel error. It allows up to **4%** per landmark to tolerate light edits, but rejects wholesale replacement, shifts, reimagined layouts and large changes to anchored objects. `qa/test-store-background-continuity.sh` verifies the source and a disposable actor-region probe pass, while a globally shifted shop, a deleted mug, and a resized canvas fail. The tests do **not** modify actual game assets.

This safeguard applies **only to a proposed clean backdrop** during an atomic art migration; it cannot recognize whether characters were really removed, whether newly repainted shelves are plausible, or whether independent sprites actually match. A test probe that edits Mateo's region is **not** a replacement drawing or a passing artist approval.

Production migration also requires, simultaneously:
1. One coherent clean background with both painted figures removed and naturally repaired interior.
2. An independently authored Mateo RGBA source and a matching sleeping calico cat RGBA source, with editable provenance and deterministic exports; no square cutouts of shelf pixels.
3. A separate perspective-correct transparent counter-front mask as needed to occlude figures without covering the player.
4. Strict `qa/check-store-layer-art.sh` source-to-export, alpha, edge, transparency and sprite checks (currently **not present**).
5. Atomic `game.toml` changes for shop background, its VN alias and both independent actor images, with interaction x/radius unchanged. `qa/check-world-composition.sh` refuses partial migrations.
6. **Actual Bevy screenshots** at 946×511, 1280×720 and 1600×900, with foreground occlusion, speech bubbles and saves checked, followed by owner physical Hyprland/RTX/audio and creative approval.

The attempted standalone shop recreations during the 2026-11-09 review **rearranged the source**; a separate generated dashboard was unrelated to the game. Neither is an acceptable production asset, and none is wired into the manifest.

**Acceptance boundary:** Passing scenery continuity alone earns no art milestone or roadmap points. The playable v0.7.8 original game remains provisional and retains the old art until the entire art set is coherent and independently verified. See [issue #51](https://github.com/sguzman/vesper/issues/51) and [issue #30](https://github.com/sguzman/vesper/issues/30).
