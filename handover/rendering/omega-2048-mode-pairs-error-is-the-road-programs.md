---
status: ready-to-wire
titles: [omega]
updated: 2026-11-20
summary: "2048-mode pairs: alpha lane, tarmac Fsl wired; left: track shadow receiver and lobe, panel-road Fsl, crowd lights, Metro Park terrain"
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
   on the HD-mode circuits goes. **Left (`omega-crowd-light`, 2026-11-20, [`omega-status.md`](../../docs/formats/omega-status.md), "The plain crowd's light inputs, read"):** the program is ours' formula
   (a probe left every pixel unchanged), so the bound `constantAmbientColour`/`liveLighting0diffuse` needed a live read
   (done, `omega-crowd-live`: they equal ours'). Sol's plain sprite was +64.5 (15,198 px; ours lit each figure fully or not at all, the original
   none): **`omega-crowd-normal` (2026-11-20, [`omega-status.md`](../../docs/formats/omega-status.md), "The crowd sprite's frame") read the route and wired it**: the unshadowed programs weight the map's `z` by `eye - position`, not the vertex normal; Sol plain +47.6 (mean abs 78.9 to 64.0), `_shadow` +3.3. **The white silhouettes in the Sol stands are closed** (`omega-sol-silhouettes`, [`omega-status.md`](../../docs/formats/omega-status.md), "The crowd-entity shapes": `fc01_colour_emissive` placeholders of the crowd entities, flags `0x0401`, no longer drawn; their mask +75.9 to -21.8, mean abs 95.7 to 35.8; the animated `Data\Crowd` rigs that replace them in the original are not loaded, open). **Left:** the permutation the original picks for the plain sprite (70); the same eye frame on non-crowd props' unshadowed permutations (a global probe moved Capital 47.26 to 42.89 and Empire 44.97 to 54.17: not a law yet). Empire's dimness is scene-wide (non-crowd structure 0.69, crowd 0.61), not the crowd program. 2048: **checked, differs** for the crowd (Vita draws crowds through `fc06_lambert_alpha`/`fc08_effects_crowd`).
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
   - **Shadow-light factor: source solved, wired for the tarmac roads, not for this road yet** (`omega-2048-road`,
     2026-11-20): `Lighting.ShadowLight direction` and `specular power scale` are authored in the circuit's
     `.envsettings` and equal the live reads (Sol exact, Queens Mall after normalising), and `Light::shadow_light`
     carries them; the panel-map road's program uses the same `Fsl`, so wiring it is the `tarmac` block's
     `shadow_light_factor` applied to this family's prelit term (a few lines; measure the four panel pairs).
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
     Climb did not move (47.00, 45.11), so their error is not this. Their tarmac lit program is now read and wired
     (`omega-2048-road`, [`omega-status.md`](../../docs/formats/omega-status.md), "The tarmac roads' lit programs"):
     the shadow-light factor moves them +0.28 and -0.14 grey, so the road program is not their gap
     either (the lobe is read and not drawn until the track has the shadow term); the lead is the track as a shadow receiver (below, Next Steps 3).
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
2. ~~Item 1's remainder: one live read of the crowd draw's constant block on Sol.~~ **Done** (`omega-crowd-live`, 2026-11-20, [`omega-status.md`](../../docs/formats/omega-status.md), "Crowd and scene light inputs, read live"): the bound `constantAmbientColour`, `liveLighting0diffuse`, direction, `Shadowed` and `Ambient` equal the file's authored values (ratio 1.000, two boots), with no per-draw light. The Sol plain sprite's all-or-nothing `N.L` is therefore the normal, not the light: next read the crowd draw's vertex block (`world`, `worldScaleInv`) and the 1,287 white pixels that skip every debug return (half a day, same rig as `scripts/wipeout-pc-crowd-light-live.py`).
3. **The track as a shadow receiver** (new, `omega-2048-road`): the race binds the shadow-map variants of the road programs
   (`tarmac` `@0xa764`, panel road `0xe144`/`0x14284`); their sun term and sun lobe are `LM.a * S`, `S = min(1, pcf(shadowMap)
   * exp2(clamp(c0 + c1 z, c2, c3)))` with the coefficients in `shadowCoeffs`. Ours has the baked `LM.a` only on the track
   (`sun_occlusion` is the hull's). Evidence it matters: with the tarmac sun switched off Capital Reach's whole frame goes
   46.98 to 45.41 and the original's near road is neutral grey where ours is warm.
   Read the pass that fills the map (casters, matrix, `shadowCoeffs` meaning) first (half a day, RE), then wire (a day), then
   the tarmac sun lobe (read and measured, taken out for want of `S`: `omega-status.md`).
4. The panel-map road's shadow-light factor (above): apply `shadow_light_factor` to that family (an hour) and measure
   Sol, Queens Mall, Altima and Unity Square.
5. The tarmac dirt layer's colour (`OAG_TARMAC_DIRT`) is still off by default; revisit after the shadow receiver.
