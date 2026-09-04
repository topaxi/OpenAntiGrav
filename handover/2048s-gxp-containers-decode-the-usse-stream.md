# 2048's GXP containers decode; the USSE stream's opcode field is located, semantics do not

`docs/rendering/shadows.md` was blocked on tooling rather than on evidence:
this repository had a PS3 fragment-microcode decoder and no Vita equivalent,
so not one of 2048's shader programs could be read. The container half is now
done - [`docs/formats/gxp.md`](../docs/formats/gxp.md),
[`scripts/vita-gxp.py`](../scripts/vita-gxp.py), `oag_formats::gxp`,
`crates/formats/tests/gxp_ground_truth.rs` - at **97,899 of 97,899 programs
decoding over 22,056 files**, header, code extents, tables and the parameter
table with its names.

The instruction stream is **located, its opcode field is now found too, and
no opcode is named** - the last of those is where this stops being
comparable to the PS3 side. **2026-09-04**: `scripts/vita-gxp.py opcodes`
swept both regions' full corpus (323,937 programs, 19.3M instructions) and
found the primary and secondary op1 field is a **variable-width prefix, not
one fixed width** - at 5 bits, 8 of 9 dominant groups (89.9% of primary
instructions) close cleanly, but the largest (`00000`, 10.1%) is not one
opcode there and keeps splitting through at least 8 bits, evidence of a real
variable-length instruction-group selector rather than a single N-bit field
- and that secondary code's last instruction sets bit 50 with a full
corpus-wide closure by two independent tests, holding even when
single-instruction blocks are excluded. Primary code has no equally clean
end bit by either test, consistent with its own end flag moving with the
variable-width format selector. See
[`docs/formats/gxp.md`](../docs/formats/gxp.md)'s "The opcode field is
located" section for the numbers and the license reasoning: Vita3K's own
USSE decoder (GPL-2.0-or-later) was read as an **oracle** to check these two
findings after they were derived from the bytes, never as a source to
transcribe from - this project's own license bar rules a GPL decoder's
tables out of an MIT/Apache-2.0 tree, so naming an actual opcode from here
needs an independently-derived reading, not a port. (A first pass here also
claimed "5 bits is where the field closes" outright and was wrong - caught
by following each dominant group to its own children rather than trusting a
monotone coverage statistic; see `gxp.md`'s own note on the trap.)

The working notes from the session that did it - the derivation route and the
four things that were wrong on the way - are in
[`gxp-decoder-scratch.md`](gxp-decoder-scratch.md) beside this file. Delete
both together.

## Open

- **The USSE instruction stream.** Both programs' extents close on their own
  declared counts and the width is fixed at 8 bytes by `+0x74` agreeing with
  `+0x40 + 8 * +0x3c` on all 97,899. **2026-09-04**: the op1 field's *bit
  position* is now found (top 5 bits, both primary and secondary; secondary's
  end bit at bit 50 closes corpus-wide by two independent tests) - see above.
  **Still open, and this is the harder half**: no opcode is *named*. This is
  the piece that would turn "`track_proximity_shadow_fp` binds no shadow
  sampler" into "here is what it computes", and it needs the sixteen-plus op1
  values this pass found to each be tied to a real operation - likely via the
  same kind of route `ps3-microcode.py` used for NV40 (an independently
  re-derivable structural argument, not a transcription), since Vita3K's own
  GPL-licensed decoder can be checked against but not copied from. Naming a
  single opcode this way, end to end, is probably a session's work by itself.
- **Which of an executable's programs is which name.** In the **v1.04 patch**
  the 67 `_vp`/`_fp` strings and the 67 containers agree in count *and* in
  their 32 / 35 split - but the **base** build embeds **111** containers
  against the same 67 names, so the correspondence is not structural and the
  count match is worth 40 at most. Within the patch build, the parameters of
  the first fifteen line up persuasively with the names
  in address order - `quantize`/`invWvp`/`depthTex`/`colourTex` reads exactly
  like `Zone2048_Voxelize_vp`, `mainScaleBias`/`bloomScaleBias` like the
  `wo_composite_*` family. **It stops lining up around the twentieth**:
  ordinal 20 is `globalAmbient`/`lightColour`/`Ka`/`Kd`, which reads like
  `RenderPrimitives_AmbientDiffuse_vp` and not like `psys_normal_vp`. The
  containers are also grouped by type (27 vertex, then 29 fragment, then a
  mixed run of 11) while the names are not, so any correspondence is per-type
  at best. **Confidence 40, nothing named on it**, per
  [ADR-0005](../docs/architecture/adr/0005-ghidra-conventions.md).
- **The pointer chase that would settle it.** There is no `.data` word holding
  either a name's or a blob's address: the Vita eboot is a **relocatable SCE
  ELF** (type `0xfe00`), and its third program header, `LOOS+0xfffff01` at
  file offset `0x540ae0`, is 0x3e13 bytes of relocation table nobody here has
  parsed. Parsing it, or reading the ARM `MOVW`/`MOVT` pairs that materialise
  the two addresses in the registration code, is the way to a real binding.
  Untried.
- **The literal record.** An 8-byte record reads as `(u32 index, 4 raw bytes)`
  with the bytes being `f32` in vertex programs and a pair of `f16` in
  fragment ones - on three records read by hand. Confidence 45, so the tool
  exposes a count and a span and no values. `0x3dc6` (log2 e) and `0x3c00`
  (1.0) sitting next to a fog term in `cf_alpha4glow` is the suggestive case.
- **Eight header words carry non-constant values and have no recovered
  meaning**: `+0x2c`, `+0x30`, `+0x38`, `+0x5c`, `+0x64`, `+0x68`, `+0x98`,
  `+0xa8`. `+0x64` correlates with a value inside the container records on the
  three programs read by hand, which is a lead and not a reading.

## Next Steps

1. ~~**Decode the USSE stream**, on the model of `scripts/ps3-microcode.py`.~~
   **The opcode field's location is done, 2026-09-04**: a variable-width
   prefix, 5-6+ bits depending on the instruction, both primary and
   secondary, plus secondary's end bit at bit 50 - see above. **What's left
   is naming a single op1 value.** `scripts/vita-gxp.py opcodes` gives the
   field and the corpus to check any candidate against
   (17.4M primary / 2.0M secondary instructions, both regions); `Vita3K`'s
   own USSE translator remains the external corroboration to check a reading
   against, never to transcribe from, per the license reasoning above.
2. **Then read `track_proximity_shadow_fp`** and settle what 2048 actually
   draws for a ship shadow. The container half already narrowed it: it binds
   no parameter whose name contains "shadow", so whatever it does it does not
   sample a named shadow texture.
3. **Fold `shadowMap` into `docs/rendering/shadows.md`'s 2048 section.** It is
   a **single-channel sampler at unit 3 on 1,799 programs**, beside `lightmap`
   (4,467) and `occlusionMap` (1,895) - so the *track* shadow is a one-channel
   texture sampled per pixel, not a projected depth map, which is a firmer
   statement than that page currently makes and one it can act on.
4. **Parse the SCE relocation segment** if the name binding matters. It would
   also serve `handover/2048s-vita-eboots-are-imported-re-not-started.md`,
   which has the same problem from the Ghidra side.

## Two traps worth carrying forward

- **A field that is constant over a sample is not a constant.** `+0x20` is
  zero on all 44,603 programs in one eboot and `data.psarc` and carries `1`
  or `0x112` on 24 programs in the two DLC archives. A first pass named it
  `always_zero` and *enforced* it, which reads as a clean two-thirds pass and
  a mysterious failure on the rest. Sweep every archive - and **both eboots**,
  which is the same trap at file scale: 111 programs against 67.
- **`docs/formats/gxp.md` already existed and was nearly overwritten.** It
  carried the 2026-08-30 Zone-composite read, traced bind sites and all, and a
  `Write` of a "new" page silently replaced it; `git status` showing `M`
  rather than `??` is what caught it. The two readings are merged now and the
  older one turned out to be the **cross-check** that lifts the header's
  confidence, not content to be preserved out of politeness. Check `git
  status` before assuming a docs page is new.
- **A vacuously-true candidate survives a weak test.** `+0x20` and `+0x24`
  both pass "the parameter table lands inside the program", because zero
  parameters fit anywhere. The closure that separates them - the last name's
  NUL landing on the declared size - decides it 3,540 to 0. `is_ok()` proves
  nothing here for exactly the reason
  `crates/formats/tests/shadow_occluder_ground_truth.rs` says it proves
  nothing there.
