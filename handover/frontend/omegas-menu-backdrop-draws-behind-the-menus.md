---
categories: [frontend, rendering]
---

# Omega's menu backdrop draws behind the menus and the campaign stills; the live campaign stage, the end-of-race screens, HD's own HD style, and two unread passes are open

2026-09-30, `omega-menu-backdrop`. Evidence and every number:
[menu-backdrop-scene.md](../../docs/ghidra/functions/ps3-hdfury-eu/menu-backdrop-scene.md)
(nine names, `names.tsv`), and the Omega half in
[omega-status.md](../../docs/formats/omega-status.md)'s menu backdrop section.

**Shipped.** `oag_ui::scene_backdrop` (widget parse, the eased working copy, the 60 s clock),
`Picture::Scene`/`Draw::SceneBackdrop`, `oag_game::render::scene` (scene into a white target,
`FEBackgroundAnim_fp`'s Roberts cross, blur, copy), `oag_game::boot::{backdrop,scene}` (one
`MenuBackdrop` slot holding the Fury clouds **or** the scene, since the two widgets are never up
together), `oag_render::mesh::rcs::psp2::build_with_vex` (Omega's geometry is a PS4 `.rcsmodel`
whose motion is in the `.vex`), and `BootProfile::menu_scene`, true for Omega alone. HD, Pulse, Pure
and 2048 `--menu-page main` captures are byte-identical before and after (double captures, plus the
Fury `--anim-seconds 4` one). Disc-backed:
`crates/game/tests/omega_menu_backdrop_ground_truth.rs`.

## Open

- **Not validated against the original.** No PS4 emulator exists and HD's only capture of its HD style
  shows a flat white page. The look is what the recovered program computes; nothing was compared.
- **The live campaign stage** (`main/campaign_stage.rs`, which the campaign-launch lane owns) and the
  **end-of-race screens** sit under the same widget in the skin and draw no backdrop in a window. The
  `--menu-page grid-select`/`cell-select` stills do draw it, because `capture/campaign_page.rs` already
  took the page's `Picture`. Omega authors no track or ship picker, so there is nothing to draw there.
- **The boot screens have none in the original**: they are outside `Top FE Screen`. Not a gap.
- **Bands** (`use_bands`, always false), **`GroundPlane`** (named `_VR`), both not drawn.
- **Chosen, not measured**: the blur kernel (`0x003e3e50` unread), linear sampling of the scene target,
  the root page taking `Main Menu` and every other `default`.
- **HD's own HD style** has the same widget; `menu_scene: true` on `oag_hd` would draw it, unverified, and
  changes HD's bytes.
- ~~**The menu rows are white on the white page**~~ - fixed 2026-09-30 (`omega-frontend-fixes`):
  where row text cannot be read on the page the frame clears to, each row and value sits on a box
  in the frame's authored `HD_Grey` (`HD_Blue` selected), see `omega-status.md`. The box size and the
  lightness threshold are chosen, not measured; Omega's real row blocks are code in a PS4 executable
  nobody has read, and HD's block numbers do not reproduce on Omega's own `file2.gtf`.
- The scene target's size and sampler state; Omega's own `AnimLength` reader (no PS4 program open).

## Next Steps

1. Draw the backdrop under the live campaign stage and the end-of-race screens (hand `Picture` to their list builders; `Live::tick`/`picture` are the two calls).
2. Read `0x003e3e50`'s kernel and replace the chosen blur.
3. Try a live RPCS3 capture of HD's HD style past its first seconds; if it draws this, flip `oag_hd`'s `menu_scene`.
