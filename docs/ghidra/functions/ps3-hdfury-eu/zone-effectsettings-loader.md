# HD's effectSettings load path is found; what it does with the file is blocked on the TOC defect

`EBOOT.elf` (Wipeout HD/Fury, PS3), image base `0x00000000`. Found while
chasing `docs/formats/effectsettings.md`'s open question of what selects a
Zone stage at runtime, from the HD side - 2048's own `Zone_UpdateStage`
(`docs/ghidra/functions/vita-2048-eu-v104/zone-environment-fallback.md`) is a
different executable and does not answer this for HD.

**No confidence-50+ claim is made here about what the load does past the
call site.** Everything below is either TOC-verified fact (manual OPD lookup,
not Ghidra's own resolution) or explicitly marked unresolved. Nothing is
renamed this pass.

## HD's own four effectSettings strings

```
0x007b3e08  Data/Environments/DetonatorModeDLC3.effectSettings
0x007b3e40  Data/Environments/ZoneModeDLC3.effectSettings
0x007b3e70  Data/Environments/DetonatorMode.effectSettings
0x007b3ea0  Data/Environments/ZoneMode.effectSettings
```

Found via `search_strings` with no `program` parameter (see the tooling trap
below - passing one is unreliable here). Plain `Data/Environments/` paths,
unlike the `data/ZoneEnvironmentHDFury/...` naming 2048's own fallback code
constructs when reaching for these same four files
(`docs/ghidra/functions/vita-2048-eu-v104/zone-environment-fallback.md`) -
that prefix is 2048's own invention for referring to HD's files, not
something HD's own executable uses for itself.

Each string sits behind its own global `char*` slot in a data region at
`0x008b8100`-`0x008b8130`, interleaved with unrelated globals (some other
pointers, at least one IEEE-754 float `1.0f`) - four adjacent declarations
the linker happened to place together, not an isolated four-entry table.
`ZoneMode.effectSettings`'s own slot is `0x008b8120`.

## `FUN_003f3fb0` loads it - TOC-verified, and independently corroborated

`FUN_003f3fb0` is not a fresh lead: `docs/formats/envsettings.md`'s own
"Where the executable touches this" section already names it (before this
session) as a large per-race setup function that also loads `"sky.gtf"`,
`"skycube"` and `"Data/Tex/ZoneSky.gtf"` - i.e. already independently
identified as environment/scene setup, from a different investigation
entirely. This session adds a second, TOC-verified data point rather than
starting cold:

1. `search_instructions(mnemonic: lwz, operand_pattern: 52a4)` found
   `lwz r3, -0x52a4(r2)` at `0x003f4140`, inside `FUN_003f3fb0`.
2. `FUN_003f3fb0`'s own OPD entry (`search_byte_patterns` for its address,
   then reading the `{func, toc}` pair `docs/ghidra/functions/ps3-hdfury-eu/memory.md`
   describes) gives its real TOC as **`0x008bd3c4`** - the *second* of the
   binary's two TOCs, not the one Ghidra assumes for everything.
3. `0x008bd3c4 - 0x52a4 = 0x008b8120` - **exactly** the `ZoneMode.effectSettings`
   string's own global slot, computed by hand rather than trusted from
   Ghidra's decompiled variable name (which would show a different, wrong
   global under its own assumed TOC).
4. The very next instruction, `0x003f4144`, is `bl 0x003d6dc8` - a direct
   call, passing the loaded string pointer as `FUN_003d6dc8`'s sole argument
   (`r3`). Call targets are absolute/PC-relative, not TOC-relative, so this
   part of the chain is unaffected by the TOC question.

So: **`FUN_003f3fb0` reads `ZoneMode.effectSettings`'s path and hands it to
`FUN_003d6dc8`.** This much is fact, not inference - two independent
verifications (manual OPD math, and a prior session's unrelated
identification of this same function as environment setup) agree.

## What `FUN_003d6dc8` actually does with it: blocked

`FUN_003d6dc8`'s own OPD entry was checked the same way: its real TOC is
**also `0x008bd3c4`**, not Ghidra's assumed `0x008ad4d8`. Its full decompile
was pulled anyway, and reads as HUD `SpeedBar`/`SpeedBarText`/
`OpponentTargetZone` widget setup - but **that reading is not trustworthy**.
Every symbolic name in it is Ghidra's resolution under the wrong TOC, which
is precisely the failure mode `memory.md` documents with its own worked
example (`FwMemHeap_Create`'s report string reading as `"forward"` under the
wrong TOC and the real string, correctly, under the right one). There is no
reason to expect this function's dozens of TOC-relative loads land on the
right globals by chance, and "coincidentally looks like a real subsystem"
(HUD widgets are a plausible thing for *some* function in this binary to
touch) is exactly how a wrong reading survives review.

**Closing this needs one of two things, neither attempted here**:

1. Re-run `scripts/import-ps3-eboot.sh` (which now includes
   `AssignPs3R2FromOpd.java` per `memory.md`) to fix TOC assignment across
   the whole database - a whole-project action, not something to do
   speculatively mid-investigation without the maintainer's say-so, since it
   touches every function's applied names.
2. Manually verify every one of `FUN_003d6dc8`'s TOC-relative loads by hand,
   the way the two loads above were - realistic for one or two instructions,
   not for a ~400-line function.

## A new trap: byte-pattern search across a TOC boundary

Worth recording because it cost real time this pass and will cost it again.
`search_instructions`/`search_byte_patterns` match on **raw encoded bytes**,
which is TOC-blind: the same `-0x52a4(r2)` displacement appears in multiple
functions, and each one's *real* target depends on **that function's own**
TOC, not a shared one. Two hits from the same byte-pattern search in this
pass:

- `FUN_003f3fb0` (TOC `0x008bd3c4`): `-0x52a4(r2)` resolves to `0x008b8120`,
  the `ZoneMode.effectSettings` string - the lead this page is about.
- `FUN_000b6b98` (TOC `0x008ad4d8`, confirmed via its own OPD entry): the
  *same* `-0x52a4(r2)` displacement resolves to `0x008a8234` - a completely
  unrelated global, one of two lookup tables `FUN_000b6b98` itself indexes
  by a field at `PTR_DAT_008a822c + 0xd4` to write a float at some object's
  `+0x160`. Initially treated as a second lead into this same mechanism; it
  is not. Ruled out only by checking its OPD entry directly, the way
  `memory.md`'s own three-step method requires - the operand text alone
  (`"-0x52a4(r2)"`, identical in both hits) cannot tell the two apart.

**So a byte-pattern/instruction search across this binary needs its own
per-hit OPD check before any hit is trusted**, the same way a string xref
already does - `memory.md`'s warning covers string cross-references
specifically, and this extends the same caution to instruction/byte-pattern
searches for the same underlying reason.

## What this settles and what it does not

- **Settles**: HD's own executable does read its `ZoneMode.effectSettings`
  path, from a function independently corroborated as per-race environment
  setup by two unrelated investigations. The load site and its immediate
  call target are fact, not inference.
- **Does not settle**: what `FUN_003d6dc8` does with the string - parse it,
  open it, thread it through as an opaque token, or something else. No
  stage-selection field (2048's `+0x634` equivalent) was found on the HD
  side; the trail into `FUN_003d6dc8` is exactly where it stops.
- **Does not settle**: whether HD's own selection mechanism looks anything
  like 2048's `Zone_UpdateStage` (a clamped, cross-faded per-craft field).
  Nothing here reaches far enough to compare.

## 2026-08-28: the TOC defect is fixed project-wide; `FUN_003d6dc8` redecompiled

The maintainer re-ran `scripts/import-ps3-eboot.sh --ps3-cspec`, closing the
whole-project blocker option 1 above named. Verified directly through the
live bridge rather than trusted from the script's own output: the program
now reports language `PowerPC:BE:64:A2ALT-32addr-PS3` (not the old
`-32addr`), still 26,100 functions, real imports (`cellFsClose`,
`sys_lwmutex_lock`, ...). `scripts/apply-ghidra-names.py` re-applied all 114
rows of this binary's `names.tsv` clean (0 skipped, 0 failed) and the program
was saved.

**`FUN_003d6dc8` no longer reads as `SpeedBar`/HUD widget setup.** That
reading is not being directly diffed against a saved copy of the old
decompile, but it depended entirely on TOC-relative loads this page already
established were resolving against the wrong TOC - so a completely different
shape on redecompile is consistent with the earlier reading being exactly
the artifact `memory.md` warns about, not a second data point confirming it.

**What the new decompile actually shows**: no `SpeedBar` shape at all.

**`iStack_24f4` is not a missing parameter - first hypothesis here, and
wrong.** The initial read of this pass guessed it was a second, stack-passed
argument Ghidra's auto-analysis missed. `analyze_dataflow` (backward, from
one of its uses) settles it instead: the value traces straight back to
**this function's own entry value of `r2`** (`"kind": "input/parameter"`,
terminating at function input) - which is exactly this function's own real
TOC, `0x008bd3c4`, already established above. The raw disassembly explains
why Ghidra couldn't fold it to a constant the way it does for the function's
*other* TOC-relative loads (e.g. `lwz r14,-0x5a84(r2)` right at entry,
decompiling cleanly to `iRam008b7940`): between entry and the first
`iStack_24f4`-relative read, the function makes several calls through
function pointers loaded from unknown runtime objects (`lwz r2,0x4(r9)`,
`lwz r2,0x4(r11)`, ...) - the standard ELFv1 convention for calling through a
pointer of unknown provenance, each one saving the caller's `r2` to
`0x28(r1)` first and restoring it after. The restore is provably faithful
(same stack slot, immediately after each call), but Ghidra's constant
propagation can't see through an indirect call to know that, so it gives up
and calls the result an opaque input rather than folding it back to
`0x008bd3c4`. So **every `iStack_24f4 + -0x58xx` read in this decompile is a
real TOC-relative global read against this function's own TOC** - resolvable
by hand exactly like the two loads earlier on this page, just with an extra
"is the round-trip faithful" step first.

**Resolved one, and it's decisive**: `iStack_24f4 + -0x58c8` is
`0x008bd3c4 - 0x58c8 = 0x008b7afc`. `inspect_memory_content` at the *base*
of that read family (`iStack_24f4 + -0x58d0 = 0x008b7af4`) finds a table of
big-endian pointers, mostly 0x20 (32) bytes apart with one 0x10 gap; the
third entry, at `0x008b7afc` (exactly the address the first offset resolves
to), holds `0x007b26c8`. Reading *that* address is the payoff: the string
**`Data/Tex/DetonatorMode0.gtf`** - HD's own texture-set naming this whole
thread has been chasing - immediately followed in memory by a shorter
placeholder string, `unlabeled`, at the 0x10-gap slot. So this is a table of
per-stage texture filenames (`Data/Tex/{Zone,Detonator}Mode{,DLC3}{N}.gtf`,
one entry short-circuiting to a generic placeholder), read one at a time and
handed off:

```
uVar14 = _opd_FUN_005d91a8(pvVar13);                       // per-call state
uVar10 = *(undefined4 *)(iStack_24f4 + -0x58c4);            // constant across the loop
uVar11 = *(undefined4 *)(iStack_24f4 + -0x58c0);            // constant across the loop
_opd_FUN_005dde98(&local_241c, *(undefined4 *)(iStack_24f4 + -0x58c8), uVar10, uVar14, uVar11);
```

