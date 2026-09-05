# Ghidra applies no PSP relocation at all; the patch is written, not installed

Measured 2026-09-05. The wart three separate sessions each worked around by
hand - byte-scanning for a `lui`/`addiu` pair, following a jump table
manually, reading the decompiler instead of the xref tool - has one cause, and
it is not the one
[workflow.md](../docs/ghidra/workflow.md#why-a-psp-import-silently-loses-every-relocation)
used to give.

`ghidra-emotionengine-reloaded`'s `EE_ElfExtension` declares
`@ExtensionPointProperties(priority = 2)` and inherits stock
`MIPS_ElfExtension`'s "any `EM_MIPS`" `canHandle(ElfHeader)`.
`ElfExtensionFactory.getLoadAdapter` is first-match-wins over a
priority-sorted list, and `Allegrex_ElfExtension` declares no priority at all
(`DEFAULT_PRIORITY = 1`). So the PS2 extension owns every PSP file,
`Allegrex_ElfExtension.processGotPlt` never runs, and **the relocation table
on all four PSP databases holds zero entries**. Confidence **92**; the whole
evidence chain is on the workflow page.

This is an *interaction* bug, not an upstream defect: without the Emotion
Engine extension installed, Allegrex sorts at index 4 and stock
`MIPS_ElfExtension` at 7, so Allegrex wins and upstream has never seen it.

Two claims this killed, both of which had been carried forward as fact:

- `psp-pulse-usa`'s `0x0894f6d8` does **not** hold `a908113c`. It holds
  `3c110028`, byte-identical to `BOOT.BIN` on disk. Nothing in any PSP
  database was ever relocated, so the "four escalating instances" were one bug
  seen four times.
- The page's leading hypothesis - set the image base in the loader options -
  was **right and confounded**, not wrong. Unpatched it measures worse (7,933
  functions against 10,679, and `jal` targets stop landing in `.text` at all)
  purely because no relocation runs in either arm. Patched it is the only
  correct order, because `AllegrexRelocationProcessor` computes every write
  against `program.imageBase` at load time, while re-applying after a rebase
  is the job of `RelocationFixupPlugin` - a GUI `ProgramPlugin`, so a headless
  `setImageBase` moves addresses and leaves bytes.

## Open

- **Nothing is installed and nothing is reimported.** The patch
  (`scripts/patches/ghidra-allegrex-psp-elf-extension-priority.patch`, wired
  into `scripts/build-ghidra-allegrex.sh`, which now pins `--ref aec4265`)
  applies cleanly and the patched `Allegrex_ElfExtension.java` compiles against
  the real Ghidra 12.1.2 classpath - but **that the patch removes the symptom
  is derived from the mechanism, not executed**. Do not record it as verified
  until an import has been measured.
- Only `psp-pulse-usa` was probed for the address-level detail. The other three
  PSP databases were confirmed empty-table only, and only by making each
  current in Ghidra by hand - `run_script_inline` ignores its `program`
  parameter and always runs against the current program, which
  `scripts/check-ghidra-import.py` now detects rather than silently mislabels.
- PS2, PS3 and Vita are unaffected *by construction* (`EE_ElfExtension` claims
  only `EM_MIPS`, so PPC and ARM imports cannot hit this, and on the PS2 binary
  EE is the correct adapter) but none of the three was separately measured.
- Whether `names.tsv` replays cleanly onto a reimported database. Addresses
  should be unchanged - the image base is the same and nothing about
  relocations moves a function - but `just apply-names` refuses rows whose
  evidence does not match, so a reimport is the moment to find out.

## Next Steps

Maintainer actions, in order. None of them is safe to do from an agent session
while a Ghidra GUI is open on the live project.

1. `just build-allegrex` - builds the patched extension to
   `data/tools/ghidra_12.1.2_DEV_*_ghidra-allegrex.zip`. It applies the patch
   itself and refuses rather than silently building without it.
2. Install it (`File > Install Extensions`), restart Ghidra.
3. Reimport one PSP binary **with the image base set in the loader's import
   options**, not after analysis. Then `just check-ghidra-import`: a nonzero
   relocation count is the pass, and on `psp-pulse-usa` the three secondary
   probes should go green too.
4. If it passes, reimport the other three and `just apply-names`. If it does
   not, the mechanism above is still measured and correct - what failed is the
   fix, and the next thing to check is whether Allegrex now wins adapter
   selection at all (`ClassSearcher.getInstances(ElfExtension.class)` ordering,
   the probe scripts used for this are described on the workflow page).
5. Delete this file and its `HANDOVER.md` index line once a reimported
   database reads a nonzero relocation count.
