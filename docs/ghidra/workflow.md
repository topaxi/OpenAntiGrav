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
- **Those "two live drift instances" were 527 of them, and the count is the
  finding.** This bullet used to name `0x0884de5c` (`Body_Integrate` where the
  docs say `Body_Init`) and `0x08811630` (`Gu_Fog_q` where the docs say
  `Gu_TexOffset`) as two isolated, unresolved cases found incidentally while
  cross-binary sweeping. Auditing the whole program on 2026-08-09 - see the
  section below - found **527 of `psp-pulse-usa`'s 753 live names disagreeing
  with `names.tsv`**, including `Gu_Fog` (95), `Xml_AttributeAsFloat` (95),
  `Wad_BuildCrcTable` (95) and `Ship_UpdateCameraRigs` (82), the spine of
  `camera.md`'s field-of-view chain, which read `Ship_UpdateSideshiftInput_q`.
  Whole families were **permuted**: the four `HandlingXml_Parse*Camera` parsers
  each carried a sibling's name, two slots along.

  **Two isolated defects and a systematic one need different responses**, which
  is why the count mattered more than either instance. Both named addresses are
  now repaired, along with the other 525, by re-running `just apply-names`;
  `names.tsv` was right throughout and needed no correction.
- **`apply-ghidra-names.py`'s default run once silently skipped four of five
  binaries' `names.tsv`.** Fixed 2026-08-05; dry-run verified against all five
  post-fix. Explicit positional args plus `--program` still work as before.

### Auditing the database against `names.tsv`: `just audit-names`

**`apply-ghidra-names.py` is additive and cannot clean up after anything.** It
writes the names the documentation has evidence for and never removes one, so a
database accumulates names from sources that left no record - a fuzzy
cross-application sweep, a rename made during an investigation, an experiment
nobody undid - and those outlive the session that made them. `just apply-names`
does not know they are there.

[`scripts/audit-ghidra-names.py`](../../scripts/audit-ghidra-names.py) reads
every named function out of a program and diffs it against that binary's
sanctioned set (`names.tsv`, plus `data/ghidra/psp-imports.tsv` for
`psp-pulse-usa`), reporting two kinds of disagreement:

| verdict | meaning | what to do |
| --- | --- | --- |
| **MISMATCHED** | `names.tsv` documents that address as something else | `just apply-names` - the table is right, the database drifted |
| **UNSANCTIONED** | no row for that address at all | judgement, see below |

`psp-pulse-usa` on 2026-08-09, before and after a repair run:

| | before | after `just apply-names` | after `--prune` |
| --- | ---: | ---: | ---: |
| mismatched | **527** | **0** | 0 |
| unsanctioned | 116 | 116 | **0** |

`psp-pulse-usa` is clean as of 2026-08-09: 659 named functions, every one of them
sanctioned. `--prune` reverts unsanctioned names to `FUN_<addr>` and deliberately
leaves mismatched ones alone, since those have a documented name and
`just apply-names` restores it.

The 116 split cleanly, and the split is what makes them actionable:

- **110 are duplicates** - a documented name sitting at a *second*, undocumented
  address, so the function has both its correct name and a stale twin. Every one
  is a live trap of the kind that cost a reading this same day, when a decompile
  of "`Ship_UpdateStartBoost`" turned out to be a steering filter because the
  real one is 0x338 further on.
- **6 looked like undocumented recoveries and are not.** All six are
  **`psp-pulse-eu` names sitting on `psp-pulse-usa` addresses** - the five M6
  authored-lighting functions plus `Mesh_ApplyShinemapReflection`, every one of
  them documented in `psp-pulse-eu/names.tsv` against
  [lighting.md](functions/psp-pulse-eu/lighting.md), and none of them documented
  for this binary. (The sixth also arrives doubly suffixed, because
  `Row.symbol` derives the `_q` from the confidence column and that row bakes
  one into the name as well.)

  **The addresses are worth keeping even though the names went**, because they
  are a free lead: each is a plausible USA counterpart of a function already
  read on the EU binary, and `diff_functions` would settle each one cheaply.

  | EU (documented) | USA address the sweep chose | confidence on EU |
  | --- | --- | ---: |
  | `AmbientLight_Init` `0x0892d708` | `0x0892d6f0` | 85 |
  | `AmbientLight_Construct` `0x0892d7ec` | `0x0892d7c4` | 88 |
  | `AmbientLight_RegisterClass` `0x0892d960` | `0x0892d944` | 90 |
  | `DirectionalLight_RegisterClass` `0x08934fc8` | `0x08934ea0` | 90 |
  | `Mesh_ApplyMaterialLighting` `0x0890cea8` | `0x0890cc80` | 78 |
  | `Mesh_ApplyShinemapReflection` `0x0890d8d4` | `0x0890d828` | 55 |

  A verified pair earns a `psp-pulse-usa/names.tsv` row and a USA evidence page;
  until then the addresses are candidates, not names, which is why the database
  no longer asserts them.

**Run the audit after any sweep**, and especially after anything that renames
off fuzzy matches. The check is cheap and the alternative is discovering a
permuted family by decompiling one of them.

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
