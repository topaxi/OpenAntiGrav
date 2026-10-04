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

## The mode enum: 22 ids, eleven of them named

Found from the loading screen, which draws a different feature depending on what
you are about to race - see
[loading-screen.md](loading-screen.md). Its mode field turned out to be the one
the executable itself names:

```text
BackendRoot has g_GameState.GetMode()==%d
```

That format string is printed with `*(int *)(*0x00936fe8 + 0xe0)`, so
**`g_GameState` is the pointer at `0x00936fe8`** and `+0xe0` is its mode. Ten
lines later the same expression is bounds-checked against `0x16` and used to
index a 22-entry jump table at **`0x00032f44`** - `GameMode_BackendDispatchTable`
- whose cases each allocate and construct one of the classes above. Matching each case's constructor call
against [the table](#the-hierarchy) names the ids:

| id | class | alloc |
| ---: | --- | ---: |
| 1 | `AIBatch` | `0x9c` |
| 2 | `Demo` | `0xa8` |
| 3, 9, 12 | `SPArcade` | `0x94` |
| 4 | `SPTournament` | `0x2d0` |
| 5, 10 | `SPTimeTrial` | `0x94` |
| 8 | `SPElimination` | `0x94` |
| 16, 18, 21 | `MPArcade` | `0x94` |
| 17 | `MPTournament` | `0x16c` |
| 19 | `MPTimeTrial` | `0x94` |
| 20 | `MPElimination` | `0x94` |
| 0, 6, 7, 11, 13, 14, 15 | *no ModeManager* | `0x94` |

**The seven that share a case are not unused ids.** They take the table's
default, which allocates the base size and constructs nothing from this list -
and the loading screen distinguishes 6, 13 and 14 from each other, so they are
real modes. The likely reading is that they are the ones with a `RaceManager`
subclass and no `ModeManager` of their own: [race-manager.md](race-manager.md)
counts fifteen concrete race modes against this file's ten, and Zone, Zone
Battle and Detonator are the obvious candidates. Not established.

**What the loading screen says about three of the seven**, 2026-10-04
(`LoadingScreen_Construct`, [loading-screen.md](loading-screen.md)): id `6`
draws its feature from `{0, 1}`, id `0xd` from `{0, 1, 3}` (the deck
`MPArcade`'s `0x15` shares), and id `0xe` always shows feature `1`. That they
carry decks of their own is what keeps them from being the default row, so
they are real modes. Which game mode each is stays open; `0xe` is also the id
`Environment_LoadStageTextures` branches Zone against Detonator on, which
fits Zone or Detonator but does not choose between them.

**Why several ids share a class.** `SPArcade` at 3, 9 and 12, `MPArcade` at 16,
18 and 21 - a single race, and presumably its variants (one-off, campaign cell,
custom) reaching the same manager. Which is which is unread.

Confidence **88** on the eleven: each is a constructor address matched against
an attribution table built independently, and the alloc sizes agree with it -
`MPTournament`'s `0x16c` and `SPTournament`'s `0x2d0` are the two that are not
`0x94`, and they are the two classes that are not. **90** on `g_GameState` and
the dispatch table, which the format string and the bounds check state outright.

**One live corroboration.** A scripted RPCS3 run of a campaign cell logged
`GetMode()==3`, and 3 is `SPArcade` - a single race, which is what a campaign
cell is. See `docs/reverse-engineering/rpcs3-debugger.md`.

**A second data point for id `14`, from an unrelated function.**
`Environment_LoadStageTextures` (`0x003d6dc8`,
[zone-effectsettings-loader.md](zone-effectsettings-loader.md)) branches its
Zone-versus-Detonator texture-table choice on `g_GameState.GetMode() == 0xe`
(`14`) specifically - a second, independent function keying real behaviour on
the same id this page already put in the Zone/Zone Battle/Detonator bucket by
elimination.

**Settled 2026-08-30: id `14` is Detonator**, and the sentence above needed
correcting in one respect - `== 0xe` is the *Detonator* side of that branch,
not the Zone side. Read out of memory rather than inferred: the `== 0xe`
branch loads its filename table from `0x008b7afc` -> `0x007b26c8`, the string
`Data/Tex/DetonatorMode0.gtf`, and the fall-through from `0x008b7b7c` ->
`0x007b2b10`, `Data/Tex/zoneMode0.gtf`. Corroborated independently by the
`SPDetonator` constructor pair in [race-manager.md](race-manager.md): the
same mode's stage counter (`RaceManager->+0x2e10`) is initialised to `1` by
`0x00064470` and `0x000649f0` and by nothing else in the binary. Zone and
Zone Battle remain unassigned within the bucket; modes `0xd` and `0x15`
both read *per-viewport* stage entries of the same RaceManager, which is the
shape a two-player mode needs. Full trace on
[zone-effectsettings-loader.md](zone-effectsettings-loader.md#2026-08-30-a-tenth-pass-all-four-0x04-source-branches-read---and-mode-0xe-is-detonator-not-zone).

**2026-09-02, a narrowing but not a close.** [`weapons.md`](weapons.md#weaponmanagers-other-ten-members-nine-named-one-inherits-from-cannon)
found that `WeaponManager_Construct` builds no weapon-adjacent member at all
for mode `6`, and builds exactly one (`LightBarrierManager`, not a weapon)
for modes `0xd` and `0x15` - consistent with `6` being single-player Zone and
`0xd`/`0x15` a Zone Battle pair, but it does not assign which id is which.

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
