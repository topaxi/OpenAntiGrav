# Physics: the per-frame tick, the fixed step, and the eight craft bodies

Recovered live, 2026-08-20, with the GDB harness described in
[rpcs3-debugger.md](../../../reverse-engineering/rpcs3-debugger.md) rather than
from strings. There is no `__FILE__` anchor for these - the route in was
[collision.md](collision.md)'s `Collision_ProcessPairs`, whose caller turned out
to be the physics step.

Read [memory.md](memory.md)'s section on the per-function TOC first. Both
functions here declare TOC `0x008ad4d8` in their own OPD entries
(`0x00874ec8` holds `000f8610 008ad4d8` and `000f8a88 008ad4d8` back to back),
which is the TOC Ghidra already assumes, so the TOC-relative loads below resolve
correctly.

## How the caller of Collision_ProcessPairs was found

A breakpoint on `Collision_ProcessPairs` during a live race stopped with
`lr = 0x000f88c0`. That return address lands inside `FUN_000f8610`, whose body
runs `0x000f8610..0x000f8a83`, and the call site is the `bl 0x00038428` one
instruction earlier at `0x000f88bc`.

**That `bl` is straight-line code.** Nothing branches around it, and
`0x000f88a8` - the instruction that falls into it - is the join point every path
out of the body loop reaches. So pair processing runs on every step of this
world, unconditionally. An earlier note in this repository explained a
once-then-never breakpoint pattern by calling the `Collision_*` functions
"contact-gated"; the disassembly rules that out, and the real cause was a
harness defect written up in the debugger page.

## `Physics_TickWorld` at `0x000f8a88`, one tick per frame

Signature `(r3 = world, f1 = frame delta)`. The constants it reads are what
identify it, and they are all in the TOC window `0x008a9364..0x008a938c`:

| TOC | Address | Bytes | Meaning |
|---|---|---|---|
| `-0x4174` | `0x008a9364` | `3f000000` | `0.5`, the half-step scale |
| `-0x4164` | `0x008a9374` | `00992ce0` | a struct whose `+0x4` is a frame counter |
| `-0x415c` | `0x008a937c` | `009389e4` | `g_CollisionWorld` |
| `-0x4158` | `0x008a9380` | `008c1758` | `g_PhysicsSubstepCount` |
| `-0x4154` | `0x008a9384` | `3c888723` | `1/60`, the fixed timestep |
| `-0x4150` | `0x008a9388` | `3d88850a` | `1/15`, the frame-delta clamp |
| `-0x414c` | `0x008a938c` | `00938560` | `g_PhysicsHalfStep` |

The body reads:

    000f8a98: lwz  r11,0x40(r3)        # the world's own tick number
    000f8aa0: lwz  r0,0x4(r9)          # r9 = *(0x00992ce0), the frame counter
    000f8aac: cmpw cr7,r11,r0
    000f8ac0: beq  cr7,0x000f8bd0      # already ticked this frame

so `world+0x40` is a tick number compared against a global frame counter, and
`world+0x40` is incremented before every return. Off the equal path the delta is
clamped with `fsubs`/`fsel` against `1/15` and stored to `world+0x44`; on the
equal path the world is stepped `*(0x008c1758)` times with a flat `1/60`. When
`*(0x00938560)` is set the delta is halved and `Physics_StepWorld` is called
twice instead of once.

**Confidence 80.** The 1/60-and-clamp-at-1/15 shape, the once-per-frame guard
and the substep loop are not ambiguous about what the function is. What is not
established is the class it belongs to, so the name says what it does and not
whose method it is.

## `Physics_StepWorld` at `0x000f8610`, and the body array

`Physics_TickWorld` passes its own `r3` straight through, so both take the same
world object. The step walks a body list four times - an integrate pass, an
enabled pass, a resolve pass, then a post pass - each through a different
vtable slot, and processes pairs between the third and fourth:

| Offset | Read as | Evidence |
|---|---|---|
| `+0x40` | u32 tick number | `Physics_TickWorld` compares and increments it |
| `+0x44` | f32 clamped delta | `stfs f1,0x44(r30)` |
| `+0x48` | inline u32 body pointers | `addi r31,r27,0x48` then `lwz r9,0x0(r31)` |
| `+0x2c8` | u32 body count | `lwz r11,0x2c8(r29)`, the loop bound |
| `+0x2cc` | u8 stepped flag | `stb r0,0x2cc(r29)` with `r0 = 1` before return |
| `+0x2d0` | sub-object | `addi r31,r3,0x2d0`, its `+0x104` gates a branch |

The array is **inline**, not a pointer to elsewhere: the loop indexes
`world+0x48` directly, so it cannot hold more than `(0x2c8-0x48)/4 = 160`
entries. Body pointers are 4-byte words - every use is truncated with
`rldicl rX,rY,0x0,0x20` - so reading them as 8-byte values yields nonsense.

Per body, two objects. The entry itself carries `+0x210`, `+0x4a0`, `+0x4c8`
(a flag word, bits 0 and 1 are tested) and `+0x4d0`, so it is at least `0x4d1`
bytes. `*entry` is a second object with the vtable at `+0x0` and an enabled byte
at `+0x40`, and it is the `this` for the virtual calls.

## Live: eight bodies in an eight-craft race

Read during a Zone-mode race on Vineta K with the harness holding thrust:

    physics step: tid 01000000, world 0x3056cda0
    A thrust: world 0x3056cda0, count 8

**Eight bodies in a race with eight craft.** Every entry had the same three
constants at `+0x010`, `+0x024` and `+0x038` - `17.333`, `24.000`, `17.333` -
which are the diagonal of a 4x4 float matrix based at `+0x10` and identical
across all eight bodies and all samples: a per-body extent or inertia diagonal.

**Confidence 75 that this array is the craft.** The count matching the grid
size and a constant shape diagonal are two independent signs, but no single
body has been tied to the player yet.

The world pointer is **not** static - it was `0x3056cea0` on one boot and
`0x3056cda0` on the next - so the breakpoint-and-poll route is the anchor, and
no global holding it has been found.

### What the moving floats are not

Sampling the bodies at three moments - thrust held, thrust held again, and
coasting - produced large float changes, and it is tempting to read the biggest
as speed. It is not supportable:

- The changes run in **both** directions across an interval where thrust was
  held the whole time (`100 -> 143 -> 76` on one body, `223 -> 99 -> 90` on
  another).
- They spread from 87 to 545 across bodies. Eight craft on one lap of one track
  would cluster near the class top speed.
- `entry+0x200` and `inner+0x0d0` hold the *same* value, as do `entry+0x208`
  and `inner+0x0d8`, and the `inner` values at `0x10` stride differ only in
  their last digits - the signature of two copies of one transform, matrix rows
  sharing a translation column.

So these are a transform that moves as the craft moves. A stored speed scalar
has not been located, and identifying the player's body needs a run that keeps
the raw dumps rather than a ranked summary.

## Globals

| Address | Name | Confidence | Evidence |
|---|---|---|---|
| `0x009389e4` | `g_CollisionWorld` | 82 | `lwz r3,0x0(r26)` feeds `Collision_ProcessPairs`; read live as `0x333ad370`, a heap pointer, stable across three samples |
| `0x008c1758` | `g_PhysicsSubstepCount` | 75 | `lwz r0,0x0(r28)` is the bound of the fixed-`1/60` step loop |
| `0x00938560` | `g_PhysicsHalfStep` | 72 | `lbz` gate that halves the delta and steps twice |

`0x00992ce0` is left unnamed. Only its `+0x4` is understood - a frame counter -
which is not enough to say what the struct is.
