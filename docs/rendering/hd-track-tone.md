# HD's track tone: the floor gap is the reference's exposure, and HD asks for no anisotropy

2026-10-21, `hd-track-tone`. Two claims were brought to this lane; both are
answered here with the numbers they rest on. **Nothing in the renderer
changed.** What landed is two reproducible probes
(`crates/render/examples/hd_slot_inspect.rs`,
`hd_lightmap_flip_census.rs`) and this page.

| Claim | Verdict |
| --- | --- |
| **A**: one missing or misapplied input of the lit track material makes HD's track too dark and too blue, and one cause explains the floor, the blue areas and the hexagon panel | **Three parts.** *Too dark*: **false**; the floor is not short of a material input (before the resolve it matches the original within 3 %), the gap is the original's exposure resolve, which at the profile's `Resolution Scale 150` is an emulator artefact. *Too blue*: **true in ours, cause open**, and it is not the exposure: the left wall is cyan before any resolve, and the blue floor on the right is an animated plasma pulse. *One cause*: **false**; the hexagon panel is a third thing |
| **B**: HD's floor, wall and glass-road draws request anisotropic filtering, and with which filter and LOD bias | **The profile's `off` matches what HD requests**: max anisotropy field 0 on every bind of two circuits. The filter is trilinear with a `-1.5` LOD bias on material textures: **true as the request** (2026-10-21, `hd-lod-bias`), **wired and on by default since 2026-10-27** (it was held off while the stored frames' mean grey rose; "Texture crispness" shows that is added detail) |

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
  transient (that lane's RPCS3 log, its latest boot, reads `Resolution Scale: 150`
  and `Anisotropic Filter Override: 0`; the Anulpha capture is earlier, so the log
  shows the profile, not that boot). The pair frames `hd/*` were taken at 150
  (`scripts/compare-environment.toml`, `resolution_scale = 150`), which brightens
  the dark circuits' originals by an exposure the game would not apply (measured on
  Anulpha; Metropia, Moa Therma and Sebenco Climb are predicted, not read).
- **Anulpha's matched frame at 100**, original final against ours:

  | Region | Original at 100 (scene, final) | Ours (final) | Ours minus original |
  | --- | --- | --- | --- |
  | floor box | 82.8, 104.3, 111.3; 98.4, 120.7, 127.4 | 90.3, 113.4, 121.5 | -8.1, -7.3, -5.9 |
  | the lane under the craft | 124.2, 153.0, 159.4; 143.3, 175.3, 181.7 | 116.2, 143.4, 149.3 | -27.1, -31.9, -32.4 |
  | blue floor right | 39.1, 56.2, 67.7; 47.7, 64.9, 75.9 | 42.3, 79.4, 95.4 | -5.4, +14.5, +19.5 |
  | whole, rows 200-900 | 43.7, 55.3, 57.1; 49.4, 61.5, 63.6 | 47.1, 62.1, 69.9 | -2.3, +0.6, +6.3 |
  | left wall | 51.7, 54.4, 53.9; 54.6, 57.1, 56.5 | 47.3, 60.6, 72.4 | -7.3, +3.5, +15.9 |

  So the box's 30 per channel becomes 8, but **the lane keeps a 27 to 32 gap**:
  the original's step from scene to final adds +19 there and ours +3. That is
  the bloom, and it is the part the adaptation law below moves (the lane goes to
  127). The blue floor right is the original's own unstable region: its G reads
  71.8 in a frame at 150 and 56.2 in one at 100 (a plasma pulse), and ours is
  bluer than either. The remaining blue is in ours (wall: G +3.5, B +15.9; whole
  frame B +6.3), so **"too blue" survives the correction.**
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

### Amphiseum's grid floor is not explained by this

The brief's grid-floor box is not recorded anywhere (the UV-flip thread quotes RMSE 46.0 to
30.3 without coordinates). Measured instead on the pair `hd/the-amphiseum` with a tinted
render's slot masks (the HUD excluded): the `track_surface*` floor slots (345,008 px) read
theirs 71.7, 79.9, 80.7 against ours 70.3, 78.1, 78.9, a **mean gap of -1.7 and a grey RMSE
of 12.2, 12.1 of it structure** (the same RMSE with each side's mean removed). The 30 comes
from what shares the box: the billboard (slot 504, RMSE 34.5, ours 12 darker), concrete
panelling (slot 377, 30.6) and the unmatched pixels (32.2). Amphiseum's readback at 150
(0.213) stayed above 0.15, so its exposure was 1.0 either way: **the 150 artefact does not
explain Amphiseum's floor**, and its residual is the thread's lightmap noise, unchanged.

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

- **The profile's `off` matches what HD requests.** Maximum anisotropy is 0 (one
  sample) on every bind. `scripts/compare-environment.toml`'s "no original forces
  it" holds for HD and now has a measurement behind it. HD has no anisotropy to
  be an `original` setting value for.
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
  shimmers more. **Wired 2026-10-21 (`hd-lod-bias`), on by default since 2026-10-27** (`hd-texture-bias`), see
  "Texture crispness" below.
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

## The capture profile moves to 1920x1080 at Resolution Scale 100 (2026-10-21, `hd-capture-profile`)

The maintainer chose scale 100. The candidate that keeps 1080p pairs was RPCS3 `Resolution 1920x1080` at `Resolution Scale 100`
(the game believes it is on a 1080p TV). Method: a **cold boot** to the Time Trial grid (Harimau, classic hull, the
cold-pair walk), then six reads of the PPU's adapted luma at `FunkLayer + 0x25c` (`0xc50ee0`'s object) two to three
seconds apart starting five seconds after the walk returned, then one frame of RSX draw state
(`scripts/hd-draw-dump.py`, `HOOK_RING_ONLY=1`) for the surface clip registers `0x200` and `0x204` of the scene's draws. Exposure is
`4 - min(adapted * 20, 3)`. Mean of the six reads (min to max in the run):

| Circuit | 1280x720, scale 150 (the old profile) | 1280x720, scale 100 | 1920x1080, scale 100 |
| --- | --- | --- | --- |
| Anulpha Pass | 0.139 (0.137 to 0.142), **exposure 1.22** | 0.2127, 0.2109, 0.2114 (three boots), exposure 1.0 | 0.2038, 0.2052 (two boots), exposure 1.0 |
| Moa Therma | 0.326, exposure 1.0 | 0.427, exposure 1.0 | 0.441, 0.440 (two boots), exposure 1.0 |
| Talon's Junction | 0.332, exposure 1.0 | 0.363, exposure 1.0 | 0.358, exposure 1.0 |

