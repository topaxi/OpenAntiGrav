# `/pure/BOOT-psp-pure-eu.BIN` - string-anchored names

Two names recovered by string anchoring, plus the import trap that makes every
address-based string tool in the bridge silently return nothing on this
program. **Read the trap section before concluding a string is unreferenced
here** - the default tooling says "0 references" for strings that are demonstrably
in use.

Applied rows are in [`names.tsv`](names.tsv). Method and scoring follow the
[confidence rubric](../../../reverse-engineering/confidence-rubric.md); both
names below are "decompilation only", whose ceiling is 84.

## Why string anchors, and not more fuzzy matching

[`corroboration.md`](corroboration.md) established that cross-binary *body*
fuzzy matching from `psp-pulse-usa` mostly does not hold here - only 7 of
10,703 named source functions matched at threshold 0.8. A user observation
sharpened it: **the Pulse disc still ships some Pure assets, yet no functions
fuzzy-match between the two titles.** Shared assets with unshared code is the
signature of a reused design rebuilt on a later SDK, so body similarity is the
wrong bridge. String references do not care about codegen drift, so they are.

## The trap: code immediates are unrelocated, data addresses are rebased

Every direct route from a string to its referencing function returns **zero**
on this import:

| probe | result |
| --- | --- |
| `find_undocumented_by_string` on five `Data\XML\*.xml` string addresses | 0 referencing functions, each |
| `get_xrefs_to 0x08a4c538` | `No references found` |
| `search_byte_patterns 38 c5 a4 08` (LE pointer to it) | no matches |
| `search_instructions lui` operand `8a5` | 0 of 485,885 instructions |

That reads exactly like "these strings are unused", and they are not. The cause
shows up in the first unfiltered `lui` result:

```
08804090  FUN_0880408c   lui  a0, 0x24
088040b4  FUN_0880408c   lui  a0, 0x2a
088042ac  FUN_08804298   lui  a0, 0x2b
```

**The immediates are pre-relocation offsets.** This is a relocatable PSP ELF
(the import carries `.rel.text` among its 17 overlay spaces); Ghidra rebased the
data listing to image base `0x08804000` but left the code's `lui`/`addiu`
immediates raw. The decompiler prints the same raw constants - `0x27da4c`, not
`0x08a81a4c`.

**This is a property of this import, not of PSP ELFs in general, and not of the
big imports.** Measured, after an earlier draft of this page guessed otherwise
from `memory_size` alone: `get_xrefs_to` on each binary's own
`Data\XML\HandlingStats.xml` string resolves correctly on **both** Pulse
binaries - `0x08a8a270` -> `SystemRoot_Create` on `psp-pulse-usa`, `0x08a89a80`
-> `SystemRoot_Create` on `psp-pulse-eu` - and `psp-pulse-eu` is a 70 MB
`memory_size` import just like this one. So size does not predict the trap.
Confirmed present on `psp-pure-eu`; **untested on `psp-pure-usa`**, which is the
same vintage and the obvious next thing to check.

### Root cause: the image base was set after import, not during it

Established 2026-08-09, and it is not a Pure-versus-Pulse difference at all.
All three PSP binaries are `e_type` `0xffa0` (PSP relocatable) and all three
declare `p_vaddr` `0x00000000`, so **every one of them has to be rebased to
`0x08804000` by hand**. What differs is *when*:

- **Ghidra applies the PSP relocations at load time, against whatever image base
  the loader is given, by rewriting instruction bytes.** Proof: at
  `psp-pulse-usa` `0x0894f6d8` the raw file on disk holds `2800113c`
  (`lui s1, 0x28`) while the Ghidra database holds `a908113c`
  (`lui s1, 0x8a9`). The bytes were rewritten.
- At the equivalent site in `psp-pure-eu` (`0x0898bab8`, file offset
  `0x187b38`) the raw file and the database hold the *same* `2800043c`. Nothing
  was rewritten.

So Pulse was imported **with the image base set in the loader options**, and
relocations were applied against `0x08804000`. Pure was imported at the default
base `0x00000000` - where applying relocations is a no-op - and only then
rebased, via `set_image_base` or the GUI's Memory Map. **Rebasing moves the
data listing but never revisits the instruction stream**, which is precisely the
mismatch this page documents.

