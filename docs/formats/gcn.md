# GCN shader programs (Omega, PS4)

Status: **understood; every program on the disc decodes** (2026-10-27,
`gcn-operand-decoder`). Wipeout: Omega Collection, PS4.

Every shader program Omega runs is GCN microcode inside a `Shdr` wrapper: there
is no shader file on the disc. They sit inside the `.rcsmaterial` files (many per
file) and in `eboot.bin` (the post-processing set). `oag_rcs::gcn` finds each
`Shdr` tag, decodes the code to instructions **with operands**, and reads the
reflection table that names the constant-buffer members, textures, samplers and
interpolants.

```sh
cargo run -p oag-rcs --example gcn_dump -- <archive.psarc> <entry>   # every program in a material
cargo run -p oag-rcs --example gcn_dump -- <eboot.bin> --summary      # one line a program
cargo run -p oag-rcs --example gcn_dump -- <file> [<entry>] --at 0x31e4
```

`s_buffer_load` lines are annotated with the constants they load, from the
program's own reflection (dword offsets map to member byte offsets / 4).

## Sources

- **AMD, *Sea Islands Series Instruction Set Architecture*, revision 1.0
  (February 2013)**, chapter 13 "Microcode Formats": every encoding prefix, field
  and opcode number in `crates/rcs/src/gcn/isa.rs` and `opcodes.rs`, cited per
  section. The PS4's GPU is a GCN 1.1 (Sea Islands) part. Two of chapter 13's
  field tables contradict the rest of the manual, and the manual's other table
  wins both times, as the disc does: SMRD `OP` is `[26:22]` (table 7.1; 13.2
  prints `[26:21]`), and MUBUF `OP` is `[24:18]` (table 8.2; 13.6 prints `[15:8]`).
- **AMD, *"Vega" Instruction Set Architecture* (28 July 2017)**, sections 12.10,
  13.3.6 to 13.3.9: the VOP3P, SDWA and DPP words below, which are not Sea Islands.
- Handwritten; no emulator source was read for the decoder. LLVM has no Sea
  Islands disassembler (`llvm-mc -mcpu=bonaire` refuses).

## The container

```text
Shdr header (offsets from the tag)
+0x00  "Shdr"
+0x10  u32  low 24 bits: bytes from the code's start to the footer's end
+0x18  u32  the code's start, relative to +0x10
+0x4c  input-usage records, 4 bytes each (u8 kind, u8 API slot, u8 register, u8 flags),
       up to the first record of kind 0x0f (the fixed tail 0f 11 13 15) - see below
+0x38  to the code, on a vertex program: input-usage records of varying length,
       one 4-byte record per vertex input (u8 slot, u8 first VGPR, u8 size, u8 0),
       then exports. The rest of the header is unread.
code
+0x00  s_mov_b32 vcc_hi, N     ff 03 eb be: the footer is (N + 1) * 8 bytes on
...    to s_endpgm, exactly the footer's code length
then bytes up to the footer (unread)
OrbShdr footer, 0x1c bytes
+0x00  "OrbShdr", +0x07 u8 version (7)
+0x08  u32  bits 2-5 stage (1 vertex, 0 pixel, 4 on 12 eboot programs); bits 8-31 code length
reflection, right after the footer
+0x00  u32 resources (constant buffers and textures, 6 words each)
+0x08  u32 constants (buffer members, 9 words each)
+0x0c  u32 samplers (4 words each)
+0x10  u8 inputs, u8 outputs (4 words each)
+0x18  u32 string-table size
       +0x04 and +0x14 zero on every material program
records in that order, then the string table
resource: slot, size, kind (0x6a0416 constant buffer, 0x70502/0x30502 texture), members, first member, name
constant: kind, byte offset, size, 0, -1, 0, -1, name, type name
sampler:  0, 0, slot, name
input/output: kind (byte 0 components - 1, byte 3 slot), 0, name, semantic
```

