# Bridging symbol names between PPSSPP and this repository

`kotcrab/ghidra-allegrex` (the Allegrex Ghidra processor module `just
build-allegrex` builds) ships two Ghidra scripts that were never used here
until this page: `PpssppImportSymFile.py` and `PpssppExportSymFile.py`, at
`data/tools/ghidra-allegrex/ghidra_scripts/`. The maintainer's own note on
them:

> PPSSPP identifies many functions automatically, it's useful to get those
> into Ghidra after doing the initial analysis. With your game loaded in
> PPSSPP use the Debug menu to save the .sym file then in Ghidra run
> PpssppImportSymFile. Enter 0 when asked for offset if your image base is
> already at 08804000. The script by default skips unknown names from PPSSPP
> so your work can only get overwritten if you've renamed one of the
> autodetected functions. Likewise PpssppExportSymFile exports your work as a
> .sym which can be imported into PPSSPP. You need to do Reset symbol table
> before importing.

**The offset is always `0`.** Every `docs/ghidra/functions/*/names.tsv`
header already states image base `0x08804000` - exactly the case the note
describes.

## The `.sym` format

Read directly from `PpssppExportSymFile.py`, one line per symbol, no header:

```
%08X %s,%04X
```

Address, name, comma, size - all hex, space-separated. `PpssppImportSymFile.py`
splits every line on the first space and reads `parts[0]` as a bare hex int
unconditionally, so **a `#` comment line crashes the import** - nothing in
this project's own output ever writes one.

## Two things this repository must never do with it

1. **Never let a PPSSPP-autodetected name into `names.tsv`.** That file's
   contract (CLAUDE.md, `scripts/apply-ghidra-names.py`) is that every row
   cites an evidence page whose text still contains the row's address and
   name; `just check-names` refuses a row that doesn't. PPSSPP's own
   detections - HLE syscall stubs and its hash-matched function-replacement
   database (`memcpy`, `sinf`, `qsort`, ...) - have no such page: nobody read
   the function and wrote why it is what PPSSPP says it is.
2. **Never load the wrong region's `.sym` into a running PPSSPP.** The USA
   and EU PSP binaries are genuinely different: `Cannon_UpdateRound` is
   `0x0886593c` on USA and `0x08865798` on EU. Loading USA names against an
   EU disc silently mislabels every address that happens to exist at the same
   offset in both.

## Export: `names.tsv` -> `.sym`, offline

`scripts/export-ppsspp-sym.py <binary>` goes **straight from `names.tsv` to
`.sym`**, not through Ghidra's own exporter - the Ghidra project is not
committed, `names.tsv` is the authoritative source, and a direct exporter
needs no Ghidra instance running and cannot drift from what the docs say.

```sh
python3 scripts/export-ppsspp-sym.py psp-pulse-usa
# -> data/ghidra/psp-pulse-usa.sym (686 functions)
```

Output goes under gitignored `data/ghidra/` by default (`--out` overrides) -
generated from `names.tsv`, never a second copy of it, the same as
`data/ghidra/psp-imports.tsv` (`just resolve-imports`'s own output, likewise
never committed).

Then in PPSSPP: **Debug menu -> Load symbol map...** and pick the file. Only
the four PSP binaries are accepted - `psp-pulse-usa`, `psp-pulse-eu`,
`psp-pure-usa`, `psp-pure-eu` - the other three `names.tsv` directories
(`ps2-pulse-eu`, `ps3-hdfury-eu`, `vita-2048-*`) name CPUs PPSSPP does not
run at all, and there is no default binary: the caller has to say which one,
every time.

Two things intentionally *not* carried across:

- **`data` rows are dropped, not exported.** `PpssppExportSymFile.py` itself
  only ever walks `getFunctionManager().getFunctions()` - there is no data
  symbol in the format it produces, so importing one as a "function" would
  tell PPSSPP's disassembly view something false. `psp-pulse-usa` alone has
  80 of these; the script counts and reports them rather than silently
  skipping.
