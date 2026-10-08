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

**Confidence 88 that this array is the craft.** The count matches the grid
size, the shape diagonal is constant per body, and - settled below - one element
responds to the controller's throttle.

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

## Which of the eight is the player, and where the throttle goes

The dumps for this are kept out of the repository - they are guest memory, which
is game content and never commits. `just audit-leakage` covers the rule.

**Body 7 is the player, and `entry+0x4c4` is engine thrust. Confidence 85.**
Driving a *pattern* rather than a single change is what settled it: two full
cycles of three samples thrusting and three coasting, twelve samples in all,
with every float offset of every body scored against the input signal. One field
of one body came out, and it is not close:

    entry+0x4c4, pattern TTT---TTT---
    body 7   r=+0.942   thrust mean  77.25   coast mean   0.73
    body 3   r=+0.530   thrust mean 118.31   coast mean  89.23
    body 4   r=+0.328   thrust mean 104.86   coast mean  95.89
    body 0   r=+0.239   thrust mean 104.87   coast mean  97.37
    body 1   r=+0.044   thrust mean 110.06   coast mean 108.47

Body 7 reads `78.6  56.9  73.1` while the button is held and `2.5  0.7  0.7`
after it is released, then `77.7  60.9  116.3` and `0.3  0.1  0.1` on the second
cycle. It collapses to nothing the moment the throttle goes and comes straight
back. Every other body carries the same field at a steady 90-120 throughout,
which is what it should be - the AI craft are running their own engines, and
only one of the eight is on the end of this pad.

That is also the direct confirmation the body array was missing, so **the
reading that this array is the craft goes to confidence 88**: one element of it
responds to the controller.

Two earlier candidates are **retracted**: body 3, flagged at
`+0x1a0`/`+0x1b4`/`+0x1c0` by a run that turned out to be measuring a crash, and
body 2, flagged at `+0x068` by a six-sample correlation that does not survive
twelve. Both clean identifications - this one and the airbrake run below -
landed on index 7, which is worth noting but is two boots, not a rule.

**The multiple-comparison arithmetic matters here and is what makes this one
safe.** 748 of the dumped fields actually varied enough to be scored, so a
`|r| >= 0.80` threshold expects about 1.3 false positives by chance alone -
which is why the six-sample result was worthless and why a single hit at 0.80
would prove nothing. At `r = 0.942` with twelve samples `p = 4.7e-6`, and over
748 tests the expected number of false positives is **0.00**. That is the whole
argument.

### Two negatives worth keeping, because both looked like results

**Braking is not a usable manipulation.** Cutting thrust and holding both
airbrakes put the craft into a wall - shield `100% -> 0%`, speed readout `100 ->
77`, the craft barrel-rolled - and the body that then stood out was as easily
carrying a contact impulse as being the player. The manipulation has to be one
that touches nothing.

**Six samples is not enough to correlate anything.** A six-sample run scored
`entry+0x068` on body 2 at `r=-0.987` and it looked clean and unarguable. At
twelve samples that same body and offset scores `r=-0.040`, and the whole
cluster around it collapses into noise. With this few degrees of freedom a
single cycle will hand out `|r| > 0.9` to chance; two cycles and twelve samples
are what made the real field separate from seven near-identical decoys.

### Steering: a candidate at `entry+0x284`, not yet a result

The same twelve-sample method run against the airbrakes - thrust held
throughout, left brake for three samples then right for three, twice over, so
the craft wanders rather than walking into a wall. One field crossed the
threshold, again on body 7:

    entry+0x284, pattern LLLRRRLLLRRR
    0.00  0.00  0.00  0.00  -31.54  -16.26  6.39  7.27  7.27  -34.36  -34.36  -34.36

Signed with the brake direction, which is what a yaw or roll term should look
like. But `r = 0.834` over 735 tested fields gives `p = 7.6e-4` and **0.55
expected false positives** - a coin flip - so it was held at confidence 65,
below the naming floor, pending more samples.

