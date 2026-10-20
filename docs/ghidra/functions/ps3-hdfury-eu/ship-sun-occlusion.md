# HD's ship sun occlusion: the track shadows the craft through a second per-ship map

2026-09-21. Read because a craft driving through a baked shadow on a Wipeout
HD circuit stayed fully sunlit in this renderer while the road under it went
dark - and `docs/rendering/shadows.md` said the original's receiver was "the
road, not the scene", which turned out to be true of only one of the two maps
each ship carries. **The hull is a receiver too**, of a map this project had
never looked at: the track around the craft rendered from the sun through its
own sun-occlusion technique.

Static only, so every score caps at 84 per the
[confidence rubric](../../../reverse-engineering/confidence-rubric.md). Every
TOC-relative load was resolved with `scripts/ps3-toc.py resolve` against the
function's own OPD TOC, not read off Ghidra's symbol names - see
[memory.md](memory.md). Nothing here has been watched live in RPCS3; the next
step section says what a capture would settle.

## The names

| Address | Kind | Name | Confidence |
| --- | --- | --- | --- |
| `0x006cdfc0` | function | `Job_RenderShips_Run` | 82 |
| `0x003f0950` | function | `Shadow_RenderShipSunOcclusionMaps` | 80 |
| `0x003ea368` | function | `Ship_DrawModels` | 78 |
| `0x00401ba8` | function | `Shadow_CompileSunOcclusionTrackRedraw` | 78 |
| `0x003e5e58` | function | `Shadow_AllocShipMapPool` | 78 |
| `0x003e4cc8` | function | `Ship_AddToRenderList` | 74 |
| `0x0076a080` | data | `g_ShipMapSizes` | 78 |
| `0x005a40f8` | function | `Rsx_SetRenderTargets` | 80 |
| `0x005c2380` | function | `Rsx_SetColorMask` | 84 |
| `0x005c2524` | function | `Rsx_SetDepthMask` | 84 |
| `0x005c36a8` | function | `Rsx_SetColorClearValue` | 84 |
| `0x005c3724` | function | `Rsx_SetZStencilClearValue` | 84 |
| `0x005c38c0` | function | `Rsx_ClearSurface` | 84 |

The five `Rsx_*` helpers are the same one-register poke shape
[material-state.md](material-state.md) decoded `Rsx_SetMethod` from: each
writes a header word `(count << 18) | method` and its argument. Headers read
directly: `0x40a70` (`NV4097_SET_DEPTH_MASK`), `0x41d90`
(`SET_COLOR_CLEAR_VALUE`), `0x41d8c` (`SET_ZSTENCIL_CLEAR_VALUE`), `0x41d94`
(`CLEAR_SURFACE`); `Rsx_SetColorMask`'s `0x40324` was already read on
[shadow-stencilvolume.md](shadow-stencilvolume.md), where the register
identity was checked against RPCS3's `gcm_enums.h`. `Rsx_SetRenderTargets`
is the fourteen-times-called surface bind [renderer.md](renderer.md) read
under "The 14 surface binds": `(ctx, &depth, &colour0, &colour1, &colour2,
&colour3)`, pointers to slots, each substituted when null; 80 because the
argument shape is confirmed at every call site this page adds to those
fourteen, but the format word it builds is still unread.

## `Job RenderShips` is two passes, and the first one was unread