- **Size is the gap to the next row's address, not a real measurement.**
  `names.tsv` carries no size column at all (`address kind name confidence
  evidence`). The first version of this exporter wrote `0000` on the
  reasoning that a zero-size symbol is "still a correctly named point" and
  inventing a size would claim knowledge the table doesn't have - checked
  against PPSSPP v1.20.4's own `SymbolMap.cpp` and found wrong: `AddFunction`
  always calls `AddLabel` too, so the entry address gets a name via the
  exact-match `GetLabelName` lookup regardless of size, but `GetFunctionStart`
  (the range lookup `memory.disasm`'s per-instruction `function` field uses)
  checks `start <= address && start+size > address` - at `size=0` that is
  false for *every* address, including the start address itself. A `0000`
  size names exactly one address per function and nothing else in its body.
  So the exporter now uses the gap to the next row's address (function or
  `data`, whichever `names.tsv` places next) - a real upper bound derived
  from data the table does carry, not a fabricated size, and the same
  heuristic `PpssppExportSymFile.py` itself uses
  (`getBody().getFirstRange().getLength()`, its own comment calling it "not
  ideal but should cover most cases").

  **Confirmed live, 2026-09-09, after the reasoning above rather than instead
  of it.** Two halves, on a fresh `PPSSPPHeadless` against
  `pulse-psp-usa.chd`. (1) `Wad_HashName` at `0x08940d0c` (exported size
  `01A4`, 420 bytes): `hle.func.rename` then `memory.disasm` returns
  `"function": "Wad_HashName"` on the entry instruction - the name comes back.
  (2) The size question directly: `hle.func.add` with an explicit 32-byte size
  inside an existing function's body registers a new function whose
  `memory.disasm` `function` field spans all 32 bytes and is `null` at
  `address+32` - it groups the body and stops at the boundary, which is
  exactly what `size=0` could not do.

  **Why a websocket test is equivalent evidence to loading a real `.sym`**, and
  this was byte-verified against PPSSPP's source with `curl` rather than a
  summarised fetch: `Qt/mainwindow.cpp`'s "Load .SYM" action calls
  `LoadNocashSym`, which for any line whose size is not `1` calls the identical
  `g_symbolMap->AddFunction(...)` that the `hle.func.add` handler's success path
  calls. The two entry points converge before anything observable. See
  [ppsspp-debugger.md](ppsspp-debugger.md#naming-a-function-live-with-hlefuncaddrename-and-a-crash-to-avoid)
  for the worked example that found this, live, over `hle.func.rename`.

The `_q` suffix carries across exactly as `apply-ghidra-names.py` derives it
(the export script imports that module rather than restating the threshold,
the same trick `check-ghidra-names.py` already uses) - a sub-70-confidence
name reaches PPSSPP looking exactly as provisional as it looks in Ghidra.

## Harvest: what does PPSSPP know that we do not?

`docs/reverse-engineering/ppsspp-debugger.md` records that `memory.disasm`
carries "PPSSPP's own symbol names for calls" - but there is a purpose-built
listing rather than a disasm-scrape: `hle.func.list` over the websocket
debugger returns every function PPSSPP's analysis found, with address, name
and size, for the whole loaded module in one call. No GUI, no Debug menu, and
it works **right after boot** - module signature scanning happens at load
time, so there is no need to reach a menu or start a race first.

```sh
# terminal 1 - boot with the debugger, breaks at the entry point automatically
PPSSPPHeadless data/cache/pulse-psp-usa.iso --debugger=47800 --graphics=software --timeout=1800

# terminal 2
uv run --with websocket-client scripts/harvest-ppsspp-symbols.py psp-pulse-usa --port 47800
```

**The honest headline: 60 real, useful names out of 13,372 functions PPSSPP
finds - and the useful 60 are dominated by the transcendental math routines
`just check-determinism` already cares about.** Measured against PPSSPP
v1.20.4, 2026-09-09, both PSP Pulse pressings:

| Binary | Total functions PPSSPP found | Named (not `z_un_`) | `zz_`-prefixed HLE stubs | Real detections | Real detections `names.tsv` doesn't already have |
| --- | --- | --- | --- | --- | --- |
| `psp-pulse-usa` | 13,372 | 408 | 335 | 73 | 60 |
| `psp-pulse-eu` | 13,364 | 407 | 334 | 73 | 63 |

**The 73 real, non-stub detections per binary are libc/newlib, soft-float
helpers, and - the interesting part for this project - the transcendental
math routines**: `sinf`, `cosf`, `tanf`, `atanf`, `atan2f`, `acos`, `sqrtf`,
`pow`, `powf`, `expf`, `logf`, `fmodf`, `scalbn`/`scalbnf`,
`copysign`/`copysignf`, `isnan`/`isnanf`, `finitef`, `rint`, plus a handful of
`sceGu*`/`sceGup*` graphics-library calls and one PPSSPP-specific hack entry,
`expensive_wipeout_pulse` at `0x08833edc` (a game-specific optimisation
PPSSPP itself carries a name for - not investigated further here). Knowing
which original addresses **are** `sinf`/`cosf`/etc is directly useful:
`just check-determinism` polices which transcendentals reach *our*
simulation code, and this is independent confirmation of which functions in
the *original* binary are the platform's own libm, as opposed to something
game-specific that merely resembles one.

The remaining 335/334 `zz_`-prefixed entries are PPSSPP's own HLE syscall
trampolines - **306 of USA's 335 already duplicate the addresses in a live
Ghidra session's `data/ghidra/psp-imports.tsv`** (`just resolve-imports`'s
own generated, gitignored output, applied at confidence 99 from the NID
table, more precise than a generic stub signature match). That comparison is
a measurement made against whatever happened to be sitting in this checkout's
`data/ghidra/` at the time, not a committed fact - `psp-imports.tsv` is
regenerated by `just resolve-imports` and never tracked, so treat "306 of
335" as illustrative of the overlap's shape rather than a number to cite
verbatim elsewhere. The `zz_` prefix itself is PPSSPP's own convention for
"this is a syscall stub, not a real function" - **do not confuse it with this
project's `_q` confidence suffix**, they mean unrelated things.

Only 13 of USA's 73 real detections (10 of EU's) land on an address
`names.tsv` already names - see the overwrite-risk table below. The harvest
itself is committed at `docs/ghidra/captures/psp-pulse-usa/ppsspp-detected.tsv`
and `docs/ghidra/captures/psp-pulse-eu/ppsspp-detected.tsv` - a sibling of
that directory's Ghidra-capture files (`functions.tsv` etc.), never merged
into them; see `docs/ghidra/captures/README.md`. Provenance is its own
`source` column (`ppsspp-hle.func.list`) rather than `names.tsv`'s
evidence-page mechanism or the real captures' Ghidra `SourceType` column -
**never merged into `names.tsv`**. `psp-pure-usa` and `psp-pure-eu` were not
harvested - Pure's `names.tsv` is thin (60/43 rows, `oag-pure` is
deliberately thinner than `oag-pulse`) and PPSSPP's detections are dominated
by newlib/libm, which do not depend on the game; the Pulse harvest already
answers "what does PPSSPP know that we do not" for that shared code.

