# HD's track tone: the floor gap is the reference's exposure, and HD asks for no anisotropy

2026-10-21, `hd-track-tone`. Two claims were brought to this lane; both are
answered here with the numbers they rest on. **Nothing in the renderer
changed.** What landed is two reproducible probes
(`crates/render/examples/hd_slot_inspect.rs`,
`hd_lightmap_flip_census.rs`) and this page.

| Claim | Verdict |
| --- | --- |
| **A**: one missing or misapplied input of the lit track material makes HD's track too dark and too blue, and one cause explains the floor, the blue areas and the hexagon panel | **False.** The floor is not short of a material input: before the resolve it matches the original within 3 %. The gap sits between the scene and the final frame, in the original's exposure resolve, and at the profile's `Resolution Scale 150` that exposure is an emulator artefact. The wall's blue cast and the hexagon panel are two separate causes |
| **B**: HD's floor, wall and glass-road draws request anisotropic filtering, and with which filter and LOD bias | **False for anisotropy** (max anisotropy field 0 on every bind of two circuits), **trilinear with a `-1.5` LOD bias** on material textures |

## Claim A

### The pair frame, as the lead measured it

`hd/anulpha-pass` of the compare page, 1920x1080, mean RGB per box, theirs
against ours (this tree, unchanged). Ours reproduces the lead's numbers to the
decimal.

| Region | Theirs | Ours | Grey RMSE | Mean grey, ours minus theirs |
| --- | --- | --- | ---: | ---: |
| grey floor centre (500,700)-(1420,1000) | 113.3, 139.2, 147.5 | 90.0, 115.3, 124.1 | 37.3 | -23.7 |
| blue floor left (80,760)-(500,1000) | 35.2, 51.4, 64.6 | 34.6, 52.2, 71.0 | 28.1 | +1.0 |
| blue floor right (1300,700)-(1800,900) | 51.8, 92.2, 105.5 | 46.9, 89.6, 108.4 | 37.2 | -2.7 |
| left wall panel (0,560)-(330,720) | 40.8, 50.8, 46.9 | 43.5, 49.8, 54.4 | 23.4 | +1.0 |
| whole frame | 62.3, 78.7, 82.9 | 53.3, 68.3, 75.9 | 30.9 | -9.6 |

The blue areas and the panel are not darker, only bluer, so a uniform lift of
23 would have broken three of the five rows. Per material (a tinted render,
`OAG_TINT_MATERIALS=1 OAG_OPAQUE_ONLY=1`, matched to slots by hue; the box
holds 183,048 px of slot 476): the grey floor is `diffuse_normal_specular` /
`ds_pit_cs.gtf` (theirs 128, 156, 166; ours 101, 128, 137), the panel is
`cf_chevron_pulse` / `ds_wallchevron_sc.gtf` (slot 442, 89 % of the box;
theirs 36, 46, 44; ours 34, 43, 53), and the blue floor right is mostly
`cf_centre_plasma_pulse`, `lambertzeroalpha` and `constantdiffuse_specular_normal`.
The floor and the panel bind a lightmap, a normal map and an albedo.

### The inputs are there

- **The lightmap is bound and its UVs land.** Slot 476's chunk declares
  `lightmapUV`, the loader binds `ile_mesh_combine1-lmap.gtf` (the atlas the
  draw names), and rendering the sampled texel unlit puts the baked shadow
  where the original's shadow is.
- **No vertex program flips the lightmap coordinate.** `hd_lightmap_flip_census`
  asks `Program::flips` of every resolved lit-race vertex block that carries
  `lightmapUV`: **0 of 733** distinct (material, block) pairs, 22,565 chunks in
  643 models, flip it (today's `1 - v` fix reads only the diffuse set, and that is
  right). The floor's block (`diffuse_normal_specular`, vertex block `0x5250`)
  writes `o[TC4].zw = v[4].xy` with no scale or offset, and builds the frame
  `T = TC3`, `B = TC0 = cross(N, T) * T.w`, `N = TC2`, which is what
  `hull_normal.wesl` computes.
