# `cloudCube`/`cloudGroup`: both are registered, and `cloudGroup` is what draws

| | |
| --- | --- |
| **Binary** | `PSP_GAME/SYSDIR/BOOT.BIN` (Pulse, PSP, UCUS-98712) |
| **Subsystem** | scene-graph environment nodes |
| **Related** | [`vex.md`](../../../formats/vex.md) (the class-ID table, the 46 registration sites), [`skycube.md`](../../../formats/skycube.md) (the open question this page closes), `crates/vex/src/cloud.rs` (the parser this page's evidence backs) |

`docs/formats/skycube.md`'s Open section speculated `cloudCube` `0x3d8` might
have no registration site at all, by analogy with `engine_fire`/`exitglow`/
`gate` - none of which do, yet all three are authored. **That guess is
refuted for both cloud classes.** Both `cloudCube` `0x3d8` and `cloudGroup`
`0x3d9` are registered, each with its own method table, and `CloudGroup_Init`
is a real, substantial function - not the "return `&self`" compiler stub a
first read of its `+4` slot suggests.

## Summary

| Claim | Confidence |
| --- | --- |
| Both classes are registered via `Vex_RegisterClass` | **92** |
| `cloudGroup`'s method table overrides `draw`; `cloudCube`'s does not | **90** |
| `CloudGroup_Init` reads its colour ramp, sprite size, overlap and seed from the node's own named-attribute list, by name | **90** |
| The shared cloud texture is `Data\Tex\Cloud\Wipeout_Clouds_D_128x64x4.mip` | **92** |
| `cloudGroup`'s draw is a rotating, camera-facing billboard per instance, its corners turned by `g_camera_roll - phase` in view space | **92** - read to the instruction and matched live on 28 sprites; see below |
| Each sprite's rotation rate is a per-instance random draw, not a shared constant | **88** - `Psys_RandFloatRange` call read directly |
| The GE state for the draw (blend, depth, fog, lighting, cull) | **90** - read off `CloudGroup_ApplyDrawState`'s literal `Gu_*` calls |
| Each sprite gets one flat baked colour from a ramp over the field's height, sampled at its bottom and top edge and averaged | **93** - read, and every baked colour word reproduced on four boots |
| A cube is a box: `volume * 0.1` records scattered `+/- 5` units along its own axes, half-size `(R +/- R*var) * 10 * scale` | **93** - read to the instruction, every record reproduced from the seed on four boots; see [the field](#the-field-cloudgroup_buildfield-0x08933c1c-and-the-cubes-records) |
| A group builds from every cube in its subtree, so a nested group's cube draws two fields | **88** - the walk is read and two live groups shared one cube |
| Each sprite draws one cell of a 4x2 atlas over the texture; shipped cubes use cells 6 and 7 | **90** - tables read, UVs read live |

## `CloudCube_RegisterClass` (`0x08932138`) and `CloudGroup_RegisterClass` (`0x0893471c`)

Found the same way every other class's site is: among the 46 callers of
`Vex_RegisterClass` (`0x08908eb8`), one call each, matching the documented
template exactly - register the id, assign the base (`Transform`) method
table, then overwrite it with the class's own derived table:

```
08932138: addiu sp,sp,-0x10
0893213c: lui   a0,0x8b6
08932140: sw    s0,0(sp)
08932144: addiu s0,a0,0x5dc8          ; s0 = &DAT_08b65dc8, the descriptor
08932148: move  a0,s0
08932154: jal   Vex_RegisterClass
08932158:  li   a1,0x3d8               ; class id, in the call's delay slot
0893215c: lui   a0,0x8ad
08932160: addiu a0,a0,0x22f4           ; DAT_08ad22f4, the Transform base table
08932164: sw    a0,0x38(s0)            ; assigned first ...
08932170: jal   FUN_08a6ba64
...
08932184: sw    a0,0x38(s0)            ; ... then overwritten with the derived one
08932188: jal   FUN_08a72d10
```

`FUN_0893471c` is byte-for-byte the same shape for class `0x3d9`. Both are
direct `Vex_RegisterClass(descriptor, class_id)` calls with no indirection, so
this is as solid as any of the 46 registrations `vex.md` already documents -
confidence **92**, capped by the rubric rather than by any doubt in the
reading.

Renamed `CloudCube_RegisterClass` and `CloudGroup_RegisterClass`
respectively; both above 70.

## The method tables: `0x08ad299c` (`cloudCube`) and `0x08ad2a24` (`cloudGroup`)

`0x88` bytes apart, matching the stride every other pair of adjacently-linked
classes shows (`exhaust.md`'s `Engine Flare`/`Trail`). Read directly with
`read_memory`, 17 pointer slots each at `+0x0c, +0x14, .., +0x84`:

| Offset | Role (by `exhaust.md`'s convention) | `cloudCube` | `cloudGroup` |
| --- | --- | --- | --- |
| `+0x24` | update | `FUN_0892b7cc` (own) | `FUN_089326c0` (own) |
| `+0x34` | submit | `FUN_0892b638` (own) | `FUN_089326d4` (own) |
| `+0x44` | draw | `FUN_0894485c` (**base default**) | `FUN_0893280c` (own) |
| `+0x54` | (unnamed) | `FUN_08931918` (own) | `FUN_0893240c` (own) |
| `+0x6c` | (unnamed) | `FUN_089319a8` (own) | `FUN_089324e4` (own) |
| `+0x74` | (unnamed) | `FUN_08931a24` (own) | `FUN_089325bc` (own) |
| `+0x7c` | init | `FUN_08931b28` (own) | `FUN_08933048` (own) |
| `+0x84` | (unnamed) | `FUN_08931920` (own) | `FUN_08932414` (own) |

Every other slot on both tables (`+0x0c, +0x14, +0x1c, +0x2c, +0x3c, +0x4c,
+0x5c, +0x64`) holds one of the `0x08944xxx`-range addresses `exhaust.md`
identifies as the shared `Transform` base defaults - unoverridden on both
classes.

**The one asymmetry that matters for rendering: `cloudCube`'s `draw` slot is
the inherited base default, and `cloudGroup`'s is its own function.** A
per-node draw dispatch on a `cloudCube` instance calls the same no-op every
undrawn class does; `cloudGroup` is what actually issues geometry. This is
read directly off two `read_memory` calls, not inferred - confidence **90**.

Both classes override *more* slots than the four `exhaust.md` names
(`+0x54`, `+0x6c`, `+0x74`, `+0x84` beyond update/submit/draw/init) - left as
`FUN_` per this project's confidence floor, since none has been decompiled.

## The `+4` field is a class-identity tag, not a dead handler

`FUN_08a72d04` (stored at `cloudCube`'s descriptor `+4`) and `FUN_08a72d10`
(`cloudGroup`'s) both disassemble to:

```
lui v0, <hi>
jr  ra
addiu v0, v0, <lo>       ; v0 = own address
```

A pure "return `&self`" stub - the exact mechanism `weatherpos.md` already
documented for `WeatherPos_RegisterClass`'s `+4` field: not a handler, an
opaque per-class identity token the registration call captures by invoking
the stub once at boot. Confirmed by disassembly rather than assumed from the
decompiler's summary, because the decompiler's rendering of this exact shape
(`return FUN_08a72d04;`) is easy to misread as "does nothing" - confidence
**88**.

## `CloudGroup_Init` (`0x08933048`)

| | |
| --- | --- |
| **Address** | `0x08933048` |
| **Confidence** | **90** |

Reached the same way `weatherpos.md`'s real constructor was: the stored
`+0x7c` init slot, resolved dynamically per node rather than through any
static call site.

Reads twelve named attributes off the node's own header, via the same
`header+0x06`/`+0x0e` list `oag_vex::vex::node_attributes` already decodes for
`AnimTransform_Bind`'s `LoopEnd`/`AnimEnd`/`FixedFrames` - confirmed here by
a second, independent consumer reading the identical list shape for entirely
different names, all of which appear verbatim at `0x08a88aa4` in the binary:

```
Overlap, Seed, SpriteRadius, SpriteRadiusVar,
HiColourR, HiColourG, HiColourB, HiAlpha,
MidColourR, MidColourG, MidColourB,
LoColourR, LoColourG, LoColourB, LoAlpha,
Midpoint
```

**One divergence from `node_attributes`'s own doc comment**: that comment
records the lookup as case-*sensitive* in the original
(`AnimTransform_Bind`'s `strcmp`). `CloudGroup_Init` looks its names up with
`strcasecmp` - case-*insensitive*. Both are read directly off their own
disassembly; the mechanism is shared, the comparison function is not. Makes
no difference to any shipped file, because every cloud attribute name on
`05_Track` matches case-exactly, but recorded on
[`node_attributes`'s own doc comment](../../../../crates/vex/src/vex/attributes.rs)
so the next consumer of that list does not assume one comparison rule covers
every caller.

An attribute absent from the list reads as `0.0`, including `Seed` - except
that when `Seed` resolves to exactly `0`, the constructor takes it as "unset"
and re-rolls one: `Psys_RandIntRange(1, 9999)`. Every shipped `cloudGroup`
node's `Seed` is unset, so every shipped cloud gets a runtime-random seed
this project's parser does not reproduce. The draw itself is now read
([prng.md](prng.md)): the constructor first reseeds the particle generator
from clock-seeded libc `rand()`, so the re-rolled seed is genuinely
unpredictable - but everything downstream of it is not, because the group's
build (`FUN_08933c1c`) reseeds the same generator from `Seed` right before
the per-sprite draws.

The shared cloud texture load is guarded by `DAT_08ac01b4 == 1` - the first
`cloudGroup` ever constructed, process-wide - and stored in a static
(`DAT_08ac01b8`) every later instance reuses:

```c
FUN_089277ac(iVar7, s_Data_Tex_Cloud_Wipeout_Clouds_D__08a88b64, 0);
```

The string at `0x08a88b64` is `Data\Tex\Cloud\Wipeout_Clouds_D_128x64x4.mip`
- read directly, and confirmed present in `Data.wad` by
`crates/vex/tests/cloud_ground_truth.rs::the_shared_cloud_texture_is_in_data_wad`
(entry size 11,920 bytes, identical to `grabbedEngineFlare128x64x8.mip`'s -
both a 128x64 image with the same mip-chain shape `oag_texture::texture`
already decodes, per that module's own doc comment). Confidence **92**.

The constructed instance is appended to a global list
(`&DAT_08b30f10`, capacity `0x20` = 32, count `DAT_08ac1eec`) that
`CloudGroup_Draw` below iterates - so multiple `cloudGroup` instances draw
through one shared pass rather than each drawing itself independently.

## The field: `CloudGroup_BuildField` (`0x08933c1c`) and the cube's records

**Confidence 93** for every step below (read to the instruction 2026-10-04,
`pulse-clouds`, and reproduced exactly from live RAM on four boots; see
[Live check](#live-check-four-boots-every-record)). Not 95: one binary.

A `cloudCube` is not a sprite. It is a box the group scatters sprite records
through, and a `cloudGroup` builds one field from every cube it collects.

### Collection: `Vex_CollectNodesOfClass` (`0x08a72d1c`), 88

`CloudGroup_Init` calls `FUN_08a72d1c(group, group+0x78, 0x20, &count, probe)`
with `probe`'s class tag set to `cloudCube`'s (`FUN_08a72d04`). The function
appends the node when its `+4` tag matches and there is room, then recurses
into its first child (`+0x10`) and each sibling (`+0xc`). It is a preorder
walk of the group's **whole subtree**, up to 32 cubes, and the count lands in
`group+0x74`. A group that collects no cube is queued for destruction.

**A nested group's cubes are collected by every group above them.** On
`05_Track` the outer group (`0x09790220` in one boot) and the group inside it
(`0x097903e0`) both held the same single cube, and each built its own 46
records from its own seed and coloured them from its own ramp. That cube
draws two fields. The cube's own `+0x74` points to its nearest group (the
inner one), and that pointer is where the half-size's `SpriteRadius` and
`SpriteRadiusVar` come from.

The name's confidence is 88 rather than 93: the walk is read and the two
groups were seen sharing the cube, but nothing else calls it with a different
class.

### `CloudCube_Init` (`0x08931b28`) and `CloudCube_CountRecords` (`0x08931bec`)

`CloudCube_Init` measures the length of each of the first three rows of the
parent's matrix (`FUN_089451dc(parent)`, `vdot_t` then `vsqrt_s`) into
`+0x7c/+0x80/+0x84`, and stores their product, the cube's volume, at `+0x78`.
`CloudCube_CountRecords` stores `(int)(volume * 0.1)` (`DAT_08a88a90` reads
`0x3dcccccd`) at `+0x88`. Live: `05_Track`'s first cube has row lengths
`7.76697` on all three axes, volume `468.55`, and authors 46 records.

### `CloudCube_ScatterRecords` (`0x08931c3c`)

Read off the disassembly, with `f12 = 10.0` from `DAT_08a88a94` (`0x41200000`)
and `f24 = 0.5`. For each of the cube's records:

```text
u = Psys_RandFloatRange(-sx, sx) / sx * 10 * 0.5     ; one draw per axis, x then y then z
position = row0 * ux + row1 * uy + row2 * uz + row3  ; the cube's world matrix (+0x30)
half     = (R + Psys_RandFloatRange(-R*var, R*var)) * (payload.scale * 10)
record.+0x14 = the cube's index in the group           ; -1 once culled
```

`R` and `var` are `SpriteRadius` and `SpriteRadiusVar` of the cube's `+0x74`
group. So a record lands anywhere in a box `+/- 5` units along each of the
cube's own axes before its scale: the editor's cube is 10 units on a side.
On `05_Track`, `R = 4`, `var = 0.2` and `scale = 1` give half-sizes in
`[32, 48]`. That is the "about 9.4" factor this page's earlier Open item
measured: it is 10 times the authored radius, give or take the variance.

The routine also stores a `(2, 2, 2)` vector (`5 * 0.4`) on the stack and
never reads it.

### `CloudGroup_BuildField` (`0x08933c1c`), in order

1. Sum the cubes' record counts into `+0x184` and allocate the records.
2. `PsysRng_Reseed(group.Seed)`.
3. `CloudCube_ScatterRecords` per cube, in collection order.
4. `CloudGroup_CullOverlappingSprites` (below).
5. Measure the field's bounds into `+0x100` (min) and `+0x110` (max): each
   record's position `+/- half`. **The live-record test here reads record 0's
   flag on every pass** (`0x08933d74`: `lw a0,0x14(a0)` off the array base,
   with no index). Record 0 is never culled, so culled records widen the
   bounds too. The live bounds matched that and not the survivors'. Their
   centre goes to `+0x120`.
6. `CloudGroup_BuildDisplayList` (below), which continues the same stream.

### The atlas: `CloudGroup_BuildAtlasUvs` (`0x089322f8`), 90

Called once from `CloudGroup_Init`, it fills `0x08af28a8` with eight cells of
four packed UVs each: cell `i` covers `u` from `(i % 4) * 0.25` to `+0.25`, and
`v` from `(i / 4) * 0.5` to `+0.5`. That is a 4x2 grid over the 128x64 cloud
texture. The corners are listed as `(u0, v1), (u0, v0), (u1, v1), (u1, v0)`.
Each UV is packed by `FUN_0893226c` as `u * 32767` in the low half and
`v * 32767` in the high half (GE 16-bit texture coordinates).

`CloudGroup_BuildDisplayList` picks a cell per sprite by the cube's payload
`kind` (`**(cube+0x64)`), from four small tables read with `read_memory`:

| `kind` | Draws, in order | Cells |
| --- | --- | --- |
| 0 | `Psys_RandIntRange(0, 3)`, phase `(0, 2*PI)`, rate `(-0.002, 0.002)` | `0x08ac01bc` = 0, 1, 2, 3 |
| 1 | `Psys_RandIntRange(0, 1)`, phase `(0, 2*PI)`, rate 0 | `0x08ac01cc` = 4, 5 |
| 2 | `Psys_RandIntRange(0, 1)`, phase `(0, 2*PI)`, rate `(-0.002, 0.002)` | `0x08ac01d4` = 6, 7 |
| 3 | `Psys_RandIntRange(0, 0)`, phase `(-PI/16, PI/16)`, rate 0 | `0x08ac01dc` = 0 |
| other | no draw | whatever the record holds |

Every shipped cube is `kind == 2`, so every shipped cloud draws the right half
of the texture's bottom row. The live vertex buffer confirms both: variant 6
sprites carry `u` 0.5 to 0.75 and `v` 0.5 to 1.0, and variant 7 sprites
carry `u` 0.75 to 1.0. Among the positions read live at `bank1`
(`roll2.json`), the cell's bottom-left texel `(u0, v1)` sits on the corner at
`(-h, -h)` before the rotation, `(u0, v0)` on `(-h, h)`, `(u1, v1)` on
`(h, -h)` and `(u1, v0)` on `(h, h)`.

### Live check: four boots, every record

a throwaway script, not kept ports `PsysRng` and the steps above
in numpy `float32`, takes each group's seed and its cubes' matrices out of a
RAM dump, and compares against what the original built:

| Dump | Layout | Seeds | Authored | Kept | Result |
| --- | --- | --- | --- | --- | --- |
| `pulse-bloom-roll/ram0.bin` | `05_Track` forward | 2079, 6378, 9169 | 46, 46, 277 | 14, 14, 74 | all equal |
| `ram1.bin` | forward | 1513, 9888, 8684 | 46, 46, 277 | 12, 13, 78 | all equal |
| `ram2.bin` | reversed | 6730, 1161, 4862 | 46, 46, 277 | 10, 14, 84 | all equal |
| `ram3.bin` | forward | 9259, 3322, 5961 | 46, 46, 277 | 13, 13, 75 | all equal |

"All equal" means: every authored position and half-size to within
`1.2e-4` units, the culled set exactly, and every kept sprite's cell, rate and
initial phase. Every baked colour word is equal too, so the ramp below is
evaluated exactly. The phases had not advanced in these dumps. Each dump was
taken with the camera away from the clouds, so the draw that advances them
had not run (see [`CloudGroup_Draw`](#cloudgroup_draw-0x0893280c)).

**The four cubes near x = -1000 to -1120 are the third group**, and the
original builds and keeps them (`0x097a9b50` in the first boot: 277 authored,
74 kept). The earlier lane's scan missed it because its filter required at
most 64 authored records.

The Rust port (`oag_fx::cloud::field`) builds from our own parse of the
track rather than the RAM's matrices.
`crates/fx/tests/cloud_field_ground_truth.rs` pins it against the first
boot's values: counts, first and last records, cull, cells, rates and colours.

## `CloudGroup_Draw` (`0x0893280c`)

**Confidence 92** (read to the instruction 2026-10-04, `pulse-bloom-roll`, and
checked live the same day; below). Earlier passes left this `FUN_0893280c` at
65 because the corner arithmetic had not been decoded. It is now.

```text
0x08932830  f20 = g_camera_roll (0x08ab10a8), read once per call
0x08932844  push: copy the current view matrix (param_2 + *(param_2+0x1694)*0x40 + 0x1410)
0x089328cc  load 0x08a907a0 (the identity, read: 1,0,0,0 / 0,1,0,0 / 0,0,1,0 / 0,0,0,1)
0x08932900  Gu_SetMatrix(1, identity)   ; view
0x0893290c  Gu_SetMatrix(2, identity)   ; model
0x08932914  s2 = *(inst+0x194)          ; the vertex buffer the display list draws
0x0893291c  Gfx_BindTexture(DAT_08ac01b8)
for each of *(inst+0x188) records r (stride 0x20, at *(inst+0x190)):
  0x08932964  r.phase += r.rate          ; stored back, no dt
  0x0893296c  a = r.phase - g_camera_roll
  0x08932998  s, c = sin a, cos a        ; vmul by vcst 2/PI, then vsin/vcos
  0x089329b4  centre = view * r.position ; vtfm4.q with the saved view matrix
  0x089329c8  corners, h = r.half_size:
              (-h,-h) -> (-(s+c), s-c) * h      (h,-h) -> (c-s, -(s+c)) * h
              (-h, h) -> (s-c, s+c) * h         (h, h) -> (s+c, c-s) * h
  0x08932a34  six vertices, 0x20 apart (first and last repeated), 0xc0 a sprite
0x08932a70  Gu_CallList(*(inst+0x1a8)); pop the view matrix, Gu_SetMatrix(1, ...)
```

Each corner `(x, y)` lands at `(x cos t - y sin t, x sin t + y cos t)` with
`t = g_camera_roll - phase`: the corners are rotated by **`roll - phase`** from
the view's right towards its up, and they are added to the centre **after** the
view transform. The quad is built in view space and drawn with identity view
and model matrices. The view matrix moves the centre only, which is why the
quad always faces the camera.

`g_camera_roll` is `Camera_SubmitScene`'s roll ([weather.md](weather.md), 88):
the angle a level line in the world makes on the screen. Adding it puts the
spin's zero on the world's horizon rather than on the screen's, so a cloud
holds its orientation in the world while the camera banks, and spins only by
its own `rate`. Earlier versions of this page called the global the camera's
heading (yaw). By the mist lane's reading, and by the check below, it is the roll.

**Live, de Konstruct Black (`05_Track` forward), PPSSPP software renderer,
2026-10-04**. The two
`cloudGroup` instances were found in RAM by their `+0x184/+0x188/+0x190/+0x194`
shape (`0x09790220`, `0x097903e0`; 46 authored records, 14 drawn, each). At
19 pauses near the clouds (section 34), steering both ways, the angle read off
every drawn quad's vertices (`atan2` of its first edge) equals
`phase - g_camera_roll` for all 28 sprites:

- at a steady roll (`0.61`, `0.82`, `-1.03` rad) to `0.0000`-`0.0003` rad;
- while the roll swings, to at most `0.046` rad, about one frame's change
  in the roll. The pause can land between `Camera_SubmitScene` writing the
  next roll and this draw reading it.

The half-size read off the vertices equals the record's `+0x10` to two decimals.
Not 95: one binary.

**Where the camera is far from the clouds, the draw does not run.** At the
start line (section 16) and section 29, the phases did not move across 20
pauses and the vertex buffer held uninitialised words. This was never traced
to a visibility test; the next address to read is whatever calls the `draw`
slot (`0x08ad2a68`) for a `cloudGroup` node.

## `CloudGroup_ApplyDrawState` (`0x08932ab4`) and `CloudGroup_RestoreDrawState` (`0x08932b78`)

**Confidence 90.** `CloudGroup_Draw` never issues a `Gu_*` call itself for
state - it replays a pre-baked display list (`Gu_CallList(*(param_1+0x1a8))`,
built once by `CloudGroup_BuildDisplayList` below). The state that display
list's own recording opens with is `CloudGroup_ApplyDrawState`, read directly
off its body (`0x08932ab4`):

```c
Fog_Disable(g_display);
Gu_DepthMask(1);              // depth write off - exhaust.md's own convention
FUN_0891e988(g_display, 0xfdb2, 0x3e9, 0);  // Gu_TexFunc-shaped call, not identified further
Gu_DepthFunc(6);               // GU_GREATER - depth *test* stays on
Gu_TexScale(1.0, 1.0);
Gu_TexOffset(0, 0);
Gu_Disable(GU_LIGHTING);       // 10
Gu_Disable(GU_CULL_FACE);      // 5 - a camera-facing quad has no stable winding
Gu_Enable(GU_TEXTURE_2D);      // 9
Gu_Disable(GU_ALPHA_TEST);     // 0
Gu_Enable(GU_BLEND);           // 4
Gu_BlendFunc(GU_ADD, GU_SRC_ALPHA, GU_ONE_MINUS_SRC_ALPHA, 0, 0);
Gu_PixelMask(0xff000000);      // alpha channel not written
Gu_Color(0xffffffff);
Gu_TexWrap(GU_CLAMP, GU_CLAMP);
```

**This is a standard alpha blend, not the exhaust's additive one** - `GU_ADD,
GU_SRC_ALPHA, GU_ONE_MINUS_SRC_ALPHA` is a lerp toward the destination, the
opposite of `exhaust.md`'s `BLEND` (`SrcAlpha`/`One`). Depth test stays on
(`GU_GREATER`, this project's already-established equivalent of
`wgpu::CompareFunction::Less` - see `exhaust.rs`'s `HALF_SIZE_TO_WORLD` doc for
the same mapping used on the flare) but depth *write* is off, and the alpha
channel is masked out of every write (`Gu_PixelMask(0xff000000)`) - the same
"writes colour only" reasoning `exhaust::Pipeline` already documents for the
flare, here read directly rather than inferred from a reset. Lighting and cull
are both off, matching a billboard with no stable winding and no per-vertex
normal.

`CloudGroup_RestoreDrawState` (`0x08932b78`) is the bracket's other half,
called after the draw: `Gu_Enable(GU_LIGHTING)` then `FUN_0891e9d0`, which
re-enables fog only if the camera is presently inside a fog volume
(`Fog_FindVolume`). So the disable above is scoped to the cloud draw alone,
not a global state change - confidence **88**.

`FUN_0891e988`'s two integer arguments (`0xfdb2`, `0x3e9`) were not resolved to
a named `Gu_*` call in this pass; it forwards to `FUN_088111cc` unchanged, and
is left as a `FUN_`-named read rather than a guess.

## The rotation rate is a random per-instance draw, not a shared constant

**Confidence 88.** `CloudGroup_BuildDisplayList` (`0x08933ec4`) is what
allocates and bakes the per-sprite record array `CloudGroup_Draw` later reads
(`*(param_1+400)`, stride `0x20`/32 bytes) - the same field `+0x1a8` it writes
is the display list `CloudGroup_Draw` calls. Per record:

| Offset | Field | Source |
| --- | --- | --- |
| `+0x00` | world position (`x,y,z,w`) | copied from the authored/culled record |
| `+0x10` | half-size | copied from the authored record (`SpriteRadius` +/- variance) |
| `+0x14` | phase | `Psys_RandFloatRange(0, 2*PI)` - `0x40c90fdb` read back as **6.283185** |
| `+0x18` | phase rate | `Psys_RandFloatRange(-0.002, 0.002)` - `0xbb03126f`/`0x3b03126f` read back as **+/-0.0020000001** |
| `+0x1c` | atlas cell | `Psys_RandIntRange` into a per-`kind` table of cells; see [the atlas](#the-atlas-cloudgroup_buildatlasuvs-0x089322f8-90). Drawn **before** the phase and rate |

All three random draws happen **once, at display-list build time** (i.e. once
per boot, like the `Seed` re-roll `CloudGroup_Init` already documents), not
once per frame. `CloudGroup_Draw` then does exactly `phase += rate` every call
with no `dt` anywhere - a literal per-frame increment, the same "no scaling,
reproduced as shipped" treatment `exhaust.rs`'s `TRAIL_TAPER_RATE` gets.

**This retires the earlier "rate not pinned down" open item, but not in the
direction that item expected.** There is no single "the rate" to capture live -
every sprite gets its own draw from a fixed, narrow, symmetric range, refreshed
each time the display list is built. A live capture would have returned one
arbitrary sample, not a constant; the range itself is what a renderer needs,
and it is a static literal read directly off the instructions
(`vmul_s`/`vsin_s`/`vcos_s` play no role in generating it - `vcst_s(5)` is
`2/PI`, the VFPU's own radians -> quarter-turn conversion `vsin_s`/`vcos_s`
require, not a speed; see `docs/psp/allegrex-vfpu.md`).

`g_camera_roll` (`0x08ab10a8`, subtracted from the phase before the trig) is
not a second rate either: it is the camera's roll, written once per frame by
`Camera_SubmitScene` (`0x08878fe8`). See [`CloudGroup_Draw`](#cloudgroup_draw-0x0893280c)
above for what it does to the quad (92, live).

## `CloudGroup_CullOverlappingSprites` (`0x08932eac`) and `CloudGroup_SpritesOverlap` (`0x08932f98`)

**Confidence 85.** Before `CloudGroup_BuildDisplayList` bakes the sprite
buffer, `CloudGroup_CullOverlappingSprites` walks every pair of authored
records (`+0x184` is the authored count, `+0x18c` the authored array,
stride `0x20`) and marks a record dead (`+0x14` set to `-1`, decrementing
`+0x188`, the *drawn* count `CloudGroup_Draw` iterates) when
`CloudGroup_SpritesOverlap` returns true for it against an earlier one still
alive:

```c
// CloudGroup_SpritesOverlap(group, a, b):
threshold = (a.half_size + b.half_size) * (1.0 - group.overlap);
return threshold * threshold <= dot(a.position - b.position, a.position - b.position);
```

i.e. two sprites "overlap" (and the later one is culled) when their centres
are closer than `(sizeA + sizeB) * (1 - Overlap)` - `Overlap` `0.0` never
culls, `1.0` culls any pair at all, and `05_Track`'s authored `0.65` sits
between. **This is a pure function of the authored positions and sizes, no
randomness involved** - unlike the phase/rate/variant draws below, a
renderer reproducing this exactly would need only the same pairwise test, not
a live capture. `crates/fx/src/cloud.rs` does not currently reproduce it -
see its own doc comment.

## `CloudGroup_BuildDisplayList` (`0x08933ec4`) bakes one flat colour per sprite

The colour bake and its ramp input: **93** (2026-10-04). The mechanism and
the ramp's input are read, and every baked colour word on four boots was
reproduced from them exactly. The `names.tsv` rows for
`CloudGroup_BuildDisplayList` (85), `CloudGroup_SampleColourRamp` (80) and
`CloudGroup_CullOverlappingSprites` (85) keep their earlier confidences;
the reproduction would support raising them.

Contrary to a first read of `hi_colour`/`mid_colour`/`lo_colour`/`midpoint` as
a per-vertex gradient across the billboard quad, the baked vertex colour is
**one flat RGBA per sprite**, written into all six vertices of its quad.
`CloudGroup_BuildDisplayList` computes it by calling `CloudGroup_SampleColourRamp`
(`0x08932ba0`) twice - once at the sprite's world position with `y - half_size`,
once with `y + half_size` - and averaging the two results component-wise before
packing to a 32-bit colour.

`CloudGroup_SampleColourRamp` itself is a plain three-point piecewise-linear
lookup: breakpoints at `0.0`, `Midpoint`, and `1.0`, with values `LoColour`/
`LoAlpha`, `MidColour`/`(HiAlpha+LoAlpha)*0.5`, and `HiColour`/`HiAlpha`
respectively - so **`MidAlpha` is not an authored attribute at all**, it is
always the average of `HiAlpha` and `LoAlpha`, which is why
`CloudAttributes`/`clouds.md`'s own attribute table never lists one. The
lookup's fraction is clamped to `[0, 1]` exactly like a fog table's, and the
function signature and field layout (`+0x104`/`+0x114` as bounds, `+0x160..`
as an interpolation table with precomputed reciprocals) is structurally
identical to this project's own `Fog` interpolation - not a coincidence, given
`Fog_Disable`/`Fog_FindVolume` bracket the same draw.

**The input is height within the field.** `+0x104` and `+0x114` are the `y`
of the field's bounds that `CloudGroup_BuildField` writes (step 5 above):
the lowest `y - half` and the highest `y + half` over every authored record,
culled ones included. So `t = (y - min_y) / (max_y - min_y)`, clamped, and
`+0x17c` (a repeat period) is `0` on every shipped group, which skips the
wrap. Each channel of the average is multiplied by 255 and truncated, then
packed with red in the low byte (`A << 24 | B << 16 | G << 8 | R`), the GE's
8888 vertex colour.

## Open

- **The far-camera draw skip is not read.** Live, with the camera far from
  the clouds (the start line, section 29), the phases did not advance and the
  vertex buffer held uninitialised words. The test that skips the draw was not
  traced. The next address is whatever calls a `cloudGroup` node's `draw`
  slot (`0x08ad2a68`). `oag_fx::cloud` draws and advances every group
  every frame.
- **No live A/B picture.** The field's values are reproduced from RAM. A
  matched frame against PPSSPP's software renderer was not captured: placing
  the craft near the clouds (track index about 3000) with
  `psp-drive.py place` either was undone by the game's own reset or threw the
  craft off the track, on two boots. The colour word at vertex `+4` of
  `*(group+0x194)` can be zeroed in a paused emulator to hide the original's
  clouds, which gives an A/B mask once a stable pose is found. Ours, from a
  free camera at `(-158, -4, 239)` looking at the field, draws a steam bank
  behind the wall.
- **`FUN_0891e988`'s two integer arguments are not identified.** Forwarded
  unchanged to `FUN_088111cc`; plausibly a `Gu_TexFunc`-shaped call given its
  position in the state list, not confirmed. It decides how the vertex colour
  combines with the texel, which only a picture can check.
- **The eight `FUN_`-named slots beyond update/submit/draw/init on both
  tables** (`+0x54`, `+0x6c`, `+0x74`, `+0x84`) - not decompiled at all.
- **`Seed` is rolled per boot.** Every shipped group leaves it unset, so the
  original takes `Psys_RandIntRange(1, 9999)` after reseeding from the
  clock-seeded `rand()`. No boot's field can be predicted, only reproduced
  once its seed is read. `oag_fx::cloud::CHOSEN_SEEDS` are one boot's
  three draws, chosen, not measured.
- **The shared particle stream is not reproduced.** Building a group reseeds
  the generator every emitter draws from. Ours gives each group its own
  generator ([ADR-0056](../../../architecture/adr/0056-a-render-side-port-of-the-particle-generator.md)).
- **Pure and PS2 parity not checked.** Pure's class-ID table is renumbered
  (`vex.md`), so `0x3d8`/`0x3d9` there would look for the wrong class
  entirely; a real check needs Pure's own table read first.
