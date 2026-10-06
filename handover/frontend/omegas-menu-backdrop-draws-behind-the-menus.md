---
categories: [frontend, rendering]
---

# Omega's menu backdrop draws behind the menus, the campaign stills and the live campaign stage; the end-of-race screens, HD's own HD style, and two unread passes are open

2026-09-30, `omega-menu-backdrop`. Evidence and every number:
[menu-backdrop-scene.md](../../docs/ghidra/functions/ps3-hdfury-eu/menu-backdrop-scene.md)
(nine names, `names.tsv`), and the Omega half in
[omega-status.md](../../docs/formats/omega-status.md)'s menu backdrop section.

**Shipped.** `oag_ui::scene_backdrop` (widget parse, the eased working copy, the 60 s clock),
`Picture::Scene`/`Draw::SceneBackdrop`, `oag_game::render::scene` (scene into a white target,
`FEBackgroundAnim_fp`'s Roberts cross, blur, copy), `oag_game::boot::{backdrop,scene}` (one
`MenuBackdrop` slot holding the Fury clouds **or** the scene, since the two widgets are never up
together), `oag_mesh::mesh::rcs::psp2::build_with_vex` (Omega's geometry is a PS4 `.rcsmodel`
whose motion is in the `.vex`), and `BootProfile::menu_scene`, true for Omega alone. HD, Pulse, Pure
and 2048 `--menu-page main` captures are byte-identical before and after (double captures, plus the
Fury `--anim-seconds 4` one). Disc-backed:
`crates/game/tests/omega_menu_backdrop_ground_truth.rs`.

## Open

- **Not validated against the original.** No PS4 emulator exists and HD's only capture of its HD style
  shows a flat white page. The look is what the recovered program computes; nothing was compared.
- ~~**The live campaign stage** draws no backdrop in a window~~ - it does, and already did:
  `menu_stage.rs` passes `shown` (the movie, else `styled.picture`) to every campaign list builder and
  ticks `styled` each frame. Walked windowed under Xvfb + lavapipe, 2026-10-02 (`omega-talon-crash`):
  `Main Menu`, `Grid Selection` and `Cell Selection` all draw the scene
  (`data/scratch/omega-talon-crash/walk_1.png` to `walk_3.png`), so no code was needed; this line was stale.
  Only the **end-of-race screens** are still open (lane `pulse-end-photo` held that code, and nothing was
  changed there). Omega authors no track or ship picker, so there is nothing to draw there.
- **The boot screens have none in the original**: they are outside `Top FE Screen`. Not a gap.
- **Bands** (`use_bands`, always false), **`GroundPlane`** (named `_VR`), both not drawn.
- **Chosen, not measured**: the blur kernel (`0x003e3e50` unread), linear sampling of the scene target,
  the root page taking `Main Menu` and every other `default`.
- **HD's own HD style** has the same widget; `menu_scene: true` on `oag_hd` would draw it, unverified, and
  changes HD's bytes.
- ~~**The menu rows are white on the white page**~~ - fixed 2026-09-30 with chosen grey boxes, then
  superseded 2026-10-06 (`omega-frontend`): Omega's rows now draw on HD's List blocks (art rows
  reversed, see `omega-status.md`), and the box fallback was removed.
- The scene target's size and sampler state; Omega's own `AnimLength` reader (no PS4 program open).

## Next Steps

1. Draw the backdrop under the end-of-race screens (hand `Picture` to their list builders; `Live::tick`/`picture` are the two calls). The live campaign stage already does.
2. Read `0x003e3e50`'s kernel and replace the chosen blur.
3. Try a live RPCS3 capture of HD's HD style past its first seconds; if it draws this, flip `oag_hd`'s `menu_scene`.
