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

### Importing a binary

All PSP binaries here are relocatable (`e_type` `0xffa0`) and declare `p_vaddr`
`0x00000000`, so each has to be placed at `0x08804000` by hand.

**Read the next section before importing a PSP binary.** Which import order is
correct depends on whether the Allegrex module carries this repo's relocation
patch, and the two answers are opposites.

#### Why a PSP import silently loses every relocation

Measured 2026-09-05 on `psp-pulse-usa`, headless, into a throwaway project.
Confidence **92**.

`ghidra-emotionengine-reloaded`'s `EE_ElfExtension` - installed here for the
PS2 work - wins ELF adapter selection for PSP files.
`Allegrex_ElfExtension.processGotPlt`, the only call site of
`AllegrexRelocationProcessor.process`, therefore never runs, and **no PSP
relocation is ever applied**. Four steps, each re-checkable with `javap`:

1. `ElfExtensionFactory.getLoadAdapter(ElfHeader)` is a plain first-match-wins
   loop over `ClassSearcher.getInstances(ElfExtension.class)`, testing only the
   **one-arg** `canHandle(ElfHeader)`.
2. `ClassSearcher` orders that list by descending
   `@ExtensionPointProperties(priority)`. `EE_ElfExtension` declares
   `priority = 2`; `Allegrex_ElfExtension` declares nothing, so it gets
   `DEFAULT_PRIORITY = 1` and sorts behind it.
3. `EE_ElfExtension` overrides only the two-arg `canHandle(ElfLoadHelper)` and
   inherits stock `MIPS_ElfExtension`'s one-arg version, which accepts **any**
   `EM_MIPS` ELF - PSP included.
4. So EE is returned for `BOOT.BIN`. Nothing logs an error, because nothing
   failed: the code was never reached.

Allegrex's own two-arg `canHandle(ElfLoadHelper)`, which checks the language is
`Allegrex`, is dead code as far as adapter selection goes - the factory never
calls it. That is why the language check it performs never prevented any of
this.

What that looks like on every PSP database in this project:

```
relocation table entries = 0
0x0894f6d8 word = 0x3c110028   <- byte-identical to BOOT.BIN on disk
0x08ac326c (g_scream_opcode_table) = 0x00189e6c 0x0018bc78 ...
```

The input is fine and the module is fine: `readelf -S BOOT.BIN` shows eleven
`LOPROC+0xa0` (`SHT_PSP_REL`) sections, `.rel.text` alone holding
`0xb6c18 / 8` = **93,443** entries; Ghidra's own `ElfHeader.parse()` sees all
eleven; and `javap -c` on the installed `ghidra-allegrex.jar` confirms
`AllegrexRelocationProcessor` passes `addToRelocationTable = true` and handles
`R_MIPS_26`. The code that consumes all of that never executes.