## Import: `names.tsv` -> Ghidra, via PPSSPP's own `.sym`

**Not run this session** - the Ghidra bridge was held by another lane, and
importing means writing to the shared Ghidra project. What running
`PpssppImportSymFile` would do, computed offline from the harvest above with
no bridge needed:

- **Offset: `0`.**
- **`skipZun=True` (the script's own default) does not protect any of the 13
  (10 on EU) collisions below.** That flag skips PPSSPP's *unknown*
  placeholder names (`z_un_<addr>`), not its *known* ones - and every
  colliding address here is one PPSSPP successfully named. A default run
  **would** overwrite these with PPSSPP's generic name, undoing work already
  in `names.tsv`:

  | Address (USA) | `names.tsv` today | PPSSPP would write |
  | --- | --- | --- |
  | `0x088107f0` | `Gu_DrawBuffer` | `dl_write_framebuffer_ptr` |
  | `0x08811748` | `Gu_Fog` | `sceGuFog` |
  | `0x088117cc` | `Gu_ColorFunc` | `sceGuColorFunc` |
  | `0x0881197c` | `Gu_BlendFunc` | `sceGuBlendFunc` |
  | `0x08811b58` | `Gu_SetState` | `sceGupSetStatus` |
  | `0x088126ac` | `Gu_Material` | `sceGupMaterial` |
  | `0x089732d8` | `strcasecmp` | `strcasecmp` (identical, harmless) |
  | `0x089733ec` | `Libc_StrChr` | `strchr` |
  | `0x0897349c` | `strlen` | `strlen` (identical, harmless) |
  | `0x08973770` | `strrchr` | `strrchr` (identical, harmless) |
  | `0x0897e030` | `cosf` | `cosf` (identical, harmless) |
  | `0x0897e300` | `sinf` | `sinf` (identical, harmless) |
  | `0x08985ff8` | `Math_TransformVec4` | `sceVfpuMatrix4Transform` |

  Seven of the thirteen are a real downgrade (a documented, game-specific
  name replaced by PPSSPP's generic one); six are no-ops because the names
  already agree. EU has the same six-real/four-identical shape at its own
  ten addresses (see `docs/ghidra/captures/psp-pulse-eu/ppsspp-detected.tsv`
  against `docs/ghidra/functions/psp-pulse-eu/names.tsv`).
- **The fix, when someone does run it with a Ghidra bridge:** import first
  (bringing in the 395 addresses - USA count; 397 for EU - PPSSPP found that
  `names.tsv` does not have anything for, mostly the 60/63-per-binary real
  detections above plus whatever `zz_` stubs a freshly regenerated
  `psp-imports.tsv` missed), then immediately re-run `just apply-names` - it
  applies every `names.tsv` row unconditionally, so it repairs exactly the
  seven real collisions above without anyone hand-tracking which ones PPSSPP
  touched. Running `apply-names` second is the safety net the maintainer's
  own note gestures at ("your work can only get overwritten if you've
  renamed one of the autodetected functions") - it can be overwritten, but
  only until the next `apply-names`.
- **Or skip the file round trip and the crash risk entirely: `hle.func.rename`
  live, one address at a time.** [`ppsspp-debugger.md`](ppsspp-debugger.md#naming-a-function-live-with-hlefuncaddrename-and-a-crash-to-avoid)
  confirms it renames a `z_un_...`/PPSSPP-detected entry in place, correctly,
  for the whole function body, with no Ghidra bridge involved at all - it
  just doesn't touch the Ghidra database the way a real `PpssppImportSymFile`
  run would, so it's a way to get a *running PPSSPP* readable, not a way to
  get Ghidra's copy of `names.tsv` updated with what PPSSPP found.
