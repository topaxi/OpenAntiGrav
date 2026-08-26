# 2048's four Vita eboots are decrypted and imported; RE itself hasn't started

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

## Open

- No `docs/ghidra/functions/vita-2048-*/` directory, no `names.tsv`, no
  `BINARY_PROGRAMS` entry in `apply-ghidra-names.py` - all four wait on a
  first recovered name, per ADR-0005's own workflow, same as before. This is
  now the *only* thing blocking that, not a toolchain problem.
- `data/keys/` does not exist in every checkout (this session rebuilt it from
  scratch after finding it absent) - worth checking whether it is worth
  syncing somewhere more durable than a single machine's gitignored `data/`,
  since regenerating it needs nothing but network access to
  nopaystation.com and is cheap, but a stale zRIF for a title NoPayStation
  later delists would not be.
- `FixupVLRImportThunks.java` (shipped with VitaLoaderRedux) - still
  unevaluated. Its own README frames it as a fallback for when the NID
  Analyzer's import-thunk naming doesn't resolve cleanly, not a mandatory
  step like PS3's `AssignPs3R2FromOpd`.

## Next Steps

- Run full auto-analysis on `/vita-2048-eu-v104/eboot.elf` (the target of
  record) if not already complete, then start recovering names the normal
  way (observe -> hypothesise -> verify -> document), same workflow as every
  other binary. First recovered name creates
  `docs/ghidra/functions/vita-2048-eu-v104/names.tsv` and the matching
  `BINARY_PROGRAMS` entry in one change, per CLAUDE.md.
- The other three programs (`usa-v104`, `eu-base`, `usa-base`) are imported
  but not analyzed - run auto-analysis on them opportunistically, same
  "corroboration only" role as `psp-pulse-eu`/`psp-pure-eu`, not urgent.
- Decide whether `data/keys/vita-zrif.tsv` should also record DLC1/DLC2
  zRIFs now that the recovery method (nopaystation.com's TSV export, grepped
  directly) is fast and repeatable - not needed yet since DLC PKGs carry no
  binaries at all.
