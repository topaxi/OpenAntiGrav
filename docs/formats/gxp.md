# `.gxp` - SceGxm shader programs (Wipeout 2048)

Wipeout 2048 ships **111 compiled SceGxm shader programs embedded in its
executable**, not as files on disc. This page is what was needed to answer one
question - what the Zone colour grade does to the picture - and it stops
there: the parameter tables are read, the GPU bytecode is not.

**First read 2026-08-30.** No Vita shader binary had been opened in this
project before.

## Where they are

`data/extracted/vita/PCSF00007/base/eboot.elf`, 111 blobs with the magic
`GXP\0`, the first at file offset `0x515f70` and the run continuing to about
`0x520000`. **There are no `.gxp` entries in `data.psarc`** - checked against
the whole 18,430-entry directory, whose census carries `.gxt`, `.at9`,
`.vex`, `.rcsmodel` and twenty other extensions but no shader files. The
`.xfx` files that look like a candidate by name are ship engine audio.

## The header, and the parameter table

The layout is the vitasdk/Vita3K-documented `SceGxmProgram`. The fields this
project reads:

| offset | field | note |
| ---: | --- | --- |
| `+0x00` | `magic` | `GXP\0` |
| `+0x08` | `size` | bytes, excluding trailing padding |
| `+0x14` | `program_flags` | |
| `+0x24` | `parameter_count` | |
| `+0x28` | `parameters_offset` | **relative to this field**, so the table is at `+0x28 + value` |

and each 16-byte `SceGxmProgramParameter`:

| offset | field | note |
| ---: | --- | --- |
| `+0x00` | `name_offset` | signed, **relative to the parameter struct**, not the file |
| `+0x04` | `category:4`, `type:4` | low nibble first, little-endian bitfield |
| `+0x05` | `component_count:4`, `container_index:4` | |
| `+0x08` | `array_size` | |
| `+0x0c` | `resource_index` | |

`category` is `ATTRIBUTE, UNIFORM, SAMPLER, AUX_SURFACE, UNIFORM_BUFFER`;
`type` is `F32, F16, C10, U32, S32, U16, S16, U8, S8, AGGREGATE`.

**All 111 blobs parse cleanly against this** - every category and type in
range, every name a readable ASCII string - which is the check that the
layout above is right rather than merely plausible. Confidence **88**.

Structure source: [Vita3K's `gxm/types.h`](https://github.com/Vita3K/Vita3K),
the community reverse-engineering of Sony's format; cross-checked by the
clean parse rather than taken on trust.

## What is *not* read: the bytecode

The instruction stream is PowerVR SGX543 USSE, and nothing here decodes it.
So a program's **declared interface** is recoverable - its uniforms,
samplers, attributes, their types and component counts - and **what it
computes is not**. Every claim on this page and in
[effectsettings.md](effectsettings.md) about a shader's *arithmetic* is
therefore either absent or explicitly marked as unrecovered.

## The composite family, and the Zone one

Three programs share the post-processing composite shape, distinguished by
which uniforms they declare:

| blob | file offset | size | parameters |
| ---: | ---: | ---: | --- |
| #75 | `0x51e690` | 619 | `bloomFactor[4]`, `luminanceFactor[4]`, `screenTintColour[3]`, `accumFactor[1]`; samplers `mainTex`, `alphaTex`, `bloomTex` |
| #76 | `0x51e8fc` | 458 | `bloomFactor[4]`, `accumFactor[1]`, `screenTintColour[3]`; samplers `mainTex`, `bloomTex` |
| **#77** | **`0x51eac8`** | **650** | `bloomFactor[4]`, `accumFactor[1]`, `screenTintColour[3]`, **`zoneEdgeColour[3]`**; samplers `mainTex`, `alphaTex`, `bloomTex` |

**Blob #77 is the Zone composite**: it is the *only* program of the 111 that
declares `zoneEdgeColour`, established by enumerating every uniform and
sampler name in all 111 blobs. The executable's own shader-name table
(`wo_composite_zone_fp`/`_vp`, `wo_composite_zone_hdfury_fp`/`_vp`, beside
`wo_composite_vp`, `wo_bloom_gate_*`, `wo_blur_*`) is the naming side of the
same family - see
[zone-environment-fallback.md](../ghidra/functions/vita-2048-eu-v104/zone-environment-fallback.md).

## How the engine binds the two colours - traced, and not what was expected

Each uniform name exists **twice** in the executable: once inside its blob's
own parameter-name table, once as a code-side string the engine looks the
parameter up by. `screenTintColour` is at `0x81424e18` and `zoneEdgeColour`
at `0x81424e38`, both used only in `FUN_81037670`, which caches the returned
resource indices:

```
81037d38  movw/movt r1, #0x81424e18   ; "screenTintColour"
81037d40  blx  0x813f22b0             ; sceGxmProgramFindParameterByName
81037d5a  str  r2, [0x8151c650]       ; cached index

81037d66  movw/movt r1, #0x81424e38   ; "zoneEdgeColour"
81037d6e  blx  0x813f22b0
81037d88  str  r3, [0x8151c654]       ; cached index
```

and the per-frame writer pairs each cached index with a colour:

```
8103a9b8  movw r1,#0xc650             ; screenTintColour's index
8103a9be  vldr.32 s0,[sp,#0x78]       ; <- staged from 0x816af060
8103a9cc  vstr.32 s0,[r2]

8103a9d6  movw r1,#0xc654             ; zoneEdgeColour's index
8103a9e4  vldr.32 s0,[sp,#0x80]       ; <- staged from 0x816af070
8103a9ee  vstr.32 s0,[r2]
```

where `sp+0x78`/`sp+0x80` were filled at `0x810394a0`/`0x810394b2` from
`0x816af060` and `g_zone_blended_stage_colour` (`0x816af070`) respectively.

**So the blended Zone stage colour drives `zoneEdgeColour`, not
`screenTintColour`.** Confidence **85** - two independent copies of each
name, cached indices matched at both the bind site and the use site, and the
two colours staged in the same order they are consumed. The whole-frame tint
takes a *different* global (`0x816af060`, written by `FUN_8103875c` and
`FUN_81038780`, unchased), so **whether the frame-wide tint is also
Zone-driven is not established**.

That is a narrower result than the name "composite grade" suggests, and it is
recorded that way deliberately: an *edge* colour is not a full-screen tint,
and what the shader does with it is in the bytecode this page does not read.
