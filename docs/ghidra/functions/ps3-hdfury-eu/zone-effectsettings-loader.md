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

**Still open**: the "reversed circuit" cross-title corroboration is still
just a shape match - tracing what sets the reversal flag at
`iVar12 + 0x1bc, +0x1b5` would test that hypothesis against a second,
independent binary rather than leave it resting on 2048's side alone.

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

## See also

- `docs/formats/effectsettings.md` - the file format this loader reaches for
- `docs/formats/envsettings.md` - `FUN_003f3fb0`'s prior identification (sky/skycube loading)
- `docs/ghidra/functions/ps3-hdfury-eu/memory.md` - the TOC defect this whole page works around
- `docs/ghidra/functions/vita-2048-eu-v104/zone-environment-fallback.md` - the 2048 side of the same question