**This corrects two things this page used to say.** It claimed Pulse's
`lui`/`addiu` constants were "genuinely rewritten", on the evidence that
`0x0894f6d8` held `a908113c` in the database against `2800113c` on disk. The
database holds `2800113c`. Nothing was rewritten, in Pulse or anywhere else -
so the four "escalating instances" this page documented (the Pure-wide
constant wart, the `jal` targets in `Billboard_ConstructResource`,
`Ship_SetState`'s jump-table base, `g_scream_opcode_table`) are not four warts
of differing scope. They are **one bug, seen four times**, and
`real = pseudo + 0x08804000` is the right correction for every one of them
because the image base is precisely what was never added.

The fix is
[`scripts/patches/ghidra-allegrex-psp-elf-extension-priority.patch`](../../scripts/patches/ghidra-allegrex-psp-elf-extension-priority.patch),
applied by `just build-allegrex`: it raises Allegrex's extension-point priority
above EE's and narrows its predicate to PSP files, so the mirror-image bug
cannot appear on PS2 imports instead. **Written and compile-checked, not
installed** - installing it and reimporting the databases is a maintainer
action, and until it happens every PSP database still has this.

#### The import order, and why it flips with the patch

**Unpatched - which is what every current database is.** Import at the default
image base, run full auto-analysis, then set the image base to `08804000`
(`Window > Memory Map > Set Image Base`, or `set_image_base` through the
bridge). Measured fresh and headless on `psp-pulse-usa`, 2026-09-05:

| | analyse, then rebase | loader image base, then analyse |
| --- | ---: | ---: |
| functions found | **10,679** | 7,933 |
| relocation table entries | 0 | 0 |
| `jal` targets landing in `.text` | 4/4 | 0/4 |
| `g_scream_opcode_table` entries in `.text` | 40/40 | 0/40 |

Nothing is relocated in either column. Analysing at base `0` wins only because
the instruction constants and the data listing agree there; setting the base
first moves the listing out from under constants that never change, so a string
pointer resolves below the image base where nothing lives and no reference is
created.

**Patched.** Set `Image Base` in the loader's import options and analyse once -
no rebase step. `AllegrexRelocationProcessor` computes every write against
`program.imageBase` *at load time*, so at `0x08804000` every relocation is
correct on the first pass. Do **not** use analyse-then-rebase on a patched
build headlessly: re-applying relocations after a base change is the job of
`RelocationFixupPlugin`, a `ProgramPlugin` listening for
`ProgramEvent.IMAGE_BASE_CHANGED`, so it exists only inside a running Ghidra
tool. A headless `setImageBase` moves the addresses and leaves the bytes.

This reverses what this page recommended before, and the old page's own
"leading hypothesis" - that setting the base in the loader options would remove
the wart - turns out to have been **right and confounded**, not wrong. It
measured worse only because the relocation pass was not running in either arm.

#### Verifying an import

```sh
just check-ghidra-import
```

It probes the program that is currently open in Ghidra and needs Ghidra
started with `GHIDRA_MCP_ALLOW_SCRIPTS=1` **in its own environment** - the
script endpoint is gated on the Ghidra plugin side, not on the bridge process,
so setting it only in `.mcp.json` is not enough. Only the *current* program is
reachable; check the others by making each current in turn.

**The decisive number is the relocation-table entry count.** A PSP program
whose table has zero entries did not have its relocations applied, full stop -
one query, no ambiguity. The three secondary probes (`psp-pulse-usa` only, since
they name addresses in that binary) are the `lui` at `0x0894f6d8`, the `jal`
calls inside `Billboard_ConstructResource` at `0x08900220`, and
`g_scream_opcode_table` at `0x08ac326c`.

**Function count is not a usable check**, and an earlier version of this page
was wrong to offer one. It quoted "wrong order gave 6,934; right order gives
8,969" for `pure-psp-eu` - but the live `/psp-pure-eu` reads 6,934 and
`/psp-pure-usa` 6,927, so either those databases are the "wrong order" ones by
the page's own metric or the 8,969 figure never meant what the page said. The
count ranges from 6,927 to 10,679 across four near-identical binaries and is
inflated by hand-created functions, so it cannot discriminate. Treat it as an
anchor, never a pass/fail.

A string still failing to resolve back to code is a real signal, just a weaker
one - it survives the relocation bug, because string references are built from
`lui`/`addiu` analysis at base 0 and then slide with the rebase:

```sh
curl -s 'http://127.0.0.1:8089/get_xrefs_to?address=0x08a4c538'
#    -> "From 08898838 in FUN_08898594"   (Data\XML\HandlingStats.xml)
#    -> "No references found"             = the import is bad, redo it
```

Decompilations full of `bad instruction data` and `halt_baddata()` truncations
are the same signal.

#### Redoing a bad import: the delete needs the GUI

`delete_file` through the bridge fails with `"BOOT.BIN is in use"` **after
`close_program` has reported success**, so a folder that has to be thrown away
and reimported cannot be removed over MCP - do the delete or rename in the
Ghidra GUI. Learned redoing `/psp-pure-eu` on 2026-08-10, which had sat at 0
functions for a session; it was reimported rather than diagnosed, which was the
cheaper call.

#### Measured 2026-09-07: `psp-pulse-eu` is reimported and xrefs work

**The patch is installed, `psp-pulse-eu` is reimported, and it reads
`relocations=108729` against zero before.** `just apply-names` applied
**301 of 301** rows to it, against 17 of 301 on the first attempt.

**`get_xrefs_to` now works on that database, for code *and* for data**, which
is the whole point of the fix and is better than `ps3-hdfury-eu` manages
(there, code xrefs work and data xrefs are off by `0xFEEC` because the
reference table was built with the wrong TOC - see the PS3 section above).
Both checked directly:

```
get_xrefs_to 08848a04   (Ship_ApplyLateralGrip)
  -> From 08849a84 in Ship_UpdateCraft [UNCONDITIONAL_CALL]

get_xrefs_to 08ab1af0   (g_vex_class_table)
  -> From 08908878 in Vex_RegisterClass [READ]
  -> From 08908508 in Vex_FindClassDescriptor_q [READ]
```

**So the standing advice to treat an empty PSP xref result as meaningless no
longer applies to `psp-pulse-eu`.** It still applies to every PSP database that
has not been reimported yet - as of this measurement that is `psp-pulse-usa`,
`psp-pure-usa` and `psp-pure-eu`, which remain unrelocated and on which
`search_instructions` and `scripts/psp-relocate.py` are still the only reliable
route.

**The import order that worked**, which is the GUI one this page already
documents rather than the loader-options one: import, `Analysis > Auto Analyze`
at the default base, let it finish, then `Window > Memory Map > Set Image
Base` and enter `08804000`. With relocations actually present,
`AllegrexRelocationFixupHandler` re-applies them on the rebase - the handler
that had never once run here, because there were no relocations for it to
re-apply. A first attempt that left the base at `0x00000000` failed
`apply-names` with `Unable to create function` on all 284 function rows, since
no address in `names.tsv` exists at base 0; the 17 that applied were the
low-address data rows.

#### Reading an unrelocated database, until it is reimported

Until the patch above is installed and the databases reimported, every PSP
database here holds **raw, base-0 instruction bytes**. The rule is one line:

> **`real = pseudo + 0x08804000`**, for every constant, `jal` target and stored
> pointer alike.

The displayed constant is what lies; the references built by analysis at base 0
are correct, so navigation and the string tools still work. Four places this
has already cost time, all the same bug:

| where | what it looks like | source |
| --- | --- | --- |
| a `lui`/`addiu` constant | `0x0898bab8` reads `2800043c`, decompiler prints `0x248538` for `0x08a4c538` | `pure-psp-eu`, 2026-08-09 |
| a `jal` target | calls in `Billboard_ConstructResource` (`0x08900220`) decompile as `func_0x0013ff08`, which `get_function_by_address` cannot resolve | [billboards.md](functions/psp-pulse-usa/billboards.md), 2026-08-28 |
| a jump-table base | `lui at,0x27; addu at,at,a0; lw at,0x7c40(at)` yields `0x00277c40`; corrected, `0x08a7bc40` lands four bytes past the end of `Ship_SetState`'s own nine-entry table (`0x08a7bc18 + 9*4`, one null word, then the next table) | [shield.md](functions/psp-pulse-usa/shield.md), 2026-09-02 |
| a jump table's contents | `g_scream_opcode_table` (`0x08ac326c`) reads forty `0x0018xxxx` values, nowhere near `.text` (`0x08804000`-`0x08a76a3b`); `decompile_function` and `create_function` refuse outright on them, which reads as "nothing is there" rather than "the address is wrong" | [sound.md](functions/psp-pulse-usa/sound.md), 2026-09-04 |

The jump-table corrections landing exactly where an adjacent structure predicts
is what confirms the rule rather than merely fitting it.

These were originally written up as four warts of increasing scope, on the
theory that the relocation pass ran but skipped whatever the auto-analyzer had
not walked. That theory is dead: the pass never ran at all, so there was never
a subset to explain. Anything at all that a relocation would have touched is
raw.

#### Stop doing it by hand: `scripts/psp-relocate.py`

`real = pseudo + 0x08804000` is right for `.text`, and **wrong** for the second
`PT_LOAD` segment - `.cplinit`, `.linkonce.d`, `.ctors` and `.bss` all need
`+0x08ad9798` instead. Worse, the hand arithmetic itself is where this keeps
going wrong: a `lui`/`lo16` pair's low half is *signed*, so a negative `lo16`
borrows from the high half, and one constant slip of `0x348` produced a whole
retracted theory about a third relocation base (see
[anim-transform.md](functions/psp-pulse-usa/anim-transform.md)).

The script replays the PRX relocation sections instead - `SHT_PRXRELOC`
(`0x700000A0`), 93,443 records in `.rel.text` alone - and reports the loaded
address each record names. `r_info`'s bits 16-23 give the segment whose base
that record adds, so nothing is guessed:

```sh
B=data/extracted/psp/pulse-usa/PSP_GAME/SYSDIR/BOOT.BIN
python3 scripts/psp-relocate.py --binary $B segments
python3 scripts/psp-relocate.py --binary $B resolve 0x08840774   # -> 0x08b30f90
python3 scripts/psp-relocate.py --binary $B xrefs 0x08b317b0     # 120 references
python3 scripts/psp-relocate.py --binary $B callers 0x0883e6f4
python3 scripts/psp-relocate.py --binary $B masked 0x1c0 0x400
python3 scripts/psp-relocate.py --binary $B member 0x08b30f90 0xb4
```

It reproduces five independently-recorded addresses with no special-casing -
`0x08ab0838` and `0x08ab0818` in segment 0, `0x08b317b0`, `0x08b32420` and
`g_speedpad_jump` (`0x08b36bec`) in segment 1 - and 97.85% of its 108,813
resolved targets land inside a loaded segment, the rest being import stubs
below the load base. There are exactly **two** `PT_LOAD` segments (`phnum=2`),
so there are exactly two bases.

Two subcommands exist for searches Ghidra structurally cannot serve here:

- **`xrefs` succeeds where `get_xrefs_to` fails**, because it matches on the
  relocated value rather than on Ghidra's index of the raw one.
- **`member` finds a global reached as base-plus-offset**, which has no
  relocation record of its own and so is invisible to any xref, relocated or
  not. `DAT_08b32428` is `DAT_08b32420 + 8`; it returns zero references from
  every tool including this script's own `xrefs`, and that zero is what
  prompted a since-refuted `$gp`-addressing hypothesis. This binary uses `$gp`
  as a load/store base **zero** times; base-plus-offset is the real mechanism,
  and `easyshield` at `stats+0x84` is the same shape.

An empty result from any of these is still worth calibrating against a known
case before believing it, for the reasons the `.rodata` string note gives.

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
- **Analyse first, rebase second - the order decides whether the database is
  usable.** See [Importing a binary](#the-import-order-and-why-it-flips-with-the-patch)
  below; `/psp-pure-eu/BOOT.BIN` is what the wrong order produces.
- **The symptom of a wrongly-imported binary is that every address-based string
  tool silently returns zero.**
  `get_xrefs_to`, `find_undocumented_by_string` and `search_byte_patterns` on a
  pointer all report no references for strings that are demonstrably in use - no
  error, just an empty result that reads as "this string is unused". A string
  listed at `0x08a4c538` is referenced in code as `0x248538`
  (`data address = code constant + 0x08804000`), and the decompiler prints the
  raw constant too. Measured **not** to affect either Pulse binary, including
  the equally large `psp-pulse-eu`, so import size does not predict it;
  `psp-pure-usa` is untested and is the same vintage. The search that does work,
  and the `lui`-high-half check that avoids the false positives it invites, are
  in [`psp-pure-eu/string-anchors.md`](functions/psp-pure-eu/string-anchors.md).
- **Re-importing that binary correctly is worth ~2,000 functions and a large
  drop in fuzzy-match noise.** A clean import (default base `0`, full
  auto-analyze) yields **8,969** functions against the current database's
  **6,934**, and the current database's decompilations are littered with
  `bad instruction data` / `halt_baddata()` truncations that the clean one does
  not have. Measured effect on cross-binary matching, using `psp-pulse-usa`'s
  `Body_Integrate` as the probe: the best-match **score is identical** (0.8013
  either way - the matcher normalises immediates, so relocation state does not
  move scores), but candidates above threshold 0.7 fall from **72 to 5**. That
  is aimed squarely at the "collision-prone" exclusions recorded in
  [`psp-pure-eu/corroboration.md`](functions/psp-pure-eu/corroboration.md), so
  **the fuzzy sweep is worth re-running once the binary is re-imported at
  `0x08804000`** - not for better scores, but for far fewer false ties.
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
- **A VFPU `lv.q`/`sv.q` quadword transfer decompiles as four separate scalar
  assignments, with nothing marking that they came from one 16-byte move.**
  `decompile_function` on `Billboard_ConstructResource_q`
  (`psp-pulse-usa`, `0x08900220`) rendered a matrix write as "sixteen literal
  loads from consecutive addresses" and that summary was read as "the literal
  identity matrix" for two passes - true for fifteen of the sixteen floats,
  false for the sixteenth, which a same-block scalar `swc1` overwrites right
  after the `sv.q` that placed it. The decompiler gives no visual cue that a
  later scalar store lands inside the block a `lv.q`/`sv.q` pair just moved as
  a unit. Confirm a transform/matrix write's row boundaries with
  `disassemble_function`, not `decompile_function`'s summary, before calling
  any part of it "identity" or "unused" - see
  [`billboards.md`](functions/psp-pulse-usa/billboards.md#construction-writes-no-transform---true-for-placement-not-true-for-identity)
  for the full correction.

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

  **Checked 2026-09-02, and all six are false leads - closing this table
  rather than leaving it open.** `diff_functions` against the USA address gave
  a middling similarity (0.59-0.75) for every row, well below the
  near-1.0/body-identical bar that actually earned `Camera_SubmitScene`/
  `Camera_PublishTripod` their confidence 90 (shared distinctive literals, not
  just a score) - `AmbientLight_Construct`'s pair is 65 EU instructions
  against 31 USA, the kind of size mismatch a real counterpart doesn't have.
  Decompiled the two highest-scoring rows to check directly rather than trust
  the score alone: `0x0892d6f0` (the `AmbientLight_Init` candidate) takes no
  arguments and writes two different constant pairs to fixed globals through
  two calls to what decompiles as the same function - nothing like EU's
  single-pass `self`-mutating `AmbientLight_Init`. `0x0890cc80` (the
  `Mesh_ApplyMaterialLighting` candidate) is a **different, unrelated node's
  submit method** - it builds a back-to-front sort key with the exact
  `349.525`/`0xfffff` constants documented on
  [`exhaust.md`](functions/psp-pulse-usa/exhaust.md)'s `ExhaustFlare_Submit`,
  which this candidate is not. Both negatives are consistent with the same
  root cause this section already names: the 2026-08-09 permutation incident
  scattered `psp-pulse-eu` names onto nearby but wrong `psp-pulse-usa`
  addresses, and a fuzzy sweep's neighbourhood match landed on whichever
  unrelated function happened to sit near the true one. **Not chased
  further**: the true USA counterparts for these six EU functions are still
  unfound, and a fresh search (not reusing these six addresses) is what
  finding them would need.

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

### `get_xrefs_to` on `ps3-hdfury-eu`: code xrefs are sound, data xrefs are not

Two different answers, and conflating them wastes a session either way.

**Code xrefs work, and this is worth knowing** - the PSP databases return
nothing from `get_xrefs_to` because Ghidra applies no Allegrex relocation, and
that defect does **not** carry over to the PS3 PowerPC database.
`get_xrefs_to 0x00054628` (`RaceManager_GetInstance`) returns 20 call sites,
several landing in independently named functions. A `bl` displacement is encoded
in the instruction with no TOC involved, so call resolution never depended on
the TOC being right. **Use it. Tracing callers and parents directly is available
on this binary and is not available on the PSP ones.**

**Data xrefs are systematically wrong for any function whose TOC is not the
analysis-time default**, in exactly the way [memory.md](functions/ps3-hdfury-eu/memory.md)
describes for the decompiler. The reference table was built with the wrong `r2`
and never recomputed. Worked example, all four checks against the same
instruction:

- `get_xrefs_to 0x008a704c` reports `From 003a43a8 in .opd.FUN_003a4380 [READ]`.
- `disassemble_function 0x003a4380` shows that instruction is
  `lwz r30,-0x648c(r2)`.
- `scripts/ps3-toc.py resolve 0x003a4380 -0x648c` gives **`0x008b6f38`**.
- `get_xrefs_to 0x008b6f38` returns **"No references found"**.

The observed shift is `0xFEEC`. Confusingly, Ghidra's *decompiler* gets the same
operand right (it prints `PTR_DAT_008b6f38`), so a decompile and an xref query
on one instruction can disagree, and the decompile is the one to believe.

**Rule: never take a data xref on this binary at face value.** Resolve the
displacement with `scripts/ps3-toc.py resolve <function> <disp>` and confirm the
reverse direction. A wrong-TOC data address will also have a plausible-looking
Ghidra label attached to it - `0x008a704c` came out as `PTR_s_WIP3OUT_008a704c`,
a real pointer to a real string, just not the one the code loads. That is how
the wrong name reached
[billboards.md](functions/ps3-hdfury-eu/billboards.md) and survived a session.

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

## Capturing a database before it is replaced

A reimport - a new loader, a relocation patch, a corrupted project - discards
everything the database held that `names.tsv` does not carry. Snapshot it first.
[ADR-0047](../architecture/adr/0047-database-state-beyond-names-is-captured-not-replayed.md)
sets out what a capture is and, more usefully, what it is not.

Two steps, because only the first needs Ghidra:

1. Run `scripts/ghidra/DumpDatabaseState.java` from the Script Manager, or
   through the MCP bridge's `run_script_inline`, with a scratch directory as its
   argument. It walks **every program in the project**, not the current one, and
   writes raw per-program TSVs plus a `survey.txt` of the counts.
2. `just capture-ghidra-state <that directory>` filters the tool's own output
   out of the dump and writes `docs/ghidra/captures/`. Update the measurement
   table in that directory's README, then `just check-captures`, which asserts
   the table matches the files.

The filter's output is deterministic - sorted by address, with the duplicate
comment that carries the function name pinned as the survivor - so re-running it
against an unchanged database produces an empty `git diff`. That empty diff is
the check worth making after step 2.

**Step 1 has been compile-verified but not yet run end-to-end** (Ghidra exited
mid-session during the PSP database swap). The captures currently in the tree
came from a functionally identical inline script. First person with Ghidra open:
run it and confirm the diff is empty.

**Identify a program by its `DomainFile` pathname, never by its display name or
executable path.** Four programs in this project are called `BOOT.BIN`, and an
old database and its fresh reimport (`BOOT.BIN` and `BOOT.BIN.0`) report the
*same* executable path, because they were imported from the same file. Only the
pathname and the version number tell them apart, and version is the reliable
one: a worked database is at version 69, a fresh import at 2. `switch_program`'s
`"success": true` is not trustworthy for these - see
`scripts/apply-ghidra-names.py`'s `Bridge.switch_program`, and note that
`audit-ghidra-names.py`'s `ensure_active` path check cannot distinguish the two
either, for exactly the shared-path reason above.

### Reading a second project

A Ghidra project from another machine can be read without merging anything into
the live one: `GhidraProject.openProject(dir, name, false)` inside a script, then
the same walk. Close it in a `finally` block. **A script that throws before
`gp.close()` leaves a lock file** (`<name>.lock`, `<name>.lock~`) beside the
`.gpr`, and the next attempt fails with "Project is locked. You have another
instance of Ghidra already running" naming this very machine. Delete the two
lock files and retry.

Never merge a foreign project into the live one. Compare it in the repository
instead - `names.tsv` is what the two machines share, and if it is doing its job
the foreign project holds nothing new.
