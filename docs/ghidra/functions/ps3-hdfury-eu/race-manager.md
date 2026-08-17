# RaceManager: the singleton holder and the base of the race-mode hierarchy

The second sweep of `EBOOT.elf`, 2026-08-17, alongside
[collision.md](collision.md). Six names from `RaceManager.cpp`. Nothing here
describes race *rules* yet - what it establishes is where the class lives, how
the rest of the game reaches it, and that it is the base of a hierarchy with at
least 22 derived classes.

Read [memory.md](memory.md) first for the per-function TOC defect, and
[collision.md](collision.md) for the `__FILE__`-at-offset-`0x30` anchor both
pages use.

## Why these were findable

`"RaceManager.cpp"` lives at `0x0077cbf0` - raw bytes
`526163654d616e616765722e63707000` - and is reached through TOC slot
`0x008a6900`. The constructor writes that pointer to object offset `0x30`,
exactly as the collision class writes its own filename to the same offset. That
one store is what attributes the whole class.

**Every function on this page was checked against its own OPD entry and all six
declare TOC `0x008ad4d8`**, the value Ghidra already assumes, so their data
reads resolve correctly despite the defect memory.md documents:

```
00872168: 00054628 008ad4d8      00872318: 000568e0 008ad4d8
00872218: 00054778 008ad4d8      00872328: 00057468 008ad4d8
008723c8: 0005d138 008ad4d8      00872330: 00057580 008ad4d8
```

All six are below `0x32d5e0`, which a full walk of the OPD shows is the
boundary under which `0x008ad4d8` is the only TOC in use - see the range table
in [collision.md](collision.md). Ghidra's cross-references can be read directly
in that range.

## The names

| Address | Name | Confidence |
| --- | --- | --- |
| `0x00054628` | `RaceManager_GetInstance` | 82 |
| `0x00054778` | `RaceManager_SetInstance` | 78 |
| `0x0005d138` | `RaceManager_Construct` | 80 |
| `0x000568e0` | `RaceManager_DestructBase` | 78 |
| `0x00057580` | `RaceManager_Destruct` | 80 |
| `0x00057468` | `RaceManager_DestructAndFree` | 80 |

**84 is the ceiling for this sweep** per the rubric's "decompilation only,
consistent call sites" row. No runtime trace, no second binary.

**Four of these six are C++ ABI boilerplate**, not behaviour: one constructor
and three destructor bodies. They are here because they anchor the class and
its 22-subclass hierarchy, and because they were nearly free once the
vtable-slot pattern was established on [`Collision`](collision.md). Only
`RaceManager_GetInstance` and `RaceManager_SetInstance` describe anything the
engine does at run time, and both are one-liners.

## The singleton holder

TOC slot `0x008a6814` points at a small structure that is not the race manager
itself but the slot that holds it:

| Offset | Meaning | Observed in |
| --- | --- | --- |
| `+0x00` | the current race manager, or null | all six functions |
| `+0x08` | the previous one, saved on every change | set/construct/destruct |
| `+0x0c` | a change counter, incremented by the constructor | `RaceManager_Construct` |

Every write to `+0x00` follows the same three-step: read the old value, store
it to `+0x08`, install the new one, then call `printf` with the string at TOC
slot `0x008a684c`. That string is at `0x0077cad0` and its bytes are
`526163656d616e61676572204368616e6765642025702c2025700a` =
`"Racemanager Changed %p, %p\n"`. It is read as raw bytes, not as a Ghidra
label, and it corroborates the structural reading rather than carrying it.

### `RaceManager_GetInstance`

`0x00054628` is one instruction of work: `return *(void **)(0x008a6814)`. It
returns the same slot the constructor installs `this` into and the destructors
clear. A one-line accessor over a singleton pointer.

82 rather than 84 because a getter body proves what it reads, not what the
thing it reads *is*; the "race manager" half of the name comes from the
constructor's `__FILE__` store, one function away.

### `RaceManager_SetInstance`

`0x00054778(newManager)` performs the three-step above and nothing else: saves
the old pointer to `+0x08`, installs the argument at `+0x00`, and notifies with
`(old, new)`. It does not touch `+0x0c`.

78: the body is unambiguous, but this function has no callers that were read,
so "who sets the race manager other than its own constructor" is unanswered.
That is the missing leg.

### `RaceManager_Construct`

`0x0005d138(this)`, and the call sites are what make it readable. It is called
from **22 distinct functions**, and the one that was read - `0x00045178` -
does this:

