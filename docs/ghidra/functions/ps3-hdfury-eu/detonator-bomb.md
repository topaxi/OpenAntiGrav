# DetonatorBomb: found chasing SortRoot's callers, not the render sort itself

2026-08-25, a side effect of the `SortRoot.cpp` sweep on
[renderer.md](renderer.md#what-was-deliberately-not-read), chasing HD's open
see-through-surfaces depth-sort question. That sweep needed to know what
subsystem owned the six real callers of
`SortRoot`'s base constructor; this is as far as that chase got before it
stopped being about draw order. Read [race-manager.md](race-manager.md) and
[collision.md](collision.md) first for the `__FILE__`-at-offset-`0x30` anchor
this page reuses, and [memory.md](memory.md) for the per-function TOC defect.

## Why this was findable

`"DetonatorBomb.cpp\0"` lives at `0x007845d8`, reached through TOC slot
`0x008aa7a8`. Both candidate constructors write that pointer to object offset
`0x30` (`param_1[0xc]`, the same word index `RaceManager_Construct` and
`Collision`'s own constructor write their filename to) - the anchor that
attributes the whole class, same as everywhere else this pattern has been
used on this binary.

Both functions' own OPD entries declare TOC `0x008ad4d8` - the standard value
for this address range per [collision.md](collision.md)'s range table - so
their decompiles are not subject to the TOC defect [memory.md](memory.md)
documents.

## The names

| Address | Name | Confidence |
| --- | --- | --- |
| `0x00135328` | `DetonatorBomb_Construct` | 80 |
| `0x00134b48` | `DetonatorBomb_ConstructComplete` | 80 |

Both decompile identically apart from which further constructor they are
reached from - the same base/complete split `RenderManager_Construct`
(`renderer.md`) and `RaceManager_Construct` (`race-manager.md`) use. This one
is resolved by caller, the same rule [mode-manager.md](mode-manager.md)
establishes: `0x00135328` has two real callers, `0x0012f088` and `0x0012f430`
(unnamed, unread - a further derived class two levels up from
`SortRoot.cpp`), the shape of a base-subobject call. `0x00134b48` has none -
only its own `.opd` self-entry - the shape of a complete object built
directly at a spawn site, not yet located.

**80, not 84**: decompilation and a consistent call-site pattern is this
sweep's whole evidentiary basis, same class of evidence as
`RaceManager_Construct`. No second binary - Pulse and Pure do not carry this
class - and no runtime trace.

## What this does and does not establish

Both constructors call `RaceManager_GetInstance()` and, when it returns
non-null, allocate a 0x380-byte object and construct it as `SortRoot`'s
derived class "X" from `renderer.md`'s `SortRoot` note (`0x00137690`'s
complete form) - i.e. a `DetonatorBomb` owns one instance of that class as a
plain member, not as a base. That confirms `SortRoot`'s derived classes sit
in gameplay/weapon code, tied to the race manager, not the renderer - the
evidence `renderer.md` uses to walk back the "draw-order root" guess.

**What it does not establish**: what `DetonatorBomb` itself does beyond
constructing (no method beyond the constructors is read here), what class
"X" actually represents (still unnamed - `renderer.md` has what is known),
or anything about render draw order. This page is a waypoint in that chase,
not an answer to it.

## See also

- [renderer.md](renderer.md#what-was-deliberately-not-read) - the
  `SortRoot.cpp` note this page grew out of
- [race-manager.md](race-manager.md) - the `__FILE__`-at-offset-`0x30`
  pattern and `RaceManager_GetInstance`
