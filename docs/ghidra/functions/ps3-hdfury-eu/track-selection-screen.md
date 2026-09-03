# TrackSelection_Screen: the `Track Creation` screen's class, and no mode filter found in it

2026-09-03. First dig into this binary's front-end/GUI layer - before this page the only
named front-end function anywhere in this Ghidra project was `FrontendRoot_Construct`
(`game-boot.md`). Landed by the same `.cpp`-filename-tag trick `memory.md` and
`mode-manager.md` already established, this time against
`data/plugins/frontend/gui/racebox_definition.xml`'s `Track Creation` screen - the shared
carousel every single-player mode including `Zone` redirects into (see
[`docs/formats/hd-frontend.md`](../../../formats/hd-frontend.md#zone-is-a-mode-list-entry-not-a-separate-screen---and-the-walk-that-would-settle-whether-it-widens-is-blocked-on-hardware-access)).

Read [memory.md](memory.md) first for the per-function TOC defect. Every address named
below is under `0x0026_0000`, well inside the "always `0x008ad4d8`, so Ghidra is right"
tail memory.md describes - and each one was still checked with
`scripts/ps3-toc.py toc <addr>` before being trusted, per this directory's own
`game-boot.md` practice. All read `exact`.

## Class attribution

`search_strings("TrackSelection")` finds two strings: `TrackSelection_Screen.cpp` at
`0x00799878` (the allocator file-tag) and `TrackSelection` at `0x007998f0` (the screen's
own registered type name, read directly inside `TrackSelection_Screen_RegisterType`
below). `get_xrefs_to` on the `.cpp` tag finds exactly two functions, `0x0025a170` and
`0x0025a2c8` - the base-object/complete-object constructor pair this project has seen
before ([`mode-manager.md`](mode-manager.md#the-names)). Unlike that page's pairs, this
class has no derived subclasses to discriminate the two by call-site multiplicity (one
has a single caller, the alloc-and-construct helper below; the other has none but a data
reference), so **neither constructor is named individually** - the same caution
`mode-manager.md` and `game-boot.md` (`GameRoot_Construct`'s own sibling at `0x00011aa0`)
already apply to an unreferenced half of a pair.

## The names

| Address | Name | Confidence |
| --- | --- | --- |
| `0x0025a8b0` | `TrackSelectionScreen_RegisterType` | 82 |
| `0x0025a0b8` | `TrackSelectionScreen_OnConfirm` | 80 |
| `0x0025d070` | `TrackSelectionScreen_OnEnter` | 68 (below 70, carries `_q`) |

### `TrackSelectionScreen_RegisterType` - `0x0025a8b0`

```c
void FUN_0025a8b0(int param_1, int param_2)
{
  if (param_1 == 1) {
    if (param_2 == 0xffff) {
      FUN_0016ab10(PTR_DAT_008b1ec4 + 4, PTR_s_TrackSelection_008b1eec);  // register "TrackSelection"
      *(PTR_DAT_008b1ec4 + 4) = PTR_PTR_008b1ecc;                         // this class's vtable
    }
  }
  else if (param_1 == 0 && param_2 == 0xffff) {
    FUN_0025a670(PTR_DAT_008b1ec4 + 4);   // one of the two destructor-variant vtable slots
    return;
  }
}
```

`param_1` is a register(1)/unregister(0) flag against a static singleton slot
(`PTR_DAT_008b1ec4 + 4`), guarded by a `0xffff` sentinel on `param_2` - a one-time
static-initializer idiom, not per-instance code. `PTR_s_TrackSelection_008b1eec` resolves
to the literal string `TrackSelection` at `0x007998f0`, so the class attribution here is
direct string evidence, not inference. Scored 82 rather than higher only because the
sentinel/flag protocol itself (what calls this with `param_1==1`, i.e. the screen-type
registry this presumably feeds) was not traced further.

### `TrackSelectionScreen_OnConfirm` - `0x0025a0b8`

```c
void FUN_0025a0b8(int param_1, undefined4 param_2)
{
  FUN_00014218(PTR_g_GameState_008b1ec0, 0);
  if (*(int *)(param_1 + 0xc4) == 0) {
    iVar3 = FUN_00209f38(*(undefined4 *)(param_1 + 0x1a8));   // get selected item off the Track list widget
    *(undefined4 *)PTR_DAT_008b1ec4 = *(undefined4 *)(iVar3 + 0x78);
    FUN_0015e8e8(PTR_s_Track_008b1ec8, PTR_DAT_008b1ec4);      // persist keyed "Track"
  }
  if (*(int *)(param_1 + 0x1ac) != 0) {
    FUN_001d3af8(*(int *)(param_1 + 0x1ac), 0);
  }
  FUN_0016a6c8(param_1, param_2);
}
```

`PTR_g_GameState_008b1ec0` is the same `g_GameState` singleton `mode-manager.md` names at
`0x00936fe8`. `PTR_s_Track_008b1ec8` is Ghidra's own auto-derived string-pointer name (the
underlying slot is a relocation target, unreadable by raw `read_memory`, but Ghidra's
analyzer already resolved and labelled it from the string table - the same
`PTR_s_<content>_<addr>` convention `memory.md`'s `PTR_s_Collision_cpp_...` etc. rely on
throughout this directory) - i.e. the literal string `"Track"`. `FUN_0015e8e8` is a
generic key/value state setter with 40+ call sites across the front end (persisted-value
writes keyed by string, the same mechanism `racebox_definition.xml`'s abstract state names
imply). So: on confirm, this reads whichever row is currently selected in the Track list
widget (`param_1 + 0x1a8`, corroborated independently below) and writes it into game state
under the literal key `"Track"`. This is the OnConfirm handler for the Track list, not
its population - included here because it is the strongest, most literal evidence this
page has for which field holds the list widget pointer, which the OnEnter reading below
depends on.

### `TrackSelectionScreen_OnEnter` (applied as `TrackSelectionScreen_OnEnter_q`, confidence 68) - `0x0025d070`

**The strongest candidate found for "the population site" - and the negative result is
the point of this section.** Reached as vtable slot 14 of `TrackSelection_Screen`'s
primary vtable (`PTR_PTR_008b1ecc`, resolved at `0x00869820`; this slot's value differs
from the equivalent slot in a second vtable read from the same data block, confirming it
is a genuine override and not an inherited/shared method). `get_xrefs_to` on this address
finds only its own vtable/OPD data reference - no direct call site - consistent with a
lifecycle callback invoked generically by the screen manager for every screen (the same
reason the `_q` suffix is warranted: the *behaviour* is read directly, but *when* the
framework calls this slot was not independently confirmed against a caller).

What it does, in call order:

1. Fetches a child widget by name (`PTR_s_TrackModel_008b1fe0`, `"TrackModel"`), stores it
   at `param_1 + 0x1ac` - a track-preview widget, not the list.
2. A mode-gated visibility check on an unidentified widget field, `param_1 + 0x1a4`:
   ```c
   int mode = *(int *)(*(int *)(<r2-relative g_GameState slot>) + 0xe0);  // g_GameState.GetMode()
   if (flagA == 0 && (flagB != 0 || (mode != 0xd && mode != 0x15 && mode != 8 && mode != 0x14)))
     param_1[0x1a4] = param_1[0xd4] + 0x44;   // shown
   else
     param_1[0x1a4] = 0;                       // hidden
   ```
   The excluded ids - `0xd`/`0x15` (the Zone-Battle pair) and `8`/`0x14` (SP/MP
   Elimination), per [`mode-manager.md`](mode-manager.md#the-mode-enum-22-ids-eleven-of-them-named)
   - do **not** include Zone's own id (`6`). This widget is hidden specifically for
   Zone-Battle and Elimination, shown for every other mode including Zone. `param_1+0x1a4`
   is not identified (used again in `FUN_0025abb0` and `FUN_0025d558` in ways consistent
   with a secondary per-row control, not the Mode list - Mode selection lives on a
   different screen, `RaceBox_Screen`, per the `Single Player` vs `Track Creation` split
   `hd-frontend.md` already found). **Does not bear on which tracks are listed** - recorded
   because it is the only Zone-adjacent branch found anywhere in this function.
3. Fetches the Track list widget itself by name, stores it at `param_1 + 0x1a8` - the same
   field `TrackSelectionScreen_OnConfirm` reads the selection from, which is the
   corroboration that this is the Track list and not some other list.
4. **The row-pool loop: exactly 32 iterations (`while (count != 0x20)`), unconditional.**
   Each iteration clones a template row widget
   (`FUN_0016aaa8(param_1, uVar3, param_1, *(int*)(*(int*)(param_1+0x114)+0x90)+1)`) and
   positions it. **No mode or zone check appears anywhere in this loop.** Read as a
   fixed-size visual row pool for a scrolling carousel (32 pooled/visible rows), not a
   per-track filter - consistent with `hd-frontend.md`'s own capture of this exact
   carousel scrolling smoothly through more than 32 real entries via recycled rows.
5. After the pool is built, the function reads `*(int *)(param_1_1a8_widget + 0x120)` -
   the **item count already resolved on the list widget object itself** - and stores it to
   `param_1 + 0x104`. This count is not computed here; it is read as a pre-existing
   property of the widget fetched in step 3. Whatever sets that property (a generic
   `List`-widget framework class binding to a data source, most plausibly at front-end XML
   load time) was not located - it is not in this function, and no caller of this function
   was found to chase further (see Open, below).
6. The rest of the function is a **restore-cursor-position search**: walks indices
   `0 .. itemCount-1` via `FUN_00209220(list, i)` and string-compares each item's `+0x94`
   name field against a persisted previous-track-name string, to find which row to
   re-select on screen entry.

**What this establishes for the wider `also_race_circuits` question**
(see [`hd-frontend.md`](../../../formats/hd-frontend.md#zone-is-a-mode-list-entry-not-a-separate-screen---and-the-walk-that-would-settle-whether-it-widens-is-blocked-on-hardware-access)):
across every method read on this page - this OnEnter, the OnConfirm above, the per-frame
`FUN_0025d558` and the per-row label formatter `FUN_0025abb0` below - **no code path
narrows or widens the Track list's item count by mode.** The only Zone-adjacent branches
found (step 2 above, and the label-formatting split in `FUN_0025abb0`) both key off
`g_GameState.GetMode()` for *display* decisions (widget visibility, record-panel label
text), never for the list's item source. That is consistent with, but does not prove, the
list being bound once to the same fixed item set regardless of Mode - the reading
`also_race_circuits: true` already assumes. It does not reach the `also_race_circuits`
question itself, which needs the widget framework's own data-binding code (Not found,
see Open) or the blocked emulator capture `hd-frontend.md` already describes.

## `FUN_0025abb0` - not renamed, recorded for the mode read it makes

Not confidently attributed to a specific lifecycle role (called from the per-frame update,
`0x0025d558`, when the focused row changes - a "refresh the details panel for this row"
shape), so left as `FUN_0025abb0` rather than guessed at. Worth recording regardless: it
reads `g_GameState.GetMode()` again and branches `mode == 6 || mode == 0xe` (Zone-bucket id
`6` per `mode-manager.md`'s elimination-by-exclusion table, or Detonator `0xe`) to select a
different label set for the per-track "records" panel (position/1st/2nd/3rd for ordinary
races vs. a Zone/Detonator-specific set). This is a second, independent front-end read of
`g_GameState`'s mode field at id `6`, corroborating `mode-manager.md`'s attribution of `6`
to the Zone/Zone-Battle/Detonator bucket from a different angle (label formatting, not
`WeaponManager` construction) - but it formats what is shown for the *currently selected*
track, and does not change which tracks are selectable.

## Not found

- **What sets the Track list widget's `+0x120` item count / its `+0x94`-per-item name
  data.** This is the actual population site the wider thread is looking for,
  and it is not in `TrackSelection_Screen`'s own vtable methods - it must be either a
  generic `List`-widget framework class's own construction/data-binding code (shared
  across every `<List>` in every screen, an unbounded search from here with no name or
  string handle found yet) or something upstream that primes the widget before this
  screen's `OnEnter` runs. No caller of `TrackSelectionScreen_OnEnter_q` was found to trace
  backward from (indirect/vtable dispatch only).
- `RaceBox_Screen` (the class implementing the `Single Player` screen where `Mode` is
  actually chosen, `.cpp` tag at `0x00797c98`, constructors `0x00247788`/`0x00247a38`) was
  only lightly explored: its base constructor and one large downstream function,
  `0x00246f08` (reads the confirmed Mode/Track/laps/difficulty selections and derives
  weapon-class/HUD flags for the race about to launch - a "commit and launch" handler, not
  a list-population one) were read and ruled out as the filter site, nothing else on that
  class was checked.
- `param_1 + 0x1a4` in `TrackSelectionScreen_OnEnter_q` (the widget hidden for
  Zone-Battle/Elimination) is not identified.
