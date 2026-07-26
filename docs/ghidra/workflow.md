# Ghidra workflow

## Setting up a project

The Ghidra project lives in `data/ghidra/`, which is gitignored. Use one project
with several programs so cross-binary comparison is easy.

### PSP

```sh
just unpack extract data/images/pulse-psp-usa.chd \
    -o data/extracted/psp 'PSP_GAME/SYSDIR/BOOT.BIN'
```

`BOOT.BIN` is an **unencrypted ELF**, so there is no decryption step. That is
the single largest piece of luck this project has had.

> **Do not import it with stock Ghidra.** The ELF loader picks
> `MIPS:LE:32:default`, which silently mis-decodes all 26,032 VFPU instructions
> in the binary as nonexistent 64-bit MIPS III instructions. The decompiler then
> produces confident, fictional C for exactly the math-heavy code this project
> cares about. Install the Allegrex processor module first, and rebase the image
> while you are at it: see
> [Allegrex and the VFPU](../psp/allegrex-vfpu.md).

Do not bother with `EBOOT.BIN`. It is the same program with an encryption and
signing wrapper.

### PS2

```sh
just unpack extract data/images/pulse-ps2-eu.chd \
    -o data/extracted/ps2 'SCES_547.48'
```

Also a plain ELF, for the Emotion Engine. The `IOP/*.IRX` modules are separate
ELF files for the I/O processor and can be imported alongside if the audio or
storage paths become relevant.

### Naming programs

Name each program for its origin, so a documentation page's "Binary" field is
unambiguous:

```
pulse-psp-BOOT.BIN
pulse-ps2-SCES_547.48
pure-psp-BOOT.BIN
```

## Analysis

Run the default auto-analysis first. On a binary this size it takes a while;
start it and do something else.

Afterwards, the useful entry points:

- **`main` or the entry point**, to find the main loop.
- **String references.** Debug strings, asset paths and format tags are the
  fastest way into an unfamiliar subsystem. Search for `.wad`, and for team
  names like `FEISAR`.
- **The largest functions.** The ship update is likely to be one of them.
- **Functions called once per frame.** Find the main loop first, then read what
  it calls.

## Before renaming anything

Read [ADR-0005](../architecture/adr/0005-ghidra-conventions.md) and the
[confidence rubric](../reverse-engineering/confidence-rubric.md).

The short version: every rename gets a page under `functions/<binary>/` with its
evidence and a confidence score. Below 50, do not rename.

This is the rule most likely to be skipped under time pressure, and the one
whose absence costs most later.

## Working with GhidraMCP

An agent can drive Ghidra through the MCP bridge. Setup is in
[toolchain](../reverse-engineering/toolchain.md).

The conventions apply unchanged, and matter more. An agent will produce a
confident, plausible reading of any function you point it at, including a wrong
one. Two habits keep this useful:

1. **Demand the evidence, not the conclusion.** "It reads `a0+0x1c` and
   `Ship_ApplyRotation` also reads that offset" is checkable. "This updates
   steering" is not.
2. **Verify at runtime before trusting anything above 84.** A breakpoint settles
   in a minute what an hour of reading cannot.

## Cross-referencing platforms

When a function is understood on one platform, look for its counterpart on the
other. Agreement raises confidence sharply. Disagreement is a finding: record
whether it is an implementation difference, a platform limitation or an
intentional change, in [comparisons](../comparisons/).

**Remember the region confound.** Our PSP copy is US and our PS2 copy is EU, so
every cross-platform difference is also a cross-region difference until proven
otherwise. See [source images](../reverse-engineering/source-images.md).

## Exporting

The Ghidra database is not committed. If a bulk export becomes useful, write it
to `data/ghidra/exports/` and keep the hand-written pages as the record of
truth: an export carries the names, not the reasoning.
