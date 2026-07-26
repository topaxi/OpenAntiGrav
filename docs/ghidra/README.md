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

## The rule

**Never rename without writing the documentation page.**

The Ghidra database is not committed, cannot be diffed and cannot be reviewed.
If the analysis lives only there, the project's understanding is one disk
failure from zero, and a new contributor has to redo it to learn anything.

The rationale is in
[ADR-0005](../architecture/adr/0005-ghidra-conventions.md), and the scoring
scheme is the [confidence rubric](../reverse-engineering/confidence-rubric.md).

## Status

Milestone **M2**, not yet started. The conventions exist now so that the first
function analysed is documented properly rather than retrofitted.

Both main executables are unencrypted ELF files, so there is no decryption
barrier to starting:

- PSP: `PSP_GAME/SYSDIR/BOOT.BIN`
- PS2: `SCES_547.48`
