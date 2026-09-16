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
| `cloudGroup`'s draw is a rotating, camera-facing billboard per instance | **85** - the shape and the rate's *source* are both read; see below |
| Each sprite's rotation rate is a per-instance random draw, not a shared constant | **88** - `Psys_RandFloatRange` call read directly |
| The GE state for the draw (blend, depth, fog, lighting, cull) | **90** - read off `CloudGroup_ApplyDrawState`'s literal `Gu_*` calls |
| Each sprite gets one flat baked colour, not a per-vertex gradient | **82** - the bake and the average are read; the ramp's *input* is not (see [Open](#open)) |

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

## `CloudGroup_Draw` (`0x0893280c`) - left unnamed

**Confidence 65 - below this project's naming floor for the verb in a name,
not for the finding itself.** Left as `FUN_0893280c`, per
[ADR-0005](../../../architecture/adr/0005-ghidra-conventions.md)'s "below 70,
`_q`" rule read together with the project's stated preference to not dress a
structural read as a verified one when the exact math is still open; the
*shape* below is read directly off the disassembly, not guessed.

What is legible:

- Copies the camera's current view-matrix stack entry
  (`param_2 + *(param_2+0x1694)*0x40 + 0x1410`) onto a small internal stack,
  the same push/pop-a-matrix-stack idiom `exhaust.md` documents for
  `ExhaustFlare_Draw`.
- For each of `*(param_1+0x188)` sprite records (stride `0x20`, source array
  at `*(param_1+400)`): advances a per-sprite phase accumulator
  (`fVar25 = record.phase + record.rate`), computes `vsin_s`/`vcos_s` of
  `(fVar25 - DAT_08ab10a8) * vcst_s(5)`, and uses the sin/cos pair to rotate a
  quad's corner offsets before transforming them through the copied view
  matrix (`vtfm4_q`) - a **rotating**, camera-facing billboard, not a static
  one.
- Binds the shared texture (`Gfx_BindTexture(DAT_08ac01b8)`) once for the
  whole batch.
- Writes output vertices at stride `0xc0` (192 bytes) per sprite - four
  times a plausible 48-byte vertex, though the exact per-vertex field layout
  (texcoord, colour, which of the two rotated basis vectors goes where) is
  not traced field-by-field.
- Restores the view-matrix stack (`Gu_SetMatrix(1, ...)`) afterwards.

This is why a future `oag_render` cloud module (not written yet) cannot
simply reuse `exhaust::sprite`'s static camera-facing quad and call the
recovery done: the
original's billboard *rotates*, on a per-instance phase and a rate this page
does not pin down (`DAT_08ab10a8` and `vcst_s(5)`'s exact values were read as
addresses, not as the seconds-per-radian constant a renderer needs). A static
substitute would be a real simplification, not a faithful reproduction, and
this page says so rather than letting a renderer's own doc comment be the
only place that is recorded.

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
| `+0x1c` | texture-variant index | `Psys_RandIntRange` into a small UV sub-region table, keyed by `cloudCube.kind` (always the `kind==2` branch on every shipped instance) |

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

`DAT_08ab10a8` (subtracted from phase before the trig) is not a second rate
either: `Camera_SubmitScene` (`0x08878fe8`/`0x0887905c`/`08879064`) writes it
once per frame as the camera's own current heading angle, derived from the
camera basis via an `atan2`-shaped call (`FUN_0897ed98`) over its
Gram-Schmidt-orthogonalised right vector. Subtracting it counter-rotates the
billboard against the camera's own turning, so each cloud's slow spin holds a
stable *world* orientation rather than appearing to spin faster or slower as
the camera yaws - confidence **80**, one live-read global, one binary, no
second corroboration attempted.

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
a live capture. `crates/render/src/cloud.rs` does not currently reproduce it -
see its own doc comment.

## `CloudGroup_BuildDisplayList` (`0x08933ec4`) bakes one flat colour per sprite

**Confidence 82** for the mechanism, **not asserted** for what the ramp's
input actually measures - see [Open](#open).

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

## Open

- **The colour ramp's input variable is not resolved.** `CloudGroup_SampleColourRamp`
  clamps some fraction derived from `(sample - +0x104) / (+0x114 - +0x104)` to
  `[0, 1]`, and the two sample points passed to it differ only in world `y`
  by `+/- half_size` - consistent with an altitude-like or distance-like input,
  but the writer of `+0x104`/`+0x114` on the cloud instance was not traced in
  this pass. **This module does not guess the axis** - see
  `crates/render/src/cloud.rs`'s doc comment for what the renderer does
  instead (one representative colour, not the ramp).
- **`FUN_0891e988`'s two integer arguments are not identified.** Forwarded
  unchanged to `FUN_088111cc`; plausibly a `Gu_TexFunc`-shaped call given its
  position in the state list, not confirmed.
- **The texture-variant sub-tables (`DAT_08ac01bc`/`DAT_08ac01cc`/`DAT_08ac01d4`/
  `DAT_08ac01dc`) are not decoded.** Each holds UV sub-regions of the shared
  `128x64` cloud texture, selected per sprite by `cloudCube.kind`; only the
  `kind==2` branch (`DAT_08ac01d4`, a 2-entry table) is ever exercised by
  shipped data. Not blocking a renderer that draws the whole texture per
  sprite rather than a sub-region.
- **The eight `FUN_`-named slots beyond update/submit/draw/init on both
  tables** (`+0x54`, `+0x6c`, `+0x74`, `+0x84`) - not decompiled at all.
- **`cloudCube`'s `kind` (always `2`) and `scale` (always `1.0`).** Ten
  shipped instances give no variation to test a hypothesis against; whether
  `kind` selects a sprite variant or `CloudGroup_Draw`'s per-record count is
  something else entirely is open.
- **`Seed`'s random draw is not reproduced.** Every shipped instance is
  unset, so the original assigns a fresh `Psys_RandIntRange(1, 9999)` per
  boot; this project's parser reports the authored `0.0` rather than
  simulating that draw, which is the right default for a deterministic
  reimplementation. A shipped cloud's *exact* jitter is therefore not
  reproducible - but a cloud with a *known* seed now is: the generator is
  read on [prng.md](prng.md), and the build reseeds it from `Seed`
  immediately before the phase/rate/variant draws, so a port of
  `PsysRng_Next` fed the same seed lands the same sprites.
- **Pure and PS2 parity not checked.** Pure's class-ID table is renumbered
  (`vex.md`), so `0x3d8`/`0x3d9` there would look for the wrong class
  entirely; a real check needs Pure's own table read first.