`_opd_FUN_005dde98` (`0x005dde98`)'s own decompile confirms the shape: `param_2` is read
with `strlen`, copied into a small-string-optimised local buffer (inline for
<=15 bytes, heap-allocated past that), and handed to
`_opd_FUN_005da868(param_1, param_4, name_copy)`; only `if (*param_1 == 0)`
(cache miss) does it go on to allocate a new refcounted handle
(`_opd_FUN_005de2d0`) and store it into `*param_1`. **A get-or-create,
refcounted, named resource cache** - consistent with "load this texture by
filename, or reuse the cached one" and nothing narrower has been checked
(what `_opd_FUN_005de2d0` does with the name is unread, so "texture"
specifically versus "named resource in general" isn't nailed down).
`_opd_FUN_005d91a8` is trivial: `return *(undefined4 *)PTR_PTR_008bf25c;`, a
plain getter for an unidentified global (a loader/allocator context,
plausibly) - not itself named, too little evidence for what that global is.

**Named, confidence-scored, and in `names.tsv`**:
`FUN_003d6dc8` -> **`Environment_LoadStageTextures`** (72) - a per-stage,
named-texture-cache loader, matching the fifteen-entry table shape and the
one resolved filename directly. `FUN_005dde98` -> **`Resource_GetOrCreateByName_q`**
(65, below 70 so the `_q` stays) - the get-or-create cache mechanism itself,
kept generic ("Resource" not "Texture") because what it caches was not
independently confirmed past the one filename string. Both renames applied
live through the bridge and saved; the rest of this page keeps calling them
`FUN_003d6dc8`/`FUN_005dde98` where it is quoting what was known at the time
each earlier section was written.

**A second finding worth flagging, not confirmed**: the block *before* the
per-stage texture loop - the fifteen 0x250-byte `memcpy`s into stack buffers,
gated by a different flag (`*(char *)(*(int *)(iVar12 + 0x1bc) + 0x1b5) !=
'\0'`), then copied back reversed - is the same shape as 2048's own
`Environment_LoadEffectSettingsFiles` reversing a table in place under a
flag "plausibly meaning reversed circuit, unconfirmed"
(`docs/ghidra/functions/vita-2048-eu-v104/zone-environment-fallback.md`).
Finding the same *shape* independently on HD's own executable is real
corroboration that a fifteen-stage table gets reversed under *some*
condition on both titles - it does not confirm what the condition means on
either one; that stays open on both pages.

**Settled: it is a texture, not a generic resource.** `_opd_FUN_005de2d0`
(`0x005de2d0`), the function `Resource_GetOrCreateByName` calls on a cache
miss, carries two unambiguous format strings read directly from memory:
`"ERROR: Can't load placeholder texture %s\n"` (`0x007cdb90`) and
`"WARNING: Can't load %s, using default texture\n"` (`0x007cdbc0`, followed
in memory by a stray `.gtf` constant). Its own control flow matches a
texture loader with fallback exactly: a probe call
(`_opd_FUN_005d9610`, unread) decides found-versus-not; not-found either
errors (if the *placeholder* itself failed to load) or warns and recursively
calls `Resource_GetOrCreateByName` with the placeholder's own name instead;
found falls through to construct the real resource via a chain of indirect
(vtable) calls ending in GCM-texture-shaped bit-packed flags
(`0x2000000 | 0x1e80`-style constants, the same family `oag_formats::gtf`
(`docs/formats/gtf.md`) already decodes from this title's own `.gtf` files)
and a refcount increment. **Renamed, in
`names.tsv`**: `Texture_LoadWithFallback` (80) - and
`Resource_GetOrCreateByName`'s own confidence moved from 65 to **72** now
that its one exercised call site is confirmed rather than merely plausible,
crossing the `_q` threshold.

**The found/not-found probe is read too.** `_opd_FUN_005d9610` (`0x005d9610`) -
**`FwFile_ResolveInSearchPaths`** (72), named on the same `Fw` prefix
`memory.md`'s framework-layer functions already use - is a generic
filename-in-search-path resolver, not itself Zone/effectSettings-specific:
it walks a search-directory list (`PTR_DAT_008bf270`, an array with
begin/end pointers at `+0x14`/`+0x18`, 0x1c-byte stride), builds each
candidate as `directory + name` (optionally `+ extension`, tried against a
caller-supplied extension list, `param_4`), and calls a virtual "does this
exist" check (`(**(puVar1 + 0xc))(candidate)`) per candidate - the first hit
copies the resolved full path into the caller's output buffer and returns.
No literal string corroborates this one; the 72 rests on the loop/`strcpy`/
`strcat`/`strstr`/vtable-call shape being unambiguous on its own, the same
standard this page's other confident reads meet.

**`iVar12` is `g_GameState`, confirmed rather than guessed.** `iVar12 = *(int
*)(iStack_24f4 + -0x58d0)` resolves, by the same TOC math as everywhere else
on this page, to the value stored at `0x008b7af4`: `0x00936fe8` -
`g_GameState`'s own address, already named at confidence 90
([mode-manager.md](mode-manager.md)). Independently corroborated by the very
next condition in the decompile, `*(int *)(iVar12 + 0xe0) == 0xe`: `+0xe0` is
exactly `g_GameState`'s documented mode field, and `mode-manager.md` already
lists `14` (`0xe`) among the seven ids with "no `ModeManager`", hypothesising
"Zone, Zone Battle and Detonator are the obvious candidates" for that bucket
- unestablished there, and this page's own finding (a Zone/Detonator
texture-table branch keyed on mode `14` specifically) is a second, independent
data point for the same hypothesis, on two different functions.

**Why the reversal flag's write site is not chased further this pass**:
`get_xrefs_to` on `g_GameState` (`0x00936fe8`) returns over 100 hits across
dozens of functions - the same "the base pointer has dozens of direct
readers" shape 2048's own `+0x634` write-site search hit three times over on
`zone-environment-fallback.md`, and that page's own conclusion applies here
unchanged: a blind sweep of a hundred-plus call sites is not a good use of a
pass, and the next attempt should identify `g_GameState + 0x1bc`'s pointee
struct type first (narrowing which readers/writers are plausible owners of
its own `+0x1b5`), or use a runtime watchpoint if one becomes available.
What *is* now settled, that was not before: the flag lives inside
`g_GameState` specifically, not an arbitrary unrelated struct - a concrete
starting point for that future pass rather than an untyped offset pair.

**Still open**: the "reversed circuit" cross-title corroboration is still
just a shape match - tracing what sets the reversal flag at
`g_GameState + 0x1bc, +0x1b5` would test that hypothesis against a second,
independent binary rather than leave it resting on 2048's side alone; and
which of Zone/Zone Battle/Detonator mode id `14` actually is remains
unestablished on both pages.

## Tooling note: `program` parameter is unreliable across the two PS3/Vita programs

Passing `program: "EBOOT.elf"` explicitly to `search_strings` and
`search_functions_enhanced` silently returned results from the *other* open
program (`eboot.elf`, 2048's Vita executable) despite `switch_program` having
already made `EBOOT.elf` current and `get_current_program_info` confirming
it. Omitting `program` entirely (relying on the already-switched "current"
program) gave correct results both times. Verified directly:
`search_strings(program: "EBOOT.elf", search_term: "effectsettings")`
returned 2048's six strings; the identical call with `program` omitted
returned HD's own four. Whoever continues this thread on either binary:
**omit `program` and verify with `get_current_program_info` first**, rather
than trusting the parameter.

**A second, unrelated tool defect found 2026-08-29**: `search_functions_enhanced`
with a `name_pattern` (`"Environment"`, `"LoadStageTextures"`, tried both
plain and `regex: true`) returned zero results even for `Environment_LoadStageTextures`,
a function confirmed present and correctly named
(`get_current_program_info`/`search_functions` both agree). The plain
`search_functions` tool (not `_enhanced`) found it immediately with the
identical pattern. Use `search_functions`, not `search_functions_enhanced`,
for a name search in this program.

## 2026-08-28, a fifth pass: the live project had reverted, and once restored, the file's own content turns out to be parsed after all

**Found before any decompile could be trusted again**: the live `EBOOT.elf` program had silently lost both the TOC fix and every rename this page documents - `get_current_program_info` reported the plain `PowerPC:BE:64:A2ALT-32addr` language, not `-32addr-PS3`, and `0x003d6dc8`/`0x005de2d0` read back as `.opd.FUN_003d6dc8`/`.opd.FUN_005de2d0`, not `Environment_LoadStageTextures`/`Texture_LoadWithFallback`. Whether this was a Ghidra restart that reopened an older save, or the fix session's own `save_program` never landing, was not tracked down - not worth chasing once the fix itself needed redoing anyway. The maintainer rebuilt and reinstalled the Ps3GhidraScripts extension (`just build-ps3-scripts`), reimported under `--ps3-cspec` (fresh creation timestamp confirms it, language now correctly `-PS3`), and `scripts/apply-ghidra-names.py docs/ghidra/functions/ps3-hdfury-eu/names.tsv --program /ps3-hdfury-eu/EBOOT.elf` reapplied all 118 rows clean (0 skipped, 0 failed) and saved. `just check-names` passes project-wide (1182 rows, 7 binaries) afterward. Recorded here as a trap for the next session: **a live Ghidra project's applied state is not assumed durable between sessions any more** - verify a known rename and the language string before trusting a decompile, the same way `program` itself already needed verifying per the tooling note below.

**With the state trustworthy again, `Environment_LoadStageTextures`'s full decompile settles the open question this page's own "Does not settle" bullet raised**: does HD's executable parse `ZoneMode.effectSettings`'s own key/value text, or only use its path as an opaque cache token? The first read of this pass, from the call shape alone (`param_1` handed to two calls with no obvious file I/O), guessed the latter - **wrong**, corrected by tracing both calls to their real bodies:

1. `FwLoader`'s own path stash (`0x005d1e50`, trivial `strcpy` into the loader context at `+0x10` - too small and generic to name past this) writes `param_1` (the effectSettings path) into a state struct at `iVar8 + 0x3660`.
2. `FUN_00679ec8`/`FUN_00679ed8` are bare cross-TOC trampolines (`ZEXT48(&TOC_BASE); bl <real target>` - the same shape this project's other cross-module thunks take) to **`FwFile_OpenByPath`** (`0x0031e820`, confidence 75) and **`FwFile_ReadChunked`** (`0x0031d570`, confidence 78):
   - `FwFile_OpenByPath` splits the path on `:` (`0x3a`), walks a linked list of mount points at a fixed global (`PTR_DAT_008b5170 + 0x20`) calling each one's virtual "does this exist" method, and allocates a `0x114`-byte handle on the first hit - a generic VFS resolver, not effectSettings-specific, structurally the open-side sibling of the already-named `FwFile_ResolveInSearchPaths`.
   - `FwFile_ReadChunked` loops a virtual `Read(handle, buffer, size)` call in `0x20000`-byte chunks until the requested byte count is met, EOF, or an error - a generic buffered-read wrapper.
3. Back in `Environment_LoadStageTextures`: after the open succeeds, it seeks to the end and back (two virtual calls with mode `2` then `1` then `0`) to get the file's size, allocates a buffer of exactly that size (`_opd_FUN_003927a8`), reads the whole file into it with `FwFile_ReadChunked`, closes the handle, and hands the buffer and its size to **`FwKeyedText_ParseBuffer`** (`0x005d3378`, confidence 82) - then frees the buffer.
4. `FwKeyedText_ParseBuffer` is a two-line loop: call **`FwKeyedText_ParseEntry`** (`0x005d2108`, confidence 80) repeatedly, each call returning the buffer offset to resume from, until the offset reaches the buffer's length. `FwKeyedText_ParseEntry`'s own decompile is unambiguous: skip whitespace, read a `"`-quoted key into a stack buffer, look it up by name (with the same small-string-optimisation shape `Resource_GetOrCreateByName`'s own table uses - inline storage up to 16 bytes, a pointer past that) against a **schema table** at `*(param_1 + 4)` (36 bytes per entry: a type-tag byte, a length, a destination pointer), skip more whitespace, then dispatch on the matched entry's type tag - `9` reads a quoted string value with `strcpy` into the destination, `1` is a no-op (skip), values `2`-`8` go through a jump table at `PTR_DAT_008bf208` (not resolved this pass - almost certainly the per-numeric-type readers: float, vec3, vec4, colour-byte-quad, the same type zoo `oag_formats::envsettings`/`effectsettings` already infers from the *shape* of the values each key carries). An unrecognised key (lookup returns `-1`) is skipped whole, silently.
5. **This is a shared, effectSettings-agnostic parser, not bespoke to Zone**: `get_xrefs_to(0x005d3378)` returns `Environment_LoadStageTextures` itself plus four other named-`FUN_*` loaders not yet identified. **Corrected 2026-08-29**: an earlier pass over-read the sixth hit, a **data** reference at `0x008a0c68`, as a separate "virtual parse callback" registration; `search_byte_patterns` for `005d3378` finds exactly one hit, `0x008a0c68` itself - it is this function's own OPD entry (`{func, toc}` descriptor, the same shape every function in this binary has, per `memory.md`'s own worked example), not a second, independent reference. Five genuine callers, not six-including-a-registration, is still real corroboration that this is the engine's general keyed-config-text reader - the same role `oag_formats::envsettings`'s own doc already infers `.envsettings` and `.effectSettings` share a tokeniser by design, arrived at independently from the disc's own file shapes rather than the executable - but the "also registered as a callback somewhere" claim itself does not hold and should not be repeated. **A general trap this leaves for the whole page and its siblings**: any `get_xrefs_to` result that includes a bare `[DATA]` hit landing inside the OPD range (`0x00870520`-`0x008a54d8`) should be checked with `search_byte_patterns` for the target function's own address before being read as a "also referenced as a function pointer" finding - it may simply be that function's own descriptor.
6. **The destination settles cleanly**: `FwKeyedText_ParseBuffer`'s caller passes `iVar8 + 0x3660` as the schema-lookup context, and the schema's own destination pointers write into a table starting at `iVar8 + 0x1000` - exactly the base address of the **fifteen 0x250-byte-stride blocks** this page's TOC-defect section already found being `memcpy`-reversed under the `g_GameState + 0x1bc + 0x1b5` flag. `0x250 * 15 = 0x22b0`, and the reversal block's own final `memcpy` copies exactly `0x22b0` bytes - the same arithmetic checks out three ways now (stage count, stride, and total size) that did not fully connect last pass.

**So, corrected from this page's own earlier hedge**: HD's executable does read `ZoneMode.effectSettings`'s key/value pairs as text and does write them into a real per-stage struct table, the same shape and stage count (15, `0x250` bytes each) 2048's own `DAT_816c4890` (356 bytes each, a different authoring pass) already established as *a* per-stage struct on that title. **What is still not found, and is now the single well-localised next step**: who *reads* `iVar8 + 0x1000` afterward. This page's separate texture-loading finding (the `Resource_GetOrCreateByName` block a few lines later in the same function, gated by a *different* flag and mode check) loads per-stage **textures** into unrelated slots (`iVar8 + 0x33b8` onward) from a **hardcoded filename table**, not from anything parsed out of this file - the two per-stage subsystems sit side by side in the same function but do not feed each other. Nothing across either subsystem resembles 2048's `Zone_UpdateStage` (a clamped per-craft index driving a cross-fade) - no per-frame reader of `iVar8 + 0x1000` was found this pass, and none was looked for past this function's own body.

**Why this does not yet license wiring HD's `Lighting.*` keys into this project's renderer**: `oag_game`'s `envsettings_light`/`envsettings_fog` already read `Lighting.Sun direction`/`Lighting.Sun color`/`Lighting.Constant ambient color`/`Fog.Fog Color`/`Fog.Fog Density` off a circuit's `.envsettings`. `zonemode.effectsettings`'s own stage 0 (`just psarc cat ... /data/environments/zonemode.effectsettings`, checked directly against the disc this pass) carries a *differently spelled* vocabulary under the same-looking group name - `"0 Start.Lighting.Sun colour"` (British spelling, capital first letter, no `direction` key at all), `"0 Start.Lighting.Constant Ambient Colour"`, `"0 Start.Lighting.Fog colour"`/`"...Fog density"` - close enough to look like the same field, not close enough to reuse the same string constant, and missing the one field (`Sun direction`) a full light rig needs. Building an override off a name-alone match, on a file whose *consumer* is still unfound, is exactly the "plausible-looking stand-in" `CLAUDE.md` warns against. The schema table at `iVar8 + 0x3664` (found this pass, not read) is where the authoritative field list actually lives - reading it is the way to settle the vocabulary question with evidence rather than a spelling guess.

## 2026-08-29: the schema table is read, and it is the full recognised vocabulary, not a guess

**Answers this page's own "single well-localised next step" and closes
`effectsettings.md`'s spelling-guess hedge with hard evidence.** With the
project state re-verified trustworthy (`get_current_program_info` still
reports `-32addr-PS3`, `Environment_LoadStageTextures` still decompiles to
the shape this page already documents), `iVar8` (`Environment_LoadStageTextures`'s
own base pointer, `= iRam008b7940`) resolves from the binary's own `.data`,
not `.bss`: reading `0x008b7940` directly returns `0x00c7dfb0`, a fixed
constant baked into the ELF at link time - so `iVar8` is a compile-time
address, not a value only a live process would have. That made the parse
context `iVar8 + 0x3660 = 0x00c81610` (in `.bss`, unreadable statically, as
expected) traceable no further by address alone - but `FwKeyedText_ParseEntry`'s
own schema-lookup loop, re-read with this in hand, gives the schema's *start*
and *end* as `*(param_1+4)`/`*(param_1+8)` with the loop count computed as
`(end-start)/36` (Ghidra's `>>2` then `* 0x38e38e39` is the classic
divide-by-36-via-multiplication idiom) - confirming the 36-byte stride this
page already inferred, but not yet where those two pointers themselves come
from at runtime.

**Found by search, not by tracing the write**: `get_xrefs_to` on
`0x007b2458` (`"%s.Lighting.Sun colour"`, found via `search_strings` on
`"Sun"` - the exact string `effectsettings.md` needed to settle its own
spelling question) returns exactly one hit, a **data** reference from
`0x008b7a9c`. That is a 4-byte pointer slot holding this string's address -
and reading outward from it, both directions, finds it sitting inside a long,
uninterrupted run of `const char*` values, each pointing to the *next*
string in the same `.rodata` pool, in strict address order. Walking that run
back to its own start (the last non-pointer word before it, a float table
this same function's default-initialisation code writes into `puVar19[..]`)
gives the array's base: **`0x008b79cc`**. Walking forward, the pointers stay
well-formed for exactly 73 entries before the shape changes into the
already-documented per-stage texture-filename table (interleaved
destination/flag words, not a flat pointer run) - the first non-schema
entry, `Data/Tex/DetonatorMode0.gtf`, is the texture table's own first
filename, already named on this page.

**This is not the 36-byte record table `FwKeyedText_ParseEntry` walks -
it is very likely the compile-time source array some (unfound) initialiser
builds that table from, in the same order.** No writer of `*(param_1+4)`/
`*(param_1+8)` was traced this pass (`.bss`, so nothing to read statically);
what is established is structural, not a write-site trace, but the
boundaries are measured, not reconciled after an off-by-one guess:

- The word immediately before the array's own base is not itself a pointer -
  `0x008b79c8` holds `0x43200000` (the float `40.0f`), the tail end of this
  same function's default-value block (`puVar19[..]` in the decompile
  above). `0x008b79cc` is confirmed as the array's *first* word by what
  comes before it changing shape, not by counting forward from a guess.
- Index 72 (the last of 73) sits at `0x008b79cc + 72*4 = 0x008b7aec`,
  holding `0x007b26a8` - `"%s.Detonator Bomb Outer Colour"`, the vocabulary's
  own last entry (below).
- The very next word, `0x008b7af0`, holds `0x00c7efb0` - **exactly
  `iVar8 + 0x1000`**, the per-stage destination base this section's own
  point 6 above already identified. The destination table's own base
  address sits in memory immediately after the key-name array ends, which
  is itself corroboration that the two are a matched pair rather than
  coincidentally adjacent.
- `0x007b26c8` (`Data/Tex/DetonatorMode0.gtf`) - the per-stage texture
  table's own first filename, already named on this page - appears two
  words further on, at `0x008b7af8`, inside that table, not this array.

So 73 is a measured length (float boundary in, destination-base boundary
out), not a count that happened to land on a round number. Separately, this
project's own binary is already known to store a keyed-config vocabulary
this exact way: `g_ShaderParameterNames` (`0x008b7f08`, confidence 95,
[renderer.md](renderer.md)) is a structurally identical flat `char*` array
about 1 KB further into the same data blob, for HD's 81 engine shader
parameters - this is a pattern the binary uses more than once, not a
one-off reading. **Named**: `g_EffectSettingsSchemaKeyNames` (data,
confidence 82 - short of the function-level confidences on this page
because the array's *consumer* is inferred from position, content, and this
sibling-array precedent, not from a traced write into the runtime schema).

### The full 73-entry vocabulary, in order

```
 0  ZoneMode
 1  Override game control
 2  Target zone level
 3  Transition start speed
 4  Transition acceleration
 5  Texture U scale
 6  Texture V scale
 7  0 Start
 8  1 Sub Venom
 9  2 Venom
10  3 Sub Flash
11  4 Flash
12  5 Sub Rapier
13  6 Rapier
14  7 Sub Phantom
15  8 Phantom
16  9 Super Phantom
17  10 Zen
18  11 Super Zen
19  12 Subsonic
20  13 Mach 1
21  14 Supersonic
22  %s.Scene.Texture Colour
23  %s.Scene.Near Colour
24  %s.Scene.Base Colour
25  %s.Scene.Base Colour Middle
26  %s.Scene.Base Colour Highlight
27  %s.Scene.Base Colour Highlight Middle
28  %s.Scene.Aniso Power
29  %s.Scene.Aniso Curve
30  %s.Scene.Luminance Power
31  %s.Scene.Gradient1.Colour0
32  %s.Scene.Gradient1.Colour1
33  %s.Scene.Gradient1.Colour2
34  %s.Scene.Gradient2.Colour0
35  %s.Scene.Gradient2.Colour1
36  %s.Scene.Gradient2.Colour2
37  %s.Scene.Gradient3.Colour0
38  %s.Scene.Gradient3.Colour1
39  %s.Scene.Gradient3.Colour2
40  %s.Scene.EQ brightness
41  %s.Track.Texture Colour
42  %s.Track.Near Colour
43  %s.Track.Base Colour
44  %s.Track.Base Colour Middle
45  %s.Track.Base Colour Highlight
46  %s.Track.Base Colour Highlight Middle
47  %s.Track.Aniso Power
48  %s.Track.Aniso Curve
49  %s.Track.Luminance Power
50  %s.Track.EQ brightness
51  %s.Lighting.Sky reflection colour
52  %s.Lighting.Sun colour
53  %s.Lighting.Constant Ambient Colour
54  %s.Lighting.Prelit Colour Scale
55  %s.Lighting.Prelit Colour Power
56  %s.Lighting.Fog colour
57  %s.Lighting.Fog density
58  %s.Lighting.Alt Fog colour
59  %s.Lighting.Alt Fog density
60  %s.Lighting.Track Fog colour
61  %s.Lighting.Track Fog density
62  %s.EQ colour tint
63  %s.EQ analogue colour tint
64  %s.Sky horizon colour
65  %s.Sky zenith colour
66  %s.Radial Bloom Intensity
67  %s.Aurora Colour
68  %s.Airbrake Colour
69  %s.Detonator Mine Colour
70  %s.Detonator Mine Electricity Colour
71  %s.Detonator Bomb Inner Colour
72  %s.Detonator Bomb Outer Colour
```

**Verified against every shipped file, not left as a positional inference**:
`just psarc cat ... | grep -o '"[^"]*"' | sort -u`, stage-prefix stripped, on
all four HD files (`zonemode`/`zonemodedlc3`/`detonatormode`/
`detonatormodedlc3.effectsettings`). Exactly **8 of the 73 entries never
appear in any of the four**: the five title-wide keys `ZoneMode`, `Override
game control`, `Target zone level`, `Transition start speed`, `Transition
acceleration` (0-4), and three `%s.`-prefixed ones - `Scene.Luminance Power`
(30), `Track.Luminance Power` (49), `Radial Bloom Intensity` (66). Every
other entry, including every `Gradient`/`Aniso`/`Near Colour`/Detonator-only
key, is exercised by `detonatormode.effectsettings` specifically (the larger
file); `zonemode.effectsettings` alone uses a smaller subset (no gradients,
no Aniso/Near Colour/Luminance Power, no Detonator/Aurora/Airbrake/Radial
Bloom keys) - consistent with `effectsettings.md`'s own file-side reading
of that file's groups.

Read directly with `read_memory`/`inspect_memory_content` against
`0x007b1f80` onward (the string pool the array's pointers resolve into), not
transcribed from any prior guess.

### What this settles

- **The "spelling guess" in `effectsettings.md` is now evidence, not a
  guess.** HD's own executable recognises `"%s.Lighting.Sun colour"` -
  British spelling, `%s`-prefixed - as a first-class schema entry, matching
  the real file's own `"0 Start.Lighting.Sun colour"` exactly once `%s` is
  substituted with entry 7's own string. `.envsettings`' American-spelled,
  unprefixed `"Sun color"` (`0x00785ec8`) is a **different table entirely**
  (`envsettings.md`'s own subject), not a second spelling this same schema
  also accepts - the two formats' vocabularies do not overlap by name at
  all, only by field intent.
- **The `%s` substitution value is the stage's own name string, not a
  generic index** - entries 7-21 are exactly the 15-stage ladder
  `effectsettings.md` already recovered from the shipped files
  (`"0 Start"` .. `"14 Supersonic"`), compiled into the executable
  verbatim as the fill-in values. That both the disc's own files and the
  executable's compiled-in schema agree on this exact ladder, independently,
  is a second corroboration of that ladder beyond the four files
  `effectsettings.md` already checked.
- **New key groups, not previously read anywhere in this project**: the
  `%s.Scene.*` (19 keys - texture/near/base colour family, two aniso
  parameters, luminance power, three RGB gradients) and `%s.Track.*` (the
  same ten-key subset of Scene's shape, no gradients) groups, plus a
  trailing group of eleven title-scoped-per-stage keys covering EQ tint,
  sky horizon/zenith colour, radial bloom, aurora, airbrake, and four
  Detonator-only mine/bomb colours. `effectsettings.md`'s own "Each stage's
  own key groups" section named a subset of these from reading the files
  directly; this is the first time the *executable's* own recognised list
  has been read, and it recognises more than what any one file's own
  contents exercise.
- **One schema serves both modes.** The Detonator-specific keys (69-72) sit
  in the same flat array as the Zone-only "%s.Radial Bloom Intensity"/etc -
  confirming `Environment_LoadStageTextures`'s parser is genuinely mode-
  agnostic, one compiled-in vocabulary recognising whichever subset a given
  `.effectSettings` file happens to use. That is a partial answer to this
  project's own open question of why `detonatormode.effectsettings` reuses
  Zone's 15-stage ladder verbatim: the loader never had a Detonator-specific
  schema to author a Detonator-specific ladder into in the first place.

### What this does not settle

- **The runtime `.bss` schema table `FwKeyedText_ParseEntry` actually walks
  (`*(param_1+4)`/`*(param_1+8)`) was not traced to this array.** The
  connection argued here is structural (measured boundaries, verified
  content, a sibling array in the same binary using the identical shape) and
  circumstantial (no live process to read `.bss` from), not a traced data
  flow from this array into that pointer pair. A future pass with a running
  RPCS3 instance, or a static trace of whichever initialiser populates
  `iVar8 + 0x3660`'s own construction, would close this properly.
- **Still not found: who reads `iVar8 + 0x1000`** (this page's own prior
  section already named this as the actual next step past the schema, and
  it still is). This pass answers *what the file can say*, not *what stage
  currently applies* or *what draws off it*.
- **Entries 7-21 are not this pass's discovery - `effectsettings.md`
  already reads the same fifteen names off the shipped files themselves -
  but finding them compiled in verbatim as the executable's own `%s`
  fill-in values is a second, independent source for the same ladder.**
  That corroborates, rather than merely echoes, `effectsettings.md`'s own
  "Open" section: HD's `speech_zone.bnk` names fourteen `MR_*` announcer
  cues in the same order at confidence 75 (traced end to end only on
  Pulse's own executable), and this pass adds the schema's own compiled-in
  copy of the names as a third data point, still on neither title's
  *selection* code. **What this does not license**: assuming HD's stage
  selection reuses Pulse's already-implemented elapsed-time zone counter
  (`crates/race/src/zone.rs`, confidence 84, recovered from Pulse's own
  executable) - that mechanism has not been checked against HD's
  executable at all, and `oag_title::ZoneAnnouncer`/`crates/game/src/audio/sfx/announcer.rs`
  (which does play milestone cues off it at runtime) is wired for Pulse's
  zone-number mechanism specifically, not confirmed to generalise. Nothing
  here narrows 2048's still-unfound writer of `Zone_UpdateStage`'s `+0x634`
  field either
  ([zone-environment-fallback.md](../vita-2048-eu-v104/zone-environment-fallback.md)) -
  that is 2048's own executable, and this pass never left HD's.

## 2026-08-29, later the same day: who reads `iVar8 + 0x1000` - two candidates found, one ruled out, one strong and unconfirmed

**Method**: `search_instructions` for the exact byte pattern `lwz Rd, -0x5a84(r2)` (the same TOC-relative load `Environment_LoadStageTextures` itself uses for `iVar8`) returns 29 hits across roughly 20 functions, clustered in the address range immediately around `Environment_LoadStageTextures` (`0x0009bb30`-`0x003dc5cc`). Per `memory.md`'s own established method, each candidate's OWN OPD entry was checked before trusting the hit (this range sits inside the two-TOC overlap, `0x32d5e0`-`0x758110`, so nothing here can be assumed by address alone). Two were checked and confirmed real (both `0x008bd3c4`, `Environment_LoadStageTextures`'s own TOC, not the byte-pattern trap this page already warns about):

### `FUN_003d0b98` - ruled out, it clears the table rather than reading it

Walks `iVar8 + 0x1000` through `iVar8 + 0x3060` - all 15 stage bases, exactly the same stride this page already established - but each one is immediately followed by `bl 0x0045dc38`, which is **`memset`**: `li r4,0x0` / `li r5,0x250` sit right before every one of the 15 calls, confirmed rather than assumed - `memset(stageBase, 0, 0x250)`, one full stage struct each. This is a clear-to-zero of the whole per-stage table, not a reader. Called from two tiny stub functions (`FUN_003d69c0`/`FUN_003d69d0`) sitting immediately before `Environment_LoadStageTextures` in the binary - plausibly an `Environment` construct/reset pair, not identified further this pass. A real, useful negative: whoever continues this thread does not need to re-check this function.

### `FUN_003da540` - the strongest candidate found so far, not confirmed to the renaming threshold

Unlike `FUN_003d0b98`, this one computes the stage offset from a **runtime value**, not an immediate. The relevant slice (line numbers from this pass's own decompile, TOC-verified the same way):

```c
param_2 = param_2 * 0x38;              // param_2: this function's own 2nd argument, stride 0x38 (56)
puVar10 = ppuVar28[-0x16a0];           // = *(int*)0x008b7944, see below
puVar11 = ppuVar28[-0x16a1];           // = *(int*)0x008b7940 = iVar8, see below
uVar55 = *(uint *)(puVar10 + param_2); // read this entity's own stage-shaped field, stride 0x38
uVar46 = (ulonglong)uVar55 - 1;        // previous stage, clamped to 0 by the following mask op
lVar47 = clamp(uVar46, 0) * 0x250;     // previous stage's byte offset into the per-stage table
lVar83 = (ulonglong)uVar55 * 0x250;    // current stage's byte offset
// then three 16-byte (RGBA-shaped) reads per stage at +0x1000, +0x1020, +0x1040 relative to
// each stage's own base, copied into a small output area at puVar11+0x34d0/+0x34f0/+0x3510
```

**`ppuVar28` is the saved TOC, resolved by hand rather than trusted from the decompiler - and it identifies both pointers.** A few lines earlier, `iVar45 = (int)puVar25[5]` (`puVar25` is `ulonglong *`, so index 5 is byte offset `0x28` - the same `r1+0x28` save slot this page's own TOC-defect section already documents every cross-module call using) then `iVar49 = *(int *)(iVar45 + -0x5a80)` - the identical `-0x5a80` displacement family `Environment_LoadStageTextures` itself uses at `0x003d6ddc`. `ppuVar28 = (undefined **)puVar25[5]` is that same saved value: this function's own TOC, `0x008bd3c4`. With a 4-byte pointer stride:

- `ppuVar28[-0x16a1]` → byte offset `-0x16a1*4 = -0x5a84` → `0x008bd3c4 - 0x5a84 = 0x008b7940` - **exactly `iVar8`'s own slot**, the identical TOC-relative load `Environment_LoadStageTextures` performs (`lwz r14,-0x5a84(r2)` at `0x003d6dd4`). So `puVar11` **is** `iVar8`, confirmed by address arithmetic, not assumed.
- `ppuVar28[-0x16a0]` → `-0x5a80` → `0x008b7944` - the neighbouring global `Environment_LoadStageTextures` itself calls `puVar19`/`puRam008b7944` and fills with roughly thirty default float fields (offsets `0x00`-`0x6c`) at the top of its own body. So **`puVar10` is that same global, not an unidentified array** - `uVar55` is read from `*(uint*)(0x008b7944 + n*0x38)`, i.e. entry `n` of whatever this default-value block turns out to be an array of.

**This is structurally the same shape as 2048's own `Zone_UpdateStage`** (a per-entity index selecting a stage, the *previous* stage computed alongside the current one for a cross-fade) - the first time this project has found that shape anywhere in HD's own executable, rather than only on 2048's. The `*0x250` stride confirms it is reading this exact table (the same arithmetic this page's TOC-defect section already checked three ways for the *loader* side); this is the first evidence of the *read* side using the identical stride.

**What is not established, and why this is not renamed this pass**:

- `param_2` (whatever the caller passes as the "which entity" index, before the `*0x38`) was traced one level up into `FUN_003aa888`'s own body. The value it derives from involves an absolute-value idiom on a dereferenced pointer, inside code that searches a small fixed list of eight-ish slots (`iVar44+0xe8` through `+0x104`) by matching an ID field at `+0x7a60` each - a shape that reads like camera/viewport/target selection, not an obvious "loop over craft 0..N". Whether `param_2` ends up being a craft index, a camera index, or something else entirely was not settled.
- The three RGBA fields copied (stage offsets `+0x00`/`+0x20`/`+0x40` relative to each stage's own `0x250`-byte block) plausibly correspond to the schema's first three `%s.Scene.*` keys (`Texture Colour`, `Near Colour`, `Base Colour`, indices 22-24 in [the vocabulary above](#the-full-73-entry-vocabulary-in-order)) by position, but the schema's own destination-pointer-to-struct-offset mapping was not read at that granularity, so this is a plausible correspondence, not a confirmed one.
- Where the blended output (`puVar11+0x34d0` onward) goes next is untraced - the call chain into this function is `FUN_0067f078` → `FUN_00757de0` → `FUN_003e26d0` → `FUN_003aa888` → `FUN_003da540`, four large functions deep, none of it decompiled past the slices needed to establish the chain itself. **`FUN_0067f078`'s own `get_xrefs_to` returns only a `[DATA]` hit at `0x00873b78`** - checked this time, not assumed: `read_memory` there finds it sitting inside a dense run of `{func, toc}` pairs at 8-byte intervals (`0x0067f078` paired with `0x008ad4d8`, immediately after `0x0067f030`'s own pair) - this is the OPD, i.e. the function's own descriptor - the same artifact this page's earlier correction, about `FwKeyedText_ParseBuffer`'s own `0x008a0c68` hit, documents. So this is **not** evidence of an indirect/virtual call site - it is simply that no direct `bl` caller of `FUN_0067f078` was found by static search, which could mean a genuinely indirect call (unresolved), a caller outside the range searched, or something this pass's method missed.

Per `CLAUDE.md`'s confidence floor, this stays unnamed (`FUN_003da540`) rather than renamed on a hedge - the mechanism (per-entity index, cross-fade math, matching stride, and now both source pointers identified by address) is real and worth recording, but "which entity" and "what consumes the output" are both still open, and a name would assert more than is established.

**Sharper next step, given `puVar10` is now a known global rather than an unknown array**: find who **writes** the stage value at `*(int*)0x008b7944 + n*0x38` (entry `n`'s own offset-0 field) - that is the actual stage-selection write site, a far more tractable target (one specific global, one specific field pattern) than tracing `FUN_003d9970` (the function `FUN_003da540` hands its read fields to, ~3 KB, four call sites) or resolving `FUN_003aa888`'s camera/viewport-selection code cold.

### 2026-08-29, a fourth pass: who writes `0x008b7944 + n*0x38` - one near-repeat of the OPD trap caught before it shipped, one real correction, one new lead

Picked up the previous pass's own sharpened next step: find the write site. Method
(per `search_instructions(mnemonic: lwz, operand_pattern: -0x5a80(r2))`) found 67 hits
across roughly 20 functions - every place in the binary that loads this global's own
TOC-relative slot at all, read or write.

**Confirmed first: `0x008b7944` is flat data, not a pointer.** `inspect_memory_content`
on both `0x008b7940` and `0x008b7944` shows the second is simply the first shifted four
bytes - the same byte run continues across the boundary. So `n*0x38` indexes the global
directly, the way every earlier pass in this file already assumed; there is no extra
indirection hiding the real array elsewhere.

> **Corrected 2026-08-29 (sixth pass): this is wrong, and it is the reason the
> candidate list below was never exhaustive.** `0x008b7944` is a **TOC slot holding
> a pointer**; `read_memory` there returns `0x008c2cb8`, the array's real address.
> The check quoted above proves nothing - reading any address and that address `+4`
> always shows "the same byte run shifted four bytes." Consequences: the array lives
> at `0x008c2cb8` in `.data` and has exactly **two** entries, and **five** different
> TOC slots hold its address, so `lwz -0x5a80(r2)` was only ever one fifth of the
> ways to reach it. See
> [the sixth pass](#2026-08-29-a-sixth-pass-the-array-is-a-pointer-away-the-candidate-list-was-one-fifth-of-the-real-one-and-two-offset-readings-were-off-by-a-folded-bias).

**A write site was found - `FUN_003cdc90` - but its caller could not be located, and
the first read of that fact was wrong.** Decompiling the fifteen small functions
clustered at `0x003cdbd0`-`0x003ce118` (all short `lwz -0x5a80(r2)` users from the
67-hit list) turned up one real store: `FUN_003cdc90` zero-initialises roughly thirty
`undefined4` fields of the global directly, including offset `0x00` of both entry `0`
and entry `1` (`puVar1[0] = 0` and `puVar1[0xe] = 0`, the exact stage-index field this
whole thread is chasing, for the two adjacent entries `FUN_003da540`'s cross-fade
reads) - a real, if narrow, write. Its `get_xrefs_to` returned one `[DATA]` hit at
`0x0088c198`, sitting inside a dense run of eight `{func, toc}`-shaped 8-byte pairs
starting with `FUN_003cdc90` itself, then `FUN_003cdd20`/`FUN_003cdd30` (two constant-
offset accessors into a *different* global) and five `0067ce*`/`0067cf*` functions
(destructor-family: `*param_1 = PTR_DAT_...` then a cleanup call, the standard
scalar-deleting-destructor shape used elsewhere in this binary). **First read: called
this a genuine 8-slot C++ vtable and `FUN_003cdc90` a real "Reset" virtual method.**
That read does not survive the check this page's own earlier correction (the
`FwKeyedText_ParseBuffer` one) exists to catch: `0x0088c198` sits inside the documented
OPD range (`0x00870520`-`0x008a54d8`), and a follow-up `get_xrefs_to 0x0088c198`
itself returns **no references at all** - nothing loads that table's address the way a
real vtable install does (`*param_1 = PTR_DAT_...`). Adjacent OPD descriptors sitting
next to each other in memory is simply what the OPD segment looks like; it is not by
itself evidence of a dispatch table. **Corrected**: `FUN_003cdc90` is a real write to
the exact field pattern, but it has **no located caller** - record it as "a write site
with no confirmed call site," not as a virtual method. Whether it runs once at load
(matching 2048's own `Zone_InitStageState`) or not at all in the traced path is open.

**`FUN_003da540` and `FUN_003d0b98` are both far larger than either pass's own slice
implied - measured this time, not assumed.** `get_function_by_address` gives real
bodies: `FUN_003da540` spans `0x003da540`-`0x003de1ff` (~15.5 KB), `FUN_003d0b98`
spans `0x003d0b98`-`0x003d69bb` (~24 KB) - both far past the excerpts either pass
actually read. Re-checked `FUN_003d0b98` for anything past the already-confirmed
memsets with two narrow nets scoped to the function alone (`stwx`, and the `rlwinm
0x6,0x0,0x19` half of the `*0x38` stride idiom `FUN_003da540` uses) - both come back
with zero hits, so the "just memsets, ruled out" finding stands, but on a function that
was never read in full; most of its 24 KB is still unsurveyed and doing something else
entirely.

**The new lead: `FUN_003da540` itself writes entry `n`'s offset `+4` field, not offset
`+0`.** Disassembling its opening block (rather than trusting the prior pass's
narrower decompiled slice) finds, at `0x003da674`: `stw r0,0x4(r9)`, where `r9` is
`entityBase(-0x5a80(r2)) + (r26-r27)` and `r0` comes from `lwz r0,0x640(r11)` -
`r11` itself an `lwzx` result from a small table indexed by `r31`. The write is gated:
`lbz r0,0xc(r9)` (a byte flag at entry-offset `0xc`) must be zero, or the whole block
from `0x003da620` to `0x003da674` is skipped. Offset `+4` is exactly the field
`FUN_003ce2c0` (below) switches on across fifteen values (`0`-`0xe`) to select one of
several downstream draw/state branches - so this is a real producer for that field,
found by tracing a store rather than guessed from the switch shape alone. **Still not
the stage index**: offset `+0` (the field that indexes the `0x250`-stride per-stage
table, confirmed by the previous pass) is not written anywhere in this block or
anywhere else this pass's nets covered - the actual stage-*selection* write remains
unfound.

**A second independent consumer of the cross-fade, found while chasing this: `FUN_003ce2c0`,
`0x003ce2c0`-`0x003d0b97` (~10.2 KB), sitting directly before the already-ruled-out
`FUN_003d0b98`.** Its opening block re-derives the identical blend `FUN_003da540`
computes - same `-1`/clamp-to-zero previous-stage arithmetic, same `*0x250` stride,
same `+0x1000`/`+0x1004` RGBA reads - independently, inline, for its own rendering use
rather than by calling the other function. Two independent call sites agreeing on the
exact same arithmetic is corroboration that this blend is a real, exercised mechanism,
not an artefact of one decompile. Immediately after its own blend, it reads entry `n`'s
`+4` field and remaps it through a chain of `if (iVar36 == k)` comparisons (`k` = `0`
through `0xe`, mapped to a small output set `{0,1,3,4,5,7,8}`) into a **fixed**, unindexed
field at `entityBase+0x74` - and `+0x74`'s value then selects which of nine large
draw/audio-parameter branches (`iVar20 == 0` through `8`) runs later in the same
function. **Two producers for the same `+0x74` field, not reconciled**: `FUN_003cddc0`
(found the previous pass, unindexed random write avoiding a repeat) and this switch
write disagree in mechanism - random-non-repeat versus deterministic-from-`+4` - and
nothing in this pass established which one runs when, or whether both do on different
paths.

> **Corrected 2026-08-29 (sixth pass): there is nothing to reconcile - `+0x74` is not
> part of this struct.** The array is two entries of `0x38`, so it ends at `+0x70`;
> `arrayBase + 0x74` is `0x008c2d2c`, a word in the *adjacent* structure (it holds
> `1` in `.data`, immediately before a sixteen-float table). Both "producers" write
> the same unrelated slot, and `FUN_003cddc0`'s `rand() & 7` reads naturally as a
> random-variant selector for whatever that structure is. Neither is a per-entity
> field of the effectSettings entity struct.

**Net effect on the open question**: the stage-selection write (`entry n`'s offset
`+0`) is still not found. What is now real and documented: the global is flat data,
one narrow write to offset `+0` exists but has no located caller, and the previously-
unread bulk of both giant neighbouring functions turned up a real (if different)
write - to offset `+4`, a distinct "event" field - that both closes part of the
`FUN_003ce2c0`/`+0x74` shape from the previous pass and opens a new one (two producers
for `+0x74`). **Next concrete step**: `FUN_003da540`'s `+4` write is gated by entry
`+0xc`'s own flag byte - trace who sets *that* flag, since it is the thing that decides
whether a fresh event value lands at all, and it is a single byte at a known offset
rather than a cold sweep. Separately, `stwx`/the shift-idiom nets used here were scoped
only to the specific functions already on this page's candidate list; a program-wide
`stwx` search was attempted and returned 500 results capped by the tool's own default
limit before reaching any of this page's address range (all candidates sit past
`0x0009bb30`, sorted-by-address results this shallow never got there) - re-running it
with a higher limit, or address-range-scoped, is unstarted.

**Not wired into Rust**: per `CLAUDE.md`'s rule against firing an effect on an
unrecovered trigger, none of this pass's findings cross the bar - the actual
stage-selection write is still unfound, and even the `+4`/`+0x74` mechanism found this
pass has two disagreeing producers rather than one settled path. `oag_formats::effectsettings`
already parses the file; nothing new here changes what is safe to wire into
`oag_render`/`oag_gameplay`.

### 2026-08-29, a fifth pass: the `+0xc` flag - answered from data already in hand, and a course-correction on search scope

Picked up this page's own next step: trace who sets the flag byte `FUN_003da540`'s
`+4` write is gated on. **A mis-scoped detour first**: ran `stb` with no operand filter
across `FUN_003d0b98` (166 hits) and `FUN_0009bb30` (117 hits) hoping to spot a store to
offset `0xc` of our entity struct - the same "drowns you" mistake the previous pass's
own advisor call had already flagged for `stw`. Both results are dominated by repeated
`stb r0/r8/r10/r11,0x0..0xf(rX)` runs against registers never confirmed to derive from
`-0x5a80(r2)` - almost certainly a generic small-struct clear idiom this binary reuses
in many unrelated places, not evidence about this specific field. Recorded as a trap
for next time: `search_instructions` needs the field's own displacement in
`operand_pattern` (`"0xc("` at minimum), never a bare mnemonic, on a function already
known to be tens of KB.

**The real answer was already sitting in this same page's own earlier decompile.**
`FUN_003cdc90` (the write-site-with-no-located-caller from the fourth pass) writes
`puVar1[3] = 0x3f000000` and `puVar1[0x11] = 0x3f000000` - dword index 3 is entry 0's
own byte offset `0xc` (`3*4`), dword index `0x11` (17) is entry 1's (`14 + 3`), the
identical entries `FUN_003da540`'s cross-fade reads. Big-endian: the top byte of
`0x3f000000` is `0x3f`, non-zero - so this single already-documented write, if it is
ever reached, leaves `+0xc` non-zero for both entries, which is exactly the state that
makes `FUN_003da540`'s own gate (`if (flag != 0) skip`) skip the block. **Two readings,
both consistent with the evidence, neither settled**: this is either a real "already
initialised, no fresh event pending" flag intentionally packed into a float field's
top byte, or a coincidental byte-level alias of a genuine `0.5f` field that happens to
read non-zero - PowerPC has no bitfield syntax in the decompile to tell the two apart,
and nothing this pass traced reads offset `0xc` as anything other than this one
byte-test. **Still unresolved**: `FUN_003cdc90` itself still has no located caller (per
the fourth pass), so even a confirmed flag write does not establish when, or whether,
it actually runs.

**A useful side-effect of re-reading this decompile carefully: three more of this
struct's fourteen dword fields are now placed, not just the two (`+0x00` stage,
`+0x04` from the prior pass) already known.** Dword index 6 (byte `0x18`) is exactly
the offset `FUN_003ce2c0` reads as the blend weight (`fVar2`); `FUN_003cdc90` seeds it
with `uRam008b7948` - a *different* global, the neighbouring default-value slot right
after the entity array's own base (`0x008b7944 + 4`), i.e. one of
`Environment_LoadStageTextures`'s own ~30 default floats, not a literal constant. Dword
index 13 (byte `0x34`, each entry's last field before the next entry starts) is seeded
`0x3f800000` (`1.0f`) for both entries. Layout now known for entry stride `0x38` (14
dwords): `+0x00` stage index, `+0x04` event code, `+0x0c` a flag byte aliased into a
`0.5f`-valued field, `+0x18` blend weight (seeded from a real default, not zero),
`+0x34` a `1.0f` field. The remaining nine dwords per entry are still unplaced.

> **Corrected 2026-08-29 (sixth pass): the gated field is `+0x2c`, not `+0x0c`, so
> this whole reading is void.** `FUN_003da540` computes `r9 = arrayBase + n*0x38 +
> 0x20` at `0x003da5f8` and *then* does `lbz r0,0xc(r9)` - a folded bias, so the
> byte tested is entry offset `0x20 + 0x0c = 0x2c`. `.data` holds `0` there, so the
> gate **passes** and the `+0x04` write does happen - the opposite of the conclusion
> below. `+0x0c` really is `0.5f`, but nothing reads its top byte.

**Net effect**: no new write to the stage index (`+0x00`) or a clean, unambiguous
`+0xc` flag write was found. What is new is real: a plausible (not confirmed) source
for `+0xc`'s own non-zero state, and five of fourteen per-entry fields now placed by
offset rather than four. **Not wired into Rust this pass either** - the layout is
useful documentation, not a trigger, and `FUN_003cdc90` having no located caller is
still the harder blocker than the flag's own meaning.

### 2026-08-29, a sixth pass: the array is a pointer away, the candidate list was one fifth of the real one, and two offset readings were off by a folded bias

Picked up the fifth pass's own next step (a caller for `FUN_003cdc90`, or the
`+0x00` write itself). The `+0x00` write is **still not found**, but the reason five
passes missed it is now understood, measured, and the search space is re-drawn rather
than merely re-swept. Program state verified trustworthy first, per this page's own
trap: `list_open_programs` reports language `PowerPC:BE:64:A2ALT-32addr-PS3`, 26,100
functions, and `0x003d6dc8` still reads back as `Environment_LoadStageTextures`.

#### `0x008b7944` is a pointer slot; the array is at `0x008c2cb8` and has two entries

`read_memory` at `0x008b7930` (80 bytes, the whole neighbourhood at once):

```
008b7940  00c7dfb0   <- iVar8, the 15 x 0x250 per-stage table's base (already known)
008b7944  008c2cb8   <- the entity array's base  ** a pointer, not the array **
008b7948  3dcccccd   (0.1f - the default FUN_003cdc90 seeds +0x18 from)
008b794c  007b1f60
008b7950  008623b8
008b7954  008c1430   <- see "a current-entity global", below
```

The fourth pass's "flat data, not a pointer" finding is **wrong** and is marked as
such inline above. It rested on observing that `inspect_memory_content` at
`0x008b7944` shows the same byte run as `0x008b7940` shifted four bytes - which is
true of *any* address and its `+4`, and proves nothing either way. The decisive
reading is the word's own value, and it is an address in `.data`.

Reading `0x008c2cb8` settles the array's extent too, because it is **initialised
data, not `.bss`** - so the compile-time contents are readable statically:

```
008c2cb8  00000000 00000000 00000000 3f000000    entry 0
008c2cc8  3f000000 3dcccccd 3dcccccd 00000000
008c2cd8  00000000 00000000 00000000 00000000
008c2ce8  00000000 3f800000
008c2cf0  00000000 00000000 00000000 3f000000    entry 1 (identical)
   ...
008c2d20  00000000 3f800000
008c2d28  ffffffff 00000001 43400000 00000000    <- NOT entry 2: different shape
008c2d38  43d00000 42800000 43e00000 42000000       (a 16-float table: 192, 0, 416,
   ...                                               64, 448, 32, 320, 128, 288, ...)
```

Two things fall out. First, **the array is exactly two entries** (`2 * 0x38 =
0x70`), which is independently corroborated by `FUN_003cdc90` zero-initialising
dwords `[0]` and `[0xe]` and nothing beyond - it clears the whole array, not "the
two adjacent entries the cross-fade happens to read." A third witness, this one on
the *code* side rather than the data side, agrees: `0x003da678` is
`cmpwi cr7,r31,0x1`, where `r31` is the incoming entity index (`or r31,r4,r4` at
`0x003da5fc`, taken before `r4` is clobbered with the array base one instruction
later). A bare compare of the index against `1` - no bound, no loop - is what a
two-element array looks like from a consumer. This matters beyond bookkeeping:
the `+0x74` correction below depends on the array ending at `0x70`, and were there
a third entry, `+0x74` would be entry 2's `+0x04` and the fourth pass's puzzle
would stand. Second, the `.data` image
**is** the value set `FUN_003cdc90` writes, field for field (`+0x0c`/`+0x10` =
`0.5f`, `+0x14`/`+0x18` = `0.1f`, `+0x34` = `1.0f`, everything else zero), so that
function is a reset-to-compile-time-defaults, and the state the game starts in is
readable off the ELF without finding its caller at all. **Stage index `+0x00`
starts at `0`, for both entries.**

Two entries also reframes what the index *is*: `FUN_003cdd40` computes it as
`*(int *)(*(int *)(ctx + 0x13e8) + 0x6284)` (with `ctx` from `FUN_00679e68`), and
`FUN_003cde60`'s real disassembly takes it from a global when the caller passes
`-1`. A 0-or-1 index reads as viewport/player, not as anything stage-related.

#### The closed candidate list was one fifth of the real one

Because `0x008b7944` is a pointer slot rather than the array, `lwz -0x5a80(r2)` is
not "every place in the binary that can reach this global" - it is every place that
reaches it *through that one TOC slot*. `search_byte_patterns` for the array's own
address, `008c2cb8`, finds **five** slots holding it:

| slot | displacement | TOC | reached by |
| --- | --- | --- | --- |
| `0x008b36c4` | `0x61ec(r2)` | `0x008ad4d8` | `FUN_002b61c8` |
| `0x008b7220` | `-0x61a4(r2)` | `0x008bd3c4` | `FUN_003aa888` (5 sites) |
| `0x008b7944` | `-0x5a80(r2)` | `0x008bd3c4` | the 67-hit list all five prior passes used |
| `0x008b7cc4` | `-0x5700(r2)` | `0x008bd3c4` | `FUN_003df360`, `FUN_003e29e0` |
| `0x008b82e0` | `-0x50e4(r2)` | `0x008bd3c4` | `FUN_003fc140`, `FUN_003ff860`, `FUN_00400a00`, `FUN_00403a30`, `FUN_004053e0`, `FUN_004074e0`, `FUN_00408fa8` |

**Every TOC in that column was read from the function's own OPD entry, not inferred
from address proximity to a verified sibling** - which is precisely the standard this
page's trap section demands and which kills two candidates further down. The eleven
OPD pairs, all `{func, 008bd3c4}` except where noted: `002b61c8`@`0x00882228`
(`008ad4d8`, the second TOC - see below), `003aa888`@`0x0088b9f0`,
`003df360`@`0x0088c468`, `003e29e0`@`0x0088c4c0`, `003fc140`@`0x0088ca88`,
`003ff860`@`0x0088ca98`, `00400a00`@`0x0088caa0`, `00403a30`@`0x0088cab8`,
`004053e0`@`0x0088cac0`, `004074e0`@`0x0088cac8`, `00408fa8`@`0x0088cad0`.

The `0x008b36c4` slot is the second TOC's own copy: `0x008ad4d8 + 0x61ec =
0x008b36c4`, and the word before it (`0x008b36c0`) holds `0x008c1430` - the same
pair, in the same order, that `0x008b7940`/`0x008b7944` form for the first TOC.
So this is the ordinary "one TOC subset per module" shape, not a coincidence of
values, and **a second module reaches the same array**.

Two searches were run to make "exhaustive" mean something this time rather than be
asserted again:

- `search_byte_patterns` for `008c2cf0` (entry 1's own address, in case some slot
  pointed mid-array) - **no matches**.
- `search_instructions` for operand `0x2cb8`, which would catch an absolute
  materialisation (`lis rX,0x8c` / `addi rX,rX,0x2cb8`) - the one addressing mode no
  `d(r2)` search can see. Thirteen hits, **none** of them this address: all are
  either struct-field displacements on non-`r2` bases (`stw r5,0x2cb8(r23)`), stack
  slots (`stfd f31,0x2cb8(r1)`), or `±0x2cb8(r2)` loads that resolve elsewhere under
  their own TOC.

So the candidate list is now genuinely closed *for this binary's addressing*, and it
is roughly twice the size the previous five passes worked from.

#### `FUN_0009bb30` - the brief's leading unexamined candidate - is the byte-pattern trap

Ruled out cleanly, by the method this page's own trap section prescribes rather than
by reading its 11 KB body. `search_byte_patterns` for `0009bb30` finds one OPD entry
at `0x008735f0`, holding `{0009bb30, 008ad4d8}` - its real TOC is **`0x008ad4d8`**,
not `0x008bd3c4`. Its single `lwz r4,-0x5a80(r2)` at `0x0009be0c` therefore resolves
to `0x008ad4d8 - 0x5a80 = 0x008a7a58`, an unrelated global. **`FUN_0009bb30` never
touches this array**; it was on the list only because `search_instructions` matches
encoded bytes, exactly the failure mode the "byte-pattern search across a TOC
boundary" section above warns about. A fifth pass spent a 117-hit `stb` sweep on it
for nothing.

`FUN_0007b388` (the sixth hit in the `-0x61a4` family) falls the same way: OPD at
`0x00872f00` gives TOC `0x008ad4d8`, so its `-0x61a4(r2)` is `0x008a7334`, not the
array. Two clean negatives.

#### The "empty stub" functions are vector getters, and they name a current-entity global

`FUN_003cde60`, `FUN_003ce000`, `FUN_003ce070`, `FUN_003ce0e0` all decompile to
`void { return; }` and all disassemble to the same real body (only the final offset
differs - `+0x40`, `+0x60`, ...):

```
003cde60: cmpwi   cr7,r3,-0x1        ; caller may pass -1 for "current"
003cde70: bne     cr7,0x003cde7c
003cde74: lwz     r9,-0x5a70(r2)     ; 0x008b7954 -> 0x008c1430
003cde78: lwz     r3,0x0(r9)         ; r3 = *(int*)0x008c1430   ** current index **
003cde7c: ... clamp r3 to >= 0 ...
003cde94: rlwinm  r11,r0,0x6,0x0,0x19  ; \  the n*0x38 idiom
003cde9c: rlwinm  r0,r0,0x3,0x0,0x1c   ; /
003cde98: lwz     r10,-0x5a80(r2)      ; r10 = arrayBase
003cdea0: lwz     r8,-0x5a84(r2)       ; r8  = per-stage table base (iVar8)
003cdeac: lwzx    r9,r10,r11           ; r9 = array[n].+0x00   ** the stage index **
003cdeb4: mulli   r9,r9,0x250          ; stage * 0x250
003cdeb8: addi    r9,r9,0x40
003cdec4: lvx     v2,r9,r11            ; r11 = 0x1000 here; load a 16-byte field
```

All read-only. Their value is threefold: a fourth independent confirmation that
`+0x00` is the `0x250`-table stage index; a **new global, `0x008c1430`**, holding the
current entity index (also reachable as `*(0x008b7954)` and, from the second TOC, as
`*(0x008b36c0)`); and the "these are almost certainly a decompiler artefact"
suspicion recorded in the brief is now confirmed fact for all four, not just
`FUN_003cde60`.

#### A folded index bias invalidates two offset claims - and the method behind them

This is the most transferable finding of the pass. This compiler habitually folds a
constant bias into the *index register* before adding the base, so a displacement in
the final load or store is **not** the struct offset. `FUN_003aa888` alone uses three
different biases (`+0x00`, `+0x10`, `+0x30`) within one function. Concretely, at
`0x003aec74`:

```
003aec74: rlwinm  r0,r27,0x3,0x0,0x1c   ; r0 = n*8
003aec78: lwz     r11,-0x61a4(r2)       ; r11 = arrayBase
003aec7c: rlwinm  r9,r27,0x6,0x0,0x19   ; r9 = n*64
003aec84: subf    r9,r0,r9              ; r9 = n*56 = n*0x38
003aec88: addi    r9,r9,0x30            ; ** bias folded in here **
003aec90: lfsx    f0,r11,r9             ; reads entry n's +0x30, not +0x00
003aec94: fmuls   f0,f0,f13
003aec98: stfsx   f0,r11,r9             ; array[n].+0x30 *= (float)*(r20+0x24)
```

Applying that check to the fourth pass's two headline offset claims, both taken from
`FUN_003da540`:

- **The gate is `+0x2c`, not `+0x0c`.** `0x003da5f0`: `subf r30,r27,r26`, and both
  halves of that subtraction are verified from the same source register rather than
  assumed - `0x003da580`: `rlwinm r26,r4,0x6,0x0,0x19` (`r4 << 6`) and `0x003da588`:
  `rlwinm r27,r4,0x3,0x0,0x1c` (`r4 << 3`), so `r30 = r4*64 - r4*8 = n*0x38`.
  Then `0x003da5f8`: `addi r9,r30,0x20`, `add r9,r9,r4` (`r4` = arrayBase), and only
  then `0x003da614`: `lbz r0,0xc(r9)`. Entry offset `0x20 + 0x0c = **0x2c**`.
  `.data` holds `0` at `+0x2c`, so the gate **passes** and the write below it
  happens - the exact opposite of the fifth pass's conclusion, which had the gate
  skipping on `0x3f000000`'s top byte. That "flag aliased into a `0.5f` field"
  reading is void; `+0x0c` is simply a `0.5f`, read by nobody found so far.
- **The `+0x04` write stands.** `0x003da658` recomputes `subf r9,r27,r26` and adds
  the base with **no** bias, so `0x003da674`'s `stw r0,0x4(r9)` really is entry
  offset `+0x04`. Verified rather than assumed, this time.

**The method, not just the two fixes**: every offset on this page derived from a bare
`d(rX)` displacement without tracing `rX`'s own provenance back through its `addi`
is suspect and should be re-derived before it is built on. That is how five passes
came to carry a wrong `+0x0c` and, below, a `+0x74` field that is not in the struct
at all.

#### `+0x74` is not a field of this struct

`FUN_003cddc0` decompiles to `*(uint *)(iRam008b7944 + 0x74) = rand() & 7` with a
retry loop avoiding the previous value - an unindexed, constant offset from the array
*base*. With the array measured at two entries (`0x70` bytes), `arrayBase + 0x74` is
`0x008c2d2c`, four bytes past its end, in the adjacent structure (it holds `1` in
`.data`, immediately before the sixteen-float table quoted above). `FUN_003ce2c0`'s
own `+0x74` write is the same slot. So the fourth pass's "two disagreeing producers
for `+0x74`, not reconciled" does not need reconciling - **it dissolves**: both write
a word that is not part of the entity struct, and `rand() & 7` reads as a
random-variant selector for whatever that neighbouring structure is. Marked inline
above.

#### Per-entry layout, re-derived and now complete by value

Combining the `.data` image with the bias-corrected access sites, all fourteen dwords
of the `0x38` stride are placed by *value*, and eight by *role*:

| offset | `.data` default | role | evidence |
| --- | --- | --- | --- |
| `+0x00` | `0` | **stage index** into the `0x250` table | `FUN_003cdbd0`-`FUN_003cdc60`, `FUN_003cde60`-`FUN_003ce0e0`, `FUN_003da540`, `FUN_003ce2c0` |
| `+0x04` | `0` | event code, switched `0`-`0xe` | written `0x003da674`; read by `FUN_003ce2c0` |
| `+0x08` | `0` | - | |
| `+0x0c` | `0.5f` | - (no reader found; **not** the gate) | `FUN_003cdc90` dword `[3]` |
| `+0x10` | `0.5f` | - | `FUN_003cdc90` dword `[4]` |
| `+0x14` | `0.1f` | - | `FUN_003cdc90` dword `[5]` |
| `+0x18` | `0.1f` | cross-fade blend weight | `FUN_003ce2c0`; `FUN_003aa888` at `0x003ae0d0` (bias `+0x10`, `lfs 0x8(r9)`) |
| `+0x1c` | `0` | index into a table at `0x008b721c` | `FUN_003aa888` at `0x003ab81c` (bias `+0x10`, `lwz 0xc(r11)`), then `<<2` and `lwzx` |
| `+0x20`..`+0x28` | `0` | - | |
| `+0x2c` | `0` | **gate flag byte** for the `+0x04` write | `0x003da614` (bias `+0x20`, `lbz 0xc(r9)`) |
| `+0x30` | `0` | float, set from a constant then scaled per frame | `0x003ada58` (store const), `0x003aec98` (multiply in place), `0x003df434` (read) |
| `+0x34` | `1.0f` | transition marker: `1.0f` on start, `0` elsewhere | `0x003aea18` (`lis r0,0x3f80`), `0x003ada7c` (`0`), `0x002b629x` (second module, same shape) |

The fifth pass had five of fourteen placed and called nine "unplaced"; this is
fourteen placed by value and a role for eight, all re-derived from disassembly with
the bias checked, not carried forward.

#### `FUN_002b61c8`: a second module runs the same "start a transition" sequence

TOC verified first (OPD at `0x00882228` holds `{002b61c8, 008ad4d8}`), so its
`lwz r7,0x61ec(r2)` genuinely is `0x008b36c4` -> the array. Its body at
`0x002b6230`-`0x002b629c` is a near-clone of `FUN_003aa888`'s own site at
`0x003ae9e8`: load the current index from `*(*(0x008b36c0))` = `*(0x008c1430)`, clamp
it, compute `n*0x38 + 0x30`, add the array base, materialise `1.0f` (`lis r0,0x3f80`)
- while writing the same `0x55c`/`0x560`/`0x564`/`0x593`/`0x5b0` fields of a
camera-shaped struct that `FUN_003aa888` writes at its own equivalent site. Two
modules, same sequence. Nothing in it touches `+0x00` either.

#### What is still open, and the bounded next avenue

**The `+0x00` write is still not found.** What changed is that "not found" now has a
defensible scope: it was never searched for across four of the five ways to reach the
array. Seven functions on the newly-opened families are entirely unread, and four
more only sampled:

- unread: `FUN_003e29e0`, `FUN_003ff860`, `FUN_00400a00`, `FUN_00403a30`,
  `FUN_004053e0`, `FUN_004074e0`, `FUN_00408fa8`
- sampled at one or five sites only: `FUN_003df360`, `FUN_003fc140`, `FUN_003aa888`,
  `FUN_002b61c8`

**The method for pass seven, deliberately bounded** (and specifically *not* another
bare-mnemonic sweep, the mistake passes four and five both made): per function, run
`search_instructions(function: <f>, mnemonic: "rlwinm", operand_pattern: "0x6, 0x0,
0x19")` to enumerate every `n*0x38` computation - there were 5 in `FUN_003aa888` and
2 in `FUN_003da540`, so this is a handful per function, not hundreds - then
`disassemble_bytes` a +-0x30 window around each and read the folded bias before
believing any displacement. Entry 0 needs no stride at all, so also check each
function for `stw rS,0x0(rX)` / `stwx` whose base traces to an array load.

**And the alternative that five static passes now argue for**: a runtime watchpoint.
`0x008c2cb8 + 0x00` is a fixed, statically-known address in `.data`, so a single
RPCS3 run of a Zone race with a read/write watch on it settles in one shot both
whether the field is ever non-zero and who writes it - including through any
indirect or computed call static xrefs cannot see, which is also what has kept
`FUN_003cdc90`'s caller hidden for three passes.

**Nothing renamed, nothing wired into Rust.** No name is asserted for the array
itself: its layout is measured but its owning subsystem is still inferred from
consumers, which is short of this project's bar for a data name, and per
`CLAUDE.md`'s rule an effect with no recovered trigger stays unwired.
`oag_formats::effectsettings` and `cross_fade_rgba8` are unchanged.

## 2026-08-30, a seventh pass: the last two candidate families are exhausted, and the `+0x00` write is not in any of them

Checked the seven functions the sixth pass left unread or only sampled, using
exactly the method that pass prescribed - per function, trace every `n*0x38`
computation from its own `rlwinm`/`rlwinm`/`subf` triple, read the bias folded into
the following `addi` before trusting a displacement, and look specifically for a
store (not a bare mnemonic sweep):

- `FUN_00403a30`, `FUN_004053e0`, `FUN_004074e0`, `FUN_00408fa8`, `FUN_003fc140`
  (the `-0x50e4(r2)` family, five of its seven members) are all the same shape as
  the sixth pass's own `FUN_003e29e0`/`FUN_003ff860`/`FUN_00400a00`: one `+0x18`
  blend-weight read feeding an `fcmpu` distance/threshold compare, then two
  duplicated `+0x1c`/`+0x20` read blocks feeding a variant-argument-list builder
  (a logging/telemetry call, tag/type/value triples pushed onto the stack) - never
  back into the array. `FUN_00408fa8` caches the array base into a 64-bit stack
  slot (`std`/`ld` at `0x1b8(r1)`) rather than reloading the TOC slot each time, the
  one structural variation found; the read-only shape is identical once traced
  through.
- `FUN_003df360` (the `-0x5700(r2)` slot's other member) reads entry `+0x30` once
  and writes the result straight to a caller-supplied output pointer, not back into
  the array.
- `FUN_002b61c8` (the second module, TOC `0x008ad4d8`, verified via its own OPD
  before trusting anything) is a near-clone of `FUN_003aa888`'s "start a
  transition" sequence, confirmed by tracing `0x002b6230`-`0x002b62a8` byte for
  byte: it writes entry `+0x30` (zeroed) and `+0x34` (`1.0f`), the same two fields
  `FUN_003aa888` already established, and touches `+0x00` nowhere.

**All five TOC-verified paths to `0x008c2cb8` (`0x008b36c4`, `0x008b7220`,
`0x008b7944`, `0x008b7cc4`, `0x008b82e0`) are now fully read, not sampled.** Every
function reachable through any of them has had its own `n*0x38` computations traced
from the instruction level, and none of them stores to entry offset `+0x00`. The
static search space for this exact address is closed: barring an indirect or
computed call this project's static tools cannot enumerate (a function pointer
table, a vtable dispatch, or an address materialised by arithmetic no byte-pattern
search can match), there is no more static ground left to cover for who writes the
stage index.

That leaves the runoff this page has flagged since the fourth pass as the only
remaining avenue: a runtime read/write watch on `0x008c2cb8` in a live RPCS3
session. **Measured the same day and worth recording here since it changes what
that avenue actually costs**: this exact RPCS3 build (`rpcs3-bin` AUR package
`0.0.42.19777-1`, build string `RPCS3 v0.0.42-19777-3be5aa99 Alpha | master`,
matching `rpcs3-debugger.md`'s own measurement byte for byte - same host, not a
stale note) replies to a GDB `Z2` (write watchpoint) packet with an empty packet,
not `OK`. `Z0` software breakpoints are the only stop mechanism this stub offers,
and per `rpcs3-debugger.md` those only fire under `PPU Decoder: Interpreter
(static)`. **Checked, not left open**: upstream `master`'s own `GDB.cpp` (fetched
directly, not assumed) never implements `Z2` at all - has not since GDB support
shipped in 2017 - and neither the `rpcs3-git` AUR package nor any newer build
would gain it, since both track the same `master` this checked. See
[rpcs3-debugger.md](../../../reverse-engineering/rpcs3-debugger.md#the-gdb-stub-is-real-and-needs-no-special-build)
for the full evidence and the emulator-patch estimate. Nothing renamed, nothing
wired into Rust from this pass - a closed static search space is a sharper
negative result, not a positive one.

## 2026-08-30, an eighth pass: independent verification, two undercounted function evidence sets corrected, and the real gap in "closed"

An independent re-derivation of all seven seventh-pass verdicts, done without
trusting the writeup's own enumeration - re-running `search_instructions` for
every array-base TOC displacement per function rather than reusing the prior
site list. **All seven verdicts stand**, but two of the seven writeups
understated their own evidence:

- `FUN_00403a30`: the writeup lists six base-load sites; there are **nine**. The
  three missed (`0x0040503c`, `0x004050ac`, `0x00405114`) were never checked.
  Traced now: all three are loads (biases `+0x10`/`+0x20`/`+0x10`, resolving to
  entry `+0x1c`/`+0x20`/`+0x1c`), same shape as the six already found.
- `FUN_003fc140`: the writeup's own text says "five more" and then lists four
  addresses - internally inconsistent, and the real count is **nine**. The four
  missed (`0x003fd744`, `0x003fd7b4`, `0x003fd81c`, `0x003fd8b4`) are traced now:
  all loads, same `+0x1c`/`+0x20` alternation as the rest of the family.

The other five verdicts (`FUN_004053e0`, `FUN_004074e0`, `FUN_00408fa8`,
`FUN_003df360`, `FUN_002b61c8`) matched the writeup's own enumeration exactly
once independently re-run, including a specific challenge to `FUN_00408fa8`'s
caching claim (its `0x1b8(r1)`/`0x1e0(r1)`/`0x1e4(r1)` frame layout looked like
it might have swapped slot numbers against its siblings; re-tracing confirmed it
uses a genuinely different layout, not a mislabelled one) and a documented
near-miss in `FUN_004074e0` (`stw r0,0x0(r3)` at `0x00408e24` would resolve to
entry `+0x00` if `r3` still held the array base; it does not - `r3` is
re-pointed at an unrelated cursor two instructions earlier, read directly
rather than assumed).

**The completeness argument was also made explicit for the first time**: a
`+0x00` write has exactly three possible shapes - an indexed store (`stwx` and
siblings, swept mnemonic-by-mnemonic: zero hits across all seven functions
except the already-known `FUN_002b61c8` `+0x30` write), a computed index
feeding a plain `stw rD,0x0(rX)` (covered by the per-site consumer tracing:
every register that ever holds the array base is followed to its one consumer,
and every one resolves to a load), and **no index arithmetic at all** - a bare
`stw rD,0x0(rB)` for entry 0 needs no `rlwinm`/`subf` nearby, which is exactly
the shape a bias-keyed search structurally cannot find. That third shape was
checked too: the one place a base register sits live across a stretch of
unrelated code without immediate index arithmetic (`FUN_00408fa8`,
`0x00409074`-`0x004090c8`, between the TOC load and its caching `std`) was
dumped in full, and the register's only consumer there is the cache store.

**A new tool trap, confirmed on this binary rather than only on the PSP
side**: `search_instructions`' `mnemonic` filter is exact-match, not the
substring match its own description claims - already documented for MIPS
delay slots in
[toolchain.md](../../../reverse-engineering/toolchain.md#search_instructionss-mnemonic-filter-is-exact-match-not-substring),
now reproduced on PowerPC: `mnemonic="st"` and `mnemonic="stf"` both return
**zero** matches against a function whose full disassembly plainly contains
`stw`, `std`, `stfs`, `stfd` and `stvx`. A prefix-style sweep for "any store"
returns a vacuous zero indistinguishable from a real negative - every count in
this pass was gathered with exact mnemonics for that reason.

**What "closed" actually means, sharpened rather than walked back**: the
seventh pass's own hedge - "barring an indirect or computed call... an address
materialised by arithmetic no byte-pattern search can match" - now has a
concrete, evidenced instance rather than a generic disclaimer. Every pass
including this one has asked *which functions reference a TOC slot holding
this address*; that question is answered exhaustively. **But a writer that
receives the array pointer as an argument - rather than loading it from a TOC
slot itself - would reference none of the five slots and appear in no
candidate list any pass has built.** This is not a hypothetical: `FUN_003df360`
demonstrates the exact shape in this thread's own evidence, reading entry
`+0x30` off its TOC slot and forwarding the result through a caller-supplied
output pointer (`stfs f0,0x0(r28)`, `r28` = its own third argument). A writer
built the same way round - taking `&array[n]` or the array base as a parameter
- is invisible to every search run so far, seventh and eighth pass included.

**Next step, concrete and bounded**: pivot from "who references this address"
to "who calls one of the now-fully-read accessor functions, and does any call
site pass an array-derived pointer to a further function" - a caller sweep of
the thirteen known accessors, not a new address search. The runtime watchpoint
remains the other option and is agnostic to how a writer obtained the pointer,
but per the section above it needs either upstream RPCS3 gaining `Z2` support
or a from-scratch patch, neither in hand.

**A weaker hypothesis, recorded but not recommended to lead with**: `+0x00` is
statically zero in both entries while the neighbouring fields carry real
non-zero initialisers (`+0x0c`/`+0x10` = `0.5f`, `+0x14`/`+0x18` = `0.1f`,
`+0x34` = `1.0f`), which invites reading "there is no writer, ever." Weak for
two reasons: it is a reinterpretation of bytes the sixth pass already read, not
new evidence, and it fights the read side directly - `FUN_003da540` computes a
current *and* previous stage from this field for a cross-fade, which only
makes sense if the field advances at runtime. If it never did, Zone/Detonator
would never progress past stage 0, which is not what the game does.

Nothing renamed, nothing wired into Rust from this pass either - a corrected
and better-scoped negative result, not a positive one.

## 2026-08-30, a ninth pass: the `+0x00` write is found - `Environment_UpdateStageBlend` at `0x003da540`, confidence 85

Not through the eighth pass's own hypothesis (a pointer forwarded to an
external callee - checked across all fourteen other direct-toucher functions
and found nowhere), but inside `FUN_003da540` itself, one of the fifteen
already-known direct touchers. **Renamed `Environment_UpdateStageBlend`**
(`0x003da540`, confidence 85, `names.tsv` updated) - independently
byte-verified in this session (`disassemble_bytes` against
`/ps3-hdfury-eu/EBOOT.elf` explicitly, not the ambient "current program" -
see the tooling trap below) before the rename, not taken on faith from a
single trace.

### Why eight passes missed it: the index is computed once, then reused by `subf`, not by a fresh `rlwinm`

Every prior pass's search method (`search_instructions(mnemonic: rlwinm,
operand_pattern: "0x6, 0x0, 0x19")`) finds the *defining* `rlwinm` pair once
per function. `Environment_UpdateStageBlend` computes `n*0x38` this way
exactly once, at function entry (`0x003da580`/`0x003da588`: `n*64`/`n*8` into
`r26`/`r27`), then reuses it **six separate times** through a plain `subf
rX,r27,r26`: `0x003da5f0`, `0x003da658`, `0x003da680`, `0x003da720`,
`0x003da770`, `0x003dc70c`. The fourth and sixth passes' own already-known
findings - the `+0x2c` gate and the `+0x04` write - both come from the
*second* reuse (`0x003da658`/`0x003da674`); nothing stopped there to check
reuse four, where the write actually is. **Method for the next function like
this**: after finding the defining `rlwinm` pair, also
`search_instructions(mnemonic: subf, operand_pattern: "<index-hi-reg>,
<index-lo-reg>")` for the specific register pair, to enumerate every reuse
site rather than assuming one `rlwinm` hit means one access.

### The write, traced and independently re-verified

```
003da718  bl      0x0067a7d8           ; transition-pending FX, draw call
003da71c  ld      r2, 0x28(r1)         ; restore this function's own TOC after the call
003da720  subf    r9, r27, r26         ; r9 = n*0x38   (reuse #4 of the r26/r27 pair)
003da724  lwz     r4, -0x5a80(r2)      ; r4 = arrayBase = 0x008c2cb8
003da72c  lfs     f13, -0x5a7c(r2)     ; f13 = *(float*)0x008b7948 = 0.1f default
003da730  addi    r10, r9, 0x10        ; r10 = n*0x38 + 0x10
003da73c  add     r11, r9, r4          ; r11 = entry_ptr = arrayBase + n*0x38 (NO bias)
003da740  add     r7,  r10, r4         ; r7  = entry_ptr + 0x10
003da744  lwz     r8, 0x4(r11)         ; r8 = entry[n].+0x04  (the requested/pending stage)
003da748  lfs     f0, 0xc(r11)         ; f0 = entry[n].+0x0c  (a 0.5f default)
003da74c  stwx    r8, r4, r9           ; *** entry[n].+0x00 = entry[n].+0x04 ***  the stage commit
003da750  stfsx   f0, r4, r10          ; entry[n].+0x10 = old entry[n].+0x0c
003da754  stw     r0, 0x8(r7)          ; entry[n].+0x18 = 0        (r7+0x8 = entry_ptr+0x18)
003da758  stfs    f13, 0x8(r11)        ; entry[n].+0x08 = 0.1f
003da75c  bl      0x0067a7e8           ; matching "end transition" call
```

`n` is `param_2`, already established elsewhere on this page as a 0-or-1
entity/viewport index, not stage-related itself. Both the OPD-verified TOC
(`0x0088c278` holds `{003da540, 008bd3c4}`, confirmed by direct `read_memory`)
and the `-0x5a80(r2)` displacement (`0x008b7944`, the array-pointer slot every
other pass on this page also resolves to) match every prior finding on this
function - this is not a new TOC-boundary trap, the same function this page
has already traced repeatedly.

The gate, decompiled and TOC-resolved:

```c
if (entry[n].+0x04 == entry[n].+0x00) {
    // already applied - the already-documented cross-fade blend runs instead
} else {
    // a new value is pending at +0x04 - draw the transition FX once (one or
    // two calls to FUN_0067a7d8 depending on a flag), then commit:
    entry[n].+0x00 = entry[n].+0x04;   // <-- the write
    entry[n].+0x10 = old entry[n].+0x0c;
    entry[n].+0x18 = 0;
    entry[n].+0x08 = 0.1f;
    FUN_0067a7e8();
}
```

So the mechanism is a **request/commit pair inside one function**, not a
setter reached from outside: `entry[n].+0x04` (already documented at this
page's fourth pass, written at `0x003da674`, gated on the `+0x2c` flag,
sourced from `g_GameState.mode`-dependent tables - mode `0xe` reads a
`FUN_00679e68()` context, modes `0xd`/`0x15` read per-index tables, a fallback
reads `PTR_DAT_008b7c00[n]->+0x640`) holds the *requested* stage. Every call to
`Environment_UpdateStageBlend` checks whether that request has already been
applied (`+0x04 == +0x00`), and if not, commits it - draws the transition
effect and copies `+0x04` into `+0x00`. No external caller passes a stage
value in by pointer, which is exactly why the eighth pass's own
argument-forwarding hypothesis came back negative across all fourteen other
functions even though the write itself is real: it was never going to be
found there.

### A second, distinct `+0x00` write, found as a side effect: `Environment_LoadStageTextures` resets the whole array unconditionally

Re-checking all fifteen direct touchers for call-argument forwarding surfaced
one more direct array access this page had not remarked on:
`Environment_LoadStageTextures` (`0x003d6dc8`) loads the array
(`lwz r9,-0x5a80(r2)` at `0x003d6ddc`) and unconditionally zero/default-resets
**both entries in full** (`0x00`-`0x6c`, matching `FUN_003cdc90`'s own
field pattern exactly - `+0x0c`/`+0x10` = `0.5f`, `+0x14`/`+0x18` = `0.1f`,
`+0x34` = `1.0f`, zero elsewhere) at `0x003d6e44`-`0x003d6eb4`, unconditionally,
every time an effectSettings file loads. This writes `+0x00` too, but only
ever to the constant `0` - a reset, not a stage-selection write, and it plausibly
explains why `FUN_003cdc90`'s own caller has never been found across five
prior passes: this inlined block may be the actual load-time reset path, with
`FUN_003cdc90` either dead code or reached from some other, still-unfound
call site. Not chased further; not what this pass was hunting for.

### A live tooling trap, worth propagating: the shared Ghidra bridge's "current program" is not stable under concurrent sessions

Mid-pass, `get_function_by_address`/`decompile_function` against known-good
PS3 addresses started silently returning "no function found" - not an error
naming the wrong program, just a negative that looks exactly like a bad
address. Cause: another concurrent session on the same Ghidra MCP bridge
switched the "current program" to `/vita-2048-eu-v104/eboot.elf`, twice,
mid-session. **`list_open_programs` plus an explicit `program:
"/ps3-hdfury-eu/EBOOT.elf"` on every call from then on** fixed it - and once,
the bare name `"EBOOT.elf"` itself resolved to the *wrong* program (a
case-insensitive substring match preferred the Vita's lowercase `eboot.elf`
over the PS3's `EBOOT.elf`), so the full path is what actually disambiguates,
not just naming a program at all. **When more than one Ghidra program can be
open at once, pass the full `program` path explicitly on every call for the
rest of the session once this has happened once - never trust "current."**
This session's own verification calls after the rename did the same and
resolved correctly both times.

### What this closes and what it does not

**Closed**: who writes the stage index, with a byte-verified instruction and a
confidence high enough to name and commit
(`Environment_UpdateStageBlend`, confidence 85). Nine passes across two
sessions on this exact question; the actual site was the fourth reuse of an
index computed once at function entry, missed by a search method that (quite
reasonably) assumed one `rlwinm` hit meant one access.

**Not closed by this pass**: the RPCS3 watchpoint and upstream-support
questions above are now moot for *this* specific write (a runtime watch was
never needed to find it), but the render-side wiring this whole page has
deferred since its first pass is still unstarted - `Environment_UpdateStageBlend`
tells us *when* and *to what* the stage index changes, not what the renderer
does with it once it has. That is
[`docs/formats/effectsettings.md`](../../../formats/effectsettings.md)'s and this
handover thread's own next step, not this page's.

## 2026-08-30, a tenth pass: all four `+0x04` source branches read - and mode `0xe` is Detonator, not Zone

The ninth pass named `Environment_UpdateStageBlend` (`0x003da540`) and left
its `+0x04` source as "`g_GameState.mode`-dependent tables" whose contents
were never read. All four branches are read here, out of the instruction
stream at `0x003da5f0`-`0x003da674` plus the three targets they jump to.

```
003da5f0  subf  r30, r27, r26        ; n*0x38
003da5f8  addi  r9, r30, 0x20        ; +0x20 folded bias - the gate is +0x2c
003da600  lwz   r4, -0x5a80(r2)      ; arrayBase = 0x008c2cb8
003da614  lbz   r0, 0xc(r9)          ; entry[n].+0x2c
003da61c  bne   -> 003da678          ; set: skip the whole source block
003da620  lwz   r9, -0x58cc(r2)      ; *(0x008b7af8) = 0x009384e1
003da624  lbz   r0, 0x0(r9)
003da62c  bne   -> 003da650          ; set: skip the mode dispatch, take the fallback
003da630  lwz   r9, -0x58d0(r2)      ; g_GameState
003da634  lwz   r0, 0xe0(r9)         ; g_GameState.mode
003da638  cmpwi r0, 0xe   ; beq 003dd5dc
003da640  cmpwi r0, 0xd   ; beq 003dd364
003da648  cmpwi r0, 0x15  ; beq 003dd3a8
003da650  ; --- fallback ---
003da654  lwz   r10, -0x57c4(r2)     ; *(0x008b7c00) = 0x0098767c, a 2-entry pointer array
003da66c  lwzx  r11, r10, r0         ; r11 = craftArray[n]
003da670  lwz   r0, 0x640(r11)
003da674  stw   r0, 0x4(r9)          ; entry[n].+0x04 = craftArray[n]->+0x640
```

All three mode targets call `FUN_00679e68` first, which decompiles to a
one-line thunk for `RaceManager_GetInstance()`, then read a field of the
returned RaceManager:

| `g_GameState.mode` | target | source of `entry[n].+0x04` |
| ---: | --- | --- |
| `0xe` | `0x003dd5dc` | `RaceManager->+0x2e10` - **not** indexed by `n` |
| `0xd` | `0x003dd364` | `*(RaceManager + 0x2dfc + n*4)` |
| `0x15` | `0x003dd3a8` | `*(RaceManager + 0x351c + n*4)` |
| anything else | `0x003da650` | `craftArray[n]->+0x640` |

### Mode `0xe` is Detonator - a correction to two pages

[mode-manager.md](mode-manager.md) records `Environment_LoadStageTextures`
"branches its Zone-versus-Detonator texture-table choice on `GetMode() ==
0xe`", and put `14` in a Zone/Zone Battle/Detonator bucket without saying
which. It is **Detonator**, on three independent readings:

1. The filename tables themselves, read out of memory rather than inferred.
   `Environment_LoadStageTextures` runs the identical gate/dispatch shape at
   `0x003d71d8`-`0x003d71f0`; the `== 0xe` branch loads `r4` from
   `-0x58c8(r2)` (`0x008b7afc` -> `0x007b26c8`, the string
   `Data/Tex/DetonatorMode0.gtf`), and the fall-through loads it from
   `-0x5848(r2)` (`0x008b7b7c` -> `0x007b2b10`, `Data/Tex/zoneMode0.gtf`).
   So `0xe` selects the **Detonator** table and Zone is the fall-through.
2. `RaceManager->+0x2e10` has exactly **three** writers program-wide
   (`search_instructions` for `stw` at `0x2e10(`). Two of them - `0x000647c4`
   in `FUN_00064470` and `0x00064d44` in `FUN_000649f0`, both `li r9,0x1`
   then `stw r9,0x2e10(r26)` - are `SPDetonator`'s **own two constructors**,
   per [race-manager.md](race-manager.md)'s already-established constructor
   table. A field initialised only by one class's constructors is that
   class's field.
3. The third writer sits in the same address range and is the increment
   below.

### Detonator's own escalation, recovered end to end

`FUN_00067b40` (`0x00067b40`), **renamed `Detonator_UpdateRace`** (confidence 75),
reached only
through its own OPD entry (`0x00872678`) - a virtual `Update`, which is why
`get_xrefs_to` finds no call site. Its gated block:

```
00067c74  lwz   r4, 0x2e10(r31)      ; stage
00067c78  addi  r0, r4, 0x1
00067c7c  cmpwi cr7, r0, 0xe
00067c80  stw   r0, 0x2e10(r31)      ; *** stage = stage + 1 ***
00067c84  ble   cr7, 0x000680d8
```

Decompiled, with the guards: it runs when `raceState->+0x7001` is set and
`raceState->+0x7810 > 1`, adds a score increment, **increments the stage by
exactly one**, and calls `FUN_000654e0(raceManager, oldStage)` while the new
value is under `0xf`. Later in the same function the race-end path is gated
`if (stage < 0xf) return;`.

So Detonator's ladder is: **start at `1`, one step per event, race over at
`15`** - and `detonatormode.effectsettings` names exactly fifteen stages,
`0`-`14`. That answers this thread's long-open "why does Detonator carry
Zone's fifteen-stage speed-class ladder": it has its own fifteen-step
escalation and consumes the table one row per step. The `Start` row is the
pre-race state, never selected once the counter is constructed at `1`.

**Not settled**: what sets `raceState->+0x7001`. Eleven instructions across
the binary touch that offset and ten are `lbz`; the single `stb`
(`0x000e4ac8`, in `FUN_000e4a30`) writes `0`, a clear. So the latch is set
somewhere an offset search does not reach - the same folded-index-bias trap
this page's sixth pass records. Not chased: it is the *event* Detonator
counts, not the stage arithmetic, which is what this pass was after.

### What this means for Zone, which is the thread's actual question

Zone is the fall-through, so its requested stage is `craftArray[n]->+0x640`,
structurally the same shape as 2048's own per-craft `+0x634`
([zone-environment-fallback.md](../vita-2048-eu-v104/zone-environment-fallback.md)).
**No writer was found.** `search_instructions` over every mnemonic with
operand `0x640(` returns 65 hits program-wide, all but a handful stack
frames, and none on this array - which proves nothing, because this
compiler folds a constant bias into the index register before adding the
base, exactly as the sixth pass established for `+0x2c`. `FUN_003d0b98`,
the ~24 KB neighbour that carries the `SpeedClass`/`NextSpeedClass`/
`NextSpeedClassBG`/`SpeedClassParent` scene-node names (the HD analogue of
2048's own Speed Class widget, and the thematic place to look next), was
sampled at those string sites and found to be building node-name buffers
there, not writing the field.

**So HD stays unwired**, and `oag_hd::race::DEFAULTS` carries
`zone_stages: None` rather than 2048's numbers - which name a thirteen-stage
ladder this title does not have. See
`crates/game/src/race/zone_grade.rs`'s module docs.

## See also

- `docs/formats/effectsettings.md` - the file format this loader reaches for
- `docs/formats/envsettings.md` - `FUN_003f3fb0`'s prior identification (sky/skycube loading)
- `docs/ghidra/functions/ps3-hdfury-eu/memory.md` - the TOC defect this whole page works around
- `docs/ghidra/functions/vita-2048-eu-v104/zone-environment-fallback.md` - the 2048 side of the same question, `Zone_UpdateStage`'s own shape
