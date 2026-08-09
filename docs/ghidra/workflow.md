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

### Bridge quirks that have each cost a session

- **Pass `program=` on every call.** With both `BOOT.BIN` and `SCES_547.48`
  open, omitting it silently targets whichever is active.
- **`dry_run` is not honoured by the rename endpoints.** They report what they
  would do and do it anyway.
- **Global names without a Hungarian type prefix are rejected**, which is not
  this project's convention. Apply data labels through `create_label` instead.
- **`scripts/apply-ghidra-names.py` reports "No function found" for functions
  that exist and are already correctly named.** It is a rename-endpoint quirk,
  not a script bug and not a real absence: probe `get_function_by_address`
  before concluding a database needs restoring, and never read that script's
  failure count as evidence that names are missing.
- **`import_file` returns `auto_analyzed: true` immediately**, and
  `analysis_status` then reports `analyzed: true, function_count: 1` - exactly
  the signature of a wrong-language import. It is not; analysis simply has not
  run. Call `reanalyze` (it times out, which is normal for a 1.9 MiB binary and
  means analysis started) and poll `analysis_status` until `analyzing` goes
  false. Judge the import on the count *after* that.
- **Analyse multiple large programs one at a time, never concurrently.**
  Rebasing/analysing `SCES_547.48` alongside two `BOOT.BIN`s at once left
  `analysis_status` reporting `analyzing: true` with the function count
  frozen indefinitely (`SCES_547.48` stuck at 1, unmoving across 5+ minutes
  of polling) - almost certainly decompiler-process contention. It converged
  correctly (`5,234`, matching history exactly) the moment it was re-run
  alone after a Ghidra restart. If a program you are polling shows
  `analyzing: true` with an unchanging count for more than a couple of
  minutes while anything else is mid-analysis, stop waiting - close the other
  programs (or restart Ghidra; `close_program` does not reliably release a
  program that is mid-analysis) and re-run the stuck one by itself.
- **Both Pulse `BOOT.BIN`s reproducibly plateau well below their
  previously-recorded function count, even run one at a time with nothing
  else open.** `psp-pulse-usa/BOOT.BIN` settled at 7,933 functions and
  `psp-pulse-eu/BOOT.BIN` at 7,932, both on Ghidra 12.1.2, both confirmed
  across repeated independent imports (`reanalyze`/`run_analysis` report `0
  new functions` on a further pass once there). The historical figures are
  `10,683`-`10,703` for the USA binary and `10,671` for the EU one, recorded
  in [`allegrex-vfpu.md`](../psp/allegrex-vfpu.md),
  [source-images.md](../reverse-engineering/source-images.md) and
  `HANDOVER.md`. This is unrelated to the concurrency bug above (it
  reproduces perfectly serially) and unrelated to Pure - both Pure
  `BOOT.BIN`s land within a handful of functions of their own historical
  counts on a single default pass. Cause unresolved: something beyond a
  default single-pass Auto Analyze (extra analyzer passes? a different
  Ghidra/extension version at the time?) produced the historical Pulse
  numbers. Always compare a fresh Pulse import's count against the reference
  figure before treating it as complete or as evidence of a bad import - a
  ~2,700-function gap here is the current known baseline, not a new failure.

### The database can disagree with `names.tsv`, and the docs win

Moved here from `HANDOVER.md` on 2026-08-09. Per
[ADR-0005](../architecture/adr/0005-ghidra-conventions.md) the docs are
authoritative when they and the database disagree, and these are the instances
that prove it is not a theoretical rule. **Only rename-sweep off addresses
actually listed in `names.tsv`, never off a live Ghidra name** - a stray
in-database rename otherwise propagates into every binary corroborated from it.

- **`apply-ghidra-names.py` can report a rename as applied when it did not
  land.** Sweeping `psp-pulse-eu` (2026-08-05) the script printed `4 applied, 0
  skipped, 1 failed` for a five-row batch, but `get_function_by_address` right
  after showed only the row it called a *failure* had been renamed. Calling
  `rename_function_by_address` directly through the bridge, one address at a
  time, worked every time; only the script's tight sequential loop showed this.
  **Verify with `get_function_by_address` afterwards - do not trust the script's
  own applied/skipped/failed count.**
- **Two live drift instances, both still unresolved.** `0x0884de5c` is
  live-named `Body_Integrate` where `psp-pulse-usa/names.tsv` documents
  `Body_Init` (`rigid-body.md`, confidence 85) and puts `Body_Integrate` at
  `0x0884e230`; and `0x08811630` is live-named `Gu_Fog_q` where the same file
  documents `Gu_TexOffset` (`texture-animation.md`, confidence 88), `Gu_Fog`
  itself being at `0x08811748`. `bulk_fuzzy_match` surfaces the live name as its
  source, not the documented one, which is how both were found while
  cross-binary sweeping. Whichever address is wrong needs a project rename to
  match its page, or a doc correction if the live name turns out right.
- **`apply-ghidra-names.py`'s default run once silently skipped four of five
  binaries' `names.tsv`.** Fixed 2026-08-05; dry-run verified against all five
  post-fix. Explicit positional args plus `--program` still work as before.

### `psp-pulse-eu`'s data addresses do not share USA's base

Its `.text` lines up with USA's function-for-function, but its
`.data`/`.rodata`/BSS do not: USA's `Wad_HashName` computes the CRC table at
`0x08afbffc` while EU's otherwise byte-identical copy computes `0x00022bac`, and
`Camera_PublishTripod` writes its fov global at EU `0x0005aec0` against USA
`0x08b34310`. Every EU data address resolved so far lands in that same low
region, across independently decompiled functions, so it is not a misread. The
likely cause is that EU's `BOOT.BIN` is genuinely smaller (3,844,732 against
3,854,564 bytes), which moves wherever the loader places uninitialised data.

**Practical consequences.** A small-offset guess from a USA data address lands
on unrelated bytes rather than erroring, so the only reliable technique is to
decompile an already-matched referencing function and read the `DAT_xxxxxxxx`
symbol Ghidra's decompiler already resolved -
[`psp-pulse-eu/corroboration.md`](functions/psp-pulse-eu/corroboration.md)'s
"Structural sweep" has the ten resolved that way and the fifteen still open.
And **no EU data rows go into `psp-pulse-eu/names.tsv` until that import is
rebased**: a name pinned to `0x0005aec0` is a name a correct import would move,
which is worse than no name. Record an EU data finding as an offset from a named
function's access instead, the way
[`psp-pulse-eu/camera.md`](functions/psp-pulse-eu/camera.md) does. Rebasing the
import is the actual fix and is nobody's task yet.

### Reading the raw binary outside Ghidra

`jal` targets in `BOOT.BIN` encode the **ELF vaddr** (base 0), not the
`0x08804000` image base. Scanning the extracted binary for calls to
`0x0884d850` means searching for `0x0C012614`, not `0x0E213614`.

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
