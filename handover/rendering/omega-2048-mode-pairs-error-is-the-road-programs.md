---
status: ready-to-wire
titles: [omega]
updated: 2026-11-16
summary: "2048-mode pairs: no single cause; road programs lead (3 families); crowd Shadowed law read, ready to wire; panelmap1 displacement unread"
difficulty: medium (40)
---

# Omega's WipEout 2048 mode pairs: the error is the road programs, split three ways

2026-11-16, `omega-2048-holistic`. Decomposition, table and evidence:
[`omega-status.md`](../../docs/formats/omega-status.md), "Where the WipEout 2048 mode pairs' error is".
Measure with `scripts/compare-decompose.py`, never the raw mean grey alone (it is dominated by decorrelation;
a correct detail fix can raise it).

## Open

1. **Crowd sprites draw white: ready to wire** (law read, confidence 70). `crowd_sprite_diffuse_normal_alpha` and
   `_shadow` author `Shadowed` `0.12178` (`0x36997860`) and `Ambient` `0.777` (`0x735324ab`); the sun-lit program
   is `Shadowed * T * (Ambient + constantAmbientColour + sun * sat(N.L))`. Recipe:
   - a new module beside `crates/mesh/src/mesh/rcs/psp2/` (`psp2.rs` is at the 1,500-line ceiling) admitting
     materials whose PS4 programs declare `Shadowed` and `Ambient` (`oag_rcs::gcn` reflection), Omega only;
   - the factor: ours' `colour_factor` path (`in.colour.rgb * (ambient + sun)`) is gated to hulls
     (`receives_shadow != 2`), so a vertex-colour `Shadowed` does nothing on track draws (tried); look first for
     an existing per-draw lit-output scale (`Light::output_scale`, `lit_output_halved`) rather than a new shader flag;
   - `Ambient` is an added term inside the bracket; name it open if not drawn;
   - acceptance: the crowd-region bias (Sol +112, Queens Mall +57, Subway +68, Arena +64) moves toward 0, read by eye
     on Sol's stands; a test that fails when the factor is dropped.
   - 2048: check whether its crowd materials author `Shadowed` (the Vita model path used by
     `psp2_material_dump` was not found in this lane).
2. **`road_alphaspec_normal_emissive_panelmap1`: the displaced panels.** Biggest single family (Queens Mall, Sol,
   Altima, Unity Square; Sol's road 38.5 dark, gradient ratio 0.24). Read its 428-instruction pixel program
   (`cargo run -p oag-rcs --example gcn_dump -- <data0N.psarc> data/environments2048/shared/objectlibrary/materials/road_alphaspec_normal_emissive_panelmap1.rcsmaterial`),
   in particular the `Cg_PanelDisplacement_*` use, the panel map (`track_a_floor_panel_df`, `R16F`, not decoded by
   `ModelTexture::from_gnf_form`) and the `EmissiveMap`. RE first.
3. **`tarmac_*` and brick sharpness is filtering.** Anisotropy 16x closes their gradient gap with no low-frequency
   change. Read the port's anisotropy setting and, if it is on, record it in the `omega` compare profile (and decide
   whether the pairs should be rendered with it); not a render law.
4. **Sky:** `Lighting.Sky brightness` unread; 1.07 registrar writes it at `+0x160` of `FUN_015eda20`'s object;
   find the reader.
5. **Capital Reach's pose** differs from the original's (about 480 px at 1920 on the Auricom banner): match the grid
   slot or heading before judging any render change on it.
6. **Unread keys seen on the way:** `Lighting.Ship diffuse brightness` (+0x164), `Sun specular colour`/`intensity`
   (RGB), the lens-flare keys (Empire Climb's sun glare), `Reflection.Spread *`.

## Next Steps

1. Wire item 1 (an afternoon if `output_scale` can carry it per draw; a day if it needs a slot bit).
2. RE item 2 in a fresh `oag-re` lane, Queens Mall and Sol as the frames.