A base-`0` import is internally consistent (code constant `0x248538` points
straight at the string, and the bridge's string tooling works). Rebasing it
afterwards is what breaks it. The fix is therefore to **set the image base at
import time**, not to rebase later - see [workflow.md](../../workflow.md).

**Mapping, confirmed not assumed:** `FUN_0898ba60` passes the constants
`0x278504`, `0x278510`, `0x278538` and `0x278544`. Reading memory at those
values plus `0x08804000` gives `/tmp/mq_lo`, `/tmp/mq_js_reserve`, `/tmp/mq_js`
and `/tmp/mq_ui` - four consecutive, individually correct hits, so

```
data address = code constant + 0x08804000
```

### Working the trap

To find what references a string listed at `A`: take `off = A - 0x08804000`,
split it into the `lui`/`addiu` pair, and search on the `addiu`. Honour the
sign-extension carry - when `off & 0xffff >= 0x8000`, the high half is
incremented and the low half becomes negative. For `0x08a81a4c`:
`off = 0x27da4c`, low half `0xda4c >= 0x8000`, so the pair is
`lui 0x28` + `addiu -0x25b4`.

**Verify the high half too.** Searching the `addiu` alone is what produced this
pass's one false positive: the pair building `0x248538`
(`Data\XML\HandlingStats.xml`) is `lui 0x25` + `addiu -0x7ac8`, and the single
`addiu -0x7ac8` in the whole program turned out to pair with `lui 0x28`, as part
of `0x278538` (`/tmp/mq_js`) instead. Low halves collide; confirm with
`decompile_function` and read the constant it prints.

Incidentally established: **no reference to `0x248538` exists anywhere in this
program**, that one `addiu` being accounted for, and the equivalent searches for
`AIControlStats.xml` (`addiu -0x7b3c`) and `AIRaceStats.xml` (`addiu -0x7b20`)
return 0 matches each. The XML loaders were not found. Note this import resolves
only 6,934 functions against `psp-pulse-usa`'s 10,703, so the referencing code
may not be disassembled into functions yet.

## The subsystem these landed in: Pure's embedded web browser

Both names below belong to Wipeout Pure's built-in browser, not to the game.
The surrounding string neighbourhood is unambiguous - `Set-Cookie`,
`Cookie: %s`, `ACCEPT_COOKIE`, `/cookies`, `Set-Cookie:` - and the message-queue
names decode as browser-engine components: `js` (JavaScript), `ui`, and `lo`
(layout), each with a `_reserve` sibling.

This is a subsystem boundary worth knowing: a large share of this binary's
unnamed functions are third-party browser middleware, which is a strong
alternative explanation for the poor Pulse fuzzy-match rate - Pulse has no
browser to match against.

## `Browser_InitMessageQueues` -> `0x0898ba60`

Confidence **78**. Creates the browser's IPC endpoints. The decompilation opens
four `/tmp/mq_*` paths, one of them with mode `0x1c0` (octal `0700`) after an
unlink-shaped call, and stores the results into fields 5, 7 and 8 of a struct
from `func_0x0018a71c`:

```c
uVar3 = func_0x00111f98(0x278538,2);   /* /tmp/mq_js  */  puVar2[5] = uVar3;
uVar3 = func_0x00111f98(0x278504,1);   /* /tmp/mq_lo  */  puVar2[7] = uVar3;
uVar3 = func_0x00111f98(0x278544,1);   /* /tmp/mq_ui  */  puVar2[8] = uVar3;
func_0x00112214(0x278510);                                /* /tmp/mq_js_reserve */
iVar1 = func_0x00111f98(0x278510,0x4202,0x1c0,&local_20); puVar2[6] = iVar1;
```

It then allocates and zeroes several `0x24`-byte blocks. Scored 78 rather than
84 because **the call sites could not be checked** - `get_function_callers`
returns nothing, for the same relocation reason as above - and because the exact
kernel primitive behind `func_0x00111f98` was not identified, so "message
queues" is read off the pathnames rather than from the syscall.

## `Browser_BuildCookieHeader` -> `0x089ec21c`

Confidence **82**. Formats the HTTP `Cookie:` request header. Fetches a cookie
string, allocates, formats and frees the source:

```c
iVar2 = func_0x001e7c98();                    /* fetch cookie string   */
iVar1 = func_0x000dc0a8(iVar2);               /* strlen                */
iVar1 = func_0x00186620(iVar1 + 9);           /* malloc(len + 9)       */
func_0x000db1a0(iVar1,0x27da4c,iVar2);        /* sprintf(buf,"Cookie: %s",s) */
func_0x00186640(iVar2);                       /* free                  */
```

`0x27da4c` is `"Cookie: %s"` (listed at `0x08a81a4c`). **The buffer arithmetic
corroborates the reading exactly**: `strlen("Cookie: ")` is 8, plus one for the
terminator is 9, which is precisely the constant added before the allocation.
That invariant is why this scores above the other, but it is still
decompilation-only with unverifiable call sites, so it stays inside the 70-84
band.

## `Screen_FindElementByPath` -> `0x088b595c`

Confidence **84** - the top of the decompilation-only band. Resolves a
`"->"`-separated element path against a screen, recursively.

```c
iVar1 = func_0x000dc318(param_2,0x24a22c);   /* strstr(path, "->")        */
if (iVar1 == 0) {                            /* leaf: hash and walk kids  */
  iVar1 = func_0x00090040(param_2);          /* hash the name             */
  iVar2 = *(int *)(param_1 + 0xa0);          /* first child               */
  while (...) {                              /* compare +0x5c (name hash) */
    if (iVar3 == iVar1) return iVar2;        /* follow +0x64 (next sib)   */
```

and in the separator branch, after copying the head segment into a 128-byte
stack buffer and resolving it, it recurses on the tail:

```c
iVar1 = func_0x000b195c(iVar2,iVar1 + 2,param_3);
```

**Three independent checks agree**, which is what earns 84:

- `0x24a22c` reads as the literal string `"->"` at `0x08a4e22c`.
- **The `+ 2` is exact**: the recursion skips precisely the two characters of
  `"->"`, no more and no less.
- Call sites pass literal paths of exactly that shape - `"Main Menu->Mode"`,
  `"Multiplayer->Mode"`, `"Create Single Race->..."`.

It stays at 84 rather than higher because this is still reading, with no runtime
trace and no second binary. Children are a linked list (`+0xa0` head, `+0x64`
next) keyed by a **name hash at `+0x5c`**, not by string compare. The hash
function is at `0x08894040`, a `strlen`-then-delegate wrapper around
`0x089041b8`, which **Ghidra has not disassembled into a function** - so whether
the front end reuses the WAD's CRC-32
([`hash_name`](../../../formats/wad.md#the-name-hash)) or a different hash is
**open, and worth an hour**: if it is the same, existing tooling reads front-end
element names directly.

## The `fe::` class taxonomy

The binary carries its front-end class names as literal strings, giving the
complete widget vocabulary without any inference. Core types:

| String | Address |
| --- | --- |
| `fe::FrontendRoot` | `0x08a4e028` |
| `fe::ScreenManager` | `0x08a4e230` |
| `fe::Screen` | `0x08a4e210` |
| `fe::FEMain_Screen` | `0x08a4de60` |
| `fe::Item` | `0x08a4e0a0` |
| `fe::StringTableManager` | `0x08a4eaf8` |
| `fe::Plugin` | `0x08a4e1b8` |

and roughly fifty `fe::*_Item` widget types in one contiguous run from
`0x08a4eb88` to `0x08a50ee8`, among them `Menu_Item`, `FocusMenu_Item`,
`List_Item`, `Text_Item`, `TextButton_Item`, `Image_Item`, `Font_Item`,
`Model_Item`, `Movie_Item`, `Viewport_Item`, `Animation_Item`, `LoadXML_Item`,
`Variable_Item`, `Watch_Item`, `Strip_Item`, `Mode3D_Item`, `MenuBitmap_Item`,
`StatCounter_Item`, `DisplayTracks_Item`, `DisplayRecords_Item`,
`ButtonMapper_Item` and `MemoryStick_Item`.

**Why this matters beyond Ghidra:** it is a direct, evidence-backed list of the
node types the front-end XML can contain, which is exactly what a
`Screens::from_xml` implementation needs to be complete against. `LoadXML_Item`
is independently interesting - it is the include mechanism, and
[`HANDOVER.md`](../../../../HANDOVER.md) records an unexplained dangling
`FEGlobals->TextColor` in Pure's `Skin.xml` whose leading hypothesis is exactly
an unmerged `LoadXML` include. Note also `fe::Browser_Item`: the browser is
reachable as an ordinary front-end widget.

No names are recorded from this table - they are type strings, not functions.
The factory that maps them to constructors was not located.

## Located but deliberately not named: the front-end menu system

**This is the highest-value next target** and the reason this page exists; the
browser names above were a by-product of getting the technique working.

`FUN_08951e58` is game code, not middleware. It builds its element references
from a contiguous string block at **`0x08a76560`** (code constants `0x2725xx`
and `0x2726xx`), which decodes to the front-end's widget-path and game-mode
vocabulary:

```
08a76560  "Main Menu->Mode"          08a76678  "Livery"
08a76570  "Multiplayer"              08a76680  "FE_Livery"
08a7657c  "Multiplayer->Mode"        08a7668c  "Championship"
08a76590  "Join Game"                08a7669c  "MultiplayerMode"
08a7659c  "Create Single Race"       08a766ac  "Mode"
08a765b0  "Create Single Race->..."  08a766b4  "Tournament"
                                     08a766c0  "Track"
                                     08a766c8  "TeamSelectio[n]"
```

The `->` paths show `func_0x000b195c(screen, name, 1)` is a **widget lookup by
path**, which makes it the key to the whole front end - it appears throughout
this region. The function computes two booleans into `param_1 + 0xe1` and
`+ 0xe2` from the current mode, and if either is false looks up `"Livery"` and
clears bits 1-2 of that element's `+0x2c` flags word, i.e. gates the livery
(ship skin) option by game mode.

**No name is recorded for it.** A defensible reading of the tail exists
(`FrontEnd_UpdateLiveryAvailability`), but the function also populates a list
widget beforehand, so a single verb-noun name would overclaim, and call sites
are unverifiable here. That puts a specific name in the 50-69 "could be wrong"
band, and the honest move at the end of a timebox is the string map above
rather than a `_q` guess that stops the next person looking.

## Next

In priority order, per user direction that menus, asset loading, game logic and
physics come first and the browser is explicitly low priority:

1. **Name `func_0x000b195c` (widget lookup by path) and work outwards.** It is
   the spine of the front end and every one of its call sites carries a literal
   element name, so each one is self-documenting once the mapping above is
   applied. Start at `FUN_08951e58`.
2. **Verify at runtime with PPSSPP under Xvfb**, which the user confirmed is
   available. Per the [rubric](../../../reverse-engineering/confidence-rubric.md)
   a breakpoint moves a 78 to a 90 in minutes, and it is the only thing that
   lifts anything here out of the decompilation-only 84 ceiling. Applies to the
   two names above as much as to new work.
3. **Fix the import rather than working around it.** If Ghidra applies
   `.rel.text`, every probe in the failing table starts working and the whole
   binary gets cheap to name. Re-importing this binary the way the Pulse ones
   were imported is the obvious first thing to try.
4. The `Data\XML\*.xml` loaders remain unfound; see the note above that their
   referencing code may not be disassembled into functions yet.
5. **Confirmed 2026-09-01: `psp-pure-usa` has the identical relocation shape.**
   `get_xrefs_to` on its own `Data\XML\HandlingStats.xml` also returns
   nothing, and the same `off = A - 0x08804000` / `lui`+`addiu` split finds the
   one real reference exactly where predicted. One addition to the method: the
   `addiu` half is often in a `jal`'s delay slot, and `search_instructions`'
   `mnemonic` field for it is `_addiu`, not `addiu` - filter by
   `operand_pattern` alone, not by mnemonic, or delay-slot hits are silently
   missed. The same wart extends to `jal` call targets too (unrelocated
   pseudo-addresses, so `get_function_callers` also returns nothing); worked
   around the same way in
   [`psp-pure-usa/rocket-and-collision-fx.md`](../psp-pure-usa/rocket-and-collision-fx.md).
