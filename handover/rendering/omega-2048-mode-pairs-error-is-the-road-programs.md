---
status: ready-to-wire
titles: [omega]
updated: 2026-11-16
summary: "2048-mode pairs: road programs lead; crowd Shadowed law ready to wire; panelmap1 road wired (march, sheen, emissive), lobe colour left"
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
2. **`road_alphaspec_normal_emissive_panelmap1` (and `_mask`): wired except the lobe's colour** (2026-11-16,
   `panelmap-wire`; [panel-displacement.md](../../docs/ghidra/functions/ps4-omega-eu/panel-displacement.md),
   "Implemented where", has the numbers). March, sky-cube sheen and emissive draw; road at 16x: Sol `LF` 32.7 to
   18.0, `bias` -32.4 to +16.1, `G` 0.35 to 0.75; Queens Mall 30.7 to 14.5, -16.9 to +8.5, 0.71 to 0.85; Altima and
   Unity Square within 1.1 grey and 0.03 `G`. Left:
   - ~~**b. The sun lobe's colour.**~~ **Wired** (`panelmap-sun`, 2026-11-16): `envsettings_light` fills
     `Light::sun_specular`, carried through the zone grade; the lobe is live (a x100 probe moves 85,305 Sol pixels)
     but draws almost nothing at the pairs' poses (Sol 339 pixels, largest step 7 grey), so no road number moves.
   - **Sol's near road overshoots** grey-blue (near patch (48, 35, 36) % original, (50, 41, 45) % ours) and the road
     `bias` is +13.7 on main. **Ruled out** by probe (`panel-displacement.md`, "Measured"): the fresnel's normal and
     the cube's blur level move the patch by under 2 %; the cube's orientation is settled for Sol (the glare sits at
     the authored sun only with the flip) and unflipping it costs Queens Mall's road `LF` 14.6 to 19.7. Ours' sheen is
     about 1.7 times the original's and bluer: next, find a missing weight on the sheen (the program's lightmap or
     shadow terms) or what the sheen is added to; read the program's sky-add instructions again.
   - **The march reshapes Sol's strips** (3.3 % of the frame moves by more than 8 levels) but does not raise road
     `G` (no-march probe 0.77 against 0.75): the `G` rise is the sheen's. Compare the strips' lit far wall by eye
     on a pose that looks down at the road.
   - **Queens Mall's cyan lines** in the pair sit on the edge of a deep trench, not on the panel-map road; ours draws
     none. Find the material there (`psp2_material_dump` at that world point) before adding anything.
   - Unity Square's road `G` 0.83 to 0.80; the shadow-light factor `0.75 + 0.25 ...` and the lip blend are not drawn.
   - 2048: **checked, differs** (no panel map on Vita). HD-mode siblings (`*PanelMap*` on Vineta K,
     `panelmap2` on Sol 2) are unread: a separate RE item if their pairs show the same gap.
3. ~~**`tarmac_*` and brick sharpness is filtering.**~~ **Settled** (`port-aniso`, `docs/formats/omega-status.md`, "The port's
   anisotropy"): the game's own S# asks `AnisoLinear` x16 on 5 to 8 of its 16 to 19 sampler states, the port honours it
   (forced level 0, no extra LOD bias), and stripping it softens the road by about 40 % of its Laplacian deviation. The
   `omega` compare profile now renders ours at `anisotropy = "16x"`, and `compare-rerender-ours.py` applies it to race pairs recorded with `off`; road `G` on the five tarmac/brick pairs goes 0.68-0.84
   to 0.82-1.06 with `LF` unchanged. **Left:** 2048 (Vita) not checked, decisive check: the GXM texture init's filter arguments on a road draw and Vita3K's honouring of them; re-render the Omega pairs (`just compare-rerender-ours --only omega`, the
   lead's step; verified on a copy); a per-sampler law and an Omega `Title` default of 16x are not wired (ours applies one level to the whole
   model); which S# each road material binds was not mapped (`+0x20` state word `0x522800`).
4. **Sky:** `Lighting.Sky brightness` unread; 1.07 registrar writes it at `+0x160` of `FUN_015eda20`'s object;
   find the reader.
5. **Capital Reach's pose** differs from the original's (about 480 px at 1920 on the Auricom banner): match the grid
   slot or heading before judging any render change on it.
6. **Unread keys seen on the way:** `Lighting.Ship diffuse brightness` (+0x164), `Sun specular colour`/`intensity`
   (RGB), the lens-flare keys (Empire Climb's sun glare), `Reflection.Spread *`.

## Next Steps

1. Wire item 1 (an afternoon if `output_scale` can carry it per draw; a day if it needs a slot bit).
2. Item 2's lobe colour (15 minutes in `raceplay/src/load/environment/light.rs`, then a Sol measurement), then
   Sol's near-road overshoot (an afternoon: a second sunny circuit's pair to settle the cube orientation).
