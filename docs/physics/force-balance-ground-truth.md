# The along-track force balance, measured against the original

**Status: the discrepancy is measured, its shape is pinned, its mechanism is not
found.** This page records a first-principles measurement of what force the
original actually applies along a craft's forward axis, taken from captured
traces rather than from decompilation. It exists because the same discrepancy has
now been diagnosed three different ways in
[engine.md](../ghidra/functions/psp-pulse/engine.md), and two of those diagnoses
are refuted here **by measurement, not by argument**.

Reproduce everything below with:

```sh
python3 scripts/trace-force-balance.py \
    data/traces/talons-junction-venom-assegai.csv \
    data/traces/talons-junction-steer.csv \
    --normal-gravity <Physical normal_gravity, off your own disc> \
    --accelcap <Engine accelcap> --amount <Engine amount> \
    --skip talons-junction-venom-assegai.csv:8 \
    --range talons-junction-steer.csv:118-199
```

## Why a forward projection is the right probe

Three facts make this measurement unusually clean, and they are worth stating
because each one removes an entire class of confounder:

1. **`<Physical mass>` is `1` for every shipped class of every team.** Measured
   acceleration therefore *is* net force, with no mass estimate anywhere in the
   chain. This matters directly: engine.md's stun-gate argument cites the
   capture's `-0.21` units/s^2 as implying "a craft mass near 23", and derives
   that 23 from the very balance it was trying to check. **With `mass = 1` that
   corroboration is retired** - it was circular, and it is also simply wrong
   about the number.
2. **Every force that acts along the surface normal cancels.** The hover spring,
   the disputed hover-epilogue downforce, and the vertical damping all act along
   `averageNormal`; the surface-alignment torque holds the ship's `up` on that
   normal, and `up . forward == 0` by orthonormality. Lateral grip cancels too -
   `Ship_ApplyLateralGrip` writes along body `X`.

   So the projection sees exactly four things: engine thrust, quadratic drag,
   rolling resistance, and gravity's along-slope component. **This is what makes
   the downforce question non-load-bearing for this blocker**: no magnitude of
   `hover::DOWNFORCE_SCALE`, at any value, can appear in this measurement. The
   resting-height contradiction recorded in the handover stays open, but it is
   not what is wrong here.
3. **Gravity is removed analytically**, which is not optional. The reference
   capture climbs a **sustained ~4.25 % grade** over its whole 200 ticks
   (`pos_y` runs `-43.66` to `-40.34` across 78.0 units of path). A ship on a
   hill is not at a flat-ground equilibrium, and "the speed is roughly steady, so
   thrust equals resistance" is biased by exactly that slope.

## What the capture actually does

A correction to the handover's reading of the same file: the reference capture's
speed is **not monotonic**. It rises from `24.271` to `25.054` by tick 10 before
falling to `23.571`, and the local ups and downs track the local grade. It is
still net-decelerating, so the conclusion drawn from monotonicity survives, but
the premise as stated is wrong.

The `speed` column is also **not** `|velocity|` - it is a consistent
`1.0367 +/- 0.0002` multiple of it across every row of both captures, so it is a
display or HUD speed. `speed_cached` (`craft+0x2ec`) *is* `dot(velocity,
forward)` to four figures. Anything computing a force balance must use the
velocity columns, not `speed`.

**Tick 8 is a real one-frame impulse, not a glitch and not a dropped frame**:
`|velocity|` jumps `23.355 -> 24.191` in a single 16.5 ms step (`a_fwd` `+52.1`,
with a simultaneous `+18.0` lateral kick) while the position/velocity continuity
ratio stays at `1.025`, inside the `0.98-1.03` band of every other row. That
shape - a forward-and-lateral kick along a section direction - matches
`Ship_ApplySpeedupPad`. **So this capture is not pad-free**, which anyone reusing
it as a clean straight-line reference needs to know. It is excluded from the fit.

## The measurement

Across both original-side captures - 279 usable ticks at full throttle, fully
grounded, no brake, no airbrake, forward speed `16.2` to `24.2`:

```text
net forward force  = +28.841 - 1.2503 * fs      (rms 1.073)
                     zero net force at fs = 23.07
```

The crate's own law over the same range predicts `+32 + fs - 0.005*fs^2`, which
is `+52.6` where the original is `-0.29`. **The ~53-unit discrepancy is real, and
it is confirmed independently of every RE reading**, because nothing above
depends on one.

It also cannot be rescued by any mass: solving the crate's balance for the mass
that would reproduce the measured acceleration gives `M = -223`.

## The discrepancy is linear in speed, not quadratic

This is the substantive new result. Fitting the missing opposing force
(crate prediction minus measurement) as a single-parameter model:

| Model | Coefficient | rms |
| --- | --- | --- |
| `k` (constant) | `50.48` | 4.007 |
| **`k * fs` (linear)** | **`2.280`** | **1.155** |
| `k * fs^2` (quadratic) | `0.1011` | 4.159 |

The one-parameter linear model is nearly as good as any two-parameter fit
(`a + k*fs` reaches only 1.066), and both the constant and the quadratic models
are more than three times worse. **The missing term is linear in speed.**

That refutes engine.md's `0.095 * v^2` target shape, which was inferred from a
single operating point and so could not distinguish the two. It also refutes the
collision-stun gate as the explanation *for this capture*: a gate makes thrust
exactly zero, which predicts a constant `net = -(0.005*fs^2 + 2.0)` of about
`-4.7`, where the measurement runs from `+7.6` at `fs = 17` to `-0.3` at
`fs = 23`. A gate cannot produce a speed-proportional balance.

