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
| `0x00181348` | `BackgroundAnimFury_AllocateTargets` | 75 |
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
faint red at the bottom right is that `0x20/255` tint, at a moment the cloud was mostly
out of view: the tint is applied in linear light and reads far brighter than its value
(below, "The tint is applied in linear light"). **Which row a screen gets** is by name,
2026-10-10: `Update` (`0x0017e4e0`, in the widget's vtable range) compares the name of
the screen showing (`TTY.log`'s `Switching Screen "A" to "B"`, B) against each 0x60-byte row
and falls back to the `default` row `Load` found; a screen that authors no row is
`default`'s. Read live on RPCS3 (item `+0xc4`, the row pointer, at every screen):
`Main Menu`, `Additional` (Options) `1.0`; `Single Player` (Racebox), `Track Creation`,
`Team Selection` and `HUD` `0.12549`. Confidence 90.

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
eight-entry queue of recent paths at `+0xd0..+0xec`. Two debug fields on
`g_FurySettings` come first: `g+0x414` is a forced path *index* (`-1` off), and
`g+0x418` a forced *kind* - `1` morph, `2` dynamic, anything else from `1` up
static, `0` rolls - which is what `scripts/hd-fury-backdrop-break.py` writes (`3`)
through the GDB stub to hold RPCS3 on static paths. Unless the index is forced:

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
right/up basis is built from it with the world up (`FUN_005a2b18`, the vectormath
`lookAt`: `z = normalise(eye - target)`, `x = up x z`, `y = z x x`, translation
`-basis . eye`) and written out as a 4x4 the render uses as `worldView` - after
multiplying it by the current cloud's own `+0x3c` matrix, identity in every file
and untouched at runtime. The projection is `perspective(fovy in degrees * PI/180,
aspect, 0.005, 50)`, aspect `4/3` or `16/9` from the display's wide flag
(`g_video+0x11c`); both matrices are then handed to `0x00182358` in place. **That
call does nothing here, measured**: stopped before and after it on eight RPCS3
frames, the view matrix is bit-identical and the projection is the plain
perspective (`0.61386 / 1.09131` for `85 deg` at `16/9`, `-1.0002 / -0.01` for
`0.005..50`). It reads two floats off the post chain's object (`0x008c3520+8` and
`+0xc`, `0.5` and `0.4` in the image's initialisers, **`0` and `0` at runtime**)
and its VMX body is a transpose, a translation by the first times a basis vector,
and a transpose back - the shape of a stereo eye offset, left unnamed at 40. The
constants slot it feeds is `clip+0x50..+0x80` for `worldView` and `+0x90..+0xc0`
for `proj` (`RadioHead2_Update` copies them to the block `RadioHead2_Upload` reads
at `+0x10` and `+0x50`; the parameter order there is `worldView`, `proj`,
`particleColour`, `spriteSizeMultiplier`, then the seven float4s).

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
fade     = sat(d * dff.x + dff.y)                              dff = depthFadeFactors, 0..1 from near to far
dof      = 1 + clamp((d - dofStart) / dofStrength, 0, 24)      dofFactors = (-dofStart, -1/dofStrength, 24, 1/dofFactor)
fog      = sat((d - fogStart) / fogLength) ^ fogExponent       fogFactors = (-fogStart, -1/fogLength, -fogStart-fogLength, fogExponent)
dist     = sat(0.4 * d)                                        MUL_SAT by the literal -0.4: one past two and a half units
ramp     = sat(4 * (1 - frac(t)) * dist - 3) ^ 6               colourRampFactors2 = (4, -3, 6, 0); never more than one
size     = dof * spriteSizeMultiplier
POS      = proj * (view + corner * size)                       a view-aligned sprite, size in world units
COL0.rgb = particleColour + ramp * colourRamp
COL0.a   = fade * (1 - fog)
```

`sat` is the NV40 saturate flag, bit 26 of the instruction's first dword, on exactly
those four instructions - `MAD_SAT`, `MUL_SAT`, `MAD_SAT`, `MUL_SAT` at 20, 24, 26
and 28. **The page's first reading missed it** (`scripts/ps3-microcode.py` did not
decode the bit until 2026-09-14) and had the ramp as an unbounded sixth power of
`1.6 * d * (1 - frac t) - 3`, which saturates every point past three units white;
RPCS3's own decoder (`Log shader programs`, `VertexProgram11`) renders the same
four lines with `clamp(.., 0.0, 1.0)`, which is what settled it. With the flag the
ramp is a band on `1 - frac(t) > 0.75`, at most one, and the hull is red where the
band is not (`crates/ui/src/backdrop/tests.rs` pins the bound). Every other SHO
block the HD pages cite was re-run with the bit decoded: only the RadioHeads
carry it (`2`-`6` each), `LiveStencilShadow_vp` and the zone and material
programs none, so no other page's arithmetic moves.

and `RadioHead2_fp` is `COL0 * tex(diffuseSampler, uv)`, `COL0` read clamped to
`0..1` by the fragment program. The sprite is **procedural**:
`PointCloud_Construct` (`0x002615b8`) makes two 32x32 six-mip textures and fills each
with `PointCloud_MakeSprite` (`0x00260e20`, `(exp0, exp1, amp0, amp1, curve, texture)`):
per mip `m` of `n`, `f = (m / (n - 1)) ^ curve`, `exp = exp0 + (exp1 - exp0) * f`,
`amp = amp0 + (amp1 - amp0) * f`, and per texel `r = |xy - half| / half` from the
texel's *index*, not its centre - so the 2x2 and 1x1 levels are wholly transparent,
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
equaliser bands at `+0x19c..+0x1a4` that the sound system feeds. RPCS3 reads the
uploaded `particleColour` between `0.040` and `0.21` over two minutes of menu music
(`musicPulse` from `0.23` to `1.2`; the disc authors base `0.7`, factor `1.1`).
**This build has no equaliser tap and plays no menu music yet**, so it sits the
bands at zero - `musicPulse = 0.23`, the quietest RPCS3 showed - rather than at
the base. The ramp's
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

Three full-screen passes follow the particles each frame; `Render` owns three
targets (`BackgroundAnimFury_AllocateTargets`, `0x00181348`, at `+0xfc`: **one at
the display's size and two at half of it** - `1280x720`, `640x360`, `640x360`,
pitch `0x1400`/`0xa00`, read off the objects at runtime - all format `8`, each with
a texture view at `+0x108..+0x110`, the full one's filter word `0x0101` (nearest),
the half ones' `0x0202` (linear)) and a ping-pong index at `+0x118`. `Render`
binds the full view (`+0x108`) to the composite's `texture` (`+0x128`) and the
half view `+0x10c + 4 * pp` to its `waveTexture` (`+0x12c`), so the trail is the
half-size pair sampled up bilinearly and the particles are the full one; which
target the particle submit and the blend pass each land in is read from the
decompile's order only (Open). From the three fragment programs and the constants
`Render` binds:

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

## Runtime verification, 2026-09-14

`scripts/hd-fury-backdrop-break.py` boots the disc in RPCS3 under the PPU
interpreter, walks to the Fury main menu, forces the picker to static paths through
the debug kind field, and on each sample stops `Render` twice - `0x0018438c`, before
the perspective helper, and `0x001843a0`, after `0x00182358` - reading the two
matrices, the clip block, the current cloud's header, the post chain's object and,
on mode 2, the constant block at `RadioHead2_Upload`'s entry, with a screenshot of
the same instant. Eighteen samples over three boots at 1280x720, six static paths
among them; the two tests in `crates/game/tests/hd_fury_backdrop_ground_truth.rs`
hold the numbers. What they settle:

- **The camera is right.** `worldView` for path 7 at frame 421 and path 6 at frame
  372 agree with this build's `lookAt` to `5e-4`, one frame's travel (the clock is
  read a frame later); `fovy` is the path's; `spriteSizeMultiplier` is `pointSize *
  1.42`, the `2.26 - 0.0011667 * 720` of a 720-line display. `0x00182358` changes
  neither matrix. The cloud's header matrix is identity at runtime.
- **The constants are the ones `Update`'s table above says**, to the float:
  `colourRampFactors = (20 - 4 * seconds, 0.08, 0.5, 0)`, `(4, -3, 6, 0)`,
  `depthFade (0.5, -0.5)`, the path's `dof` and `fog` factors, `colourRamp = Particle
  Ramp Colour * 0.46` at 720 lines, `particleColour = 0.541 * 0.7 * musicPulse *
  0.46`.
- **So the white was the shader reading, not a scale.** The four saturates above.
  The page's earlier diagnosis - a `0.4` missing on the eye-to-point distance,
  because scaling view space by `0.4` made the colour census match - is retracted:
  the `0.4` was the program's own `MUL_SAT ... c[201].x`, and the census it matched
  was against a capture of a different mode at a different resolution.

**What still differs, measured on path 6 at frame 372 against
`/tmp/hd-fury-backdrop-break3/05.png`** (both 1280x720, the pulse there `1.12`
against this build's `0.23`, so colour levels are not compared): the hull, its
framing, the band's place and the red-to-yellow ordering all agree. In a sparse
region (rows 440..560, columns 1050..1250) the mean light is the same to a percent
(`13.6, 5.4` against `13.0, 5.2` in 8-bit RGB) but RPCS3 spreads it over dots with
a half-maximum footprint of **37 pixels against 13** - the same energy, ~1.7x the
diameter - and in a dense region that spread overlaps into a saturated `139, 89,
31` mean against `10, 3, 0` here. Same total, wider dots. The half-size trail
targets were the first suspect and are now drawn at half size here, sampled
linear - **which changed nothing measurable**: with the bands at zero the trail's
cap is its `0.02` floor and it carries almost no light, in this build or (between
beats) in RPCS3. So the spread is on the fresh particles themselves, and the
candidates are the sprite's mip selection (the RSX sampler state
`PointCloud_DrawRaw` sets, unread - a larger, softer level would widen a dot
without brightening it), `srcScale` (dropped as one) and the blend pass's quad
grid, which could draw the particles more than once.

## The tint is applied in linear light, and what a screen change does

2026-10-10, RPCS3 on `hdfury-ps3-eu`, Fury style, lane `fury-backdrop-ship-select`.
Reference frames: `data/reference/hd-capture/fury-backdrop-ship-select/`.

**Ship Select's backdrop is not brighter than the table says; it is the `default`
row, and that row reads bright.** Item (`BackgroundAnimFury`) `0x3291a100` in that boot,
rows at `item+0xb8`..`+0xbc` (`default`, `Main Menu`, `Additional`, `Controls Menu`,
`Extras`, `SoundTest`, `Manual Part 1..3`, 0x60 bytes each, the tint's four floats at
`+0x40`), current row at `+0xc4`, `default` at `+0xc8`. `Team Selection`, `Single
Player`, `Track Creation` and the race `HUD` all point at `default`: `0.12549 x3, 1.0`.
`TrackHexSelection` authors no row either, so it is `default`'s by the same rule (not
measured live: no walk reached it).

**The brightness law.** The `default` row's tint was edited live (GDB write of three
floats at `row+0x40`) and the screen grabbed a quarter second after each write, in groups
of five tints so a group sees one cloud. Mean excess brightness over 28 groups (red
channel, 14 with the backdrop in view), as a ratio to tint 1:

| tint | 1 | 1/2 | 1/4 | 1/8 | 1/16 |
| --- | --- | --- | --- | --- | --- |
| measured | 1 | 0.80 | 0.62 | 0.40 | 0.23 |
| literal scale | 1 | 0.5 | 0.25 | 0.125 | 0.0625 |
| sRGB encode of the scaled sum | 1 | 0.73 | 0.53 | 0.39 | 0.28 |

At tint 0 nothing draws; at `0.125` the 99th percentile is still 255. So the composite adds
`tint * source` in **linear light and the buffer encodes it**, as an sRGB render target does.
This build is gamma-space throughout (ADR-0020), so `fs_composite` encodes itself
(`srgb_encode(sum * tint)`) and the blend adds. Confidence 75: the shape and the 0.125 point
are measured, the exact transfer curve is not (the clipped top and a moving cloud limit the
fit; plain sRGB is the nearest standard curve). Exact where the page behind the backdrop is
black, which it is under every Fury page. The practical effect: Main Menu is brighter than it
was (`tint 1` is encoded too, matching the original's white-hot core), and every other page
shows the cloud, where the literal scale left it near black. Test:
`crates/game/tests/hd_fury_ship_select_ground_truth.rs`.

**`BackgroundAnimFury_OnEnable` fires on every screen change, not on a team step.** Polling
the item's pulse block (`+0x1648` frame counter, `+0x164c` frames, `+0x1650` seconds, `+0x1654`
60) at 10 Hz while pressing: `Team Selection` left and right (five presses) change nothing in
it, and the cloud pointer (`+0x224`) moved once in 16 s, on the clip's own clock; each of
`Single Player`, `Track Creation`, `Team Selection` and the race entry reset the counter to 0
and ran it to a new random length (0.704, 0.528, 0.597, 0.391 s measured, inside the
documented 0.2..1.6). So the pulse is a **screen-change** event of 0.2-1.6 s; the cloud
thickening around a team step in the 26 s film is **not** `OnEnable` and its mechanism is
unlocated. Not wired: this build's `Fury` has no screen-change signal, and the pulse's
brightness peak (`0.65 * lerp + 0.7` through `(1 - sin(PI t)) / 2`, `lerp` from the four seeded
randoms) is read at confidence 70 from a decompile that was not followed to the shader
constant. The trigger is now located; the amplitude is the open half.

**Omega:** checked, differs - its skin authors only the scene widget (`BackgroundAnim`,
[menu-backdrop-scene.md](menu-backdrop-scene.md)), no `BackgroundAnimFury`, so none of this
applies; whether the scene filter's output is encoded the same way is not checkable (no PS4
emulator in this project). **Below 720p** (640x360) no Fury cloud draws at all, before and
after this change: the points are under a pixel. Open, outside this lane.

## Open

- The dots' spread: the dots here carry the right light over half the diameter.
  Four leads from `Render`'s tail, read as far as they go on 2026-09-14 and no
  further: (1) `PointCloud_DrawRaw` sets no sampler state of its own, and the
  sprite texture's filter word is `0x0206` (mag linear, min trilinear) from
  `PointCloud_Construct` - so not the sampler. (2) In the steady-state path the
  particle submit (before `FUN_006781f8`) comes before any
  set-target call (`FUN_00677c48` -> `0x005a40f8`: it sets
  `target[0]`, later `target[1 + pp]`), so the fresh particles may draw onto the
  page directly and `target[0]` be the full-size accumulation - which a moving
  camera would smear into wider dots. (3) `FUN_00678268` -> `0x003cd038` is a
  FunkLayer copy: it sets `target[1 + pp]`, binds `view[0]` and draws one quad
  under `BlendFunc(CONSTANT_COLOR, ONE)` with the screen tint as the constant,
  so the half targets receive the tinted full one, not the particles. (4) The
  thirty-vertex grid is drawn after the second set-target and before the
  `texture`/`waveTexture` binds. Which program each of the three quads runs
  is the reading that orders them; the render-target contents cannot be read
  through the stub (rpcs3-debugger.md), so it is a Ghidra read.
- Modes `5`, `9`, `10`, `11`: the vertex programs are paired and disassemble; their
  `Update` functions and the two extra streams (`targetPosition`, the line/quad
  offsets) are unread.
- The blend pass's quad grid and the exact target ping-pong.
- The equaliser: what feeds `+0x19c..+0x1a4` and `+0x1dc..+0x1e4`, and whether the
  menu's own music track is what it analyses.
- `0x00182358`'s purpose: inert here with its two inputs at zero, shaped like a
  stereo eye offset, unnamed.
- The HD-style `BackgroundAnim_Item.cpp` (`FEBackgroundAnim_vp`/`_fp`/`Copy_fp`, the
  `.vex` ring with `blur`/`use_bands`), which answers hd-frontend.md's open question
  about which non-Fury program the HD style resolves to: its own, not a RadioHead.
