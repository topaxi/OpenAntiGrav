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
| `psp-pulse-eu` | 2 | 0 | 0 | 0 | 0 | 34 | 284 |
| `psp-pure-usa` | 12 | 0 | 0 | 0 | 6 | 0 | 14 |
| `psp-pure-eu` | 7 | 0 | 0 | 0 | 0 | 0 | 9 |
| `ps2-pulse-eu` | 15 | 0 | 0 | 0 | 1 | 24 | 158 |
| `ps3-hdfury-eu` | 44 | 0 | 0 | 0 | 15 | 1600 | 608 |
| `vita-2048-eu-v104` | 16 | 0 | 0 | 0 | 19 | 12 | 26 |

`psp-pulse-eu` is at version 2 because its database had already been replaced by
the relocation-patched reimport before this capture ran, and `just apply-names`
had already replayed `names.tsv` into it. Its row is what a *reconstructed*
database looks like, and it is the direct evidence that the reconstruction
works: 284 names and 34 labels, none of them orphaned.

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

## `names.tsv` held, and a second machine proves it

The three PSP databases under deletion pressure carry **zero orphan function
names** between them: every name in `psp-pulse-usa` (884), `psp-pure-usa` (14)
and `psp-pure-eu` (9) already has a `names.tsv` row. The table is in fact ahead
of the databases - `psp-pulse-usa`'s 961 sanctioned rows include 77 that were
never applied - so a reimport does not lose a name, it gains the ones a stale
database was missing.

A second Ghidra project, rsynced from another workstation and eight days behind
this one, was checked against the same measurement. It holds seven programs and
**not one name this machine lacks**. There is a single disagreement in the whole
corpus: `ps2-pulse-eu` at `0015ca48`, `Body_ClearAccumulators` on the second
machine against `Body_ResetAccumulators` here, which `names.tsv` settles in
favour of the local name. Two machines' RE work stayed in sync across eight days
because the names travelled in the repository rather than in the databases -
which is the whole argument of ADR-0005, now measured rather than asserted.

The other project also answers what its five same-named `BOOT.BIN` programs are.
There is **no fifth PSP binary**: `psp-pure-eu-reimport/BOOT.BIN` has the same
executable path as `psp-pure-eu/BOOT.BIN` and is a second import of Wipeout Pure
EU, carrying 8,969 functions against the original import's 6,934 and no applied
names at all. The extra 2,035 functions are what the Allegrex relocation patch
recovers, the same gain the local reimports show.

### What did diverge, and why it argues against syncing comments

The maintainer's own account of the mechanism is that "apart from `names.tsv`,
anything else in ghidra will have been invisible between workstations". That is
correct, and the traffic it failed to carry can now be counted: comparing all
six shared programs, **0 structs, 0 prototypes, 0 variable names and 0 names
diverged. 34 plate comments did** - one present only on the other machine, 33
only on this one.

The single item that diverged *onto* the other machine is a claim this project
has since disproved. Its comment at `ps3-hdfury-eu` `0x008b79cc` calls the region
a "flat array of 73 const char\* key-name templates" and cites `get_xrefs_to` as
verification. A later pass here established the opposite and wrote it up in
[../functions/ps3-hdfury-eu/zone-effectsettings-loader.md](../functions/ps3-hdfury-eu/zone-effectsettings-loader.md):
it is not an array, it is the module's TOC, and `get_xrefs_to` on it returns
nothing. The correction reached the docs tree and both machines; the stale
comment sat in the other database anyway, because a comment cannot be corrected
remotely.

An unshared comment is therefore not just an unshared asset - it is an
uncorrectable wrong claim. That is why
[ADR-0047](../../architecture/adr/0047-database-state-beyond-names-is-captured-not-replayed.md)
declines to track comments as a replayable record and has the capture check
their `docs/` references instead.

### The real backlog

