---
status: ready-to-wire
titles: [omega]
updated: 2026-11-16
summary: "2048-mode pairs: road programs lead (3 families); crowd Shadowed law and panelmap1 road law (ray march, skyCube sheen) read, ready to wire"
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
2. **`road_alphaspec_normal_emissive_panelmap1` (and `_mask`): ready to wire** (2026-11-16, `panelmap-re`; law
   85 to 88, no live run). The whole law, the authored values per circuit, the panel map's and the cube's decode are
   on [`panel-displacement.md`](../../docs/ghidra/functions/ps4-omega-eu/panel-displacement.md); nothing below needs
   Ghidra or a disassembly. Biggest single family (Queens Mall, Sol, Altima, Unity Square). In order of what it is
   worth on the pairs:
   - **a. The sky sheen** (Sol's 38.5 grey; no new vertex data). Load the circuit's `skyCube.gnf` (beside
     `skycube.rcsmodel`; BC7 cube array, 4 cubes x 6 faces x 11 levels, layout on the page and in
     [`gnf.md`](../../docs/formats/gnf.md), "Two shapes the decoder does not read yet") as a cube-array texture and
     add `ReflectivityAmount * albedo.a * (ReflectivityMin + ReflectivityScale * (1 - N.V)^ReflectivityPower) *
     mix(cube[floor(2 * Blurrines)], cube[floor(2 * Blurrines) + 1], fract(2 * Blurrines))` looked up at
     `(R.x, R.y, -R.z)`, `R` the view reflected about the normal-mapped normal. `_mask` multiplies it (and the
     lobe) by `ReflectionMask(uv).rgb`. Parameter hashes on the page. Omega only, admitted by material name plus the
     four `Reflectivity*` parameters being present.
   - **b. The sun lobe**: `liveLighting0specular * SpecStrength * albedo.a * lightmap.a * sat(R.L)^p * 0.0698 /
     max(0.0698, 0.1745 * min(1, Blurrines))`, `p = SpecPower * (1 - 0.75 * min(1, Blurrines))`.
     `liveLighting0specular` is taken as `Lighting.Sun specular colour` times `Lighting.Sun specular intensity`
     (the product `sun-intensity.md` shows in the front end's light block; the race scene's upload was not read,
     so name it inherited); Sol authors `(6.1, 3.3, 1.6) * 1` with `SpecStrength` 5.
   - **c. The displacement** (the gradient ratio `G`; Sol 0.24, Queens Mall 0.61). Decode the `R16F` panel map
     (format `0x02`, float; page, "The panel map") to an `R16Float` texture and run the 10-sample sphere trace in
     the fragment stage; sample diffuse, normal and emissive at the shifted coordinate with the unshifted
     gradients. The lip blend can be left out (inert, page). Needs the world tangent and bitangent in the fragment
     stage, which the Omega normal-map path already carries (`rcs/psp2/normal_map.rs`).
   - **d. The emissive** is added unscaled at the shifted coordinate (Queens Mall's and Unity Square's cyan lines);
     check whether `glow.rs` already draws it before adding anything.
   - Where: a new module beside `crates/mesh/src/mesh/rcs/psp2/tarmac.rs` (same pattern: admit by name and
     samplers, a glow-table program mode, extra bindings); `psp2.rs` is at the 1,500-line ceiling.
   - **Acceptance** (`scripts/compare-decompose.py --pair data/reference/compare-page/omega/<id> --road
     road_alphaspec`, numbers before from `omega-status.md`): `sol-2048` road `bias` -38.5 moves toward 0 by at
     least half and road `LF` share (51 %) falls with a. and b.; `sol-2048` road `G` 0.24 and `queens-mall-2048`
     road `G` 0.61 rise toward 1 with c., with their road `LF` not rising; `queens-mall-2048` road `bias` -18.0
     toward 0. Read by eye on Sol: the diagonal strips sit below the plates with a lit far wall. A test that fails
     when the reflection weight or the march is dropped; `altima-2048` and `square-2048` not worse.
   - **Open, named on the page**: the cube's orientation is checked on Sol only (sun on `+Z`, world `-Z`); which of
     the four lightmapped programs a draw selects; sampler states; the fresnel's doubled-bitangent normal `N2`
     (start with the shading normal and say so); the dynamic `shadowMap` factor on the sun terms (ours has its own
     shadow path; leave at ours).
   - 2048: **checked, differs** (no panel map on Vita). HD-mode siblings (`*PanelMap*` on Vineta K,
     `panelmap2` on Sol 2) are unread: a separate RE item if their pairs show the same gap.
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
2. Wire item 2 in a fresh `oag-wire` lane, sheen first (a day: the cube-array loader is the new part), then the
   displacement (a day: the `R16F` decode and the march), Sol and Queens Mall as the frames.
