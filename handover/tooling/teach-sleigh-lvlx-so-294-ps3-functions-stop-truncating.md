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

What is deliberately not done: nothing is installed, and the live project is
untouched. Shipping the constructor means a new `.sla` in the Ps3GhidraScripts
extension, a language id or `version` bump so Ghidra notices, and a reimport
(or a re-disassembly of the 294 ranges) of the live `ps3-hdfury-eu` program -
the same maintainer-run step the cspec switch was, and it is slow on 26,100
functions.

## Open

- Whether to bump the existing `PowerPC:BE:64:A2ALT-32addr-PS3` language's
  `version` (Ghidra offers an in-place language upgrade, which re-disassembles
  nothing by itself) or to add a second id and reimport. A reimport is the
  known-good path (`just apply-names` replays the 331 rows); an upgrade plus
  "clear and disassemble" over the 294 ranges would keep every comment and
  bookmark but has not been tried.
- Whether the constructor's semantics are exact enough for the decompiler to
  fold the `lvlx`/`lvsl`/`vperm` unaligned-load idiom into a single 16-byte
  load. Disassembly and flow do not depend on it; the decompiled expression
  does. Check against `Collision_MarchSegment`'s `v10 = *r5 - *r4` reading
  once it decompiles.
- Whether to publish `scripts/ghidra-ps3-language/` (and this constructor)
  upstream to clienthax/Ps3GhidraScripts. The sleigh sources being patched are
  Ghidra's (Apache-2.0), not clienthax's, so the ask-first rule applies to the
  PR, not to carrying the patch here. Maintainer decision, not made.

## Next Steps

1. In `scripts/build-ghidra-ps3-scripts.sh`: copy
   `$GHIDRA_INSTALL_DIR/Ghidra/Processors/PowerPC/data/languages/{*.sinc,ppc_64_isa_altivec_be.slaspec}`
   into the checkout's `data/languages/`, add `cell_lvlx.sinc` from
   `scripts/ghidra-ps3-language/` (the constructor on toolchain.md, verbatim),
   generate `ppc_64_isa_altivec_ps3.slaspec` (stock plus the one `@include`),
   compile with `$GHIDRA_INSTALL_DIR/support/sleigh`, and point
   `ppc_ps3.ldefs` at the new `.sla`. Verify the zip carries the `.sla`, as the
   script already does for the cspec.
2. Install into a scratch project first, per this thread's own precedent:
   `analyzeHeadless` import of `EBOOT.elf` under the updated language, then
   `scripts/scan-ps3-cell-vector-ops.py`'s site list against
   `getFunctionContaining` - every site should now be an `lvlx` instruction
   inside a function, and `Collision_MarchSegment`'s body should be near 2,272
   bytes.
3. Ask the maintainer to reimport the live program (or try the upgrade path
   in Open), then `just apply-names` and re-run the mapping script to confirm
   `0 undefined` at the 851 sites.