- **The fragment program is the one ours evaluates.** Block 7 of
  `diffuse_normal_specular` (`HalfBrightIleLightmapSunSpot0SVC0`), with the live
  draw's constants, is
  `albedo * (3 * pow(lightmap, 1.6) + sun * lightmap.a * N.L) + specular`, with
  the sun `(1.0865, 1.6937, 1.7896)`, scale 3 and power 1.6 patched in exactly
  as the circuit's `.envsettings` authors them, and the last instruction ends
  `/2`. Its `N.L` is unclamped (`DP3` with no `_SAT`); the specular facing term
  is `sat(8 N.L)`.

### Before the resolve, the floor matches

A restored Anulpha state (`hd-timetrial-anulpha-pass-harimau-classic-start`) was
paused, its resolved scene target dumped (`0x40cc0000`, 1280x720
`A8R8G8B8`: the picture before the exposure, the bloom and the HUD) and the
screenshot taken at the same pause. Ours was rendered at the capture's camera
(`--camera-pose`, within 0.03 units) with the HD bloom's summand switched off
(a temporary `Glow::Suppressed`; not committed), so it holds what the
original's scene target holds. Region means, 1920x1080:

| Region | Original, scene target | **Ours, bloom off** | Original, final frame | Ours, final |
| --- | --- | --- | --- | --- |
| floor box | 82.9, 107.4, 115.7 | **85.7, 109.3, 117.7** | 119.9, 145.2, 153.5 | 90.3, 113.4, 121.5 |
| the lane under the craft | 115.4, 142.8, 148.5 | **113.1, 139.6, 145.3** | 160.9, 192.2, 198.2 | 116.2, 143.4, 149.3 |
| blue floor right | 36.2, 71.8, 84.9 | **41.0, 78.0, 94.0** | 52.7, 96.6, 111.4 | 42.3, 79.4, 95.4 |
| rows 200-900 | 44.6, 56.9, 60.4 | **44.6, 58.9, 66.7** | 63.8, 78.6, 82.7 | 47.1, 62.1, 69.9 |
| left wall | 45.6, 48.2, 49.2 | **46.6, 59.7, 71.4** | 59.8, 63.0, 64.5 | 47.3, 60.6, 72.4 |

The floor and the lane agree to 3 %. The original's **final over scene** is
1.4 in encoded value on the floor (2.2 in linear light), ours 1.05. That is the
whole 23.

### What the original's exposure is, and why it read 1.6

The resolve multiplies the scene by `scale = 4 - min(adapted * 20, 3)`, so it
exceeds 1 only when the adapted luminance is under 0.15. The bloom gate's fade
constant is patched live into the gate program
(`1 - min(adapted * Bloom adaption boost * 0.25, 1)`; Anulpha authors boost 12,
Amphiseum 13), so `adapted` can be read off the draw state, and
`FunkLayer + 0x240` (the PPU's mean of the last ladder level, `0xc50ee0`
through `*0x8b73bc`) holds the same number beside its mean colour:

| Circuit, Resolution Scale | Gate fade | `adapted` | exposure | PPU luma at `+0x25c` | scene encoded-mean luma | encode of the linear mean |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Anulpha, **150** (the compare profile) | 0.645566 then 0.631679 | **0.118 then 0.123** | 1.64, 1.54 | not read | 0.220 | 0.314 |
| Amphiseum, 150 | 0.349187 | 0.200, then 0.213 | 1.0 | 0.2132 | 0.264 | 0.321 |
| Anulpha, **100** | 0.327756 | 0.224 | 1.0 | 0.224 | 0.227 | 0.322 |
| Amphiseum, 100 | not read | 0.266 | 1.0 | 0.266 | 0.271 | 0.329 |
| Talon's Junction, 100 | not read | 0.382 | 1.0 | 0.382 | 0.379 | 0.449 |

