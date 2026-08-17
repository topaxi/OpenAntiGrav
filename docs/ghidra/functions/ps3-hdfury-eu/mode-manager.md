# ModeManager: the second mode hierarchy, and the constructor pair resolved

`ModeManager` is the sibling of [`RaceManager`](race-manager.md) - a parallel
class family with its own `.cpp` per mode. This page closes its hierarchy the
same way, and lands the two names that the race-manager sweep could not: for the
first time in this binary, **both members of a C++ constructor pair are
distinguishable**, because both are called.

Read [memory.md](memory.md) first for the per-function TOC defect. Every function
named or listed here is below `0x32d5e0`, where Ghidra's TOC is the right one, so
its string references can be read directly.

## The names

| Address | Name | Confidence |
| --- | --- | --- |
| `0x0003c580` | `ModeManager_Construct` | 84 |
| `0x0003c480` | `ModeManager_ConstructComplete` | 82 |

Both reference `"ModeManager.cpp"` at `0x0077b1c0` and neither is referenced by
anything else, so the class attribution is not in doubt. What is new is that
their *roles* are separable:

- **`0x0003c580` has 20 callers**, and every one of them is a derived
  `*_ModeManager` constructor - the ten pairs in the table below, with nothing
  left over in either direction. Under the C++ ABI a derived constructor calls
  its base's **base-object** constructor, so that is what this is.
- **`0x0003c480` has exactly one caller**, at `0x00032df8`, and the call site is
  allocate-then-construct rather than a base chain:

  ```
  00032dd8: bl 0x00676218      ; allocate, cross-module (r2 restored after)
  00032de8: stw r24,0xc(r29)   ; tag the allocation record
  00032df4: or  r3,r31,r31     ; r31 = the new object
  00032df8: bl 0x0003c480      ; construct it
  00032e00: lwz r0,0x34(r31)   ; and start using it
  ```

  That is `new ModeManager`, which is the **complete-object** constructor's
  reason to exist.

84 and 82 are the rubric's "decompilation only, consistent call sites" band; no
runtime trace, no second binary. The second is a point lower because its role
rests on one call site where the first rests on twenty.

**This is the discriminator the race-manager page could not apply.** There, both
members of every leaf class's pair were unreferenced, so which was which could
not be observed and none were named. Here the complete-object constructor is
actually used, and the asymmetry names both.

## The hierarchy

`ModeManager_Construct`'s 20 callers are ten classes, two constructors each:

| Class | Constructors | File string |
| --- | --- | --- |
| `Demo` | `0x0003b578`, `0x0003b5e8` | `0x0077b0a0` |
| `MPArcade` | `0x0003d500`, `0x0003d548` | `0x0077b2c8` |
| `MPElimination` | `0x0003daf0`, `0x0003db38` | `0x0077b360` |
| `MPTimeTrial` | `0x00040f00`, `0x00040f48` | `0x0077b4e8` |
| `MPTournament` | `0x00041ba0`, `0x00041ec8` | `0x0077b5c8` |
| `SPArcade` | `0x00060780`, `0x000607c8` | `0x0077cf18` |
| `SPElimination` | `0x00068db8`, `0x00068e00` | `0x0077d870` |
| `SPTimeTrial` | `0x0006f5c8`, `0x0006f610` | `0x0077dd40` |
| `SPTournament` | `0x00071bc0`, `0x00071e48` | `0x0077dfe0` |
| `AIBatch` | `0x0007b140`, `0x0007b1a8` | `0x0077e7b0` |

Ten pairs, twenty callers, eleven `*ModeManager.cpp` strings on the disc counting
the base. Nothing left over. `0x00041ec8` is placed by elimination - it is the
only caller not otherwise attributed, and `MPTournament` the only file with one
constructor unaccounted for.

The individual constructors are **not named**, for the reason
[race-manager.md](race-manager.md) sets out: nothing derives from a leaf class,
so both members of its pair are unreferenced and indistinguishable. The table is
the attribution; the renames would be a coin flip.

## Two ways this differs from RaceManager

**There is no intermediate multiplayer base.** All five `MP*` mode managers call
`ModeManager_Construct` directly, where the `MP*` *race* managers go through
[`MPRaceManager_Construct`](race-manager.md#the-eleventh-pair-is-the-multiplayer-base-class)
at `0x00045178`. Two parallel families, and only one of them factored its
multiplayer half into a shared base.

**The two families do not cover the same modes.** Fifteen race managers against
ten mode managers, and the sets differ in kind rather than by a constant:

| | Race manager | Mode manager |
| --- | --- | --- |
| `SPArcade`, `SPElimination`, `SPTimeTrial`, `SPTournament` | yes | yes |
| `SPDetonator`, `SPFreePlay`, `SPNitro`, `SPZone` | yes | **no** |
| `MPArcade`, `MPElimination`, `MPTimeTrial`, `MPTournament` | yes | yes |
| `MPNitro` | yes | **no** |
| `Demo`, `AIBatch` | yes | yes |

So five modes have a race manager and no mode manager, and none has the reverse.
What a mode manager does that a race manager does not is unread, and that
asymmetry is the interesting question this page leaves open - the four modes
without one are Detonator, Free Play, Nitro and Zone, which are exactly the ones
that are not conventional races.

## Not recorded

- **The twenty derived constructors.** Attributed in the table, unnamed for the
  pair-ambiguity reason above.
- **`0x00032b18`**, the function that news up a `ModeManager`. Read only far
  enough to establish that its call to `ModeManager_ConstructComplete` follows an
  allocation; what it is remains unexamined.
- **What a `ModeManager` is.** Nothing on this page reads a field, a vtable slot
  or a method. It establishes the class family and its shape, and no behaviour at
  all.
