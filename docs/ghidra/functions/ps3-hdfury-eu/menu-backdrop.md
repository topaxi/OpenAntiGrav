# The Fury menu backdrop: a point cloud of a ship, flown past, trailed, and tinted

2026-09-14. The red-and-gold field behind Fury's main menu is not a movie and not the
`FrontEndScene_HD_ATG.vex` ring: it is a **second widget**, `BackgroundAnimFury_Item.cpp`,
which draws one of nineteen `Data/FE/Fury/*.points2` point clouds - a ship hull sampled
to ~55,000 points - as camera-space sprites along an authored camera path, through
thirteen GPU "effect modes" named `RadioHead0`..`RadioHead12` after the LIDAR video the
look is borrowed from, and composites the result with a feedback trail and a per-screen
tint. Every number the look depends on is either in `Data/fe/fury.envsettings` (plain
text) or in this binary, and this page is where each one was read.

Read [memory.md](memory.md) first for the per-function TOC defect. The widget's own
functions are under `0x0026_0000` and read `exact` with TOC `0x008ad4d8`; the point
renderer at `0x0025f000`-`0x00279000` too. Where Ghidra's stack-slot reading of a TOC
load (`iStack_f4 + -0x11f8`) was found to be off by one slot against
`scripts/ps3-toc.py resolve`, the resolver's answer was used and the field mapping
below was re-derived from the render's own use of the field (the attribute slot a
`glVertex`-style call names, the sampler a `bind texture` call names).

## The names

| Address | Name | Confidence |
| --- | --- | --- |
| `0x00186e08` | `BackgroundAnimFury_Construct` | 85 |
| `0x00186220` | `BackgroundAnimFury_StaticInit` | 85 |
| `0x00186b90` | `BackgroundAnimFury_ParseScreenSetting` | 80 |
| `0x00182a78` | `BackgroundAnimFury_Load` | 82 |
| `0x00183c88` | `BackgroundAnimFury_Render` | 78 |
| `0x0017e810` | `BackgroundAnimFury_OnEnable` | 70 |
| `0x00180a60` | `BackgroundAnimFury_PickPath` | 82 |
| `0x0017ea80` | `FurySettings_Construct` | 82 |
| `0x001876f8` | `FuryStaticPath_Bind` | 82 |
| `0x00187400` | `FuryMorphPath_Bind` | 78 |
| `0x00187010` | `FuryDynamicPath_Bind` | 78 |
| `0x00187cb0` | `FuryStaticPath_Camera` | 75 |
| `0x00695e80` | `FuryPath_Duration` | 80 |
| `0x00695e88` | `FuryStaticPath_Dof` | 78 |
| `0x00695ea8` | `FuryStaticPath_Fog` | 78 |
| `0x00695ec8` | `FuryStaticPath_Ramp` | 72 |
| `0x00695ef8` | `FuryStaticPath_Fovy` | 80 |
| `0x00695f00` | `FuryStaticPath_PointSize` | 80 |
| `0x00695f08` | `FuryStaticPath_EffectMode` | 80 |
| `0x0025f098` | `PointCloud_ModeNeedsNewCloud` | 78 |
| `0x0025f1d8` | `PointCloud_Prepare` | 75 |
| `0x00263aa0` | `PointCloud_Submit` | 78 |
| `0x0025fa88` | `PointCloud_DrawRaw` | 75 |
| `0x0025fd60` | `PointCloud_Draw` | 75 |
| `0x0026d858` | `RadioHead2_Update` | 82 |
| `0x0026e598` | `RadioHead2_Upload` | 80 |
| `0x0026d708` | `RadioHead2_Register` | 85 |
| `0x0026db78` | `RadioHead2_Resolve` | 80 |
| `0x003f27f8` | `Points2_Load` | 78 |
| `0x002615b8` | `PointCloud_Construct` | 80 |
| `0x00260e20` | `PointCloud_MakeSprite` | 78 |
| `0x006774b8` | `ShaderRegistry_RegisterPair` | 80 |
| `0x0099f8a0` | `g_FurySettings` (data) | 82 |
| `0x009209b4` | `g_FuryCloudNames` (data) | 85 |