- **At 150 the readback is wrong, at 100 it is the frame.** Anulpha's `adapted`
  at 150 is 0.12, half of what the same frame reads at 100, and the exposure
  follows (1.6 against 1.0). A cold boot (`rerender-cold`'s Anulpha original,
  13.9 s after GO) shows the same floor as the restored state
  (117.1, 143.4, 151.7 against 113.3, 139.2, 147.5), so this is not a savestate
  transient. The pair frames `hd/*` were taken at 150
  (`scripts/compare-environment.toml`, `resolution_scale = 150`), which brightens
  every dark circuit's original by an exposure the game would not apply.
- **Anulpha's matched frame at 100.** Original final (98.4, 120.7, 127.4) against
  ours (90.3, 113.4, 121.5) in the floor box: the 30-per-channel gap is 8, and
  the whole-frame boxes agree to a few units (original 49.4, 61.5, 63.6; ours
  47.1, 62.1, 69.9).
- **The adaptation law in `oag_post::hd_bloom` disagrees with the live
  readback.** At 100 the PPU's luma equals the mean of the **encoded** per-pixel
  luma (0.224, 0.266, 0.382 against 0.227, 0.271, 0.379: within 1.3 %), not the
  encode of the linear mean our ladder computes (0.322, 0.329, 0.449: 15 to 43 %
  high). Ours' `adapted` stays at or above 0.15 on all three, so the exposure is
  1.0 there either way; what it moves is the bloom gate's frame-term fade
  (`1 - min(adapted * boost * 0.25, 1)`): Anulpha's is 0.33 live and 0.04 in
  ours. **Whether RPCS3's ladder filters in the encoded domain where the hardware
  filters in linear light is not known**, so this is a measurement of the
  reference, not yet of the console.
- **Trying it**: averaging the encoded value instead (`fs_reduce_first` encodes
  the quarter-res copy, `fs_adapt` no longer does) moves Anulpha's floor box to (102.0, 125.0, 132.7) against
  the 100 original's (98.4, 120.7, 127.4), and the pair's mean grey difference
  19.54 to 16.95. It also moves `tech-de-ra` 14.96 to 15.10, `talons-assegai-c1`
  12.62 to 12.72 and `hd-race/talons-pack` 53.02 to 53.12 (each at or over the
  0.1 rise this lane was bound by, against references taken at 150). **Not
  shipped.**

### The wall and the hexagon panel are their own causes

