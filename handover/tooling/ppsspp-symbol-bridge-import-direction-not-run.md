# PPSSPP symbol bridge: export and harvest land, the Ghidra import direction does not

[`docs/reverse-engineering/ppsspp-symbol-bridge.md`](../../docs/reverse-engineering/ppsspp-symbol-bridge.md)
is the full writeup. Summary: `kotcrab/ghidra-allegrex` ships
`PpssppImportSymFile.py`/`PpssppExportSymFile.py`
(`data/tools/ghidra-allegrex/ghidra_scripts/`) to move symbol names between
Ghidra and PPSSPP's Debug menu. Two of the three directions are done:

- **Export** (`names.tsv` -> `.sym`, offline): `scripts/export-ppsspp-sym.py`.
  All four PSP binaries tested (`psp-pulse-usa` 686 functions,
  `psp-pulse-eu` 411, `psp-pure-usa` 14, `psp-pure-eu` 9), evidence-checked,
  every line hand-verified to parse under `PpssppImportSymFile.py`'s own
  splitting logic. Size is the gap to the next `names.tsv` row's address, not
  `0000` - a live test plus PPSSPP's own `SymbolMap.cpp` source confirmed a
  `size=0` entry gets a name at its exact start address only, with zero
  function-body grouping in `memory.disasm`, so a real (if approximate)
  extent was worth deriving instead. See the doc's "Export" section.
- **Harvest** (what PPSSPP's own analysis already knows, live over the
  websocket debugger, no GUI): `scripts/harvest-ppsspp-symbols.py`, run
  against both PSP Pulse pressings and committed as
  `docs/ghidra/captures/psp-pulse-usa/ppsspp-detected.tsv` /
  `psp-pulse-eu/ppsspp-detected.tsv` - a sibling of that directory's
  Ghidra-capture files, never merged into them (they're PPSSPP state, not
  Ghidra state). 408/407 named functions each, 335/334 of which are `zz_`
  HLE stubs (306 of USA's overlapping a live `psp-imports.tsv`, itself
  gitignored so that count is illustrative, not a fact to cite verbatim); the
  73 real detections per binary are libc/newlib plus the transcendental math
  routines (`sinf`/`cosf`/`atan2f`/`sqrtf`/`pow`/... - directly useful since
  `just check-determinism` polices exactly that set), of which 60 (USA) / 63
  (EU) are addresses `names.tsv` doesn't already have a row for.

**The import direction (PPSSPP's `.sym` -> Ghidra, i.e. actually running
`PpssppImportSymFile`) was not run.** The Ghidra bridge was another lane's
this session (`bloom`), and importing writes to the shared Ghidra project -
out of lane per this thread's own instructions.

What running it would need is already computed offline in the doc's own
"Import" section, so picking this up needs no new investigation, just the
bridge:

- Offset is `0` (image base already `0x08804000`).
- `skipZun=True` (the script's own default) does **not** protect 13 addresses
  on USA / 10 on EU that PPSSPP's autodetection also names - those aren't
  `z_un_` placeholders, they're names PPSSPP successfully resolved, so the
  import **will** overwrite them. Seven of the thirteen (six of the ten on
  EU) are a real downgrade (e.g. `Gu_Fog` -> `sceGuFog`,
  `Math_TransformVec4` -> `sceVfpuMatrix4Transform`); the rest already agree
  and are harmless. Full before/after table is in the doc.

## Open

- Nobody has run `PpssppImportSymFile` against a live Ghidra project yet, on
  any binary.
- The 395 (USA) / 397 (EU) PPSSPP-detected addresses that `names.tsv` does
  not have anything for are sitting in the harvest tables, unexamined beyond
  "libc/newlib/math, not gameplay code" - it's possible a handful are worth a
  closer look (the one PPSSPP-specific hack entry,
  `expensive_wipeout_pulse` at `0x08833edc`, was noticed but not chased).
- `hle.func.scan` exists in the PPSSPP binary (confirmed via `strings`,
  listed in `ppsspp-debugger.md`'s table) but was never exercised - unlike
  `.add`/`.remove`/`.rename`, which now are: see the next bullet.

## Next Steps

1. With the Ghidra bridge free: `scripts/export-ppsspp-sym.py <binary>` to
   get a fresh `.sym` (or use the harvest tables directly), then run
   `PpssppImportSymFile` in Ghidra with offset `0`.
2. Immediately re-run `just apply-names` against that program - it applies
   every `names.tsv` row unconditionally, which repairs the seven-USA/six-EU
   real collisions the import will have just overwritten, without hand
   tracking which addresses PPSSPP touched.
3. `scripts/audit-ghidra-names.py --binary <binary>` afterward, the same
   safety net `the-eu-name-gap-...md` already used after its own
   bulk-apply pass.
4. Already done, live over the debugger this session: `hle.func.rename`
   works as a file-free, per-address alternative to the Debug-menu round
   trip - confirmed to rename correctly with full function-body grouping in
   `memory.disasm`. **`hle.func.remove` immediately followed by
   `hle.func.add` at the same address crashes PPSSPP**, reproduced twice
   independently (`.../stl_vector.h:1253` assertion). See
   `ppsspp-debugger.md`'s "Naming a function live" section for the exact
   call shapes and the crash repro - don't repeat that sequence live.
