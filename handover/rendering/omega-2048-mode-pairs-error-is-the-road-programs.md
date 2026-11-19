---
status: ready-to-wire
titles: [omega]
updated: 2026-11-19
summary: "2048-mode pairs: Omega alpha lane wired (crowd, foliage); left: plain crowd lights, Fsl, Metro Park terrain blend, Vita lanes"
difficulty: medium (40)
---

# Omega's WipEout 2048 mode pairs: the error is the road programs, split three ways

2026-11-16, `omega-2048-holistic`. Decomposition, table and evidence:
[`omega-status.md`](../../docs/formats/omega-status.md), "Where the WipEout 2048 mode pairs' error is".
Measure with `scripts/compare-decompose.py`, never the raw mean grey alone (it is dominated by decorrelation;
a correct detail fix can raise it).

## Open

1. ~~**The plain crowd sprites still draw white.**~~ **Mostly closed** (`omega-road-programs`, 2026-11-19): the cause was the
   alpha lane, not a light term. `mesh::rcs::psp2` built every PS4 material's slot word with channel 0 (red) as the
   alpha; it now carries `slots::DEFAULT` (lane 3), Vita only for the alpha-lit families
   ([`omega-status.md`](../../docs/formats/omega-status.md), "The alpha lane"). Crowd bias: Queens Mall +48.5 to +4.7,
   Sol +115.7 to +38.2, Metro Park +14.4 to -10.8; Metro Park also gains its trees, shrubs and flags, and the red slab
   on the HD-mode circuits goes. **Left:** Sol's +38.2 and Empire Climb's -37.2: the plain sprite program's light
   inputs (`constantAmbientColour` and `liveLighting0diffuse` as bound for these draws, the normal-map `N.L`), still
   unread. 2048: **checked, differs** for the crowd (Vita draws crowds through `fc06_lambert_alpha`/`fc08_effects_crowd`).
2. **`road_alphaspec_normal_emissive_panelmap1` (and `_mask`): wired except the lobe's colour** (2026-11-16,
   `panelmap-wire`; [panel-displacement.md](../../docs/ghidra/functions/ps4-omega-eu/panel-displacement.md),
   "Implemented where", has the numbers). March, sky-cube sheen and emissive draw; road at 16x: Sol `LF` 32.7 to
   18.0, `bias` -32.4 to +16.1, `G` 0.35 to 0.75; Queens Mall 30.7 to 14.5, -16.9 to +8.5, 0.71 to 0.85; Altima and
   Unity Square within 1.1 grey and 0.03 `G`. Left:
   - ~~**b. The sun lobe's colour.**~~ **Wired** (`panelmap-sun`, 2026-11-16): `envsettings_light` fills
     `Light::sun_specular`, carried through the zone grade; the lobe is live (a x100 probe moves 85,305 Sol pixels)
     but draws almost nothing at the pairs' poses (Sol 339 pixels, largest step 7 grey), so no road number moves.
   - ~~**Sol's near road overshoot: the gate lane.**~~ **Wired** (`sheen-gloss-wire`, 2026-11-16): the `if panel`
     block in `shade.wesl` reads `first.a` (the program gates by the diffuse alpha, `0xe144` `@0xde0-0xde4`,
     confidence 94; ours read `texel.a`, the red lane on a PS4 slot word). Road at 16x, before to after: Sol `LF`
     17.0 to 8.8, `bias` +13.8 to -3.0, `G` 0.74 to 0.79; Queens Mall `LF` 14.7 to 15.5 (its base is darker than
     the original's, next bullet), `bias` +8.6 to -0.6, `G` 0.85 to 0.83; Altima `LF` 24.7 to 24.9, `bias` +6.4 to
     +6.7; Unity Square `LF` 16.1 to 16.2, `bias` -14.3 to -14.4 (not worse beyond noise, almost no sheen there).
     HD `talons-pack` and 2048 `anulpha-pass` re-render pixel-identical. Test:
     `omega_panel_map_lit_path::the_sheen_is_gated_by_the_diffuse_alpha_not_its_red`. Sol's near road is now
     red-brown; it stays greyer and bluer than the original's, which is the base under the sheen (next bullet).
   - **Left after that, both the base under the sheen:** (a) the program's shadow-light factor
     `Fsl = 0.75 + 0.25 sat(Nw.SL) + SLSD.y sat(R.SL)^SLSD.x` on the prelit term is not drawn; live `SLSD` is
     `(10, 1, 0, 0)` on both circuits and `SL` `(0.791, -0.375, -0.484)` on Sol, `(-0.307, 0.445, -0.841)` on Queens Mall,
     and the original's sheen-off base is 1.2 to 2.3 times ours in linear red in Sol's left and far boxes; (b) Queens
     Mall's sheen-off road is 10 to 35 % brighter in the original (live `novaScaleBiasPower` `(2, 0, 2.5)`; Sol's is
     `(1, 0, 1)`): compare with what ours binds there first. `scripts/wipeout-pc-road-sheen-live.py --pre
     ReflectivityAmount=0` gives the original's base at the pair's pose (about 4 minutes a boot).
   - ~~**Wider, open: other families reading `texel.a` as red.**~~ **Omega: wired** (`omega-road-programs`, 2026-11-19, the
     alpha lane above): every PS4 material reads lane 3, so the generic specular mask, the snow term, the hull's probe
     reflection, the glass pane's cover and clear and the output alpha read the diffuse alpha. `LIT_ALPHA` materials
     read it on both titles. **Left:** Vita's other families (2048's GXP programs: which gate by the diffuse alpha is
     unread; lane 3 everywhere moved `moa-therma` 32.55 to 34.23: **that was the awning, not the hull's reflection**
     (`vita-race-pairs`, 2026-11-19: wired with the reflective window, `moa-therma` 32.55 to 27.56; lane 3 everywhere
     then moves `modesto-heights` +0.07 and `empire-climb` +0.09), and whether Omega's `tarmac_*` and brick programs gate their specular by it: Capital Reach and Empire
     Climb did not move (47.00, 45.11), so their error is not this.
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

1. Metro Park's terrain (new, `omega-road-programs`): `grassmudrock_blend_diffuse_normal_conditional` (about 100,000
   triangles over eight submesh groups, ten textures: grass, gravel, rock, mud, grass-transition diffuse and their
   normals, program `Cg_ConditionalBlender1NormalMap`) draws grass in ours where the original draws brown mud and
   dirt in the lower left, the largest scenery area of that pair. Nothing in the tree reads the material. Read the
   program's blend weights (vertex colour or a mask), then wire (half a day RE, then a day to wire).
2. Item 1's remainder: the plain crowd program's light inputs (half a day, RE).
3. Item 2's shadow-light factor on the prelit term (`Fsl`): it reaches Queens Mall among the four worst pairs only,
   and nothing in our code carries a shadow-light direction (`SL` was read live), so the first step is its source in
   the lighting data; Queens Mall's road bias is already positive after the crowd fix (+5.7), so the factor would
   brighten a road that is not dark (half a day, RE).