```c
RaceManager_Construct(param_1);              // chain to the base first
*param_1 = PTR_PTR_008a6320;                 // then overwrite with a derived vtable
param_1[0xb8e] = ...;                        // then initialise fields past 0xb77
```

That is a derived-class constructor calling its base. The base's own fields
stop around index `0xb75`; every field `0x00045178` touches afterwards is above
that. So `0x0005d138` is the base-object constructor of a class with at least
22 subclasses, which for a title with this many race modes is the shape you
would expect.

The body itself: chains to the shared base constructor `0x003271d8`, installs
vtable `0x008628e0`, writes the `"RaceManager.cpp"` pointer to `+0x30`, fills a
`99 x 0xc`-word table at `+0x14c` with `-1` and zero, initialises twelve
`0xffffffff` slots at `+0x4c`..`+0x78`, bumps the holder's `+0x0c`, installs
itself at `+0x00`, and notifies. It then loads three named resources, walks a
list looking for a matching id, and counts objects of type `0xd`, `0xe` and `8`
against a threshold of three - which reads as a mode-availability check and is
deliberately not named.

80: "constructor" is read off the shape and the base-chain call sites, not off
a symbol.

### `RaceManager_Destruct` / `RaceManager_DestructAndFree` / `RaceManager_DestructBase`

Three bodies, one behaviour, three different ways of being reached. The class's
vtable at `0x008628e0` holds, at slots 12 and 13:

```
008628e0 + 0x30: 00872330 -> 00057580     (RaceManager_Destruct)
008628e0 + 0x34: 00872328 -> 00057468     (RaceManager_DestructAndFree)
```

- `0x00057580` is reached **only** from that vtable slot. 67 instructions,
  268 bytes.
- `0x00057468` is reached **only** from the adjacent slot, and its body is
  `0x00057580`'s plus a trailing `FUN_006761f8(param_1)` that frees `this`.
- `0x000568e0` has the **same opcode hash as `0x00057580`**
  (`1df7b8081a40c020…`, 67 instructions, 268 bytes) and is called directly from
  26 sites, among them `0x00043c70`, `0x000440f8` and `0x00063250` - the
  destructors of the derived classes whose constructors call
  `RaceManager_Construct`.

That is the Itanium C++ ABI's destructor triple: complete-object destructor in
the first vtable slot, deleting destructor in the second, base-object
destructor called by subclasses. [`Collision`](collision.md) has the same pair
at the same two slots, which is why the slot ordering is recorded here as a
pattern of this binary rather than an assumption imported from the ABI.

All three restore the vtable pointer, clear a flag bit at `+0x50` of the object
at TOC slot `0x008a685c`, set the word at `0x008a68fc`'s target, set
`*(0x008a68e0) = 1`, and then run the singleton three-step **with null**:
save the old pointer to `+0x08`, write `0` to `+0x00`, notify `(old, 0)`.
Clearing the same slot the constructor filled is the closing half of the
argument.

`RaceManager_DestructBase` scores 78 rather than 80 because "base-object
destructor" rests on it being byte-identical to the vtable's complete-object
destructor while having its own call sites; nothing distinguishes the two
bodies directly.

## Not recorded

- **`0x000569f0`.** The constructor's other half - 639 instructions to
  `RaceManager_Construct`'s 641, different opcode hash, and referenced by
  nothing but its own OPD entry at `0x00872320`. It is the unused member of the
  C++ constructor pair. The two are not identical, so which is the
  complete-object constructor is a real question and not one that identical
  bodies would even pose; it is left open rather than guessed.
- **`0x00054650`.** `if (a == 1 && b == 0xffff) *(u32 *)(holder + 4) = 0x2d90;`.
  The obvious reading is a size or version registration, and it is already in
  trouble: `0x2d90` is *smaller* than the highest offset the constructor
  writes (`0x2dd4`), so it cannot be `sizeof(RaceManager)`. A hypothesis that
  is contradicted by the first check it is put to does not get a name.
- **`0x000556d8`, `0x0005ac00`, `0x0005e948`.** All three reference the
  singleton holder and sit in the `RaceManager.cpp` address range. Not read.
- **The 22 derived-class constructors and 26 derived-class destructors.** They
  are the race modes, and naming them is the obvious next piece of work - each
  one carries its own `__FILE__` string by the same mechanism, so the
  attribution is already there to be read.
- **Race rules of any kind.** Nothing on this page describes lap counting,
  positions or scoring. This sweep found the class, not its behaviour.
