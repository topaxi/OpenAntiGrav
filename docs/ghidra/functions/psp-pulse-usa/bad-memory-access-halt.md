# The reproducible in-race PPSSPP halt is the LeachBeam fire bit

`E[MEMMAP] Bad memory access detected! 00000030 Stopping emulation.` has killed
at least four PPSSPP sessions on `pulse-psp-usa`, always in a race, always at
the identical JIT block `08872f98_z_un_08872f54` on a `movaps` load. Two pages
recorded it as an unexplained hazard -
[`ppsspp-debugger.md`](../../../reverse-engineering/ppsspp-debugger.md) blamed a
stale watchpoint, and [`psp-pulse-eu/lighting.md`](../psp-pulse-eu/lighting.md)'s
ninth pass left it "not established", suspecting the Missile *or* LeachBeam bit
write that preceded it.

**It is neither a watchpoint nor the Missile.** Setting the **LeachBeam** fire
bit (`0x8000`) by hand puts a weapon instance in a pool with its matrix pointer
`+0xa0` still null, and the next update tick loads a quad from `null + 0x30`.
Reproduced deliberately, with the null pointer read at a breakpoint one
instruction before the fault, alongside a Missile run and a rocket run in the
same session that both fired cleanly and did not halt.

## The function: a floored distance between two world positions

`FUN_08872f54` (`0x08872f54`-`0x0887301f`, 52 instructions) has **no xrefs** -
it is reached through a dispatch table, and its un-rebased pointer
(`0x0006ef54`) does not appear anywhere in `BOOT.BIN`'s data, so the table entry
is built at runtime rather than authored.

```text
08872f54: addiu sp,sp,-0xa0
08872f5c: lw    s1,0x5c(a0)        ; s1 = param->0x5c        (a scene node)
08872f64: lw    a1,0x2c(s1)        ; s1->0x2c                (node flags)
08872f68: move  s0,a0
08872f6c: andi  a0,a1,0x1000       ; the dirty bit
08872f7c: beql  a0,zero,0x08872f98
08872f80: _lw   a0,0x30(s1)        ;   nullified unless the branch is taken
08872f84: jal   0x00140544         ; else resolve the transform first
08872f88: _move a0,s1
08872f94: lw    a0,0x30(s1)        ; a0 = s1->0x30           (cached world matrix)
08872f98: addiu a0,a0,0x30         ; <- the JIT block starts here
08872f9c: lw    a1,0xa0(s0)        ; a1 = param->0xa0        (a second matrix)
08872fa0: addiu a1,a1,0x30
08872fa4: lv.q  C400,0x0(a1)       ; CANDIDATE A: faults if param->0xa0 == 0
08872fb4: lv.q  C300,0x0(a0)       ; CANDIDATE B: faults if s1->0x30  == 0
08872fc0: vsub.q C320,C300,C310    ; difference of the two positions
08872fe4: vdot.t S220,C200,C200
08872fe8: vsqrt.s S230,S220        ; its length
08872ff8: lwc1  f12,0x13c(s0)      ; param->0x13c            (a floor)
08872ffc: c.le.s f12,f0
08873004: bc1fl ...                ; return max(length, param->0x13c)
```

Decompiled, that is:

```c
float FUN_08872f54(int param_1) {
  int node = *(int *)(param_1 + 0x5c);
  if ((*(uint *)(node + 0x2c) & 0x1000) != 0) func_0x00140544(node);
  int matrix = *(int *)(node + 0x30);
  auVar1 = vsub_q(*(v4 *)(matrix + 0x30), *(v4 *)(*(int *)(param_1 + 0xa0) + 0x30));
  float d = vsqrt_s(vdot_t(auVar1, auVar1));
  return d < *(float *)(param_1 + 0x13c) ? *(float *)(param_1 + 0x13c) : d;
}
```

**Confidence 90** on the reading itself: full disassembly and decompile agree,
every offset is read directly off the instructions, and `+0x30` as the
translation row of a 4x4 matrix is corroborated independently by
`func_0x00140544` below (which writes `puVar12[0xc..0xe]` - matrix elements
12/13/14 - as its translation).

## `func_0x00140544` is a lazy scene-node world-transform resolver

Un-rebasing (`0x00140544 + 0x08804000`) gives `FUN_08944544`. It is worth
naming here because it explains what `node+0x30` *is*:

- Gated entirely on `node->0x2c & 0x1000` - a **dirty bit**, which it clears
  (`&= ~0x1000`, then `|= 0x50000`) on the way out.