**Twenty-four samples refute it.** The same run repeated at double the length
put *nothing* above `|r| = 0.80`, on any body, in either object. `entry+0x284`
was the coin landing heads. **No steering field is identified**, and the
threshold that found the throttle is not sensitive enough to find one - either
the airbrakes do not write a field in the range dumped, or they write one that
the eight-second sampling cadence cannot resolve.

The run did confirm the player a third time, and independently of any
correlation: holding an airbrake throughout depressed `+0x4c4` to a mean of
`62.65` for body 7 against `97.87` to `106.40` for the other seven.

### Speed is not in the bytes that were dumped

With the player identified, the same twelve samples were re-scored against a
leaky integral of the throttle - what a speed that accumulates thrust and bleeds
off would look like - at three decay rates. Nothing in body 7 reached `|r| =
0.85` in either object. Scored against the square wave directly, `+0x4c4` is
alone at `0.942` and the next field down is `0.591`.

So **no speed scalar lives in the entry's first `0x500` bytes or the inner
object's first `0x200`**, and the throttle field is the only input-responsive
one in that range. The search moves outward: the entry's true size is unknown
past `0x4d1`, and the craft almost certainly has a game-side object that is not
this rigid body.

## The array is not craft-only: debris appends to it

The twenty-four-sample run ran long enough for the accumulated airbrake wall
scrapes to destroy the player's craft, and the count moved with it:

    r19:  8 bodies
    r20: 27 bodies      <- the craft explodes
    r21: 23 bodies
    r22: 23 bodies
    r23: 21 bodies
    r24:  8 bodies      <- respawned, back to the grid

The screenshot for `r20` is a fireball with debris fragments thrown across the
sky and no HUD. So the array holds **every** body the physics world simulates,
and wreckage joins it as transient entries.

Two things follow. **`world+0x2c8` is not the grid size** - it is 8 in steady
state during an eight-craft race, which is what made the count such good
evidence, but it is not a constant and anything keyed on it must re-read it
every sample. And **the craft hold stable low indices while transients append
above them**: the throttle field at index 7 separated cleanly across all
twenty-four samples including the ones with 27 bodies, which it could not have
done if the array were being compacted or reordered.

This refines the reading above rather than contradicting it. The array is the
physics world's body list; during a race in steady state that list *is* the
eight craft.

## The craft class, its integrator, and the throttle input

Every body's `*entry` object carries its vtable at `+0x0`, and reading that live
splits the array without needing any of the statistics above:

| Vtable | Indices | Class |
|---|---|---|
| `0x008636e0` | 0-7 | `g_CraftVtable` - the eight racers |
| `0x008638e8` | 8-26 | `g_DebrisVtable` - the wreckage |

The two never mix, and the debris vtable appears only in the samples where the
count spiked. That is an independent confirmation of the split inferred from the
count, arrived at by reading four bytes rather than by correlating anything.

`Physics_StepWorld` calls vtable slots `+0x38`, `+0x3c`, `+0x40` and `+0x44`.
Slot `+0x3c` resolves through OPD `0x00874c88` to **`Craft_IntegrateHull` at
`0x000ef450`**, and it is the craft's per-frame integration: AltiVec throughout,
`r3` the craft, `f1` the delta, four near-identical blocks that each advance a
hull point at `craft+0x120`/`+0x130`/`+0x140`/`+0x150` by a shared vector at
`craft+0x240`, build a query point at `craft+0x160`..`+0x190`, and call
`0x000364c0` - four hull points, four queries a step, which is what a hovering
craft with four suspension points needs.

**The craft object runs to at least `0x360`**, which is why the first pass at
this found nothing: it dumped `0x200`. `Craft_IntegrateHull` reads `craft+0x264`
(a flag word), `craft+0x270` (the rigid body - the same object the world's array
points at, so the two reference each other), `craft+0x2b4`, `craft+0x2b8`,
`craft+0x344` and `craft+0x35c`.

### `craft+0x30c` is the throttle input, and it is exact

Dumping `0x600` and re-running the twelve-sample thrust pattern on body 7:

    craft+0x30c   100.00 100.00 100.00   0.00   0.00   0.00 100.00 100.00 100.00   0.00   0.00   0.00

