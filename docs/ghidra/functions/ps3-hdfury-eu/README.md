# Wipeout HD / Fury PS3 functions

Functions from `PS3_GAME/USRDIR/EBOOT.elf` (Wipeout HD Fury, PS3, BCES-00664),
language `PowerPC:BE:64:A2ALT-32addr`, big endian.

**The imported file does not exist on the disc.** A PS3 `EBOOT.BIN` is an
encrypted SELF; what gets analysed is the ELF `rpcs3 --decrypt` writes beside
it. That makes this the only binary in this tree whose program name differs
from the file the disc ships, and the reason
`scripts/apply-ghidra-names.py`'s `BINARY_PROGRAMS` carries an `.elf` entry for
it. Reproduce the whole import with:

```sh
scripts/import-ps3-eboot.sh     # close Ghidra first; it needs the project lock
```

Setup, the compiler-spec fix this needs, and the traps are in
[toolchain.md](../../../reverse-engineering/toolchain.md#ps3).

## What this binary is, and is not

**Not a reverse-engineering target.** Wipeout HD / Fury is a
[later title](../../../overview/goals.md#scope) and no milestone is open on it.
What this directory exists for is the lineage question: HD is the first title
after Pulse, on completely different hardware, and whether its subsystems are
recognisably the same shapes is worth knowing cheaply before anyone plans work
on it. Names here are opportunistic, and any claim about *Pulse* still has to
be proved against Pulse's own binaries.

Two structural facts to expect, both different from every other binary here:

- **PPC64 with an OPD.** A function pointer is a descriptor pair
  `{code address, TOC value}` in the data segment, not a code address. The
  entry point `0x870530` is an OPD entry resolving to `func=0x00010230`,
  `toc=0x008ad4d8`. `AssignPs3R2FromOpd.java` in the script pack is what makes
  the decompiler read TOC-relative data correctly.
- **Stripped of symbols, but not of library names.** There is no symbol table
  and the section-name table is gone, so `readelf` shows 156 unnamed sections.
  The 122 `cell*`/`sce*` identifiers that survive are the module and export
  names, which `AnalyzePs3Binary.java` resolves against its NID database - so
  imports get real names and everything else starts as `FUN_`.

## Pages

- [memory.md](memory.md) - the allocator, its heap, the mutex around both, and
  **the per-function TOC defect this database has**. Read it before trusting
  any data or string reference in this program.
- [collision.md](collision.md) - `Collision.cpp`: the arena, the class, and the
  MeshAABB narrowphase.
- [race-manager.md](race-manager.md) - `RaceManager.cpp`: the singleton holder
  and the base of the race-mode hierarchy.

Add a row to [`names.tsv`](names.tsv) and the page it cites in the same change:
`scripts/apply-ghidra-names.py` refuses a row whose address and name do not
both still appear on the page named in its last column.
