---
status: ready-to-wire
titles: [omega]
updated: 2026-11-16
summary: "2048-mode pairs: road programs lead; crowd _shadow wired, plain crowd sprites still white; panelmap1 wired, lobe colour left"
difficulty: medium (40)
---

# Omega's WipEout 2048 mode pairs: the error is the road programs, split three ways

2026-11-16, `omega-2048-holistic`. Decomposition, table and evidence:
[`omega-status.md`](../../docs/formats/omega-status.md), "Where the WipEout 2048 mode pairs' error is".
Measure with `scripts/compare-decompose.py`, never the raw mean grey alone (it is dominated by decorrelation;
a correct detail fix can raise it).

## Open

1. **The plain crowd sprites still draw white** (the `_shadow` materials are wired, branch `omega-crowd-shadowed`:
   `mesh::rcs::psp2::crowd`, glow-table program 16; Sol +117.6 to +103.3, Queens Mall +53.4 to +40.6, Arena +62 to +15,
   method and the `_shadow` sprites' own +160 to +58 in `omega-status.md`, "The crowd's law"). `crowd_sprite_diffuse_normal_alpha`
   and `crowd_diffuse_normal` author no `Shadowed` and their programs declare none, yet they are most of the crowd's
   pixels (Sol 46,650) and the original draws them dark and coloured. Giving the plain sprite the `_shadow` constants
   (a stand-in, not shipped) takes Sol to +39.7, Queens Mall to +6.4, Subway to -13.2, so the missing term is of that
   size. Find it: the `constantAmbientColour` and `liveLighting0diffuse` the original binds for these draws (a per-draw
   or per-material light set, the `Lighting.*` keys), or the normal-map `N.L` of a sprite whose tangent frame is built
   in the vertex program (`0x3bc4`). `Ambient` (the term inside the bracket) is drawn for the `_shadow` materials.
   2048: **checked, differs** (Vita draws crowds through `fc06_lambert_alpha`/`fc08_effects_crowd`, no `Shadowed`).
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

1. Item 1: read what lights the plain crowd sprite (half a day, RE: one `Shdr` is read, the inputs are not).
2. Item 2's lobe colour (15 minutes in `raceplay/src/load/environment/light.rs`, then a Sol measurement), then
   Sol's near-road overshoot (an afternoon: a second sunny circuit's pair to settle the cube orientation).