Adding a linear damping `K` to the crate's law puts the equilibrium at
`22.94 units/s` for `K = 2.28`, against the measured zero-crossing of `23.07`;
with `K = 0` the crate equilibrates at `228 units/s`, which is what the replay
does. **`K` is fitted here and must not be implemented as a constant** - it is
recorded as the size and shape of the target, exactly as the confidence rubric
requires, and the mechanism is still missing.

Confidence **85** on the shape and magnitude. It is a direct measurement over two
captures with a 1.5x speed range, but a single track, a single team and a single
speed class, and the linear/quadratic separation would be firmer over a wider
range.

## The thrust side is correct, and that is now instruction-level

engine.md's `T ~= 58` recomputation had never been audited. It has been now, from
the PSP `BOOT.BIN` disassembly directly, and **every step of it holds**:

| Claim | Evidence | Confidence |
| --- | --- | --- |
| `Engine.amount` is scaled by `0.001` at load | `HandlingXml_ParseEngine` `0x088394ac` materialises `0x3a83126f` (`0.001f`) and `0x08839520` multiplies `+0xbc` by it | 95 |
| `cap = 0.5 * speed + accelcap` | `0x0884c788` loads `0x3f000000` (`0.5f`), `0x0884c790` `mul.s`, `0x0884c7b0` **`add.s`** | 95 |
| `T = min(T, cap)` | `0x0884c7f4` `c.lt.s`, `0x0884c804` stores the cap | 95 |
| the `* 2.0` really is applied | `0x0884c928` materialises `0x40000000` (`2.0f`); the store at `0x0884c934` is **overwritten** by the delay slot at `0x0884c93c`, and `0x0884c938` is `bc1f`, *not* branch-likely, so that store always lands | 92 |
| the throttle ramp is discarded | `0x0884c6b0`-`0x0884c700` ramps `craft+0x2b8` by `gain`/`falloff`, then `0x0884c728` overwrites it with the raw input before any use | 92 |

A sign error in the cap was the obvious hypothesis for a thrust that falls with
speed, and it is **refuted**: `0x0884c7b0` is `add.s`. The cap rises with speed,
as engine.md states.

## Terms checked and eliminated

- **`Body_Integrate`'s linear velocity damping is `0.01`**, confirmed at
  instruction level: the ship-entity constructor materialises `0x3c23d70a`
  (`0.01f`) at `0x08841490` and stores it to *both* `body+0x380` and
  `body+0x384` at `0x088414a0`/`0x088414ac`. At `0.01` it contributes `0.23`
  against the `53` needed. Right shape, wrong size by 228x. Confidence 90.
- **The quadratic drag coefficients are exactly as documented**, read from the
  instruction stream: `-0.005` grounded (`0xbba3d70a`), `-0.002` airborne
  (`0xbb03126f`), `-0.1` reversing (`0xbdcccccd`), and `-0.9` on the
  `craft+0x2a4 == 0` branch (`0xbf666666`, the "about 180x" engine.md
  estimates). Confidence 95.
- **Rolling resistance really does normalise**, so it is constant-magnitude and
  cannot be a hidden linear term: `Ship_ApplyRollingResistance` (`0x08848f4c`)
  runs `vdot` (`0x64088818`) then `vrsq` (`0xd0111918`) then `vscl`
  (`0x65188819`) on the vector at `craft+0x190`. Confidence 80 - the VFPU
  opcodes were decoded by hand, without the Allegrex processor module.

## A term that exists and this crate does not implement

`passive.rs`'s module doc states "there is no dedicated airbrake drag term".
**That is wrong.** `Ship_UpdateAirbrakes` reads `Airbrake.drag` (class `+0xe8`,
reached as `+0x54` through the `craft+0x70` pointer) at `0x0884cce8` and forms

```text
|airbrake_l - airbrake_r| * Airbrake.drag * |steer| * 0.01 * forwardSpeed
```

at `0x0884ccc8`-`0x0884cd08`, gated on `forwardSpeed > 0`. Confidence 85.

It is **speed-proportional**, which is the shape being hunted - but it is
identically zero in both captures used here, because both hold `airbrake_l ==
airbrake_r == 0`. So it is a real missing term worth implementing for fidelity,
and it is **not** this discrepancy. It does mean a capture with asymmetric
airbrake input would be a good independent test of it.

## Where the mechanism has to be

By elimination, a linear-in-velocity opposing force of roughly `2.28 * fs` is
applied by something in `Ship_UpdateCraft` that this crate either does not
implement or implements with the wrong shape. The forward projection proves it is
*not* along the surface normal, so the remaining candidates are the ones whose
VFPU bodies have not been read instruction by instruction:

1. **The hover spring's damping** (`Ship_UpdateHover`, `0x0884870c`), if it damps
   the full velocity vector rather than only its component along the normal. A
   spring-damper written `-c * velocity` instead of `-c * (v . n) * n` is
   precisely a linear drag, and it is the single best-fitting hypothesis.
2. **The inline vertical damping** in `Ship_UpdateCraft` itself, for the same
   reason - `VERTICAL_DAMPING` is `-0.25` along the ship's own up axis, and if it
   is actually applied to the whole velocity the shape is right even though
   `0.25` alone is not the size.
3. `Ship_ApplyLateralGrip` (`0x08848b78`), if its force is not purely along body
   `X`.

All three are VFPU-heavy and need the Allegrex processor module
(`just build-allegrex`) rather than hand decoding. That is the next step.

## What not to do

Do **not** set `ENGINE_OUTPUT_SCALE`, or any drag coefficient, to whatever closes
this. Every constant on the thrust side is now confirmed at instruction level in
the table above, and the drag coefficients are confirmed in both binaries. The
measurement on this page is the result; the constant that would hide it is not.
