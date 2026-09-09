# GXP decoder - full findings (scratchpad, 2026-09-02)

Scratchpad named at the start of the task. The durable results are in
`docs/formats/gxp.md`, `scripts/vita-gxp.py`, `crates/rcs/src/gxp.rs`,
`crates/rcs/tests/gxp_ground_truth.rs` and
`handover/tooling/2048s-gxp-containers-decode-the-usse-stream.md`. This file records
the route, including the wrong turns, and can be deleted with the thread.

## Result

**97,899 of 97,899 GXP containers decode**, over 22,056 files: both eboots,
`data.psarc`, `dlc1.psarc`, `dlc2.psarc`. Three further `GXP\0` magics are
chance hits inside `.probes` files and are rejected as not-containers.

Scope reached: **step 1 (locate) and step 2 (container) complete; step 3
(instructions) deliberately not started** - the container work filled the time
box and a half-read disassembly is worse than none.

## Step 1 - locate

- No `.gxp` file on the disc. Programs are embedded, found by magic scan.
- `base/eboot.elf`: 111 containers, run starting `0x515f70`.
- `patch-v104/eboot.elf`: 67 containers, three runs, first at `0x52628c`.
- `.rcsmaterial` files: 97,721 across the three packages, back to back inside
  each file, the same arrangement PS3 `SHO` blocks have.
- Both eboots hold the same **67** `_vp`/`_fp` shader-name strings, including
  `track_proximity_shadow_vp`/`_fp` at `0x4272e8`/`0x427304` (patch build).
- Neither name is in any archive entry. `shadowMap` is: 370 entries in
  `data.psarc`, 250 in `dlc1`, 175 in `dlc2`.

## Step 2 - container

Header and parameter layout, the closure argument and the confidence per claim
are in `docs/formats/gxp.md` and not repeated here. The derivation route:

1. **Span-vs-sum on the eboot's blob array.** 64 of 66 inter-blob gaps are
   exactly `align4(size)`; the 2 exceptions are run boundaries with unrelated
   `.data` between.
2. **Offset sweep over every 4-byte-aligned header word**, requiring the
   parameter table to land inside the program with every name resolving to a
   NUL-terminated ASCII string. Three survivors; two of them vacuous because
   `+0x20` is zero.
3. **Closure separates them**: last name's NUL == declared size, 3,540 to 0.
4. **Region tiling** found the remaining tables by requiring the space between
   the code and the parameter table to be accounted for exactly.
5. **`+0x58 * 4 == residue`** on 3,549 of 3,549 was how the uniform-image
   block was found - a sweep for "which field times which stride equals the
   unexplained gap".

## Four things that were wrong on the way, and how each was caught

1. **`+0x20` named `always_zero` and enforced.** True on 44,603 programs
   (one eboot + `data.psarc`), false on 24 in the DLC archives. Caught by
   running the census over all three archives rather than the one. **A field
   that is constant over a sample is not a constant.**
2. **A zero name offset read as an offset.** 72 programs
   (`fc06_lambert_simple_bonecount2.rcsmaterial`, three circuits) carry an
   unnamed `UNIFORM_BUFFER` parameter whose `name_offset` field is literally
   `0`. Read as an offset it points back at the parameter's own entry and the
   name region stops closing. They surfaced as "a parameter name overlaps the
   parameter table" on exactly 72 programs, which is what made them findable.
3. **The table at `+0x7c` was missed entirely**, because its count is zero on
   97,827 of 97,899 programs. The same 72 skinned programs have a non-empty
   one, *after* the containers rather than before - so the fix was to stop
   assuming a fixed table order and sort the six spans by start. Those 72
   went from being the failures to being the evidence for the field.
4. **`docs/formats/gxp.md` already existed** and a `Write` of a "new" page
   replaced it. Caught by `git status` showing `M` rather than `??`. Restored
   from `HEAD` and merged. The older page turned out to be the **cross-check**
   that lifts the header's confidence: it read the layout off Vita3K's
   documented `SceGxmProgram` and arrived at the same field assignment this
   sweep derived from the bytes.

## The type nibble, confirmed rather than assumed

The old page named the second nibble of `+0x04` `type`, enum
`F32, F16, C10, U32, S32, U16, S16, U8, S8, AGGREGATE`. Cross-tabulated
against `category` over 1,167,316 parameters:

| category | type | count |
| ---: | ---: | ---: |
| 0 `ATTRIBUTE` | 0 `F32` | 292,835 |
| 1 `UNIFORM` | 0 `F32` | 489,361 |
| 1 `UNIFORM` | 1 `F16` | 341,049 |
| 2 `SAMPLER` | 0 `F32` | 43,999 |
| 4 `UNIFORM_BUFFER` | 9 `AGGREGATE` | 72 |

**`type == 9` occurs on exactly the 72 uniform buffers and nowhere else**, and
`AGGREGATE` is index 9. Two independent facts landing on each other.

`F16` was then confirmed structurally rather than left resting on the enum's
ordering, which was a single source. Measuring the `resource_index` gap
between adjacent uniforms in the same program: a `type=0` `float3` or `float4`
advances the register by **4**, a `type=1` of either width by **2**, and a
`type=1` `float3[6]` by **12**. Half the register space for the same declared
width is what half precision means. Over 138,701 adjacent pairs, so the field
is named at **85**.

Also verified rather than assumed: **every archive entry carrying a container
is a `.rcsmaterial`** - 693 in `data.psarc`, 327 in `dlc1`, 240 in `dlc2`, and
no entry of any other extension holds one. The per-source table in `gxp.md`
had been hand-adjusted from an earlier census whose counters included the
stray-magic files; these are the swept numbers.

## What this settled for shadows

- `shadowMap` is a **single-channel sampler at texture unit 3**, bound by
  **1,799** programs, beside `lightmap` (4,467, unit 1, four channels) and
  `occlusionMap` (1,895, unit 2). The track's shadow term is a one-channel
  texture sampled per pixel with the lightmap, not a projected depth map.
- `ShadowColour` (float3), `ShadowSelect` (float1) and `ShadowAlpha` (float1)
  are **uniforms with no sampler beside them** in
  `Ships/materials/Debris.rcsmaterial` - a flat colour, not a lookup.
- `track_proximity_shadow_fp` binds **no parameter whose name contains
  "shadow"**.
- `docs/rendering/shadows.md`'s "blocked on tooling" paragraph is rewritten to
  say all of this.

## Not established, and why

- **Which eboot program carries which name.** Counts and the 32/35 split match
  in the patch build; the base build has 111 against the same 67 names, so the
  correspondence is not structural. Parameter-to-name matching is persuasive
  for roughly the first fifteen in address order and breaks around the
  twentieth. **Confidence 40, nothing renamed.**
- The Vita eboot is a **relocatable SCE ELF** (type `0xfe00`) so no `.data`
  word holds either address literally; the relocation table is its third
  program header, `LOOS+0xfffff01` at `0x540ae0`, 0x3e13 bytes, unparsed.
- Literal record interpretation: 45.
- Eight header words with non-constant values and no recovered meaning.

## Commands

```sh
python3 scripts/vita-gxp.py census data/extracted/vita/PCSF00007/base/eboot.elf
python3 scripts/vita-gxp.py grep   data/extracted/vita/PCSF00007/base/PSP2/data.psarc shadow
cargo nextest run -p oag-formats --release --run-ignored all -E 'binary(gxp_ground_truth)'
```
