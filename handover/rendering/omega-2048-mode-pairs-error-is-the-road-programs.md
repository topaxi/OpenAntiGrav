---
status: ready-to-wire
titles: [omega]
updated: 2026-11-16
summary: "2048-mode pairs: road programs lead; plain crowd sprites white; panelmap1's sheen overshoot is ours' gate lane (red, not alpha): recipe in item 2"
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
   - **Sol's near road overshoot: cause found, ready to wire** (2026-11-16, `sheen-re`; live knockouts on the native
     port, `panel-displacement.md`, "Live"). The sheen and the sun lobe are gated by the diffuse map's **red** lane in
     ours and by its **alpha** in the program (`0xe144` `@0xde0-0xde4`, confidence 94): `shade.wesl`'s `texel.a` is
     `coverage[(slots >> 3) & 3]` and a PS4 lightmapped material's slot word is `SECOND_IS_LIGHTMAP` alone, so the lane is
     0. `track_a_floor_ds_disp`'s alpha has mean 0.195, its red about 0.34 linear: ours' sheen was 1.4 to 2.3 times the
     original's in seven boxes and is x0.8 to x1.2 with the alpha lane; the cube, its flip, the level law and the fresnel
     are confirmed by the same run (a mirror-road state agrees within x0.8 to x1.13). **Recipe** (15 minutes plus the
     gate):
     1. `crates/mesh/shaders/shade.wesl`, the `if panel` block that builds `specular` and `sky_reflection`: replace
        `let gloss = texel.a;` with `let gloss = first.a;` (`first` is the diffuse sample at the marched coordinate, in
        scope there). Change nothing else: do **not** give the PS4 slot word an alpha lane globally in this change
        (`mesh::rcs::psp2`, `role`), which would move every lightmapped Omega and 2048 material that reads `texel.a`.
     2. `crates/mesh/tests/omega_panel_map_lit_path.rs`: add a pixel check that fails with the red lane, a diffuse
        texel whose red and alpha differ (red 0.8, alpha 0.2) and a sheen that follows the alpha.
     3. Acceptance, `scripts/compare-decompose.py --pair data/reference/compare-page/omega/<id> --road road_alphaspec`
        (16x): `sol-2048` road `LF` 17.0 to about 8.8, `bias` +13.8 to about -3.0, `G` 0.74 to about 0.79;
        `queens-mall-2048` road `bias` +8.6 to about -0.6 with `LF` 14.7 to 15.5 and `G` 0.85 to 0.83 (measured on a
        scratch build; Queens Mall's `LF` does not fall because its base is darker than the original's, next bullet).
        Altima and Unity Square: re-measure, expect the road `bias` to fall (not measured here). HD and 2048 frames
        must not move (`hd-race/talons-pack`, `2048-vita/anulpha-pass`).
   - **Left after that, both the base under the sheen:** (a) the program's shadow-light factor
     `Fsl = 0.75 + 0.25 sat(Nw.SL) + SLSD.y sat(R.SL)^SLSD.x` on the prelit term is not drawn; live `SLSD` is
     `(10, 1, 0, 0)` on both circuits and `SL` `(0.791, -0.375, -0.484)` on Sol, `(-0.307, 0.445, -0.841)` on Queens Mall,
     and the original's sheen-off base is 1.2 to 2.3 times ours in linear red in Sol's left and far boxes; (b) Queens
     Mall's sheen-off road is 10 to 35 % brighter in the original (live `novaScaleBiasPower` `(2, 0, 2.5)`; Sol's is
     `(1, 0, 1)`): compare with what ours binds there first. `scripts/wipeout-pc-road-sheen-live.py --pre
     ReflectivityAmount=0` gives the original's base at the pair's pose (about 4 minutes a boot).
   - **Wider, not measured:** every PS4 and Vita material whose slot word carries no alpha lane reads the first
     texture's **red** as `texel.a` (`mesh::rcs::psp2` builds `role` as 0 or `SECOND_IS_LIGHTMAP`; whether another
     path adds `slots::alpha_channel` for other families was not traced). Census
     which Omega and 2048 programs gate a term by the diffuse alpha (the `*alphaspec*` and `fc16_phong_AlphaSpec*`
     families first) before changing the default; 2048's own road was **not checked** here.
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
2. Item 2's gate lane (15 minutes plus the gate: one line in `shade.wesl`, one pixel test, the two pairs
   re-measured), then the shadow-light factor on the prelit term (half a day: live constants are on the page).