`FuryStaticPath_Ramp` is 72 because the three values it hands out are hard-coded
defaults (`10.0`, `0.2`, `0.5`) with no envsettings key and no other reader; what they
do is known only through `RadioHead2_Update`'s use of them.

## Class attribution

| `.cpp` tag | Address | Constructors via `attrib` |
| --- | --- | --- |
| `BackgroundAnimFury_Item.cpp` | `0x00788858` | `0x00186920`, `0x00186e08` |
| `BackgroundAnim_Item.cpp` | `0x007882a0` | `0x0017d2b0`, `0x0017d350` |

Both widgets are authored in `DATA00`'s `skin.xml` `Top FE Screen`, `<BackgroundAnim
name="bgAnim">` (the HD style: the `.vex` ring under `UseModelCamera`, with `blur` and
`use_bands` screen settings - a separate widget this page does not read) and
`<BackgroundAnimFury name="bgAnimFury" startenabled="false">`, enabled by the Fury
style. The Fury widget's `<ScreenSetting>` rows carry `tint` and `equaliser`: `Main
Menu`, `Additional`, `Controls Menu` and `Extras` are `tint="0xFFFFFFFF"`; `default`,
`SoundTest`, `Controls`, `Race Records` and the five manual pages are `0xFF202020`.
The capture of the OPTIONS page (`hd-menu-style-toggle/01-before.png`) showing only a
faint red at the bottom right is that `0x20/255` tint.

## The shader pairs, and the registration route the census could not see

[renderer.md](renderer.md#the-accounting-closes-exactly-and-one-part-of-it-does-not)
left 62 program names unpaired with their `SHO` blobs because their registrar is not
`ShaderRegistry_Register`'s constructor. It is `ShaderRegistry_RegisterPair`
(`0x006774b8`, `(slot, name, blob)`), called from each subsystem's static init with
both loaded from the TOC. Reading `r4` and `r5` at every `bl 0x006774b8` in the
registrar pairs them; the route is the same for all 62.

| Program | `SHO` block | Registered by |
| --- | --- | --- |
| `FEBackgroundAnimFury_vp` / `_fp` | `0x00936700` / `0x00936000` | `BackgroundAnimFury_StaticInit` |
| `FEBackgroundAnimFuryBlend_vp` / `_fp` | `0x00936600` / `0x00935e80` | same |
| `FEBackgroundAnimFuryWave_vp` / `_fp` | `0x00936500` / `0x00935d00` | same |
| `RadioHead0_vp` / `_fp` | `0x00935700` / `0x0092fd80` | `0x00265648` |
| `RadioHead2_vp` / `_fp` | `0x00933780` / `0x0092fb00` | `RadioHead2_Register` |
| `RadioHead5_point_vp` / `_line_vp` / `_quad_vp` | `0x00932200` / `0x00932580` / `0x00931b00` | `0x00270690` |
| `RadioHead5_fp` / `_texture_fp` | `0x0092f980` / `0x0092f900` | same |
| `RadioHead9_vp` / `_fp` | `0x0092fe80` / `0x0092f680` | `0x00276f58` |
| `RadioHead10_vp` / `_fp` | `0x00935000` / `0x0092fd00` | `0x00267d38` |
| `RadioHead11_vp` / `_fp` | `0x00934880` / `0x0092fc80` | `0x00269d20` |
| `SPU_RadioHead_vp` / `_fp` | `0x0092fe00` / `0x0092f600` | `0x00278f18` |

`scripts/ps3-microcode.py vp 0x00933780` disassembles any of them. Two things that
tool learned on these blocks: a vertex program's **literal constants** sit in the
program sub-header (`+0x14` count, registers, then float4s on the next 16-byte
boundary - `literal c[202] = (1, 2, 0, -1)` for `RadioHead2_vp`), and the NV40
**condition-code predicate** bits, without which `MOV R3.z, R0.z` / `MOV R3.z, 1.0`
read as the second line overwriting the first when it is `MOV R3.z (NE.x), 1.0`.

Every parameter and sampler hash in these blocks resolves to a name in the string pool
beside the program names (`0x0079a170`-`0x0079b1c8` for the RadioHeads,
`0x007886e8`-`0x00788810` for the FE passes), by `~crc32`: `position` `normal` `rand`
`quadOffset` `worldView` `proj` `particleColour` `spriteSizeMultiplier`
`colourRampFactors` `colourRampFactors2` `colourRamp` `depthFadeFactors` `dofFactors`
`fogFactors` `musicMultiplier` `diffuseSampler` for mode 2; `texture` `waveTexture`
for the final pass; `srcTexture` `dstTexture` `srcScale` `dstScale` `srcMax`
`uv0ScaleBias` for the blend; `colourScale` `colourBias` `biasFactor` for the wave.

## `.points2` - the cloud

`Points2_Load` reads the first `0x80` bytes of the file verbatim as the cloud object,
then `count * stride` bytes of records into video memory. Measured on all 19 files
(`crates/rcs/tests/points2_ground_truth.rs`):

```text
+0x00  u32   0
+0x04  u32   count            51,336..55,000 (detonator smallest, seven at 55,000)
+0x08  u32   stride           16
+0x0c  u32   0
+0x10  f32*3 bounding-box min
+0x1c  f32*3 bounding-box max
+0x28  u32   data offset      0x80
+0x2c  u32   0
+0x30  f32*3 0 0 0
+0x3c  4x4   identity, rows at +0x3c +0x4c +0x5c +0x6c
+0x80  records
```

A record, read off `PointCloud_DrawRaw`'s three vertex-array bindings (`SF x3 @0`,
`UB x4 @6`, `UB x4 @10`, stride 16) and confirmed against the header's box:

```text
+0   f16*3  position          inside the f32 bounding box to within one f16 ulp (0.004)
+6   u8*4   rand              0..=254 on every file, no structure, bound normalised
+10  u8*3   normal            v/255*2-1, unit length within the byte quantisation on all but 46 points
                               (four files; `127 127 127`, the zero vector's nearest encoding)
+13  u8*3   0
```

The shader reads `rand` as `v[2].xyzw` and uses `.x` only in mode 2. Nineteen clouds ship: one `_c1` per team
(twelve, plus `detonator`), and `_n1` variants for seven of them. `Load` picks
**twelve distinct files at random** into the widget's cloud table (`+0x1f4`, a
19-entry used-set at `g+0x16bc` so no file repeats), then two distinct table slots as
the previous and current cloud (`+0x224`, `+0x228`). Which ship the menu shows is
random per boot; which one comes next is `rand() % 12` avoiding the current. The
nineteen names are the table `Load` reads through TOC slot `-0x11c8`
(`scripts/ps3-toc.py resolve 0x00182a78 -0x11c8` gives `0x009209b4`,
`g_FuryCloudNames`): `data/FE/Fury/AG_Systems_c1.points2` .. `Triakis_n1.points2`,
forward-slashed and mixed-case where the image names beside them are backslashed -
`oag_hd::frontend::names::FURY_CLOUDS` carries them as spelled.

## `fury.envsettings` - the paths and the colours

`FurySettings_Construct` builds the tables `Load` fills from
`Data/fe/fury.envsettings` (`oag_tables::envsettings` reads the syntax already; the
file is 436 `"key"=values` lines). The layout, from the constructor and the three
`*Path_Bind` functions that register each key against a field:

| Table | Count | Stride | Fields registered |
| --- | --- | --- | --- |
| `staticPaths[i]` at `g+0x4f0` | 8 | `0xa0` | `pointSize +0x24` `fovy +0x28` `duration +0x2c` `dofStart +0x30` `dofStrength +0x34` `dofFactor +0x38` `fogStart +0x3c` `fogLength +0x40` `fogExponent +0x44` `start +0x60` `end +0x70` `focusStart +0x80` `focusEnd +0x90` |
| `morphPaths[i]` at `g+0x9f0` | 8 | `0xa0` | the same plus `effectMode` (default 9) |
| `dynamicPaths[i]` at `g+0xef0` | 4 | `0x1f0` | `effectMode` (default 5) and `sections[0..4]` of the static fields each |

Top-level keys, with the constructor's defaults and what the disc authors:

| Key | Field | Default | Authored |
| --- | --- | --- | --- |
| `Particle Colour` | `g+0x430` | `0.045³` | `0.541 0.024 0.0` |
| `Particle Ramp Colour` | `g+0x440` | `1 1 7` | `11.76 3.09 0.23` |
| `Feedback` | `g+0x450` | `0.65` | `0.647³` |
| `Equaliser Damp Speed` | `g+0x460` | `0.02` | `0.02` |
| `Equaliser/Feedback multiplier` | `g+0x464` | `0` | `0` |
| `Feedback Zoom` | `g+0x468` | `0` | `0` |
| `Music Pulse Base` | `g+0x4dc` | `0.6` | `0.7` |
| `Music Pulse factor` | `g+0x4e0` | `1.2` | `1.1` |

Not authored, hard-coded in the constructor and read by the passes: wave
`colourScale` `g+0x470 = (0.94, 0.94, 0.955)`, `colourBias` `g+0x480 = -0.0003³`,
`biasFactor` `g+0x490 = 8`; the blend cap's clamp `g+0x4a0 = (0.02, 0.02, 0.023)`
to `g+0x4b0 = 0.15³` with gain `g+0x4c0 = 0.016³` and `g+0x4d0 = 24`; the
projection's near and far `0.005` and `50` (`FUN_006781e8`'s arguments in `Render`).

Per-path effect modes as authored: every static path is mode **2** (its
`EffectMode` getter returns the literal); the morph paths are `9 9 9 9 10 10 11 11`;
the dynamic paths `5 5 5 9`. Durations 6-10 s for static and morph, 5-14 s per
dynamic section.

## `BackgroundAnimFury_PickPath` - which path, which cloud

Called by `Render` when the running clip's frame reaches its frame count, with the
eight-entry queue of recent paths at `+0xd0..+0xec`. Unless the debug `Force Path`
`g+0x414` is set:

1. `type = rand() % 3`: `0` static (index `rand() % 8`), `1` morph (`% 8`),
   `2` dynamic (`% 4`).
2. Rejected, up to 64 tries, if the path's duration is `<= 1e-4` or the path is
   in the recent-eight queue.
3. The clip object (`+0x1640`, `0x150` bytes, the parameter block every mode's
   `Update` reads) takes `[0] = effectMode` if it is one of `{0, 2, 5, 9, 10, 11}`,
   else `2`; `[2] = 0` (frame); `[5] = 60.0` (fps, `TOC-0x12e4`); `[4] = duration`;
   `[6] = duration * 60` (frames).
4. If `PointCloud_ModeNeedsNewCloud(mode)` - true for `{5, 9, 10, 11}` - the previous
   cloud becomes the current one and a new current is `table[rand() % 12]`, redrawn
   until it differs.

`Render` counts `clip[2]` up once per frame while the widget is enabled and the
game is not paused, and `PointCloud_Prepare` writes `clip+0x20 = frame / fps`
(seconds) and, for mode 2, `clip+0x1c = fmod(3 * frame / frames, 3)`.

## `FuryStaticPath_Camera` - the camera

`t = seconds / duration`, clamped so a zero duration reads as `1e-4`. The eye is
`start + (end - start) * t`, the target `focusStart + (focusEnd - focusStart) * t`,
both straight lerps; the forward is `target - eye` normalised (the VMX
reciprocal-square-root with one Newton step, guarded against a zero length), and a
right/up basis is built from it with the world up and written out as a 4x4 the
render uses as `worldView`. The projection is `perspective(fovy in degrees * PI/180,
aspect, 0.005, 50)`, aspect `4/3` or `16/9` from the display's wide flag
(`g_video+0x11c`), then handed through `0x00182358` before the clip's `+0x90` slot.

## `RadioHead2` - what a static path draws

Mode 2 draws the current cloud as **quads** (the mode's bit in `PointCloud_Submit`'s
`0x1fd1` mask), four vertices per point, with the attribute streams `position`
(record `+0`), `normal` (`+10`), `rand` (`+6`) and `quadOffset` (a per-vertex half2
corner stream the renderer generates), additively blended (`BlendFunc(SRC_ALPHA, ONE)`,
`FUNC_ADD`, depth test and cull off) into the particle target. The vertex program,
`scripts/ps3-microcode.py vp 0x00933780` read line by line with the parameter names
above and the literals `c[201] = (-0.4, 0.5, 1, 0)`, `c[202] = (1, 2, 0, -1)`:

```text
corner   = quadOffset * (1, 2) + (0, -1)                       -1..1 on both axes
uv       = quadOffset * (0.5, 1) + (0.5, 0)
r        = rand.x * 2 - 1
p        = position + normal * musicMultiplier
view     = worldView * p                                       view-space position
t        = ((position.z - crf.x) + r * crf.z) * crf.y          crf = colourRampFactors; the attribute's own z, before the displacement
d        = -view.z                                             distance down the view axis
if view.z < fogFactors.z: view.z = 1                           beyond the fog end: behind the eye, culled
fade     = d * dff.x + dff.y                                   dff = depthFadeFactors, 0..1 from near to far
dof      = 1 + clamp((d - dofStart) / dofStrength, 0, 24)      dofFactors = (-dofStart, -1/dofStrength, 24, 1/dofFactor)
fog      = ((d - fogStart) / fogLength) ^ fogExponent          fogFactors = (-fogStart, -1/fogLength, -fogStart-fogLength, fogExponent)
ramp     = max(0, 4 * (1 - frac(t)) * 0.4 * d - 3) ^ 6         colourRampFactors2 = (4, -3, 6, 0); LG2 of a negative is -inf, so 0
size     = dof * spriteSizeMultiplier
POS      = proj * (view + corner * size)                       a view-aligned sprite, size in world units
COL0.rgb = particleColour + ramp * colourRamp
COL0.a   = fade * (1 - fog)
```

and `RadioHead2_fp` is `COL0 * tex(diffuseSampler, uv)`. The sprite is **procedural**:
`PointCloud_Construct` (`0x002615b8`) makes two 32x32 six-mip textures and fills each
with `PointCloud_MakeSprite` (`0x00260e20`, `(exp0, exp1, amp0, amp1, curve, texture)`):
per mip `m` of `n`, `f = (m / (n - 1)) ^ curve`, `exp = exp0 + (exp1 - exp0) * f`,
`amp = amp0 + (amp1 - amp0) * f`, and per texel `r = |xy - centre| / half`,
`alpha = 255 * amp * (1 - min(r, 1)) ^ exp`, RGB white. The first texture, the one
every quad mode binds, is `(0.6, 2.0, 0.07, 1.0, 1.0)`: a sprite that is dim and soft
when it covers many pixels (mip 0) and bright and tight when it is a dot. The second,
`(5.0, 5.0, 1.5, 0.5, 0.5)`, is modes 8 and 12's. `quadOffset` is the shared
four-corner stream `(-1,0) (1,0) (1,1) (-1,1)` (`0x0025f848`), which the program
above turns into corners on `-1..1` and uvs on `0..1`.

`RadioHead2_Update` fills those constants from the clip `C` every frame:

| Constant | Value |
| --- | --- |
| `particleColour` | `C+0x110` = `Particle Colour * brightness * musicPulse * resScale` |
| `colourRamp` | `C+0x120` = `Particle Ramp Colour * resScale` |
| `spriteSizeMultiplier` | `C+0x24` = `pointSize * clamp(2.26 - 0.0011667 * height, 0.5, 2.5)` - `1.0` at 1080 lines |
| `colourRampFactors` | `(0.2 * 100 - 0.2 * 200 * seconds / 10, 0.08, 0.5, 0)` = `(20 - 4 * seconds, 0.08, 0.5, 0)` |
| `colourRampFactors2` | `(4, -3, 6, 0)` literal |
| `depthFadeFactors` | `(1 / (3 - 1), 1 / (1 - 3), 0, 0)`: fade from `C+0x28 = 1.0` to `C+0x2c = 3.0` |
| `dofFactors` | `(-dofStart, -1 / dofStrength, 24, 1 / dofFactor)` |
| `fogFactors` | `(-fogStart, -1 / fogLength, -fogStart - fogLength, fogExponent)` |
| `musicMultiplier` | `C+0x13c`, written `0` by `Render` before the submit |

where `resScale = clamp(0.0015 * height - 0.62, 0, 4)` (`1.0` at 1080 lines),
`brightness` is `0.7` at rest (a short random pulse on enable - `OnEnable` seeds four
`rand() * 2^-29 - 1` values and a `0.2..1.6` s duration, `Render` runs
`0.65 * lerp + 0.7` through a `(1 - sin(PI * t)) / 2` ease while it lasts), and
`musicPulse = 1 + (mean(eq[3]) - Music Pulse Base) * Music Pulse factor` from three
equaliser bands at `+0x19c..+0x1a4` that the sound system feeds. **This build has no
equaliser tap**, so it sits the bands at the base: `musicPulse = 1`. The ramp's
`0.2` and `10.0` are `FuryStaticPath_Ramp`'s hard-coded values (`+0x4c`, `+0x48`).

So the visible motion on a static path is the camera's lerp, the sprites growing with
distance past `dofStart`, and a band of ramp colour sweeping down the hull's `z` at 4
units a second with a 12.5-unit period and half a unit of per-point jitter. The
morph modes (`9`, `10`, `11`: `targetPosition`, `targetToSource`, `animationFactors`,
`center`, `time`) and the line mode (`5`: `lineOffset`, `fuziness`, `focusPoint`,
`trailLength*`) are **not read on this page** - their vertex programs are listed above
and their `Update`s sit beside `RadioHead2_Update` in the jump table at `0x008b2124`
(`PointCloud_Prepare`) and `0x008b2164` (`PointCloud_Submit`).

## The post passes, as far as they are read

Three full-screen passes follow the particles each frame; `Render` owns several
1080p targets (`0x00181348` at `+0xfc`, sized from the display) and a ping-pong index
at `+0x118`. From the three fragment programs and the constants `Render` binds:

- **Blend** (`FEBackgroundAnimFuryBlend`): `out = min(src * srcScale, srcMax) + dst *
  dstScale`, `dstScale = Feedback`, `dst` sampled through `uv0ScaleBias` (`Feedback
  Zoom`, authored `0`, so identity), `src` the fresh particle target and `dst` the
  previous accumulation - a trail that decays by `0.647` a frame. `srcScale` and
  `srcMax` are equaliser-driven: `clamp(0.016 * 24 * eq, (0.02, 0.02, 0.023), 0.15)`
  with the same base-level stand-in as `musicPulse`.
- **Wave** (`FEBackgroundAnimFuryWave`): `h = tex * colourScale; out = h +
  saturate(h * biasFactor) * colourBias + colourBias` - the accumulation dimmed by
  `0.94`/`0.955` and pulled down by `0.0003` so trails die out rather than settle.
- **Fury** (`FEBackgroundAnimFury`), to the page: `out = (texture + waveTexture) *
  (257/256) - 1/257` with `BlendFunc(CONSTANT_COLOR, ONE)` and the blend colour the
  screen's `tint` byte replicated - so the backdrop is the bright fresh particles plus
  their softened trail, scaled by the tint, added over the cleared page.

**Confidence 60 on the pass order and which target feeds which**: the three formulas
are read from microcode (90), the tint mechanism from the blend-constant call and the
skin (85), but the ~30 immediate-mode quad vertices `Render` emits for the blend pass
(a grid, `FUN_00678218` at `0x00185??`) and the target indexing (`+0xfc + 4 * pp`)
were not followed vertex by vertex. Open below.

## What shipped, 2026-09-14

- `oag_rcs::points2` parses the cloud (`crates/rcs/src/points2.rs`), with
  `crates/rcs/tests/points2_ground_truth.rs` over all nineteen files: layout words,
  count against length, positions inside the box to a half ulp, unit normals but for
  46 counted degenerates, zero padding.
- `oag_tables::fury_backdrop` is the typed `fury.envsettings` over
  `oag_tables::envsettings`, defaults from `FurySettings_Construct`;
  `crates/tables/tests/fury_backdrop_ground_truth.rs` pins the modes `2`, `9 9 9 9 10
  10 11 11`, `5 5 5 9` and the colours.
- `oag_ui::backdrop::Fury` is the picker, the clip clock and the camera, off
  `oag_core::rng::Rng`; `Tints` reads the `<ScreenSetting>` rows.
  `oag_hd::frontend::names::FURY_CLOUDS` carries the name table.
- `oag_game::render::backdrop` (`backdrop.wgsl`) draws mode 2 - the vertex
  arithmetic above written from the pseudocode, the procedural sprite, the additive
  pass, then Blend, Wave and the tinted composite into three viewport-sized
  `Rgba8Unorm` targets - where the draw list carries `Draw::FuryBackdrop`.
- `oag_game::boot::fury` loads twelve clouds and reports; static paths only, morph
  and dynamic re-rolled, no equaliser, a fixed seed. `--menu-page main` on the Fury
  disc draws it; `--anim-seconds` runs the clip in.

**It does not yet look like the original, and the numbers say how.** On
`data/reference/hd-main-menu-screenshot-2/00.png` (RPCS3, Fury main menu), over the
region rows 200..960 by columns 160..1760 at 1080p, the backdrop is 66% black, 9.3%
red, 2.7% orange-yellow and **0.4%** white; the same census on this build's first
frame is 51% black, 0% red, **8% white and 35% grey** (white at low alpha). The
cause is traced to the reading rather than the code: the ramp `max(0, 1.6 * d *
(1 - frac(t)) - 3) ^ 6` exceeds one for most points at the distances the static
paths author (`d` of 4-20), so `o[COL0]` saturates white (the NV clamp on colour
outputs is applied in the WGSL), and at those distances the hull overflows the
frame - `staticPaths[4]` even ends its eye at `(0, 0.3, -2.5)`, inside the hull's
own box.

One diagnostic, run and reverted the same day: scaling the view-space position by
`0.4` uniformly (every point `0.4` times as far from the eye, projection untouched)
brings the census to 69% black, 4.1% red, 1.8% yellow, 0.6% white, 6.9% grey and
the hull into frame with the red band along its top edge where the reference has
it. Scaling the *cloud* by `0.4` about the origin with the eye left where the path
puts it does not - 87% black, 0% red, 9.8% white - so what is missing acts on the
**eye-to-point distance**, not on the field of view or the cloud's own size. It is
not the vertex program: its bits, literals and the `Update` constants were re-read
against `scripts/ps3-microcode.py`. `PointCloud_Draw` applies no model matrix, so
the candidates are the `worldView` `Render` hands the program and the projection's
own path through `0x00182358`, the one step on it this page has not read. Until one
of them is, what draws is the arithmetic as it stands, not a tuned stand-in.

## Open

- The missing scale on the eye-to-point distance, above - the first thing to read:
  `Render`'s `worldView` bind and `0x00182358`.

- Modes `5`, `9`, `10`, `11`: the vertex programs are paired and disassemble; their
  `Update` functions and the two extra streams (`targetPosition`, the line/quad
  offsets) are unread.
- The blend pass's quad grid and the exact target ping-pong.
- The equaliser: what feeds `+0x19c..+0x1a4` and `+0x1dc..+0x1e4`, and whether the
  menu's own music track is what it analyses.
- `0x00182358`'s effect on the projection matrix.
- The HD-style `BackgroundAnim_Item.cpp` (`FEBackgroundAnim_vp`/`_fp`/`Copy_fp`, the
  `.vex` ring with `blur`/`use_bands`), which answers hd-frontend.md's open question
  about which non-Fury program the HD style resolves to: its own, not a RadioHead.