Where the two projects genuinely differ is in what the *local* non-PSP databases
have accumulated and `names.tsv` has not: `ps3-hdfury-eu` holds 411 user-defined
names with no row, and `ps2-pulse-eu` holds two
(`TrackSelectionScreen_OnConfirm`, `TrackSelectionScreen_RegisterType`). Most of
the PS3 set is SDK and syscall naming rather than recovered gameplay code, but
it is unrecorded either way, and neither database is protected by the discipline
that saved the PSP ones. That gap - not structs - is the real backlog.

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

**`ppsspp-detected.tsv` (`psp-pulse-usa`, `psp-pulse-eu`) is not one of the
files above, and not a Ghidra capture at all** - it is PPSSPP's own
auto-detected function names, harvested live over its websocket debugger
(`scripts/harvest-ppsspp-symbols.py`), a completely different tool and a
different kind of claim from everything else in this directory. It sits here
because the shape is convenient (per-binary, tab-separated, address-first)
and `scripts/check-ghidra-captures.py` validates its structure the same way,
but it is exempt from the evidence-tree and README-table checks that apply to
real captures, and its `source` column names the API call that produced a
row (`ppsspp-hle.func.list`) rather than Ghidra's `SourceType` vocabulary.
Full writeup: [ppsspp-symbol-bridge.md](../../reverse-engineering/ppsspp-symbol-bridge.md).
Its `data`-kind counterpart does not exist yet - `labels.tsv`'s column shape
(`address`, `symbol_type`, `name`, `namespace`) is the natural home if a
PPSSPP data-symbol harvest is ever wanted, since PPSSPP does not export
functions and data through the same channel any more than Ghidra's own
capture files do.

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

**Captured prose is verbatim, and some of it cites in-flight thread files.**
CLAUDE.md forbids a `docs/` page from pointing into the thread tree, because a
thread file is deleted the moment its work lands and the reference dies with
nothing to catch it. Three captured comments do exactly that. They are not
edited, and they are not Markdown links - they are a transcript of what a
database contained, and rewriting one would make it a worse copy of the docs
page instead of a record of the database. This page keeps to the rule itself and
names such a file by its basename only.

What resolves the tension is that `just check-captures` now tracks those
references. It resolves every documentation and thread path a comment names,
including the ones Ghidra hard-wrapped across a `\n` mid-path, and fails on any
that does not exist unless it is in the script's shrink-only ratchet. So a
deleted thread surfaces as a ratchet entry rather than rotting unnoticed - the
capture turns the rule's predicted failure into the rule's enforcement.

That failure is already in the set, which is the point. Two references dangle
today: `docs/ghidra/functions/psp-pure-usa/rocket-visuals.md`, a page nobody
wrote, and `the-airbrake-flap-rotation-axis-is-chosen-not.md`, a thread that
landed and was deleted while `Airbrake_Update`'s comment still pointed at it.
Neither was findable before this check existed: `check-doc-links.py` walks prose
in `docs/`, not TSV payloads.

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

The filter is deterministic: rows are sorted by address, and where a comment
arrives twice the copy carrying the function name is the one kept, pinned
explicitly rather than left to Ghidra's iteration order. Verified by re-running
against a reversed dump and diffing - byte-identical. That matters because two
machines have to produce the same file for a sync to be a readable diff rather
than a wall of moved lines.

**`DumpDatabaseState.java` has been compile-verified but not executed
end-to-end.** It compiles cleanly against Ghidra's own jars (one expected
deprecation note for `CodeUnit.PLATE_COMMENT`), and the captures in this
directory were produced by a functionally identical inline script that did run.
Ghidra exited during this work - the PSP database swap - so the committed file
itself has not yet been through a live run. To close that gap:

```sh
cp scripts/ghidra/DumpDatabaseState.java ~/ghidra_scripts/
# in Ghidra: Script Manager -> DumpDatabaseState.java, argument /tmp/ghidra-capture/
just capture-ghidra-state /tmp/ghidra-capture/
git diff --stat docs/ghidra/captures   # an empty diff is the proof
```

`just check-captures` validates every file here offline, and is part of the
`just` gate.