`r = 1.000`, and the values are not merely correlated - they are exactly `100`
and exactly `0`, which is what a digital button written as a percentage looks
like. **Confidence 90**: this is the throttle as the game reads it, not
something derived from it.

Three neighbours came with it, all on the craft rather than the rigid body:

- **`craft+0x340`** mirrors `entry+0x4c4`, the thrust force - `73.38 / 61.59 /
  67.45 / 9.23 ...` against `75.17 / 63.76 / 69.42 / 9.16 ...`. Same quantity,
  sampled a moment apart.
- **`craft+0x308`, `+0x320`, `+0x324`, `+0x348`** rise monotonically across all
  twelve samples regardless of throttle, by about 3.8 each: game-time clocks,
  offset from one another by fixed amounts.
- **`craft+0x344`** sits at `4.12`, occasionally `4.95`.

This run also reproduced the throttle force on a **third** boot - `entry+0x4c4`
at `r = 0.935` against `0.942` - so that finding now rests on three independent
races.

### `Collision_MarchSegment` at `0x000364c0`, the per-hull-point query

The call `Craft_IntegrateHull` makes four times a step. Until 2026-09-15
Ghidra stopped its body at `0x000364e3` - the word after it is a Cell `lvlx`
that stock sleigh cannot decode (see
[toolchain.md](../../../reverse-engineering/toolchain.md#ps3), the `lvlx`
trap) - and the reading below came from disassembling past that by hand. The
PS3 language decodes it now, the body is 2,260 bytes, and the decompile
agrees with the hand reading line for line: `vectorSubtractFloatingPoint` of
the two 16-byte loads, a `vmaddfp` and two folds for the dot product,
`vrsqrtefp` with the Newton step and the compare-to-zero guard, the
`fctidz`-then-`+1` step count and the `1.0 / (double)count` step:

    v10 = *r5 - *r4                      # the segment, query point minus hull point
    v13 = dot(v10, v10)                  # vmaddfp then two vsldoi/vaddfp folds
    v1  = vrsqrtefp(v13)                 # with a Newton refinement and a zero guard
    f0  = length * <tuning>              # -0x75e8(r2)
    r14 = (long)f0 + 1                   # fctidz, so a step count
    f0  = 1.0 / (float)r14               # fcfid then fdivs against 1.0 at -0x7604
    v30 = v10 * f0                       # one step along the segment
    v7  = v7 + v30                       # and walk it

So it **marches a segment in a bounded number of equal steps**, the count scaled
from the segment's own length - which is what a hover craft's suspension probe
needs, and it explains the four calls: one per hull point.

Its arguments, from the call sites: `r3` = `craft+0x35c`, `r4` = the hull point,
`r5` = the query point, `r6` = an entry in the `craft+0x2b8` array at `0x60`
stride, `r7` = `body+0x4cc`, and it returns a byte stored into the
`craft+0x2b4` array - a hit flag per point.

**Confidence 72.** The marching is not in doubt; the `Collision_` prefix is the
inference, resting on the function's literals - the `0.5` at `0x008a5e8c`, the
`1.0` at `0x008a5ed4` and the globals between them - sitting within `0x50` of
the `Collision.cpp` string slot at `0x008a5ec8` that [collision.md](collision.md)
established, which is how the linker groups a compilation unit's TOC entries.
Five functions call it, so it is shared machinery rather than craft-specific.

### Speed is not stored in either object

Two scans over the deeper dumps, both empty. Correlating every scalar against
the throttle pattern finds only `+0x4c4` and `+0x30c`/`+0x340`. Correlating
every three-float **vector magnitude** finds the same fields and nothing else,
so speed is not hiding as a velocity whose components each look like noise.

A third scan dropped correlation entirely and looked for the *shape* a speed
must have - staying above a quarter of its peak while coasting rather than
falling to zero, higher on average under throttle, falling through each coast
stretch and rising through each thrust stretch. **Zero fields match, in the
entry's first `0x500` bytes or the craft's first `0x600`.**

The fields that do respond all collapse to near zero when the throttle is
released, which makes them forces, not velocities. The likely answer is that the
HUD's km/h is computed at draw time from a velocity the renderer reaches
elsewhere, and that no speed scalar exists to find here.

## Globals

| Address | Name | Confidence | Evidence |
|---|---|---|---|
| `0x009389e4` | `g_CollisionWorld` | 82 | `lwz r3,0x0(r26)` feeds `Collision_ProcessPairs`; read live as `0x333ad370`, a heap pointer, stable across three samples |
| `0x008c1758` | `g_PhysicsSubstepCount` | 75 | `lwz r0,0x0(r28)` is the bound of the fixed-`1/60` step loop |
| `0x00938560` | `g_PhysicsHalfStep` | 72 | `lbz` gate that halves the delta and steps twice |

`0x00992ce0` is left unnamed. Only its `+0x4` is understood - a frame counter -
which is not enough to say what the struct is.

## The body's pose fields, and writing them (2026-10-07)

`+0x1d0/+0x1e0/+0x1f0` basis rows, `+0x200` position, `+0x110..+0x130` the transpose, `+0x190` velocity; the player is `ship+0x6944` in the array at `0x0098d7c0`. Confidence 88. Evidence and the teleport that writes them: [rpcs3-capture.md](../../../reverse-engineering/rpcs3-capture.md#teleporting-the-craft-in-hd-2026-10-07-rpcs3-teleport).

## The handling update, read against a live trace (2026-10-08, hd-handling)

Read with capstone past Ghidra's AltiVec truncation
(`data/scratch/hd-weapon-blasts/ppcdis.py`) and checked against per-frame dumps of the
player's craft from [`scripts/rpcs3-trace.py`](../../../../scripts/rpcs3-trace.py)
(Racebox Time Trial, Talon's Junction, Venom, Feisar concept1). The measured comparison
with this engine is [hd-handling-ground-truth.md](../../../physics/hd-handling-ground-truth.md).
"Craft" below is the object at `ship+0x5fac`, the one this page's earlier sections
call the entry and the craft (`craft+0x270` is the rigid body).

### Two corrections to the sections above

- **`craft+0x340` is speed, not thrust.** Each frame it equals the previous frame's
  forward speed (`dot(velocity, forward)` of the body) to the printed precision (`55.833`,
  `90.890`, `104.508` against the previous rows' `55.833`, `90.890`, `104.508`), and it is
  the speed `Craft_UpdateAirbrakes` gates on and multiplies by. "Speed is not stored in
  either object" above is therefore wrong: the twelve-sample scan read it as a force
  because it falls when the throttle is released. Confidence 88.
- **The craft integrates a variable step.** `craft+0x308` is the game clock in seconds:
  it advances by 0.6 to 1.5 sixtieths a frame on a normal boot and by two sixtieths
  when RPCS3 drops to 30 fps, and distance over speed times its step is `0.996`-`0.999`
  across twenty runs. Whatever `Physics_TickWorld`'s flat `1/60` steps, the craft's motion
  per frame follows the frame's own delta. Confidence 85 for the measurement; how the two
  fit together is not read.

### The craft's control fields (live, confidence 90 each)

| Offset | Content | Evidence |
| --- | --- | --- |
| `craft+0x78` | team block: `+0x78` `AirbrakeGraphics amount` in radians (40 deg = `0.698`), `+0x7c` up 500, `+0x80` down 100, `+0x84..+0x9c` `<Misc>` with `weight_distribution` `-4` at `+0x9c` | live dump, every value matches `feisar_c1/handlingstats.xml` |
| `craft+0x7c` | the race's class block, `0x80` a class: `+0x00` ride_height ... `+0x24` engine gain, `+0x28` engine amount x0.001, `+0x30` accelcap, `+0x38` turning gain, `+0x3c` turning amount, `+0x44`/`+0x48` airbrake gain/falloff, `+0x4c` amount x0.0001, `+0x50` turn, `+0x54` drag, `+0x58` slidegrip x0.0001, `+0x5c` sideshift, `+0x60..+0x6c` `<Physical>`, `+0x70..+0x7c` `<Pitch>` | live dump of all four classes |
| `craft+0x84` | the control record, `PlayerInput+0x4c`: `+0x00` steer, `+0x08`/`+0x0c` left/right airbrake, `+0x10` pitch, each `0..100` | the pointer equals `PlayerInput + 0x4c`; L2 writes `+0x08`, R2 `+0x0c` |
| `craft+0x308` | game clock, seconds | above |
| `craft+0x30c` | throttle `0..100` | [above](#craft0x30c-is-the-throttle-input-and-it-is-exact); reads 92 with Pilot Assist on |
| `craft+0x314` | ramped steering, `+8.3` a frame | `<Turning gain="500">` |
| `craft+0x318`/`+0x31c` | ramped left/right airbrake, `+13.3` a frame; copied to `+0x2fc`/`+0x300` | `<Airbrake gain="800">` |
| `craft+0x32c`/`+0x330` | flap ramps, `+8.3` a frame, from the team block's `+0x7c`/`+0x80` | `<AirbrakeGraphics up_speed="500">`, so the visual flap, not the force |
| `craft+0x340` | previous frame's forward speed | above |

### `Craft_UpdateAirbrakes` at `0x000ee730`

Ramps `+0x318`/`+0x31c` toward the control record's `+0x08`/`+0x0c` at the class block's
`+0x44` up and `+0x48` down, clamps them to `0..100` (`0x008a914c` = 0.0, `0x008a91a8` =
100.0), copies them to `+0x2fc`/`+0x300`, then ramps the flap pair from the team block.
Past the AltiVec at `0x000ee848`, gated on `craft+0x340 > 0`:

    slide   = |L - R| * block+0x54 (drag) * 0.01 * |steer|          # 0x000ee860..0x000ee8dc
    lateral = right-axis force from L * amount and R * amount        # block+0x4c, 0x000ee90c..0x000ee958
    yaw     = (speed * R * turn - speed * L * turn) * 0.001          # block+0x50, fnmsubs at 0x000ee964

with `0.01` at `0x008a91a0` and `0.001` at `0x008a91a4`, then `bl 0x000f5860` and
`bl 0x000f5dd8` on the rigid body. That is Pulse's airbrake law term for term
(`oag_physics::airbrake::evaluate`). **Confidence 85**: the decompile and the hand
disassembly agree, and the two ramps' rates are read live; the force terms themselves
are not isolated at runtime. Not renamed further than the update: `0x000f5860` and
`0x000f5dd8` are the body's force and torque appliers by their arguments, not read.

### `Craft_UpdatePitch` at `0x000f0980`

    gate = dot(craft+0x200 (up), ship+0x7850 * -1.0) > 0.9          # 0x008a9110, *0x008a9208
    if gate and grounded:   torque.x = control+0x10 * block+0x74 (pitch_ground)
    if gate and airborne:   torque.x = control+0x10 * block+0x70 (pitch_air) + team+0x9c (weight_distribution)
    if !gate and airborne:  torque.x = team+0x9c
    if !gate and grounded:  no torque
    bl 0x000f5dd8 on the body

`craft+0x268` is the ship (`0x332a9a20` in both). The gate stayed open on every pitch
run (the craft's up never left the reference by more than a few degrees), so it is read
but not seen to close. Pulse's grounded law is the same product
(`oag_physics::engine::pitch`); the airborne `weight_distribution` bias is the one
`engine.rs` lists as not implemented. **Confidence 80** (static reading; the input and
the block values are live, the torque is not).

### What the four hull points are (live, confidence 85)

The craft's `+0x120..+0x150` and `+0x160..+0x190` hold four hull points and their query
points in world space; in the body's frame they sit at `(+/-1.5, -1.125, +/-4.5)` and
`(+/-1.5, -5.25, +/-4.5)`. This engine's two Pulse probes sit at `(0, -1.125, +/-4.5)`
with a 4.125 reach. Same fore-aft arm, same drop, same reach, and four springs instead
of two: the first place to look for HD's shallower pitch.
