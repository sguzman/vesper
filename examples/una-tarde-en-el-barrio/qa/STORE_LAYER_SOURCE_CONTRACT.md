# Shop art export contract (not finished production artwork)

**Status:** production artwork has not been delivered or accepted. This document specifies reproducible source and export checks for a later *atomic* transition away from the original flattened `store-illustration-study.png` painting. It does not authorize using the synthetic QA drawings in the playable game.

## Source files and deterministic exports

| Artist-owned source, to be provided later | Game export, to be created later |
| --- | --- |
| `sources/backgrounds/store-clean.png` (opaque RGB 1280×720) | `assets/backgrounds/store-clean.png` |
| `sources/characters/mateo-isolated.png` (transparent RGBA) | `assets/npcs/mateo.png` |
| `sources/characters/shop-cat-isolated.png` (transparent RGBA) | `assets/props/shop-cat.png` |
| `sources/layers/shop-counter-front.png` (transparent RGBA, aligned to 1280×720) | `assets/layers/shop-counter-front.png` |

These paths are a proposed **canonical set** for the art delivery. The files do not exist yet. They must be authored coherently from the same painted shop, not cropped opaque rectangles or loosely matched newly imagined scenery. Source lineage, artist/licensing/creation details and edit decisions must accompany the completed artwork in `sources/SHOP_ART_PROVENANCE.md` (also not yet present). Do not add a provenance record containing invented assertions.

`tools/rebuild-store-layer-art.sh` converts each provided source to a deterministic game PNG using ImageMagick 7. It deliberately does **not** generate artwork, remove backgrounds, invent transparent silhouettes, rescale bodies, or alter the source. It will fail while the four files are missing.

## Required atomic manifest changes (once all four real assets exist)

The original game's world background and VN `store_interior` alias must both change to `assets/backgrounds/store-clean.png`. The Mateo NPC image becomes `assets/npcs/mateo.png`, and the cat prop becomes `assets/props/shop-cat.png`. Add `[exploration.locations.store.layers.counterfront]` with `image = "assets/layers/shop-counter-front.png"` and `plane = "foreground"`. Author sprite elevation and scale where needed using the generic engine fields from PR #58. Do not move existing gameplay hotspots (Mateo x820 radius140, cat x302 radius85, ledger x555 radius70, display x270 radius80, exit x88) merely to accommodate art.

Keep the original background and invisible-hitspot images until *all* of these are complete. `qa/check-world-composition.sh` already requires an atomic change. In the production state it will invoke `qa/check-store-layer-art.sh` as well as the existing background-continuity gate.

## What the source validator establishes

`qa/check-store-layer-art.sh` verifies the four source/export pairs and required manifest references; deterministic decoded RGB(A) pixel equality after a clean shadow rebuild; actual PNG geometry, alpha coverage and transparent sprite margins; a sparse foreground layer that does not obscure the upper room; provenance document presence and the subjects it must discuss; continuity of seven intact original scene landmarks; and significant alterations in both original painted actor regions (to reject an untouched source mislabeled 'clean').

`qa/test-store-layer-art.sh` synthesizes **one intentionally crude positive fixture and six negative fixtures in a disposable directory**, covering stale export pixels, opaque cat cutouts, untouched old background, wrong plane, a mask covering the room/heads, and a missing editable source. CI runs this without adding synthetic drawings to the game.

**Not proved:** removal of every baked-in figure pixel, plausible repainting of missing shelves/counter, faithful Mateo and cat identities, contact shadows, clean edges under actual compositing, art direction, or reliable audio/hardware. A fake rounded solid-color sprite can pass source/alpha checks. The visually unacceptable synthetic fixture passing is a test of file mechanics, not aesthetic acceptance.

## Remaining acceptance evidence

Inspect actual native Bevy frames at 946×511, 1280×720 and 1600×900 (both shop entrance and Mateo/cat conversations), preserving face-safe dialogue and counter occlusion. Re-run all three original endings, ledger privacy, F5/F9, street/shop return, and original game asset/source checks. Obtain owner physical Hyprland/RTX 3090 appearance and audio signoff. See [issue #51](https://github.com/sguzman/vesper/issues/51) and [issue #30](https://github.com/sguzman/vesper/issues/30).

Formal project milestone remains **54/100 (engine 54/60, original game 0/40 accepted)** until those separately defined gates are met.