[rcsmaterial.md](../../../formats/rcsmaterial.md#the-frames-pass-order)
names `Job RenderShips` as the twelfth job and quotes its run function as
`0x3ea368`. That is the second half. The job's vtable is `TOC[-0x6240]` =
`0x0086c958` - `Scene_PrepareFrame` (`0x003aa888`) stores it at
`0x003ae514` right after constructing the base with the string
`TOC[-0x60b8]` = `0x007b1168` `'Job RenderShips'` at `0x003ae4c0` - and its
slot 3 is OPD `0x0088b870` -> **`0x006cdfc0`**, which is three calls long:

```c
void Job_RenderShips_Run(_, ctx, arg) {
    Shadow_RenderShipSunOcclusionMaps(ctx, arg);    // 0x003f0950
    Rsx_SetMethod(ctx->gcm, 0x1fec, 1);
    Ship_DrawModels(ctx);                           // 0x003ea368
}
```

So before any ship is drawn, every ship gets a map rendered for it. Confidence
82 on the trampoline: it is reached from the job's own name string through
its own vtable, the same route [shadow-model-maps.md](shadow-model-maps.md)
used for the sibling jobs.

## What each ship carries: four textures at `+0xe0`

`Ship_DrawModels`, `Shadow_RenderModelShadowMaps` and
`Shadow_DrawOccluderStencilVolumes` all read a pointer at `+0xe0` of the
`0x1b0`-byte ship render record (the array at `PTR_DAT_008b7da0`, count
`+0x933c4`, index list `+0x933c8` -
[shadow-model-maps.md](shadow-model-maps.md)). It points into a pool of
**eight 16-byte records** at `PTR_DAT_008b7da0 + 0x95fd0`, built by
`Shadow_AllocShipMapPool` (`0x003e5e58`):

```c
for (i = 0; i < 8; i++) {
    rec = pool + 16 * i;
    rec[0] = Texture_Create(w0[i], h0[i], 0x20, 0, 0, 0, 0);   // FUN_005a6ce0
    rec[1] = Texture_View(rec[0]);                              // FUN_005aa1d8
    rec[2] = Texture_Create(w1[i], h1[i], 9, 0, 0, 0, 0);
    rec[3] = Texture_View(rec[2]);
}
```

with the four size tables at **`g_ShipMapSizes`** `0x0076a080`
(`TOC` pointer `0x008b7dd0`), read directly off the image:

| Slot | `+0` (format `0x20`) | `+8` (format `9`) |
| ---: | --- | --- |
| 0 | 512 x 512 | 256 x 256 |
| 1 | 512 x 512 | 128 x 128 |
| 2 | 256 x 256 | 128 x 128 |
| 3 | 128 x 128 | 128 x 128 |
| 4 | 64 x 64 | 64 x 64 |
| 5-7 | 32 x 32 | 32 x 32 |

Which slot a ship gets is not read here (the record pointer is assigned
outside every function this page covers; `Ship_FreeRenderSlot`-shaped code at
`0x003e55d0` nulls it without freeing, so the pool outlives the ship). The
`0x20`/`9` format codes are `FUN_005a6ce0`'s own vocabulary and were not
decoded; what settles what each texture *is* is how it is bound, below.

The same `Texture_Create`/`Texture_View` pair, with the same refcount
dance, is what [renderer.md](renderer.md)'s bloom ladder allocates its
buffers with, so the reading of the two calls is shared rather than new.

### `+0`: the ship's own depth map, and it is depth

`Shadow_RenderModelShadowMaps` (`0x003ed810`) binds it as the **depth** slot
- `Rsx_SetRenderTargets(ctx, &ship->maps[0], &0, &0, &0, &0)` at
`0x003eda10`, the record pointer in the depth position and four zeroed stack
words for colour - then `Rsx_SetColorMask(0,0,0,0)`, `Rsx_SetDepthMask(1)`,
`Rsx_SetZStencilClearValue(0xffffffff)`, `Rsx_ClearSurface(3)` (depth and
stencil), and draws the ship with `0x1830` set to `0x404` (`GL_FRONT`) for
the pass and restored to `0x405` after: a front-face-culled, depth-only
render from the sun. **This corrects a reading on
[shadows.md](../../../rendering/shadows.md)**: the receiver's `TXP` with no
compare instruction was taken there to mean the map holds coverage. It
holds depth, and the compare is the RSX's own depth-texture sampling; the
track's `1 - TXP` and the ship's gate below both read a *result*, not a
value. Confidence 84 - every binding call is read, no capture yet.

### `+8`: the track around the ship, drawn as its sun-occlusion mask

`Shadow_RenderShipSunOcclusionMaps` (`0x003f0950`), for every ship whose
byte `+0x140` is set and whose flag word `+0xe4` has bit `0x2` (the same
bit that gates the stencil volume -
[shadow-stencilvolume.md](shadow-stencilvolume.md)):

1. `Rsx_SetRenderTargets(ctx, &0, &ship->maps[2], &0, &0, &0)` at
   `0x003f0bcc`: the `+8` texture as **colour 0**, no depth.
2. `Rsx_SetColorClearValue(v)` then `Rsx_ClearSurface(0xf0)` - colour only.
   `v` is **`0` (black)** unless `g_ZoneEffectsActive` is set or the loaded
   track's name (`gamestate+0x1bc -> +0x78`) is one of `'17_Track'`,
   `'18_Track'`, `'24_Track'`, `'23_Track'`, `'07_Track'`, `'15_Track'`
   (`TOC[-0x5544]..[-0x5530]`, six `strcasecmp`s at `0x003f0c70..0x003f0fc8`),
   in which case it is **`0xffffffff` (white)**.
3. `Shadow_BuildShipShadowMatrices(sun, ship, bbox, ship+0x80, ship+0x40)` -
   the **same** per-ship sun-view box the depth map uses - and publishes
   `ship+0x80` as `viewProj` (engine parameter 1).
4. `Rsx_SetColorMask(0,0,1,0)`, `Rsx_SetDepthMask(0)`,
   `Rsx_SetMethod(0xa74, 0)` (depth test off): only the **blue** channel is
   written, and nothing is depth-tested.
5. `Shadow_CompileSunOcclusionTrackRedraw(10.0, ctx, job, ship->pvs_cell
   (+0x194), ship->position (+0x30), sun)` - below.
6. Colour mask back to `(1,1,1,0)`, depth test back on, the texture's
   refcount released.

`Shadow_CompileSunOcclusionTrackRedraw` (`0x00401ba8`) is a sibling of
`Shadow_CompileShadowedTrackRedraw` (`0x004053e0`) with the same skeleton:
PVS-gated (`Pvs_IsUsable` / `Pvs_NearestCellCached`) walk of the track's
chunk list, a test of each chunk's bounding sphere, then a compiled command
stream handed to `FUN_005fc728`. **The test is a cylinder along the sun, not
a sphere around the ship.** Its arguments are `(10.0, ctx, job, ship->pvs_cell
(+0x194), &ship->position (+0x30), &sun)` - `r6` and `r7` at `0x003f0cdc` /
`0x003f0de0` are the stack copies of `ship+0x30` and the normalised
`Lighting.Sun direction` - and per chunk the VMX idiom at
`0x00401cd8..0x00401d60` is `d = chunk.centre - ship.pos`, then
`cross(d, sun)` by the rotated-multiply pattern, squared and summed, compared
`>` against `chunk.radius^2 + 10.0^2` (the `10.0` is `fRam008b7e98`, squared
at `0x00401c98`) to *skip* the chunk. `|cross(d, sun)|` is the distance of
the chunk's centre from the sun line through the ship, so a chunk is drawn
when its sphere comes within ten units of that line, however far along the
line it sits; the box's own near/far (70 units either side) is the only
depth bound. Confidence 78 on the cylinder: the cross-product idiom is the
same one `Shadow_BuildShipShadowMatrices` uses for its `up` and reads
cleanly, but nothing has been watched live. (`Shadow_CompileShadowedTrackRedraw`'s
"within 50 units" on shadow-model-maps.md is the same shape of code with
the same sun argument and is very likely the same cylinder; not re-read.)

What differs from that sibling is the technique it compiles each chunk with:
not `Shader_GetVariantHash(word)` but a fixed hash chosen by the chunk
material's own two low flag bits (the word at `material -> +0x20 -> +8`):

| Flag | Technique hash (`base + 0x4a20 + ...`) | Name |
| --- | --- | --- |
| bit 1 | `PTR_DAT_008b8360` -> `0x00d42c58` (`+0x4a38`) | `SunOcclusionLightmap` |
| bit 0 | `PTR_DAT_008b835c` -> `0x00d42c5c` (`+0x4a3c`) | `SunOcclusionVertex` |
| neither | not drawn | - |

Those two are the standalone techniques
[renderer.md](renderer.md#the-engines-own-parameter-table-read-from-its-initialiser-2026-08-24)
already located as hashed past the 4096-word permutation table. What they
compute is on the disc - `talons_junction/materials/track_surface.rcsmaterial`
variants 3 and 4, `scripts/ps3-microcode.py fp-file`:

```text
SunOcclusionLightmap  (fp @0x4820)          SunOcclusionVertex  (fp @0x46d0)
  MOV H0.w, {1}                               MOV H0.w, {1}
  TEX H1.w, f[TC0] unit0   <- lightmap        MOV H0.xyz, f[TC0].wwww  <- colourSet.w
  MOV H0.xyz, H1.wwww END                     END
vp @0x4740: position * viewProj, o[TC0].xy = lightmapUV
```

That is, each chunk within ten units of the sun line through the craft is
drawn from the sun's view writing **its own sun-occlusion mask** -
`lightmap.a` on a lightmapped chunk, the colour set's fourth byte on a
vertex-lit one, the two carriers renderer.md's "The sun is real and it is
masked" already matched - into the map's blue channel, over black. The
selection is by lighting family alone: a transparent chunk with a lightmap
(Talon's Junction's glass floor) is drawn like any other, and a chunk of
the flat-`Ambient` family, with neither carrier, is not. No depth: the road
under the craft and anything else in the cylinder simply overwrite in chunk
order.

Confidence 80 on the pass, 78 on the compiler (its skeleton is
`Shadow_CompileShadowedTrackRedraw`'s, read at 75 on shadow-model-maps.md,
plus the technique selection read here; what the SPU job does with the
stream is not traced, same as there).

### `+4` and `+0xc`: what the ship draws with

`Ship_DrawModels` (`0x003ea368`), per drawn ship, when `ship+0xe0` is
non-null (at `0x003ea560..0x003ea5b4` and again in the second walk at
`0x003ea9e0..0x003eaa34`):

```c
params[0x23c] = ship->maps[1] + 0x20;   // slot 17: directionalLight0ShadowTex
params[0x25c] = ship->maps[3] + 0x20;   // slot 18: directionalLight0LightmapTex
params[0x218] = ship + 0x40; params[0x21c] = 4;   // slot 16: directionalLight0Proj
variant |= 0x40;                        // ShadowMap
```

The slot arithmetic is the one shadow-model-maps.md already used for
`0x518`/`0x53c` (slots 40/41): value pointer at `0x18 + 0x20 i`, sampler at
`0x1c + 0x20 i`. `0x218` -> 16, `0x23c` -> 17, `0x25c` -> 18 - three
consecutive slots landing on the three consecutive
`directionalLight0Proj` / `ShadowTex` / `LightmapTex` names in renderer.md's
initialiser order, with the vec4 count of 4 on the one called `Proj`. And
`ship+0x40` is the bias-form shadow matrix
`Shadow_BuildShipShadowMatrices` writes, the same one the track redraw
publishes as `shadowMatrix`. `0x40` is bit 6 of the permutation word,
`ShadowMap` ([rcsmaterial.md](../../../formats/rcsmaterial.md)) - which is
why that page found the bit "only on the model and ship path": this is the
path.

The ship's own material has the variant. `data/materials/ships/diffuse_vcol.rcsmaterial`
#3 `Static/HalfBrightAmbientSunShadowMapSpot0SVC0` (fp `@0x36e0`):

```text
@0x01  TXP R0.x, f[TC0] unit1     <- 0x9becc725 directionalLight0ShadowTex   (the depth map, compared)
@0x02  TXP R0.z, f[TC0] unit2     <- 0xa51b8b14 directionalLight0LightmapTex (the occlusion map's blue)
@0x03  MUL H0.w, R0.xxxx, R0.zzzz
@0x05  DP3_SAT R0.x, R1, {directionalLight0DirectionWorldSpace}
@0x07  MUL H0.xyz, R0.xxxx, {directionalLight0Colour}
@0x0c  MAD H0.xyz, H0, H0.wwww, {constantAmbientColour}    <- sun * N.L * gate + ambient
```

against the ordinary `#2` `HalfBrightAmbientSunSpot0SVC0` (fp `@0x32e0`),
which computes `ambient + sun * N.L` with **no gate at all**. So on the
hull:

```text
sun_gate = shadow(own depth map, hardware compare) * occlusion(track mask under the craft)
colour   = albedo * vcol * (ambient + sun * N.L * sun_gate) ; then fog
```

Both taps use one projective coordinate, `f[TC0]`, which the paired vertex
program derives from `directionalLight0Proj` - the maps share a box, so the
same texel of each lands on the same point of the hull. The `.z` read on
unit 2 is the blue channel the occlusion pass's colour mask left open.
Confidence 84 on the shader (disassembled twice, once by
`ps3-microcode.py` and once by `oag_rcs::rcsmaterial::fragment` through
`crates/render/examples/hd_ship_material_dump.rs`, agreeing); 80 on the
binding.

## What this means

- **The hull is a shadow receiver in HD**, of two things multiplied: its own
  silhouette from the sun (self-shadowing, and nothing else - the map holds
  only that one ship), and the track's baked sun occlusion sampled along the
  sun through the craft. A craft under a bridge or on a road the artists
  painted dark goes dark with it, because the road under it says so; a craft
  over nothing (off the track, or the parts of the box no chunk within ten
  units covers) reads the clear value - black on most circuits, so **no
  sun**, white on the six named tracks and in Zone.
- **Ship-on-ship shadowing does not exist**: each ship's depth map has one
  caster.
- The occlusion map is the *track's own answer* re-sampled, not a second
  shadow computation - which is what makes it consistent with the lightmap
  by construction. It is also why HD's static shadows and this receiver
  cannot disagree the way a real-time cascade and a lightmap would.
- The `0x1830` register: called with `0x404`/`0x405` (`GL_FRONT`/`GL_BACK`)
  it can only be `SET_CULL_FACE`'s argument domain; the depth-map pass culls
  front faces, the standard shadow-map acne trade. Confidence 75, not
  checked against `gcm_enums.h` this round.

## What this project does with it

`oag_render::shadow::occlusion` renders the same map - the track's draw calls
whose bounds come within ten units of the sun line through each craft, from
a sun-view box 12 units wide and 70 deep either side of the craft, writing
`lightmap.a * sun_mask` over black - and `mesh.wgsl` gates the hull's sun
term by it. Measured on Talon's Junction (autopilot, `OAG_DUMP_SUN_OCCLUSION`):
in the grid-start tunnel at tick 2100 the map under the craft reads 12/255,
on the glass floor at tick 4800 it reads 245/255 - the road's own answer in
both places. `oag_render::shadow::self_shadow` renders the depth half the
same way this page reads it - depth only, no colour target, front faces
culled, cleared to far, one caster, the occlusion layer's own matrix - and
`mesh.wgsl` multiplies the two taps as the microcode does; on the Feisar at
tick 4800 it darkens the cockpit recess and the engine pods' inner faces and
nothing else in the frame (`OAG_DUMP_SELF_SHADOW=<png>` writes the layer).
What is this project's there: the four-tap compare and its bias, and that
every craft gets slot 0's 512 texels (the depth-to-occlusion ratio above is
read; which craft gets which slot is not). The clear-value exception for the six track names is not
reproduced either, because what those names map to on this disc's
environment folders was not resolved (`02_track`, `03_track` exist; `17_Track`
and friends are manifest names of a different vocabulary).

## What a capture would settle

- The slot assignment: which ship gets the 512-texel maps (the player, by
  distance, by grid position). `rpcs3-drive.py capture` on
  `PTR_DAT_008b7da0 + 0x95fd0` and each ship's `+0xe0` mid-race.
- The occlusion map's contents - dump `maps[2]` from RSX memory the way
  [engine-trail.md](engine-trail.md) dumped vertex buffers - which would
  also show whether the blue-only mask leaves the other channels at the clear
  value, and what `format 9` is.
- Whether the six named tracks are in fact the tracks with no usable
  occlusion carrier, which is what a white clear would be for.

## Read live on RPCS3: the map is sRGB-encoded, and the six names are three circuits (2026-10-20, `hd-hull-pearl`)

Harimau classic at the Talon's Junction Time Trial start (`hd-timetrial-talons-harimau-classic-start`, Write Color Buffers on),
one paused frame, the hull's draws (498-530 of 615). The hull's `diffuse_with_specular_from_alpha_n` draw binds the livery
on unit 0, its normal map on 1, **a 512x512 depth texture on unit 2** (`0x01f50000`, format byte `0xb2`, `DEPTH16`, linear, pitch
1024: the self-shadow map) and **a 256x256 one-byte texture on unit 3** (`0x01fd0000`, format byte `0xa1`, `B8`, linear, **pitch 512**:
the occlusion map), then the two probe maps on 4 and 5. Both unit words carry gamma nibble 0 (`0x60030303`).

**What the map holds.** Read at `0xC1FD0000`, 256 rows of 512 bytes (the right 256 bytes of every row are padding: read at
a pitch of 256 and every other row looks black):

| | texels |
| --- | --- |
| the road under the craft | **61** in 95 % of the map (percentiles 5 to 99: 59, 61, 61, 61, 62), mean 60.1 |
| smallest | **13** |
| largest | 68 |

and the lightmap those chunks sample (`ile_mesh_combine_track01_01-lmap.gtf`, 1024x1024, decoded with `hd_gtf_png`) has a
dominant alpha of **12** (392,312 of 1,048,576 texels) and a smallest of **1**. `encode(12/255) * 255` is 61.4 and
`encode(1/255) * 255` is 12.9, both to the byte. So the occlusion pass writes the mask **through the sRGB encode** (the colour target
is written in sRGB) and the hull reads the stored byte raw, with no decode, as its sun gate: the road under a Talon's start gates the
sun at **0.24**. Confidence 85 for the law on the lightmap carrier (two exact byte matches over a whole map); the vertex carrier goes
through the same write and is unmeasured. Our pass wrote the mask linear (12 where the original holds 61), a fifth of the original's
gate. Fixed in `crates/render/shaders/occlusion.wesl`; pinned by `sun_occlusion_map` (half sun reads 188, alpha 12 reads 61) and by
`hd_sun_occlusion_ground_truth` (the Talon's start map's median texel is 61).

What it changes on screen, six hand-picked flat facets of the Talon's hull (the original and ours at the same pose, mean RGB, the
pose not pixel exact so only large flat facets are read): left wing top **(114, 103, 71) against (80, 89, 75) before and (112, 104, 75)
after**, right wing top (156, 160, 145) against (140, 155, 146) and (157, 163, 148), fuselage left (121, 117, 112) against (100, 108, 108)
and (117, 116, 110), the right upper wing (106, 131, 131) against (100, 124, 125) and (108, 130, 126); the right wing tip got worse,
(127, 163, 200) against (123, 160, 206) and (129, 170, 214), and the green nose (54, 63, 54) against (50, 62, 61) and (61, 69, 63). Mean
absolute difference over the six: **8.5 before, 4.1 after**. The warm orange of the wings, grey before, is back.
**What does not hold up:** over the whole hull (every pixel the change touched, `talons`, `talons-assegai-c1`, `the-amphiseum`) mean
luma is 108.9 / 62.4 / 81.3 for the original and 109.4 / 62.8 / 81.1 before, 119.9 / 72.1 / 86.6 after: the hull got about 10 levels
brighter on average while the original's mean was already matched. A block regression of (original - before) on (after - before) over
eroded interior blocks has a slope of -0.59, -0.35 and -0.31 (correlation -0.2): the added light does not land where the original has
more. The ship's silhouette is a few pixels off in these pairs (the resting heading lead), so a pixel or block metric is dominated by
misalignment, and the six facets are the only valid read; but the hull-wide mean says the encode is not the whole of the sun term.
Specular is not it: zeroing it moves the six facets from 4.1 to 3.4 only. Open: the self-shadow tap (the depth map reads back as zeros
over the GDB read, the RSX keeps a depth target on the host), the `x8` on the specular's saturate in block 17 (`MOV_SAT x8 R0.w, R0`),
and whether the extra light is where the original is shadowed by the wing.