- **Left wall, before the resolve**: ours is cyan where the original is grey
  (G +11.5, B +22), so it was bluer before any exposure. A candidate, the
  original's unclamped `N.L` (a surface facing away from the sun subtracts the
  sun colour, which is cyan), was tried: it makes the wall neutral and far too
  dark (32.6, 37.8, 42.4 against the original's 54.6, 57.1, 56.5), because the
  other terms of that wall's program are not read. **Not shipped.**
- **The hexagon panel** is `cf_chevron_pulse`, a pulsing emissive family. The
  original's panel carries a green outline in the pair frame and a navy one in a
  scene dump taken at another moment, so it is animated; ours draws a grey
  outline. That is the emissive mask's colour and phase, not the lit material.

So "one cause explains both reports" is false: the floor's brightness is the
reference's exposure; the wall's colour is a second cause; the panel a third.

## Claim B: the sampler

One hooked frame of each circuit (`scripts/rpcs3_draw_hook.py`, `rsx-draw-list.py
--samplers`, the draws named with `scripts/rsx-fp-match-material.py`). The
game's own words, read from the pushbuffer, are independent of the emulator's
anisotropy override.

| Circuit | Draws, binds | Texture control `ctl0` | Filter words |
| --- | ---: | --- | --- |
| Anulpha Pass | 530, 2,619 | `0x80078000` on every bind | `0x02063e80` 1,597; `0x02062000` 824; `0x02022000` 198 |
| The Amphiseum | 867, 3,935 | `0x80078000` on every bind | `0x02063e80` 3,269; `0x02062000` 467; `0x02022000` 199 |

- **Maximum anisotropy is 0 (one sample) on every bind.** `scripts/compare-environment.toml`'s
  "no original forces it" holds for HD and now has a measurement behind it. HD has
  no anisotropy to be an `original` setting value for; the profile's `off` stands.
- **The filter is trilinear with a LOD bias of -1.5 on material textures**:
  `0x02063e80` is magnification LINEAR, minification LINEAR_MIPMAP_LINEAR, bias
  `0x1e80` = -384/256 = -1.5. It sits on the albedo, normal and lightmap units
  alike. Render targets, probes and the ladder read `0x02062000` (bias 0) or
  `0x02022000` (linear, no mip).
- **Sample draws** (unit: format, address word, filter):

| Class | Circuit, draw | Material | Units |
| --- | --- | --- | --- |
| floor | Anulpha #109 | `reversed_diffuse_specular_normal` | u0 `0x60710101`, u1 `0x60010101`, u2 `0x60010101`, u3 `0x60710101`, all `0x02063e80` |
| floor | Amphiseum #178 | `track_surface` | u0 `0x60710101`, u1-u3 `0x60010101`/`0x60710101`, all `0x02063e80` |
| floor | Amphiseum #97 | `track_surface_no_emissive` | u0 `0x60710101`, u1-u2 `0x60010101`, all `0x02063e80` |
| wall | Anulpha #105 | `cf_chevron_pulse` | four units, all `0x02063e80` |
| wall | Amphiseum #115 | `track_wall` | four units, all `0x02063e80` |
| wall | Anulpha #86 | `diffuse_specular` | three units, all `0x02063e80` |
| glass | Anulpha #373 | `diffuse_normal_specular`, blended | u0-u1, u3 `0x02063e80`; u2 (a render target) `0x02022000` |
| glass | Amphiseum #399 | `glass_reflect` | u0, u2 `0x02063e80`; u1, u3 (probes) `0x02062000` |
| glass | both, #102 and #76 | `ds_booth_glass` | u0-u1 `0x02063e80`, u2 `0x02062000` |

- **Ours** samples HD materials with a LOD bias of 0 (the texture-detail slope
  law is the PSP's). The original's -1.5 reads sharper on a distant floor and
  shimmers more. **Checked, applies, not wired**: a shader-side `textureSampleBias`
  on the three units, with the cost of an extra bias constant, is a separate
  change.
- Also read: the albedo unit's address word carries gamma nibble 7 and the
  normal and lightmap units 0 (so the lightmap is raw, as `shade.wesl` reads it);
  `ds_booth_glass`'s albedo carries 0.

## Cross-title

- **Omega, 2048**: the adaptation and exposure readback, the sampler words and
  the `Resolution Scale` artefact are **not checkable**: both need an RPCS3
  frame, and neither title runs there. 2048's Vita and Omega's PS4 sampler state
  is not reachable from this project's captures.
- **The lightmap coordinate census** reads the PS3 `.rcsmaterial` vertex
  microcode, so it is **not checkable by this reading** for 2048 and Omega, as
  `docs/formats/rcsmodel.md`'s flip section already says.

## Open

1. **Retire `resolution_scale = 150` as a brightness reference.** The profile
   needs the original at 100 for any frame whose `adapted` falls under 0.15, or at
   least a note on the pair page. Every dark circuit's `theirs.png` (Anulpha;
   probably Metropia, Moa Therma, Sebenco Climb) carries an exposure the game does
   not apply. To re-make Anulpha at 100: `OAG_RPCS3_EXTRA_EDITS="Video:Resolution
   Scale=100"` before the restore (these edits are applied last, so they win over
   the profile), and judge
   `FunkLayer + 0x25c` (`0xc50ee0`'s object) against the gate fade as above.
2. **Decide the adaptation law** against the console, not the emulator: whether
   the hardware's ladder filters linear or encoded. The live number says encoded;
   the static read of `FUN_003b4690` and the gamma nibble say linear.
3. **The wall's cyan cast** (`cf_chevron_pulse` and the wall slots, pre-resolve)
   and **the pulse's colour and phase** are open and are not lighting.
4. **HD's `-1.5` LOD bias** on material textures.
