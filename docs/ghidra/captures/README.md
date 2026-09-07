# Captured Ghidra database state

What the Ghidra databases hold that `names.tsv` does not. Captured 2026-09-07,
immediately before the PSP databases were replaced by fresh imports built with
the Allegrex relocation patch.

`names.tsv` is the reproducible record of *names*. It is not the only thing a
database accumulates: a database also carries struct definitions, function
prototypes, variable names and comments, and none of those survive a reimport.
This directory is the measurement of how much of that there actually was.

## The measurement

Every database in the project, on 2026-09-07:

| Binary | ver | structs | prototypes | variable names | plate comments | user labels | named functions |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `psp-pulse-usa` | 69 | 0 | 0 | 0 | 11 | 116 | 884 |
| `psp-pure-usa` | 12 | 0 | 0 | 0 | 6 | 0 | 14 |
| `psp-pure-eu` | 7 | 0 | 0 | 0 | 0 | 0 | 9 |
| `ps2-pulse-eu` | 15 | 0 | 0 | 0 | 1 | 24 | 158 |
| `ps3-hdfury-eu` | 44 | 0 | 0 | 0 | 15 | 1600 | 608 |
| `vita-2048-eu-v104` | 16 | 0 | 0 | 0 | 19 | 12 | 26 |

**Not one hand-recovered struct, prototype or variable name exists anywhere in
the project.** The raw databases do report composites, signatures and named
parameters, and every one of them is a tool's own output rather than a person's
claim:

- **Composites** are Ghidra's ELF loader types (`Elf32_Ehdr`, `Elf32_Phdr`), the
  PS3 PRX module tables (`_scelibent_ppu32`), the Vita SCE module tables, and
  the libc headers a header-parse imported into `ps3-hdfury-eu` (`stat`, `tm`,
  `lconv`). None describes a game structure.
- **Prototypes** are Ghidra's library signature database. All 10 on
  `psp-pulse-usa` are `IMPORTED`-source libc (`strlen`, `sinf`, `powf`); all 425
  on `ps3-hdfury-eu` are libc or libm, 397 `IMPORTED` and 28 applied by a bulk
  library sweep (`erf`, `isalnum`, `getpid`).
- **Variable names** are the parameters those same library signatures carry
  (`__nptr`, `__s1`, `__x`). Removing the functions whose signature Ghidra
  supplied leaves zero named variables in every database.
- Most **comments** are the ELF loader's section markup (`_elfSectionHeaders::`,
  identical in an untouched reimport of the same executable) or
  vita-loader-redux's NID plates (`--- IMPORTED FUNCTION ---`).

So the format question the maintainer asked - how to track structs, prototypes
and local variable names - has no backlog to rescue. It is a question about
work not yet done, not about work at risk.

## What is here

One directory per binary. A file is absent when it would have been empty.

| File | Columns |
| --- | --- |
| `functions.tsv` | `address`, `name`, `source` - every `USER_DEFINED` function name |
| `labels.tsv` | `address`, `symbol_type`, `name`, `namespace` - every `USER_DEFINED` non-function symbol |
| `comments.tsv` | `address`, `kind`, `function`, `text` - plate comments a person wrote, `\n` escaped |
| `signatures.tsv` | `address`, `name`, `source`, `prototype` |
| `variables.tsv` | `address`, `function`, `kind`, `variable`, `type`, `storage` |

Addresses are as the database reported them, in that program's own space and
without an `0x` prefix, so a row can be compared against `names.tsv` after the
usual `lower().replace("0x", "")` normalisation.

`signatures.tsv` and `variables.tsv` exist in the table above and nowhere on
disk: every binary's filtered set came out empty.

## What this is not

**It is not a second record of truth.** The evidence pages under
`docs/ghidra/functions/<binary>/` remain authoritative, exactly as
[ADR-0005](../../architecture/adr/0005-ghidra-conventions.md) says, and
`names.tsv` remains the replayable name table. This is a snapshot of a working
copy, taken because the working copy was about to be discarded.

**It is not applied to a database.** Nothing here is replayed by
`just apply-names`. A capture records what a database held; it does not assert
that a fresh database should hold it. See
[ADR-0047](../../architecture/adr/0047-database-state-beyond-names-is-captured-not-replayed.md)
for why the two are kept apart, and what would have to be true for a capture to
become a replayable row.

**It carries no game content.** Addresses, names the project wrote, and prose
the project wrote. No bytes are read out of any executable, so
[ADR-0006](../../architecture/adr/0006-no-copyrighted-content.md) is satisfied
by construction - see `scripts/capture-ghidra-state.py`, which has no memory-read
path at all.

## Reproducing

`scripts/ghidra/DumpDatabaseState.java` runs inside Ghidra and writes the raw
per-program dumps; `scripts/capture-ghidra-state.py` filters a tool's output out
of them and writes this directory. Both are described in
[../workflow.md](../workflow.md).

`just check-captures` validates every file here offline, and is part of the
`just` gate.
