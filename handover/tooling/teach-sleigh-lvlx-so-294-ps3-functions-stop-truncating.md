# Teach sleigh `lvlx`, so 294 PS3 functions stop truncating at it

2026-09-15. The condition the previous thread was waiting on - "once the 851
`lvlx` sites are actually in the way" - is measured and met. Findings, method
and the compile-tested constructor are on
[toolchain.md#ps3](../../docs/reverse-engineering/toolchain.md#ps3) under
"Some Cell vector instructions are missing from Ghidra's sleigh"; the short
form:

- All 851 `lvlx` words in `EBOOT.elf` are undefined bytes in no function body
  (confidence 95). Ghidra stops disassembling at each one and never resumes
  at the fall-through, so every site opens a hole in one of **294 functions**
  - a body ends there when no later branch target is reachable, and resumes
  past it otherwise. 213,044 of the 418,640 bytes between those entries and
  the next function are undisassembled - 39% of every undisassembled byte in
  the executable lies in a range with an `lvlx` site in it, which is
  co-location, not attribution (confidence 80; the range map mixes in padding
  and neighbouring code).
- Eight of those functions are already in `names.tsv`. `Collision_MarchSegment`
  has a 36-byte body in a 2,272-byte range and decompiles to
  `halt_baddata()`; `Collision_TestMeshObb` keeps half. `physics.md` read the
  first by hand-disassembling past the hole.
- The fix needs nothing of Ghidra's vendored: copying the stock PowerPC
  `.sinc`/`.slaspec` from the local install at build time and adding one
  14-line `cell_lvlx.sinc` compiles with `support/sleigh` in 5.4 s, warnings
  identical to the stock spec. That is the same shape as
  `scripts/build-ghidra-allegrex.sh`'s tracked patch, which the maintainer
  pointed at on 2026-09-15 as the precedent to follow for the PS3 extension.
- `scripts/scan-ps3-cell-vector-ops.py` reproduces the count and lists the
  sites offline. The other seven forms (`lvrx`, `stvlx`, `stvrx`, the `l`
  variants) have zero sites, so one constructor is the whole job for this
  binary.

**Landed later on 2026-09-15: the language ships the constructor and it is
verified on a scratch import.** `just build-ps3-scripts` now compiles
`ppc_64_isa_altivec_ps3.sla` from copied stock sources plus
`scripts/ghidra-ps3-language/cell_lvlx.sinc`, `ppc_ps3.ldefs` points at it
(same id, same `version`), and the built extension is installed in
`~/.config/ghidra/ghidra_12.1.2_DEV/Extensions/Ps3GhidraScripts` (the
2026-08-26 install moved to `data/tools/Ps3GhidraScripts.oag-backup-20260826`
- it must not stay under `Extensions/`, see toolchain.md's PS3 step 3). A
headless import into a scratch project under it: **848 of 851 sites are
`lvlx` instructions inside functions**, 26,112 functions, and all eight named
bodies grow to their ranges (`Collision_MarchSegment` 36 -> 2,260 bytes). The
three remaining undefined sites are in code the disassembler never enters at
all. Full numbers on toolchain.md#ps3.

What remains is the live project, which the running Ghidra GUI holds locked
and which loaded the *old* `.ldefs` at startup, so it needs a restart before
it sees the new `.sla` at all.

## Open

- Whether the constructor's semantics are exact enough for the decompiler to
  fold the `lvlx`/`lvsl`/`vperm` unaligned-load idiom into a single 16-byte
  load. Disassembly and flow do not depend on it; the decompiled expression
  does. Check against `Collision_MarchSegment`'s `v10 = *r5 - *r4` reading in
  `physics.md` once the live program decompiles it.
- Whether to publish `scripts/ghidra-ps3-language/` (and this constructor)
  upstream to clienthax/Ps3GhidraScripts. The sleigh sources being patched are
  Ghidra's (Apache-2.0), not clienthax's, so the ask-first rule applies to the
  PR, not to carrying the patch here. Maintainer decision, not made.

## Next Steps

1. Close Ghidra, then from the main checkout run
   `scripts/import-ps3-eboot.sh --ps3-cspec` - it overwrites
   `OpenAntiGrav/ps3-hdfury-eu/EBOOT.elf` in place with the same loader,
   scripts and language the scratch verification used. The scratch run took
   roughly ten minutes on this machine.
2. Reopen Ghidra (it now loads the new `.sla`), then `just apply-names` to
   replay the 331 `names.tsv` rows, and re-run the site check against the live
   program: the inline-script recipe on toolchain.md#ps3, expecting
   `undefined=3`, `inFunction=848`.
3. Decompile `Collision_MarchSegment` and settle the semantics question in
   Open against `physics.md`'s hand reading.
