# Wipeout Pure's sun lens flare

Measured 2026-10-26 (lane `pure-lens-flare-re`) from Pure's `BOOT.BIN` and live on
PPSSPP (software renderer, Vineta K, Time Trial grid). The functions, addresses and
the live dumps are in
[`ghidra/functions/psp-pure-usa/lensflare.md`](../ghidra/functions/psp-pure-usa/lensflare.md).
**Wired 2026-10-26 (lane `pure-lens-flare-wire`)**: see "What our code has". Confidence is per claim, rubric as in
[`confidence-rubric.md`](../reverse-engineering/confidence-rubric.md).

The flare is **ten additive quads on a 2x2 atlas, laid along the line from the sun's
screen position through the screen centre**, faded by the angle to the sun and by how
much of the sun's pixel is hidden behind scenery. It is drawn in 2D after the 3D scene and before the
post passes and the HUD (depth test off), so it overlays the track.

## What the disc authors, what the executable owns

| Item | Source | Confidence |
| --- | --- | ---: |
| Sun **direction** | the `lensflare` node (class `0x390`, `oag_vex::lensflare`): payload row 3 (`+0x30`, three `f32`), **normalised, no parent transform** | 90 |
| Flare **colour** | node header attributes `ColourR`, `ColourG`, `ColourB` (`f32`, name match without case, missing = 0) | 90 |
| Atlas | the circuit's `LensflareComponents.tga`, 64 x 64, 8 bpp, 256-colour palette, identical on all seven circuits | 95 |
| Ring count 10, ring spacing, ring sizes, cell choice, alpha, blend, occlusion | **the executable**, not authored | 90 |

The 80-byte payload is a 4x4 matrix (row-major) then a `u32` (`187` on Vineta K) and 12
more bytes. Only row 3 is read; the word and the tail are not (the importer allocates
`0x44` bytes for the block and reads `+0x30` alone). Rows 0-2 are the identity on Vineta K.

Seven circuits author one (`lensflare` nodes, plus a `*shape_` child that is another
class): Vineta K, Modesto Heights, Chenghou Project, Blue Ridge, Citta Nuova V, Sebenco
Climb, Sol 2. None of Sinucit, the four Classics, the four Zone circuits
(`crates/pure/tests/class_table_ground_truth.rs`):

| Circuit | Direction | Colour |
| --- | --- | --- |
| Vineta K | `(-0.8897, 0.2193, 0.4004)` | `(0.7107, 0.7107, 0.7107)` |
| Modesto Heights | `(-0.7458, 0.4781, -0.4639)` | `(1.0, 0.6, 0.0)` |
| Chenghou Project | `(-0.2286, 0.3591, -0.9049)` | `(0.923, 0.6317, 0.4043)` |
| Blue Ridge | `(-0.7009, 0.6100, -0.3697)` | `(1.0, 0.9457, 0.866)` |
| Citta Nuova V | `(-0.4386, 0.4980, 0.7481)` | `(0.5759, 0.6033, 0.3421)` |
| Sebenco Climb | `(-0.7791, 0.4323, -0.4540)` | `(1.0, 1.0, 1.0)` |
| Sol 2 | `(-0.9413, 0.0790, -0.3283)` | `(0.3967, 0.3967, 0.3967)` |

The earlier note's Vineta K anchor `(-51.65, 12.76, 23.4)` was off by a few tenths; the
file reads `(-51.826, 12.774, 23.323)`, which is what the game holds live.

## The law

Let `d` be the node's direction, `fwd` the camera's forward axis, `S_ndc = (x, y)` the sun's
normalised device coordinates (the direction rotated by the camera, projected; the
camera's translation cancels), and `S = (x * 16/9, y)` that point in the draw's own
screen space: **x in `[-16/9, 16/9]`, y in `[-1, 1]`, y up, one unit is 136 px**
(480 x 272 output).

**1. Visibility angle** (`FxLensflare_UpdateIntensity`, confidence 92, matched to four digits on Vineta K:
`0.4736` computed, `0.47363` read)

```text
c      = dot(d, -fwd)            ; > 0 required, else nothing is drawn
c      = clamp(c, -0.999, 0.999)
angle  = acos(c)                 ; radians
if angle > 1.0: nothing is drawn
intensity = 1.0 - angle          ; 0 .. 0.955
```

`-fwd` is `-(cam[0x40].z, cam[0x50].z, cam[0x60].z)`, the third column of the camera's
3x3, not a row. The sun is visible inside a one-radian cone (57 degrees) and fades
linearly to nothing at its edge. Nothing else fades at the screen edge.

**2. Occlusion** (`Lensflare_QueueOcclusionProbe` + `Lensflare_ReadOcclusion`, confidence 90;
live 12/16 = `0.75`):

```text
(px, py) = the sun's pixel in the 480 x 272 output, y down
if the sun is behind the camera, or px outside (-4, 484), or py outside (-4, 276): occ = 0
else: occ = (count of the 16 depth samples at px+{-3,-1,1,3}, py+{-3,-1,1,3} whose
         16-bit depth word is below 0x3b3) / 16
```

Depth runs 65535 near to 0 far; the sky box reads `23`, a pole `4300`. In our renderer
that is "no scenery in front of the sky at this pixel" for each of the sixteen
points; any equivalent gives `occ` in steps of 1/16. A sample off the 480 x 272 area
does not count. **Unread:** which frame's depth buffer the probe sees (the signal
runs inside the frame; the value used by the draw may be one frame old).

