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
| `0x003b8550` | `FunkLayer_DrawScanBand` | 85 |
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
(`musicPulse` from `0.23` to `1.2`, and to the law's own `1.33` on a longer sample: see
"The frame, measured"; the disc authors base `0.7`, factor `1.1`).
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
  previous accumulation - a trail that decays by `0.647` a frame. **`srcScale` is the
  screen's tint and `srcMax` is 1**, measured below; the page's first reading had them
  equaliser-driven.
- **Wave** (`FEBackgroundAnimFuryWave`): `h = tex * colourScale; out = h +
  saturate(h * biasFactor) * colourBias + colourBias` - the accumulation dimmed by
  `0.94`/`0.955` and pulled down by `0.0003` so trails die out rather than settle.
- **Fury** (`FEBackgroundAnimFury`): `out = (texture + waveTexture) * (257/256) - 1/257`,
  with blending **off**. The first reading gave it `BlendFunc(CONSTANT_COLOR, ONE)` under
  the tint; that call belongs to the pass before the wave, and its colour is not the tint.

The pass order and which target feeds which were 60 on this page until 2026-11-05; the
next section replaces that reading with a measured one.

## The frame, measured (2026-11-05)

Lane `fury-0251-verify`, checking a third party's notes from a static recompilation of the
02.51 update. Everything in this section is **our own capture of 02.00** (the disc,
`APP_VER=02.00`) on RPCS3 0.0.42-19980: one frame's command stream dumped with
`HOOK_RING_ONLY=1 HOOK_MIN=30 scripts/hd-draw-dump.py` and listed with
`scripts/rsx-pass-events.py`, the patched fragment programs read back out of local memory
at their `SET_SHADER_PROGRAM` offsets and disassembled with `scripts/ps3-fp-live.py`.
Main Menu at 1280x720, Fury style: **51 draws**, then the next frame. The sender's count and
their order agree except where this section says so.

| # | Target | Program | Blend | What it does |
| --- | --- | --- | --- | --- |
| 1 | particle field, `0x00f50000`, surface `0x3148`, pitch `0x2800` | the mode's | `SRC_ALPHA, ONE` | One `QUADS` draw, four vertices a point (214,296 on this cloud; 220,000 on a 55,000-point one) |
| 2 | accumulation, 1280x720 | Blend | off | `min(field * tint, 1) + self_last_frame * 0.647` |
| 3 | half target A, 640x360 | copy (`TEX; MOV`) | `CONSTANT_COLOR, ONE`, colour = glow gain | Adds accumulation times the gain onto A, which holds last frame's wave |
| 4 | half target B | Wave | off | Four quads meeting at a moving pivot (below); the two half targets swap each frame |
| 5 | particle field | Fury | off | `(accumulation + B) * 257/256 - 1/257`, overwriting the field |
| 6 | particle field | band | off | The smear band, reads its own target |
| 7 | particle field | colour | `ZERO, SRC_COLOR` | Scanlines: multiplies every 4th row by `(0.05, 0.1, 0.25)` |
| 8 | `0x00cc0000`, plain | copy | off | Resolves the field to a plain target. 720p only; at 1080p the ladder reads the field directly |
| 9-24 | the shared post ladder down to 160x90 | | | The same chain as a race, and on the menu more than a luminance readback: see "Three more readings" below |
| 25 | the display buffer | final composite | off | Decode, exposure, bloom; [renderer.md](renderer.md). Reads the field half a texel off on both axes |
| 26-51 | the display buffer | UI | `SRC_ALPHA, ONE_MINUS_SRC_ALPHA` | The front end |

**Confidence 90** on the table: read off the stream at every step, twice (two frames per
dump) and again at 1920x1080. A second build's agreement would not lift it past 94.

