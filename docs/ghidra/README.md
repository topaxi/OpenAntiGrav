# Ghidra

Conventions and per-function documentation for the binary analysis work.

| Document | Covers |
| --- | --- |
| [Workflow](workflow.md) | Setting up projects, where to start, working with GhidraMCP |
| [Naming conventions](naming-conventions.md) | Quick reference for names, structures, enums |
| [Function template](function-template.md) | The template every documented function uses |
| [Functions](functions/README.md) | Per-function pages, per binary |
| [Memory maps](memory-maps/README.md) | Address space layouts |
| [Structures](structures/README.md) | Recovered structure layouts |
| [Captures](captures/README.md) | What each database held beyond `names.tsv`, and the measurement showing how little that was |

## Getting the names back

The database is not committed, so a fresh import has no names in it. One command
restores them:

```sh
just apply-names
```

It reads [names.tsv](functions/psp-pulse-usa/names.tsv) and re-derives the import
stubs from the binary, then applies both through the MCP bridge. See
[the rename summary](functions/psp-pulse-usa/README.md#renames).

## The rule

**Never rename without writing the documentation page.**

The Ghidra database is not committed, cannot be diffed and cannot be reviewed.
If the analysis lives only there, the project's understanding is one disk
failure from zero, and a new contributor has to redo it to learn anything.

The rationale is in
[ADR-0005](../architecture/adr/0005-ghidra-conventions.md), and the scoring
scheme is the [confidence rubric](../reverse-engineering/confidence-rubric.md).

## Status

Milestone **M2**, in progress on the PSP binary. `BOOT.BIN` is analysed with the
Allegrex module, five subsystems have evidence pages, and **400 symbols are
applied**. The PS2 executable has not been imported yet.

Both main executables are unencrypted ELF files, so there is no decryption
barrier to starting:

- PSP: `PSP_GAME/SYSDIR/BOOT.BIN`
- PS2: `SCES_547.48`