**3. Alpha base**

```text
a0 = occ * intensity              ; 0.75 * 0.4736 = 0.355 on the live frame
```

**4. Ring layout**, with `S` above, `t = |S| / (5/3)`, `fade = 1.0` when `t >= 0.2` else
`1.0 - (0.2 - t) * 5.0` (the rings vanish as the sun reaches screen centre; **ring 0 does not**):

| Ring | Centre | Half-size (square, x and y) | Alpha | Cell |
| ---: | --- | --- | --- | ---: |
| 0 | `S` | `(5/3 - \|S\|) * 1.5 + 0.8` | `a0` (no random) | 0 |
| 1 | `0.8 S` | `0.2 t j * 1.0` | `a0 fade r * colour.a` | 1 |
| 2 | `0.7 S` | `0.2 t j * 1.45 * 2.0` | same | 1 |
| 3 | `0.55 S` | `0.2 t j * 1.45^2 * 0.5` | same | 1 |
| 4 | `0.325 S` | `0.2 t j * 1.45^3 * 1.2` | same | 2 |
| 5 | `-0.0125 S` | `0.2 t j * 1.45^4 * 1.0` | same | 2 |
| 6 | `-0.51875 S` | `0.2 t j * 1.45^5 * 2.0` | same | 3 |
| 7 | `-1.278 S` | `0.2 t j * 1.45^6 * 0.5` | same | 3 |
| 8 | `-2.417 S` | `0.2 t j * 1.45^7 * 1.2` | same | 3 |
| 9 | `-4.126 S` | `0.2 t j * 1.45^8 * 1.0` | same | 3 |

Ring `k >= 1` sits at `(1 - 0.2 * 1.5^(k-1)) S`, size factor `1.45^(k-1)` times the cycle
`[1.0, 2.0, 0.5, 1.2][(k-1) & 3]`. `j` is one random per frame in `[0.98, 1.02]`, `r` one
per ring in `[0.5, 0.9]` (when the engine's flag `DAT_002883fc` is set: `j = 1.0`, `r = 0.7`;
who sets it is unread). Ring 5 sits within `0.0125 |S|` of the screen centre whatever `S` is: **that is the
faint circle the maintainer sees at the start line**.
Confidence 92 on positions (ten live centres matched to three decimals), 88 on sizes (within `j`'s
band on the ten live quads, ring 0 exact), 85 on alpha (ring 0 `0.353` against `0.355`; ring `1..9`
`0.20..0.31`, all in `a0 * [0.5, 0.9]`).

The atlas cell by ring is **`[0, 1, 1, 1, 2, 2, 3, 3, 3, 3]`** (confidence 90 live; the
constructor builds blocks in cell order `0,1,2,3,3,1,2,3,3,1` and sorts them by cell,
the sort's key inferred from its result, 75). Cell `c` is the quad of the 2x2 atlas with
`u0 = (c & 1) / 2`, `v0 = (c >> 1) / 2`, each half wide, texture v down.

**5. Colour.** Ring 0 is `ColourR/G/B`. Each next ring converts the **previous ring's**
colour to HSV with hue in sextants `0..6`, adds `-0.01 * k` (`k` = 1 for ring 1), wraps to
`0..6` and converts back, so hue drifts `-0.005 k (k + 1)` sextants from ring 0 (`-0.45` at ring 9,
about 27 degrees); saturation and value are unchanged, and a grey stays grey. **Measured live on
Modesto Heights** (colour `(1, 0.6, 0)`, hue `0.6` sextants): the ten ring colours read
`G = 0.60, 0.59, 0.57, 0.54, 0.50, 0.45, 0.39, 0.32, 0.24, 0.15`, exactly `0.6 - 0.005 k (k + 1)`
(confidence 90). Vineta K's grey shows no drift (all ten `0.7107`).
Vertex colour is the ring's `RGB` as 8-bit (`0.7107 -> 0xb5`), vertex alpha the ring's alpha.

**6. Draw state** (read from the recorded GE list, confidence 92):

- additive: `out = src.rgb * src.a + dst` (`BLENDMODE 0xA2`, `FIXB 0xFFFFFF`); texture function
  `0x100`: modulate by the vertex colour **with the texture's alpha** (`TFUNC` is never written in the
  recorded list; its value is the register snapshot at the dump's start, word `0xC9000100`; confidence
  85, one frame), so `src.a = texel.a * vertex.a`; linear magnify, mipmap-linear minify on the four authored mips
