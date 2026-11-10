# NA08 — native Hyprland/Wayland acceptance record

**Status:** **NOT EXECUTED / NOT APPROVED**. This is a reproducible owner/hardware
acceptance checklist, **not** a report of a completed physical test. NA01–NA07
are verified; NA08 earns **0/6 points** until its remaining native presentation
and interaction checks have real evidence and owner approval.

## Already independently verified in Linux CI

| Evidence | GitHub Actions run | What the automated result supports |
| --- | --- | --- |
| Modal keyboard ownership, world movement and 946×511 ↔ 1600×900 resize | [41754053770](https://github.com/sguzman/vesper/actions/runs/41754053770) | Real Bevy player under virtual X11; F6 save overlay, A/D, E and window-resize screenshots |
| Reader preferences in exploration conversation | [41756212101](https://github.com/sguzman/vesper/actions/runs/41756212101) | Native rendered NPC bubbles at 946×511 with high contrast, 160% text, reduced motion and version-two saves |
| Optional author-provided direct interaction | [41761627922](https://github.com/sguzman/vesper/actions/runs/41761627922) | Distant NPC reached by T; E remains proximity-based; modal save/Q cancellation and exact return position |
| Simultaneous E/T cues and independent interactions | [41765634848](https://github.com/sguzman/vesper/actions/runs/41765634848) | Distinct nearby and shortcut conversations, genuine world screenshots, safe saves, full Bevy and narrative regressions |
| Baseline narrative/panel and mixed-mode regressions | [41753473604](https://github.com/sguzman/vesper/actions/runs/41753473604) | NA05–NA07 acceptance on the external Atlas, Dawn and exploration fixtures |

These jobs use **Xvfb/X11 software rendering**, not native Hyprland,
physical GPU/monitor, fractional scaling, real audio devices, or human comfort.
They cannot substitute for the following gate.

## Reproduce on a real Wayland desktop

Use an optimized release build made with the development machine's normal `mbx`
build coordinator. The CI development-preview archive uses a stripped debug-profile
binary, so it is not a replacement for the native release build in performance QA. Run the **same player executable**
with external game directories. For a workspace build from the repository root:

```sh
target/release/vesper-player examples/exploration-schema
target/release/vesper-player examples/bubble-showcase
target/release/vesper-player examples/atlas-unwritten-skies
```

The first fixture starts in the authored walk-first street. The second is a
**pure VN** bubble example and must not require an avatar or walking. The third
is an unchanged traditional VN regression. If a binary is not available, build
using the local machine's established coordinator rather than creating a
separate compiler/target workflow for this checklist.

For the optional **T shortcut**, test a **copy** of the exploration fixture.
Add `quick_interact = "neighbor"` directly under
`[exploration.locations.street]` in the copied `game.toml`, and launch that
copied game directory. Do not change the repository example or pretend the
unmodified fixture declares a shortcut. The copy must retain its own assets
and `.vn` script.

With the app on the physical compositor, record the actual window size,
monitor scale and GPU in the session record. For relevant Hyprland sessions,
`hyprctl monitors -j` and `hyprctl clients -j` can help identify display
scale and the player's actual window dimensions. Never apply X11 `xdotool`
automation as proof of Wayland behavior.

## Physical acceptance cases (record each as PASS / FAIL / NOT RUN)

| Case | Steps / expected observation | Result |
| --- | --- | --- |
| H1 — Walking and focus | Exploration street: walk A/D and arrows; releasing the key stops; E shows the nearest eligible NPC/exit and never activates faraway targets | NOT RUN |
| H2 — Opt-in convenience | On copied shortcut fixture, T opens neighbor from the spawn without walking; near friend, E and T are both visible and open their *different* scenes | NOT RUN |
| H3 — No accidental movement | While P/M pause/menu, F6 slots, H history, or appearance settings are active, movement/interaction does not leak into the world; closing restores controls | NOT RUN |
| H4 — Saves and mode return | F5/F9 in world and during conversation, F6 slots, Q cancel, authored `explore return`, and street ↔ café exit maintain expected location/X/facing | NOT RUN |
| H5 — Window layouts | At approximately 946×511, 1280×720, 1600×900, and fullscreen, body text, bubble border/tail, choices, prompt, and player sprite remain legible, with no overlap or cut-off | NOT RUN |
| H6 — Reader preferences | In an actual bubble conversation use high contrast, 160% text, reduced motion; confirm readable content, prompt/choice focus cues and usable scrolling at compact size | NOT RUN |
| H7 — Pure VN isolation | `examples/bubble-showcase` starts without world controls; `examples/atlas-unwritten-skies` keeps its usual panels, menu, keyboard/pointer choices and saves | NOT RUN |
| H8 — Device/compositor behavior | Test actual Wayland focus changes, window ↔ fullscreen, physical GPU rendering and audio output; note stutters or driver/compositor issues without inventing FPS | NOT RUN |

Where a sequence fails, record the exact key/action, window size, visible symptom,
repeatability and a reference to its screenshot or local log. Save files and
preferences are reader-owned; avoid uploading personal data. Screenshots, if
recorded, must come from the actual Wayland session (a compositor-supported
capture method is fine), not from the Xvfb artifacts.

## Session evidence and approval (fill in only after running)

- Date / local time: **NOT RECORDED**
- Tested `main` commit SHA: **NOT RECORDED**
- Linux distribution / desktop / compositor version: **NOT RECORDED**
- GPU / renderer / driver: **NOT RECORDED**
- Monitor resolution, scale and refresh: **NOT RECORDED**
- Window sizes actually exercised: **NOT RECORDED**
- Actual screenshot/log artifact references: **NONE**
- Issues found and reproduction steps: **NONE RECORDED**
- H1–H8 completed: **NO**
- Owner visual/interaction approval: **PENDING**
- NA08 final 6/6 points awarded: **NO**

Only replace this template with actual observed outcomes and source evidence.
Do not award NA08 points for a checklist, green X11 CI, or an unobserved
hardware configuration.