- **Claim A: true for the exposure, 3.5 percent off for the number.** At 1920x1080/100 the adapted luma is -3.5 percent (Anulpha),
  +3.2 percent (Moa Therma) and -1.4 percent (Talon's) against 1280x720/100, a sign that changes with the circuit, and the run to run
  spread at 720/100 is 1 percent (three Anulpha boots), so the difference is not zero noise but it is small, not a bias in one direction,
  and the exposure is 1.0 on all three, which is the quantity the profile needs. Anulpha stays above the 0.15 threshold by 36 percent.
  At 150 the same readback is 35, 24 and 9 percent low (Anulpha, Moa Therma, Talon's); only Anulpha is low enough to move the exposure.
  So **the profile becomes 1920x1080 at scale 100** and the pairs stay 1920x1080.
- **The 1080p path renders a reduced, dynamic width.** Scene surface (`0x148`, FP16, pitch `0x2000`) clip at 1920x1080/100: **1300x1080** (Anulpha, three
  frames of one boot; Talon's; Moa Therma's second boot), **1320x1080** (Anulpha, second boot), **1660x1080** (Moa Therma, first boot). The final stage
  (27 draws) is 1920x1080. So the width of the original's scene is not fixed: it changed between boots of one circuit (Moa Therma 1660 and 1300) and
  stayed put inside a boot (three dumps of Anulpha, all 1300). Reading the cause (a load-driven dynamic resolution, the game's own) would
  need the drawing code; not read. At 1280x720 the scene surface is 1280x720 at both scales: at 150 the game still draws 1280x720 and RPCS3
  scales the surface, which is where the exposure's readback goes wrong.
- **What it costs.** The pair's original is 1300 to 1660 px of real horizontal resolution upscaled to 1920 (full 1080 lines), a little
  soft and different from boot to boot in the horizontal axis; 1280x720/100 would be a fixed 1280x720. The maintainer's rule was to keep 1080p if the
  exposure agreed, which it does.
- Cold boot Anulpha Pass pair re-made on the final profile: grey difference over the frame 17.33 before, 14.43 after (the
  pair's ship and camera drifted 0.01 and 0.02 units from the stored pair).

## Open

1. ~~Retire `resolution_scale = 150` as a brightness reference.~~ Done 2026-10-21 (`hd-capture-profile`): the profile is 1920x1080 at
   scale 100 (section above). The thirteen stored `hd/*` originals were taken at 150 and carry the artefact until the batch is re-made.
2. **Decide the adaptation law** against the console, not the emulator: whether
   the hardware's ladder filters linear or encoded. The live number says encoded;
   the static read of `FUN_003b4690` and the gamma nibble say linear.
3. ~~**The wall's cyan cast** (`cf_chevron_pulse` and the wall slots, pre-resolve)~~
   **closed below (2026-10-21, `hd-blue-tone`)**: the wall programs light the
   albedo's alpha. **The pulse's colour and phase** (the chevron's emissive
   mask, `emask.a * colour * wave`) and its `+ emask.rgb` diffuse term stay
   open and are not lighting; the phase, the blocker and the routes are in
   "The chevron panel draws black" below (2026-10-22).
4. ~~**HD's `-1.5` LOD bias** on material textures~~ **wired and on, "Texture crispness" below**.

## The blue cast (2026-10-21, later, `hd-blue-tone`)

**Claim**: our HD frames carry a blue/cyan cast the original does not, caused
by one input to the lit track material or the frame chain. **True, and
fixed.** The cast is one material family's, on one circuit: Anulpha Pass's
`reversed_diffuse_specular_normal` and `cf_chevron_pulse` (walls, floor
plates, the lead-in chevrons) **light the albedo texture's alpha and tint the
specular with its RGB**, and ours lit the RGB. Those textures (`ds_wall_sc`,
`ds_wallchevron_sc`, `ds_floorplain_cs`) carry a blue specular tint in their
RGB (encoded mean B/G 1.35, 1.26, 1.28) and a dark grey picture in their alpha
(mean 64, range 7 to 126), so the surface came out cyan. Judged by
chromaticity (linear-light R/G and B/G of region means), which the original's
exposure does not change; whole-frame mean grey is quoted only as the brief's
regression bound. Nothing else in the chain was the cause: the lightmap
atlases decode to the same format on every slot (`0x88`, DXT5, 1024 square),
the vertex programs of the cast and non-cast materials write the same
interpolators (`o[TC4].zw = v[4].xy`, the same T/B/N frame), the live sun
direction is unit length (`(-0.70069, 0.504497, 0.504497)`), the sun colour's
`w` (the specular scale) is 1, and the prelit scale and power patch in as 3 and
1.6.

### The cast is Anulpha's alone

Whole-frame linear chromaticity, ours minus theirs, on the stored pairs before
this lane (`data/reference/compare-page/hd/*`): Anulpha Pass **B/G +0.089**,
rows 200 to 900 **+0.136**; every other circuit within -0.057 to +0.027
(Chenghou +0.027, Moa Therma +0.012, Sol 2 +0.003, Amphiseum -0.001, Talon's
-0.010, Ubermall -0.010, Sebenco -0.018, Assegai -0.020, Vineta K -0.022,
Metropia -0.023, Tech de Ra -0.046, Modesto -0.057). Chenghou's R/G -0.08 (ours
less warm) is not this cause and is unchanged by it.

### Which materials, by the original's scene target

Per material slot (a tinted render's slot masks, `hd_slot_list` ordinals), the
original's scene target against ours with the glow suppressed, at the restored
Anulpha state, 1920x1080; `dB/G` is ours minus theirs in linear light:

| Slot | Material, texture | Theirs | Ours | dR/G | dB/G |
| ---: | --- | --- | --- | ---: | ---: |
| 476 | `diffuse_normal_specular`, `ds_pit_cs` (the floor) | 99.5, 125.7, 134.7 | 95.5, 121.1, 130.6 | +0.008 | +0.003 |
| 442 | `cf_chevron_pulse`, `ds_wallchevron_sc` (the left wall) | 31.9, 40.7, 39.0 | 33.1, 43.9, 56.2 | -0.129 | **+0.532** |
| 486 | `reversed_diffuse_specular_normal`, `ds_floorplain_cs` | 57.4, 81.1, 87.9 | 66.5, 93.5, 123.8 | -0.135 | **+0.572** |
| 485 | `reversed_diffuse_specular_normal`, `ds_wall_sc` | 43.5, 61.0, 61.9 | 52.5, 79.0, 106.4 | -0.089 | **+0.842** |
| 483 | `cf_chevron_pulse`, `ds_floorchevron_sc` | 40.9, 66.7, 66.6 | 53.8, 71.5, 89.2 | +0.147 | **+0.640** |
| 492 | `diffuse_specular`, `m_metalpanelv08_d` | 35.1, 38.1, 31.7 | 35.7, 41.5, 43.5 | -0.246 | +0.392 |

The floor matches (the previous section's finding, now per slot), and the
cast sits on the two families above. Slot 492's row is a camera artefact: that
scene dump (`d7`) is not at the pair's camera, which the far, small slots show
and the floor does not; against the aligned original (below) `diffuse_specular`
reads +0.033.

### The programs, read

The lit-race block each material resolves to (`HalfBrightIleLightmapSunSpot0SVC0`,
`scripts/ps3-microcode.py fp-file`; the offsets are pinned by
`crates/rcs/tests/rcsmaterial_ground_truth.rs`), unit 0 the albedo, unit 1 the
normal map, unit 2 the lightmap:

- **`reversed_diffuse_specular_normal`** (`0xe9d0`): the same light as the floor's
  (`@0x31 MAD H1.xyz, pow(lm, power), {scale}, sun * lm.a * N.L`) and the same
  specular (`N.H^32 * sat(8 N.L) * lm.a * sun.w * sun.rgb`), combined the other
  way round:

  ```text
  @0x36  TEX H0, f[TC4] unit0               ; albedo
  @0x3b  MUL H2.xyz, H1.wwww, {sun}         ; specular colour
  @0x3d  MUL H0.xyz, H0, H2                 ; albedo.rgb * specular
  @0x3e  MAD H0.xyz, H1, H0.wwww, H0        ; light * albedo.a + that
  @0x41  MAD /2 H0.xyz, fog, H0, fogColour  END
  ```

- **`cf_chevron_pulse`** (`0xf6d0`): the same, plus the chevron's emissive mask
  (unit 2, `ds_wallchevron_emask`) on both sides of the multiply:

  ```text
  @0x16  MUL H7.xyz, H6.wwww, {0x02ab9f07}  ; emask.a * pulse colour ((0, 1, 0) live)
  @0x2f  TEX H0.xyz, R0 unit4               ; the wave, at uv * 0.2 + phase
  @0x37  MUL H7.xyz, H7, H0                 ; the emissive
  @0x43  ADD H6.xyz, H4.wwww, H6            ; albedo.a + emask.rgb
  @0x44  MAD H5.xyz, H2, H6, H7             ; light * that + emissive
  @0x49  MAD H2.xyz, H2, H4, H5             ; + specular * albedo.rgb
  ```

- **`diffuse_normal_specular`** (`0x5430`, the floor) and **`diffuse_specular`**
  (`0xf030`) end on `MUL albedo.rgb, light` and `MAD specular, albedo.wwww, that`:
  the colour is the surface and the alpha masks the specular.

The albedo unit's address word carries gamma nibble 7 (RGB decoded, alpha raw;
the sampler section above), so the grey picture is the raw alpha and the
specular tint the decoded colour. Confidence 90: the microcode's own
arithmetic, two materials agreeing, the live constants read off the hooked
draw (`a-c0753100.bin`: direction, sun, scale, power, the pulse colour, the
wave scroll 0.2 and its phase), and the rendered result below landing on the
original's chromaticity to 0.01.

### What landed

- `oag_rcs::rcsmaterial::fragment::Program::lit_albedo(unit)`: the register the
  prelit scale (`0x8670f0be`, now `rcsmaterial::PRELIT_SCALE`) or the constant
  ambient is patched into is the diffuse light; its lanes are followed forward
  per lane, and the first `MUL`/`MAD` pairing a light lane with the albedo
  unit's sample answers by that sample's swizzle (`.wwww` is `Alpha`, `.xyz` is
  `Colour`). Per lane, because the light register's spare `.w` routinely holds
  the specular scalar; two channels of the sample meeting in one lane
  (`lambertemissive`'s `albedo * (1 - albedo.a)`) answer nothing; a unit fetched
  one lane at a time (`2diffuse_plus_spec_blend_via_alpha_n`'s blend mask) is
  not an albedo.
- `oag_mesh::mesh::slots::LIT_ALPHA` (bit 31; the glow index is masked to seven
  bits, 64 entries), set by `skin::roles` where the program reads `Alpha` **and**
  `picks` bound a picture: `m_diffuse_normal_constantspecular` lights
  `constant * normal.a` off a normal map at unit 0, the renderer binds that
  family's constant as its picture, and flagging it painted Ubermall's and Tech
  de Ra's pipes white (mean grey +2.8 and +1.5, caught by the pair table and
  reverted). Its `normal.a` factor is a separate, unwired term.
- `shade.wesl`: on the flag, `surface = vec3(albedo.a)` (raw) and the specular is
  tinted by the decoded albedo instead of masked by its alpha.
- Disc-wide reach (`crates/render/examples/hd_lit_albedo_census.rs`, every chunk
  of 637 models resolved to its lit-race `Static` block, 54,629 chunks): four
  families read `Alpha`, all Anulpha Pass's: `reversed_diffuse_specular_normal`
  378 chunks, `cf_chevron_pulse` 172, `chevron_pulse` 9, and the gated
  `m_diffuse_normal_constantspecular` 94. 29,049 chunks read `Colour`; 24,927
  carry no prelit or ambient patch (emissive, glass, hull families) and keep
  their own paths. Pinned by `rcsmaterial_ground_truth.rs` and
  `crates/render/tests/hd_lit_alpha_ground_truth.rs` (Anulpha's slots by number).

### Before and after

Against the aligned original (the `hd-track-tone` lane's `pair3`, the final
frame at its own camera), the same slots:

| Slot | dR/G, dB/G before | dR/G, dB/G after |
| ---: | --- | --- |
| 486 `reversed`, floor plate | -0.177, +0.613 | **-0.002, -0.003** |
| 485 `reversed`, wall | -0.118, +0.520 | **+0.021, -0.010** |
| 442 `cf_chevron_pulse`, left wall | -0.241, +0.362 | **+0.006, -0.010** |
| 476 floor | -0.077, +0.036 | -0.076, +0.036 |
| 492 `diffuse_specular` | +0.027, +0.033 | +0.027, +0.032 |

Regions, same pair: left wall (0,560)-(330,720) R/G -0.317 and B/G +0.332 to
**-0.003 and +0.005**; rows 200 to 900 B/G +0.158 to +0.028; whole frame
+0.109 to +0.042.

The brief's acceptance, on the compare page's own pair `hd/anulpha-pass` (its
original at Resolution Scale 150), ours minus theirs in linear light:

| Region | dR/G before | dB/G before | dR/G after | dB/G after |
| --- | ---: | ---: | ---: | ---: |
| grey floor centre (500,700)-(1420,1000) | -0.066 | +0.036 | -0.063 | +0.031 |
| blue floor left (80,760)-(500,1000) | -0.035 | +0.073 | -0.013 | -0.004 |
| blue floor right (1300,700)-(1800,900) | -0.051 | +0.180 | +0.022 | +0.014 |
| left wall (0,560)-(330,720) | -0.095 | +0.113 | +0.056 | +0.013 |
| rows 200 to 900 | -0.062 | +0.136 | -0.015 | +0.019 |
| whole frame | -0.034 | +0.089 | -0.008 | +0.024 |

Chenghou Project and Sebenco Climb (the two more circuits): every region
identical before and after, as the census predicts. `just compare-rerender-ours`
over every HD pair: Anulpha Pass mean grey **19.74 to 17.33**, RMSE 0.124 to
0.112, edge correlation 0.35 to 0.39; the other twelve time trials and
`hd-race/talons-pack` unchanged to the decimal (no pair rose).

What remains on Anulpha after this: the floor's -0.06 R/G (ours less red than
the stored original; pre-resolve the floor matches, so this is the original's
exposure and bloom at 150), the blue floor right's pulse, the chevron panel's
emissive (`emask.a * (0, 1, 0) * wave`, a green outline live) and its
`+ emask.rgb` diffuse term (mean 0.05), neither drawn.

### Tried and not shipped

- **The unclamped diffuse `N.L`** (every program above dots the sun without
  `_SAT`): on the lightmapped chunks it darkens the floor by 6 (95.5 to 89.8
  against 99.5) and the left wall by 16, and moves no reversed slot's
  chromaticity (485 B/G 1.290 either way). The lightmap's alpha already zeroes
  the sun where a surface faces away; whatever is left is not this cast.
- **The specular on the reversed slots** carries the remaining blue there
  (485 B/G 1.290 with it, 1.095 without, the original 0.979 in the misaligned
  dump); it is the program's own term with the live constants, and the aligned
  pair shows the slot within 0.01, so it stays.

### Cross-title

Both re-ship Anulpha Pass with the same families and textures, so the finding
applies to both and neither is wired:

- **Omega (checked, applies, not wired)**: `uroot/data08.psarc` (the patch)
  carries `environments/15_Anulpha_Pass/Materials/Reversed_Diffuse_Specular_Normal{,_1,_2}.rcsmaterial`,
  `cf_Chevron_Pulse{,_1,_2,_Displacement2Out*}.rcsmaterial` and
  `Chevron_Pulse*`; `data01.psarc` carries `TexturesPS4/ds_wall_sc.gnf`,
  `ds_floorplain_cs.gnf` and `ds_wallchevron_N.gnf`. The reader here is the PS3
  fragment microcode, so `LIT_ALPHA` is never set on Omega; whether its PSSL
  pixel shader for the family multiplies the alpha the same way is a GCN read
  (`ps4-omega-eu/lightmap-prelit.md`'s method), not done.
- **2048 (checked, applies, not wired)**: the Vita base `data.psarc` ships
  `textures/ds_wall_sc.gxt`, `ds_wallchevron_sc.gxt` and `ds_wallchevron_n.gxt`
  under its Anulpha Pass directory; the material's `.gxp` program is not read
  by this lane.

## The LOD bias, wired and held (2026-10-21, `hd-lod-bias`)

**Claim**: ours samples HD's material textures at bias 0 where the original
asks for `-1.5`, so our distant floor and wall textures are blurrier.
**True for the request, false for the picture.**

**Wiring** (one mechanism, off for every title today): `Title::looks.material_lod_bias`
(`Rule`, with `oag_title::MATERIAL_LOD_BIAS = -1.5`) -> `Looks::material_lod_levels`
-> `Model::material_lod_bias` on the track model and the models behind its glass
(`oag_raceplay::load::geometry::track_model`) -> the `material_lod_bias`
pipeline constant (`mesh_render::texlod::constants`) -> `material_sample` in
`crates/mesh/shaders/texture_lod.wesl`, a `textureSampleBias` that the circuit
material path in `shade.wesl` (albedo, lightmap, the emissive and mask units,
the second-UV reads) uses in place of `textureSample`; the two `textureSampleGrad`
reads scale their gradients by `exp2(bias)`, as `Grad` ignores a bias. Left
at bias 0, as unmeasured: the environment probes and every `textureSampleLevel`
read (the census shows the probes at filter word `0x02062000`, bias 0), the
flame, absorb and fire programs, the Zone stage, the shadow maps, and every
hull (the census named track draws only; a hull's units are unread). Test:
`oag-render::it psp_slope_lod::a_material_lod_bias_reads_a_finer_level_than_the_samplers_own`
(a minified stripe texture, contrast 0 at bias 0, 117 at `-1.5`) and
`oag-hd effects::tests::the_material_lod_bias_follows_its_rule`.

**Measured**: ours re-drawn with the bias on (`compare-rerender-ours`, all 13 HD
time trials) against the stored RPCS3 frames, 1920x1080:

| | bias 0 | bias -1.5 |
| --- | --- | --- |
| Whole-frame mean grey difference, 13 pairs | Anulpha 17.33, Chenghou 8.54, Metropia 11.24, Moa 20.96, Modesto 16.70, Sebenco 18.57, Sol 14.68, Talons 12.57, Talons Assegai 11.62, Tech 15.87, Amphiseum 13.47, Ubermall 12.08, Vineta 18.09 | 17.53, 8.60, 11.54, 21.25, 16.96, 19.18, 15.09, 12.69, 11.74, 16.26, 13.80, 12.70, 18.16 (**all 13 up, 0.06 to 0.62; 11 of 13 over the 0.1 allowance**) |
| Edge correlation | unchanged to 0.01 on every pair, except Sebenco 0.31 -> 0.30 | |

Per region (grey RMSE against the original, high-pass energy = the standard
deviation of the Laplacian of the 0.7-blurred grey, structure correlation of
that high-pass), Anulpha Pass, The Amphiseum and Ubermall, five regions each
(distant floor, mid floor, left wall, right floor, far right wall): RMSE rose in
14 of 15 regions, by up to 2 grey levels, and the high-pass structure
correlation rose by at most 0.05 (one region) and fell in 5. The
high-pass energy of **ours at bias 0 already sat at 0.9x to 1.4x the
original's** (Anulpha far floor 9.25 against 10.53, Amphiseum far floor
15.9 against 11.25, Ubermall far floor 19.4 against 16.7), and the bias took it
to 1.2x to 1.5x: sharper than the original in most regions, with no
better alignment, which is the shimmer rather than detail.

**The capture's `Resolution Scale 150` is not the explanation**: the same
three pairs re-drawn with `--presented --render-scale 150` (supersampled the way
the original's frame is) before and after the bias read the same way (mean grey
17.40 -> 17.53, 13.34 -> 13.53, 12.00 -> 12.31; RMSE up in 14 of 15 regions;
the bias-0 high-pass energy 0.9x to 1.2x the original's). **What stays open**:
whether RPCS3 applies the filter word's bias at all (the pushbuffer is the
game's request; the emulator's sampler is not read here), and whether the
console's own mip chains (authored in the texture, ours generated for a
texture with none) are sharper. Either would explain why a requested `-1.5`
leaves the stored frames no sharper than ours.

**Held off**: HD's rule is `Rule::UNREAD` with the measured request written in
the comment; turning it on is one line in `crates/hd/src/effects.rs`
(`Platforms::Any`, `Origin::Measured`) and re-judging against a capture taken
at `Resolution Scale 100` once `hd-capture-profile` lands. Pulse and Pure carry
no such rule (the GE's slope law selects the level). **Omega and 2048: not
checkable** (a Vita or PS4 sampler state, no capture path), so neither carries
the rule: an inherited bias would change two tracks nobody has measured.

### Re-judged on the cold originals: still held off (2026-10-22, `hd-sharp-hud`)

**Claim: turning the `-1.5` bias on moves ours' high-frequency energy toward the original's in the Tech de Ra glass grid box and a Modesto
Heights background-building box. Result: false on the picture's total, true for the grid's structure.** Ours re-drawn with the rule on
(`Rule { on: Any, origin: Measured }`, nothing else) against the 13 cold pairs (1920x1080, Resolution Scale 100, `pair.json` clean), boxes chosen
before the change (x, y, w, h):

| box | metric | original | ours, bias 0 | ours, bias -1.5 |
| --- | --- | --- | --- | --- |
| Tech de Ra, road grid `(630, 570, 250, 75)` | Laplacian variance, grey | 288.1 | 322.6 | 655.5 |
| Tech de Ra, mid band `(650, 540)-(1400, 600)` (the section above's metric) | RMS of the Laplacian of the 0.7-blurred grey | 0.0379 | 0.0304 | 0.0410 |
| Modesto Heights, background towers `(440, 280, 100, 130)` | Laplacian variance, grey | 232.3 | 733.1 | 977.2 |

The grid box is already above the original's in raw Laplacian variance at bias 0 (its edges are the panels'), and the bias doubles it; the section's
own blurred metric reads the other way (0.0304 to 0.0410 against 0.0379, the gap closed from 0.0075 to 0.0031), and a 3x crop shows the
bias is what brings the grid lines and the panel outlines back into the glass. The Modesto box is ours' over-sharp already (3.2x), so the
bias moves it away. **Whole-frame mean grey difference** (HUD-alignment commit as the baseline), bias 0 to bias -1.5:
Anulpha Pass 12.51 to 12.87, Chenghou 7.73 to 7.77, Metropia 10.04 to 10.26, Moa Therma 10.22 to 10.54, Modesto 15.84 to 16.08, Sebenco 15.44 to
15.76, Sol-2 10.48 to 10.60, Talon's Assegai 10.40 to 10.50, Talon's 9.39 to 9.43, Tech de Ra 15.79 to 16.25, Amphiseum 11.60 to 11.87, Ubermall 9.26
to 9.71, Vineta K 18.46 to 18.44: **12 of 13 up, 8 by more than 0.2** (the limit the lane was held to). Left off. The rule that would fix the
grid is the bias on the glass sheen's grid tap alone, which is a chosen scope, not measured; not wired.

**Mips.** The compressed (HD `.gtf`) path uploads the authored chain and drops a level only when it is wider than the device's
`max_texture_dimension_2d` or not block-aligned (`first_fitting_level`, `mesh_render/texture.rs`); HD's textures top out at 2048, so nothing is dropped
on a desktop adapter (read from the code; the per-model census line was not captured on this run). **Anisotropy.** The RSX texture control word
`0x80078000` is anisotropy 0 on every bind and RPCS3 ran with `Anisotropic Filter Override: 0`, so the original samples without it; the pair
profile already draws ours with `--anisotropy off`, which is the same. The shipped default is `16x`, a player setting that sharpens grazing floors beyond
the original. Reported, not wired.


## Texture crispness: ours is 10-20 % soft in the near fine band, and the original's `-1.5` bias is the cause (2026-10-27, `texture-crispness`)

**Claim (maintainer: "ours look washed out in all HD shots"): ours is blurrier than the original on the cold HD pairs. Result: true, a modest
gap, concentrated in near surfaces; the named cause is the held-off `material_lod_bias`. Confidence 75** (5 pairs ablated, 2 metrics, one
RPCS3 capture per pair, no second boot of the original).

**The metric matters, and the first two were wrong.** On the 13 cold pairs (`data/reference/compare-page/hd/*`, 1920x1080, Resolution Scale
100), per-tile Laplacian variance says ours is 2.1-3.7x *sharper* than the original: it counts polygon silhouettes, which ours draws harder
(why the original's edges are softer was not investigated). Median |Laplacian| (edge-robust) says 0.78-1.00x. The metric used for every number
below is `scripts/texture-sharpness.py`: boxes 120x72 with the 99.5th-percentile Sobel under 160 in both images (no hull, HUD or silhouette),
band energy = std of a difference-of-Gaussian band (fine 0.7-1.4 px, mid 1.4-2.8 px), median ratio ours/original, split at mid-frame into near
(lower) and far (upper). Written down before the captures: false if the median fine and mid ratios were at least 0.95 near and far.

| ablation (ours, 5 pairs, 90 boxes pooled) | fine near | fine far | mid near | mid far |
| --- | --- | --- | --- | --- |
| baseline (bias 0, `--anisotropy off`, shipped pair profile) | 0.81 | 0.98 | 0.88 | 0.94 |
| `material_lod_bias` -0.75 | 1.00 | 1.10 | 0.95 | 0.99 |
| `material_lod_bias` -1.0 | 1.06 | 1.14 | 0.96 | 1.00 |
| `material_lod_bias` -1.5 (the filter word's value) | 1.11 | 1.19 | 0.98 | 1.00 |
| `--anisotropy 16x`, bias 0 | 1.05 | 1.19 | 0.96 | 1.03 |
| `--render-scale 200` | 0.81 | 0.98 | 0.88 | 0.94 |

Pairs: Tech de Ra, Vineta K, the Amphiseum, Sebenco Climb, Talon's. Per pair the baseline near fine ratio is 0.73 (Amphiseum), 0.81 (Vineta K), 0.84
(Sebenco), 0.86 (Tech de Ra, 2 boxes), 1.10 (Talon's); only Tech de Ra does not move with any ablation (its edge-free boxes are glass and
unlit wall). Ours re-rendered with the current build reproduces the stored `ours.png` (same mean grey and RMSE) before each variant.

- **(a) mip selection: the cause.** The near/far pattern (near soft, far fine) is what a missing negative bias predicts, and the bias lifts the
  near fine band from 0.81 to 1.0-1.1. A crop (Vineta K, the grass-textured kerb) shows the original's grass blades and ours softened at bias 0, then
  matched at -0.75; at -1.5 ours is crisper than the original in the fine band. The filter word's `-1.5` therefore over-corrects slightly in
  our sampler: best fit is between -0.75 and -1.0 (mid band reaches 0.96 only near -1.0). Not understood: whether RPCS3's own LOD computation or
  our `textureSampleBias` with derivative scaling accounts for the 0.5 level; **one capture each, not a measured law**.
- **(b) our mip chain:** unchanged from the section above (authored GTF chain, read from code); not re-measured.
- **(c) anisotropy:** `16x` reaches the same sharpness without the bias and goes beyond the original on grazing floors; the original asks for none
  (control word anisotropy 0, RPCS3 override 0). The shipped default is `16x`, so the player build already gets most of this; the **pair profile**
  (`off`) is what looks soft. A player-facing fix is the bias; the pair profile is a measurement setting.
- **(d) decode/premultiply:** no evidence of one: broadband contrast ratio is 0.90-1.00 and far fine ratio about 1.0 at bias 0, so the decode
  keeps the amplitude the far boxes show.
- **(e) render scale:** `--render-scale 200` gives a byte-identical frame to the baseline in the pair path (the flag is overridden or
  pinned there; not investigated), so supersampling is unmeasured. Post is off in the profile (`--screen-filter off`, `--msaa off`).
- **(f) the original is not upscaled:** horizontal against vertical median gradient is the same in both (0.48-0.68 original, 0.51-0.68 ours on
  five pairs), so RPCS3's frame is not a narrower render stretched wide.
- **(g) the compare page:** the metric reads the raw PNGs, not `side.png`.

**Not blur, but part of "washed out".** On the Vineta K crop the original has a warm light pool on the kerb top that ours lacks, and on Tech de
Ra the glass road grid is absent (the section above): missing content, not sharpness, and the bias does not bring either back.

**Whole-frame cost:** the pair script's mean-grey difference rises with bias -1.5 on every ablated pair (Tech de Ra 7.15 to 7.43, Sebenco 7.21 to
7.42, Talon's 5.72 to 5.77), at -0.75 by 0.05 or less on four of five; that is what held the rule off in the section above.

**Pulse and Pure** (same band metric rescaled for 480x272, PPSSPP software renderer): Pulse's two race pairs read fine 1.04 and 0.99, mid 1.01
and 1.00; Pure's Modesto 0.99 and 0.94 (Blue Ridge, Chenghou, Vineta have 1-4 boxes, 0.98-1.39, too few to read). No gap, so the cause does not
apply; the maintainer's impression there is the simpler textures.

**Landed (2026-10-27, `hd-texture-bias`).** HD applies the filter word's own `-1.5` (`Looks::material_lod_bias`, `Platforms::Any`,
`Origin::Measured`). **The request is read off the original (the filter word); choosing it over the -0.75 fit to RPCS3 is the maintainer's
decision**, made after comparing full frames at -0.75 and -1.5 against the original; it is not a fitted value. It reaches only circuit
materials: `material_lod_levels()` is read once, in `oag_raceplay::load::geometry::track_model`, and set on the track and behind-glass
models (`Model::material_lod_bias`), so the HUD, shadow and sky samplers never see it. Nothing else gates it (no setting).

Re-render of the 13 cold race pairs (`compare-rerender-ours --only hd/`, into a scratch copy), band metric from `scripts/texture-sharpness.py`:
median over pairs of the per-pair ratio ours/original, **fine near 1.11, fine far 1.16, mid near 0.97, mid far 1.02** (before: 0.81 near, 0.98
far on five pairs). Per pair fine near: anulpha 1.06, chenghou 1.12, metropia 1.11, moa 1.06, modesto 1.11, sebenco 1.18, sol-2 1.04, talons
1.17, talons-assegai 1.19, tech-de-ra 0.86 (2 boxes, glass and unlit wall), amphiseum 1.09, ubermall 1.23, vineta-k 1.13; far 1.03-1.38.
Mean grey difference to the original, summed over the 13: up 0.05-0.33 on twelve pairs (detail, not error), **ubermall +0.77** the one over
0.5: looked at, the frames match; the overhead hologram boards (ours show a logo where the original shows "GO") dominate that pair, which is
the hologram lane's, not the bias's. Its mean grey went 9.11 to 9.88. Closing summary: `18 pair(s) ... 0 better, 13 worse, 5 same (the five
menus); hd summed mean grey 141.3 -> 144.3 (+3.0)`.

**Omega and 2048 (checked, applies, not wired).** Both leave `material_lod_bias` at `Rule::UNREAD` (`Looks::unread`). The `0x02063e80` word is an
RSX register; no bias request was read from 2048's Vita sampler states or Omega's GNM samplers, so HD's measurement is not carried over. Reading
those sampler states is the open step, not a one-line rule.

**Left (likely explained below, "The 0.5-level gap is ours being brighter").** The 0.5-level gap against the RPCS3 fit (-0.75 fits, the word says -1.5): RPCS3's LOD computation or our `textureSampleBias` with
derivative scaling; the warm light pool on the Vineta K kerb and the Tech de Ra glass grid (missing content, not blur).

### The 0.5-level gap is ours being brighter (2026-10-27, lead)

The band ratio is an absolute std, so a brighter surface reads as more detail. On the same five ablated pairs, near boxes,
ours/original (`scripts/texture-sharpness.py` boxes, medians):

| Pair | mean luma | coarse contrast (blur 4 std) | fine at -1.5 | fine at -1.5 over mean luma |
| --- | --- | --- | --- | --- |
| Vineta K | 1.18 | 0.90 | 1.13 | 0.94 |
| Sebenco Climb | 1.18 | 0.97 | 1.18 | 1.01 |
| The Amphiseum | 1.11 | 0.90 | 1.09 | 0.93 |
| Talon's Junction | 1.01 | 1.00 | 1.17 | 1.15 |
| Tech de Ra (2 boxes) | 1.01 | 1.06 | 0.86 | 0.85 |

With brightness divided out, `-1.5` lands at 0.93 to 1.01 on the three circuits where ours is 11 to 18 % brighter, so the
`-0.75` fit was ours' extra brightness standing in for detail, not an RPCS3 LOD difference (confidence 60: five pairs, one
capture each; Talon's and Tech de Ra are not brighter and do not move). The brightness itself, with coarse contrast at 0.90,
reads as a lifted, flatter tone on those circuits: an open lead, see the exposure adaptation law above ("Decide the
adaptation law") as the first candidate.

### The dark-end lift is the final encode (2026-10-27, `hd-brightness`)

**Claim**: "ours is 11-18 % brighter with coarse contrast 0.90 on Vineta K, Sebenco Climb and The Amphiseum, not on Talon's and Tech de Ra, because of X." **Half true: it is not an exposure gain, and X is the final encode curve.**

Method: ours re-rendered from scratch copies of the cold pairs (all captured at Resolution Scale 100, so not the scale-150 artefact), same box picker and metrics as above. Falsifier written first: a global exposure or adaptation error scales the whole frame, so whole-frame luma would move with the near-box luma.

1. **It is a dark-end lift, not a gain.** Whole-frame luma ours/original is 1.000, 1.017, 1.029, 1.019, 1.017 (Vineta K, Sebenco, Amphiseum, Talon's, Tech de Ra). A 6x8 tile grid on Vineta K reads 0.93 to 1.08 everywhere except the shadowed left wall tiles (original mean 16 to 35), at 1.2 to 1.45. Binned by the original's blurred linear luma, ours minus original is +2 to +7e-3 below 0.04 and about 0 or negative above 0.08, with ratios 1.2 to 2.5 under 0.01. Coarse contrast 0.90 is the same fact: lifted blacks shrink the std. The split is box content: Talon's has 1 to 2 % of pixels under 0.005 where Vineta K has 17 %, and Tech de Ra has two boxes (its bins still read +2 to +6e-3).
2. **Falsified: the adaptation law (suspect 1).** Encoded ladder mean (`fs_reduce_first` encodes, `fs_adapt` drops its `pow`): near-box luma 1.182 -> 1.184 (Vineta K), whole frame +0.00 to +0.03 (slightly brighter, as the fade predicts). Share 0.
3. **Bloom is not it.** `+ bloom` zeroed: dark bins (original under 0.02) move by under 1e-3 to 2e-3 (Vineta K 4.2 -> 3.1e-3 at 0.005); near-box luma 1.182 -> 1.157. Share under 15 %. Bloom removal does cut the bright end (Vineta K whole frame 0.88), so the bloom there is right.
4. **The cause: `fs_encode` ends `pow(resolved, 1/2.2)`.** The original's resolve writes through the ROP's sRGB encode (the shader's own comment: "approximated by one `1/2.2` curve"). At linear 0.01 the two curves give 31 and 25 levels, at 0.1 they agree. Swapped for the exact sRGB encode (local edit, patch kept in scratch; not committed):

| Pair | near-box luma | coarse | whole-frame luma | dark (orig < 60) mean diff |
| --- | --- | --- | --- | --- |
| Vineta K | 1.182 -> 0.948 | 0.904 -> 1.009 | 1.000 -> 0.970 | +4.4 -> -0.8 |
| Sebenco Climb | 1.177 -> 0.955 | 0.972 -> 1.053 | 1.017 -> 0.986 | +3.3 -> -2.0 |
| The Amphiseum | 1.109 -> 0.997 | 0.902 -> 0.994 | 1.029 -> 0.992 | +5.6 -> +0.9 |
| Talon's | 1.012 -> 0.981 | 1.000 -> 1.078 | 1.019 -> 1.003 | +3.1 -> -1.8 |
| Tech de Ra | 1.011 -> 1.017 | 1.057 -> 1.083 | 1.017 -> 0.996 | +3.4 -> -1.5 |

   The transfer curve by bin goes to 0.97 to 1.01 on Vineta K from 0.002 to 0.16. Across all 13 `hd/*` pairs the dark mean diff goes from +3.1..+8.8 to -2.0..+1.8, except Moa Therma (+8.8 -> +5.0, other content). Confidence 75: one render per pair (deterministic), ROP sRGB is the console's documented behaviour, no console frame at the dark end.
5. **Costs, honest.** Whole-frame rmse rises 1 to 3 % (Vineta K 0.0616 -> 0.0624) and Anulpha's mean luma falls 0.966 -> 0.913: the lift had been hiding the known bloom deficit (see "Before the resolve, the floor matches"), now unmasked. Fine band over the original reads 1.09 to 1.27 at `-1.5` once the brightness is right, so the "`-0.75` fit was brightness" reading above is **reopened**: re-judge the bias after the encode lands. Amphiseum keeps +1 to +5e-3 under 0.04 (its own content, hologram board).

**Wired** (2026-10-27, `hd-srgb-encode`): the final `pow(resolved, 1/2.2)` in `fs_encode` (`crates/post/shaders/hd_bloom.wesl`) is the sRGB encode (`srgb_encode`, local to that pass: the other shaders' copies live in other packages). Re-scored on the 13 cold pairs: dark-end mean diff -2.0..+1.8 (Moa Therma +5.0); whole-frame luma ratio moved toward 1.0 on 10 pairs and away on Anulpha (0.966 -> 0.913), Vineta K (1.000 -> 0.970) and Ubermall (1.000 -> 0.994), the bloom deficit unmasked; rmse up 0.0001-0.0020 on 12 pairs, down on Chenghou. `-1.5` bias fine-near band ratio at the new encode: Vineta K 1.27, Anulpha 1.18, Moa 1.12, Tech de Ra 0.88 (bias not changed). Material decode/packer pairs stay 2.2 and cancel; an exact-sRGB packer is a separate lead.

**Moa Therma's road** (box 560,620 800x460): ours has stronger grime than the original (coarse contrast 1.37, near-box luma 1.03). Bias 0 against -1.5 leaves coarse at 1.36 and moves only the fine band (1.06 -> 0.94), so it is not the LOD bias. It is content-level (normal or lightmap strength), the opposite sign of the dark lift above; not chased here. Not checked: Moa's vertex program kind (`2w - 1` or raw).

**Omega/2048**: `fs_encode` runs only for a title that authors `HDR and Bloom` and does not run 2048's `wo_*` chain. Omega's EnvSettings carry a `Tonemap.*` block instead of HD's `HDR and Bloom.*` (`docs/formats/omega-status.md`), so Omega never reaches this pass and keeps its own tonemap end (its output curve is still unread: open). 2048 runs `wo_composite`, also untouched; not checkable (no HD-style resolve measured). Changed: HD only.

## The reflection map is the sky, and Moa Therma's glass reads it (2026-10-21, `hd-one-pair-moa`)

**Target**: the HD pair with the largest difference, `hd/moa-therma` (cold original, 1920x1080, Resolution Scale 100; mean grey 15.05
on main `027182103`). **Cause found, ranked by pixels times error**: the window glass (`dc_windows_a`, slots 541, 560, 577, 596, 599,
605, 609, 629: the ground-floor grandstand, the upper stand and the tower) is near black in ours and pale teal in the original. The
frame's regional ranking before the fix: lower-left grandstand cell 49 grey, top-middle tower 40, right stand 34; after it the top
seven cells hold under 3 of the 10.3 and the rest is spread over structure (the cast shadow, texture sharpness).

**What `paraboloidReflectionTex` is (measured, confidence 90).** On RPCS3 cold boots (`Write Color Buffers` on, no savestate, a GDB read
of the VRAM range) the engine-wide `paraboloidReflectionTex` is **one 512x256 `A8R8G8B8` target at `0xc4065380`, bound on units
1 to 3 of 443 draws of one Moa frame** (the glass, the water sheets, the mag floors; the same address the Vineta K sea's probe
sat at). Its sampler word is `0x60730303`, the gamma-nibble-7 word the hull's probe units carry. It is **not** the hull's 128x64 map (a `.probes`
blend in main memory) and it is **not** filled by any draw of the frame. Its bytes are the circuit's own `sky.gtf`: on four cold boots
(Moa Therma 2048 cubemap, Anulpha Pass, Metropia, Talon's Junction 1024) the read equals the cube's raw bytes averaged over each texel
to 0.008-0.014 rms (correlation 0.99-0.998), against 0.19 rms for any other circuit's sky:

- two discs, `x < 0` on the left half and `x > 0` on the right, each 256 texels across; in a disc `a` runs across it `-1..1`,
  `b = 1 - 2v` up it, the direction is `(+-sqrt(1 - a^2 - b^2), b, z)` with `z = a` on the left and `z = -a` on the right;
- the cube is sampled with GL addressing (the table `sky_cube` draws the faces from), **no gamma step, no `Sky colour` tint and no `Sky
  rotation`**: Talon's Junction (tint 128, rotation 17) fits at rotation 0 with correlation 0.98 and at 17 degrees with 0.55, Moa Therma
  (rotation 300) at 0;
- static: two reads seconds apart are byte-identical, and nothing in the frame writes the target.

The lookup law the programs apply to it is the hull's (`u = 0.25 + (d.x > 0 ? 0.5 + d.z / 4 : -d.z / 4)`, `v = 0.5 - 0.5 d.y`), so the composite a
pane shows is the sky mirrored in `z`. Both halves are measured; the mirror is not chosen. The producer (`liveRsxPanXForm_CubemapToDualParaboloide_fp`) was
**not decoded**: the law is fitted to four reads, not read off its microcode, so no confidence above the fit's is claimed for the corner
region (the original's corners hold a chevron of overshoot nothing reads; ours writes the nearest disc edge, **chosen, not measured**).
**A trap**: a read taken after the draw hook's pause (`rpcs3_draw_hook.py`) came back as `sky^2.2` (a gamma-decoded map) where three fresh
boots read raw bytes; Moa's own fresh re-read is raw. Do not trust a render target read after a hook dump.

**The glass program (`dc_windows_a`, lit block `@0x43d0`, read with `ps3-microcode.py fp-file`, confidence 85).**
`N = nz N0 + nx T + ny B` from a tangent-space normal map (unit 0), `E = normalise(TC1)` towards the eye,
`d = reflect(-E, N)`, `uv = env_probe_uv(d)` (the hull's law), `R = probeTint(tex(unit 2, uv))` (the luma lerp the hull's `probe_tint` is),
`rgb = fog(0.8 R + light Texture1 + specular) / 2`, alpha the diffuse's. The live Moa program (lightmapped variant `0x78c841`, unit 3 for the map)
carries the tint `0x7480de6d` as **`{1, 1, 1, 0}`, where the hulls beside it in the same frame read `{4, 4, 4, 0}`**: one circuit and
one family measured, so `ProbeTint::SCENERY` is that value for every race (Zone mode's is unread). The weight `0x5e5b1937` is `0.8` on every instance
(parameter read per slot, not transcribed).

**Wired (`mesh::sky_paraboloid`, `rcs::window_glass`, `shade.wesl`).** The map is built once at load from the sky model's six faces (a 4x4
box per texel, 0.0104 rms against the original on Moa), uploaded `Rgba8UnormSrgb` and bound at group 2, binding 17; `dc_windows_a` slots take a glow-table
entry of rate 9 carrying the weight; the shader adds `probe_tint(sky(reflect(-V, N))) * weight` before the fog, like the hull's term. **Left out, named**:
the program's normal map (a track vertex carries no tangent frame here), so the reflection reads the vertex normal; the specular and
light terms stay the renderer's own. The other families that declare the map (`glass_reflect`, `ds_booth_glass`, `glasstest`, `mt_windows_a`, the water
sheets, the mag floors, the pipe family, the ice) read it with their own programs and are **not** wired: each needs its block read first.

| HD pair (mean grey vs the cold original) | main | now |
| --- | ---: | ---: |
| moa-therma | 15.05 | 10.32 |
| the other twelve | unchanged | 0.00 each (only Moa Therma carries `dc_windows_a`) |

Pinned by `hd_sky_reflection_ground_truth` (ten texels of the original's map rebuilt from Moa's `sky.gtf` within 8 bytes; Moa's glass slots tagged with weight 0.8).

**Cross-title (2048, Omega).** 2048 authors `skyParaboloid.gxt` per circuit at **699,072 bytes, which is exactly a 512x256 `A8R8G8B8` map with a full mip chain**
(`512*256*4*4/3 = 699,051` plus a header): the same map HD builds at load, shipped baked. **Checked, applies, not wired**: whether 2048's glass samples it is
unread (its fragment programs are declared-interface only) and its layout and `z` sign were not compared (its sky is a dome mesh, not a cube).
Omega: **not checkable** (no capture path; its HD-lineage circuits ship the cube, so the map would build from it, but its materials were not read).

**Open**: the shadows under the craft (the original's is harder), the teal advert wall at the top right (ours 28 grey brighter in its cell, a material not found by slot
bisection this lane), the normal map on the glass, and the other reflection-reading families above.

## The sky-reflecting families, one at a time (2026-10-22, `hd-glass-families`)

The families besides `dc_windows_a` that declare `paraboloidReflectionTex` (`0x9edd3243`), each read off its own resolved
fragment block (`scripts/ps3-microcode.py fp-file`) and tagged in `mesh::rcs::window_glass`'s family table (a row: the family,
its weight parameter, and how its program combines the reflection). Census of the 13 cold pairs' circuits (`hd_slot_list`,
material names containing `glass`, `window` or `reflect`): `glass_reflect` on 11 circuits, `ds_booth_glass` on 11,
`glass_reflect_opacity` on Vineta K, `03_track` and Chenghou (14 slots there), `glass_texture_customr` on Modesto Heights (7 slots)
and Talon's Junction (2), `mt_windows_a` on Anulpha Pass (3).

### Modesto's right-hand glass: `glass_texture_customr` is a pane (confidence 85)

**Found** by slot skip on `hd/modesto-heights` (`OAG_SKIP_SLOT=760,762,789,793,808,809,824` empties the stand's glass band and shows
the interior the original shows through it). Slot 762 (40 chunks) names `j_sky.gtf` at entry 0 (`0x8365b1f3`) and `glass_dark_r.gtf`
at entry 1 (`0x3bdc0403`, `Texture1`); 760 names `j_tower_glass_r.gtf` there. Drawn `One`/`OneMinusSrcAlpha`, state `0x39`.

**The program** (resolved variant: fragment block #5 `@0x2db0`, vertex `@0x2b60`):

- `T1 = tex(unit 0 = Texture1, TC5.zw)`, `a = T1.a`;
- `light = TC0 + TC5.x * N.L * sun` (`TC0` the vertex block's prelit colour, `TC5.x` the vertex sun mask);
- `R = probeTint(tex(unit 1 = paraboloidReflectionTex, env_probe_uv(reflect(-E, N))))` (`iblScalePower` `0x7480de6d`, the hull's law);
- `rgb = fog(a * light * T1.rgb + (1 - a) * (specular + k * R)) / 2`, `alpha = a`, with `k` the material's `Refbrightness`
  (`0xab31c2b1`, `0.205` on slot 760, read per slot) and the specular `pow(N.H, 200) * sun`;
- `TC1.xyz` and `TC2.xyz` are written `0` by the vertex block, so the closing `ADD /2 R1, H0` adds nothing.

**`j_sky.gtf` is never sampled**: none of the material's twelve fragment blocks declares `0x8365b1f3`. Ours bound it as the picture
because the program's colour lane is `Mixed` (the sky map reaches it) and `skin::picks` then fell to entry 0; with `One` as the source
factor the pane added a whole opaque picture of a city skyline over the stand. And the generic emissive reading
(`rcs::emissive`) saw the program accumulate unit 1 and added `Texture1` again as a still glow layer, taking the slot's glow
index; unit 1 is the sky map, so the window-glass families are now refused there as `glass_sheen` is. **Wired**: `window_glass::pane_picture` binds `Texture1`'s
entry as picture and coverage for a pane family, and `shade.wesl` splits the sum by the coverage (`tint.y` 1 on the glow-table entry):
the lit picture times `a`, the specular and `k * R` times `1 - a`. Nothing of the block is left out: it samples no normal map.

| `hd/modesto-heights` | theirs | ours before | ours after |
| --- | ---: | ---: | ---: |
| glass box `1550,170 350x120`, mean RGB | 112.8, 118.4, 128.8 | 139.6, 144.9, 155.0 | 117.9, 123.7, 128.7 |
| whole-frame mean grey difference | | 10.162 | 9.691 |

The other twelve HD pairs: unchanged (Talon's two slots are not in either Talon's pair's view; five pairs differ by one level on 1-7
pixels, shader-recompile rounding). Pulse PSP `amphiseum-black` and Omega `anulpha-pass` byte-identical. At player size the
pane now shows the stand's interior through a faint sky reflection, as the original's does. **Left open**: the original's interior
reads a little sharper behind the glass (its lit strip along the pane's foot is brighter); `N` and `E` of the sky lookup were not
dumped live.

**Omega: checked, applies, not wired.** Omega ships `Glass_Texture_CustomR` (and `_1`, `_2`) on Modesto Heights and Talon's Junction with
the same `j_sky` and `glass_dark_r` textures (`data02.psarc`, `.gnf`); its PS4 programs are not decoded, so whether its pane skips
`j_sky` too is unread, and Omega's `psp2` material route does not run `window_glass`.

### Chenghou's background windows: `glass_2nduv_reflect_glow` is an opaque reflective window with a glow (confidence 80)

**Found** by family skip on `hd/chenghou-project` (skipping slots 69, 71, 75, 89, 90, 256, 348, 370 bares the stepped tower behind the
gantry, whose green-lit window strips are what differ). The family is on Chenghou (8 slots), Metropia (`02_track`, 32), Vineta K (2),
Sebenco (1), Sol 2 (7) and Talon's (2).

**The program**, both race variants read (`04_chenghou_project`: lightmapped block #5 `@0x4380`, vertex-lit block #21 `@0xc900`;
Metropia's copy of the file resolves its vertex-lit slots to `@0x18350`, not read separately):

- `T = tex(unit 1 = 0xc29b64e4, first coordinate set)`, the window picture;
- `G = tex(unit 0 = 0x290a5adb, TC3.zw).a`, the `*_alpha.gtf` mask at the **second** coordinate set (`Uv2`);
- `light` = the atlas (`@0x00`-`@0x10`, lightmapped) or `TC0` (vertex-lit), plus `TC4.x * N.L * sun`;
- `rgb = fog(light * T.rgb + glow * (1 - G) + (1 - T.a) * probeTint(sky(reflect(-E, N)))) / 2`, alpha 1, opaque, no specular term;
- `glow` is parameter `0xfc21467a`, per slot (`0.016, 0.145, 0.018` on Chenghou's slot 348, `1.0, 0.97, 0.7` on Metropia's slot 7); the
  reflection carries no weight parameter.

**What ours drew**: the generic picture reading fell to entry 0, which is the glow **mask**, so the window drew its mask's colour,
with no reflection and no glow. **Wired**: the row binds `0xc29b64e4` as the picture, the window entry (`tint` `[1, 0, 1]`) weights the
reflection by `1 - T.a`, and `offset` 1 adds `glow * (1 - G)` with the colour in the next glow-table entry and the mask bound third
(the binding `snow_rock`'s mask uses) at `texcoord2`. The pair is deduplicated across slots: Metropia's 32 slots at two entries each
filled the 64-entry table and blacked out its floor panels (+6.6 mean grey) until they shared. The glow is added raw, as the program
adds it; the units' gamma nibbles were not read live (**chosen, not measured**: the picture decoded like every other colour unit).

| `hd/chenghou-project` | theirs | ours before | ours after |
| --- | ---: | ---: | ---: |
| tower box `1380,20 180x200`, mean RGB | 62.8, 56.5, 44.9 | 62.8, 55.6, 47.1 | 66.7, 60.6, 49.4 |
| whole-frame mean grey difference | | 6.760 | 6.697 |

The tower's faces now carry the green strips the original shows, but **the box reads 4 grey brighter than either the original or
ours before**. Measured in steps: the picture and the reflection alone gave 57.6, the glow adds 1.3 more; zeroing the renderer's own
specular (which this program does not have) left the box at 58.9, so it is not the cause. The excess is the picture-plus-reflection
term and the glow, whose split between the reflection's brightness and the units' gamma is unread. Metropia 8.432 to 8.427; the other HD pairs unchanged, Pulse PSP and
Omega byte-identical. **Left open**: the reflection's brightness on these panes (the `N` of a far facade and the sky unit's live gamma
were not dumped), and Metropia's `@0x18350` block, assumed the same law.

**Omega: checked, applies, not wired.** Omega ships `Glass_2nduv_reflect_glow` (and `_1`, `_2`) on Vineta K, Metropia and Chenghou
Project, under `Materials/` and `Materials_DLC/`; its PS4 programs are not decoded and its `psp2` material route does not run `window_glass`.

### Anulpha's `mt_windows_a` is `dc_windows_a`'s window (confidence 85)

Slot 579 is the one that draws (4 chunks, lightmapped block #15 `@0xc6e0`; 563 and 571 name the family and draw nothing in this
frame), blended `SrcAlpha`/`OneMinusSrcAlpha`. The block is Moa Therma's glass law with its own units: the normal map (`0xfc52b822`)
at unit 0, the picture `dc_window_a_diffuse` (`0x11cb4f74`) at unit 1, the atlas at unit 2, the sky at unit 3;
`rgb = fog(w * probeTint(sky(reflect(-E, N))) + light * T.rgb + specular) / 2` with `w` the same parameter `0x5e5b1937` (`0.336` here)
and the specular colour `0x370a63cb`, alpha `T.a`. The picture and the alpha were already right; **wired**: the row tags it like
`dc_windows_a`. **Left out, named**: the normal map, as on Moa's glass.

| `hd/anulpha-pass` | theirs | ours before | ours after |
| --- | ---: | ---: | ---: |
| window strip box `80,200 180x80` (behind the lap counter), mean RGB | 34.4, 36.9, 37.1 | 38.4, 40.2, 39.6 | 40.0, 42.1, 42.2 |
| whole-frame mean grey difference | | 10.049 | 10.079 |

**Worse in its box by 2 grey and in the frame by 0.03**, with the program's term as read: the strip is mostly hidden behind the HUD,
and the pane's normal map, which tilts each reflection, is the named omission. The other HD pairs unchanged, Pulse PSP and Omega
byte-identical.

**Omega: checked, applies, not wired.** Omega's Anulpha ships `MT_Windows_A` (and `_1`, `_2`, under `Materials/` and
`materials_DLC/`); its PS4 programs are not decoded and its `psp2` material route does not run `window_glass`.

### `ds_booth_glass` reflects the sky through its normal map's alpha (2026-10-27, `hd-tech-de-ra-2`, confidence 85)

**What a player sees.** The glass band above the pit-lane arches (Sebenco, Talon's, Modesto) and the booth windows behind the stand
railings (Tech de Ra, Sol 2, Vineta K) drew solid black; the originals show a dim blue-grey glass with the sky's streaks in it. Found by a
slot-skip census of the Tech de Ra pair (`OAG_SKIP_SLOT` bisection on the black strip behind the right-hand railing: slot 389).

**The program.** Both circuits' slots resolve to the lightmapped block #7 (`@0x5f80` on Tech de Ra, `@0x5ca0` on Sebenco Climb; the
vertex block #7 at `@0x5d60` writes `TC0.xyz` and `TC1.xyz` as 0, so no vertex light reaches it), read with `scripts/ps3-microcode.py
fp-file`:

`rgb = fog(light * C + Texture1.a * mix(T * sky, T * luma(sky), T.w)) / 2`, `alpha = 0`, drawn opaque (`Transparency::Opaque`,
factors `0x302`/`0x303` unused),

with `C` the constant `0x7611a2d8` (`0, 0, 0` on every booth the 13 pairs show), `T` `iblScalePower` (`0x7480de6d`, `ProbeTint::SCENERY`),
`sky` `paraboloidReflectionTex` (unit 2) at `reflect(-V, N)` through the dual-paraboloid address, and `Texture1` the tangent normal map at
unit 0 (`ds_glass_panels_ns.gtf`, alpha 0..31 over the whole 128x128, mean 18). The gate is the unit-0 fetch's own `w`: `MUL H0.xyz,
R1.wwww, H0` on Tech de Ra, `MUL H1.xyz, R0.wwww, H5` on Sebenco, a register the `TEX` wrote and no later instruction overwrote. So the
glass is the sky at about 7 % before the store's encode, which reads as the original's dim glass. (The table in `rcsmaterial.md`'s
"near-white pit-wall arches" section read the Sebenco gate as a literal; corrected there.)

**Wired** (`mesh::rcs::window_glass`, row `ds_booth_glass`, mode `Gated`, `tint` `[1, 0, 2]`; `skin`'s `AlphaUse::Gate` builds the
picture as the constant with the map's alpha; `shade.wesl` weights the reflection by `a` when `tint.z` is 2). **Left out, named**: the
normal map's tilt, as on every glass family here. Tests: `hd_sky_reflection_ground_truth::tech_de_ras_booth_glass_reflects_the_sky_through_its_normal_maps_alpha`
(the tag and the picture) and `hd_shadowed_track_redraw::the_booth_glass_reflects_the_sky_by_its_alpha_not_its_complement` (the
shader weights by `a`, not `1 - a`).

Fixed boxes (blur-7 mean abs RGB, luma bias), reproducible from `theirs.png` and `ours.png` alone:

| Pair, box | before | after |
| --- | ---: | ---: |
| `hd/tech-de-ra`, the booth behind the right-hand railing, `1700,355 220x40` | 21.18, -21.61 | 7.01, +4.57 |
| `hd/modesto-heights`, the pit arch glass, `146,360 300x90` | 15.23, -12.93 | 3.42, +3.18 |
| `hd/sebenco-climb`, the pit arch glass, `180,200 300x90` | 22.22, -20.47 | 6.24, -0.45 |

**Measured** on the 13 cold pairs (blur sigma 7, mean absolute RGB, and signed luma `ours - theirs`; "changed" is the pixels the change
moved by more than 4):

| Pair | whole before | after | changed px | changed before | after | luma bias before | after |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| anulpha-pass | 7.81 | 7.82 | 16,665 | 4.0 | 4.9 | -2.1 | +2.5 |
| chenghou-project | 4.62 | 4.63 | 17,930 | 6.7 | 5.6 | -6.8 | +5.6 |
| metropia | 5.73 | 5.73 | 0 | | | | |
| moa-therma | 4.70 | 4.63 | 10,154 | 18.0 | 5.1 | -18.6 | +0.8 |
| modesto-heights | 5.89 | 5.51 | 23,126 | 34.5 | 4.2 | -34.1 | +3.8 |
| sebenco-climb | 4.87 | 4.43 | 31,347 | 31.3 | 6.2 | -30.8 | +1.4 |
| sol-2 | 3.71 | 3.58 | 20,276 | 16.4 | 5.5 | -15.7 | +3.4 |
| talons | 3.78 | 3.56 | 15,144 | 31.3 | 4.4 | -31.2 | +2.5 |
| talons-assegai-c1 | 4.05 | 3.82 | 16,083 | 31.1 | 4.1 | -31.0 | +2.7 |
| tech-de-ra | 4.30 | 4.20 | 28,187 | 13.0 | 6.6 | -12.9 | +4.9 |
| the-amphiseum | 7.00 | 7.00 | 9,714 | 5.2 | 4.5 | -4.7 | +1.9 |
| ubermall | 3.76 | 3.74 | 19,116 | 5.6 | 3.2 | -4.9 | +1.6 |
| vineta-k | 5.22 | 5.11 | 17,991 | 16.9 | 4.3 | -17.6 | +3.1 |
| **mean of 13** | **5.033** | **4.905** | | | | | |

Mean grey difference (the compare page's score) 10 better, Anulpha +0.01 and Amphiseum +0.02. The glass now reads 1 to 5 luma over the
original's: the omitted normal map is the candidate (it tilts each texel's reflection off the vertex normal's).

**2048: checked, differs.** 2048 ships `ds_booth_glass` on its DLC circuits (`DLC1/environments/Anulpha_Pass`, `Chenghou_Project`); the
file declares `Constant1` and `Texture1` and no reflection sampler at all, so its glass is the lit constant (as `gxp.md` already draws
it). **Omega: checked, differs, not wired.** Omega's `DS_booth_Glass*` (Moa Therma, Anulpha, Amphiseum, Tech de Ra) declares
`Texture1`, `Constant1` and `cubeReflectionTex` with `reflectiveBlurCoeffs`: a cube capture, not HD's paraboloid sky, through a program
not decoded; its `psp2` route does not run `window_glass`.

## The chevron panel draws black: the emask and its swept outline are unbound (2026-10-22, `hd-scenery-materials`)

**Claim**: Anulpha Pass's hexagon panel on the left wall (`hd/anulpha-pass`, cold RPCS3 original, 1920x1080 scale 100) draws
black in ours because `cf_chevron_pulse`'s emask and wave terms are not drawn. **True; cause measured; wired 2026-10-22 by
route 2 (`hd-texture-slots`, "Six material textures" below).**

**Where.** Box `180,600 200x80` (the panel, away from the craft): mean absolute grey difference **27.90**, unchanged at the
end of this lane. A slot-skip sweep over that box in a tint render (`OAG_TINT_MATERIALS=1`, `OAG_SKIP_SLOT`) names one slot:
**442**, `cf_chevron_pulse` / `ds_wallchevron_sc` (box change 76.5; nothing else above 0.5). Mean RGB in a 120x40 box inside
the panel (`220,620`): the original `(3, 38, 16)` (a green outline over a near-black fill), ours `(0, 0, 0)`.

**Why it is black.** The program lights the albedo's **alpha** (`slots::LIT_ALPHA`, "The blue cast" above) and
`ds_wallchevron_sc`'s alpha is 0 inside both chevrons; the picture there is the emask's, unit 2 (`ds_wallchevron_emask`, 1024x512
DXT5: RGB a blue hexagon fill, alpha the hexagon outline, both 0 outside the chevrons). Block #7 `@0xf6d0` of
`15_anulpha_pass/materials/cf_chevron_pulse.rcsmaterial` (`scripts/ps3-microcode.py fp-file`):

```text
@0x15  TEX H6, f[TC4] unit2                     e = emask
@0x16  MUL H7.xyz, H6.wwww, [0x02ab9f07]        e.a * Colour          (record: (0, 1, 0))
@0x27  MAD R0.xy, uv, [0x220cf0e6], [0xa6f5352f].yy   uv * k + offset (record: k 0.2, offset 0)
@0x2f  TEX H0.xyz, R0 unit4                     w = ds_wave_c (512x512 DXT1, one bright horizontal band)
@0x37  MUL H7.xyz, H7, H0                       the outline
@0x43  ADD H6.xyz, H4.wwww, H6                  albedo.a + e.rgb
@0x44  MAD H5.xyz, H2, H6, H7                   light * that + outline
@0x49  MAD H2.xyz, H2, H4, H5                   + specular * albedo.rgb
@0x4a  MAD /2 H0.xyz, fog, H2, fogColour
```

**The sweep is authored (confidence 85).** `0xa6f5352f` is `~crc32("offset")` (`scripts/ps3-sho.py hash offset`), a material
parameter the record authors as 0, and **the material's own Edge Animation curve drives it**: one channel, hash `0xa6f5352f`
component 0, sampled through `rcsmodel::material::Curve::sample` at 0, 0.5, 1, 2, 2.9, 3.0, 3.1, 4.5 s as 0, 0.1667, 0.3333,
0.6667, 0.9667, 0, 0.0333, 0.5: `offset = fract(t / 3)`, the same on all eight `cf_chevron_pulse` slots of Anulpha's
`track.vex` (441, 442, 474, 475, 480, 481, 483 and the reversed copies); `chevron_pulse` (445, 477, 482) carries no curve.
`curve_track::UvCurveTrack` reads only `uvOffset`/`uvScale` channels, so this one is dropped today. **Which clock a scenery
curve runs on is not measured** (the gantry board's is, `crates/raceplay/src/gantry/clock.rs`; this is a different material),
so the outline's strength at the pair's instant cannot be predicted yet: the original's outline is at full strength there.

**The blocker: five textures, four bindings.** The program samples albedo (unit 0), normal map (unit 1, `ds_wallchevron_n`),
emask (2), lightmap atlas (3) and wave (4). Ours binds albedo, atlas, the **normal map in the third binding**
(`hull_normal`, which reads only its `xyz`) and has the fourth free. A prototype that put the emask in the third binding drew
the panel (box 27.90 to about 19) but took the normal map from every `cf_chevron_pulse` wall: the left wall's box
`50,800 300x150` went from `(28, 34, 38)` to `(48, 60, 64)` against the original's `(22, 30, 36)`, and the whole frame from
12.62 to 16.15. It was reverted. The prototype's renders all ran with that wrong normal, so they say nothing about the emask
and wave's gamma (their address words were not read; the albedo unit's is 7).

**Routes, for whoever wires it** (one is a renderer decision, so none was taken here):

1. **Pack `emask.a` into the normal map's alpha.** Both are 1024x512 at the same coordinate (`f[TC4]`); the normal is DXT1 with
   no alpha and the shader reads `xyz` only. Decode both to RGBA8 at load, put the result in the third binding, the wave in the
   fourth. `emask.rgb` (the `+ e.rgb` fill) then has no home: draw without it and say so in the load report. It is the dimmer
   term: the original's fill is near black.
2. **A fifth texture binding** in the material group, which every pipeline variant and the bindless arrays carry.
3. **Let the chevron claim the third binding before `hull_normal`**, as the magstrips' emissive picture does; this drops the
   normal map, which the wall measurement above shows is visible.

The outline's phase would ride as an `AnimTrack` that plays the `offset` channel (a vertex `anim` of 0, its row named by the
slot's glow-table entry), with the vertex stage writing `uv * k + offset(t)` into the chunk's unused `lightmap_texcoord.zw`:
the animation table is a vertex-stage binding and the coordinate is linear in `uv`.

**Other titles.** Omega ships `cf_Chevron_Pulse{,_1,_2,_Displacement2Out*}` and `Chevron_Pulse*` (section above); its
programs are GCN and unread: **checked, applies, not wired**. 2048 ships the `ds_wallchevron` textures as `.gxt` (above) and no
RSX program: **not checkable**.

## Six material textures, and the chevron wired (2026-10-22, `hd-texture-slots`)

**Route 2 above, taken.** Group 1 carries six textures: albedo (binding 0), lightmap (2), the third (3, `Model::pad_masks`), the
fourth (4, `Model::wave_maps`), and two new ones, the fifth (5, `Model::fifth_maps`) and the sixth (6, `Model::sixth_maps`), all
through the one sampler (1). A slot that names none binds the white 1x1 `build` already shares, so the bindless arrays gain no
element. The bindless row (`mesh_render::bindless::Row`, `MaterialSlot` in `bindings.wesl`) is seven indices: the four textures,
the sampler, then the fifth and sixth.

**The limit it meets, and the fold that keeps it at 16.** Group 1's six plus group 2's twelve would have been 18 sampled
textures in the per-slot fragment stage, over wgpu's default and WebGPU's guaranteed 16. What each target allows, read off
`wgpu-hal` 30.0.1 and a WebGPU survey:

| Target | Sampled textures per stage |
| --- | --- |
| Vulkan (desktop, Android) | the device's `maxPerStageDescriptorSampledImages`, capped by the set limit over two stages (`vulkan/adapter.rs`) |
| DX12 | the resource-binding tier's SRV count (`dx12/adapter.rs`) |
| Metal | 96 on macOS and Apple6+, 72 on Apple4/5, 23 on older iOS (`metal/adapter.rs`) |
| WebGPU (browser) | 16 or 48, a tier the browser picks: AMD and Qualcomm report 48, Intel 16, ARM both (web3dsurvey) |
| GL, WebGL2 | `MAX_TEXTURE_SLOTS` = 16, a `wgpu-hal` constant (`gles/mod.rs`) |

An 18-texture layout fails validation on any 16 adapter, and then no mesh draws at all. So the Zone stage's Inner and Outer
sets are now **one 2-layer array each** (`mesh_render::zone::layered`): binding 1 holds `zoneModeTrack<n>` at layer 0 and the
swept-out stage's at layer 1, binding 10 the same for `zoneMode<n>`, and bindings 11 and 12 are gone. The arrays are copied
GPU-side from the two uploads (material textures gain `COPY_SRC` for it). An outer of another size or format cannot share
the array; it repeats the inner, which makes the sphere a no-op, and is logged. HD/Fury's 30 stage files are all 87,552
bytes. The per-slot fragment stage binds 16 again (group 1's six, group 2's ten), so `required_limits` keeps wgpu's
default. `material_layout`'s
`the_fragment_stage_binds_no_more_sampled_textures_than_the_default_limit` pins the count. The bindless arrays count against
the binding-array limits, not this one (`wgpu-core` 30, `binding_model.rs`).

**No pixel moved from the bindings or the fold.** All 13 HD pairs, Pulse PSP `amphiseum-black` and an Omega race frame
(`--race`, 300 ticks, 1280x720) are byte-identical before and after the bindings, bindless and `OAG_BINDLESS=0` alike. After
the fold they are byte-identical to the unfolded result with the chevron and the snow wired. Both Pulse PSP Zone pairs
(`pulse-psp-zone-moa-white-z50`, `pulse-psp-zone-talons-white-z8`) are byte-identical to their a1e81dec0 renders. HD's own
Inner/Outer selection is pinned by `zone_recolour`.

**The chevron** (`mesh::rcs::chevron_pulse`): the emask rides the fifth binding and the wave the sixth, so the normal map
keeps the third. `shade.wesl` adds `emask.rgb` to the albedo's alpha inside the light multiply, and `emask.a * Colour *
wave` after it, block #7's `@0x43`/`@0x44`. The wave's coordinate `uv * k + offset` is laid in the vertex stage into the
chunk's unused second pair. `offset` is the material's own curve, played as an `AnimTrack::Param` row named by the glow
entry's `offset`. Eight slots bind (441, 442, 474, 475, 480, 481, 483, 484: the wall's `ds_wallchevron_*` and the floor's
`ds_floorchevron_*`), each with its normal map still third. Pinned by `hd_chevron_pulse_ground_truth` and
`hd_chevron_pulse_lit_path`.

Chosen, not measured:
- **The clock.** It is the scenery's animation clock, the race clock unless `--world-clock-offset` moves it, the clock
  every other HD material curve here runs on. Which clock the original's scenery curves run on is unread.
- **The emask's and the wave's decode.** Both are decoded like every colour unit read live so far (gamma nibble 7 on the
  albedo, the probes and the leach noise). These two units' address words were not read.

**Acceptance on `hd/anulpha-pass`**, mean absolute grey difference (PIL `L`) against the 2026-10-22 re-capture:

| Box | main | wired, at the pair's clock | wired, outline off the panel |
| --- | ---: | ---: | ---: |
| panel `180,600 200x80` | 8.84 | 16.75 | 5.25 |
| inside the panel `220,620 120x40` | 8.32 | 18.95 | 5.62 |
| left wall `50,800 300x150` | 9.12 | 9.12 | 9.12 |

Sweep of `--world-clock-offset` (panel box, every 0.25 s of the 3 s period):

| offset s | 0 | 0.25 | 0.5 | 0.75 | 1.0-2.25 | 2.5 | 2.75 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| box | 16.75 | 10.07 | 9.16 | 6.93 | 5.25 | 33.76 | 44.27 |

The re-capture shows the panel's blue hexagon fill with **no outline**. Ours at the pair's clock draws the outline at full
strength, so the panel box got worse there. A sweep of `--world-clock-offset` over the curve's 3 s period puts the band off
the panel from 1.0 to 2.25 s (42 % of the cycle), where the box is 5.25. So what is left is the phase, which follows from
the clock: the fill and the outline are both drawn. The wall is unchanged because the normal map stayed bound.
**Where the outline is off, the fill matches the original's**: the same blue hexagons on a near-black ground, ours about 5
grey levels brighter (mean RGB `(8, 15, 21)` against `(3, 9, 16)` inside the panel), with a faint rim on the hexagon edges.
Before this, ours drew the panel black. Whole frame:
9.935 to 9.945.

**Not in a Zone race.** Every Zone variant of `cf_chevron_pulse` (fragment blocks #15-#34, #45-#64, #75-#93, those declaring
`zoneColourTint`) declares neither the emask nor the wave, as the magstrips' do not, so both terms are off when
`scene.zone.enabled` is set. Without that gate Anulpha's Zone panel drew the outline (panel green 16 to 37 against main);
with it the Zone frame is main's. `sebenco_snow`'s Zone variants declare all six of its samplers, so the snow keeps its mix.

**Other titles.** Unchanged from the chevron section above: Omega's `cf_Chevron_Pulse*` programs are GCN and unread, so
**checked, applies, not wired**. The fifth and sixth bindings are on every title's pipeline, so wiring it needs only the
GCN program's units. 2048 ships no RSX program for it: **not checkable**.

## Moa Therma's trees are cut off by the glass: the trees write depth while blending (2026-10-22, `hd-scenery-materials`)

**Claim**: the tree crowns left of centre in `hd/moa-therma` are cut off because a blended surface draws over them. **True;
cause measured, mechanism read, wired 2026-10-22 (`hd-tree-depth`, below).**

**Where.** Box `880,360 120x60` (the crowns against the grandstand): mean grey difference **27.05**. The slot-skip sweep
(tint render) names `dc_windows_a` slot 560 (box change 72.4) and three `cf_tree` slots, 557, 550, 548 (17.1, 14.3, 6.4);
skipping 560 alone brings the crowns back in our frame. Our transparent list is drawn in a fixed build-time order (layer,
then file chunk order; `docs/rendering/draw-order.md`), and the tree chunks (299 to 383) come before the glass chunks (458,
703), so the glass paints over the crowns. A camera-distance sort would not fix it: the glass chunk 703 (radius 127) is
317.7 from the pair's eye, between tree chunks at 247.8, 310.5 and 358.0.

**Why the original shows them: the trees write depth.** Their state word is `0xbd`, the glass's `0x39`; `0xbd` adds bit 2
and bit 7. `Material_ApplyRenderState` feeds state bit 2 to `Rsx_SetDepthMask` and, on draws whose pass word has bit 3,
state bit 7 to an alpha-to-coverage path with blending off
([material-state.md](../ghidra/functions/ps3-hdfury-eu/material-state.md), "State bit 2 is depth write, bit 7 is
alpha-to-coverage"). Either way the trees write depth, and the glass behind them fails the depth test.

**The experiment** (a temporary build, not committed): drawing every `_atoc`-textured chunk through our cutout pipeline
(depth write on, alpha tested) brings the crowns back as in the original. Box **27.05 to 17.85**; whole frame 10.32 to
10.30; the other twelve HD pairs move by -0.026 to +0.004. The crowns' edges are harder than the original's.

**Not wired, and why.** The faithful change is renderer-wide: a blended HD draw whose state carries bit 2 writes depth
(335 blended records disc-wide, `hd_state_census`), and a bit-7 draw takes alpha-to-coverage instead of the blend when the
pass asks for it. Two things are unread and decide the picture: which pass word bits are set in the race's scene pass (bit 6
gates the depth mask, bit 3 the coverage path), and how a blend-plus-depth-write tree with no alpha test (`0xbd` has bit 1
clear) looks without MSAA (its clear texels would write depth too, which the original frame does not obviously show).
Next: dump the tree draw's `SET_DEPTH_MASK`, `SET_BLEND_ENABLE` and `SET_ANTI_ALIASING_CONTROL` on a cold Moa Therma frame
(`scripts/hd-draw-dump.py`), then key a depth-writing blend pipeline on state bit 2 (`mesh_render`'s blend pipelines are
built with `depth_write_enabled: false` for every blended draw today).

**Other titles.** The state word is the `.rcsmodel` material record's, and Omega and 2048 read their models through the
same reader; their render-state code is not on an RSX (GCN, GXM) and is unread: **not checkable** for the bit meanings.

**Wired (2026-10-22, `hd-tree-depth`).** A live cold Moa Therma frame
([material-state.md](../ghidra/functions/ps3-hdfury-eu/material-state.md), "The race pass sets pass bits 3, 5 and 6")
answered both open questions: the race pass sets pass bits 3, 5 and 6, so the 31 tree draws are issued **unblended**, alpha
test off, alpha-to-coverage on, depth written, among the opaque draws; and the blended `0x2d`/`0x3d` draws write depth while
the `0x39` glass does not. The scene target has two samples. `oag_title::Looks::material_render_state` (HD `Measured`,
every other title unread) has the track loader call `Model::honour_render_state`: a state-bit-7 draw moves from the blended
list to the cutout list, unblended, at a 0.5 reference that stands in for two-sample coverage on our single-sample scene
(**chosen, not measured**), and a blended draw with bit 2 takes a depth-writing variant of its blend pipeline.

| Pair (re-captured 2026-10-22, whole-frame mean grey) | before | bit 7 | bit 7 + bit 2 |
| --- | --- | --- | --- |
| anulpha-pass | 10.877 | 10.877 | 10.637 |
| chenghou-project | 7.486 | 7.486 | 7.489 |
| metropia | 9.140 | 9.115 | 9.113 |
| moa-therma | 9.853 | 9.825 | 9.826 |
| moa-therma box `880,360 120x60` | 25.62 | 16.70 | 16.70 |
| modesto-heights | 14.242 | 14.244 | 14.232 |
| sebenco-climb | 13.722 | 13.722 | 13.708 |
| sol-2 | 8.534 | 8.534 | 8.534 |
| talons-assegai-c1 | 8.826 | 8.824 | 8.825 |
| talons | 8.461 | 8.458 | 8.459 |
| tech-de-ra | 12.886 | 12.886 | 12.804 |
| the-amphiseum | 10.663 | 10.663 | 10.667 |
| ubermall | 9.823 | 9.823 | 9.818 |
| vineta-k | 17.031 | 17.031 | 17.031 |

(The box read 27.05 against the pair captured before the hull shake was patched out.) Seen as a player: the crowns stand
whole in front of the grandstand glass, in the pair and in moving frames just after GO; their edges are harder than the
original's, which the console's two-sample coverage softens. Tech de Ra's glass road gains the dark seam the original shows.

**Where else it reaches.** Material records per circuit file (blended with bit 2 / bit 7): Moa Therma 11 / 43,
Chenghou 19 / 21, Modesto 35 / 29, Ubermall 49 / 6, Sebenco 27 / 6, Amphiseum 21 / 10, Talon's 14 / 7, Anulpha 13 / 4,
Tech de Ra 11 / 1, Metropia 10 / 8, Vineta K 2 / 5, Sol 2 1 / 5; each reversed variant about the same; the four Zone
circuits 1 to 3 / 0. The reversed variants and the Zone circuits have no pair; the one HD race pair outside `hd/`
(`hd-race/talons-pack`) moved 46.50 to 46.14. **The Zone pass's own pass bits are unread**: the frame above is a race
frame, and the rule applies to a Zone race's model as well (bit 2 only, no Zone circuit authors bit 7).

**Other titles, re-checked.** Omega and 2048 author HD's state word on their own materials (`psp2::material::Material::state`),
and a census of all 16 Omega and 14 2048 circuit models finds `cf_tree` at `0xbd` (Omega also `0xbe`, `0x2bd`; 2048 20
records) and blended states with bit 2 (`0x3d`, `0x7d`, `0x2d`) on both. Their builder
(`mesh::rcs::psp2::transparency::route`) reads bits 0 and 1 only, and their executables' handling of bits 2 and 7 is unread:
**checked, applies, not wired** for both.

## Vineta K's sky was dark because ours tinted the encoded texel (2026-10-22, `hd-tone-vineta`)

**Claim** (the maintainer, from the compare page: "`hd/vineta-k`: the light looks
different, our sky is too dark"): on `hd/vineta-k`, our sky and overall light are within
10 grey of the original's in a sky box and a lit-scenery box, and the cause is fixed from
measured law. **True.** The stage that carried it is the **sky tint**, not the exposure, the
tone map or the sky texel; one lit surface family on the right-hand building is a separate
cause and not tone (below).

### The boxes

The pair `hd/vineta-k` (the originals re-made at 1920x1080, Resolution Scale 100, race
clock 13.8 s), mean RGB, theirs, ours before (`0e678255a`) and ours after:

| Box (x, y, w, h) | Theirs | Ours before | Ours after | Grey, before -> after |
| --- | --- | --- | --- | ---: |
| sky (360, 5, 160, 45) | 206.0, 192.5, 186.9 | 139.3, 130.4, 126.8 | 196.5, 184.0, 178.7 | -63.0 -> **-8.8** |
| sky (1160, 5, 160, 40) | 201.5, 186.2, 172.4 | 136.2, 126.2, 116.8 | 192.0, 177.9, 164.5 | -60.3 -> -8.5 |
| sky (600, 5, 200, 40) | 215.2, 198.8, 193.3 | 145.4, 134.5, 130.5 | 205.3, 189.7, 184.1 | -65.7 -> -9.4 |
| lit scenery, the sunlit rail right of the craft (1150, 470, 250, 50) | 155.5, 130.1, 95.5 | 152.4, 126.9, 93.8 | 150.6, 125.1, 92.4 | -2.6 -> **-4.4** |

The sky was the whole of "too dark": ours over theirs was 0.676, 0.677, 0.678 per channel,
one factor. The lit scenery was already inside 3 grey and moves 1.8 darker after the fix,
because a brighter sky lifts the frame's adapted luminance and with it the bloom gate's fade.

### The decomposition

Our own frames at the pair's pose, the sky box (360, 5, 160, 45), with two temporary
switches (not committed): the HD glow suppressed (`Glow::Suppressed`, the scene times the
exposure, encoded) and the sky's tint forced to 1.

| Ours | Sky box |
| --- | --- |
| as shipped (tint 134/255, glow on) | 139.3, 130.4, 126.8 |
| glow off, tinted | 126.7, 118.5, 115.2 |
| glow off, untinted (the texel times the exposure) | 241.1, 225.6, 219.3 |
| glow on, untinted | 253.5, 242.3, 235.5 |

**Tinted over untinted, glow off, is 0.5255 (R) and 0.525 (G): `134 / 255 = 0.5255`**, in
the encoded output. The live law (renderer.md, "The sky law, read off live draws",
measured on four circuits in 2026-10-08) is `texel * (byte / 255) ^ (1 / 2.2)` in display
terms, 0.746 at 134. So ours applied the authored byte in the display domain. The reason
is in the shader, not the loader: the sky's faces upload raw (`Rgba8Unorm`), its corners
carry `lit` 0.0, and `shade.wesl`'s stand-in path multiplies `texel * colour` on the encoded
texel and decodes to light afterwards. The loader handed it `byte / 255`, the linear-light
value. `0.525 / 0.746 = 0.704` is the predicted shortfall; the box read 0.677 with the bloom
in. The exposure is not the stage: it is 1.0 on both renders (the ratio is exact, which a
moving exposure would break), and the texel itself is not short (untinted, glow off, it
reads 241 against theirs' 206).

**Confidence 95** that ours mis-applied the tint's domain (the ratio, and the code path read).
The law itself keeps its 2026-10-08 evidence.

**The 2026-10-08 port table** in renderer.md ("Port: before and after") read Talon's
Junction at x0.93 of the law after the port, where this measurement says ours sat at
`(128/255)^(1 - 1/2.2) = 0.69` of it. No commit after the port changed the sky's shading
domain (`git log -p fe5313ff3..0e678255a` on `shade.wesl` and `sky_cube.rs`), and the
stand-in path's gamma multiply dates from `acccdfabb` (2026-08-20), so the table did not
measure what it says; how its renders were made is not recorded well enough to say why.

### The fix

`oag_mesh::mesh::sky_cube::build` keeps taking the measured linear-light tint, `byte / 255`,
and writes the cube's corners as `tint ^ (1 / 2.2)`, the factor the encoded texel is
multiplied by. Scoped to the sky's own builder: every other model on the stand-in path
(Pulse's prelit colours, authored in the gamma domain) is unchanged, and the Zone sky and
untinted callers pass `[1.0; 3]`, which the conversion leaves at 1. Tests that fail without
it: `sky_cube`'s `the_vertex_colour_is_the_displayed_factor_the_live_draw_shows` (0.736 at
128, 0.765 at 140, the live draw's numbers, within 0.01) and `hd_sky_tint_ground_truth`
(the loaded cube's corners are `(byte / 255) ^ (1 / 2.2)` on four circuits).

### Second circuit, Talon's Junction (byte 128)

`hd/talons`, a cloud box (1090, 130, 190, 80): theirs 132.3, 159.6, 172.0; ours before
91.1, 108.6, 117.0 (grey -49.1); after 129.4, 155.0, 166.9 (**-4.2**). A higher box
(1100, 20, 180, 40): -47.8 to -1.3. Independent of Vineta: a different byte, a different
archive (`DATA00` against `DATA02`) and a different sky texture.

### The 13 pairs

`scripts/compare-rerender-ours.py --no-page` over a copy of `data/reference/compare-page/hd/*`,
the script's mean grey difference over the whole frame, before (`0e678255a`) and after:

| Pair | Before | After | Change |
| --- | ---: | ---: | ---: |
| `hd/anulpha-pass` | 10.06 | 10.06 | +0.00 |
| `hd/chenghou-project` | 6.76 | 6.76 | +0.00 |
| `hd/metropia` | 8.48 | 8.48 | +0.00 |
| `hd/moa-therma` | 9.27 | 9.27 | +0.00 |
| `hd/modesto-heights` | 13.81 | 10.15 | -3.66 |
| `hd/sebenco-climb` | 8.59 | 8.59 | +0.00 |
| `hd/sol-2` | 7.82 | 7.82 | +0.00 |
| `hd/talons` | 7.93 | 5.72 | -2.21 |
| `hd/talons-assegai-c1` | 8.32 | 6.15 | -2.17 |
| `hd/tech-de-ra` | 12.27 | 12.27 | +0.00 |
| `hd/the-amphiseum` | 10.10 | 10.09 | -0.01 |
| `hd/ubermall` | 9.35 | 9.35 | +0.00 |
| `hd/vineta-k` | 16.46 | 10.93 | -5.53 |

None worse. The pairs that move are the circuits that author `Sky colour` under 255 and show
sky; the 255 circuits are bit-identical (`x ^ (1/2.2)` is 1 at 1). `hd-race/talons-pack`, the
one moving pair, shows no sky and stays at 46.60. A moving frame of ours on Vineta K
(autopilot, 522 km/h, race clock 0.04.2, 1920x1080, read at full size) shows the pale warm
sky the original's grid frame has, where the old build drew it a flat grey; the track box
(800, 500, 300, 200) moved 0.9 grey darker, the whole frame 4.0 brighter.

### What is left, and not tone

- **The sky's residual, -8.5 to -9.4 grey.** The texel and the tint now match the law, so
  this sits in the bloom's share of the sky (the sky's glow mask is its fourth byte, 16 on
  Vineta K) or in the adaptation. The held adaptation patch (encoded against linear, "Decide
  the adaptation law" above) was not tried here: the claim's bound holds without it and the
  question it needs is the console's, not this pair's.
- **The right-hand building's horizontal bands** (box (1500, 280, 350, 40): theirs 67.5,
  59.3, 47.4; ours 98.5, 89.9, 78.6 before and 97.2, 88.7, 77.3 after, +30 grey) are cream
  white and sunlit in ours and dark in the original. Glow off they are still +23, and
  skipping every material matching `lightstrip` removes only the blue strip, not the bands,
  so this is a lit surface family's shading (sun, specular or shadow), not the exposure and
  not an emissive. It is most of what still reads as "the light looks different" on this
  pair. Not investigated further; the next step is a slot mask (`OAG_TINT_MATERIALS=1`,
  `hd_slot_list`) to name the material.

### Omega

**Checked, differs.** Omega's Vineta K (`data01.psarc`) draws its sky as 2048's `SkyCube`
dome (`SkyCube/*.rcsmaterial`, `SkyCube_Vineta_K_*.gnf`), which this engine reads through
`psp2_sky_model`, not the cube this fix touches, and its `track.EnvSettings` authors no `Sky
colour` at all: it has `Sky brightness` 3, `Sky scale` 3.5 and `Sky height offset` 0, whose
consumer is unlocated. So the tint law does not carry over, and Omega's sky brightness is a
separate open question.

## HD's blends combine encoded values: the packer runs before the blend (2026-10-22, `hd-blend-space`)

**Claim**: a blended HD draw is combined with what the scene target already holds in
display (gamma-encoded) values, not in linear light. **True for the reference frames,
confidence 85.**

**The register read.** Every draw into the race's scene target (`SET_SURFACE_COLOR_AOFFSET`
`0xf50000`, format word `0x3148`: colour format 8, `A8R8G8B8`, two-sample) on three
existing RPCS3 frame dumps of race frames (one carries the six `scanlinebillboard`-shaped
draws below, which places it on Talon's Junction; the other two are a hull-brightness and a
start-gantry capture; `scripts/rpcs3_draw_hook.py`'s `draws_of`) carries
`SET_SHADER_PACKER` (`0x1fec`) = 1 and `SET_SHADER_CONTROL` (`0x1d60`) with no 32-bit
exports (low byte 0, or `0x0e` on one draw). The blended ones among them use
`SET_BLEND_EQUATION` `0x80068006` (`FUNC_ADD` on colour and alpha) with, per frame:

| `SET_BLEND_FUNC_SFACTOR` / `DFACTOR` | meaning (colour) | draws (three frames) |
| --- | --- | --- |
| `0x3020302` / `0x3030303` | `SRC_ALPHA`, `ONE_MINUS_SRC_ALPHA` | 52, 42, 48 |
| `0x3020302` / `0x10001` | `SRC_ALPHA`, `ONE` | 44, 9, 19 |
| `0x10001` / `0x10001` | `ONE`, `ONE` | 12, 16, 16 |
| `0x302` / `0x3030303` | `SRC_ALPHA`, `ONE_MINUS_SRC_ALPHA` (alpha `ZERO`) | 19, 0, 12 |
| `0x10001` / `0x3030303` | `ONE`, `ONE_MINUS_SRC_ALPHA` | 6, 1, 1 |

No blended scene draw uses a destination-alpha factor.

**Where the encode sits.** `SET_SHADER_PACKER` is the fragment output packer: the sRGB
encode is applied to the program's colour output **before** the blend unit sees it, and the
blend then adds or mixes the encoded source with the encoded bytes already in the
`A8R8G8B8` target. RPCS3, which made every reference frame, implements exactly that: its
fragment epilogue (`RSXROPEpilogue.glsl`, `_ENABLE_FRAMEBUFFER_SRGB`) applies
`linear_to_srgb` to `col0..col3` in the shader when the packer is non-zero and the program
does not export 32-bit registers (`RSXThread.cpp`), and the attachment is
`VK_FORMAT_B8G8R8A8_UNORM` (`VKGSRender.cpp`), so the fixed-function blend runs on encoded
values. Alpha is not encoded. (The earlier wording "the ROP encodes sRGB on write" in
renderer.md, "`SET_SHADER_PACKER`", is the same register; the encode is per fragment, ahead
of the blend, not after it.) The console's own G70-class RSX is a pre-Direct3D-10 part, and
the inference that it, too, encodes before blending rather than linearising the destination
is not measured here; the confidence is for the reference frames.

So for the race's scene target:

```text
stored' = enc(src.rgb) * sfactor(src.a) + stored * dfactor(src.a)      (encoded, clamped to [0,1] per draw)
```

where ours computed `enc(dec(stored) * dfactor + src.rgb * sfactor)`.

**The discriminating fit, Talon's grill.** `scanlinebillboard` slot 435 is a `ONE`/`ONE`
add whose program output is grey (`overlay * 0.2893 / 2`, renderer.md, "`scanlinebillboard`
is the fourth shape"). An encoded add lifts every channel by the same `enc(src)`, whatever
lies behind; a linear add lifts a dark channel more than a bright one. Ours with the
material skipped (`OAG_SKIP_MATERIAL=scanlinebillboard`) stands in for the destination:

| box | behind (ours, grill skipped) | theirs | theirs minus behind | ours minus behind | corr(lift, behind) R,G,B: theirs / ours |
| --- | --- | --- | --- | --- | --- |
| 790,10 60x200 | 49.9, 101.4, 122.4 | 104.9, 157.7, 180.1 | **+55.0, +56.3, +57.7** | +53.5, +29.6, +24.6 | -0.05, 0.07, 0.04 / -0.48, -0.18, -0.09 |
| 800,130 50x80 | 40.7, 102.4, 124.1 | 101.2, 162.7, 185.2 | **+60.4, +60.3, +61.1** | +56.8, +27.6, +22.9 | 0.07, 0.21, 0.09 / -0.66, -0.30, -0.27 |

The original's lift is grey and uncorrelated with what is behind it; ours, with a grey
linear source of about 0.092 on every channel, is not. For a linear add to give theirs, the
source would have to be 2.6 times bluer than red (0.112 against 0.289 linear in the
smaller box), against a grey program. The size is close but not exact: ours' source
encoded is `+84` on every channel against theirs `+60`, so about half of ours' linear
source would fit. The advert's phase (the scanlines move with the clock) and the program's
`/2` are the candidates for that remainder; the old "the original's add is about 3x the
program's" note in renderer.md was this encode (0.1 linear is 0.35 encoded).

**Other titles.** Pulse renders into an 8-bit gamma target already (no change). Omega's
pixel shaders export fp16 (`v_cvt_pkrtz_f16_f32`) into a float target that its own tone map
reads (`oag_post::omega_tonemap`), so its blends are linear: **checked, differs**. 2048 has
no RSX; its target format is unread: **not checkable** here.

### The holographic taps are decoded on fetch

The same Talon's frame carries six `ONE`/`ONE` scene draws of `scanlinebillboard`'s shape
(draws 434-439 of the dump, fragment programs `0x78f441` to `0x791d81`): unit 0 the advert
(`512x256`, and `128x256` on two, the `banner_02` variant the parameter census names), unit 1
the `128x128` scanline texture, unit 2 the `32x256` second layer, unit 3 the circuit's
`1024x1024` lightmap. **Units 0, 1 and 2 read with gamma nibble 7** (sRGB-decoded on fetch,
`SET_TEXTURE_ADDRESS` bits 20-23), unit 3 with 0 (raw), the same split renderer.md records for
albedo and lightmap units. `holo.wesl` used all three taps raw as if they were light. It now
decodes each tap on the race's linear target before the overlay; the gamma-target path is
unchanged. Confidence 80 for `scanlinebillboard` (one frame, six draws); the three
holographic shapes run the same combine and are decoded the same way by inference, not by a
live read.

### Wired, and measured

`Looks::blends_encoded` (HD `Measured`, every other title `UNREAD`) sets
`oag_post::hd_bloom::Params::store` to `SceneStore::Encoded`. Every pipeline that draws into
the HD scene is built for it and encodes its output (`target_out` in
`crates/shader-check/shaders/target.wesl`: the mesh entry points, the ghost, the seven `fx`
shaders), the glass grab decodes what it reads (`target_in`), and the chain decodes the scene
at its two taps (the first downsample and the resolve), so opaque surfaces round-trip
unchanged (a 400x300 opaque box of `hd/talons` is identical to the decimal). Tests:
`oag-fx::it encoded_blend` (a grey flash lifts `0.2, 0.5, 0.8` by `0.3` each, encoded, and by
`0.3^2.2` stored as computed), `oag-post hd_bloom::tests::an_encoded_scene_resolves_as_the_light_it_encodes`,
`oag-render::it hd_holo_billboard_lit_path::on_the_race_target_the_taps_are_decoded_before_the_overlay`,
`oag-hd effects::tests::hd_blends_encoded_and_an_unread_title_does_not`.

Whole-frame mean grey difference against the cold originals (the compare page's metric,
`magick ... -colorspace Gray -compose difference`), before (main `0fad6e937`), with the encoded
store alone, and with the holo decode as well:

| pair | before | encoded store | + holo decode |
| --- | ---: | ---: | ---: |
| `hd/anulpha-pass` | 10.06 | 10.05 | 10.05 |
| `hd/chenghou-project` | 6.76 | 6.76 | 6.76 |
| `hd/metropia` | 8.48 | 8.43 | 8.43 |
| `hd/moa-therma` | 9.27 | 9.09 | 9.09 |
| `hd/modesto-heights` | 10.15 | 10.16 | 10.16 |
| `hd/sebenco-climb` | 8.59 | 8.21 | 8.21 |
| `hd/sol-2` | 7.82 | 7.72 | 7.72 |
| `hd/talons` | 5.72 | 6.06 | 5.90 |
| `hd/talons-assegai-c1` | 6.15 | 6.41 | 6.26 |
| `hd/tech-de-ra` | 12.27 | 8.77 | 8.77 |
| `hd/the-amphiseum` | 10.09 | 10.51 | 10.51 |
| `hd/ubermall` | 9.35 | 9.34 | 9.34 |
| `hd/vineta-k` | 10.93 | 10.88 | 10.88 |

Tech de Ra's pale haze over the road is gone (the grid reads as the original's); the
blown-white glass and stands on Sebenco, Ubermall and Sol 2 move toward the original. A
Pulse PSP pair (`pulse-psp/amphiseum-white`) and an Omega pair (`omega/anulpha-pass`) render
byte-identical to main.

**Boxes** (mean RGB, 0-255):

| box | theirs | ours before | ours after |
| --- | --- | --- | --- |
| Talon's grill `790,10 60x200` | 104.9, 157.7, 180.1 | 103.4, 131.0, 147.0 | 148.6, 192.2, 206.6 |
| Talon's grill, inner `800,130 50x80` | 101.2, 162.7, 185.2 | 97.6, 130.0, 146.9 | 126.5, 186.8, 208.0 |
| Modesto right banner `1130,412 170x28` | 98.6, 102.0, 98.6 | 156.0, 141.8, 92.8 | 137.4, 130.2, 84.2 |
| Modesto left banner `850,385 80x35` | 102.4, 105.4, 99.0 | 122.3, 116.8, 97.0 | 113.4, 107.8, 82.7 |
| Amphiseum hologram `840,0 240x120` | 54.1, 77.1, 89.8 | 120.9, 151.3, 165.3 | 140.9, 171.7, 183.5 |

The grill's lift is now grey on every channel, as the original's is, and over-sized: the
large box's mean grey difference rises 23.8 to 36.1 while the inner one falls 27.7 to 24.6.
The Modesto banners lose some saturation (right box 35.1 to 24.7) but are not the original's.

### What is left, and why the two worse pairs do not argue against the law

- **Amphiseum (+0.42)** is one surface, **not `hologram.rcsmaterial`** (corrected 2026-10-27):
  the board over the start gantry is `dc_hologramwithstatic.rcsmaterial` (slot 637). The
  2026-10-22 bisect used `OAG_SKIP_MATERIAL=hologram`, a substring needle that also dropped
  `dc_hologramwithstatic`, `dc_hologramwithstatic2` and the `hologramscanlines` texture's
  slot; `OAG_SKIP_MATERIAL=materials/hologram.rcsmaterial` moves nothing in the box. Wired
  since: see "The Amphiseum hologram board" below.

- **Talon's (+0.18) and its Assegai pair (+0.11)**: the grill's source is still about 1.5x
  the original's after the decode. The phase of the scanlines (they move with the clock) and
  the second layer's scroll are the candidates.
- **The Modesto banners at the turn are not a blend-space defect.** A slot bisect on the
  banner's yellow (`R - B` of the right box) finds slot **773**,
  `uvanim_diffuse_emissivealpha.rcsmaterial` with `egx_lcd.gtf`: an LCD material, blended.
  The original draws it as a pale, see-through scanline grid with no logo; ours draws the
  texture's logo, saturated. Its program is unread. Next: read that program's fragment
  block and its units' gamma nibbles on a live Modesto frame.
- **Weapon effects are stronger.** Every additive weapon draw now adds its authored display
  value rather than its linear value. The Bomb's rings at 1.8 s burn through to white where
  they drew yellow (`hd_bomb_blast_ground_truth` re-derived on the new build), and a rocket's
  detonation glare is whiter. There is no original frame of either. The falsifier is an
  RPCS3 capture of the Bomb's rings at about 1.8 s. The fireball's program
  (`slots::BOMB_FIRE` in `shade.wesl`) reads its three taps raw as light, as `holo.wesl`
  did before its units' gamma nibbles were read; the nibbles of its units on a live blast
  are the next read. Three additive sources measured too strong under this law (the grill,
  the hologram, possibly the Bomb) point at the same pattern: HD's unlit programs reading
  taps raw where the console decodes them.
- `oag_post::hd_zoom` reads the chain's quarter-res copy, which is decoded, so it needed no
  change; it is not exercised by any of the 13 pairs.

## Vineta K's warm light pool on the wall top: ours draws a second, lightmap-less copy over it (2026-10-27, `vineta-warm-pool`)

**Claim (the maintainer, from the compare page):** on `hd/vineta-k` the original shows a warm, orange-white pool of light on the
top of the dark wall left of the craft; ours has only a thin orange line. **Result:** the pool is in our data and our slot
392 draws it. A second chunk, slot **633**, covers it with the unlit-by-lightmap navy of the same wall. The original does not
draw that chunk at this camera (confidence 70, one boot, see below). Why it does not is open.

**Size (the pair's `theirs.png` against `ours.png`, blur sigma 7, mean abs RGB):**

| Box (1920x1080 frame) | Pixels | Original mean RGB | Ours mean RGB | Luma gap |
| --- | ---: | --- | --- | ---: |
| pool core `330,515 150x30` | 4,500 | 79.5, 67.1, 56.3 | 21.4, 28.8, 39.4 | +37.7 |
| slot 633's footprint (`0,470` to `801,613`) | 17,254 | err 18.3 | | area x error 0.315 M |
| the same footprint with slot 633 skipped | 17,254 | err 5.2 | | 0.090 M |

With `OAG_SKIP_SLOT=633` the pool core reads 81.7, 69.3, 58.1 against the original's 79.5, 67.1, 56.3. Over the footprint
the skipped render is closer to the original in 15,345 px and worse in 134 px, so the original shows no part of 633 from
this camera. The residual 0.09 M is the pool's own shape and the wall's grain.

**Method, and what each step showed (confidence 95 for the ours-side attribution):**

1. `OAG_TINT_MATERIALS=1` at the pair's camera, then `OAG_SKIP_SLOT` on the dominant slots of the box: skipping 392 darkens the
   box a little (the pool is not slot 392 alone being painted over), skipping **633** brings the pool back. With
   `OAG_ONLY_SLOT=392` the whole wall, its top and the warm pool are drawn; with `OAG_ONLY_SLOT=633` only a thin navy strip along
   the wall's top face is drawn. The thin orange line in ours is slot 392 showing at the strip's edge.
2. Slot 633 is `diffuse_normal_specular_emmissive` with `ds_wall_cs.gtf`, the same material and texture as 392, but its role is
   `lit_default` (no lightmap) where 392's is `lightmap|no_ambient` (`hd_material_probe_dump`). It is one chunk (index 1556 of the
   `.rcsmodel`): 24 vertices, 60 indices, `render_flags` 0 (392's chunks carry bit 0, "track"), world-space bias
   `(11.38, 38.44, -83.27)`. Its `+0x30` hash `debc5798` matches **no** `.vex` node (the neighbouring wall chunk 1038, slot 392,
   hash `35f10d84`, does). It belongs to a small unreferenced cluster, chunks 1554 to 1557 (slots 779, 633/634, 481), all
   `render_flags` 0, 35 to 47 units from the start line.
3. **The original's draw list, one cold boot, the start grid** (`rpcs3-drive.py serve`, `hd-draw-dump.py`, `HOOK_TGT=1920000`):
   666 draws on the scene target. A draw's `c[467]` is its chunk's bias and its index count is the chunk's, and that key finds
   known chunks: slot 392's chunk 1038 (bias `(-48.89, 36.64, -85.16)`, 768 indices) is draw 145 and again draw 672 (a second
   pass with a fourth sampler). No draw has chunk 1556's bias: the nearest scene draw is 17 units away, and no 60-index draw
   sits near it. Falsifier written beforehand: a 60-index draw at that bias binding `ds_wall_cs` would have killed the claim.

**What is not known.**

- *Why* the original skips the chunk. Candidates, none measured: the engine draws only chunks a node reaches plus a set of
  special materials (of 2,081 unreferenced chunks, 30 matched a scene draw in this dump, all glow, billboard, glass or sea
  materials; no `diffuse_normal_specular_emmissive` one); a PVS cell that excludes it; a mode-gated piece (the `+0x06` bits 9 and
  10 table in rcsmodel.md); or a depth test our pass resolves the other way. The speed pads are the counter-example that
  unreferenced chunks *are* drawn (pads.md), so "unreferenced is not drawn" is false as a rule.
- The match key works for draws whose chunk position is a world bias; a node-local chunk is not found by it, so the
  referenced chunks of the frame (14 of 389 matched) say nothing and only the unreferenced count above is usable.
- One boot, one pose (the start grid, a different ship from the pair's `harimau classic`, the account having it locked).
  The chunk is 36 units from the start line and the same area is in view in both, but a second boot and a mid-lap
  pose have not been taken. Confidence in "not drawn there": 70.
- The Tech de Ra glass grid is not this cause (a glass family, `glass_reflect`, not a wall copy); it is not examined here.

**Census of "unreferenced" chunks against seven original draw lists (2026-10-27, `hd-unreferenced-chunks`; supersedes the
"30 of 2,081" count above).** No new boot: the draw lists already dumped for Vineta K, The Amphiseum, Talon's Junction (two
frames), Sebenco Climb, Sol 2 and Tech De Ra were re-listed with `scripts/rsx-draw-list.py <dir> <stem> --const 467 --const 256
--const 257 --const 258 --const 259 --const 465` and joined to each circuit's `.rcsmodel`, `.vex` and `.pvs`.

1. **The `c[467]` key was per chunk, and it is per surface.** A multi-surface chunk draws each surface with that surface's own
   `Mesh::bias` in `c[467]`. Chunk 1038 of Vineta K has five surfaces; draw 144 is its first (bias `-48.14, 35.90, -129.03`)
   and draw 145 its second (`-48.89, 36.64, -85.16`), which the section above read as a bias of the chunk. Chunk 1556 has two
   surfaces (60 + 60 indices) at `(11.38, 38.44, -83.27)` and `(11.44, 38.16, -130.35)`; **neither is in the dump** (tolerance
   0.05), so the absence now rests on both surfaces and on the texture key (no draw binds `ds_wall_cs` near it).
   Confidence in "chunk 1556 is not drawn at the start grid": 80 (one boot, one pose; both keys agree).
2. **The frame's camera and view-projection are in the dump.** In a scene draw `c[465]` is the eye (Vineta K dump:
   `45.63, 40.12, -94.99`, 6 units from the pair's camera) and `c[256..259]` the model-view-projection as four columns (132 distinct values on Vineta K; on a world-baked
   draw such as draw 144 it is the view-projection, and the eye projects to clip `0,0,-1.08,0`; `c[260..263]` is the 75-degree second-target projection). Confidence 85.
3. **The original's single PVS cell is recoverable and the law holds on six circuits.** The cell whose bitmap has the most of a
   frame's matched chunks set: Vineta K cell 104 (or 376) holds 387 of 388, Amphiseum 340/344, Talon's 196/197 and 151/152,
   Sebenco 354/354, Sol 2 438/438, Tech De Ra 175/175. Chunk 1556's bit is set in cells 99 to 108, 375 and 376 (33 of 472 cells),
   so **the PVS does not drop 1556 at the start grid**. Confidence 90.
4. **The frustum does not drop it either.** `Visibility_FrustumCullChunks` boxes a world-space chunk with its render record's
   `+0x20` (centre) and `+0x30` (half extent); chunk 1556's is centre `11.39, 38.41, -106.82`, half extent `35.12, 0.30, 25.68`,
   and projects to ndc `(0.18, 0.08)` of the dump's own view-projection: dead centre of the frame. Confidence 85.
5. **No material, flag or record field separates it.** Orphan chunks (no `Mesh` node names the hash) are the normal state:
   1,557 of 1,811 on Vineta K. Over the six lists, a chunk that passes the cell bitmap and the box test is drawn in
   about 95 % of cases (Vineta K 252 of 272, Amphiseum 187 of 191, Talon's 147 of 153 and 151 of 175, Sebenco 354 of 476, Sol 2
   297 of 317, Tech De Ra 84 of 86; the box test is rough, 0 to 153 matched chunks per list fall outside it, so read these as a
   bound). The family of 1556, `diffuse_normal_specular_emmissive`, is drawn in 8 of 9 orphan candidates. Its
   record is all zero beyond the bounds (flags 0, like the drawn 1554, 1555, 1557 around it), its bounds block entry is
   ordinary (`11.43, 38.19, -106.63, r 43.4`), and `Scene_SubmitVisibleChunks` (`0x003fab50`) appends every surface of
   every chunk whose bit is set to the bucket of `g_MaterialDrawClass[material]`, with no other test.
6. **What remains is a short list of exceptions, not a rule.** Eligible (cell bit set, box inside) orphans with no draw: 4 on Vineta
   K (1556, the glass 628 and the two 15,756-index `lambertzeroalpha` 619 and 636 of the x = 105 booth row), 4 Amphiseum, 8 Talon's
   (two frames), 4 Sebenco, 8 Sol 2, 2 Tech De Ra. Materials: `jd_simplespecular`, `loopmaterial`, `cf_alpha4glow`, `clouds`,
   `uv_anim_diffuse_alpha`, glass, `nr_crowd_bustle`, `diffuse_normal_specular_emmissive`. No shared flag, family, surface count or
   size; several are probably drawn by a path the `c[467]` key does not see (a second view, a conditional pass).

**Refuted as R for 1556:** (a) the cell bitmap (bit set at the original's own cell); (b) the frustum (centred in view);
(c) "only special materials are drawn when unreferenced" (orphans on `jd_simplespecular`, `lambertzeroalpha`,
`diffuse_specular`, `diffuse_normal_specular_emmissive` and 1557 beside it are drawn; the old "30 drawn" came from a key that also
required a draw's index count to equal the surface's, and a draw's `idx` rarely does: on the surfaces with a unique bias, the
list's `idx` equals the file's `index_count` for only 44 of 342 matched on Vineta K, 66 of 275 on Talon's, 94 of 337 on Sebenco,
and 1557 is 168 in the file against 120 in draw 127, 1554 and 1555 72 each against two draws of 36. Each of those chunks has one
submesh with one format word, so no submesh field pairs the short draws; whether the engine draws a sub-range or
`rsx-draw-list.py`'s batch sum undercounts is **open**, and it does not touch the absence of 1556, which is keyed on bias
and texture alone); (d) a render flag
(1554 to 1557 are all flags 0, the first, second and fourth drawn); (e) the chunk's bounds block entry.

**Not found, and the next read.** Rule R for 1556 is open at confidence 30. What is left in the executable: the **consumer
of bucket B1** (`Scene_SortAndLightVisibleChunks`, `0x003fdae8`: a record with flag bits 4 and 5 clear goes to B1, and
[zone-effectsettings-loader.md](../ghidra/functions/ps3-hdfury-eu/zone-effectsettings-loader.md) leaves B1's consumer
unresolved; every flag-0 chunk, 1556 included, passes through it), then the **B4 consumer `FUN_00400a00`** (opaque materials,
`(material + 0x10 & 1) == 0`). A per-record test there that the submit loop does not make is where an absent draw would be
decided. Also open: a second Vineta K boot and a mid-lap pose (not taken; the census above needed none), and the
possibility that the original draws the slab in a later frame (the start grid draws once per dump).

**2048 / Omega: not checkable here.** The finding is an HD chunk of Vineta K's `.rcsmodel` with a draw list from RPCS3; 2048
and Omega ship Vineta K re-cooked with their own mesh-object order and PVS dialect (mesh objects, not chunks), and no 2048
or Omega draw list exists (shadPS4 gives no memory readout).

**Not wired.** Skipping 633 would match this picture and would be tuning to it: the rule has to be the engine's. Ready to
wire once the original's draw rule for such a chunk is known; see the `hd-fidelity` thread.

**2048 / Omega:** not checkable here. The finding names an HD chunk of Vineta K; 2048 and Omega ship Vineta K as a re-cooked
model with their own chunk order, and no draw capture of either exists (Omega's shadPS4 shows no memory readout).

### Resolved: the original culls back faces per triangle, ours culls none (2026-10-27, `hd-chunk-drop-re`)

**The rule R is not a chunk rule.** Reading `Scene_SortAndLightVisibleChunks`, the B4 consumer `FUN_00400a00` and the draw
dispatch `0x005d7590` found no per-record drop ([chunk-cull.md](../ghidra/functions/ps3-hdfury-eu/chunk-cull.md)). The draw
of a described chunk is written by the SPU `EdgeGeom` job with the **surviving** triangles, and a surface whose triangles
all die issues no draw. Material state bit 4 is the cull enable (the draw's `cull` register agrees on 1,531 of 1,531
matched surfaces), the front face is CCW, and chunk 1556's 40 triangles are all clockwise from the start-grid eye. The
"index-count anomaly" above (44 of 342) is the same law: the draw's count is `3 x` the counter-clockwise triangles in the
frustum, matching 1,139 of 1,523 culling surfaces. 29 of the 30 census exceptions have no surviving triangle (18 all
outside the frustum, 8 all clockwise, 3 mixed); the Amphiseum's additive `lightbarrierpanel_new_additive` does not fit
(open). Confidence 85 for 1556, 75 for the count law.

**Scored on three pairs** (a local edit, not committed: the track `Drawable` built with `cull_back = true`, blanket rather
than per material; pairs re-rendered on copies, baseline and edit from the same tree). Blur sigma 7, mean abs RGB:

| Pair | Pool core `330,515 150x30` (original 71.2, 61.2, 52.0) | Footprint `0,470 801x143` error | Full-frame error | rmse | edge |
| --- | --- | --- | ---: | --- | --- |
| `hd/vineta-k` | 21.9, 29.2, 38.6 -> **73.1, 63.4, 54.1** (error 34.3 -> 3.6) | 7.96 -> 5.55 | 4.94 -> 4.75 | 0.0618 -> 0.0598 | 0.422 -> 0.426 |
| `hd/the-amphiseum` | | | 5.91 -> 5.65 | 0.0674 -> 0.0635 | 0.409 -> 0.413 |
| `hd/sebenco-climb` | | | 4.30 -> 4.18 | 0.0639 -> 0.0624 | 0.394 -> 0.397 |

Nothing vanished (the winding convention is wgpu's default, CCW front, `Face::Back`), the pool and the wall top match the
original, and the Amphiseum's dome and the lines over the sky read closer. Pixels more than 2 grey levels better against
worse: 39,674 / 2,674 (Vineta K), 44,833 / 24,921 (Amphiseum), 17,323 / 5,187 (Sebenco); the Amphiseum's worse pixels were
not traced beyond its top band, where the dome and the lines over it read closer to the original.

||||||| 43802148e


## The Amphiseum hologram board

`dc_hologramwithstatic.rcsmaterial` (slot 637 of `amphiseum/track.vex`, `SRC_ALPHA`/`ONE`,
authored pair `770`/`1`; the quad over the start gantry, advert `advert_a`). Its program is
read live and wired (2026-10-27, `hd-hologram-board`):

- **Program** (fragment block #3 `@0x1530` of the material, `scripts/ps3-microcode.py fp-file`;
  vertex block #3 `@0x13a0` writes one pair, `TC4.xy = v[2].xy`, and `TC1.xyz = c[208].x = 0`).
  With `t` the engine clock and `(u, v)` the diffuse pair:
  `colour = fog(advert(u + S t, v) + static(u, v + V t)) / 2`, `alpha = scan(u, v).x * K`.
  `S = 1.0` (`0x31182e0d`), `V = 4.5384` (`0xb6573513`), `K = 0.73802` (`0xeedee991`) are the
  record's own numbers, read, not typed. Units: 0 `dc_scanlines` (`0x05dff912`, 128x128 DXT1),
  1 `advert_a` (`0x3bdc0403`, 512x256 DXT45), 2 `dc_gradient_horizontal` (`0xdba0a35a`,
  128x128 DXT1).
- **Read live** (RPCS3, Amphiseum Time Trial start save, `scripts/hd-draw-dump.py`, draw 700 of
  the dump, fragment program `0x7bd200`): the live program is block #3 with the engine's `time`
  (`79.8857` at the pause), the fog colour (`0.0118, 0.0118, 0.0196`) and density (`0.0015`)
  patched in; blend `SrcAlpha`/`One`, depth test `LEQUAL`, **`SET_TEXTURE_ADDRESS` `0x60710101`
  on units 0, 1 and 2: gamma nibble 7, all three decoded on fetch**, so the mask's red is
  decoded too. Confidence 88 (one frame, one draw).
- **Arithmetic self-check** (ours against the program's own law, not against the original):
  the mean of `decode(scan.r) * K` over the texture is `0.185` and the mean of
  `encode(0.5 (decode(advert) + decode(static)))` is `0.32, 0.49, 0.50`, so the expected lift
  is `+15, +23, +24` (0-255, encoded add); ours lifts the box by `+13.4, +22.8, +23.5` over
  the board skipped (`OAG_SKIP_SLOT=637`).
- **Strength against the original** (the scan mask does not scroll, so its stripes read the
  board's strength): per column of `x 880-1060, y 5-95`, the standard deviation of the
  picture minus its 12-row vertical moving average, mean grey: theirs `18.3`, ours `17.4`,
  the board skipped `14.1` (the scene's own floor), the old stand-in `34.7`. The board's
  own stripe energy, `sqrt(a^2 - 14.1^2)`, is `11.7` theirs against `10.2` ours: **ratio
  0.87**, so the board is not 1.4-2x too strong.
- **Measured** (box `840,0 240x120` of `hd/the-amphiseum`, mean grey 0-255; theirs `54.1, 77.1,
  89.8` = 73.7): before `139.6, 170.3, 181.9` = 163.9 (mean of per-pixel |difference| 90.5);
  after `75.5, 103.5, 112.5` = 97.2 (**26.2**, difference of means +23.5); the whole board
  skipped (`OAG_SKIP_SLOT=637`) `62.1, 80.7, 89.0` = 77.3 (19.0, +3.6). The pair's whole-frame
  mean grey 9.61 to 8.59; the other 17 HD pairs unchanged (`ours.png` identical, `cmp`). **The brief's bar (within 10)
  is not met**, and skipping the board still scores better on the box, because the board's
  strength now matches the original's (stripe ratio above) while the scene behind it is about
  20-25 grey brighter in ours (by subtraction: ours without the board `62, 81, 89` against
  theirs with it `54, 77, 90`); the picture, read by eye, shows the original's
  faint scanline-striped rings. The next lead is the pillar and wall behind it.
- **Wired** as glow-table mode 7 (`PROGRAM_STATIC_HOLOGRAM`, `mesh::rcs::static_hologram`),
  keyed on the family name and the block's declared samplers and parameters, so
  `dc_hologramwithstatic2` (a different program) keeps the generic reading. Pinned by
  `hd_static_hologram_ground_truth` (disc-backed) and `hd_static_hologram_lit_path` (the
  shader, no game content). Ours had added the advert to the scanlines' colour at full
  strength with no alpha mask and no `/2`.
- **The `/2`**: this program multiplies `scene.light.output_scale` into the colour sum
  explicitly. A constant-colour probe through this unlit path gave an unexplained gain of
  about 1.43 on the added value (0.2 to +0.287, 0.4 to +0.57, blend alpha 1); the stripe ratio
  above is the check that the whole chain lands at the original's strength. Why that gain
  exists, and whether it is the grill's 1.5x, is not resolved here.

**Omega and 2048:** Omega ships `dc_hologramwithstatic_1.rcsmaterial` (and `..2_1`) beside
the same Amphiseum layout: **checked, applies in name, not wired** (a PS4 GCN program, unread).
2048's DLC1 ships only `hologram.rcsmaterial` for Amphiseum and Modesto Heights: **checked,
differs** (no `dc_hologramwithstatic`). `hologram.rcsmaterial` itself (slots 604-624 on Amphiseum) adds nothing to the start frame's
box (skipping it changes the frame by 0.04 mean grey) and its program stays unread.

## Tech de Ra's near-left wall (slot 384): the wash is a bitangent sign (2026-10-27, `tech-de-ra-wash`)

**Claim brought to this lane:** "slot 384 on `hd/tech-de-ra` (`diffuse_normal_specular_emmissive`, `ds_sf_cs.gtf`) draws
a yellow wash over the original because our generic path does X where its program does Y."
**Verdict: true, with a different X than the brief supposed.** It is not a light term and not the emissive term:
it is the **normal**. Ours drops the normal map's `ny` on this slot where the original's vertex program keeps it.
Falsifier written first: if the wash were the emissive add, `OAG_ALBEDO_ONLY` plus a zeroed `_ne.a * W` would
remove it; if a light term, a zeroed specular or sun term would. Neither did (below); the sign flip did.

### What the slot draws

Slot 384 (`hd_material_probe_dump` ordinal; `lightmap|no_ambient`, specular exponent 32) resolves to the file's
**fragment block #7 at `0x5860`, vertex block #7 at `0x5640`** (the lightmapped variant with no shadow tap;
`scripts/ps3-microcode.py fp-file|vp-file`). Static read of the fragment program (register trace in the
`pads.md` "Speedup" law, the same shape):

```text
N      = nx*TC5 + ny*TC2 + nz*TC3        _ne.rgb*2-1; TC5 tangent, TC2 bitangent, TC3 vertex normal
lit    = pow(lightmap.rgb, C(0x002c73e8)) * C(0x8670f0be) + lightmap.a * (N.L) * C(0x2dba643d)
spec   = (sat(N.Hhat))^32 * sat(8 N.L) * lightmap.a * sun.w * sun.rgb * diffuse.a
colour = diffuse.rgb * lit + _ne.a * C(0x7611a2d8) + spec
out    = (TC1.xyz + fog(colour)) / 2     @0x4f `ADD /2`; TC0.xyz and TC1.xyz are 0 in this variant's
                                         vertex program (no vertex light), so out = fog(colour) / 2
```

Every term is already in `shade.wesl`'s authored path; the slot's inputs match except `N`.

### The vertex program's bitangent

Vertex block #7 (`0x5640`) decodes the tangent attribute `0xdbe5f417` as `MAD R1, v[2], 2, -1` on **all four
components** and writes `o[TC2].xyz = cross(N0, T) * R1.w` (instructions 6, 12, 21): the bitangent's scale is
**`2w - 1`, so `w = 0` gives -1**, not 0. `shade.wesl`'s tangent branch (`hull_normal(...)`) and `pad.wesl`'s
`pad_normal` use the raw `w` (0 or 1), which leaves a `w = 0` vertex with **no bitangent**, so `ny` never reaches
`N` and the bump loses its second axis: the wall reads smoother and brighter than the original's grain.

**Another compile of the same family reads `w` raw.** `12_sol_2`'s copy of the material (vertex block #7 at
`0x5420`) ends `MUL o[TC0].xyz, B, v[2].wwww`: raw `w`, so `w = 0` is no bitangent. That is the compile
`pads.md` measured ("`w` is 0 on 232 of 232 vertices ... `B` is the zero vector"); it is right **for it**.

**Census** (`just psarc cat` of each circuit's `diffuse_normal_specular_emmissive`, `track_surface` and
`weapon_pads`, every vertex block that reads `0xdbe5f417`, `ps3-microcode.py vp-file`; every block of every file
classified, none left over; one file is uniformly one convention):

| Compile | Circuits (files read) | Tangent blocks | Bitangent scale |
| --- | --- | --- | --- |
| signed | Amphiseum, Modesto Heights, Talon's Junction, Tech de Ra (3 files each: 46 to 72 blocks), Zone 1 to 4 (`weapon_pads`, 28 each) | **all signed, 0 raw** (a full read of the tangent with a negated constant, as a `MAD` or a `MUL` then `ADD`, and no `.wwww`) | `2w - 1` |
| raw | Vineta K, `02_track`, `03_track`, Chenghou, Sebenco Climb, Ubermall, Sol 2, Anulpha Pass (2 files each: 40 to 136 blocks) | **all raw, 0 signed** (`v[tangent].wwww`) | `w` |

So it is a property of the circuit's own material file, and the four original-game circuits plus Zone are the
signed ones. **`pads.md`'s "bitangent is the zero vector" holds for Sol 2 and is wrong for Talon's pads** (it
noted Talon's program was "a different compile" but read only the alpha-gated `MAD`, not this scale).

### The local try (uncommitted shader edit, box and frame scored)

Scratch copy of `hd/tech-de-ra`, re-rendered with the main-tree build plus one local edit: in `shade.wesl`'s tangent
branch, `hull_normal(.., vec4(in.tangent.xyz, in.tangent.w * 2 - 1))` for `pad_ne` slots and track receivers
(`receives_shadow == 2`). Box `480,600 240x120`, blur sigma 7, mean abs RGB difference (the handover's metric):

| | box abs | box luma ours-theirs | slot 384 mask (137,866 px) mean abs | area x error | frame |
| --- | ---: | ---: | ---: | ---: | ---: |
| main today | 13.08 | +10.30 | 6.65 | 0.917 M | 4.205 |
| signed bitangent | **2.74** | **-1.81** | **3.68** | **0.507 M** | 4.205 -> 3.840 |

Box mean RGB 143.1, 128.7, 114.7 -> 131.1, 115.8, 103.3 against the original's 133.2, 117.6, 104.9. Two renders of
the edited tree are byte-identical in score (the render is deterministic; the original frame is one capture).

**Terms ruled out** on the same box (each a local toggle on slot 384, one render): the specular off gives luma
-6.66 (so the specular carries +17 of the box; with the sign fixed its share is whatever the normal change leaves);
`_ne.a * W` off gives R -23, G -13, B 0 (the yellow of the glow is right); unsaturated `N.L` in the sun term and
no vertex light change nothing measurable. `OAG_ALBEDO_ONLY` removes the light and the glow together and so cannot
separate them; the per-term toggles do.

### Other pairs, and why the change is per file

The same edit applied to every circuit, frame mean abs RGB, before to after: Modesto Heights 9.004 -> 7.975,
Talon's 6.011 -> 6.002, Tech de Ra 7.187 -> 6.767, Amphiseum 9.623 -> 9.594 (the four signed circuits, all
better or level); **Vineta K 7.471 -> 8.767 and Sol 2 7.058 -> 9.192 (raw circuits, worse)**. The two regressions
are the census made visible, not a counter-example: applying the sign to a raw compile adds a bitangent the
original does not have. **The wiring must read the sign from the vertex program, never from the circuit name.**
Modesto's changed pixels (18,227 px at x 1578 to 1920, y 435 to 707) go 33.4 -> 6.5; Tech de Ra's wall 24.1 -> 4.8.

### Wired (2026-10-27, `hd-bitangent-sign`)

**Where the sign is read from: each material's resolved vertex block's instruction stream** (confidence 90: the
census read every block of every file and none was left over; the reader reproduces it on the disc).
`oag_rcs::rcsmaterial::vertex::Program::bitangent_is_signed(tangent_hash)`: the block declares the `tangent`
attribute and no instruction reads that input register with a `.wwww` swizzle (the raw compile's
`MUL o[TC0].xyz, B, v[t].wwww`). Never the circuit's name.

- `mesh::rcs::hull_normal::hull_normal` classifies the slots it marks (`TangentFrame::Raw` or `Signed`) beside the
  tangent flag; `Model::material_tangent` is now `Vec<TangentFrame>` (`Absent`, `Raw`, `Signed`), `Surface::tangent`
  likewise. The `pad_ne` slots need no second site (they already carry a tangent through this path).
- `mesh::rcs::baked_tangent` bakes `w` as the scale `(scale + 1) / 2` (signed: `2w - 1`; raw: `w`), and
  `hull_normal.wesl` reads `2w - 1` back, snapping `|s| < 0.01` to 0 so a raw `w = 0` (byte 128) stays no bitangent.
- Tests: `crates/rcs/tests/hd_bitangent_sign_ground_truth.rs` (Tech de Ra and Modesto Heights read signed on every
  tangent-reading block; `12_sol_2` and `01_vineta_k` read raw), `hd_hull_normal_lit_path` (scale -1 tilts against
  `B`).

Result, ours re-rendered on scratch copies of the twelve HD race pairs (frame mean abs RGB against the original;
before is `b222a0ef2`): Tech de Ra 7.474 -> 7.062 (the box `480,600 240x120`, unblurred, 16.16 -> 5.89), Modesto
Heights 9.129 -> 8.085, Talon's 5.993 -> 5.983, Amphiseum 8.767 -> 8.736. The raw circuits are **byte-identical**
in score to three decimals (Vineta K, Sol 2, Anulpha Pass, Chenghou, Metropia, Moa Therma, Sebenco Climb,
Ubermall): the regression the unconditional edit caused is gone. No pair moved by more than 0.6 mean grey.

**Ship hulls:** the same reader over `DATA02`'s `assegai`, `feisar`, `qirex`, `piranha`, `goteki` and `triakis` finds
every tangent-reading block **raw** (0 signed of 42), so `hull_normal.wesl`'s raw-`w` reading on hulls stays right
and nothing changed for them. The other ship directories were not named by this sweep (confidence 70 for "all
hulls raw"). **`pad_normal`** (derived frame, no authored tangent) is untouched: slot 384's chunks carry a tangent
and take the tangent branch, and no chunk without one was checked.

**Not measured here:** the original's live fragment or vertex program on a slot-384 draw (`ps3-fp-live.py`): the
law is a static read of both programs plus the pixel score, so the cause is confidence 80, the census 90 (every
block of every file read).

**Omega and 2048 (CLAUDE.md check):** Omega ships the family in `data00` to `data02`
([rcsmaterial.md](../formats/rcsmaterial.md), "The light-bar family") and 2048 shares its asset lineage (CLAUDE.md,
the 2048/Omega note; not re-listed here);
both compile their shaders again (GNM and Vita GXP), so the RSX reading **does not transfer** and a per-compile
split like this one may or may not exist there. **Checked, not answerable by the reader** (`hd-bitangent-sign`):
`oag_rcs::gxp::usse` classifies instruction words only and reads no operands (`scripts/vita-usse.py` reads some), so
neither the `2w - 1` scale nor a raw `w` can be told on a 2048 shader today, and Omega's GNM shaders have no reader. this
wiring is HD's RSX vertex blocks only, so 2048 and Omega keep the raw `w`. Open: read the
tangent's `.w` use in 2048's vertex GXP with `vita-usse.py`, and Omega's in `eboot.bin`/shadPS4; named in the HD
fidelity thread.
