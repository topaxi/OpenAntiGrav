# `.gxp` - SceGxm shader programs (Wipeout 2048)

Status: **container understood**. Wipeout 2048, PS Vita.

Every shader program the game runs is a `GXP\0` container, and there is no
`.gxp` file on the disc: they are embedded, in the executable and inside
`.rcsmaterial` files. **97,899 of 97,899 of them decode** - header, code
extents, the literal/uniform/container tables, and the parameter table with
its **names** - swept across both eboots and all three `.psarc` packages,
22,056 files. The GPU bytecode inside is located exactly and **not decoded**;
see [What is not read: the bytecode](#what-is-not-read-the-bytecode).

Two readings are stacked here and they agree:

- **First read 2026-08-30**, to answer one question - what the Zone colour
  grade does to the picture. It read the parameter tables of the base build's
  111 embedded programs off Vita3K's documented `SceGxmProgram`, and found the
  Zone composite. That work is [below](#the-composite-family-and-the-zone-one),
  unchanged.
- **2026-09-02**, the corpus decoder: every field re-derived from the bytes by
  corpus-wide survival, a closure argument, a census over the whole disc, and
  a Rust implementation. The two arrived at the same field assignment from
  opposite directions - documentation and arithmetic - which is why the header
  below is at 95 and not at 88.

Read it with [`scripts/vita-gxp.py`](../../scripts/vita-gxp.py), the reference
the Rust ports, the way
[`scripts/ps3-microcode.py`](../../scripts/ps3-microcode.py) is the reference
for [the PS3 half](rcsmaterial.md):

```sh
python3 scripts/vita-gxp.py census data/extracted/vita/PCSF00007/patch-v104/eboot.elf
python3 scripts/vita-gxp.py dump   <psarc>:<entry> 14
python3 scripts/vita-gxp.py grep   <psarc> shadow
```

`oag_formats::gxp` is the implementation and
`crates/formats/tests/gxp_ground_truth.rs` is the sweep, `#[ignore]`d because
it needs game content.

## Where they are

| Source | Files carrying one | Programs | Vertex | Fragment | Stray magics |
| --- | ---: | ---: | ---: | ---: | ---: |
| `base/eboot.elf` | 1 | 111 | 47 | 64 | 0 |
| `patch-v104/eboot.elf` | 1 | 67 | 32 | 35 | 0 |
| `base/PSP2/data.psarc` | 693 | 44,536 | 36,413 | 8,123 | 1 |
| `dlc1/PSP2/dlc1.psarc` | 327 | 30,777 | 24,582 | 6,195 | 1 |
| `dlc2/PSP2/dlc2.psarc` | 240 | 22,408 | 17,915 | 4,493 | 1 |
| **total** | **1,262** | **97,899** | **78,989** | **18,910** | **3** |

*(Of 22,056 files opened. The sweep reads every archive entry, not the
`.rcsmaterial` files it would be natural to sample - see
[the trap](#the-trap-this-page-exists-to-record). The three files holding only
a stray magic are not counted as carrying one.)*

**There are no `.gxp` entries in `data.psarc`** - checked against the whole
18,430-entry directory, whose census carries `.gxt`, `.at9`, `.vex`,
`.rcsmodel` and twenty other extensions but no shader files. The `.xfx` files
that look like a candidate by name are ship engine audio. They are found by
scanning for the magic instead, which is what the two shapes they ship in
require:

- **In an executable**, runs of back-to-back programs in the `.data` segment,
  each padded to a 4-byte boundary - three such runs in the patch build, the
  first at file offset `0x52628c`; the base build's run starts at `0x515f70`.
  **64 of the 66 gaps between consecutive programs are exactly
  `align4(size)`**, and the two that are not are the run boundaries, where
  unrelated `.data` sits between. That is the container-array closure: each
  program's own declared size predicts where the next one starts, 64 times
  running.
- **In a `.rcsmaterial`**, likewise back to back - `cf_alpha4glow.rcsmaterial`
  holds 86 - which is the same arrangement the PS3's `SHO` blocks have inside
  the same file type.

**The two builds embed different numbers of programs against the same names.**
The base executable holds 111 and the v1.04 patch holds 67, while both carry
the same 67 `_vp`/`_fp` shader-name strings. The natural reading is that the
patch stripped variants nothing registers, but that is a reading; what matters
here is that **sweeping one build would have made whichever it was look like
the number**, so the ground-truth test sweeps both.

**The three stray magics are not programs.** `GXP\0` is four bytes and a
1.7 GiB corpus contains it by accident; all three sit inside `.probes` files
and declare a "size" that runs past the file holding them. They are reported
as rejected magics rather than as failed decodes, so a real regression cannot
hide inside a known false positive. Confidence: **95**.

## The header, and the parameter table

Little-endian throughout, like the rest of this title's package and unlike
[`.rcsmaterial`](rcsmaterial.md)'s big-endian PS3 microcode.

The layout is the vitasdk/Vita3K-documented `SceGxmProgram`
([Vita3K's `gxm/types.h`](https://github.com/Vita3K/Vita3K), the community
reverse-engineering of Sony's format). **Every field below was also
independently re-derived from the bytes** by the sweep in
[the closure argument](#the-closure-argument) - the same double standard
[`gxt.md`](gxt.md) holds its own header to.

| Offset | Type | Field | Confidence |
| --- | --- | --- | ---: |
| `+0x00` | `u32` | magic `GXP\0` | 100 |
| `+0x04` | `u8` | major version - `1` on all 97,899 | 90 |
| `+0x05` | `u8` | minor version - `4` on all 97,899 | 90 |
| `+0x06` | `u16` | toolchain version - `0x0165` on all 97,899 | 60 |
| `+0x08` | `u32` | size, the whole program | 100 |
| `+0x0c` | `u32` | binary guid | 60 |
| `+0x10` | `u32` | source guid | 60 |
| `+0x14` | `u32` | `program_flags`; **bit 0 set = fragment** | 90 |
| `+0x24` | `u32` | parameter count | 100 |
| `+0x28` | `u32` | parameter table offset, self-relative | 100 |
| `+0x3c` | `u32` | primary instruction count | 95 |
| `+0x40` | `u32` | primary code offset, self-relative | 95 |
| `+0x44` | `u32` | secondary instruction count | 95 |
| `+0x48` | `u32` | secondary code offset, self-relative | 95 |
| `+0x4c` | `u32` | secondary code end, self-relative | 95 |
| `+0x58` | `u32` | uniform-image length, in 4-byte words | 85 |
| `+0x70` | `u32` | literal count | 85 |
| `+0x74` | `u32` | literal table offset, self-relative | 95 |
| `+0x78` | `u32` | count of the 8-byte records at `+0x7c` | 70 |
| `+0x7c` | `u32` | that table's offset, self-relative | 85 |
| `+0x80` | `u32` | count of the 4-byte records at `+0x84` | 75 |
| `+0x84` | `u32` | that table's offset, self-relative | 85 |
| `+0x88` | `u32` | count of the records at `+0x8c` - zero on all 97,899 | 50 |
| `+0x8c` | `u32` | that table's offset, self-relative | 85 |
| `+0x90` | `u32` | container count | 80 |
| `+0x94` | `u32` | container table offset, self-relative | 85 |

and each 16-byte `SceGxmProgramParameter`:

| Offset | Type | Field | Confidence |
| --- | --- | --- | ---: |
| `+0x00` | `i32` | `name_offset`, **self-relative to the parameter struct**; `0` means unnamed | 95 |
| `+0x04` | `u8` | `category:4`, `type:4` - low nibble first | 90 / 85 |
| `+0x05` | `u8` | `component_count:4`, `container_index:4` | 90 / 80 |
| `+0x08` | `u32` | `array_size` - `1` for a scalar or vector, `4` for a `float4x4` | 85 |
| `+0x0c` | `u32` | `resource_index` - register, or texture unit for a sampler | 85 |

**A "self-relative" offset is the field's own position plus its value**, not
an offset from the start of the program. That is not a stylistic detail: read
as absolute, none of the tables land anywhere and the closure below fails on
every program.

Every other header word is **left unread**, per
[the confidence rubric](../reverse-engineering/confidence-rubric.md) - below 50
nothing gets a name. `+0x2c`, `+0x30`, `+0x38`, `+0x5c`, `+0x64`, `+0x68`,
`+0x98` and `+0xa8` all carry non-constant values and none of them survived a
sweep for a meaning.

### Category and type, checked against the names they carry

`category` is `ATTRIBUTE, UNIFORM, SAMPLER, AUX_SURFACE, UNIFORM_BUFFER`;
`type` is `F32, F16, C10, U32, S32, U16, S16, U8, S8, AGGREGATE`. Four
category values and three type values occur, over 1,167,316 parameters:

| `category` | `type` | Count | Examples |
| ---: | ---: | ---: | --- |
| 0 `ATTRIBUTE` | 0 `F32` | 292,835 | `position`, `normal`, `uv1`, `lightmapUV` |
| 1 `UNIFORM` | 0 `F32` | 489,361 | `viewProj`, `fogColour`, `eyePositionWorldSpace` |
| 1 `UNIFORM` | 1 `F16` | 341,049 | `offset0`..`offset3`, `bloomFactor` |
| 2 `SAMPLER` | 0 `F32` | 43,999 | `lightmap`, `DiffuseMap`, `occlusionMap`, `shadowMap` |
| 4 `UNIFORM_BUFFER` | 9 `AGGREGATE` | 72 | unnamed, every one |

Three things make this a check rather than a transcription of the enum.

- The names are in the file and `position` is not a sampler, which pins
  `category` directly.
- **`type == 9` occurs on exactly the 72 `UNIFORM_BUFFER` parameters and
  nowhere else.** `AGGREGATE` is index 9 of the documented enum and a uniform
  buffer is exactly what an aggregate parameter is, so two independent facts
  land on each other.
- **A `type == 1` parameter occupies exactly half the registers a `type == 0`
  one of the same width does**, which is what half precision means and is not
  something the enum's ordering could have produced. Measured on the
  `resource_index` gap between adjacent uniforms in the same program:

  | `type` | components | array | next register | pairs |
  | ---: | ---: | ---: | ---: | ---: |
  | 0 `F32` | 3 | 1 | **+4** | 32,505 |
  | 0 `F32` | 4 | 1 | **+4** | 99,435 |
  | 0 `F32` | 4 | 4 | **+16** | 55,985 |
  | 1 `F16` | 3 | 1 | **+2** | 102,412 |
  | 1 `F16` | 4 | 1 | **+2** | 36,289 |
  | 1 `F16` | 3 | 6 | **+12** | 232 |

  A `float3` or `float4` takes a whole four-slot register; a half3 or half4
  takes two, and an array scales linearly from there.

Confidence: **90** for `category`, **85** for `type`.

`component_count` takes 0-4, which is what a component count can be, and is 0
on precisely the aggregates.

**A zero name offset is *no name*, not a name at position zero.** Read as an
offset it points back at the parameter's own entry and the name region stops
closing - which is what the 72 aggregate entries look like when it is read
wrong. A skinned material's 6,144-element palette buffer is the case in hand,
unnamed while the `skinPalette` uniform that indexes it is named right above
it. Confidence: **85**.

## The closure argument

Nothing in the tables above rests only on the published structure. Each field
is the **only** assignment that survived the whole corpus, and the test that
did the separating is arithmetic that cannot come out even unless the reading
is right.

### Step 1: which word is the parameter count

Sweeping every 4-byte-aligned header word for a pair `(count, offset)` under
which the parameter table lands inside the program and every entry's name
resolves to a NUL-terminated ASCII string leaves **three** survivors:
`(+0x20, +0x28)`, `(+0x20, +0x2c)` and `(+0x24, +0x28)`.

Two of the three are **vacuous**: `+0x20` is zero, and zero parameters fit
anywhere. Adding one requirement - **the last parameter name's NUL is the
program's last byte** - decides it 3,540 to 0. `(+0x24, +0x28)` closes on
every program that declares a parameter; the other two close on none.

That is also what fixes the parameter stride at 16 and the name offsets as
self-relative: a stride of 12 or 20, or an absolute name offset, does not land
the last NUL on the size field's own value 3,540 times. Confidence: **95**.

### Step 2: the six tables tile the space exactly

Between the primary code's end and the parameter table sit six spans, each
declared by a `(count, offset)` pair or derived from one:

```text
literals            8 bytes per record,  count at +0x70, offset at +0x74
uniform image       4 bytes per word,    count at +0x58, immediately after
table at +0x7c      8 bytes per record,  count at +0x78
table at +0x84      4 bytes per record,  count at +0x80
table at +0x8c      empty on all 97,899, count at +0x88
containers          8 bytes per record,  count at +0x90, offset at +0x94
```

They **tile that space with no gap and no overlap on all 97,899 programs**.
That is one check on six strides at once: one wrong stride shifts every table
after it and the chain stops meeting the parameter table.

**Their order is not fixed and is not assumed.** The table at `+0x7c` follows
the containers on the 72 skinned programs that have one
(`fc06_lambert_simple_bonecount2.rcsmaterial`, three circuits) and sits empty
at the parameter table on the other 97,827. The check sorts the six by start
rather than hard-coding a layout - which is how those 72 stopped being
"failures" and became the evidence for `+0x78`. Confidence: **85**.

### Step 3: `+0x74` states the primary program's end a second time

`0x74 + u32@0x74` equals `primary_offset + 8 * primary_instruction_count` on
**all 97,899 programs**. Two independent statements of the same address agree
every time, and that is what fixes the **USSE instruction width at 8 bytes**
without decoding a single instruction: at 4 or 16 the two disagree
immediately. The same shape holds for the secondary program, whose `+0x4c`
end field equals `offset + 8 * count` on all 97,899. Confidence: **90**.

### What is left over

`primary_offset - secondary_end` is **4 on 97,144 programs and 8 on the other
755, and never zero** - alignment padding between the last code block and the
primary program, 394,616 bytes over the corpus. It is reported rather than
enforced: a third width would be a fact about the container worth learning,
not a decode failure.

### The fragment bit

Bit 0 of `+0x14` is the whole of the vertex/fragment test, and the evidence
for it is one check, not two:

- **Within a program.** A vertex program declares vertex inputs and a fragment
  program cannot. Every program with the bit clear declares at least one
  `ATTRIBUTE` parameter and every program with it set declares none -
  **97,899 of 97,899, no exceptions.** Confidence: **90**.
- **A count that agrees, and does not generalise.** In the patch build the bit
  splits its 67 programs 32 / 35, and that executable's string table holds 32
  `_vp` and 35 `_fp` names. That looks like a second, independent
  confirmation - and the base build refuses it: 111 programs, 47 / 64, against
  the same 67 names. So the container-to-name correspondence is **not
  structural** and the count match is suggestive at **40**. It is recorded
  because it is true, and explicitly *not* the reason the bit is believed.

## The trap this page exists to record

**`+0x20` is zero on all 44,603 programs in one eboot and `data.psarc`, and is
not zero.** Sweeping `dlc1.psarc` and `dlc2.psarc` as well turns up 24
programs carrying `1` or `0x112` there. A first pass named it `always_zero`
and enforced it, which is a check that passes on two thirds of the corpus and
fails on the rest for no reason a reader could act on.

It stays **unread**. The general lesson, and the reason
`scripts/vita-gxp.py census` walks every entry of every archive and both
eboots rather than the `.rcsmaterial` files it would be natural to sample:
*a field that is constant over a sample is not a constant.* The 111-against-67
split between the two builds is the same lesson at file scale.

## What is *not* read: the bytecode

The instruction stream is PowerVR SGX543 USSE, and nothing here decodes it.
Its **extent** is now exact - both programs close on their own declared
counts, at 8 bytes an instruction - and its **content** is handed back as
bytes. So a program's **declared interface** is recoverable - its uniforms,
samplers, attributes, their types and component counts - and **what it
computes is not**. Every claim on this page and in
[effectsettings.md](effectsettings.md) about a shader's *arithmetic* is
therefore either absent or explicitly marked as unrecovered.

This is where the PS3 pair still goes further:
`oag_formats::rcsmaterial::fragment` decodes RSX microcode, 37,461 of 37,461
blocks. Doing the same for USSE is the open thread.

Two smaller things are also left unread on purpose:

- **The literal record's interpretation.** An 8-byte record reads as
  `(u32 index, 4 raw bytes)` with the bytes being `f32` in vertex programs and
  a pair of `f16` in fragment ones, on three records read by hand.
  Confidence: **45**, so the tool exposes a count and a span and no values.
  `0x3dc6` (log2 e) and `0x3c00` (1.0) sitting next to a fog term in
  `cf_alpha4glow` is the suggestive case.
- **Which of an executable's programs carries which `_vp`/`_fp` name.** The
  ordinal correspondence is at 40 and nothing is named on it; the reasoning is
  in the handover thread this page's index line points at.

## The composite family, and the Zone one

*Read 2026-08-30, against the base build. Blob indices below are that build's;
the patch build renumbers them, and both are given.*

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

**In the v1.04 patch the same program is blob #39 at `0x52a9c0`**, 634 bytes,
and is still the only one of that build's 67 declaring `zoneEdgeColour`
(`resource_index` 6 in both builds). A blob index without its executable is a
stale reference waiting to happen; both are recorded for that reason.

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

## What this settles for 2048's shadows

[`docs/rendering/shadows.md`](../rendering/shadows.md) could name
`track_proximity_shadow_vp`/`_fp` and `shadowMap` from the string table and
read neither. With the container decoded:

- **`shadowMap` is a single-channel sampler**, `comp = 1` on all **1,799**
  programs that bind it, at texture unit 3, immediately beside `lightmap`
  (4,467 programs, unit 1, four channels) and `occlusionMap` (1,895 programs,
  unit 2). So the track's shadow term is a **one-channel texture sampled per
  pixel next to the lightmap**, not a depth map the renderer projects and
  compares - consistent with the "track proximity" the shader pair's name
  claims and inconsistent with HD's shadow-map pipeline, which the same page
  had already shown 2048 does not carry.
- **`ShadowColour`, `ShadowSelect` and `ShadowAlpha` are uniforms, with no
  sampler beside them** - `Ships/materials/Debris.rcsmaterial`. The
  `ShadowColour` technique the executable's technique ladder names is a flat
  colour with a select and an alpha, not a lookup.
- **`track_proximity_shadow_fp` binds no parameter whose name contains
  "shadow".** Whatever it draws, it does not sample a named shadow texture.

## Related

- [`rcsmaterial.md`](rcsmaterial.md) - the PS3 sibling, and the file type that
  holds 97,721 of these on the Vita. **Every** archive entry carrying a
  container is a `.rcsmaterial` - 693, 327 and 240 of them across the three
  packages, and no entry of any other extension holds one.
- [`gxt.md`](gxt.md) - the Vita's texture container, the same little-endian
  package.
- [`2048-status.md`](2048-status.md) - what else the Vita disc holds.
- [`effectsettings.md`](effectsettings.md) - what the Zone read was for.
- [`../rendering/shadows.md`](../rendering/shadows.md) - what this unblocked.