- **The whole chain runs in linear light.** Method `0x1fec` is 1 on draws 1 to 25 and 0 on
  the UI, and every texture those passes bind has the gamma bits `0x00700000` set in its
  address word (`0x60730303`). That is why
  [the tint reads the way it does](#the-tint-is-applied-in-linear-light-and-what-a-screen-change-does)
  and it replaces the guess there about where the encoding happens. **Confidence 80**: the
  bits are read directly; `0x1fec` as "encode on write" is inferred from its on/off split and
  the sender's agreement, not from a register document.
- **The tint reaches the shader as a fragment constant, not as a blend colour.** The
  constant at slot 2 of the Blend program (`0x00740600` at 720p) is patched per frame.
  Experiment: three floats written to the current row at `row+0x40`, the program read back
  half a second later. Main Menu: `1.0` before, `0.37` after writing `0.37`. Campaign
  Selection (`default` row): `0.12549` before, `0.37` after. Both restored afterwards. The
  sender's reading is right and this page's was wrong. **Confidence 92.**
- **The blend colour is not `5/255` as a constant; it is the glow gain.** It read `0x050505`
  on every Main Menu and Campaign Selection dump but one, and `0x262626` on the other, with
  no relation to the tint (`0.37` written: `0x26` on one dump, `0x05` on the next). `5` and
  `38` are the floor and ceiling of the clamp at `0x00184f00`
  (`0.02 * 255`, `0.15 * 255`): `g+0x4a0`, `g+0x4b0`, gain `g+0x4c0`, `g+0x4d0`, fed by the
  mean of the second band set `+0x1dc..+0x1e4`, one byte replicated into all four channels
  for `FUN_00678258`. So `g+0x4a0`'s `0.023` in blue never reaches the screen. The sender's
  `5/255` is the floor; **confidence 85** on floor and ceiling, **70** on the shape in
  between, which was seen at only those two values and read from a VMX decompile.
- **The particle field is antialiased at 720p and plain at 1080p.** Surface format `0x3148`
  is `A8R8G8B8`, `Z24S8`, antialias 3 (2x1), pitch doubled to `0x2800`. At 1920x1080 the same
  pass draws to `0x148`, pitch `0x2000`, no antialias, no resolve (draw 8 above is gone).
  **Confidence 90**, both read off the stream. Particles and passes otherwise match.
- **The wave's grid is a pinch.** Sixteen vertices, four quads meeting at one point. Corners
  sample `0.003..0.997` across and `0.005..1.005` down, so the outer edge is pulled in by
  `0.3` percent and everything drifts by `0.5` percent down the screen; the shared point's
  uv is its own screen position, so nothing moves there. The point itself wanders
  (`-0.177, -0.135` and `0.047, 0.036` in two dumps). Confidence 85
  on the vertices; what drives it is under "Three more readings".
- **The smear band and scanlines are one shared function**, `FunkLayer_DrawScanBand`
  (`0x003b8550`), called once, from `Render` at `0x00185a24` through the stub at `0x00678278`. Its statics at `0x008c2bfc`: speed `0.6`, half
  height `0.2`.
  - The band is two quads, each `0.2` clip tall, colour `1` at the outer edges to `0.3`
    across the middle, `w` running with it. The program samples the target at
    `x + (1 - w)^10 * -0.01` and multiplies by the colour.
  - It moves down by `0.01` clip a frame (`0.99381` then `0.9838` on consecutive frames) and
    wraps on `fmod(.., 2)`, so a 200-frame period. The argument is a double read from a
    global timer object (`+0x18`), not the clip's frame counter, which agrees with the
    third party's "not locked to the section clocks".
  - Scanlines are one quad per fourth row: 180 at 720 lines, 270 at 1080 (1,080 vertices
    counted). `c[467]` carries the colour for that draw only.
  **Confidence 90.**
- **The 40 percent fade is not there.** The sender places a fade with `c[467] = (0, 0, 0,
  0.4)` between the composite and the band. No such constant appears in any of 20 dumped
  frames here (ten dumps of two frames: Main Menu, Campaign Selection; static mode `2`, morph
  `9`, `10`, `11`, at 720p and 1080p; a dynamic path was forced but its mode `5` could not be
  told apart from static in the dump), nor a draw that could be it, and no `0.4` float is loaded in `Render` or
  the band function in **either** 02.00 or 02.51 (every `r2`-relative float in both ranges
  scanned). Confidence 75 that it does not exist on 02.00; 02.51 could not be run far enough
  to look (see [update-0251.md](update-0251.md)). A `(0,0,0,0.4)` written straight into a
  constant would escape that scan, which is why this is not higher.
- **Mode 11's extra draws are real but the count moves.** With the path forced (`g+0x414 =
  6`, `g+0x418 = 1`): the field copied down `640 -> 320 -> 160 -> 80 -> 40`, pitch `0x1000`,
  then 250 quads on the field from the 640 level. The sender counts about 500; 250 is one
  frame of one section. At 1920x1080 one frame of it draws 501 such quads, chain `960 -> 60`, so the count
  moves with the resolution too and the sender's "about 500" may be a 1080p figure. `FUN_00678298`, gated on
  `*(g+0x16b0)`, is the call. Modes `2`, `9` and `10` add no draws. Confidence 85.

**Three more readings, 2026-11-05, lane `fury-backdrop-pass-graph`** (same disc, RPCS3, Main Menu and
Campaign Selection at 1920x1080 and window 1920x1080; forced static path 6; the game paused through the
stub, the state read, the display given two seconds, then the frame grabbed. A `import` of the running
game shows a frame about 100 clip frames older than the counter beside it, which cost a wrong diagnosis
first).

- **What moves the wave's pivot is the cloud and the bands.** `Render` at `0x00184fc0`..`0x001851bc`:
  the current cloud's bounding-box centre (`(min + max) * 0.5`, `+0x10`..`+0x24`) through the view and
  projection, divided by `w`, plus `(2 * band - 1) * 0.05` per axis from `+0x1e8`, `+0x1ec`, then clamped to
  `-1..1`. The literals are TOC `-4456` (`2.0`), `-4452` (`0.05`), `-4448` (`-1.0`). `+0x1b4..+0x1f0` is
  sixteen bands, the same sixteen the `equaliser` debug loop at the end of `Render` draws. Checked on 20
  dumped frames of ten dumps: the pivot stays inside `0.05` of a cloud's projected centre on both
  axes, largest residual `0.0435`. The check picks the best of nineteen boxes, so it is weaker than it
  looks; the disassembly is the evidence. Wave pinch and drift are `g+0x4d4 = 0.003`, `g+0x4d8 = 0.005`,
  read live. Pivot uv is `(x + 1) / 2`, `(1 - y + 0.005) / 2`, as the vertices show. **Confidence 85**; the
  earlier "40 on the equaliser" line above is this.
- **The glow gain's shape is a threshold.** `dVar46` is `g+0x4d0` (`24`) only while the band mean is above
  `g+0x1708`, and `1` otherwise, so `0.016 * 24 * eq` or `0.016 * eq`: with the clamp, nearly always the floor
  or the ceiling, which is why the dumps show `05` or `26` and nothing between. `g+0x1708` read `0.646`; what
  moves it is unread. With no equaliser the gain is `5/255`. **Confidence 80.**
- **The field reaches the screen half a texel off, and the game asks for it.** Draw 25's immediate vertices at
  1080p carry uv `(0.00026, 0.000463)` at the top left and `(1.00026, 1.000463)` at the bottom right:
  `0.5 / 1920` and `0.5 / 1080`, on both axes, with unit 0's filter `0x2022000`, linear, and the window's pixel
  centre word `0`. So this is the game's quad, not an emulator's convention, and each display pixel is four
  field texels averaged in linear light. On screen, eight paused frames: rows `0` and `3` of every four dim and
  equal, `1` and `2` bright and equal, ratio `0.68` to `0.77`; frame 484 reads `46.7, 67.1, 66.9, 46.3`. The mean of
  one and `0.05` in linear light encodes to `0.7`. **Confidence 90.** Every frame that ends in this composite gets
  it, race frames included if their quad is the same; only the menu's was read.
- **On the menu the ladder is not a luminance readback alone.** Read live at 1080p: the gate `0x01111fc0` is
  `rgb * a * 0.75` with its from-frame contribution at `0`, so it blooms from the **field's alpha**; the two blurs follow; the final
  composite `0x01113c40` is `field * 1 + bloom * 1 + 0` with the radial mix at `0`. The field's alpha is
  `min(a * tint.a, 1)` through the accumulation, zeroed on scanline rows, times `w` in the band. So the original
  has a soft alpha bloom over this picture that this build does not draw. **Confidence 80** on the programs; how much
  light it adds on this picture is unmeasured.

**`musicPulse` peaks at 1.33, by law.** Sampled for 115 s at 24 Hz on Campaign Selection
(2,786 samples, muted emulator): every band stays on `0..1` and the three-band mean reaches
exactly `0` and `1`, so the law's own range is `1 + (0 - 0.7) * 1.1 = 0.23` to
`1 + (1 - 0.7) * 1.1 = 1.33`. The `1.2` above was a short sample, not a ceiling; nothing in
the code caps at it. **The bank is `+0x19c`**: `particleColour` read out of the ring beside
both band sets for 25 s fits the law to a mean of `0.11` on `+0x19c..+0x1a4` and `0.37` on
`+0x1dc..+0x1e4`. Confidence 90 on the range, 85 on the bank (the error left is a frame's
lag on a moving signal). The sender's percentiles (median `0.856`) are not reproduced: over
115 s here `+0x19c` gives a median of `0.385`, and their numbers look like the `+0x1dc` set
(`0.848`). The peak agrees; the distribution is unresolved and depends on the track.

**`OnEnable` does run on Main Menu to Campaign Selection.** The sender says it does not. Block
`+0x1648`..`+0x1650` read after each of four presses, Campaign, Main, Campaign, Main, Campaign:
`(12, 0.211 s)`, `(24, 0.403)`, `(27, 0.453)`, `(37, 0.625)`, `(33, 0.563)`. A new random length
each time, and the only code that writes the seconds field after construction is `OnEnable`
(`stfs f0, 12(r29)` at `0x0017e8a8`, `r29 = item + 0x1644`; `Render` only reads it at
`0x001843c0`, the rest are the two constructors and static init). **Confidence 85**, no
breakpoint. What their
measurement may have seen instead: the pulse's effect on `particleColour` could not be
separated from the music here, so the visible amplitude stays open.

**Omega:** checked, differs - no `BackgroundAnimFury` string in the 1.07 `eboot.bin`, only
`BackgroundAnim_Item.cpp` and `FEBackgroundAnim*`. **2048:** checked, differs - no
`BackgroundAnim` string at all in the v1.04 executable. None of this section applies to either.
The wiring below reaches only `MenuBackdrop::Fury`, which the Fury style alone selects; the HD-style and
Omega scene backdrop is untouched and its ground truth passes.

## The pass graph, wired (2026-11-05)

`oag_game::render::backdrop` runs the table above: points, blend, glow, wave, composite, band, scanlines,
then a present pass for the final composite's half texel. **Decision on the linear pipeline: an sRGB format, not by hand.** Every
target is `Rgba8UnormSrgb`, so decode, encode and linear blending are the format's; the page stays
gamma-space (ADR-0020) and receives the stored bytes through a plain view, so nothing is decoded and encoded
twice. `fs_composite`'s hand encode and the chosen sRGB curve are gone. The accumulation and the field read
themselves in the original, so here the accumulation is a swapped pair and the band reads a copy.

The particle pass is encoded with the rest: `0x1fec` is `1` on draw 1, and a build that wrote it as plain bytes
read `R mean 0.4` where the encoded one reads `15.0`, against the reference's `48.3` (bands at zero in both).

Measured on Main Menu, static path 6, `mirage_c1`, clip frame 484, paused, over rows `230..790` and columns
`600..1900`, against the frame kept at `data/reference/hd-capture/fury-backdrop-pass-graph/paused-02.png`
(`paused.json` beside it holds the state):

| | R mean | R q90 | R q99 | G mean | RMSE | Rows by `row % 4` |
| --- | ---: | ---: | ---: | ---: | ---: | --- |
| reference | 48.3 | 137 | 255 | 3.9 | | 46.7, 67.1, 66.9, 46.3 |
| before, as shipped, bands at zero | 2.2 | 8 | 23 | 0.0 | 44.6 | |
| **after, as shipped, bands at zero** | **15.0** | **46** | **85** | **0.1** | **35.5** | |
| before, level **fitted**, band `0.95` | 12.6 | 49 | 108 | 0.4 | 40.3 | 14.3, 14.2, 14.2, 14.1 |
| after, level **fitted**, band `0.95` | 53.0 | 117 | 191 | 3.5 | 26.3 | 49.0, 68.3, 68.1, 48.7 |

**As shipped the backdrop is about a third of the reference's level, and that gap is not fully explained.** The
fitted rows are a scratch build with the band forced to match the level, for comparing shape; nothing ships
them. The state read at the pause was bands at zero (`particleColour 0.0877`), so the fit contradicts the one
reading of this frame's music, and no reason is known: `Equaliser Damp Speed` makes a jump within a frame unlikely.
Split in linear light over the two clean rows of each four, the reference means `0.118`, ours `0.012`, fitted
`0.084`. The difference from bands at zero keeps `0.45` of its energy under an 8-pixel Gaussian, about what the
reference's own picture keeps (`0.48`), so it carries the dots' structure and a blur alone cannot be most of it;
what fitting leaves (`0.034`) keeps `0.22`. Candidates, none tested: the music reaching this frame some way the
state read missed, the alpha bloom, the glow at its ceiling (`+0x1dc` sat over the threshold on three or four of
the eight paused frames).

Shape, at the fitted level only: the dots land on the reference's, the best integer alignment is `3` pixels
sideways and none vertically (not checked across the frame), and the frames' autocorrelation falls to one half at
`5` pixels in both, `4` as shipped. So the page's "dots 1.7x wider" no longer reproduces at that level, which
points at the encoding but is not a closed question until the level is. The smear band sits in the reference
near row `745` of that frame at about `0.7` of its neighbours; ours is `0.63` at its own phase, the band's clock
being a global timer that differs between the two.

Stored pairs re-rendered from scratch copies, a random cloud and clip on each side: Ship Select mean grey `19.55`
to `21.61` against `21.55`, RMSE `28.89` to `29.76`; Campaign Select `32.18` to `32.35` against `32.51`, RMSE
`15.58` to `15.78`. The level moves to the reference; RMSE cannot improve on a different cloud. A `--menu-page`
still now runs `FRAMES_TO_SETTLE` frames of history first, because a picture that feeds back has nothing in it
after one frame, and with no `--anim-seconds` it stands that many frames into the clip.

Tests: `crates/game/tests/hd_fury_ship_select_ground_truth.rs` fails if the tint moves back to the composite
(the tint gone for three frames leaves a decaying picture, not black), and pins the two-row scanline.
`crates/ui/src/backdrop/tests.rs` holds the glow byte, the band's 200-frame wrap across clips and the pivot.

Not wired: the 720p field's 2x1 antialiasing, which changes no dot this build compares (1080p is plain), and
mode 11's mip chain, which needs morph paths to draw at all. RPCS3's `sampleTexture2DMS` defect is not emulated.

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

**Closed 2026-11-05 by "The pass graph, wired"; kept for the method. What still differs, measured on path 6 at frame 372 against
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
The Track Select screen the player sees (title `TRACK SELECT`) logs as `Track Creation` and is
`default` too, so Ship Select and Track Select get the same tint; `TrackHexSelection`, the
name this project's docs use for the same picker, authors no row either. This build's pickers
open from page `race` (depth 2), so the root's `Main Menu` tint never reached them, in
captures or live; no change was needed there.

Tool: `scripts/hd-fury-row-poll.py` finds the item and prints the current row.

**The brightness law.** The ratios stand; the reading of them does not. The tint is `fc0` of the Blend program,
in linear light, before the feedback ("The frame, measured"); the two laws this section weighs are both
retired, and the build carries neither. Silent, this build's own ratios sit under this table (about `0.66` at
`1/2`, `0.40` at `1/4`) because the composite's `1/257` takes more off a dim picture than the music-lit one this was
read from. The `default` row's tint was edited live (GDB write of three
floats at `row+0x40`) and the screen grabbed a quarter second after each write, in groups
of five tints so a group sees one cloud. Mean excess brightness over 28 groups (red
channel, 14 with the backdrop in view), as a ratio to tint 1:

| tint | 1 | 1/2 | 1/4 | 1/8 | 1/16 |
| --- | --- | --- | --- | --- | --- |
| measured | 1 | 0.80 | 0.62 | 0.40 | 0.23 |
| literal scale | 1 | 0.5 | 0.25 | 0.125 | 0.0625 |
| sRGB encode of the scaled sum | 1 | 0.73 | 0.53 | 0.39 | 0.28 |

At tint 0 nothing draws; at `0.125` the 99th percentile is still 255. So the tinted sum is
added in **linear light** and the buffer encodes it, as an sRGB render target does. This build
is gamma-space throughout (ADR-0020), so `fs_composite` does it itself:
`encode(tint * decode(sum))`, with the blend only adding. **Two laws fit these ratios
identically** - this one, and `encode(tint * sum)` - and differ only at tint 1, where the
second brightens Main Menu and the first leaves it. The ratios cannot tell them apart; the
earlier Main Menu census (above, "What still differs", means `13.6, 5.4` against `13.0, 5.2`)
leaves it alone, so the first is shipped and Main Menu is unchanged. Confidence 75 on the
shape and the 0.125 point (measured); **the curve is the standard piecewise sRGB, chosen, not
measured** - it misses the 1/2 and 1/4 points by 0.07-0.09, and a moving cloud and a clipped
top limit the fit. Exact where the page behind the backdrop is black, which it is under every
Fury page. The practical effect: every page but Main Menu now shows the cloud, where the
literal scale left it near black. Tint 1 is dimmer than the original's at the same moment
(red channel of the backdrop pixels, `q90 148` against `247`, over 20 clocks here and 28 frames there): this build sits `musicPulse` at 0.23
where RPCS3's menu music takes it to 1.2, not a tint law. Test:
`crates/game/tests/hd_fury_ship_select_ground_truth.rs`.

**`BackgroundAnimFury_OnEnable` fires on every screen change, not on a team step.** (Re-checked
2026-11-05 against a third party's contrary claim: see "The frame, measured".) Polling
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
live memory readout on Omega's emulator). **Below about 414 lines** no Fury cloud draws, before and after this change: `resScale =
clamp(0.0015 * height - 0.62, 0, 4)` is 0 there, and it scales the particle and ramp
colours (above), so this is the original's own law, not a gap.

## Open

- The alpha bloom over the field ("Three more readings"): wired neither here nor in the front end at all.
- Modes `5`, `9`, `10`, `11`: the vertex programs are paired and disassemble; their
  `Update` functions and the two extra streams (`targetPosition`, the line/quad
  offsets) are unread.
- The equaliser: what feeds the sixteen bands at `+0x1b4` and the banks at `+0x19c` and `+0x1dc`, what moves
  the glow threshold `g+0x1708`, and whether the menu's own music track is what it analyses.
- Why the backdrop's level on a paused frame is three times what its own `particleColour` predicts.
- `0x00182358`'s purpose: inert here with its two inputs at zero, shaped like a
  stereo eye offset, unnamed.
- The HD-style `BackgroundAnim_Item.cpp` (`FEBackgroundAnim_vp`/`_fp`/`Copy_fp`, the
  `.vex` ring with `blur`/`use_bands`), which answers hd-frontend.md's open question
  about which non-Fury program the HD style resolves to: its own, not a RadioHead.
