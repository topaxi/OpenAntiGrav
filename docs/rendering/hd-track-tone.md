# HD's track tone: the floor gap is the reference's exposure, and HD asks for no anisotropy

2026-10-21, `hd-track-tone`. Two claims were brought to this lane; both are
answered here with the numbers they rest on. **Nothing in the renderer
changed.** What landed is two reproducible probes
(`crates/render/examples/hd_slot_inspect.rs`,
`hd_lightmap_flip_census.rs`) and this page.

| Claim | Verdict |
| --- | --- |
| **A**: one missing or misapplied input of the lit track material makes HD's track too dark and too blue, and one cause explains the floor, the blue areas and the hexagon panel | **Three parts.** *Too dark*: **false**; the floor is not short of a material input (before the resolve it matches the original within 3 %), the gap is the original's exposure resolve, which at the profile's `Resolution Scale 150` is an emulator artefact. *Too blue*: **true in ours, cause open**, and it is not the exposure: the left wall is cyan before any resolve, and the blue floor on the right is an animated plasma pulse. *One cause*: **false**; the hexagon panel is a third thing |
| **B**: HD's floor, wall and glass-road draws request anisotropic filtering, and with which filter and LOD bias | **The profile's `off` matches what HD requests**: max anisotropy field 0 on every bind of two circuits. The filter is trilinear with a `-1.5` LOD bias on material textures: **true as the request** (2026-10-21, `hd-lod-bias`), **wired but held off**, because ours already matches the stored frames' sharpness without it and the bias makes every pair worse (see "The LOD bias, wired and held") |

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
  shimmers more. **Wired and held off (2026-10-21, `hd-lod-bias`)**, see "The LOD
  bias, wired and held" below.
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
3. ~~**The wall's cyan cast** (`cf_chevron_pulse` and the wall slots, pre-resolve)~~
   **closed below (2026-10-21, `hd-blue-tone`)**: the wall programs light the
   albedo's alpha. **The pulse's colour and phase** (the chevron's emissive
   mask, `emask.a * colour * wave`) and its `+ emask.rgb` diffuse term stay
   open and are not lighting.
4. ~~**HD's `-1.5` LOD bias** on material textures~~ **wired, held off, below**.

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
