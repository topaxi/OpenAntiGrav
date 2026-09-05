# The reproducible in-race PPSSPP halt is a game-side null world matrix

`E[MEMMAP] Bad memory access detected! 00000030 (0x7f8300000030) Stopping
emulation.` has killed at least three PPSSPP sessions on `pulse-psp-usa`,
always a few minutes into a race, always at the identical JIT block
`08872f98_z_un_08872f54` on the identical host instruction
`movaps xmm6,[rbx+rbp]`. Two separate pages recorded it as an unexplained
hazard - [`ppsspp-debugger.md`](../../../reverse-engineering/ppsspp-debugger.md)
blamed a stale watchpoint, and
[`psp-pulse-eu/lighting.md`](../psp-pulse-eu/lighting.md)'s ninth pass left it
"not established" and suspected the Missile/LeachBeam bit write that preceded
it.

**It is neither.** It is the emulated game dereferencing a null pointer, inside
one 52-instruction function, and the faulting instruction is one of exactly two
candidates.

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

**Which of the two is not determined statically.** They differ only in the
VFPU register they target (`C400` versus `C300`), and PPSSPP's own register
allocation decides which host XMM each becomes; `xmm6` alone does not pick one.
Program order puts candidate A first, which is a weak argument for it, not a
proof.

- **Candidate A** (`param_1->0xa0 == 0`): a second matrix pointer the caller
  supplies, never resolved through `func_0x00140544` at all.
- **Candidate B** (`node->0x30 == 0`): a scene node whose world transform has
  **never been resolved** and whose dirty bit is *clear*, so the resolver was
  skipped. A freshly constructed node, or a torn-down one - the dirty bit being
  clear on an unresolved node is the specific inconsistency.

Either way this is a **game-side null dereference**, not an emulator artefact.

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

- **Which of the two `lv.q`s faults**, and therefore which pointer is null. One
  execution breakpoint at `0x08872f98` reading `a0` (already the raw
  `s1->0x30`) and `s0->0xa0` settles it in one hit.
- **Who calls `FUN_08872f54`.** No static xref exists and its pointer is not in
  the data section, so the caller is a runtime-built table entry. `ra` at a
  breakpoint on `0x08872f98` names it directly, and that is also what would put
  `FUN_08944544` and this function's owning class above the rename floor.
- **Whether the halt reproduces with no debugger instrumentation at all.** If
  it does, every "my test crashed the emulator" reading of it is wrong.
