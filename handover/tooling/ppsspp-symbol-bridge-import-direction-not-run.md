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
  splitting logic.
- **Harvest** (what PPSSPP's own analysis already knows, live over the
  websocket debugger, no GUI): `scripts/harvest-ppsspp-symbols.py`, run
  against both PSP Pulse pressings and committed as
  `scripts/ppsspp-detected-psp-pulse-usa.tsv` /
  `...-psp-pulse-eu.tsv`. 408/407 named functions each, 335/334 of which
  duplicate `psp-imports.tsv`'s NID-based stubs; the ~73 real detections per
  binary are libc/newlib plus the transcendental math routines
  (`sinf`/`cosf`/`atan2f`/`sqrtf`/`pow`/... - see the doc for the full list).

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
- `hle.func.add`/`.remove`/`.rename`/`.scan` exist in the PPSSPP binary
  (confirmed via `strings`, listed in `ppsspp-debugger.md`'s table) but were
  never exercised - a live-session alternative to the file-based Debug-menu
  round trip, unverified.

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
4. Optional: try `hle.func.add`/`.rename` live over the debugger as a
   file-free alternative to the Debug-menu round trip, and record whether it
   actually works.
