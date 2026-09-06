# Reminder: install the Allegrex patch and reimport the PSP databases

Untracked scratch file. Delete it once step 5 is done.
Written 2026-09-05. Full write-up: `handover/ghidra-applies-no-psp-relocation-the-patch-is.md`

## Why

Ghidra applies **no PSP relocation at all**. `ghidra-emotionengine-reloaded`'s
`EE_ElfExtension` declares `priority = 2` and inherits stock MIPS's "any
`EM_MIPS`" predicate, so the **PS2** extension wins adapter selection for every
PSP file and Allegrex's relocation pass never runs. The relocation table on all
four PSP databases holds **zero entries** against 93,443 in `.rel.text` alone.

That is the single cause behind every "Ghidra says no references" workaround -
the `jal` pseudo-addresses, the raw jump-table base, the `.data` opcode table.
One bug, seen four times. Root cause measured at confidence 92.

The patch is written, wired into `just build-allegrex` and compile-checked
against the real Ghidra classpath. **It is not installed and nothing is
reimported** - that it removes the symptom is derived from the mechanism, not
executed. Do not record it as verified until an import is measured.

## Do this, in order

Not safe to run from an agent session while a Ghidra GUI holds the live project.

1. `just build-allegrex`
   Builds the patched extension to `data/tools/ghidra_12.1.2_DEV_*_ghidra-allegrex.zip`.
   It applies the patch itself and refuses rather than silently building without it.

2. Install it: `File > Install Extensions`, then restart Ghidra.

3. Reimport **one** PSP binary with the image base set in the **loader's import
   options** - not with `set_image_base` after analysis. Then:

       just check-ghidra-import

   A nonzero relocation count is the pass. On `psp-pulse-usa` the three
   secondary probes should go green too.

4. If it passes: reimport the other three, then `just apply-names` to replay
   all 1385 recovered names onto the fresh databases.

   If it fails: the mechanism is still measured and correct - what failed is
   the fix. Next thing to check is whether Allegrex now wins adapter selection
   at all (`ClassSearcher.getInstances(ElfExtension.class)` ordering; the probe
   scripts are described on `docs/ghidra/workflow.md`).

5. Delete the handover thread and its `HANDOVER.md` index line once a
   reimported database reads a nonzero relocation count. Delete this file too.

## Watch out

- **The import order flips with the patch.** Unpatched, the current
  analyse-then-rebase order is genuinely better (10,679 functions vs 7,933,
  `jal` targets 4/4 vs 0/4). Patched, image-base-at-load becomes the *only*
  correct order, because `AllegrexRelocationProcessor` computes against
  `program.imageBase` at load time, while re-applying after a rebase is
  `RelocationFixupPlugin`'s job - a GUI plugin, so a headless `setImageBase`
  moves addresses and leaves bytes behind.

- **Three evidence pages quote raw pre-relocation constants** and go stale the
  moment a correct reimport lands. The handover thread names them.

- **A carried-forward "fact" was false**: `psp-pulse-usa` `0x0894f6d8` holds
  `3c110028`, byte-identical to disk - not the `a908113c` that
  `docs/ghidra/workflow.md` claimed. Already corrected there.
