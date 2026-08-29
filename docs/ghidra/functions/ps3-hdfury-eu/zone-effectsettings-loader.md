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

**Net effect**: no new write to the stage index (`+0x00`) or a clean, unambiguous
`+0xc` flag write was found. What is new is real: a plausible (not confirmed) source
for `+0xc`'s own non-zero state, and five of fourteen per-entry fields now placed by
offset rather than four. **Not wired into Rust this pass either** - the layout is
useful documentation, not a trigger, and `FUN_003cdc90` having no located caller is
still the harder blocker than the flag's own meaning.

## See also

- `docs/formats/effectsettings.md` - the file format this loader reaches for
- `docs/formats/envsettings.md` - `FUN_003f3fb0`'s prior identification (sky/skycube loading)
- `docs/ghidra/functions/ps3-hdfury-eu/memory.md` - the TOC defect this whole page works around
- `docs/ghidra/functions/vita-2048-eu-v104/zone-environment-fallback.md` - the 2048 side of the same question, `Zone_UpdateStage`'s own shape
