# The along-track force balance, measured against the original

**Status: resolved. The force law is correct; the discrepancy was contact.** Jump
to [the resolution](#resolved-the-force-law-was-right-and-both-reference-captures-were-in-sustained-contact)
for the standing-start capture that settles it, and read the sections before it
as the investigation that got there - they are kept because each closes a
hypothesis, and because the measurement technique is reusable.

The one-line version: the recovered force law reproduces a real launch from a
standstill to `fs = 47.7` with **rms `0.127` units of force**, and the `~52`-unit
deficit seen in the two older captures is a **`3.67 %` per-frame reduction of the
velocity applied outside the force accumulators**, recorded all along in the
trace's own `speed` column. `2.28 * fs` was the right shape and the right size
for the wrong kind of thing.

This page records a first-principles measurement of what force the
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

1. **The mass the integrator divides by is `1`.** Measured acceleration therefore
   *is* net force, with no mass estimate anywhere in the chain.

   This is the load-bearing premise of the whole page, so it is verified rather
   than assumed. engine.md records "`mass` is read from `body+0x374`, not from
   the XML `mass` at `+0xf4`; how the two relate is not determined" - **the
   relation is now determined, and it is the identity**:

   ```text
   08849854  lw   $a0, 0x1cc($s0)     ; craft->body
   08849858  lw   $a1, 0x70($s0)      ; craft->class block
   0884985c  jal  Body_SetMass        ; 0x0884d850
   08849860  lwc1 $f12, 0x60($a1)     ; delay slot: class+0x60 == XML mass (0x94+0x60 == 0xf4)
   ```

   and `Body_SetMass` (`0x0884d850`) is four instructions: `body+0x374 = m`,
   `body+0x378 = 1/m`, with no scaling. `<Physical mass>` is `1` for every
   shipped class of every team, so `body+0x374 == 1`. Confidence **92**.

   Getting this wrong would invert the page's conclusion, which is why it is
   spelled out: at an integrator mass of `2` the missing force's scaling exponent
   comes out at `1.94` instead of `1.09`, i.e. the quadratic shape this page
   refutes. The refutation is only as good as this reading.

   It also retires a corroboration: engine.md's stun-gate argument cites the
   capture's `-0.21` units/s^2 as implying "a craft mass near 23", and derives
   that 23 from the very balance it was trying to check. **It was circular, and
   the number is wrong by a factor of 23.**

   *A trap worth recording*: `jal` targets in `BOOT.BIN` encode the **ELF**
   vaddr (base `0`), not the Ghidra image base `0x08804000`. Scanning for a call
   to `0x0884d850` as `0x0E213614` finds nothing; the encoded word is
   `0x0C012614`. Subtract the image base before building a `jal` pattern.
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
`1.0367 +/- 0.0002` multiple of it across every row of both captures.

> **That "display or HUD speed" guess was wrong, and the discrepancy it names is
> the answer to this whole page.** `speed` is `body+0x398`, which `Body_Integrate`
> writes as `sqrt(dot(v, v))` from the same register it stores as the velocity -
> so it *is* `|velocity|`, and the `3.67 %` gap means something shrinks the
> velocity after the integrator runs. See
> [the resolution](#resolved-the-force-law-was-right-and-both-reference-captures-were-in-sustained-contact).
> The operational advice below is still right, for a different reason: a force
> balance must use the velocity columns.

`speed_cached` (`craft+0x2ec`) is `|dot(velocity, forward)|` - the absolute value
is now read at instruction level, `vabs.s` at `0x08849930`.

**Tick 8 is a real one-frame impulse, not a glitch and not a dropped frame**:
`|velocity|` jumps `23.355 -> 24.191` in a single 16.5 ms step (`a_fwd` `+52.1`,
with a simultaneous `+18.0` lateral kick) while the position/velocity continuity
ratio stays at `1.025`, inside the `0.98-1.03` band of every other row. That
shape - a forward-and-lateral kick along a section direction - matches
`Ship_ApplySpeedupPad`. **So this capture is not pad-free**, which anyone reusing
it as a clean straight-line reference needs to know. It is excluded from the fit.

The `+18.0` lateral component also reads like a collision, which would arm
`craft+0x290` for 0.5 s - about 30 ticks of *zero* thrust. **Checked directly,
and it did not happen**: median implied thrust over ticks 9-38 is `4.154`,
against `4.619` over ticks 40-199 and `4.659` over ticks 0-7. The small gap is
fully explained by those ticks sitting at a higher speed (median `fs` `23.85`
against `22.92`, on a slope of about `-0.54`); a fired stun would have driven the
figure to `0`, not moved it by `0.4`. **The stun gate does not fire anywhere in
this capture**, which is the direct confirmation the handover asked for and
independently closes commit `21798c6`'s open question.

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
are more than three times worse.

**That fit alone would be overclaiming, and the check that matters is this one.**
The two captures cover disjoint speed bands, so a regression across both mixes a
between-capture offset into the slope: fitted *within* each capture the slope is
only about `-0.55` (assegai) and `-0.60` (steer), not the `-1.05` the pooled fit
reports. The robust statement is therefore a **two-point** one, which needs no
slope at all - just the size of the discrepancy at two well-separated speeds:

| Capture | median `fs` | median missing force | s.e.m. |
| --- | --- | --- | --- |
| assegai | `23.034` | `52.538` | 0.07 |
| steer recovery | `20.040` | `45.133` | 0.35 |

That is a scaling exponent of **`p = 1.09`**. Anchoring on the assegai point and
predicting the steer point:

| Assumed shape | Predicted | Measured | Error |
| --- | --- | --- | --- |
| **linear, `fs^1`** | `45.71` | `45.13` | **+1.3 %** |
| quadratic, `fs^2` | `39.77` | `45.13` | -11.9 % |
| constant, `fs^0` | `52.54` | `45.13` | +16.4 % |

Both alternatives miss by an order of magnitude more than the measurement error.
**The missing term is linear in speed**, with a hint of a small constant
component as well (the shallow within-capture slopes are what a `C + K*fs` mix
would produce).

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

> **This section is right about the shape and the size, and wrong about what it
> is.** `2.28 * fs` is what a `3.67 %`-per-frame *velocity* reduction looks like
> when you insist on reading it as a force: `0.0367 / 0.0165 = 2.22`. Both
> captures used here were recorded with the craft in sustained wall contact for
> their whole length, so `K` is a property of the collision response, not of the
> force law. See [the resolution](#resolved-the-force-law-was-right-and-both-reference-captures-were-in-sustained-contact).
> The instruction to not implement `K` as a constant stands, and now has a
> reason rather than a caution.

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
  cannot be a hidden linear term. **Now read with the Allegrex module, magnitude
  included**: `Ship_ApplyRollingResistance` (`0x08848f4c`) is
  `vmul.t`/`vfad.t`/`vrsq.s` to form `1/|v|`, a `vpfxs [-X,-Y,-Z,0]` prefix on
  `vscl.t C610,C200,S600` giving `-v/|v|`, and then `vadd.t C610,C610,C610` - a
  self-add that doubles it. So the magnitude is exactly `2`, from the
  instruction rather than from a parameter or a prefix-encoded literal.
  Confidence raised from 80 to **92**.
- **The cached speed `craft+0x2ec` is `|dot(velocity, forward)|`.**
  `Ship_UpdateCraft` runs `vdot.t` at `0x0884992c` and **`vabs.s` at
  `0x08849930`** before storing it, and a scan over every `0x2ec(` displacement
  finds that store to be its only writer on a craft base. Two consequences, both
  now implemented in `crates/physics`: the reversing branch of
  `Ship_ApplyQuadraticDrag` (`craft+0x2ec < -0.2`, the `-0.1` coefficient) is
  **unreachable in the original**, and the drag term is dissipative in both
  directions of travel rather than adding energy while reversing - which settles
  an open sign question `passive.rs` had recorded as "a guess awaiting M3".
  Confidence 90 on the `vabs.s`, 88 on the sole-writer negative.

## A term that exists, and that this crate implements after all

`passive.rs`'s module doc states "there is no dedicated airbrake drag term".
**That is wrong.** `Ship_UpdateAirbrakes` reads `Airbrake.drag` (class `+0xe8`,
reached as `+0x54` through the `craft+0x70` pointer) at `0x0884cce8` and forms

```text
forward * |airbrake_l - airbrake_r| * Airbrake.drag * |steer| * 0.01 * 0.001 * speed
```

at `0x0884ccc8`-`0x0884cdb4`, gated on the cached speed being positive.
Confidence 88.

**Second correction, to this section itself: "and this crate does not implement
it" was also wrong.** `crates/physics/src/airbrake.rs`'s `evaluate` has applied
the term - with both literals, in the binary's association order - since the
physics crate was written; it was transcribed from `docs/physics/README.md`'s
pseudocode, and three separate prose claims (this section, `passive.rs`'s
header, and `airbrake.rs`'s *own* header a hundred lines above the code) said
otherwise. All three are now corrected. The lesson is cheap and worth keeping:
a claim that the crate is missing a term is a claim about the crate, and
nothing here had checked it against the crate.

The force block has since been read end to end (`0x0884ccb4`-`0x0884cf94`, see
[engine.md](../ghidra/functions/psp-pulse/engine.md)), which settles two things
this section had left open or stated loosely:

- **The sign. It accelerates.** `sp+0x10` is written fresh at `0x0884cdb8` and
  the `vadd.t` at `0x0884cf88` adds it to `craft+0x330`, with every factor
  non-negative. The crate had implemented the literal reading and flagged it
  "a guess awaiting M3"; the guess was right. Confidence **90**.
- **The gate is on `craft+0x2ec`, the *absolute* cached speed**, not on a
  signed forward speed as written above and in earlier revisions of this page.
  A **reversing** ship therefore still gets the term, pointed along `+forward`,
  which for it is a deceleration.

**Correction to the first reading of this term: the scale is `1e-5`, not `0.01`.**
There are two literals, not one - `0x3c23d70a` (`0.01`) at `0x0884ccf4` and then
`0x3a83126f` (`0.001`) at `0x0884cd78`, applied to the same vector by the
`vscl.q` at `0x0884cd98`. The capstone pass that first found this term stopped at
the first literal. The direction is `craft+0x180`, the ship's own forward axis,
so unlike every other passive term this one **does** appear in the forward
projection - it is simply zero in both captures.

It is **speed-proportional**, which is the shape being hunted - but it is
identically zero in both captures used here, because both hold `airbrake_l ==
airbrake_r == 0`. So it is **not** this discrepancy. A capture with asymmetric
airbrake input was taken for exactly that reason
(`data/traces/talons-junction-airbrake-asymmetric.csv`); it corroborates the
magnitude only weakly - consistent within a factor of two, confidence ~55,
because the capture is wall-contaminated throughout and carries an unexplained
~4.8-unit baseline deficit. That is a floor on the evidence, not a target: the
crate implements the **read** values and is deliberately not fitted to it.

> **The clean magnitude check that section asked for now exists, and the term
> survives it.** The Time Trial lap holds 1,997 wall-free intervals with the
> term active, reaching `31.5` units of force, and it fits at **`1.03` to
> `1.07`** of the value read from the two literals - raising the magnitude
> evidence from ~55 to **88**. See
> [cornering-ground-truth.md](cornering-ground-truth.md#the-forward-axis-holds-while-cornering-and-it-finally-measures-the-airbrake-drag-term).
> The instruction to implement the read values and not fit to a capture stands
> unchanged; the remaining few per cent sit inside the residual the method has.

## Where the mechanism has to be - and where it turns out not to be

By elimination, a linear-in-velocity opposing force of roughly `2.28 * fs` is
applied by something this crate either does not implement or implements with the
wrong shape. The forward projection proves it is *not* along the surface normal.
An earlier revision of this section named three candidates and called the first
"the single best-fitting hypothesis". **All three have since been read with the
Allegrex module installed, and all three are refuted.** Each is recorded with the
reading that kills it, so nobody re-opens it:

1. **The hover spring's damping is a normal projection, and it is not additive at
   all.** `Ship_UpdateHover` (`0x0884870c`) is not the spring - it is a
   twenty-instruction dispatcher that calls `Ship_HoverFourCorner` (`0x0884ae90`)
   or `Ship_HoverTwoPoint` (`0x0884a658`) and then `Ship_UpdateMagLock`
   (`0x0884ba0c`). The spring lives in `Ship_HoverTwoPoint`, and its damper is a
   *multiplier on the spring magnitude*, not a `-c * v` term:

   ```text
   pointVelocity = craft+0x190 + basis * cross(r_i, body+0x150)   ; 0884a960-0884aa10
   vn            = dot(pointVelocity, contactNormal_i)             ; 0884aa28 vdot.t
   damper        = 1 + clamp(-0.1 * vn, -1.0, 2.0) * ...           ; 0884aa88-0884ab20
   force         = springMagnitude * damper * craft+0x160          ; 0884ab34-0884ab68
   ```

   `vn` is the component of the contact point's velocity along the *contact
   normal* - the projection the hypothesis said might be missing is there, at
   `0x0884aa28`. And the force direction is `craft+0x160`, the ship's own **up**
   axis (a copy of `body+0x10`, made at `0x08849874`-`0x08849884`), so
   `dot(force, forward) == 0` exactly by orthonormality whatever the damper does.
   Confidence **90**.
2. **The inline vertical damping is a normal projection *and* is switched off
   while grounded.** Step 14 of `Ship_UpdateCraft`, at `0x08849c60`:

   ```text
   08849c6c  sub.s  f12,f13,f12    ; 1 - craft+0x2b0, the grounded fraction
   08849c78  mul.s  f12,f12,f14    ; * -0.25
   08849ca0  vdot.t S601,C110,C200 ; dot(up, velocity)
   08849ca4  vscl.t C610,C110,S601 ; * up
   08849ca8  vscl.t C610,C610,S600
   ```

   It is `up * dot(up, v) * -0.25 * (1 - grounded)`, so it is both along the ship's
   up axis - cancelling in the projection - and identically **zero** on the fully
   grounded craft this capture records. Confidence **92**. It was also a genuine
   crate bug: `crates/physics` scaled it by `1 - magLockBlend` instead, and that
   is fixed.
3. **`Ship_ApplyLateralGrip` writes only body-local `X`.** `0x08848c4c` takes
   `dot(craft+0x190, craft+0x170)` - velocity against the **right** axis - and
   `0x08848c6c`/`0x08848cbc` store the result into the `.x` slot of an otherwise
   zero vector before adding it to `body+0x110`. Two such terms, grounded and
   airborne, both `.x` only. Confidence **90**.

**With those three gone, the enumeration of the force path is complete and
contains no linear-in-velocity term at all.** Everything else in it was read in
the same pass:

| Term | Shape, at instruction level | Forward projection | Conf |
| --- | --- | --- | ---: |
| `Ship_ApplyQuadraticDrag` `0x08848e28` | `k * craft+0x2ec * velocity`, `k` from a four-way branch | `k * fs^2` | 92 |
| `Ship_ApplyRollingResistance` `0x08848f4c` | `-2 * velocity / \|velocity\|`, gated `craft+0x2ec > 0` | `-2` | 92 |
| `Ship_UpdateAirbrakes` `0x0884c9a4` | `forward * fs * \|abL-abR\| * drag * \|steer\| * 1e-5`, plus `right * fs * amount * (abR-abL)` | zero when the airbrakes are equal | 88 |
| inline gravity, step 7 | `world.y -= (normal_gravity * gravityMulAirborne * grounded + flight_gravity * (1 - grounded)) * mass` | along world `-y` only | 88 |
| hover downforce | `-track_gravity * mass * grounded * (1 - magLock) * avgNormal` | zero, along the normal | 88 |
| `Body_Integrate` `0x0884e230` damping | `velocity -= velocity * h * body+0x384`, `body+0x384 == 0.01` | `-0.01 * fs`, i.e. `0.23` | 88 |
| `Ship_UpdateMagLock` `0x0884ba0c` | rewrites `body+0x140` to `normalize(v - blend*dot(v,m)*m) * \|v\|` | **speed-preserving by construction**, and gated on `craft+0x280 != 0` | 85 |

The integrator reading is on [rigid-body.md](../ghidra/functions/psp-pulse/rigid-body.md);
the sub-stepping does **not** multiply the `0.01` damping up, because
`(1 - (dt/N)*c)^N` is `1 - dt*c` to first order.

So the missing `2.28 * fs` **is not a force term in `Ship_UpdateCraft`, and it is
not the integrator.** The cap's sign, the obvious remaining explanation for a
thrust that appears to fall with speed, is refuted a second time here in Ghidra
rather than capstone: `0x0884c780` materialises `0x3f000000` (`+0.5`),
`0x0884c790` is the `mul.s`, and `0x0884c7b0` is `add.s` in an always-executed
delay slot; the `min` at `0x0884c7f4` and the always-landing `* 2.0` at
`0x0884c93c` hold too.

**That leaves only the measurement's own premises, and the standing start is what
falsifies them.**

# Resolved: the force law was right, and both reference captures were in sustained contact

`data/traces/talons-junction-standing-start.csv` - 300 ticks, full throttle from
a standstill, with the `stun_timer` and `timer_2e0` columns the earlier captures
lack - settles the whole blocker, and does it twice over.

> **Retaken 2026-07-29, and it holds.** The original recording was lost with the
> rest of `data/traces/`, which is gitignored. It is regenerated by
> `just scripted-emu verification/scenarios/standing-start.inputs data/traces/talons-junction-standing-start.csv`
> - **the scenario is the durable artefact, the CSV is not**, and committing the
> scenario is why the loss cost an hour rather than the finding. The retake is
> not the same run (it stays clean for 186 ticks rather than 66, and hits the
> wall at 119 units/s rather than 54), and it reproduces both legs: **launch
> acceleration 32.69 on its third tick** against this page's 32.52 and the
> predicted 31.8, and a contact-friction floor never crossed in 114 contact
> ticks - see
> [contact-response.md](../ghidra/functions/psp-pulse/contact-response.md#re-measured-2026-07-29-on-an-independent-capture).

## The recovered force law reproduces the launch to `0.13` units of force

Over its first 58 usable ticks the capture accelerates from `fs = 0.56` to
`fs = 47.67`, and the law this project already implements

```text
net = fs + 2 * accelcap - 2.0 - 0.005 * fs^2
```

fits it with **rms `0.127` and residuals bounded by `-0.267 .. +0.181`**, at a
best-fit `accelcap = 16.89`. That is a 47-unit speed range, an 80-fold change in
`fs`, and a two-decade change in the quadratic term, reproduced by a
one-parameter fit. A missing `2.28 * fs` would have to subtract `109` at the top
of that segment; the whole residual is a quarter of a unit.

**The one parameter is not free, which is what makes this a prediction rather
than a fit.** `accelcap` is a shipped handling value, and `crates/physics` loads
it off the disc: replaying the same capture with `oag-trace` accelerates the
craft from rest at `34.07` units/s^2 with the rolling resistance gated off,
i.e. `2 * accelcap` for `accelcap = 17.03`. Pinning the law to *that* value
instead of the fitted one gives **rms `0.252`, residuals `-0.487 .. -0.039`** -
a uniform quarter-unit offset rather than any speed dependence. So the free fit
and the disc agree to `0.8 %`, and the law reproduces the launch either way.
Neighbouring values do not: `16.5` and `17.5` give rms `0.79` and `1.22`, and
the residual stops being flat.

So `mass = 1`, the `* 2.0`, the `min`, `cap = 0.5 * fs + accelcap`, the `-2.0`
rolling resistance and the `-0.005` grounded drag are **all confirmed
simultaneously and end to end**, on real data, with no decompilation in the
chain. At `fs = 0.56` the measured acceleration is `32.52` against a predicted
`2 * accelcap - 2 = 31.8`, which is the zero-model-assumption thrust check
[engine.md](../ghidra/functions/psp-pulse/engine.md) asked for.

## Then the ship hits a wall, and never leaves it

| Tick | `\|v\|` | `speed / \|v\|` | residual vs the law |
| ---: | ---: | ---: | ---: |
| 52-61 | `40.0 -> 49.9` | `1.0000` | `~ +0.1` |
| **62** | `45.03` | `1.1346` | one-frame impulse, `\|v\|` drops `4.9` |
| 63-65 | `45.5 -> 47.0` | `1.0000` | `~ +0.2` |
| **66 onwards** | falls to `~21` | `1.0420 -> 1.0367` | `-50` to `-56`, for the remaining 230 ticks |

`grounded` is `1.0`, `steer` is `0.0`, both airbrakes are `0.0` and **both
`stun_timer` and `timer_2e0` are `0.0` on every one of the 300 ticks** - so the
engine's early return, which the handover kept open as a possibility, is
directly ruled out by the columns rather than argued away from a force balance.
`oag-trace show` reports the same thing independently ("stun timer: never armed;
`craft+0x2e0` never above zero").

**That the contact is a wall is an inference, and this is the evidence for it.**
`dot(velocity, right)` is `-0.01` for the whole launch, jumps to `-13.2` on tick
62 alongside the `4.9`-unit speed drop, decays to `-7.1` by tick 65, is knocked
back to `-11.5` on tick 66, and then **never returns to zero** - it is still
`-2.2` at tick 295, 230 ticks later. `dot(velocity, up)` stays under `0.14`
throughout, so it is not a floor or ceiling interaction. A sustained lateral
velocity that the lateral grip cannot null is what a craft pressed against a wall
looks like. Confidence **75**: the signature fits and nothing else in the capture
explains it, but the contact itself is not in the trace, and the stun timer
staying at zero across the impact is unexplained (`Ship_ApplyCollisionImpulse`
arms `craft+0x290` only when flag `0x10` of the pickup word is clear, and that
flag has not been read). **Nothing below depends on the attribution** - the
physics conclusion needs only that the velocity is being reduced outside the
accumulators.

## The deficit is a velocity scale, not a force, and the trace already recorded it

This is the finding, and it is why enumerating the force path could never have
found the mechanism.

`speed` is `body+0x398`, which
[rigid-body.md](../ghidra/functions/psp-pulse/rigid-body.md#body0x398-is-linear-velocity)
shows `Body_Integrate` writes as `sqrt(dot(v, v))` **from the same register it
stores as the velocity**, four instructions apart. The two are the same number by
construction. So whenever a trace samples `speed` above `|velocity|`, something
shrank `body+0x140` after the integrator ran and before the next
`Ship_UpdateCraft` read it - a change made in *velocity space*, which no force
accumulator can see and no enumeration of `Ship_UpdateCraft` can find.

Converting that per-frame speed loss into an equivalent force reproduces the
residual tick by tick, on all three captures:

| Capture | `speed / \|v\|` | `(speed - \|v\|) / dt` | residual vs the law |
| --- | --- | ---: | ---: |
| standing start, ticks 0-61 | `1.0000` | `0.00` | `+0.1` |
| standing start, ticks 90+ | `1.0369` | `47` to `50` | `-50` to `-52` |
| `talons-junction-venom-assegai` | `1.0367` | `50` to `53` | `-51` to `-55` |
| `talons-junction-steer` | `1.0334`-`1.0479` | `40` to `50` | `-42` to `-56` |

The two columns agree to a couple of units everywhere, including through the
steer capture's slow-down and recovery, where both wander together. **The
`1.0367` ratio this page recorded as an unexplained curiosity of the `speed`
column is the missing force.**

`body+0x398` having exactly one per-frame writer is what licenses that reading,
and it was checked rather than assumed: `search_instructions` over the `0x398(`
displacement finds only `Body_Init` (a zero) and `Body_Integrate` on a body base,
and an `addiu`-based scan - the one that catches VFPU accesses through a rebased
pointer, which the displacement scan misses - finds **no** `0x398` rebase
anywhere in the image.

**The loss is multiplicative, not a constant impulse, and the ratio's flatness is
what proves it.** From tick 80 to tick 200 of the standing start `|v|` falls from
`33.1` to `21.2`, a 36 % drop, while the ratio moves only from `1.0400` to
`1.0371`. A fixed per-frame velocity *decrement* would have driven the ratio from
`1.0400` to `1.0625` over that range. So the per-frame loss scales with speed,
which is exactly why it reads as a force linear in `fs`.

It also explains why the deficit looked linear in speed. A *multiplicative* loss
of `3.67 %` per `16.5 ms` frame is `0.0367 * |v| / 0.0165 = 2.22 * |v|` of
equivalent force - and the fitted coefficient on this page is `2.280`. **The
linear shape and its magnitude were both correct; only the assumption that it was
a force was wrong.**

## What this means for the two captures this page was built on

Both of them are in the post-contact regime for **their entire length** - the
ratio never drops to `1.0000` in either file. They are recordings of a ship
scraping along a wall at a speed the wall response, not the force law, is
setting. Everything measured from them about *resistance* is a property of the
collision path.

That does not retract the parts of this page that do not depend on it: the
`mass = 1` reading, the tick-8 speed-pad identification, the stun-gate negative,
and the thrust-side confirmations all stand, and the standing start independently
confirms the last of those. What is retracted is the conclusion - **there is no
missing resistance term, the target shape `2.28 * fs` should not be implemented
as a force, and `crates/physics`'s force law needs no new term at all.**

Confidence **90**. It rests on three independent legs that agree: a
one-parameter fit at rms `0.13` over a 47-unit speed range, a recorded column
whose derivative matches the residual on three captures, and an instruction-level
reading of where that column comes from.

# Closed: the mechanism is contact friction, and the coefficient is `0.035`

The two steps this page left open - find the writer, then match `wall.rs` to it -
are both done, and the answer is a number read out of the binary rather than
fitted to this page.

`Body_ResolveContact` (`0x0884e968`) applies, per contact, an impulse

```text
j * n  -  contact->friction * (v_point - n * dot(v_point, n))
```

and `Body_ApplyImpulseAtPoint` (`0x0884d64c`) turns that into `velocity +=
impulse * invMass` with `invMass == 1`. The tangential half is therefore a flat
multiplicative loss on the tangential velocity, once per frame per contact, in
velocity space - **exactly the shape this page measured and could not attribute**.
`contact->friction` is `contact+0x34`, filled by `Collision_AddContact`
(`0x08816864`) as the average of the two colliders' `collider+0x64`, forced to
zero if either is negative. A wall's is `0.05`; the craft's own box collider is
given `0.02` by the ship-entity constructor. **So a craft scraping a wall loses
`(0.05 + 0.02) / 2 = 3.5 %` of its tangential speed every frame.** Full evidence
in
[contact-response.md](../ghidra/functions/psp-pulse/contact-response.md).

**The measurement is a one-sided test of that prediction, which is stronger than
the asymptote match it looks like.** The normal impulse can only *add* loss on top
of the tangential one, so `3.5 %` is a floor the data must approach from above
and never cross. The standing start's contact ticks read `5.21`, `4.03`, `3.885`,
`3.797`, `3.636`, `3.577`, `3.560 %` - monotone, converging, and above `3.5 %` on
every one of its 230 contact ticks. A coefficient of `0.036` is refused by tick
295 alone.

So the `3.67 %` this page quotes as the settled value is not the coefficient; it
is the transient still an eighth of the way from the impact. The coefficient is
`0.035`, and `crates/physics/src/wall.rs` now implements the law rather than the
number: a unit test pins a pure scrape at `0.965 x` per frame, and the constants
it multiplies are the two disc literals.

## What to do next

1. ~~**Recapture a clean straight.**~~ **Done, and it went further than a
   straight.** `data/traces/talons-junction-time-trial-lap.csv` is 95.2 %
   wall-free with a longest clean run of **313** ticks, retiring this item's
   original premise that no trace held more than 60. `speed / |velocity| ==
   1.0000` is the cheap, exact test that selects them, and any future capture
   should be checked with it before being used as a reference. It is also a
   *positive* test: a capture in wall contact should read the surface's
   friction, so the ratio identifies which regime a capture is in rather than
   only flagging that it is not clean. The measurement that capture made
   possible is [cornering-ground-truth.md](cornering-ground-truth.md), which
   confirms the forward law at a slip angle and settles the lateral axis and the
   yaw accumulator.
2. **Close the `0x10` pickup flag.** The wall attribution above is capped at
   confidence 75 because the stun timer stays at `0` across the tick-62 impact.
   The gate is narrowed but not closed: the flag word is `*(entity+0x4c) + 0x1b8`
   and bit `0x10` suppresses both the impulse and the `+= 0.5`; what sets it is
   unread.
3. **Angular response.** `crates/physics` resolves contacts at the centre of mass,
   so the original's `cross(r, impulse)` torque and its contribution to the
   resolver's denominator are both absent. That is the largest remaining gap
   between this crate's contact response and the original's.

## What not to do

Do **not** set `ENGINE_OUTPUT_SCALE`, or any drag coefficient, to whatever closes
this. Every constant on the thrust side is now confirmed at instruction level in
the table above, and the drag coefficients are confirmed in both binaries. The
measurement on this page is the result; the constant that would hide it is not.
