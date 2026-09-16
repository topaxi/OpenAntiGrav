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
| `cloudGroup`'s draw is a rotating, camera-facing billboard per instance | **65**, structural only - see [Open](#open) |

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
this project's parser does not (and, being a boot-time RNG draw with no
recorded algorithm here, currently cannot) reproduce.

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

## Open

- **`CloudGroup_Draw`'s exact rotation rate and per-vertex layout.** The
  phase, its rate of change, and `vcst_s(5)`'s value were not read live; a
  breakpoint capture (the method `weatherpos.md` and `exhaust.md`'s flare-size
  correction both used) would settle it in one session.
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
  reimplementation but means a shipped cloud's *exact* jitter is not
  reproducible even if the rest of `CloudGroup_Draw` were.
- **Pure and PS2 parity not checked.** Pure's class-ID table is renumbered
  (`vex.md`), so `0x3d8`/`0x3d9` there would look for the wrong class
  entirely; a real check needs Pure's own table read first.