Names are `i32` offsets relative to their own field. Confidence **90** for the
code bounds and the reflection order, 75 for the vertex-input records (below).

**What a program is held to** (`Program::parse`): the code marker's literal lands
exactly on `OrbShdr`; the header's span agrees with it; the code decodes from its
first byte, every instruction in step, to `s_endpgm` at exactly the footer's
length. The reflection is held to every name landing inside the string table and
every member range inside the constant table; a reflection that does not close is
a named error on the program, not a refused program.

**Vertex inputs.** The fetch shader the driver builds fills VGPRs before the
program runs; the header says which. The input-usage records before that table
vary in length (some are 8 bytes), and no header word was found to count them,
so the reader finds the input records as the one run whose slot bytes are the
reflection's input slots in order. Checked: on Tech de Ra's track program,
`position` lands in `v4..v6`, and those registers are exactly what the `world` and
then `viewProj` multiply reads before `exp pos0`.

## Input-usage records, and which texture a program decodes as a normal map

2026-10-27, `omega-normal-map`. The header holds, from `+0x4c`, four-byte records `(kind, API slot, register, flags)`, read
against the `s_load` offsets and the reflection of `Diffuse_Normal_Specular` and `Mageffect08_LH_1`
(`oag_rcs::gcn::Usage`). Kind 0 is a texture (T#) at the SGPR `register` (inline in the user data), 1 a sampler (S#), 2 a
constant buffer, `0x13` a pointer to the **resource table** (the T# of API slot `k` is `s_load_dwordx8 ..., k * 8` dwords from
it), `0x1b` a pointer to an **extended-user-data table** (a kind-0 record there has `register = 16 + dword offset`). The list
ends at the first record of kind `0x0f`, the start of a fixed tail. Confidence 85: every `2x - 1` sample over all
nine archives places (0 unplaced, asserted per archive in `gcn_ground_truth`).

**A normal map is a texture sampled and then decoded `v_mad_f32 d, 2.0, x, -1.0`.** `Program::signed_textures` follows each
`image_sample`'s destination to such an instruction and names the API slot of its T#; `signed_samplers` runs it over a
material file (the reflection's resource name without its `Texture` suffix is the sampler name an instance hashes) and
intersects across the programs that sign anything but `shadowMap`. A signed sampler is not always a normal map (a diffuse
scaled for a reflection, a detail map), so the mesh keeps only what the instance binds that is neither diffuse nor lightmap
(`psp2::normal_map::choose`); several left are named ambiguous. The numbers are in
[hd-track-tone.md](../rendering/hd-track-tone.md), "Omega's normal maps are bound". **2048 (Vita): checked, differs** - the
programs are GXP/USSE and no operand is read ([gxp.md](gxp.md)), so no equivalent rule exists and 2048's own circuits stay
unbound.

## Census

`crates/rcs/tests/gcn_ground_truth.rs`, one test per archive plus one per eboot;
the whole survey is a ratchet.

| Source | `.rcsmaterial` | programs | pixel | vertex | instructions |
| --- | ---: | ---: | ---: | ---: | ---: |
| `data00` | 2,993 | 138,837 | 31,332 | 107,505 | 16.9 M |
| `data01` | 1,601 | 152,141 | 30,731 | 121,410 | 19.4 M |
| `data02` | 2,549 | 240,668 | 49,385 | 191,283 | 31.0 M |
| `data04` | 768 | 66,112 | 11,314 | 54,798 | 8.2 M |
| `data05` (patch) | 7,912 | 597,755 | 122,752 | 475,003 | 76.5 M |
| `data08` (patch) | 7,874 | 589,851 | 121,403 | 468,448 | 75.9 M |
| `data03`, `data07`, `data09` | 0 | 0 | | | |
| base `eboot.bin` | | 203 | 134 | 57 (+12 stage 4) | 9,911 |
| patch `eboot.bin` | | 213 | 140 | 61 (+12 stage 4) | 10,462 |

**1,785,364 programs in the archives and 416 in the eboots; 0 refused.** The
material count comes from each archive's own manifest (`oag_assets::psarc`).

Named and counted, not decoded:

- **Opcode numbers past the Sea Islands tables**, all inside known formats (so the
  stream stays in step and the program still closes): VOP1 84-88 and 91, VOP2 50,
  51, 53, 55-58, VOPC 201, 203, 204, 206. VOP2 53 alone is 2.6 M instructions in
  `data05`. **None occurs in a plain-named material** (below). They sit beside
  the VOP3P words, so they read as PS4 Pro 16-bit additions, but no public
  manual numbers them this way, so they stay unnamed.
- **Reflection**: 8 (base) and 9 (patch) eboot pixel programs have a non-zero word
  at `+0x14` (0x4 to 0x20), read as a sixth record kind and not parsed; the 12
  stage-4 programs have a different reflection layout, unread.
- **Vertex inputs**: 4 eboot vertex programs have no matching input run.

## The PS4 Pro compiles are the `_1`/`_2` files

About 61 % of the archive programs (1,099,818) carry a word that is **not** Sea
Islands: VOP3P packed 16-bit math (Vega 13.3.6's layout and opcode numbers, e.g.
`v_pk_fma_f16`, `v_mad_mix_f32`, but the prefix byte `[31:24]` reads `0xCC`
where Vega's manual prints `0xD3`), SDWA (`SRC0` 249) or DPP (`SRC0` 250). These
programs still close on `s_endpgm` at exactly their length, which an
instruction-length mistake could not survive.

**They are confined to the materials whose stem ends `_<digit>`.** Of the 7,868
plain-named materials across six archives, **0 programs** carry a Pro word or an
unlisted opcode (asserted per archive). On Amphiseum's 60 materials: plain 1,840
programs with none, `_1` 1,856 of 2,027, `_2` 1,690 of 1,838. The circuit models
name the plain file (`track.final.rcsmodel` names
`dc_hologramwithstatic.rcsmaterial`, not `_1`). Reading: **the plain file is the
base PS4's compile, and `_1`/`_2` are PS4 Pro compiles** of the same material.
Confidence 80 for that reading; what selects `_1` against `_2` at run time is
unread (`eboot.bin`, not searched).

## What it answered (2026-10-27)

Each answer is on the page it belongs to; this is the index.

- **Omega's tangent `w` is read raw, never `2w - 1`**, by every tangent-reading
  vertex program on all nine archives (ratcheted; plain compiles read it with
  `v_mul_f32` by `worldScaleInv`, Pro compiles with `v_mad_mix*`), and the data
  is a signed byte that is exactly ±127 on every vertex measured. So Omega's
  bitangent is always ±1, with no compile split like HD's:
  [hd-track-tone.md](../rendering/hd-track-tone.md), "Tech de Ra's near-left wall".
- **The Amphiseum hologram board is HD's law without the `/2`**:
  [hd-track-tone.md](../rendering/hd-track-tone.md), "The Amphiseum hologram board".
- **The tone curve is unchanged by the 1.07 patch, and the composite after it is
  not**: [tonemap.md](../ghidra/functions/ps4-omega-eu/tonemap.md), "Read with
  the decoder".

## Cross-title

**2048 (Vita): checked, differs.** Its programs are GXP/USSE
([gxp.md](gxp.md)); `oag_rcs::gxp::usse` classifies words and reads no operands,
so none of the three answers above can be read on a 2048 program by this decoder.
**HD (PS3)** uses RSX microcode, read by `oag_rcs::rcsmaterial`.

## Open

- What the unlisted Pro opcodes are, and what selects `_1` against `_2`.
- The `+0x14` reflection records and the stage-4 layout (eboot only).
- What kinds `0x0f`, `0x11`, `0x13` (second, no flags) and `0x15` of the header's tail are, and the flags byte of a usage record.
