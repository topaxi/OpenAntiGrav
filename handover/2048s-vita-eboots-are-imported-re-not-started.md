# 2048's four Vita eboots are decrypted and imported; loose ends from getting there

2026-08-26. All four of WipEout 2048's `eboot.elf` (base + patch v1.04, USA +
EU) are genuinely plaintext ARM ELF32 and open in Ghidra via VitaLoaderRedux:
`ARM:LE:32:v7`, base address `0x81000000`, named `.text`/`.data`/`VarImport`
blocks, real function counts after auto-analysis (five figures on the patch
build). Program paths: `/vita-2048-eu-v104/eboot.elf` (target of record - EU
over USA and patch over base are both explicit policy here, the same
`psp-pulse-eu`-is-corroboration-only precedent extended to two axes instead
of one), `/vita-2048-usa-v104/eboot.elf`,
`/vita-2048-eu-base/eboot.elf`, `/vita-2048-usa-base/eboot.elf`
(corroboration-only).

**What the prior session's "eboot strips clean" thread got wrong**: `readelf`
accepting `strip-vita-self.py`'s output only meant the ELF header and program
header table - both always plaintext, even in a still-encrypted SELF - parsed
cleanly. The actual code/data segments were still NpDrm-encrypted underneath
(confirmed directly: not zlib, ~8 bits/byte entropy), and that script never
attempted decryption - it only located `elf_offset` and copied from there to
EOF. `scripts/strip-vita-self.py` is removed; `scripts/vita-self-decrypt.py`
replaces it and does the whole job. Full trail, traps included, in
[toolchain.md#vita](../docs/reverse-engineering/toolchain.md#vita).

**The klicensee needs no hardware and no F00D service** - it decodes straight
out of the zRIF with pure zlib (`scripts/zrif-to-klicensee.py`,
[KorewaWatchful/libzrif](https://github.com/KorewaWatchful/libzrif)'s
`keyflate.c` read directly). That is a different, lighter layer than
`psvpfsparser`'s own `-f00d_url`/`-f00d_cache`, which is for the *PFS
filesystem* key and does need F00D - the two should not be conflated, and the
prior session's `vita-f00d-cache.tsv` note was about the PFS layer, not this
one. `data/keys/` (gitignored) holds `vita-zrif.tsv` (zRIF + derived
klicensee for both regions' base app - a patch reuses its base license, same
content ID family, checked directly) and its own README explaining the
decode. Recovered this session from nopaystation.com's own TSV export,
fetched and grepped by title ID directly rather than through a rendered page
(the prior session found that unreliable for picking one row out of a large
file).

The actual decrypt algorithm (klicensee -> NpDrm-unwrap -> metadata-block AES
-> per-segment AES-128-CTR -> inflate) is
[Vita3K/Vita3K](https://github.com/Vita3K/Vita3K)'s own `sce_utils.cpp`,
reimplemented in Python rather than shelled out to, since Vita3K is a whole
emulator with no standalone CLI for just this step.

**Update, same day**: `docs/ghidra/functions/vita-2048-eu-v104/` and its
`BINARY_PROGRAMS` entries now exist - the first RE pass named `Game_Main` and
`GameRoot_Construct`. See
[game-boot.md](../docs/ghidra/functions/vita-2048-eu-v104/game-boot.md) for
the boot allocator and manager/root constructors that thread went on to name,
and [vita-2048-eu-v104/README.md](../docs/ghidra/functions/vita-2048-eu-v104/README.md#the-lineage-question-is-answered-confirmed)
for the codebase-lineage question it raised, now confirmed. What's left here
is loose ends unrelated to either.

## Open

- `data/keys/` does not exist in every checkout (this session rebuilt it from
  scratch after finding it absent) - worth checking whether it is worth
  syncing somewhere more durable than a single machine's gitignored `data/`,
  since regenerating it needs nothing but network access to
  nopaystation.com and is cheap, but a stale zRIF for a title NoPayStation
  later delists would not be.

**`FixupVLRImportThunks.java` is resolved, not just evaluated**: checked
directly against `eu-v104` (already fully auto-analyzed) rather than left
open - VitaLoaderRedux's loader and NID Analyzer already assign systematic
`Library_NID` names to import thunks and their call sites without it, so the
script's fallback case doesn't apply here. Evidence in
[toolchain.md#vita](../docs/reverse-engineering/toolchain.md#vita).

## Next Steps

- The other three programs (`usa-v104`, `eu-base`, `usa-base`) are imported
  but not fully analyzed - run auto-analysis on them opportunistically, same
  "corroboration only" role as `psp-pulse-eu`/`psp-pure-eu`, not urgent.
  **In progress as of 2026-09-04**: `run_analysis` triggered on all three in
  the shared Ghidra JVM; confirmed via `get_metadata` (distinct executable
  paths, sizes) that each is a genuinely separate binary, not one program
  analyzed three times under a colliding path. `usa-v104` climbed from 1,197
  to 9,623+ functions over ~10 minutes and is still running; `eu-base` and
  `usa-base` had not started moving past their pre-analysis 1,197 in the same
  window, apparently queued behind `usa-v104` in the one JVM (also contended
  by a concurrent session's `oag-view` GXP work). Whoever picks this up next:
  check `analysis_status` for all three, and once each reports
  `analyzing: false`, run `save_program` (or `save_all_programs`) - Ghidra
  analysis lives in memory until saved and is not otherwise durable.
- Decide whether `data/keys/vita-zrif.tsv` should also record DLC1/DLC2
  zRIFs now that the recovery method (nopaystation.com's TSV export, grepped
  directly) is fast and repeatable - not needed yet since DLC PKGs carry no
  binaries at all.