**The six names are three circuits.** `DATA00`'s `Data\Plugins\frontend\definition.xml` pairs every `PI_Track` with its
environment: `17_Track` and `18_Track` load `Data\Environments\Talons_Junction`, `23_Track` and `24_Track` `Tech_De_Ra`, `07_Track`
and `15_Track` `12_Sol_2` (the forward and the reversed entry each). So the white clear
(`0xffffffff`) of `Shadow_RenderShipSunOcclusionMaps` is Talon's Junction, Tech De Ra and Sol 2 in both directions, and Zone.
**Read, not wired**: it only shows where no chunk covers a craft's 12-unit box, which the Talon's grid frame never does (the
original's map above holds no texel over 68). The earlier page's "what those names map to was not resolved" is closed.

**A second divergence, not changed.** The original draws only chunks whose lighting family has a carrier (`SunOcclusionLightmap`
for the lightmap flag, `SunOcclusionVertex` for the vertex flag, nothing for the flat-`Ambient` family). Ours draws every chunk in
reach, and an Ambient-family chunk with no lightmap and no colour set writes `1 * 1`: a white line crosses our map at the Talon's
start (`talons_junction` chunks with `lightmap=-`, variant `0x56c94426`), where the original's holds none. It lies outside the hull's
footprint on that frame.

**Cross-title.** Omega: checked, applies, not checkable (it carries HD's engine, but its original shadow tier is `Unbuilt`, its
hulls build through the PS4 path, and no PS4 capture exists). 2048: checked, differs (its shadows are a different mechanism,
`Unbuilt`). Pulse, Pure: no sun-occlusion map.
