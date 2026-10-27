# `.gxp` - SceGxm shader programs (Wipeout 2048)

Status: **container understood; instruction stream classified and, for 21 of
its 23 used classes, listed** (2026-10-27). Wipeout 2048, PS Vita.

Every shader program the game runs is a `GXP\0` container, and there is no
`.gxp` file on the disc: they are embedded, in the executable and inside
`.rcsmaterial` files. **97,899 of 97,899 of them decode** - header, code
extents, the literal/uniform/container tables, and the parameter table with
its **names** - swept across both eboots and all three `.psarc` packages,
22,056 files. The GPU bytecode inside is classified word by word and listed
operand by operand for **97,799 of the 97,899 programs** (the other 100 hold a
load/store or an 8-bit packed multiply that is refused by name); see
[The instruction stream](#the-instruction-stream).

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

`oag_rcs::gxp` is the implementation and
`crates/rcs/tests/gxp_ground_truth.rs` is the sweep, `#[ignore]`d because
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

## The instruction stream

2026-10-27, lane `gxp-usse-decoder`. The stream is PowerVR SGX543 USSE code,
one 64-bit little-endian word per instruction, in two blocks per program: the
**primary** program (per vertex or per pixel) and the **secondary** program
(per draw call, run before it, writing the uniform registers). Its extent has
been exact since the container work; its **content** is now read.

```sh
python3 scripts/vita-usse.py disasm <eboot.elf|psarc[:entry]> <program index>
python3 scripts/vita-usse.py census <source> [...]
```

`oag_rcs::gxp::usse` (`crates/rcs/src/gxp/usse.rs`) is the Rust half: it names
the class of a word and says whether the listing reads its operands. The
listing itself is Python (`scripts/vita-usse.py`), the way
`scripts/ps3-microcode.py` is the reference for the PS3 half: nothing at
runtime consumes operands yet, and the census needs only classes. Operand
decode belongs in Rust when a renderer needs a shader's arithmetic.

### Classes (confidence 90)

A word's class is the first of **35 ordered layouts** whose fixed bits it
carries (the short-opcode classes are prefixes of the long ones, so order
matters). Every layout is 64 characters: fixed bits and named fields. Names
and listing syntax are ours.

**Census** (`crates/rcs/tests/gxp_ground_truth.rs`, over the same 97,899
programs and 22,056 files as the container; **5,883,746 words** in both
blocks, every one matching a layout; the 178 eboot programs - 111 base, 67
patch - included, and the Rust and Python censuses agree on every count):

| Class | Words | What it is | Listed |
| --- | ---: | --- | --- |
| `nmad32` + `nmad16` | 648,251 + 1,529,291 | vector add, multiply, min, max, fract, dot, derivative (f32 / f16) | yes |
| `pck` | 1,004,731 | pack / unpack between f32, f16, 8 and 16-bit integers, c10 | yes |
| `comp` | 706,227 | reciprocal, reciprocal square root, log2, exp2 | yes |
| `mad` | 705,429 | 3- and 4-vector multiply-add through the internal registers | yes |
| `mad2` | 532,342 | multiply-add, three sources (f32 / f16) | yes |
| `mov` | 302,346 | move, conditional move | yes |
| `bw` | 128,062 | and, or, xor, shifts, rotate (also the immediate load) | yes |
| `phase` | 98,654 | phase boundary (texture / feedback scheduling) | yes |
| `spec` | 90,417 | special marker | yes |
| `dp` | 61,957 | 3- and 4-component dot product | yes |
| `nop` | 47,565 | no-op | yes |
| `slsi` | 13,174 | sets the repeat increments of the next instructions | yes |
| `smp` | 11,762 | texture sample | yes |
| `tst` | 2,305 | test (sets a predicate) | yes, **sign / zero / combine fields raw** |
| `kill` | 753 | discard pixel | yes |
| `ldst` | 288 | load / store | **refused by name** |
| `imad32` | 144 | 16 x 16 to 32-bit integer multiply-add | yes |
| `sop2m` | 28 | two-source op with a mask | **refused by name** |
| `imad16` | 13 | 16-bit integer multiply-add | yes |
| `branch` | 3 | branch | yes, **raw offset** |
| `imad32x2` | 2 | 32-bit integer multiply-add | yes |
| `depthf` | 2 | depth feedback | yes |

Twelve of the 35 layouts never occur (`dual`, `sop2`, `sop3`, `imad8`,
`imad8x2`, `tstmsk`, `sbo`, `limm` and the four illegal opcodes); the test pins
that, so a corpus that grows one fails by name. **Zero words are unclassified**
(`seen.unclassified == 0`). Operands are listed for 5,883,430 of the
5,883,746 words (99.995%); 316 words in 100 programs are `ldst` or `sop2m` and
are refused by name. **The 178 eboot programs are listed in full**
(`census` on the two executables: 178 of 178; the brief's 111 + 67). The
`rcsmaterial` corpus is listed for 97,621 of 97,721 programs.

The earlier sweep on this page (323,937 programs, 17,377,433 primary words)
used a different source set, base and patch archives counted separately;
the numbers above are the 97,899 of the container census.

### How the layouts were established, and what they were checked against

The field maps and the operand semantics were taken from a reading of Vita3K's
USSE translator (GPL-2.0-or-later: read for facts about the hardware's
encodings; the tables here are re-expressed as masks, values and `(name,
lowest bit, width)` field lists under our own names, the listing code and its
syntax are handwritten, and mnemonics are ours) and from public SGX documentation. The
listing is then **checked against the emulator's own disassembly logs**:
a scratch harness (not landed, since it has to parse that emulator's own text) decodes every
distinct line of 589 `.dsm` logs (4,765 (word, text) lines, written by Vita3K runs of 2048's menus and
races; 552 programs have both log and container) and compares mnemonic and
operands:

| Class | Lines agreeing | Differing |
| --- | ---: | --- |
| `nmad` | 2,215 | 0 |
| `mad2` | 908 | 0 |
| `pck` | 557 | 5 (repeat steps) |
| `mov` | 282 | 29 (repeat steps) |
| `comp` | 249 | 0 |
| `mad` | 205 | 0 |
| `bw` | 115 | 13 (repeat steps) |
| `smp` | 96 | 6 (the log prints the lod operand as `INVALID`) |
| `dp`, `imad16`, `slsi`, `tst`, `kill`, `depthf`, `phase`, `spec`, `nop` | 33, 6, 19, 14, 4, 1, 2, 4, 2 | 0 |

Every non-repeating line of every sampled class agrees; the remaining
differences are the log printing one line per repeat step with register
numbers stepped (that is how the repeat count was confirmed: a program whose log is longer
than its block differs by repeated words). `slsi`, `kill`, `tst` and `spec` are compared on decode success
and mnemonic only, and **`imad32`, `imad32x2`, the integer forms of `tst`, and
`branch` have no log line to check against** (and the never-occurring `limm` is not listed at all) (confidence 60 on those operands,
90 on the others). Classes `ldst` and `sop2m` are the open part.

**Register naming and the constant bank (confidence 85).** `pa` primary
attributes (vertex inputs, or iterated varyings / sampled texels in a pixel
program), `sa` secondary attributes (uniforms and the sampler descriptors),
`r` temporaries, `o` outputs, `i` the internal registers, `c` a hardware
constant ROM (`c0.x` = 0, `c15.y` in f16 = 1.0). Vectors of f16 occupy
register *pairs*, so `pa2.xyzw` in f16 is words `pa2` and `pa3`; the number
printed is the 32-bit word. A uniform's `resource` field in the parameter
table **is its `sa` register**: on the composite program (blob #77) the
parameters read `bloomFactor` 0, `accumFactor` 2, `screenTintColour` 4,
`zoneEdgeColour` 6, and the code uses `sa6.xyzw` as a colour multiplier of the
edge strength and `sa4.xyzw` as the closing add.

**The literal record resolved (confidence 75; was 45).** An 8-byte record is
`(u32 word index, u32 value bits)`: the word is placed in the secondary
attribute file at that index within its container, and the value is read as
`f32` or as a pair of `f16` by whichever instruction uses it. On Canvas3D's
vertex program the records (indices 8 to 15) are `0`, `5.0`, `2.0`, `4.0`,
`1.0`, `3.0`, `1/6` as two f16 and `6.0`, which the code uses as the
permutation `v * 6 + {3, 1, 4, 2, 5, 0}` and an f16 `1/6` weight; on its
fragment program, `(1.0, 0.0)` and `0.005` as two f16. That is the 8-byte
record the container work found, and it is why the vertex program's constants
read as nonsense under a per-program-type f32 or f16 rule.

### The cross-checks (acceptance 4)

- **Blob #77 (`zoneEdgeColour`, base eboot, `wo_composite`)** lists as a
  three-tap edge detector: two differences of three `mainTex` taps, absolute
  values summed (`abs(a - b) + abs(a - c)`), dotted with a constant vector
  (from the constant ROM), scaled by `sa6` (`zoneEdgeColour`) plus a bias, clamped
  to `[0, 1]`, and blended; `screenTintColour` (`sa4`) is a final add. That
  agrees with the page's earlier reading that `zoneEdgeColour` is the Zone
  grade's own colour and `screenTintColour` is not.
- **`fc01_dummy`'s fragment program** (see
  [`2048-material-params.md`](2048-material-params.md)) lists as
  `sa2 = -fogColour + 0` in the secondary program and
  `out = pa0.x * sa2 + sa0 = fogColour * (1 - pa0.x)`, then alpha 1.0.
  **That corrects the 2026-10-19 reading** ("clamps the negated `fogColour`
  at zero, so the pixel is `fogColour` whatever the interpolant holds"): the
  op is an add (`op2 = 1`, which the oracle confirms as an add on 2,215 other
  words), not a max (`op2 = 6`). The interpolant is the vertex program's
  exponential-squared fog visibility: `o4.x = min(sa36.x, exp2(-(sa32.w *
  z)^2 * sa36.z))` with `z` the view depth (clip `w`), 1 near and 0 far. So the road is **black near and
  `fogColour` far**, not flat `fogColour`. Confidence 85 on the arithmetic
  (the operand decode agrees with the oracle), 60 on the interpretation of the
  vertex output as a fog visibility. No renderer is changed here; the owner of
  the 2048 material path should re-read what `psp2::FOG_FLAT` draws.

### Canvas3D's lattice shaders

Decoded and explained on
[`canvas3d-lattice.md`](../ghidra/functions/vita-2048-eu-v104/canvas3d-lattice.md).

### Omega cross-check

**Checked, differs.** Omega's programs are PS4 GCN (`.sb`), not USSE; none of
this decoder applies. Its eboot carries `Canvas3D_vp`/`Canvas3D_fp` as the
lattice page records, so the GCN half of the same law is the open work on
that title. 2048's DLC and Omega's re-shipped `.rcsmaterial` programs differ
only in this instruction set, so a law read here (the fog visibility above) is
checked against Omega's own program, never assumed.

### What is still unread

- `ldst` (288 words) and `sop2m` (28): refused by name, in 100 programs.
- `tst`'s sign, zero and combine fields are kept raw; `branch`'s offset unit.
- The unread header words and the per-program name binding on this page's
  earlier lists are unchanged.
- The `smp` `lod` and `fconv` fields are listed raw (`lod2` is the only mode
  the sampled programs use with a lod register); the gather and info modes
  never occur in the corpus.

### History: the opcode field was located first (2026-09-04)

What follows is the earlier derivation and its trap, kept because the trap
generalises. It is **superseded** where it says nothing is named.

### The opcode field is located, corpus-wide - nothing is named yet

`scripts/vita-gxp.py opcodes` (2026-09-04) sweeps every instruction word in
both regions' full corpus - both eboots, `data.psarc`/`data1.psarc`/
`data2.psarc`, `dlc1.psarc`, `dlc2.psarc` - 323,937 programs,
17,377,433 primary and 1,975,822 secondary instructions, each a 64-bit
little-endian word, bit-indexed by the word's own bit position (`bit 0` is
the LSB of the unpacked `u64`, not a position from any external ISA
reference). The base and patch archives are counted separately and likely
repeat much of the same shipped material, so the instruction *counts* below
overweight whatever both builds carry, but the *ratios* the findings rest on
are not sensitive to that. Two findings, both derived from the bytes here and
checked afterward against Vita3K's own USSE decoder as an **oracle, not a
source** - see the license note below:

- **Primary op1 is a variable-width prefix, not one fixed field width.** At
  5 bits, primary code resolves into 9 dominant values covering 99.7% of
  instructions. Widening to 6 bits leaves 8 of those 9 unchanged - each
  one's dominant 6-bit child still holds over 99.9% of it - but the largest,
  `00000` (1,750,703 instructions, 10.1% of the corpus), is not one opcode
  at 5 bits: it splits roughly 2:1 into `000001`/`000000` at bit 6, and
  several of the resulting groups keep splitting through at least 8 bits,
  never closing the clean way the other 8 groups do by 6-7 bits. **A first
  pass here read "9 of 9 groups cover 99.7%+ regardless of width" as a knee
  at 5 bits and was wrong** - the statistic it used (top-9 coverage) is
  monotone by construction across widths and can't show a split; following
  each dominant group to its own children is what catches one, the same
  "check the claim against the population it was read from" trap this
  project keeps re-finding elsewhere in its own reverse-engineering work.
  Secondary
  code shows the same shape at smaller scale: 8 of 9 dominant 5-bit groups
  are stable by 6 bits, and the ninth, `11111` (155,211 instructions, 7.9%
  of secondary code), splits roughly 86/14 at bit 6 before settling.
- **Secondary code's last instruction always sets bit 50, and by both of the
  two tests that can find an end bit - surviving a check for the vacuous
  case too.** All 247,049 non-empty secondary blocks have bit 50 set on the
  final instruction (the *sufficiency* test - it never fails to mark the
  end). Of the 227,944 programs where bit 50 fires on exactly one
  instruction, that one is the last on all 227,944 (the *discrimination*
  test - when it fires once, it is never early) - and restricting that test
  to the 230,422 blocks with **two or more** instructions, excluding the
  16,627 single-instruction ones where "fires once" and "fires on the last"
  are the same statement by construction, still gives 211,317 of 211,317,
  100%. The remaining 19,105 also set it on an earlier instruction, so it is
  not exclusively an end marker, but it never misses the end. Confidence
  **80**: an exact, corpus-wide closure by two independent tests that
  survives the vacuous-case check, but a located flag rather than a decoded
  meaning.
- **Primary code has no equally clean end bit, by either test.** The best
  discrimination candidate, bit 56, lands on the last instruction on 92.3%
  of the programs where it fires exactly once (261,370 of 283,088); the best
  sufficiency candidate, bit 31, covers 99.6% of programs but is usually also
  set on earlier instructions, reading more like a commonly-true field than
  a marker. No primary bit clears both bars the way secondary's bit 50 does -
  consistent with primary op1's own variable width above: an end flag whose
  bit position moves with the instruction format cannot show up as one clean
  bit across every format at once.

**Naming an opcode was not attempted then (superseded 2026-10-27), and the oracle rule
changed (maintainer, 2026-10-19).** The two findings above were derived from the bytes
and checked afterward against Vita3K's USSE decoder. The maintainer has since
approved reading Vita3K's frames, its behaviour, the `.dsm` shader logs it
writes and its source code as evidence. Everything that lands here is
handwritten: no opcode tables, bit-field names, mnemonics or code are pasted,
ported line by line or translated, and the `.dsm` files stay under
`data/scratch`, never tracked (Vita3K is GPL-2.0-or-later, and
[ADR-0024](../architecture/adr/0024-in-process-codecs-and-ffmpeg-as-a-last-resort.md)
still bars a strong copyleft from this project's sources). Each finding names
where it came from: the bytes, a frame, or a `.dsm`. First use:
`fc01_dummy`'s fragment program, read from the bytes and from the `.dsm`
logs of neighbouring programs, in
[2048-material-params.md](2048-material-params.md).

Two smaller things are also left unread on purpose:

- **The literal record's interpretation (resolved above, 2026-10-27).** An 8-byte record reads as
  `(u32 index, 4 raw bytes)` with the bytes being `f32` in vertex programs and
  a pair of `f16` in fragment ones, on three records read by hand.
  Confidence: **45**, so the tool exposes a count and a span and no values.
  `0x3dc6` (log2 e) and `0x3c00` (1.0) sitting next to a fog term in
  `cf_alpha4glow` is the suggestive case.
- **Which of an executable's programs carries which `_vp`/`_fp` name.** The
  ordinal correspondence is at 40 and nothing is named on it; the reasoning is
  in the handover thread on 2048's GXP containers (`just threads --category tooling`).

## Which sampler carries the colour (2026-10-19, `normal-maps-as-colour`)

**No instruction is read for this.** A `.rcsmaterial` holds a fragment program per
permutation of one shader source, and the compiler declares a sampler only in a permutation
that reads it. The smallest permutation declares `constantAmbientColour` and `fogColour` and
no `liveLighting*` uniform: the surface with the lights off. Over `data.psarc`'s 204 material
families that have both that permutation and a lit one, **eight families' ambient-only program
declares no sampler at all while their lit ones do**: `constantdiffuse_specular_normal` and
`New` (`Texture1`, `norm`), `ds_booth_glass` (`Texture1`), `fc15`/`fc17_effects_vscroll_water`
(`NormalMap`), `dc_seawater`, `dc_seawater_cheap` and `reflectplane_dc_seawater`
(`DiffuseTexture`, `DiffuseTexture1`). Their colour is the one other uniform that program
declares (`Constant1`, `ColourConstant`, `Watercolour`); the texture is a lighting lookup.
Confidence 80: the rule is structural, and it agrees on the one family HD's traced program
settles (`constantdiffuse_specular_normal`: `Texture1` only through a `2c - 1` unpack into
the light dots, `rcsmaterial.md`).

**The rule never looks at a name, and the names agree with it.** Every sampler it calls
lighting-only over the whole package (`psp2_sampler_use_ground_truth` pins the list): `Norm`,
`Normal`, `NormalMap`, `NormalAndSpecular`, `NormalMapTexture`, `NormalTexture`, `Normal_Map`,
`NormalAlphaMap`, `Spec`, `SpecularTexture`, `Specular_Texture`, `norm`, plus `Texture1/2/4`,
`screenSpaceRefractionTex` and the seawaters' `DiffuseTexture`/`DiffuseTexture1`. The last
two are the check that it is the programs and not the author's names that decide: the
seawater shaders call their wave normal map `DiffuseTexture` (`dc_waternormalmap`), and the
rule refuses it where a name rule (`Material::diffuse_texture`'s `DIFFUSE_SAMPLER_NAMES`)
bound it. Engine samplers (`lightmap`, `occlusionMap`, `shadowMap`, the reflection
captures) are lit-only by construction and left out.

**Only these eight are acted on.** The build refuses a sampler and draws the constant only where
`SamplerUse::colourless` holds (no material sampler in the ambient-only permutation). Many other
families carry an extra lit-only sampler beside a picture they do sample (`NormalMap`,
`Specular_Texture`, `NormalTexture` and so on, with colour uniforms that are not colours such as
`time` or `Bloom`): recorded here, **not wired**. The state word of `ds_booth_glass` (2048 and Omega) and Omega's
`reflectplane_dc_seawater` is `124`, mode 0, opaque, so the constant's alpha turns no see-through
draw opaque there; the other 2048 water files' words were not read.

**A permutation that does sample in the ambient-only case is a colour or a mask, not a
lookup, and the rule leaves it alone**: `m_diffuse_normal_constantspecular` declares
`Texture1` (`Norm` on `New`) there, and the Altima water edge declares `NormalMap` there
(beside an `Opacity` uniform). Both stay open.

**Omega: checked, applies, inherited** (`rcsmaterial.md`): its programs are PS4 and unread;
three of these families carry 2048's instance record exactly and take the answer.

**`m_diffuse_normal_constantspecular`, second pass (2026-10-20, `normal-maps-2`): still
open, with what was learned.** Read from the bytes plus the dictionary of 64-bit words
labelled by every `.dsm` Vita3K had written (a `.dsm` file name is the SHA-256 of the whole
GXP container):

- The family's ambient-only fragment program (#1 on Moa Therma, Anulpha, Vineta K; `NEW`
  on Chenghou is #1 with `Norm`) is four primary words: `PHAS`, two unlabelled
  `VF16MAD2`-shaped words, and the output scale. Compared with the five-word ambient
  programs of sibling families, its second word has the fog-mix shape (`pa2.xxxx` factor,
  `fogColour` as `sa4`) and its first word reads `pa0.wwww`, the sampled texel's alpha.
  So the sample's **alpha** reaches the colour, not an rgb picture; what it multiplies is
  not decoded (confidence 40). It is not classified.
- The texture is `ds_pipe_nc.gxt` (Vineta K, Moa Therma, Anulpha, Chenghou). The suffixes
  on the start-line set read `_cs` colour+spec, `_ne` normal+emissive, `_ns` normal+spec,
  `_nc` normal+constant; the texels read as a tangent-space normal map
  (`track_albedo::is_normal_map`). Name agreement only, not a program reading.
- On Vineta K the three draws (materials 550, 553, 554 of `track.rcsmodel`, state word 124)
  sit at (11, 37, -107), radius about 27, 28 units ahead of grid slot 1: the start line's
  chain strip, beside `ds_sfline_trench`, `ds_sfblend_ns` and `ds_sf_cs/ne`.
- **Vita3K compiled none of this family's fragment programs** over two laps of Vineta K
  from the grid (358 `.dsm` logs; the word-level search finds only the shared `lightmap`
  program #10), while `diffuse_normal_specular*` and the seawater programs of the same
  circuit did compile. Either the original does not draw these three draws in a race, or
  the log misses them; one race is not enough to say which. Ours draws a dark chain band
  across the road there; the original frames at the grid and at lap 2's line show plain
  road.

To close it: a Vita3K race on a circuit whose start line is reachable at a matched pose
(Chenghou's `ds_sf_ne`/trench draws are the largest set), then label words 1 and 2 of #1.

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
and what the shader does with it is in the bytecode, listed under
[The instruction stream](#the-instruction-stream).

## What this settles for 2048's shadows

[`docs/rendering/shadows.md`](../rendering/shadows.md) could name
`track_proximity_shadow_vp`/`_fp` and `shadowMap` from the string table and
read neither. With the container decoded:

- **Revised 2026-10-19:** the receiver reads as a depth compare. Vita3K's `.dsm`
  for `fc06_lambert_simple` #14 and `diffuse_normal_specular` #14 subtract the
  sample from the fragment's light-space depth and select between two values on
  the sign ([ship-shadows.md](../ghidra/functions/vita-2048-eu-v104/ship-shadows.md),
  confidence 70). The "not a depth map the renderer projects and compares" below
  is the container-only reading it supersedes; the single-channel finding stands.
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

## Sources for the instruction stream

Read, never copied (landed code and text are handwritten): Vita3K's USSE
translator (`vita3k/shader`: the instruction layouts, register banks, the
constant ROM and operand semantics) and its shader logs (`.dsm`) as the
oracle. No other documentation of the instruction set was used.

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