- Branches on `node->0x2c & 0xf000000`, a **transform kind**: `0x8000000`
  translate-only (`vtfm3.t` against the parent's `+0x30` row), `0x4000000`
  scale (`vmscl.q` by `node->0x54`), `0x2000000`, `0x1000000` full 4x4
  (`vmmul.q` against the parent), and `0` inherit-parent-unchanged.
- Every branch ends `node->0x30 = <resolved matrix>`, and the function returns
  `node->0x30`.
- Called with `param_1 == 0` it returns `&DAT_0002aca8`, a lazily-initialised
  **static identity matrix** copied once from `_DAT_0028c7a0`.

So `node+0x30` is a **cached pointer to the node's world matrix**, and it is
null until the node's transform has been resolved at least once. **Confidence
85**; not renamed - see below.

## Why the fault address is exactly `0x30`

Both `lv.q`s load from `pointer + 0x30` where `pointer` came straight out of
memory. A null `pointer` therefore produces guest address `0x00000030` - which
is precisely the address PPSSPP reports. The host instruction is `movaps`, a
16-byte aligned **load**, which is what PPSSPP's x86 JIT emits for `lv.q` and
not for `sv.q` (a store, `movaps [mem],xmm`), so the fault is one of the two
loads and not one of the three `sv.q`s in the same block. And the JIT block is
named for its start address `08872f98`, the `beql` join point - both `lv.q`s lie
inside `0x08872f98`-`0x0887301c`.

**Confidence 92** that the fault is one of `0x08872fa4` / `0x08872fb4`: the
guest address, the access width, the access direction, and the block range all
agree, and no other instruction in the block can produce a load at `0x30`.

**Statically the two cannot be told apart.** They differ only in the VFPU
register they target (`C400` versus `C300`), and PPSSPP's own register
allocation decides which host XMM each becomes; `xmm6` alone does not pick one -
and the host operand differs run to run (`[rbx+rbp]` in the earlier sessions,
`[rbx+r8]` in this one), so it is not a stable discriminator either. Program
order puts candidate A first, which is a weak argument for it, not a proof.
**Live measurement settles it as candidate A** - see below.

- **Candidate A** (`param_1->0xa0 == 0`): a second matrix pointer the caller
  supplies, never resolved through `func_0x00140544` at all.
- **Candidate B** (`node->0x30 == 0`): a scene node whose world transform has
  **never been resolved** and whose dirty bit is *clear*, so the resolver was
  skipped. A freshly constructed node, or a torn-down one - the dirty bit being
  clear on an unresolved node is the specific inconsistency.

Either way this is a **game-side null dereference**, not an emulator artefact.

## Its only caller: a weapon-instance pool update, beside the LeachBeam

`get_xrefs_to` on `0x08872f54` returns nothing, and that is an artefact worth
knowing about rather than a fact about the binary: **Ghidra renders `jal`
targets in this program un-rebased** (`FUN_08872f54`'s own body calls
`jal 0x00140544`, not `0x08944544`). So a call to `FUN_08872f54` is encoded as
`jal 0x0006ef54` and Ghidra hangs the reference on `0x0006ef54`, where nobody
thinks to look.

Scanning `BOOT.BIN` directly for all three ways the address could be reached -
this is a relocatable PRX, so everything is stored un-rebased - finds exactly
one:

| How it could be reached | Encoding searched for | Hits |
| --- | --- | ---: |
| Direct call | `jal` word `0x0c01bbd5` | **1**, at `0x08866c80` |
| Function pointer in data | u32 `0x0006ef54` | 0 |
| Address built in code | `lui`/`addiu`/`ori` with lo16 `0xef54` | 0 |

So `FUN_08872f54` has **one caller in the entire executable**, `FUN_08866b08`
(`0x08866b08`-`0x08866f13`), and cannot be reached indirectly - there is no
pointer to it anywhere. **Confidence 88**: the scan is exhaustive over the three
encodings, and the one `lo16 = 0xef54` match elsewhere in the file
(`0x088da994`) resolves to `0x0027ef54`, a string, not this function.

`FUN_08866b08` is a **live weapon-instance pool update**:

- Early-outs entirely on `self->0x68 == 0`, so it costs nothing when the pool is
  empty. `self->0x68` is the live count; `self->0x64 + i*4` the instance
  pointers; the loop compacts the array and frees `+0x50`/`+0x4c` as it retires
  an entry.
- Each instance carries a kind at `+0x54` (`1` and `2` are both handled), flags
  at `+0x3c` (bit `0x1` armed, bit `0x4` retire-me, bit `0x40` latched), an owner
  index at `+0x48` into the craft array `DAT_00057ff0`, a target at `+0x5c`, and
  the matrix at `+0xa0` that `FUN_08872f54` measures from.
- It reads the **fire-request word** in the same loop -
  `*(uint *)(craft->0x4c + 0x1b8) & 0x10` - the same `craft+0x1b8` that
  [`weapon-fire.md`](weapon-fire.md) and `scripts/psp-fire-weapon.py` use.
- `FUN_08872f54`'s result is compared against `stats->0x11c`, adjacent to the
  `+0x114`/`+0x118` **LeachBeam lock distances** [`missile.md`](missile.md)
  records.

It is **vtable slot `+0x24`** of a scene-node class: its pointer appears exactly
once in the binary, at `0x08acaa5c`, in a method table whose base is
`0x08acaa38` - the same table shape [`bloom.md`](bloom.md) identified, where
`+0x44` is the draw slot. Slot `+0x24` is where the bloom class leaves the
framework default `0x001407ac`, so it is a **per-frame update** override. The
class's code sits at `0x08864xxx`-`0x08866xxx`, immediately around
`Weapon_FireLeachBeam` (`0x08866658`).

**Confidence 78** on "this is weapon-instance pool machinery": the fire word,
the craft-array index, the stats offset adjacent to the LeachBeam lock
distances, and the code's address neighbourhood all agree. **Confidence 60** on
anything narrower than that - whether the pool is specifically the LeachBeam's,
the Missile's lock, or a shared one is *not* established here.

### Why this matters for reproducing the halt

`FUN_08872f54` runs **only** when that pool is non-empty. A mode with no weapons
in play never calls it and therefore can never produce this halt - so a Time
Trial is not a valid negative control for "does the game crash on its own", only
for "does the emulator crash on its own". Any attempt to reproduce or rule out
the halt has to put a weapon instance in the pool first.

## Settled live: it is the LeachBeam fire bit, and the null is `instance->0xa0`

**Measured 2026-09-05, PPSSPP v1.20.4, `pulse-psp-usa.chd`, Time Trial on
Talon's Junction, one debugger connection throughout.**

The instrument is a single execution breakpoint at `0x08872f98` - the `beql`
join point, reached on both paths, where `addiu a0,a0,0x30` has *not* executed
yet so `a0` is still the raw `node->0x30`. Because only the most recently added
execution breakpoint fires on v1.20.4, the fire bit is written at a
`Weapons_DispatchFire` breakpoint which is then **removed** before the join
point is armed - sequenced, never interleaved.

| Bit written on craft 0 | Bit consumed | Hits at `0x08872f98` | Halt |
| --- | --- | ---: | --- |
| `0x0080` rocket (positive control) | yes | **0** | no |
| `0x0040` **Missile** | yes | **0** | **no** |
| `0x8000` **LeachBeam** | yes | **1** | **yes, immediately** |

The single LeachBeam hit, one instruction before the fault:

```text
hit 0  ra=0x08866c88  s0=0x09b74750  s1=0x09a031b0
       a0 (node->0x30) = 0x09835820     <- valid
       s0->0xa0        = 0x00000000     <- NULL
       s1->0x2c        = 0x0105e006     <- dirty bit 0x1000 clear
```

**Reproduced on a second, independently booted emulator**, same track and mode,
to the byte:

```text
hit 0  ra=0x08866c88  s0=0x09b74750  s1=0x09a031b0
       a0 = 0x09835820   s0->0xa0 = 0x00000000   s1->0x2c = 0x0105e006
```

Every value identical across the two boots - the allocation is deterministic,
so this reproduces for anyone with the same disc, and the halt followed within
a frame both times.

The emulator log, immediately after:

```text
E[MEMMAP]: Core/MemFault.cpp:330 Bad memory access detected! 00000030
(0x7f3d00000030) Stopping emulation. Info:
08872f98_z_un_08872f54
movaps xmm6, [rbx+r8]
```

That settles every open question this page opened with:

- **Candidate A, not B.** `node->0x30` was a perfectly valid `0x09835820`; the
  null is `instance->0xa0`, the weapon instance's own matrix pointer. The
  faulting instruction is therefore `0x08872fa4`, `lv.q C400,0x0(a1)`.
  **Confidence 95** - the pointer was read null at a breakpoint one instruction
  ahead of a fault whose reported guest address, width and direction all match
  that load; the alternative was measured non-null in the same hit; and the
  whole observation repeated identically on a second boot.
- **`s1->0x2c & 0x1000` is clear and `node->0x30` is valid**, so
  `func_0x00140544` was correctly skipped. The transform-resolver path is not
  implicated at all.
- **`ra = 0x08866c88`** is the instruction after the delay slot of the `jal` at
  `0x08866c80`, confirming live what the byte scan found statically: the caller
  is `FUN_08866b08` and there is no other.
- **The halt is caused by the LeachBeam fire bit specifically.** Reproduced
  deliberately on the first attempt, against a freshly booted emulator, with a
  rocket positive control in the same session that fired cleanly and produced no
  hits at all.
- **The Missile is not implicated.** Bit `0x40` was written, consumed, and
  produced **zero** hits at the join point and no halt. Pairing Missile with
  LeachBeam under one crash - which is how
  [`psp-pulse-eu/lighting.md`](../psp-pulse-eu/lighting.md)'s ninth pass and
  `scripts/psp-fire-weapon.py`'s docstring both recorded it - is **wrong**.
- **"The bit was never observed consumed" does not reproduce.** All three bits,
  Missile included, read back set at the dispatcher breakpoint and read back
  clear afterwards.

### Why the raw bit write produces a null

`instance->0xa0` is the matrix the beam measures *from*. Setting `craft+0x1b8`
by hand is equivalent to the player pressing fire, but **only for the dispatch**
- it does not run whatever the pickup-grant path does. The LeachBeam's instance
lands in `FUN_08866b08`'s pool with `+0xa0` never filled in, and the very next
update tick measures from it.

**This is a defect of the test technique, not of the game**, and it is the one
weapon of the four tried where the technique is not sound. Firing a LeachBeam
this way will halt the emulator, every time, within a frame. **Confidence 92**
on the mechanism (two independent boots with identical registers, plus three
prior sessions' identical signature); **confidence 60** on "the grant path is what fills `+0xa0`" - that
path is still unread, per [`../../../gameplay/pickups.md`](../../../gameplay/pickups.md).

## What this corrects

- **`ppsspp-debugger.md`'s "a stale or invalid watchpoint address ... can stop
  emulation outright"** attributed this halt to a watchpoint armed on address
  `0x30`. A PPSSPP memory watchpoint cannot produce a host `movaps` fault with
  the guest base in `rbx` and `0x30` in `rbp` - only the guest's own `lv.q`
  does that, and the coincidence is that the bad watchpoint address `0x30` was
  computed the same way (`someObject + 0x30`) as the faulting one. The trap's
  *advice* stands - bounds-check a computed watch address - but for the
  original reason (a watch on a bogus address counts nothing), not because the
  watchpoint halts the emulator.
- **Arming any memory breakpoint is still plausibly why PPSSPP *halts* rather
  than masking the access**, since mem-checks push the JIT onto a validating
  memory path. That is a hypothesis about the *reporting*, not the *fault*, and
  is untested here.
- **`psp-pulse-eu/lighting.md`'s ninth pass** recorded the halt as possibly
  caused by writing the Missile (`0x40`) or LeachBeam (`0x8000`) fire bit.
  Nothing in `FUN_08872f54` touches a fire word, a weapon record or
  `craft+0x1b8`; the connection, if any, would have to be indirect (a handler
  that ran and left a node half-constructed). See that page for the live test.

## Not renamed

`FUN_08872f54` stays `FUN_08872f54`. What it *computes* is read with confidence
90, but **which subsystem owns it is unknown** and a name has to say that. The
obvious guess - a positional-audio distance attenuation, `max(distance,
minDistance)` - is contradicted by the one struct this project has actually
measured: [`positional-audio.md`](positional-audio.md)'s emitter carries its
radius at `+0x38` and its flags at `+0x5c`, where this function reads a **node
pointer** at `+0x5c` and its float at `+0x13c`. Different layout, so different
class. Below the 50 floor, no rename, per
[ADR-0005](../../../architecture/adr/0005-ghidra-conventions.md).

`FUN_08944544` is likewise left unnamed pending one live `ra` read - see
`## Open` below.

## Open

- **Which class owns `FUN_08866b08`.** Its `Vex_RegisterClass` id is not
  recovered, so whether the pool is the LeachBeam's, the Missile's lock, or a
  shared weapon-instance pool is open. That id is what would take these names
  above the rename floor.
- **What fills `instance->0xa0` on a legitimately granted LeachBeam.** That is
  the pickup-grant path, still unread. Knowing it would turn
  `scripts/psp-fire-weapon.py`'s raw-bit technique into a sound one for this
  weapon instead of an emulator-killer.
- **Whether a legitimately fired LeachBeam ever hits this path with a null.** If
  it cannot, the game has no bug here and only the test does.