- depth test off, depth writes off, culling off, lighting off, alpha test off
- one triangle strip of 60 vertices, ten blocks of six, in **memory block order** (not ring
  order); additive, so the order does not change the sum
- queue key `0x4e000000` (the node's occlusion callback `0x4b000001`). **Position in the frame**,
  from one recorded GE list (618 draws, the flare is draw 5 of the 32 after the 586 scenery draws,
  confidence 85): after every scenery draw; then a 16-vertex through-mode sprite draw that
  **samples the framebuffer as its texture** (`TEXADDR 0x88000` = the render target; a post pass, bloom
  or glow, so the flare is bloomed with the scene); two 1,320-vertex through-mode sprite batches;
  then the HUD (pixel-space ortho `0.004 / -0.007`) and nothing 3D. So: scene, **flare**, post passes, HUD

## Live check

Vineta K grid, one frame, the sun at NDC `(-0.4515, 0.4343)`, so `S = (-0.8026, 0.4343)` and
`|S| = 0.9126`, `t = 0.5476`. Ten quad centres read from the vertex buffer
`(x / S.x)`: `1.0, 0.80, 0.70, 0.55, 0.325, -0.0125, -0.518, -1.278, -2.417, -4.126`,
the table's. Ring 0's half-size `1.931` is `(5/3 - 0.9126) * 1.5 + 0.8 = 1.9312`;
ring 1's `0.108` is `0.2 * 0.5476 * 0.986`. Alphas are `0x5a 0x4d 0x4c 0x50 0x4e 0x48 0x3a 0x38 0x4e
0x33` by block. Frame, read by eye: a flare-on frame and a second frame about a second later with the ring
alphas zeroed in the live colour buffer. **The pair is not the same tick** (the race clock ran, so the HUD
digits and the camera edges differ too). Beside those, the difference holds soft glows at the
predicted places: the large halo near the sun's pixel `(262, 154)`, the centre disc at `(484, 272)`,
the ring-7 glow at `(757, 423)`. At grey 0.7 and alpha around 0.3 each ring adds a faint tint, which
is why a still shows it so weakly. This places the glows; it does not measure their brightness (the
GE-state and vertex dumps do).

## What our code has

Wired on Pure on a PSP disc, where a circuit authors a `lensflare` node (seven of them).

- **Data**: `oag_vex::lensflare` reads the node (`classes::V4.lensflare` is `0x390`) and
  `oag_raceplay::load::lens_flare` pairs it with the circuit's own `LensflareComponents.tga`
  (`oag_vex::vex::textures`, by asset path), into `oag_fx::lens_flare::Setup`, carried by
  `scenery_fx::Plan`. A circuit with no node, a node that will not parse or no atlas draws nothing and
  says so in the loader report (`lens flare: ...`). The census is
  `crates/pure/tests/class_table_ground_truth.rs` (`the_light_classes_are_on_every_pure_circuit`).
- **Rule**: `Looks::lens_flare` (Pure, `Platforms::Only(&[Psp])`, `Origin::Measured`); Pulse, HD, 2048 and
  Omega are `Rule::UNREAD` (`crates/game/tests/title_behaviour_data.rs`, `only_pure_on_a_psp_draws_the_sun_lens_flare`).
  The rule is the law and the node is the trigger: Omega's archives also carry `lensflare_*` nodes and must not
  switch Pure's law on.
- **Law**: `oag_fx::lens_flare` (`Placement::new`, `rings`, `ring_colours`, `intensity`), tested against this
  page's live numbers (`lens_flare/tests.rs`): ring centres and sizes of the live check, the Modesto hue list, the
  cone, a sun behind the camera, and the pair's live camera giving `0.4736` and the sun's pixel `(131, 77)`.
- **Draw**: `oag_fx::lens_flare::pipeline` (shader `lens_flare.wesl`), one pass per view in
  `Scene::finish_view`, over the resolved scene, before the post chain and the HUD; placed per view in
  `prepare_view` from that view's unjittered view and projection, clipped to the view's rectangle. The additive
  blend is `psys::BLEND`; `crates/fx/tests/lens_flare_draw.rs` renders it over black and checks an open sun adds light
  and a covered one none.

**One law detail the live frame forced.** The cosine is the sun's component on the camera matrix's third axis **as
the matrix holds it, not renormalised**: Pure's tripod matrix is a shear (`camera.md`), its rows a few parts in a
thousand off unit length, and the original dots the raw row. Renormalised, Vineta K's grid reads `0.5137` instead of
the measured `0.4736` (`0.8649` raw cosine against `0.8848`). The sun's NDC position needs no care: the live
`(-0.4515, 0.4343)` and ours `(-0.4513, 0.4342)` agree, and `(131.7, 77.0)` is the pixel the original probed.

**Chosen, not measured** (each also in the module's doc):

- The random factors fixed at the engine flag's values, `j = 1.0` and `r = 0.7`. Both lie inside the measured bands, so
  the picture is reproducible and does not flicker; whether the original redraws `j` and `r` each frame is unread.
- A pass of its own after the scene pass instead of a draw at queue key `0x4e000000` inside it: the occlusion test
  reads the scene's depth, which a pass cannot sample while it is attached. Cost: mist and the screen flash
  (key `0x4f`) land before the flare here, after it in the original; neither is on Vineta K.
- **Occlusion is a same-frame depth-sample pass, no readback**: the 16 `textureLoad`s run in the flare's vertex stage
  (sample 0 under MSAA), counted when the depth equals the clear and sky value `1.0`; the original counts depth words
  below `0x3b3` and may read the previous frame's. The 2 px grid pitch is scaled by `viewport height / 272`.
- Ring 0's half-size, were it negative (a sun beyond the screen's corner), is clamped to zero.
- The atlas is the base level alone: a version-4 `.vex` authors no mips below it (`oag_vex::vex::textures`), so
  minify is linear where the original samples four.
- Not drawn when the sun's pixel is beyond the 4 px margin (the original's occlusion is then `0`), or off the
  16:9 aspect: ring spacing is in the original's `16/9` screen space and maps to NDC, so it holds on any aspect and
  squares stay square.

**Checked against the original** (PPSSPP software, Vineta K, Feisar Vector, Time Trial; ours posed from the state's own
RAM through our own chase rig):

| Moment | Original | Ours |
| --- | --- | --- |
| Grid, race clock 16.6 s | a hazy bright frame, no distinct ring | masked full-frame mean against the original `-29.6, -26.2, -29.8` (flare off) becomes `-3.3, -4.0, -6.7` (on); the centre patch reads `104` off, `158` on, original `160` |
| 6 s of held thrust, against a wall | a double ring left of the pillar at about `(190, 112)` in the 480 x 272 frame | the same double ring at about `(215, 117)`; the camera is yawed a little differently (the state's hull pose is not ours), brightness `+10.5, +6.2, +0.3` against `-26, -24, -34` off |

The grid frame is the finding: most of the old "ours is darker" gap on circuits with a `lensflare` node was this
halo (ring 0, half-size `1.93` of the screen's half-height, additive at `a0`), not scene or hull lighting.
Multi-view: a two-player split of Vineta K shows each view's own haze inside its own rectangle. MSAA on and off
render the same wash. Modesto Heights' sun is behind the grid camera and draws nothing in the pair, as in the original.

## Cross-title

- **HD, 2048, Omega: checked, differs, not wired.** HD's strings name `LensFlare.cpp`, shaders
  `LensFlare_vp`/`_fp`, `LensflareComponents_256x256.gtf` and `_Rays.gtf`, and the key
  `Lighting.Use Lens Flare` in `.envsettings`; Omega adds H-ray keys and a sun scale
  (`handover` census). So they have a ray texture and a flag, Pure has neither, and HD's track
  files carry no `lensflare` node by name. Pure's law is not assumed for HD; HD's `LensFlare.cpp`
  (`0x002aee58`) is unread.
- **Pulse: absent** (`0x3de` is in the name table, zero nodes, zero strings).
- **Pure's DLC packs and the EU disc:** the same Vineta K node is on the EU disc; the packs are not searched.

## Open

- Whether the depth sample is one frame old (we read the same frame); the sort order of the queue keys themselves (the frame position is measured above).
- The random source and the `DAT_002883fc` flag (we hold `j = 1`, `r = 0.7`).
- A split screen's flares are checked on one capture only; the draw sits in `finish_view`, which every view runs.
- What the node's `lensflare_shape_` child and the payload's word `187` are for (neither is read by the flare).
