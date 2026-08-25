# Engine, brakes, steering and pitch

Functions in `PSP_GAME/SYSDIR/BOOT.BIN` (Pulse PSP, UCUS-98712), image base
`0x08804000`. **The names here are applied**, from [names.tsv](names.tsv).

This page covers the part of the craft force law that
[physics](../../../physics/README.md) does not: how the analog stick, the
throttle and the shoulder buttons become force. It also fixes the handling
parameter block's field layout to the byte, read out of the XML loader rather
than inferred from consumers.

> **Correction, from measurement rather than reading.** Three conclusions below
> have since been refuted against captured traces; see
> [the along-track force balance](../../../physics/force-balance-ground-truth.md),
> which is the authority on the longitudinal balance.
>
> 1. **"a craft mass near 23" is wrong, and the argument was circular.** The
>    integrator mass is `1`: `Ship_InitCraft` calls `Body_SetMass` at
>    `0x0884985c` with `class+0x60` (the XML `mass` at `+0xf4`) in the delay
>    slot, and `Body_SetMass` (`0x0884d850`) stores it unscaled. Confidence 92.
> 2. **The collision-stun gate does not explain the reference capture.** The
>    finding itself stands - the early return is real - but the gate provably
>    never fires in that capture, checked by comparing implied thrust across its
>    tick bands. A gate also predicts a *constant* deficit, and the measured one
>    is speed-proportional.
> 3. **The `0.095 * v^2` target shape is wrong.** The missing resistance scales
>    as `fs^1.09` between two operating points three units/s apart; a quadratic
>    misses by 12 %, a constant by 16 %.
>
> **And the blocker itself is now resolved, which retires the framing of the
> whole "thrust-gap inverted" investigation below.** A standing-start capture
> shows this page's own force law - `T = 2 * min(throttle * amount, 0.5 * fs +
> accelcap)`, against `2.0` of rolling resistance and `0.005 * fs^2` of drag, at
> `mass = 1` - reproducing the original's forward acceleration from `fs = 0.56`
> to `fs = 47.67` with **rms `0.127` units of force**. There is no missing
> resistance of any shape. The `~52`-unit deficit in the older captures is a
> `3.67 %`-per-frame reduction of the *velocity*, applied by the collision path
> outside every accumulator on this page, and both of those captures were
> recorded with the craft in sustained wall contact from end to end. Every
> subsection below that reasons from "resistance is 12x short" or "`0.095 * v^2`"
> is superseded; they are kept because each closes a hypothesis.
>
> **A second correction, from reading rather than measurement.** That page named
> three candidate sites for the missing linear resistance - the hover damper, the
> inline vertical damping, and the lateral grip - and **all three are now refuted
> at instruction level**, with the Allegrex module installed. The force path is
> fully enumerated and contains no linear-in-velocity term. Three consequences
> land on *this* page and are folded in below where they belong:
>
> - **`Ship_UpdateHover` (`0x0884870c`) is not a force term.** It is a
>   twenty-instruction dispatcher. See [step 8](#the-frame-ship_updatecraft).
> - **The inline vertical damping at step 14 is scaled by `1 - grounded`**, so it
>   is zero on a grounded craft, and it is a normal projection.
> - **`craft+0x2ec` is `|dot(velocity, forward)|`** - `vabs.s` at `0x08849930` -
>   which makes `Ship_ApplyQuadraticDrag`'s reversing branch unreachable.
>
> What is **confirmed** by that page, at instruction level: the `0.001` load
> scale on `Engine.amount`, `cap = 0.5 * speed + accelcap` (`add.s` at
> `0x0884c7b0`, so no sign error), the `min`, and the final `* 2.0`. **The `T ~=
> 58` recomputation is correct.** The error is on the resistance side, exactly as
> the "thrust-gap inverted" section concluded - just not in the shape it
> proposed.

## The parameter block, read from the loader

`Handling_ParseStats` (`0x0883a2f0`) walks `handlingstats.xml` and dispatches one
parser per element. Each parser compares an attribute name and stores the parsed
float at a fixed offset, so **the name-to-offset mapping is a direct read, not an
inference**. The class index is a file-scope global (`g_handling_parse_class`,
`0x08b36bfc`), set when a `<Class name="...">` attribute matches one of the four
speed-class names; the stride is `class * 0x80`.

| Block | Parser | Offsets (`+ class * 0x80`) |
| --- | --- | --- |
| `Antigrav` | `HandlingXml_ParseAntigrav` (`0x08839278`) | `ride_height 0x94`, `rebound 0x98`, `landing_rebound 0x9c`, `rebound_jump_time 0xa0`, `grip_ground 0xa4`, `grip_air 0xa8` |
| `Brakes` | `HandlingXml_ParseBrakes` (`0x0883962c`) | `gain 0xac`, `amount 0xb0`, `falloff 0xb4` |
| `Engine` | `HandlingXml_ParseEngine` (`0x0883945c`) | `gain 0xb8`, `amount 0xbc`, `falloff 0xc0`, `accelcap 0xc4`, `turbo 0xc8` |
| `Turning` | `HandlingXml_ParseTurning` (`0x088398e0`) | `gain 0xcc`, `amount 0xd0`, `falloff 0xd4` |
| `Airbrake` | `HandlingXml_ParseAirbrake` (`0x08839a04`) | `gain 0xd8`, `falloff 0xdc`, `amount 0xe0`, `turn 0xe4`, `drag 0xe8`, `slidegrip 0xec`, `sideshift 0xf0` |
| `Physical` | `HandlingXml_ParsePhysical` (`0x08838f50`) | `mass 0xf4`, `normal_gravity 0xf8`, `flight_gravity 0xfc`, `track_gravity 0x100` |
| `pitch` | `HandlingXml_ParsePitch` (`0x0883977c`) | `pitch_air 0x104`, `pitch_ground 0x108`, `pitch_damping 0x10c`, `antigrav_height_adjust 0x110` |

Confidence **90**.

Every one of those parsers reads its values with `Xml_AttributeAsFloat`
(`0x0895379c`), which **cannot read exponent notation** - see
[xml-reader.md](xml-reader.md). Nothing shipped uses it, but a tool that
regenerates `handlingstats.xml` must emit plain decimal.

**The arithmetic closes exactly.** 6 + 3 + 5 + 3 + 7 + 4 + 4 = 32 fields; 32
floats is `0x80` bytes; the observed per-class stride is `0x80`; the first field
sits at `0x94` and the last at `0x110`, so the block runs `0x94 .. 0x114` and
class *n* + 1 starts precisely where class *n* ends. **There is no gap and no
overlap anywhere in the table, and no offset in the range is unaccounted for.**
Independently, `Ship_UpdateCraft` caches `stats + class * 0x80 + 0x94` in a
member and every consumer reads through that pointer, which is only consistent
if `0x94` is the block base.

Not higher than 90 because nothing here has been observed at runtime: the
mapping shows the loader and the consumers agree on a layout, not that a real
file produces the values a race then uses.

Two of the four unlabelled string references were read out of memory rather than
inferred: `0x08a7b164` is `"gain"`, `0x08a7b084` is `"mass"`, `0x08a7b1d0` is
`"drag"`, `0x08a7b1d8` is `"turn"`. So every one of the 32 rows above is a
name comparison the loader actually performs.

Note that memory order is **not** XML document order: `Antigrav` and `Brakes`
precede `Engine`, and `Airbrake` follows `Turning`. Nor is field order within a
block consistent between blocks: `Engine`, `Brakes` and `Turning` are
`gain, amount, falloff`, but `Airbrake` is `gain, falloff, amount`. A decoder
that assumed either regularity would place five fields wrongly.

### Four parameters are pre-scaled at load

The parsers do not store four of the values verbatim. This answers the **Units**
open question in [handling stats](../../../formats/handling-stats.md), which that
page could not settle from the XML alone:

| Field | Stored as | Site |
| --- | --- | --- |
| `Engine.amount` | `xml * 0.001` | `0x0883949c` region, in `HandlingXml_ParseEngine` |
| `Brakes.amount` | `xml * -0.01` | in `HandlingXml_ParseBrakes` |
| `Airbrake.amount` | `xml * 0.0001` | in `HandlingXml_ParseAirbrake` |
| `Airbrake.slidegrip` | `xml * 0.0001` | in `HandlingXml_ParseAirbrake` |

Confidence **88**. `Turning.amount` was later read the same way and is **not** on
this list - `HandlingXml_ParseTurning` stores it verbatim, confidence 95, see
[Steering](#steering). That is worth stating explicitly because it removes the
obvious explanation for the 22x yaw discrepancy recorded there.

Two consequences worth stating outright:

- **`Brakes.amount` is negative in memory.** The brake term applies force along
  `+normalize(velocity)`, so the sign lives in the parameter, not in the force
  law. A reimplementation that negated at the use site as well would accelerate
  under braking.
- **`slidegrip` is confirmed as "percent of grip retained".** With the `1e-4`
  scaling, an XML value on 0..100 becomes 0..0.01, and the grip term's
  `(0.01 - slidegrip)` therefore runs from `0.01` at `slidegrip = 0` down to
  exactly `0` at `slidegrip = 100`. Combined with the `max(L, R)` factor
  (airbrake state is also 0..100), the coefficient reaches exactly 0 at full
  airbrake and `slidegrip = 0`, and stays at `-1` for `slidegrip = 100`. This
  was read as an interpretation in [physics](../../../physics/README.md) at
  confidence 86; the load-time factor makes it arithmetic. Confidence **90**.

`AirbrakeGraphics` lives on the stats base rather than in a class block:
`up_speed 0x70`, `down_speed 0x74` (`HandlingXml_ParseAirbrakeGraphics`,
`0x08839c68`), driving the two graphics-only deflection states at `craft+0x2d8`
and `craft+0x2dc`. Confidence **80**. `Ship_UpdatePitch` also reads
`stats_base + 0x90`, a per-team scalar outside every class block. **Its element is
`<Misc weight_distribution>`**, identified from the PS2 build's
`HandlingXml_ParseMisc` - see the Cross-platform section below and
[handling-stats.md](../../../formats/handling-stats.md#misc-sits-on-the-stats-base-and-its-offsets-are-ps2-confirmed).

## The frame: `Ship_UpdateCraft`

`Ship_UpdateCraft` (`0x08849618`) is the whole per-frame craft update, and every
force term below is reached from it. Signature, from register use:

```c
void Ship_UpdateCraft(float dt /* f12 */, Craft *craft /* a0 */, RigidBody *body /* a1 */);
```

**The register assignment above is the corrected one.** Reading register use
statically put the craft in `a1` and the body in `a2`; breaking on the function
in a running race says otherwise, and says so unambiguously:

| At entry | Value | What settles it |
| --- | --- | --- |
| `a0` | `0x09a0dc40` | `+0x1c8` reads `0.0168` (a frame time), `+0x2b0` reads `1.0` (grounded), `+0x2b8` reads `100.0` when and only when thrust is held |
| `a1` | `0x09a0dfb0` | **`a0+0x1cc` holds exactly this value**, and `craft+0x1cc` is the body pointer per this page's own description of the stash |
| `a2` | `0x08849618` | the function's own address, left there by the vtable dispatch at the call site |
| `f12` | `0.016523` | dt, as documented |

The same offsets read off `a1` are nonsense (`+0x1c8` as `-162.689`, `+0x2b8` as
`9.2e-33`). Confidence **95**: three independent fields agree, and the
`a0+0x1cc == a1` identity is an equality rather than a plausibility.

### The call site, and what dispatches it

This page previously capped its own confidence at 82 because the call site was
unknown, and the [roadmap](../../../overview/roadmap.md) tracks that as an open
question. It is `0x0884ff70`, inside the function beginning at `0x0884f70c`
(size `0xba8`), recovered by reading `ra` at a breakpoint on entry - stable
across every hit.

```
0884ff48  addiu s2,s2,-0x8      ; walk an entity array backwards, stride 8
0884ff4c  lw    a0,0x3C(s2)
0884ff54  sw    zero,0x370(a0)  ; clear a field on the entity before updating it
0884ff58  lw    a0,0x40(s2)     ; the object
0884ff5c  lw    a1,0x3C(s2)     ; the body
0884ff60  lw    a2,0x38(a0)     ; its vtable
0884ff64  addiu a2,a2,0x70      ; slot 0x70
0884ff68  lh    a3,0x0(a2)      ; this-adjustment, 16-bit
0884ff6c  lw    a2,0x4(a2)      ; function pointer at +4 of the entry
0884ff70  jalr  a2
0884ff74  addu  a0,a0,a3        ; delay slot: apply the adjustment to `this`
0884ff78  bne   s1,zero,0x0884FF34
```

So the dispatch is an ordinary C++ virtual call through a vtable at
`object+0x38`, whose entries are 8 bytes of `{ i16 this-adjust, void *fn }` -
the layout a compiler emits when a base is at a non-zero offset. The `craft`
that arrives in `a0` is therefore the *adjusted* pointer, `object + adjustment`,
which is why it is a craft and not the entity. The loop around it steps an array
backwards in strides of 8 with the object at `+0x40` and the body at `+0x3c`, so
`Ship_UpdateCraft` is one entry in a per-entity update pass rather than something
the race code calls directly.

Confidence **95** for the call site and the dispatch shape, both runtime-observed
in disassembly. The enclosing function is **not renamed**: what list it walks and
what else is in that list are not established, and ADR-0005 says a guess dressed
as a name stops other people from looking.

### The rigid body, measured rather than read

The body's own layout was recovered by dumping `0x400` bytes at each of six
consecutive frames and keeping the fields that changed. It is what
[`psp-trace.py`](../../../../scripts/psp-trace.py) records.

| Offset | Field | Why it is that |
| --- | --- | --- |
| `+0x00`, `+0x10`, `+0x20` | transform rows: right, up, forward | each is unit length to six digits and the three are mutually orthogonal; the row order matches this page's own right/up/forward convention, established independently from the raycast direction and the grip term |
| `+0x30` | position | moves smoothly, and its per-frame delta equals velocity times the frame's dt |
| `+0xc0`, `+0xd0`, `+0xe0` | the same rotation transposed | element for element the transpose of the first three rows |
| `+0x140` | linear velocity, world | integrates into `+0x30`, as above |
| `+0x374` | mass | reads `1.0`; this page already says `mass` is read from `body+0x374` |

Confidence **85** for position, velocity and the transform, which agree with each
other arithmetically.

`+0x398` was recorded here at confidence **60** as "`|velocity|` to seven digits
in one sample but about 4 % above it across a 200-tick trace, so **it is not the
speed**". **The first half of that is now settled and the second half was the
wrong inference.** `Body_Integrate` (`0x0884e230`) ends with

```text
0884e450  vmul.t C010,C300,C300
0884e458  vsqrt.s S000,S010
0884e460  sw     a1,0x398(a0)      ; |velocity|
0884e468  sv.q   C300,0x0(t3)      ; t3 == body+0x140, the same register
```

so `+0x398` **is** the speed, by construction, from the same vector it stores as
the velocity. Confidence **90**; see
[rigid-body.md](rigid-body.md#body0x398-is-linear-velocity). The 4 % is therefore
not evidence against the identity - it means something changes the velocity
between the integrator's last store and the point a trace samples it, which is
the same `1.0367` factor
[force-balance-ground-truth.md](../../../physics/force-balance-ground-truth.md)
reports on the capture's `speed` column. That column is this field. What touches
the velocity in between is still open.

### The cached speed, and its staleness, measured

Over 200 ticks of a moving ship at about 22 units/s, holding thrust:

| Claim | Test | Result |
| --- | --- | --- |
| `craft+0x2ec` is `\|dot(velocity, forward)\|` **of the previous frame** | compare against both frames, tolerance 0.01 | previous frame **199/199**, current frame 137/199 |
| the transform rows are orthonormal | row lengths over all 600 rows | `1.000000 .. 1.000001` |
| position integrates velocity | `\|Δposition\| / (\|velocity\| · dt)` | mean `1.012`, range `0.965 .. 1.039` |
| controls are on a 0..100 scale | throttle while thrust is held | `100.0`, every tick |

The first row is the interesting one. 199 of 199 against 137 of 199 is not a
tolerance question: the cached speed **is** the forward speed, and it **is** one
frame behind, which is exactly what this page derived from the frame ordering and
what makes a reimplementation that resolves contacts first diverge on every
takeoff and landing frame. Confidence **95**, runtime-measured.

The position-integration row is included because it is *not* exact: velocity
sampled at function entry is last frame's, so 1.012 rather than 1.000 is the
expected residue of the same staleness, not an error bar on the offsets.

It stashes `dt` at `craft+0x1c8` and `body` at `craft+0x1cc`, copies the body's
4x4 transform, linear velocity and position into the craft, zeroes the four
accumulators, then runs the terms in a fixed order and finally hands each
accumulator to the body.

Row conventions, established by cross-checking three independent uses (the
raycast direction, the `|dot(v, row2)|` speed scalar, and the lateral grip term's
`dot(v, row0)`): **row 0 is right, row 1 is up, row 2 is forward.** Units are
still not determined; handedness now is, and the naming of row 0 does not
survive it.

### The basis is positively oriented, and row 0 points left

Two measurements off a running race, and they constrain the coordinate question
that [physics](../../../physics/README.md) and the
[roadmap](../../../overview/roadmap.md) both carry as open.

**`cross(row0, row1) = row2` exactly, on 200 of 200 sampled ticks**, with
`dot(cross(row0, row1), row2) = 1.000000` throughout. The transform is a proper
rotation and the ordered triple is positively oriented **under the ordinary
component-wise cross product** - the same arithmetic a reimplementation uses. So
there is no component-level handedness difference between the original's basis
and ours to flip anything. Confidence **95**.

**Steering rotates the ship the other way from its own sign.** Holding left and
holding right, one after the other from clean restarts, thrust held in both:

| Held | mean `craft+0x2c0` | mean yaw about row 1 | agreement |
| --- | ---: | ---: | --- |
| left | `-96.44` | `+1.51 rad/s` | 199/199 ticks positive |
| right | `+96.55` | `-1.42 rad/s` | 0/199 ticks positive |

where yaw is `dot(cross(fwd_t, fwd_t+1), up) / dt`. The mirror symmetry in both
sign and magnitude is what rules out collision or track camber as the cause.
So the measured law is `yaw_rate = -k * steer`, `k` about `0.0155 rad/s` per unit
of steer at roughly 22 units/s. Confidence **90**.

Two things follow, and the second matters more than the first.

1. **`craft+0x2c0` is signed, left negative, and it exceeds 100** - values of
   `-108` were seen. The `0..=100` control range is the input's, not this field's,
   which is consistent with this page's own `yaw = (steer + craft+0x2e4) * amount`
   and its added steering bias.
2. **Turning left rotates forward toward `+row0`.** Read in a right-handed frame
   with the ordinary cross product - which the first measurement says is the
   right way to read it - **row 0 is the left direction, not the right one**. It
   is named `right` above from its use sites, which fix the axis but not its
   sign. A reimplementation that maps row 0 onto its own `+x = right` will steer
   backwards, and the lateral grip term, which is the other `row0` consumer, will
   push the wrong way with it.

This also does **not** support the handedness explanation offered for the two
cross-product sign contradictions on the physics page. The basis is positively
oriented under our own arithmetic, so a frame-handedness difference is not
available as the common cause. If both of those terms really do need flipping,
the sign lives downstream - in how the angular accumulators are applied to the
body, which is [an open question in its own right](../../../overview/roadmap.md)
and now has a measured constraint to satisfy: whatever the accumulator holds,
`yaw = steer * Turning.amount` written into local `.y` has to come out as
`-k * steer` about row 1.

**That paragraph now has its answer, and this measurement is one of the two legs
supporting it.** The PS2 integrator has been read at instruction level: it
advances the basis rows by `e' = e x w`, not the textbook `e' = w x e`. So the
engine's angular velocity is the negative of the physical one, the accumulators
carry that same negated sign - whichever of torque or angular acceleration they
turn out to hold, which is still open above - and the two "contradictory" terms
- the `-400` surface alignment and the hover spring's `F x r` - are **correct as
written** rather than in need of flipping. Nothing compensates them because
nothing has to.

The measurement above is what makes that reading more than a code artifact: it
is end-to-end, accumulator sign in and observed rotation sign out, so it bypasses
every intermediate transform in the integrator. `steer < 0` giving
`angularLocal.y < 0` and yet `+1.51 rad/s` about row 1 on 199 of 199 ticks *is*
`w_game = -w_physics`, provided `Turning.amount > 0` - which is untested but
plausible, since it is not one of the four fields this page shows being scaled at
load. The weathervane term corroborates it without even that caveat:
`cross(forward, velocity) * (-0.1 / -0.3)` turns the nose *toward* the direction
of travel only under the negated convention.

Read the derivation and its confidence split - 88 for the PS2 build, 80 for the
claim generalising here - in
[ps2-pulse-eu/craft-update.md](../ps2-pulse-eu/craft-update.md#the-angular-sign-convention-w_game---w_physics).
**This page's own 84 ceiling is a methodology cap on decompilation alone, and it
does not bind that result**, which rests on raw disassembly, a vendor ISA manual
and this runtime measurement together. What is being raised is specifically the
angular-accumulator sign convention. The handedness question is *not* raised by
it and is not the same question: the basis is positively oriented in both builds,
measured here and rebuilt as `row0 = row1 x row2` by the PS2 orthonormaliser.
The two were being conflated, and separating them is most of the answer.

Order of terms, all of which write only accumulators unless noted:

| Order | Function | Writes |
| ---: | --- | --- |
| 1 | `Ship_UpdateEngine` (`0x0884c5c8`) | local `.z`, local `.y` |
| 2 | `Ship_UpdateBrakes` (`0x088489d8`) | world `.x`, `.z` |
| 3 | `Ship_UpdateAirbrakes` (`0x0884c9a4`) | world `.xyz`, angular-local `.y`; `sideshift` straight to the body |
| 4 | `Ship_UpdateSteering` (`0x08848788`) | angular-local `.y` |
| 5 | `Ship_UpdatePitch` (`0x08848d08`) | angular-local `.x` |
| 6 | `Ship_ApplyQuadraticDrag` (`0x08848e28`) | world `.xyz` |
| 7 | inline gravity | world `.y` |
| 8 | `Ship_UpdateHover` (`0x0884870c`) | nothing itself - see below |
| 9 | `Ship_ApplyLateralGrip` (`0x08848b78`) | body local-force accumulator directly, `.x` only |
| 10 | inline dead branch | nothing |
| 11 | `Ship_ApplyWeathervaneTorque` (`0x08848dc4`) | angular-world `.xyz` |
| 12 | `Ship_ApplyAngularDamping` (`0x08848ed0`) | angular-local `.xyz` |
| 13 | `Ship_ApplyRollingResistance` (`0x08848f4c`) | world `.xyz` |
| 14 | inline vertical damping | world `.xyz`, and **only while airborne** |
| 15 | track-section force (`0x08848f9c`) | world `.xyz` |

**Step 8 is a dispatcher, not a force term.** `Ship_UpdateHover` is twenty
instructions and does nothing but choose a hover model and then run the magstrip:

```text
0884870c  craft+0x1c0 &= ~1                ; clear the contact flag
08848758  jal 0x0884ae90                   ; Ship_HoverFourCorner, when the mode says so
08848768  jal 0x0884a658                   ; Ship_HoverTwoPoint, otherwise
08848770  jal 0x0884ba0c                   ; Ship_UpdateMagLock, always
```

so every claim about "the hover spring" belongs to `Ship_HoverTwoPoint`
(`0x0884a658`), which is where the two-point loop, the spring, the damper and the
epilogue all live. See [collision.md](collision.md) for the names and
[force-balance-ground-truth.md](../../../physics/force-balance-ground-truth.md)
for the spring's shape read at instruction level. Confidence **92** - the
dispatcher is short enough to read in full.

**Step 14, written out**, because it is inlined and so has no page of its own:

```text
08849c60  lwc1   f12,0x2b0(s0)      ; the 0/0.5/1 grounded fraction
08849c6c  sub.s  f12,f13,f12        ; 1 - grounded
08849c78  mul.s  f12,f12,f14        ; * -0.25   (0xbe800000)
08849c94  lv.q   C110,0x10(a1)      ; a1 == craft+0x80, so this is craft+0x90 == up
08849ca0  vdot.t S601,C110,C200     ; dot(up, velocity)
08849ca4  vscl.t C610,C110,S601
08849ca8  vscl.t C610,C610,S600
08849cac  vadd.t C300,C300,C610     ; craft+0x330
```

`craft+0x2b0` is the grounded fraction, not `craft+0x280` the magstrip blend -
the two are distinct fields and this term reads the first. **So a grounded craft
gets no vertical damping at all.** Confidence **92**.

**The three body-axis copies, settled.** `Ship_UpdateCraft` copies them at
`0x08849874`-`0x088498a8`, and the mapping is not in row order:

| Craft | From | Axis |
| --- | --- | --- |
| `craft+0x160` | `body+0x10` | up (row 1) |
| `craft+0x170` | `body+0x00` | right (row 0) |
| `craft+0x180` | `body+0x20` | forward (row 2) |

Every consumer on this page is consistent with it: the hover force is along
`craft+0x160`, the lateral grip dots against `craft+0x170`, the cached speed and
the airbrake drag use `craft+0x180`. Confidence **92**. `craft+0x190` is the
linear velocity and `craft+0x1a0` the position, copied from `body+0x140` and
`body+0x30` in the same run.

Steps 1 to 6 are skipped entirely when `craft+0x2a4` (a mode or lifecycle enum)
is 4, 5, 6 or 8, or when the control-input pointer is null. Step 2 is
additionally gated on grounded and on not being in the four-corner mode. Step 9
only runs when the timer at `craft+0x2e0` has expired.

Confidence **84** for the ordering, which is read straight off one basic-block
sequence. `get_function_callers` finds **no direct caller**, so the call is
almost certainly through a vtable; "once per frame per craft" is therefore an
inference from the structure, not something located. That is the single cheapest
thing that would raise this page.

## The four accumulators

The craft holds four 16-byte accumulators, zeroed at the top of every
`Ship_UpdateCraft` and drained at the bottom into the rigid body:

| Craft | Body | Frame | Meaning |
| --- | --- | --- | --- |
| `+0x320` | `+0x110` | body-local | Force. `x` right, `y` up, `z` forward |
| `+0x330` | `+0x100` | world | Force |
| `+0x340` | `+0x120` | body-local | Angular. `x` pitch, `y` yaw, `z` roll |
| `+0x350` | `+0x130` | world | Angular |

`+0x320` is added to `body+0x110` inline with a single `vadd.q`; the other three
go through one-line helpers, `Body_AddForceWorld` (`0x0884d4c8`),
`Body_AddTorqueLocal` (`0x0884d5bc`) and `Body_AddTorqueWorld` (`0x0884d604`),
each of which is nothing but `body[off] += *v`. Confidence **82** for the force
pair, **74** for the two angular ones: the local/world distinction rests on
which frame the terms feeding each are expressed in (`Ship_UpdatePitch` feeds
`+0x340` a scalar times a body-axis component; `Ship_ApplyWeathervaneTorque`
feeds `+0x350` a cross product of two world vectors), not on reading the
integrator.

**Whether the two angular accumulators hold torque or angular acceleration is
not determined.** Nothing in any term visibly divides by an inertia, and
[physics](../../../physics/README.md) calls this `angAccelLocal`. The `AddTorque`
names are chosen for the `Subsystem_VerbNoun` scheme and should not be read as
settling the question; the integrator was not examined.

**Their sign convention is determined, though**, from the PS2 integrator plus the
steering measurement below: whichever of the two they hold, they hold it with the
sign of `-w_physics`. See
[ps2-pulse-eu/craft-update.md](../ps2-pulse-eu/craft-update.md#the-angular-sign-convention-w_game---w_physics).
The torque-versus-acceleration question does not disturb it - an inertia tensor
is positive definite and cannot invert a sign.

### Every write, by component

Enumerated from the `Ship_UpdateCraft` call tree. Component names use the row
convention above.

**Local force `+0x320`:**

- `.z += thrust` and `.y += boostLift` - `Ship_UpdateEngine`, the only two
  writes, both via `vadd.s` on single components.
- `.x += grip` - `Ship_ApplyLateralGrip`, written to `body+0x110` directly
  rather than through the craft accumulator, twice (a grounded and an airborne
  term).

**World force `+0x330`:** brakes, the airbrake lateral and slide terms, gravity,
quadratic drag, rolling resistance, the hover spring epilogue's downforce, the
vertical-damping term and the track-section force. All whole-vector `vadd.t`.

**Angular local `+0x340`:** `.y += airbrakeYaw` (`Ship_UpdateAirbrakes`),
`.y += steerYaw` (`Ship_UpdateSteering`), `.x += pitch` (`Ship_UpdatePitch`),
`.y += 30 * right.y * (1 - magLockBlend)` (hover epilogue), and
`.xyz += (-pitch_damping, -5, k) * bodyAngularVelLocal`
(`Ship_ApplyAngularDamping`, `k` is `-5` in mode 0 and `-2` otherwise).

**Angular world `+0x350`:** the surface-alignment torque (hover epilogue) and
the weathervane torque.

### `Ship_UpdateAirbrakes`' force block, read end to end

`0x0884c9a4`; the force block is `0x0884ccb4`-`0x0884cf94`, read with the
Allegrex module. Everything before it is the two airbrake ramps and their
`0..100` clamps. The block produces three terms and adds two vectors:

```text
0884ccb4  lwc1   f12,0x2ec(s0)    ; the cached speed, |dot(v, forward)|
0884ccb8  c.le.s f12,f14          ; f14 == 0.0f, set once at 0884c9cc
0884ccc0  bc1t   0x0884cf98       ; speed <= 0 skips all three terms
0884ccc8  lwc1   f12,0x2c4(s0)    ; ramped airbrake L
0884cccc  lwc1   f13,0x2c8(s0)    ; ramped airbrake R
0884ccd0  sub.s  f12,f12,f13
0884ccd4  lw     a0,0x78(s0)      ; the input snapshot
0884ccd8  lwc1   f15,0x0(a0)      ; steerX, raw
0884ccdc  abs.s  f15,f15
0884cce0  abs.s  f12,f12
0884cce4  lw     a0,0x70(s0)
0884cce8  lwc1   f16,0x54(a0)     ; Airbrake.drag, class +0xe8
0884ccec  mul.s  f12,f12,f16
0884ccf0  mul.s  f12,f12,f15
0884ccf4  lui    a0,0x3c23        ; 0x3c23d70a == 0.01f
0884cd00  mul.s  f12,f12,f13      ; slide
0884cd14  addiu  a0,s0,0x180      ; craft+0x180 == forward
0884cd1c  vscl.q C300,C500,S400   ; forward * speed
0884cd54  vscl.q C300,C500,S400   ; * slide
0884cd78  lui    a1,0x3a83        ; 0x3a83126f == 0.001f
0884cd98  vscl.q C300,C500,S400   ; * 0.001            -> sp+0x10, a fresh write
0884cdcc  addiu  a1,s0,0x170      ; craft+0x170 == right
0884cdd4  vscl.q C300,C500,S400   ; right * speed
0884ce18  vscl.q C300,C500,S400   ; * (L * Airbrake.amount)
0884ce5c  vsub.q C320,C300,C310   ; sp+0x10 -= that
0884ce94  vscl.q C300,C500,S400   ; right * speed, again
0884ced8  vscl.q C300,C500,S400   ; * (R * Airbrake.amount)
0884cf18  vadd.q C220,C200,C210   ; sp+0x10 += that
0884cf3c  lwc1   f13,0x2ec(s0)    ; the yaw term: speed * turn * (R - L) * 0.001
0884cf68  mul.s  f12,f13,f12      ; f12 still holds the 0.001f from 0884cd80
0884cf6c  swc1   f12,0x4(sp)      ; the .y lane of sp+0x0
0884cf88  vadd.t C300,C300,C600   ; craft+0x330 += sp+0x10  (world force)
0884cf8c  vadd.t C230,C230,C610   ; craft+0x340 += sp+0x0   (angular local)
```

So, with `speed` the cached `craft+0x2ec`:

```text
slide          = |L - R| * Airbrake.drag * |steerX| * 0.01
worldForce    += forward * speed * slide * 0.001
worldForce    += right * speed * Airbrake.amount * (R - L)
angularLocal.y += speed * Airbrake.turn * (R - L) * 0.001
```

Confidence **88** on the whole block, raised from the 84 the function carried
on decompilation alone.

**Both `(R - L)` terms change sign on the way into a reimplementation, and the
literal transcription is a bug.** The two conversions this page establishes
above both apply to this block, and both are a single negation:

- `right` here is `craft+0x170`, which is `body+0x00`, which is **row 0**, which
  ["points left"](#the-basis-is-positively-oriented-and-row-0-points-left). A
  crate whose own `+x` is a genuine right must write `-right * ...`.
- `angularLocal` carries the negated physical rotation (`e' = e x w`), so a
  positive `.y` here turns the nose **right**, where a right-handed
  `(right, up, forward)` frame turns it left under a positive yaw. This is the
  same conversion `Ship_UpdateSteering`'s `yaw = steer * Turning.amount` needs,
  and the measured law `yaw_rate = -k * steer` is what pins it.

So in a right-handed `(right, up, forward)` frame, **braking the left side turns
the nose left, toward the braked side, and pushes the body to the right** - the
craft rotates into the corner while its mass runs wide, which is what
`Airbrake.slidegrip` cutting lateral grip at the same time is for.

Now also measured directly rather than derived, because the derivation runs
through two conventions and either could have been misapplied:
`verification/scenarios/airbrake-left-only.inputs` holds `ltrigger` alone with
`steer == 0` for 90 ticks, so the yaw term above is the only drive in the whole
force list that can rotate the craft. `forward` rotates toward `+row0` at
`+0.36 rad/s`, **33 of 33 wall-free ticks positive** (`speed/|velocity|` reads
`1.0000` to tick 147), against `+0.005` on the pre-brake straight and `-0.07`
after the release. Confidence **90**.

`crates/physics/src/airbrake.rs` had transcribed `(R - L)` literally and so ran
both terms backwards from `5ad69f3`, where the steering sign was fixed in
`Ship_UpdateSteering`'s counterpart alone, until Task #34. The reason it
survived a scenario written to exercise the term is worth having here: the
committed `airbrake-asymmetric.inputs` holds brake and steering on the *same*
side by design, and the steering drive is several times the larger, so an
inverted airbrake yaw still curves the run the way the stick asks and shows up
only as a wrong rate. **A scenario that mirrors two inputs to cancel a sign
convention cannot then be used to check one.**

> **All three terms are now runtime-confirmed**, on the Time Trial lap's
> wall-free intervals, against the values read above and with no fitting on the
> instruction side: the forward `drag` term at `1.03`-`1.07`, the lateral
> `amount` term at `0.9565 +/- 0.009`, and the yaw `turn` term at
> `1.0042 +/- 0.0016`. Leaving the yaw term out inflates the fitted steering
> drive by 56 %, so it is not a rounding-error term. Evidence and method in
> [cornering-ground-truth.md](../../../physics/cornering-ground-truth.md).
> The measurement also confirms the fourth bullet below directly: pairing the
> ramp states with the wrong tick is what a naive reading of a capture does, and
> it is worth a factor of `1.68` on the steering law.

Four details are worth having explicitly, each of which corrects or settles
something written elsewhere:

- **The `drag` term's scale is `1e-5`, from two literals.** `0.01` at
  `0x0884ccf4` and `0.001` at `0x0884cd78`, applied to the same vector. The
  capstone pass that first found the term recorded only the first.
- **The `drag` term accelerates.** `sp+0x10` is a *fresh* write at `0x0884cdb8`
  - nothing is accumulated into it beforehand - every factor is non-negative,
  and `0x0884cf88` **adds** it to the world force accumulator. This had been
  recorded in `crates/physics` as an unresolved sign ("a guess awaiting M3");
  the literal reading was right. Confidence **90** on the sign specifically,
  since the chain was read including the accumulate.
- **The gate is on the absolute cached speed.** `docs/physics/`'s pages wrote
  it as "gated on `forwardSpeed > 0`", but `craft+0x2ec` is the `vabs.s`-ed dot
  product, so a **reversing** ship is not excluded: it gets the same
  `+forward` push, which for it is a deceleration.
- **`steerX` is the raw input, `L`/`R` are the ramped states.** The term mixes
  `craft+0x78 + 0x0` (the input snapshot) with `craft+0x2c4`/`+0x2c8` (this
  function's own ramp output). Unifying the two is a natural-looking tidy-up
  that would diverge; `crates/physics/src/airbrake.rs` pins it with a test.

Also visible here and consistent with the accumulator survey below: the whole
block writes `.xyz` of the world force and only `.y` of the angular-local
accumulator - `0x0884cf6c` stores a single word into the `.y` lane of a quad
initialised at `0x0884c9b4` from the constant at `0x08a90a00`, which reads
`00 00 00 00 | 00 00 00 00 | 00 00 00 00 | 00 00 80 3f`, i.e. `(0, 0, 0, 1)`.
The `vadd.t` at `0x0884cf8c` then takes only `.xyz`, so `.x` and `.z` are
provably zero rather than merely unwritten. That is the instruction-level form
of "airbrakes produce no direct roll torque".

### The roll negative, stated precisely

[physics](../../../physics/README.md) says "airbrakes produce no direct roll
torque. No Z component is ever written." The first sentence holds; **the second
is too strong and should be narrowed when that page is next edited**:

- **No control input writes an angular Z component.** Not the engine, not the
  brakes, not the steering, not the airbrakes, not the pitch axis. Roll is never
  commanded.
- Angular Z *is* written, in exactly two places: `Ship_ApplyAngularDamping`
  (`-2.0` or `-5.0` times the body's local angular velocity `z`) and, via the
  world accumulator, the surface-alignment torque. Both are passive.

Confidence **80** on the narrowed negative. It rests on reading every writer in
the `Ship_UpdateCraft` call tree, and it is weaker than it could be because
`Ship_HoverFourCorner` (`0x0884ae90`) and `Ship_UpdateMagLock` (`0x0884ba0c`)
were not read line by line. A mechanical check partly covers them:
`Ship_HoverFourCorner`'s only accumulator base address is `craft+0x340`, so it
touches no other accumulator. The equivalent scan of `Ship_UpdateMagLock`
truncated, so it is **not** covered, and any Z write there would be missed.

## Engine

`Ship_UpdateEngine` (`0x0884c5c8`). Reads `controls->thrust` at
`*(craft+0x78) + 0x04`; `grounded` is the 0/0.5/1 fraction at `craft+0x2b0`;
`speed` is `|dot(velocity, forward)|`, cached at `craft+0x2ec`.

```
throttle = controls.thrust                  // NOT ramped, see below
T = throttle * Engine.amount                // amount already * 1e-3 at load
T = T * grounded + T * 0.2 * (1 - grounded) // 20 % thrust with no contact

if (normal mode) {
    cap = 0.5 * speed + Engine.accelcap
    if (throttle > 100.0)  cap = throttle * 0.01 * cap
    if (flags & 0x0008)    cap = 1e10                   // uncapped
    T = min(T, cap)
} else {                                                // four-corner mode = Zone
    if ((flags & 1) && !(flags & 2))
        T = g_autospeed_base + g_autospeed_step * (float)(uint32)craft+0x28c
    else
        T = 0.0                                         // branch-likely delay slot
}

if (flags & 0x0004)  T *= craft+0x2a0                   // the speed-up pickup, 1.2
lift = 0
if ((flags & 0x0200) || (flags & 0x0400)) and craft+0x2a4 == 1 {
    T += Engine.turbo
    if (controls.buttons & 1)  lift = g_boost_lift * T
}
T = T * craft+0x294 * 2.0                               // start-boost mul, 1.0 when racing
if (craft+0x31c < 1.0) { T *= craft+0x31c; craft+0x31c = 1.0 }  // one-shot scale
if (flags & 0x2000) { throttle = 0; T = 0; lift = 0 }

localForce.z += T
localForce.y += lift
```

Confidence **84**.

### The two engine multipliers are recovered, and both are 1.0 while racing

Earlier revisions of this page recorded `craft+0x294` and `craft+0x2a0` as
"unrecovered - nothing was found that writes them". **Both writers have now been
read.** Neither is a hidden global scale on thrust; both are gameplay modifiers
that sit at their neutral value during ordinary racing.

#### `craft+0x294` is the start-line boost multiplier

Three independent sites write it, and every one of them writes `1.0` outside the
start-line window:

| Address | Function | Store |
| --- | --- | --- |
| `0x08849424`..`0x0884947c` | `Ship_InitCraft` (`0x08849354`) | `craft+0x294 = 0x3f800000` (`1.0`) |
| `0x08827250` | `Race_ResetCraftBoosts_q` (`0x088271b4`) | `entity->craft(+0x94)+0x294 = 1.0`, per craft, in a loop over the field |
| `0x0883ff40` | `Ship_UpdateStartBoost` (`0x0883fdec`) | `craft+0x294 = 1.0` once the boost window has elapsed |

`Ship_InitCraft` is unambiguously the craft constructor: alongside `+0x294` it
initialises `+0x1c0` (the flag word) to 0, `+0x2a4` to 1, `+0x2b0` (grounded
fraction) to 0, `+0x2b8` (throttle state) to 0, `+0x2fc` to `0.5`, `+0x2b4` to
`10.0` and `+0x31c` (the one-shot scale) to `1.0` - the same offsets this page
reads elsewhere. It is called from `0x08840c74`.

`Ship_UpdateStartBoost` is where the non-1.0 values come from. Its shape:

```
if (craft+0x2a4 != 1) { player+0x888 = 0; return }   // timer reset
t = player+0x888                                     // seconds since the start
if (t < g_boost_overallDuration + 0.5) {
    if (t < g_boost_overallDuration) {
        switch (player+0x36c) {                      // the start-grade, 0..3
        case 0: craft+0x294 = g_boost_normalMul
        case 1: craft+0x294 = g_boost_stallMul
        case 2: craft+0x294 = g_boost_boostMul
        case 3: craft+0x294 = g_boost_boostMul * perClassTable[...]
        }
    } else {
        craft+0x294 = 1.0                            // window over
    }
    player+0x888 = t + dt
}
```

The four globals are XML-loaded, not literals. `Xml_ReadBoostSettings`
(`0x088390b4`) fills them by attribute name, which is what pins their meaning:

| Global | Attribute |
| --- | --- |
| `0x08ab0d70` | `windowStart` |
| `0x08ab0d74` | `windowEnd` |
| `0x08ab0d78` | `stallEnd` |
| `0x08ab0d7c` | `overallDuration` |
| `0x08ab0d80` | `stallMul` |
| `0x08ab0d84` | `normalMul` |
| `0x08ab0d88` | `boostMul` |

The numeric values live in a disc XML file, not in the executable - all four
addresses read as zero in the image. That is not an unrecovered code gap; it is
data, and it also means that with nothing supplying `overallDuration` the window
test falls straight through to the `= 1.0` branch.

**So `craft+0x294 == 1.0` for the whole of a normal lap.** Confidence **88**:
three writers agree, the constructor identification is corroborated by seven
other offsets this page already reads, and the XML attribute names are literal
strings compared by `Xml_AttributeNameIs`.

#### `craft+0x2a0` is the speed-up pickup, and it is 1.2

`0x0883b3b8` (a per-frame player/HUD update) is the only writer of `craft+0x2a0`
that uses a craft base, and it writes exactly two values, in the same branch that
sets and clears the `0x0004` flag that gates the read:

```
if ((pickup->0x1b8 & 0x800) == 0) {          // no speed-up pickup held
    craft+0x2a0  = 0
    craft+0x1c0 &= ~0x0004
} else {
    craft+0x1c0 |= 0x0004
    craft+0x2a0  = 0x3f99999a                // 1.2f
}
```

The flag and the value are written together, so the `0` case is never read:
`Ship_UpdateEngine` only loads `+0x2a0` when bit `0x0004` is set, and that bit is
set only on the branch that stores `1.2`. **The multiplier is a flat +20 % on
thrust while a speed-up pickup is active**, and it contributes nothing at all
otherwise. Confidence **85** - the constant is read straight out of the
instruction stream; what is slightly weaker is the identification of the `0x800`
pickup-flag bit as "speed-up" specifically, which comes from the surrounding
code (it also feeds `FUN_08848664` with the pickup's `+0x148` timer and forces
the HUD icon id to 6) rather than from a string.

#### What this closes, and what it does not

The measurement recorded on `ENGINE_OUTPUT_SCALE` in
`crates/physics/src/engine.rs` implies the original's thrust is about **17x**
smaller than this crate computes. **It is not these two multipliers.** Both are
`>= 1.0` in the branch that a Time Trial lap with no pickups takes, so no
combination of them can produce a factor below one, let alone `0.084`.

The enumeration behind that negative: `search_instructions` over every store with
a literal `0x294(` or `0x2a0(` displacement, plus the same over `0x290(` to catch
a `sv.q` covering `0x290`-`0x29c` as a quad. Every `sv.q` at `0x290` in the image
is stack-relative (`0x290(sp)`); no vector store aliases these fields off a craft
base. What that scan cannot see is a store through a rebased pointer, the way
`Ship_UpdateEngine` itself writes its accumulator via `addiu a0,a0,0x320`, so the
negative is confidence **80** rather than higher.

Where the 17x has to live instead, by elimination on the force balance already
recorded in `engine.rs`. **The three subsections below are the investigation in
the order it happened, and the last one overturns the premise of the first two**:
the shipped values turn out to be right, and the error is on the resistance side
of the balance rather than the thrust side. They are kept because each closes a
hypothesis that would otherwise be re-opened.

A steady 24 units/s needs `T ~= 4.88`, so
`min(throttle * Engine.amount, cap) ~= 2.44` before the `* 2.0`. But
`cap = 0.5 * 23.35 + Engine.accelcap = 11.68 + accelcap`, which cannot be as low
as `2.44` unless `accelcap` is about `-9.2`. So either

- `Engine.amount` as loaded is about `0.0244` where this crate has about `0.418`
  - a **17x error in the value this crate ends up with**, not in the force law; or
- `Engine.accelcap` is *negative* for Assegai/Venom and the cap arm binds after
  all.

#### The second branch is refuted by the force law itself

**A negative `accelcap` cannot be what ships**, and the argument needs no data.
The cap is `0.5 * |forwardSpeed| + accelcap` and the selection is a plain `min`,
confirmed in the disassembly at `0x0884c7f4`:

```
0884c7f0: lwc1 f15,0x0(sp)      # T
0884c7f4: c.lt.s f12,f15        # cap < T ?
0884c7fc: bc1f 0x0884c808
0884c804: _swc1 f12,0x0(sp)     # T = cap
```

There is no clamp at zero anywhere between that `min` and the accumulate. So
with `accelcap = -9.2`, a ship at a standstill on full throttle gets
`cap = -9.2`, `min` selects it over any non-negative `throttle * amount`, and the
accumulated thrust is `-18.4` - **the engine pushes the craft backwards off the
start line, and keeps doing so until the forward speed passes 18.4 units/s**,
which it cannot reach because nothing is pushing it forwards. The `throttle >
100` rescale does not rescue it either: it multiplies the cap by a factor above
one, which makes a negative cap more negative, not less.

That leaves **one surviving hypothesis**: the engine `amount` this crate ends up
using is about 17x larger than the original's.

#### And it is not the load-time scaling

`HandlingXml_ParseEngine` (`0x0883945c`) stores `amount` at
`class * 0x80 + 0xbc` and then multiplies that word in place by exactly
`0.001`; `accelcap` (`+0xc4`), `gain`, `falloff` and `turbo` are stored verbatim
with no scaling at all. `crates/gameplay/src/handling.rs` applies
`ENGINE_AMOUNT_SCALE = 0.001` to the same field and nothing else, so **the two
agree exactly** and the scaling is not the 17x. Confidence **90**.

#### And the shipped values settle it: the thrust path is correct

The Assegai `Engine` block has since been read off a real disc, in both the PSP
and the PS2 asset sets, which agree with each other. **The values are not
recorded here**, per [handling-stats.md](../../../formats/handling-stats.md)'s
standing decision to keep shipped design data out of this repository, but three
things follow from them:

- `accelcap` is **positive**, confirming the refutation above empirically as well
  as structurally.
- `amount` is written as a **plain integer**, so the exponent-blind
  `Xml_AttributeAsFloat` has nothing to misread and that mechanism does not apply.
- `raw * 0.001` reproduces **exactly** what `crates/physics`'s
  `params::Engine::amount` already holds, so the two loaders agree on the value as
  well as on the scale.

Substituting the real numbers into the force law: at the recorded speed the **cap
arm binds**, and the original's own `Ship_UpdateEngine` puts about **58** into the
local force accumulator - which is the same figure this crate computes.

**That inverts the whole investigation.** The recording is steady or very slightly
decelerating at that speed, so the original's *total resistance* there is also
about 58. This crate's resistance is `0.005 * 24^2 + 2.0 = 4.88`. The missing
factor of about **12 is on the resistance side**, and every term in
`Ship_UpdateEngine` - the `amount`, the `accelcap`, the `* craft+0x294`, the
`* 2.0` - is now positively confirmed rather than merely unrefuted.

A precise target: a grounded quadratic coefficient of `0.1` rather than `0.005`
puts the equilibrium at **23.6 units/s** (`T = 57.60` against `R = 57.70`), the
exact floor of the recorded band. The coefficient itself is not the answer - it is
confirmed as `-0.005` grounded in *both* binaries, delay slots included, with
`-0.1` sitting on the `forwardSpeed < -0.2` reversing branch in both - so what the
coincidence pins is the *magnitude* to look for: something contributing roughly
`0.095 * v^2` of opposing force to a grounded craft.

#### The track-section force is the speed-pad boost, and it is the wrong sign

`FUN_08848f9c` was the obvious candidate - the one world-force writer both
unimplemented in `crates/physics` and track-dependent, which the reference
capture is. It has now been read, and it is **`Ship_ApplySpeedupPad`**:

```
q = FUN_08887144(g_world, craft+0x1a0, &section, ...)   // locate by position
if (q == 0) { craft+0x1d0 = 0; craft+0x2d0 += dt }
else {
    craft+0x2d0 = 0
    craft+0x298 = g_speeduppad_time[class]              // 0x08b36bd0
    craft+0x29c = g_speeduppad_amount[class]            // 0x08b36bc0
    craft+0x1b0..0x1bc = section->direction             // section + 0x20
    if (section changed) { ...lap and sector counting... }
}
if (craft+0x298 > 0) {
    craft+0x298 -= dt
    f   = (craft+0x298 > 0.1) ? craft+0x29c * 10.0 * craft+0x298 : craft+0x29c
    dir = craft+0x1b0
    if (controls->0x24 & 1)  dir += craft+0x160 * g_speedpad_jump   // 0x08b36bec
    if (craft+0x2cc < 1.0)   f *= craft+0x2cc
    worldForce += dir * f
}
```

**The two tables are now read**, and they settle the sign. Both are filled by
`Xml_ReadGlobalSettings` (`0x0883a970`) from

```xml
<GlobalClass name="<speedclass>">
    <SpeedupPads amount="..." time="..."/>
    <WeaponPad refresh_time="..." elimination_refresh_time="..."/>
    <GravityMul airborne="..."/>
</GlobalClass>
```

one entry per speed class, indexed by `g_handling_parse_class`: `0x08b36bc0` is
`amount` (the magnitude, into `craft+0x29c`) and `0x08b36bd0` is `time` (the
duration, into `craft+0x298`). The element name string is at `0x08a7b41c` and the
`time` attribute name at `0x08a7b428`; both were read out of memory rather than
inferred. Confidence **85**.

So the term applies a force **along** the section direction for `time` seconds
after touching a pad, ramping down as the timer expires. It is a boost. **It
cannot be the missing resistance**, and implementing it would move this crate's
speed the wrong way. The lead is closed.

The tables are `.bss` and hold nothing in the image, so the per-class numbers are
shipped data and are not recorded here.

##### Two corrections to the pseudocode above

Both were found by reading the trigger side; see
[`pads.md`](pads.md), which supersedes them here.

**`FUN_08887144` does not return a track "section", and `+0x20` is not a
"direction" field.** It returns the **matrix** of the pad that was hit, at node
`+0x30`, in the row-major translation-in-row-3 layout every `.vex` matrix uses.
So `+0x20` is floats 8 to 10 - **row 2**, the pad's local `+Z` axis. The boost
pushes along the pad's own forward direction, which is why the reading still
works out to "along the track": the pads are authored aligned with it. The
function is now `Pads_TestCraft` (`0x08887144`, confidence 90), and the
containment it delegates to is `Pad_ContainsPoint` (`0x088866bc`, 85).

**The `if (section changed)` block is not lap and sector counting.** It bumps
pad-visit statistics: `racer+0x8d0` counts pads taken, and
`racer+0x994 + (lap-1)*0x10` counts them per lap for the up-to-20 laps at
`racer+0xac8`. Lap counting is still unfound - see
[`track.md`](../../../formats/track.md).

The one real gameplay consequence in that block is Zone mode's:

```c
if (DAT_08ab07e3 == '\0' && DAT_08b31048 == 6) DAT_08b3435c = 1;
```

That condition is this repository's already-established **Zone-mode selector**
(see [`zone-mode.md`](zone-mode.md), confidence 84), and `Zone_Update`
(`0x0882f5cc`) consumes and clears the flag:

```c
if (DAT_08b3435c != '\0') { DAT_08b3435c = '\0'; score += 100; }
```

So **a speed pad is worth 100 points in Zone mode**, and only in Zone mode.
Confidence **85**: `Zone_Update`'s other three constants - 1 point per tick, 500
per zone, 500 for a clean zone - are the ones `crates/race/src/zone.rs` already
carries from unrelated evidence, and all three agree, which is what identifies
`+0x1a1c` as the score.

**One more gate, found when the block was re-read for the implementation:** the
whole of it - both statistics counters *and* the Zone flag - is wrapped in
`if (*(int *)(racer + 0x368) == 0)`. `racer+0x368` is unidentified. So a pad taken
by whatever that flag describes scores nothing and is not counted. Confidence 90
that the gate is there (it is one branch); **0 on what it selects**, and it is not
reimplemented for that reason.

##### `craft+0x2cc` is not the contact ratio, and the scale it gates is not ported

The pseudocode's `if (craft+0x2cc < 1.0) f *= craft+0x2cc` reads naturally as
"scale the boost by how grounded the craft is", and an earlier plan for the
implementation took it that way at confidence 60. **That reading is wrong**, and
finding out cost one instruction search.

`craft+0x2cc` has exactly two writers on a craft base: `Ship_InitCraft`
(`0x08849524`) sets it once, and `Ship_UpdateEngine`'s own prologue
(`0x0884c5ec`-`0x0884c5f4`) does this every frame:

```c
craft+0x2cc = (craft+0x1c0 & 0x200) ? 0.0 : craft+0x2cc + dt;
```

That is an **accumulator of seconds**, reset while flag `0x200` is set, and an
unbounded one - it cannot be a `0..1` groundedness. What it holds is *time since
flag `0x200` was last set*, and `0x200` is one of the eleven undecoded bits of
`craft+0x1c0` (see the bottom of this page). So the term is a **one-second
fade-in after some unidentified state ends**, not a contact scale.

`crates/physics/src/engine.rs`'s `speedup_pad` therefore leaves it out rather than
guessing. **It is now the only branch of that function left out** - the
`controls->0x24 & 1` one beside it is implemented, see below. That is also what
the measurement wants: `docs/physics/cornering-ground-truth.md` records five pad
crossings at full magnitude, so `0x200` is evidently not set during ordinary
racing and the gate would be inert anyway.

##### `<Special speedpad_jump>` is recovered, applied, and is not a jump

Both unknowns in `if (controls->0x24 & 1) dir += craft+0x160 * g_speedpad_jump`
were closed live in PPSSPP, and the branch is implemented.

| Piece | What it is | How | Conf |
| --- | --- | --- | ---: |
| `craft+0x160` | the **hull up axis** | live dot `+1.000000` against `body+0x010`, `0.000000` against the right and forward rows, unit length | 90 |
| `controls->0x24 & 1` | **d-pad Up** | one-hot sweep of all twelve buttons against `*(craft+0x78)+0x24`; only Up sets bit 0 | 88 |
| `g_speedpad_jump` | `<Global><Special speedpad_jump>` | read live at `0x08b36bec`, `0.1` on both shipped discs, written by `Xml_ReadGlobalSettings` | 88 |

**The sum is not renormalised**, and that is what makes the name misleading. At
`0.1` against a unit pad direction the boost tilts by `atan(0.1)` = **5.71
degrees** and gains `sqrt(1.01)` = **0.5 %** of force, for at most the class's
`time` - `0.27/0.27/0.27/0.24 s`, also read live, which incidentally confirms the
engine flare's `0.8 s` outliving every one of them. A hover spring holding the
craft on its cushion absorbs most of a 5.7-degree tilt, which is why nothing
visibly leaps in the original.

Note also that `dir` is a **local recomputed inside the `timer > 0` block**, not
the stored `craft+0x1b0`. So the tilt tracks the button every tick and can start
or stop while a boost runs on behind the pad; baking it into the stored direction
when the pad arms the boost would be a different behaviour.

**Applied 2026-08-08** as `oag_physics::engine::speedup_pad`, with the value read
off the player's own disc through `oag_formats::handling::Special`. One judgement
call, and it is documented at the constant rather than buried: the original tests
a **bit**, and `oag_physics::ShipControls` is normalised by contract, so
`SPEEDPAD_JUMP_THRESHOLD` (`0.5`, this project's own number) stands in.
`oag_gameplay::ship_controls` maps d-pad Up to `steer_y = -1` - up on the stick
pitches the nose down in this game, and the inversion lives at the input boundary
- so the gate is on the **negative** side of that axis. The determinism reference
moved for this, deliberately; see `crates/physics/tests/determinism.rs`.

##### The two tables are filled unscaled, and the timer is re-armed every tick

Two more details the implementation needed and the pseudocode above does not make
plain, both read this pass:

- **No load-time scale.** `Xml_ReadGlobalSettings` stores
  `Xml_AttributeAsFloat`'s return value straight into `0x08b36bc0[class]` and
  `0x08b36bd0[class]` with nothing in between, unlike the four fields
  `HandlingXml_ParseEngine`/`ParseBrakes`/`ParseAirbrake` pre-multiply. The
  factor of ten in the force law is the force law's own. Confidence **90**.
- **`craft+0x298` is assigned, not triggered.** The `else` arm runs on *every*
  tick the craft is inside a pad, so the timer is refreshed to `time` for as long
  as the ship is on the pad and only starts counting down when it leaves. That is
  what makes a slow crossing boost for longer, and it is why
  `oag_physics::forces::Environment::pad_hit` is a per-tick containment answer
  rather than an entry edge. The decrement is likewise **before** the force is
  read off the timer, which lowers the peak by one `dt` of ramp.

#### Two damping terms in `Body_Integrate` that no page had recorded

Re-reading `Body_Integrate` (`0x0015d088`, PS2) for this turned up a pair of terms
missing from [physics](../../../physics/README.md) and from
[craft-update.md](../ps2-pulse-eu/craft-update.md). Inside the sub-step loop, after
both force accumulators are applied:

```c
velocity        -= velocity        * h * body+0x384;
angularVelocity -= angularVelocity * h * body+0x380;
```

`Body_Init` (`0x0015cb98`) zeroes both, and the ship-entity constructor
(`FUN_00150d20`, which is also what calls `Ship_InitCraft` and
`Handling_LoadForTeam`) sets **both to `0.01`** (`0x3c23d70a`), together with
`body+0x388 = 0.4` and `body+0x394 = 0.1`. `Body_Init` also defaults the `invMass`
at `body+0x378` to `1.0`. Confidence **80** - read off VU-macro output, and the
`0x388`/`0x394` fields' meanings are not established.

A craft therefore carries a linear velocity damping this crate does not implement.
It should be, for fidelity, but **it is not the gap**: at `0.01` against an
`invMass` of `1.0` it opposes motion with about `0.24` of equivalent force at 24
units/s, against the ~53 that is missing.

#### The engine has an early return that produces no thrust at all

**This is the resolution, and the pseudocode above is missing it.**
`Ship_UpdateEngine` does not always reach the force law. Its prologue, at
`0x0884c634`, can return having written nothing:

```
0884c634: lwc1  f14,0x290(a0)     # the stun timer
0884c638: c.le.s f14,f13          # <= 0 ?
0884c640: bc1fl 0x0884c678        # > 0 -> skip to the flag test
0884c648: lw    a1,0x1c0(a0)
0884c64c: andi  a1,a1,0x10        # (only when the timer is clear)
0884c658: bnel  a1,zero,0x0884c678
0884c660: lwc1  f14,0x2e0(a0)
0884c66c: bc1t  0x0884c6b0        # craft+0x2e0 <= 0 -> the normal path
0884c678: andi  a1,a1,0x200
0884c684: bne   a1,zero,0x0884c6b0   # flag 0x200 -> the normal path anyway
0884c6a4: swc1  f13,0x2b8(a0)        # throttleState = 0
0884c6a8: b     0x0884c99c           # return, no thrust, no lift
```

So, with flag `0x200` clear, **`craft+0x290 > 0` means zero thrust and a zeroed
throttle state.** `craft+0x2e0 > 0` does the same when flag `0x10` is clear.
Confidence **88** - read from disassembly with the branch-likely delay slots
resolved.

`craft+0x290` is a **collision stun timer**, and two functions pin that:

- `Ship_ApplyLateralGrip` (`0x08848b78`) opens with
  `if (craft+0x290 > 0) { craft+0x290 -= dt; }` **and returns** - so while the
  timer runs, the craft also gets *no lateral grip* and slides.
- `Ship_ApplyCollisionImpulse` (`0x0883f274`) is what arms it. When the entity has
  a pending impulse at `entity->0x4c + 0x110` and flag `0x10` of the pickup word is
  clear, it projects the impulse onto the ship's forward axis, applies it to the
  body, and then does `craft+0x290 += 0.5` before clearing the impulse.

**A hit costs half a second of engine and grip, and repeated contact keeps
re-arming it.** Confidence **85**.

This closes the arithmetic that the rest of this section could not. With the real
shipped `Engine` values the force law gives `T ~= 58` at the recorded speed, and no
resistance term in `Ship_UpdateCraft` - all of which are now enumerated, see below
- comes anywhere near that. A craft inside the stun window has `T = 0` instead, and
the only remaining longitudinal forces are quadratic drag and rolling resistance,
`0.005 * 24^2 + 2.0 = 4.88`, which decelerates it gently and monotonically. The
reference capture decelerates from 24.271 to 23.571 over its 200 ticks - about
`-0.21` units/s^2, i.e. `4.88 / m` for a craft mass near 23.

**So the missing "12x of resistance" is not resistance at all.** It is thrust this
crate applies and the original does not.

**Which of the two arms fired in the capture is not established.** The arithmetic
shows only that the early return was taken. `craft+0x290` fits if the run touched
a wall - though a Time Trial holding accelerate should not be scraping one, and the
stun also kills lateral grip, which would show as a slide. `craft+0x2e0` fits a
capture taken near a race start, and its arming condition has never been read.
Neither timer is in the capture, so re-reading the existing CSV cannot settle it.
**The decisive measurement is to add `craft+0x290` and `craft+0x2e0` to
`scripts/psp-trace.py`'s column list and re-capture**; that also confirms the gate
fired at all, rather than leaving this an inference from a force balance.

#### The world-force writer list is complete

Checked directly rather than from the step table: `Ship_UpdateCraft`'s full callee
set is `Ship_UpdateHover`, `Ship_UpdateEngine`, `Ship_UpdateBrakes`,
`Ship_UpdateSteering`, `Ship_UpdatePitch`, `Ship_UpdateAirbrakes`,
`Ship_ApplyQuadraticDrag`, `Ship_ApplyRollingResistance`, `Ship_ApplySpeedupPad`,
`Ship_ApplyLateralGrip`, `Ship_ApplyWeathervaneTorque`, `Ship_ApplyAngularDamping`,
the three `Body_Add*` helpers, and three that had no names:

| Address | Name | What it is |
| --- | --- | --- |
| `0x0884d850` | `Body_SetMass` | `body+0x374 = m; body+0x378 = 1/m` |
| `0x0884da24` | `Body_ClearAccumulators` | zeroes `body+0x100/0x110/0x120/0x130` |
| `0x0884da5c` | `Body_ClearVelocity` | zeroes `body+0x140` and `body+0x160` |

Three more of the body layer have since been read and named, on
[rigid-body.md](rigid-body.md): `Body_AddForceAtPoint` (`0x0884d510`, what the
hover spring applies its force through), `Body_Init` (`0x0884de5c`) and
`Body_Integrate` (`0x0884e230`). **None of them is a force term either**, which
is what makes the enumeration above complete rather than merely unrefuted: the
integrator's own linear damping is `velocity -= velocity * h * body+0x384` with
`body+0x384 == 0.01`, and sub-stepping does not multiply it up.

None of the three is a force term, and gravity and vertical damping are applied
inline. **There is no unenumerated resistance term**, which is what forced the
conclusion above.

#### A caveat on the framing

All of the above treats the recording as a speed *equilibrium*. It is a
**3.33-second window**, and over that long a slow transient toward a much higher
equilibrium is not distinguishable from a steady state by the speed range alone.
What makes it an equilibrium is the recorded *sign* - the capture is described as
very slightly decelerating, and a ship climbing toward a higher equilibrium would
be accelerating. The whole "resistance is 12x short" conclusion rests on that one
sign. It is worth re-confirming directly off the capture before anyone changes a
coefficient, because if the sign is positive the diagnosis flips to the **mass**,
which cancels out of an equilibrium but sets the entire timescale of a transient.

### Engine `gain` and `falloff` are dead

The function computes a per-second ramp of the throttle state toward the input,
in exactly the shape the airbrakes use, stores it, and then **overwrites it with
the raw input two instructions later**. Verified in disassembly rather than in
the decompiler, because this is precisely where Ghidra's branch handling could
mislead:

```
0884c700: c.lt.s f12,f13          # ramped < 0 ?
0884c708: bc1f   0x0884c714
0884c70c: swc1   f12,0x2b8(a0)    # delay slot: store the ramped value
0884c710: swc1   f13,0x2b8(a0)    # ... or zero
0884c714: lw     a1,0x78(a0)      # a1 = controls
0884c720: lwc1   f12,0x0(a1)      # f12 = controls.thrust
0884c728: swc1   f12,0x2b8(a0)    # throttleState = raw input, unconditionally
0884c72c: lwc1   f15,0x28(a2)     # Engine.amount
0884c734: mul.s  f12,f12,f15      # and the raw input is what gets multiplied
```

Both branches of the ramp land on the store at `0x0884c728`, and the value the
thrust is computed from is the one loaded at `0x0884c720`, not the ramped state.
So `Engine.gain` (`+0xb8`) and `Engine.falloff` (`+0xc0`) **are read from the XML
and have no effect on the simulation.** Confidence **85**.

Worth flagging loudly, because the obvious reimplementation is to give the engine
the same ramp the airbrakes have. It would be wrong, and the resulting throttle
lag is exactly the kind of difference that reads as "feels close enough".

### The Y-component add reads a register the function never loads

The final accumulate is:

```
0884c978: addiu a0,a0,0x320
0884c97c: lv.s  S222,0x8(a0)      # accumulator .z
0884c980: lv.s  S600,0x0(sp)      # T
0884c988: lv.s  S601,0x0(a1)      # lift
0884c98c: vadd.s S222,S222,S600
0884c990: vadd.s S221,S221,S601   # S221 was never loaded here
0884c994: sv.s  S222,0x8(a0)
0884c998: sv.s  S221,0x4(a0)
```

`S221` (the accumulator's `.y` slot in the same VFPU column) is never loaded from
memory in this function; Ghidra names it as an input register. The plausible
reading is that the caller's `vzero.q` over `craft+0x320` left that element at
0.0 and the compiler exploited it across the call boundary, making the observable
result `accumulator.y = lift`. **Confidence 65 on that explanation**, and it is
recorded because a reimplementation should write `accumulator.y = lift` rather
than try to reproduce a register carry. If the register is ever *not* zero, this
is a live bug in the original.

## Brakes

`Ship_UpdateBrakes` (`0x088489d8`). Called from `Ship_UpdateCraft` only when
`(flags & 1) != 0`, i.e. **braking does nothing in the air**, and only outside
the four-corner mode.

There is no dedicated brake axis. The brake engages when **both airbrake inputs
are positive at once**:

```
if (controls.airbrakeL > 0 && controls.airbrakeR > 0)
    brake = min(brake + Brakes.gain * dt, 100.0)
else
    brake = max(brake - Brakes.falloff * dt, 0.0)

if (brake > 0) {
    speed = |velocity|
    dir   = speed != 0 ? velocity / speed : velocity
    if (speed < 10.0)  dir *= speed * 0.1          // fade out at low speed
    worldForce += dir * Brakes.amount * brake      // amount is negative
}
```

`brake` lives at `craft+0x2bc`, next to the throttle (`+0x2b8`), steering
(`+0x2c0`) and the two airbrake states (`+0x2c4`, `+0x2c8`). Confidence **82**.

Unlike the engine, the brake's `gain`/`falloff` ramp is **live**: nothing
overwrites `craft+0x2bc` afterwards.

A VFPU target prefix (`vpfxt`) sits on the accumulate. Read as the standard
per-component selector it zeroes the `y` and `w` lanes, which would mean the
brake force is horizontal-only in whatever frame the world accumulator uses.
**Confidence 55**: the prefix encoding was not confirmed against a reference, so
this is recorded as a caution rather than a claim. It is also low-stakes -
braking only runs while grounded, where the velocity is close to horizontal, so
zeroing the world `y` lane changes very little either way.

## Steering

`Ship_UpdateSteering` (`0x08848788`). Reads the analog X axis at
`*(craft+0x78) + 0x00`.

```
target = controls.steerX
if (target > 0)
    steer = (steer < target) ? steer + Turning.gain    * dt
                             : steer - Turning.falloff * dt
else if (target < 0)
    steer = (steer > target) ? steer - Turning.gain    * dt
                             : steer + Turning.falloff * dt
else {                                        // decay to exactly zero
    if (steer > 0) { steer -= Turning.falloff * dt; if (steer < 0) steer = 0; }
    if (steer < 0) { steer += Turning.falloff * dt; if (steer > 0) steer = 0; }
}
if (craft+0x2a4 == 5 || craft+0x2a4 == 6)  steer = 0

yaw = steer * Turning.amount
if ((flags & 0x20) && *(craft+0x1c4) + 0x368 == 0)
    yaw = (steer + craft+0x2e4) * Turning.amount     // an added steering bias

if ((flags & 0x40) && *(craft+0x1c4) + 0x368 == 0) { // reversed controls
    yaw = craft+0x2e8 > 1.0 ? -yaw
                            : yaw * (1.0 - 2.0 * craft+0x2e8)
}

angularLocal.y += yaw
```

Confidence **84**. Two details worth keeping:

- **`gain` and `falloff` are asymmetric by intent, not by sign.** `gain` is the
  rate while moving *toward* a larger-magnitude target and `falloff` the rate
  while moving back toward centre, on both sides of zero. So the same pair of
  parameters gives a fast bite and a slow return, or the reverse.
- **`craft+0x2e8` is a 0-to-1 blend into reversed steering, not a boolean.** At
  0 steering is normal, at 0.5 it is dead, past 1 it is fully inverted. A
  reimplementation that treated the reverse-controls pickup as a flag would snap
  where the original ramps.

`Turning.amount` feeds body-local yaw acceleration directly, with no speed
factor. Steering authority at a standstill is therefore not zero, and any
speed-dependence must come from the damping and grip terms.

### The whole path is now read, and it disagrees with the hardware by 22x

Every link was followed to instruction level, and none of them carries a scale:

- `HandlingXml_ParseTurning` (`0x088398e0`) stores `amount` with
  `swc1 f0,0xd0(a0)` - **verbatim**. This settles the "untested but plausible"
  caveat above: `Turning.amount` is genuinely *not* one of the four pre-scaled
  fields, and it is positive. Confidence **95**, read from disassembly.
- `Xml_AttributeAsFloat` (`0x0895379c`) is a plain decimal reader - digits,
  one `.`, one `-`, no exponent - so an authored value arrives unchanged.
- `Ship_ApplyAngularDamping` (`0x08848ed0`) adds
  `(-pitch_damping, -5, -2) * localAngularVelocity` to `craft+0x340`, the **same**
  accumulator steering writes to. The `-5` is `viim_s(5); vneg_s`, and the `-2`
  becomes `-5` in the undecoded `craft+0x2a4` mode 0.
- `Body_AddTorqueLocal` (`0x0884d5bc`) forwards `craft+0x340` to `body+0x120`
  with no scaling, and `Ship_UpdateCraft` zeroes `craft+0x340` at the top of every
  frame, so it is a per-frame accumulator rather than persistent state.

**This closes the "torque or angular acceleration" question** left open by the
sign-convention section above, in the only way that matters for magnitude: it does
not matter. Steering and damping land in one accumulator, so any common factor -
the inertia tensor included - cancels at equilibrium, leaving

```text
omega_y = steer * Turning.amount / 5
```

unconditionally. Substituting the shipped Assegai `Turning` block and the
full-deflection `steer` near 96 that the captures show gives a terminal yaw rate
about **22x** the `1.5 rad/s` the hardware actually turns at - the same
`1.5 rad/s` this page measures above at confidence 90, and independently
reproducible from both captures in `data/traces/` as
`dot(cross(fwd_t, fwd_t+1), up) / dt`. (The product is not recorded here, per
[handling stats](../../../formats/handling-stats.md)'s standing decision.)

Confidence **90** on the discrepancy: the law is read from disassembly and the
measurement is end-to-end on hardware, so the two are hard to reconcile away.

**So a yaw-opposing term in `Ship_UpdateCraft` is missing from this page.** The
longitudinal axis had the same shape of gap - "resistance is 12x short" - and it
turned out not to be resistance at all but *thrust the original does not apply*,
via `Ship_UpdateEngine`'s early return on the collision-stun timer `craft+0x290`.
A yaw analogue is the first place to look, and `Ship_ApplyLateralGrip`
(`0x08848b78`) returns early on that same timer.

**It has since been found, and it is not a missing force term.** The original
damps angular *momentum* rather than angular velocity - `Ship_ApplyAngularDamping`
(`0x08848ed0`) loads `body+0x160` at `0x08848f08` - so the body's inverse inertia
tensor does not cancel out of the yaw equilibrium the way this section assumed.
That tensor is a hard-coded solid box giving `I_yy = 21.6`, and
`oag_physics::forces::YAW_INVERSE_INERTIA` is its reciprocal, `0.046296`,
replacing the fitted `YAW_DRIVE_CALIBRATION = 0.0452` that stood here. See
[rigid-body.md](rigid-body.md), and
`crates/trace/tests/yaw_authority_ground_truth.rs` for the replay against the two
captures.

**It scales the whole body-local yaw axis, not the steering term.** All three yaw
drives - the airbrake differential, steering, and the hover epilogue's
`30 * right.y` bank-to-yaw - land in `craft+0x340` together, so whatever
resistance is missing acts on all of them. Scaling only steering was tried first
and fails in play: it leaves bank-to-yaw 22x too strong *relative to* steering,
dropping the break-even camber where a corner out-turns full opposite lock to
about 14 degrees, so a banked section steers the ship for the player. The damping
is deliberately left outside the factor - scaling that too would cancel straight
back out of the equilibrium.

**The rest of this paragraph described `YAW_DRIVE_CALIBRATION`, which is retired,
and its two hedges are now settled.** It read "a calibration, not a recovered
value, confidence 80 on reproducing the captures and 45 on any mechanism", with
a lead that `1 / 0.0452` is `22.1` against a textbook box tensor of `16.6` for
the shipped Assegai hull - "the right order, 33 % out". Both are superseded:
`Body_SetBoxInertia` (`0x0884e1ac`, conf 92) is called with a **code literal**
box at a mass that is not `Physical.mass`, giving `I_yy = 21.6` exactly, and the
Time Trial lap measures `avel_y / omega_y` at `-20.948` against that `-21.6` -
`3 %`, at 90-164 units/s. The `16.6` was the right instinct applied to the wrong
hull dimensions. See
[cornering-ground-truth.md](../../../physics/cornering-ground-truth.md#pitch-and-roll-the-momentum-column-does-not-explain-the-rotation).

**And the accumulator law above is now confirmed at runtime, exactly.** Fitting
`d(avel_y)/dt` against all five writers of the `.y` lane over 2,845 wall-free
intervals gives `steer * Turning.amount` at **`0.9998 +/- 0.0013`** and the
angular damping at **`-5.0387 +/- 0.0097`**, rms `3.05` on a signal of `129.6`.
So `Turning.amount` really is consumed verbatim, the damping really is `-5`, and
**nothing is missing from the accumulator at all** - the whole of the "22x" was
the inertia tensor between it and the observable. Confidence **92** on both,
runtime-verified on one binary. One trap that measurement exposed and that any
reuse of a capture must apply: the ramped `craft+0x2c0`-`0x2c8` columns are
sampled at the top of `Ship_UpdateCraft` and therefore **lag the frame that used
them by one tick**, because both `Ship_UpdateSteering` and
`Ship_UpdateAirbrakes` ramp and then consume in the same call. Pairing naively
multiplies the fit's residual by 5.6 and pulls the damping to `-5.61`.

Applying it to the input rather than to the damping is itself evidence-led.
Raising the damping to about `107` reaches the same equilibrium but collapses the
time constant from `0.2 s` to `0.009 s`, and the captures rule that out: `steer`
chatters about +/-8 % at roughly 20 Hz - the authentic non-clamping ramp above -
and the recorded yaw rate does not follow it, it decays smoothly.

## Pitch

`Ship_UpdatePitch` (`0x08848d08`). Reads the pitch axis at
`*(craft+0x78) + 0x10`.

```
p = 0
if (FUN_088492bc(craft))                       // gate, not decoded
    p = controls.pitch * (grounded ? pitch_ground : pitch_air)
if (!grounded)
    p += *(craft+0x6c) + 0x90                  // per-team in-air pitch bias
angularLocal.x += p
```

Confidence **82**. `pitch_air` and `pitch_ground` are plain gains on the input
axis, not rates: there is no ramp and no state. `pitch_damping` is consumed
elsewhere, by `Ship_ApplyAngularDamping`.

The gate at `0x088492bc` was not decoded; without it, pitch input would apply
unconditionally. **Measured at runtime, it does not block a grounded craft
standing still**: holding the pitch axis on the start line rotates the craft
immediately, so whatever the gate tests, it is open in the ordinary case.

### The pitch axis, read at runtime

`*(craft+0x78) + 0x10` was probed in PPSSPP while cycling the pad, which settles
two things the static read could not (confidence **90**, one binary, one
emulator version):

| Input | `*(craft+0x78) + 0x10` | What the craft does |
| --- | ---: | --- |
| `up` on the d-pad | `-100` | nose **down** |
| `down` on the d-pad | `+100` | nose **up** |
| analog `y = +1` | `-100` | nose down, i.e. the same field and the same sign |
| analog `y = -1` | `+98.8` | nose up |

- **The scale is `+/-100`, not a normalised `-1..1`** - the same in-memory range
  the steering axis at `+0x00` uses, which
  [Steering](#steering) established from the ramp rates. The `98.2`-`98.8` the
  analog stick reads is the PSP's own byte quantisation of a full deflection.
  So `p = axis * pitch_ground` is evaluated with `axis` at `100`, and any
  reimplementation feeding it a `1.0` is **100x** weak on this term.
- **The axis is negative-nose-up.** `oag_physics::ShipControls::steer_y` is
  documented "positive nose up", so the two disagree by a sign, and the
  polarity that `crates/physics/src/engine.rs` recorded as "a guess awaiting M3"
  is now measured.

Both are visible in `data/traces/talons-junction-pitch-both-ways.csv`; the
scenario is `verification/scenarios/pitch-both-ways.inputs` and the analysis is
in [angular-velocity-column.md](../../../physics/angular-velocity-column.md#the-input-measured-rather-than-assumed).

## The sideshift is a force, and its direction is read rather than guessed

Two questions closed together: what a sideshift *is* (this page and
[physics/README.md](../../../physics/README.md) both said "a one-shot lateral
impulse applied straight to the body", with impulse-versus-velocity open), and
which way it goes.

**The application**, in `Ship_UpdateAirbrakes` (`0x0884c9a4`), after the airbrake
force block and gated on the craft's contact flag:

```text
if (craft+0x1c0 & 1) {                           ; the contact bit
    if (craft+0x1c0 & 0x800)
        Body_AddForceWorld(body, craft+0x170 *  handling+0x5c);
    if (craft+0x1c0 & 0x1000)
        Body_AddForceWorld(body, craft+0x170 * -handling+0x5c);
}
```

`handling+0x5c` is block-relative for absolute `0xf0`, which the parser table
above assigns to `Airbrake.sideshift`. So a sideshift is an **ordinary world
force**, added every frame that its flag is set - divided by mass by the
integrator like every other force - and **switched off in the air**. It is not an
impulse and not a velocity change. Confidence **85**.

Note the ordering consequence: `Ship_UpdateAirbrakes` is step 3 and
`Ship_UpdateHover` (which clears and rebuilds the contact bit) is step 8, so the
flag read here is **last** frame's contact, like the other pre-hover terms.

**The two flags and their timers**, in `Ship_UpdateSideshiftInput_q`
(`0x08846a54`, named here, confidence 72 - the sideshift half is unambiguous, the
tap-history half above it is only partly read):

```text
0884759c  if (entity+0x8a4 > 0)  craft+0x1c0 |= 0x800   else craft+0x1c0 &= ~0x800
088475ec  if (entity+0x8a8 > 0)  craft+0x1c0 |= 0x1000  else craft+0x1c0 &= ~0x1000
```

Both timers are decremented by `dt` earlier in the same function
(`0x0884729c`/`0x088472b4`) and are set to the literal `0.2`
(`0x3e4ccccd`) when a shift fires, so **one sideshift pushes for 0.2 s**. Both can
run at once, in which case the two forces cancel; nothing picks a winner.

**What fires them** depends on a control-scheme test (`Options_ControlSchemeFlag`,
`0x08836828`), and both branches agree on the direction. The branch this section
reads in full (`0x08847440`-`0x08847598`) is the **flick**: the button bound to
input action
`7` must be held (`*(craft+0x78)+0x24`, a button mask, tested against
`1 << FUN_088366bc(7)`), an "armed" flag (`entity+0x860 & 0x400`) must be set -
which happens whenever the axis is inside `+/-10` - and then the steering axis at
`*(craft+0x78)+0x00` crossing

| Axis | Sets | Flag | Force |
| --- | --- | --- | --- |
| `> +10` | `entity+0x8a8` | `0x1000` | `-row0 * sideshift` |
| `< -10` | `entity+0x8a4` | `0x800` | `+row0 * sideshift` |

and `row0` is the ship's **left** ([the basis section](#the-basis-is-positively-oriented-and-row-0-points-left)),
while the same `+/-100` axis is measured **positive-right** by the
`steer-left`/`steer-right` captures (the `steer` column runs `0 .. -108` holding
left). **So a craft shifts toward the side it was flicked**, which is what
`oag_physics::ship::Sideshift::Right` mapping to `+Body::right()` already meant -
a confirmation of an unevidenced guess rather than a correction, and the mirror
image of the airbrake polarity bug that motivated the check.

The other branch (`0x088472f8`-`0x08847438`) is the **button** scheme: the
buttons bound to actions `5` and `6` set `entity+0x8a4` and `entity+0x8a8`
respectively - so action 5 is a left shift and action 6 a right one, the same
pairing the flick produces.

**Three things this paragraph used to say are now settled or corrected**, on
[`input-bindings.md`](input-bindings.md), which reads the whole options module
and the rest of this function:

- The button branch is a **double-tap within `0.25 s`**, not the "repeat
  lockout" an earlier edition of this page called it. `entity+0x89c` and
  `entity+0x8a0` are the left and right tap *windows*.
- There **is** a lockout, `entity+0x8ac`, and it is **`1.0 s` common to both
  branches**, gating the whole trigger block rather than sitting inside the
  button one.
- The two branches are **novice** (flick) and **veteran** (double-tap), and the
  action ids resolve to real buttons: action 7 is `L`, actions 5 and 6 are `L`
  and `R`. So the flick gesture is *hold `L` and flick the stick*, and the
  double-tap gesture is *tap the airbrake twice*.

**A capture would still be worth taking**, and the recipe is now concrete:
hold **`L`** on a novice-scheme profile and flick the stick - 30 ticks is
enough, since the whole event is 12 frames. The direction claim above does not
rest on it: it has an instruction-level leg plus two *measured* legs (row 0 is
the ship's left; the steering axis is positive-right in the `steer-left` and
`steer-right` captures).

A third trigger path above these - a three-entry tap history
(`entity+0x88c`/`+0x890`/`+0x894`) fed by d-pad bits and by the axis crossing
`+/-90`, matching the patterns `2,1,2` and `1,2,1` - sets two other flags
(`entity+0x860 & 0x100`/`0x80`) that drive a separate `entity+0x87c` charge.
Earlier editions of this page parked that as unknown. It is **the barrel
roll**, read out end to end at confidence 80 in
[`input-bindings.md`](input-bindings.md#the-tap-history-path-is-the-barrel-roll):
a three-tap left-right-left alternation with a `0.6 s` inter-tap timeout, paid
for out of shield energy, arming a roll phase that completes rather than
reverses and only rewards the pilot if it finished before the craft landed. It
is still unimplemented, and the parts of it that remain unread are why this
function's name keeps its `_q`.

## The passive terms

Each of these is small, always on, and easy to overlook. All carry confidence
**76-80**, decompilation only.

**Quadratic drag**, `Ship_ApplyQuadraticDrag` (`0x08848e28`):

```
k = craft+0x2a4 == 0     ? -0.9
  : forwardSpeed < -0.2  ? -0.1
  : grounded             ? -0.005
  :                        -0.002
worldForce += velocity * forwardSpeed * k
```

Note the airborne coefficient is **smaller** than the grounded one, and that
reversing (`forwardSpeed < -0.2`) multiplies drag by 20 to 50 times, which is
what makes the ship reluctant to fly backwards.

**Rolling resistance**, `Ship_ApplyRollingResistance` (`0x08848f4c`): when
`forwardSpeed > 0`, adds `2 * unit(velocity)` under a source prefix that negates
all three lanes, so `-2 * unit(velocity)`. A constant-magnitude opposing force,
independent of speed and of every handling parameter.

**Weathervane torque**, `Ship_ApplyWeathervaneTorque` (`0x08848dc4`):

```
angularWorld += cross(forward, velocity) * (grounded ? -0.1 : -0.3)
```

Three times stronger in the air. This, and not any explicit control term, is
what turns the nose toward the direction of travel.

**Angular damping**, `Ship_ApplyAngularDamping` (`0x08848ed0`):

```
angularLocal += (-pitch_damping, -5.0, k) * bodyAngularVelocityLocal
                                       // k = -5.0 in mode 0, else -2.0
```

Only the pitch axis is tunable per ship. Yaw damping is a hard `-5.0` and roll
damping a hard `-2.0` for every craft in the game.

**Vertical damping**, inline in `Ship_UpdateCraft`:

```
worldForce += up * dot(up, velocity) * -0.25 * (1 - magLockBlend)
```

**Gravity**, inline in `Ship_UpdateCraft`:

```
worldForce.y += -( normal_gravity * g_class_gravity_scale[class] * mass * grounded
                 + flight_gravity                               * mass * (1 - grounded) )
```

as a single `vmul.p` pair chain with a source prefix negating both lanes.
`normal_gravity` applies on the ground and `flight_gravity` in the air, blended
by the same 0/0.5/1 grounded fraction, and only `normal_gravity` gets the
per-class scale from `0x08ab0dcc`.

The chain was read register by register when the table was decoded, and it is
worth writing out because it is what settles which lane the scale rides:

```text
0x08849b48  C600 = (class+0xf8, class+0xfc) = (normal_gravity, flight_gravity)
0x08849b94  S610 = g_class_gravity_scale[class]
0x08849b98  S611 = 1.0                          ; viim.s
0x08849ba0  C620 = (mass, mass)                 ; body+0x374, splatted by vmov.s
0x08849bac  S630 = craft+0x2b0                  ; the grounded fraction
0x08849bb8  S631 = 1 - S630                     ; vocp.s, one's complement
0x08849bb0  vpfxs -X,-Y,-Z,0                    ; negates the next source
0x08849bb4  C600 *= C610
0x08849bbc  C600 *= C620
0x08849bc0  C600 *= C630
0x08849bc4  worldForce.y += S600 + S601
```

**The scale lands on lane 0 - the grounded lane - and the airborne lane is
multiplied by a literal `1.0`**, even though the XML attribute that fills the
table is named `airborne`. `0x08ab0dcc` has exactly three references in the whole
binary (this pair of reads and one write in `Xml_ReadGlobalSettings`), so there is
no adjacent table to have confused it with. Confidence **90**, raised from 78:
the table now has a read writer and read readers, and the `Physical` offsets it
indexes against were confirmed independently out of `HandlingXml_ParsePhysical`. `mass` is read from `body+0x374`, not from the
XML `mass` at `+0xf4`; how the two relate is not determined.

The negation matters and is checkable without trusting the prefix encoding: the
hover spring multiplies by `normal_gravity + track_gravity` and must push the
ship **up**, so both are positive in the data. A downward gravity force therefore
requires the sign to be applied here, in the code. The same prefix bit pattern
appears on the rolling-resistance term, which must also oppose motion.

## The hover geometry and the alignment torque, read instruction by instruction

This section re-reads `Ship_HoverTwoPoint` (`0x0884a658`) and the probe cast that
feeds it, to answer one question asked by
[angular-velocity-column.md](../../../physics/angular-velocity-column.md#the-pitch-response-rings-and-the-alignment-torque-is-not-why):
does the surface-alignment torque really have its right-axis component projected
out, and if so is a pitch-restoring component being lost in transcription? The
answer is that the standing transcription is **correct as written**, and the
reading turned up the probe geometry and the lever arm alongside it.

Everything below decodes with the [Allegrex processor
module](../../../psp/allegrex-vfpu.md) installed.

### The alignment torque: the projection is real

Twenty-six instructions, `0x0884ac24`-`0x0884ad28`, with the intermediate
spill-and-reload pairs the compiler leaves everywhere in this function elided:

```text
0884ac24  a2 = craft+0x140                  ; the averaged contact normal
0884ac30  a2 = craft+0x160                  ; up   (the sp+0x600 spill of it)
0884ac44  vcrsp.t C320,C300,C310            ; cross(up, avgNormal)
0884ac5c  lui a2,0xc3c8                     ; -400.0f
0884ac78  vscl.q C300,C500,S400             ; torque = cross(up, avgNormal) * -400
0884ac94  a3 = craft+0x170                  ; right
0884acac  vdot.t S220,C200,C210             ; d = dot(torque, right)
0884accc  vscl.q C300,C500,S400             ; right * d
0884acfc  vsub.q C320,C300,C310             ; torque -= right * d
0884ad18  a3 = craft+0x350                  ; the WORLD angular accumulator
0884ad24  vadd.t C210,C210,C600             ; craft+0x350 += torque
```

Confidence **92**. Every element of the standing transcription holds:

- the operands are `cross(up, avgNormal)`, in that order, both world vectors;
- the scale is the literal `-400.0f` materialised inline at `0x0884ac5c`,
  matching the PS2 immediate the roll-oscillator work already recorded;
- **the projection exists, it is a Gram-Schmidt removal, and its axis is
  `craft+0x170`**, which this page's "three body-axis copies, settled" table
  identifies as the ship's **right** axis, corroborated in this same
  function by the bank-to-yaw term reading `craft+0x174` - the *world y* of that
  vector, i.e. how far the ship is banked - at `0x0884ad40`;
- the destination is `craft+0x350`, the world accumulator, added with a
  three-lane `vadd.t`.

A torque about the right axis is pitch by definition, so the term levels roll and
yaw and **contributes nothing to pitch at all**. Nothing is lost in
transcription and there is no hidden pitch-restoring component. `craft+0x140` is
the running sum of the contacting probes' normals, halved at `0x0884abe0` when
both probes hit, so "average normal" is exact rather than approximate.

**This is a validated negative**: the term the crate implements is the term the
binary has, and the pitch ringing has to come from somewhere else. The
measurement that says where is in
[angular-velocity-column.md](../../../physics/angular-velocity-column.md#the-pitch-response-rings-and-the-alignment-torque-is-not-why).

### The probe geometry is a code literal, like the inertia tensor

`Ship_InitCraft` (`0x08849354`) writes both probe offsets as immediates and then
scales each by the global at `0x08ab0e1c`:

| Where | What |
| --- | --- |
| `0x08849480`-`0x0884949c` | `craft+0xc0 = (0, -1.5, +6, 0)` - `0xbfc00000`, `0x40c00000` |
| `0x088494a0`-`0x088494b4` | `craft+0xd0 = (0, -1.5, -6, 0)` - `0xc0c00000` on `z` |
| `0x088494b8`-`0x08849520` | both scaled by `DAT_08ab0e1c` via `vscl.q` |

`DAT_08ab0e1c` is [`TARGET_GLOBAL_SCALE`](../../../physics/README.md), and it is
written with the literal `0.75` (`lui 0x3f40`) at `0x08841000` in the ship-entity
constructor `FUN_08840c74` - **which calls `Ship_InitCraft` afterwards**, at
`0x088412d4`, so the scale is in place before the offsets are written. The static
image holds `1.0` there and never uses it. So the offsets are

```text
front = (0, -1.125, +4.5)      rear = (0, -1.125, -4.5)
```

Confidence **92** on the instructions and the ordering. Like
[`Body_SetBoxInertia`'s box](rigid-body.md#the-arguments-are-literals-and-the-mass-is-not-the-flying-mass),
this is **identical for every craft in the game** - no handling parameter enters
it, so a reimplementation deriving the spacing from `<Misc length>` is deriving
it from the wrong thing whatever factor it picks.

### `Ship_CastHoverProbes` (`0x08849ed4`) builds the lever arm

Named here; it has no direct caller (`jal` scan included - the call is through a
vtable, like `Ship_UpdateCraft` itself), and it owns the craft fields
`Ship_HoverTwoPoint` then consumes. Per probe `i`, stride `0x10`:

```text
0884a03c  craft+0x120 := craft+0xc0                 ; a PLAIN COPY of the offset
0884a060  vtfm4.q C000,E100,C200                    ; basis(craft+0x80..0xb0) * offset
0884a08c  craft+0xe0  := that + craft+0x1a0         ; + position -> WORLD probe point
0884a0a0  craft+0xec  := 1.0
0884a0a4  lwc1 f12,0x2f0(s6)                        ; the hover target height
0884a0b4  craft+0x100 := craft+0xe0 - up * craft+0x2f0    ; the ray's far end
          Collision_RaycastWorld(world, craft+0xe0, craft+0x100, craft+0x1e0, ...)
0884a0..  craft+0x1d8+i := hit
```

Three things fall out, and each corrects something:

- **`craft+0x120` is the *unrotated* local offset**, a straight `lv.q`/`sv.q`
  copy of `craft+0xc0`. `HANDOVER.md`'s older hover-probe note calls
  `craft+0x120`/`+0x130` "the rotated copies"; they are not, and the distinction
  is load-bearing because `Ship_HoverTwoPoint`'s damper crosses that field with
  the angular velocity and then rotates the *result* into world space.
- **The lever arm the spring's torque acts through is `craft+0xe0 -
  body+0x30`**, i.e. exactly `basis * (0, -1.125, +/-4.5)`.
  `Ship_HoverTwoPoint` passes `craft+0xe0` as the third argument to
  `Body_AddForceAtPoint` at `0x0884ab54`-`0x0884ab6c`. The force itself is along
  `up`, so the offset's *vertical* component contributes nothing to the torque
  (it is parallel to the force); only the `+/-4.5` does. Confidence **90**.
- **The raycast's far end is `probe - up * craft+0x2f0`**, the same field
  `Ship_HoverTwoPoint` springs against at `0x0884aa38`. Read literally, the
  probes reach exactly as far as the height they are trying to hold, which would
  make the spring one-sided - compression only, never a pull. **The captures
  refute that; see the next subsection.**

### The damper's point velocity is the conventional `omega x r`

`crates/physics/src/hover.rs` carries a flagged unresolved sign here - the page
it was written from records the probe velocity as the *negation* of the textbook
rigid-body point velocity, and the crate uses the textbook form anyway with the
discrepancy recorded rather than assumed away. The instructions settle it in the
crate's favour:

```text
0884a954  C310 = body+0x150                         ; the angular velocity
0884a960  C300 = craft+0x120+0x10i                  ; the LOCAL probe offset
0884a970  vcrsp.t C320,C300,C310                    ; cross(r_local, w)
0884a9b8  vtfm4.q C000,E100,C200                    ; basis * that  -> world
0884a9f8  vadd.q C220,C200,C210                     ; + craft+0x190, the velocity
0884aa28  vdot.t S220,C200,C210                     ; dot(that, probe normal)
```

The cross product is `r x w`, which looks like the negation - but `body+0x150`
is `-omega` under the
[`w_game = -w_physics`](rigid-body.md#body0x160-is-angular-momentum-body0x40-is-the-inverse-inertia-tensor)
convention this engine uses throughout, so `basis * (r x -omega)` is
`basis * (omega x r)`: the conventional point velocity, in world space. The
recorded sign and the recorded frame were both right, and they cancel.
Confidence **88**. The crate's note can be closed rather than carried.

### `rebound_jump_time` gates the landing bounce, and it gates it *in the air*

Read out of `Ship_HoverTwoPoint` (`0x0884a658`) on 2026-08-12, chasing a report
that the original gives a small bounce on touchdown. Confidence **85**: taken
from the decompiler rather than a capture, but every neighbouring field offset
matches the `Antigrav` layout already recorded above, and the two either side of
it are the ones the confidence-91 rebound blend already uses.

`craft+0x70` points at the parsed `Antigrav` block, so `+0x4` is `rebound`,
`+0x8` is `landing_rebound` and `+0xc` is `rebound_jump_time`. `craft+0x2b4` is
the time-since-landing the blend reads, and `craft+0x284` is how long the craft
has been off the ground. The tail of the function:

```c
if ((craft+0x1c0 & 1) == 0) {            // no probe touched anything this tick
    if (handling->rebound_jump_time < craft+0x284) {
        craft+0x2b4 = 0;                 // arm the landing response
    }
} else {
    craft+0x2b4 += craft+0x1c8;          // grounded: count up by dt
}
```

**The timer is zeroed while the craft is still airborne, not when it lands**, and
only once the flight has already lasted longer than `rebound_jump_time`. So the
sequence is: fly long enough, the timer arms at zero; touch down, it starts
counting and `landing_rebound` blends over the first 0.2 s.

The consequence is the point of the mechanism: **a hop shorter than
`rebound_jump_time` never arms it**, so the craft lands out of a small bounce
with the ordinary `rebound` and no landing response at all. Only a real flight
earns the bounce.

`crates/physics/src/forces.rs` does it differently - it resets
`time_since_landing` on the grounded 0 -> 1 edge whatever the flight was worth,
and `rebound_jump_time` is parsed, carried through `oag_gameplay::handling` into
`oag_physics::params::Antigrav`, and **never read by the force law**. That makes
ours apply the landing blend on every momentary loss of contact, including the
`grounded` 1.0 -> 0.5 -> 1.0 flicker a craft does constantly on rough ground.

### The probes depenetrate as well as push

Also in `Ship_HoverTwoPoint`, and worth recording because it is a *position*
change rather than a force:

```c
local_510 = dot(craft+0xe0 - craft+0x1e0, up);   // probe point to hit point
craft+0x308 = local_510;                          // what the spring measures
if (local_510 < 1.0 && craft+0x208 == 1) {
    Body_Translate(up * (1.0 - local_510), craft+0x1cc);
}
```

Within one unit of the probe origin the hull is treated as through the surface
and the body is **moved** out along `up` by the shortfall, before any spring
force.

`crates/physics/src/hover.rs` already had this as `HoverProbe::escape`. What it
had wrong was the **ordering**: it returned early on a non-hoverable surface and
so never escaped from one, where the original escapes first and only then asks
whether the collider's type is 1 or 3. A probe a hand's breadth inside a `Wall`
is extruded from it in the original and was not here. Fixed 2026-08-12; neither
determinism reference moved, because no probe in either scenario is ever inside
a wall.

**What `craft+0x208` holds is now recovered, 2026-08-25 (previously "not
recovered" here).** That field is written at `0x08816e14` by
`Collision_RaycastWorld` from a stack local (`sp+0x68`), alongside the collider
index at `+0x20` and a float at `+0x24`; the hit record is `0x30` bytes and
this was its last identified-by-offset-only member. **It is not a copy of the
surface-type value read off the collider** - it is a fixed constant each of the
two narrowphase raycasts writes about *itself*, regardless of what it hit:
`Collision_RaycastMesh` always writes `1`, `Collision_RaycastBox` always writes
`3`, and no other value ever reaches this field from this call site. Which of
the two narrowphase functions runs for a given collider is gated by a
`collider+0x78` virtual read - the same accessor `Collision_DispatchPair`
switches on for its own pairing, confirmed byte-for-byte identical and cross-read
against `Collision_BoxAgainstMesh`'s own field accesses: `1` is a mesh-shaped
collider, `3` a box-shaped one, the same numbering both call sites use (an
earlier draft of this note guessed it might be the reverse; it is not). It is
still a shape classifier, not a copy of the already-recovered `Surface` enum -
see [collision.md](collision.md#raycasts) for the full derivation.

That leaves the specific claim `crates/physics/src/hover.rs`'s own doc comment
already makes - "`1` is the **`Floor` class**" - exactly as well-founded as it
was before this pass: correct enough that the implementation built on it has
not needed correcting, sourced from an earlier, independent read this pass
neither confirms nor refutes at the identity level (this pass read what gets
*written* at `+0x28`, not what the upstream classifier that selects between the
two writers actually denotes). **The gate itself is not missing** -
`hover.rs::probe_from_hit` already reproduces it, as
`hit.surface == Surface::Floor` guarding the escape translation, landed before
this pass and using the crate's own already-recovered `Surface` enum rather
than mirroring `craft+0x208` bit-for-bit. No code change follows from this
pass; what changes is that the field's own mechanism - which function wrote it,
and that it is a literal rather than anything read off the collider - is no
longer an open question. Confidence **90** on the mechanism (up from 80, the
missing-gate deduction that lowered it no longer applies); the `1 = Floor`
identity itself keeps whatever confidence the pass that first asserted it
carries, unchanged by this one.

### Resolved: neither reading was refuted, the load was

The two readings this section used to record as contradicted by
`data/traces/talons-junction-pitch-both-ways.csv` - the reach equalling the
target, and the `-1.125` vertical drop - are **both correct as read**, and the
refutations were arithmetic performed with the wrong load. What was missing was
the downforce below; the whole story is one term.

The refutations were, in their own words, that a reach equal to the target leaves
only `0.116` of extension headroom while the capture holds `grounded` at exactly
`1.0`, and that a probe `1.125` lower than the centre would sit `2.888` above the
track where the capture's own pose reads `4.009`. Both computed the resting probe
height as `target - sag` with `sag` derived from the craft carrying
`normal_gravity` alone, which gives `0.147`. The craft does not carry
`normal_gravity` alone:

```text
2 * 0.3 * K * (normal_gravity + track_gravity) * compression
    = normal_gravity * classScale + track_gravity        ; gravity + the downforce
compression = 1.25                                       ; classScale == 1.0
```

so a resting probe sits `4.125 - 1.25 = 2.875` above the surface with `1.25` of
travel in *each* direction - and the live read on the start line measures
`craft+0x308 = 2.8878`, and the resting centre height `4.002`-`4.009` is that
probe height plus the `1.125` drop. Every number the refutations quoted is
reproduced by the corrected model; `4.009` was the *centre*, not the probe.

Three independent legs now agree on the geometry:

| Leg | Predicts | Measured |
| --- | ---: | ---: |
| The literals plus the corrected load | probe rest `2.875`, centre `4.000` | `2.8878`, `4.002`-`4.009` |
| Two probes at `+/-4.5` on the recovered tensor | pitch `omega_n` `9.4` rad/s | `9.32`-`9.72` |
| The damper at that load through those arms | `2*zeta*omega_n` `6.6` | `8.85` fitted, `8.90` reproduced |

**Landed**: `crates/physics/src/hover.rs` implements the offsets, the
target-length reach and the downforce together, and the pitch step response goes
from ringing at `2*zeta*omega_n = 1.75` to reproducing the original's extrema to
the third decimal. See
[angular-velocity-column.md](../../../physics/angular-velocity-column.md#resolved-the-missing-49-is-the-hover-downforce).

### The grounded downforce, read instruction by instruction

`Ship_HoverTwoPoint`'s epilogue, `0x0884ad78`-`0x0884ae18`, after the alignment
torque and the bank-to-yaw term:

```text
0884ad78  a0 = craft+0x1cc                 ; the rigid body
0884ad7c  lwc1 f12,0x374(a0)               ; mass - the same field gravity reads
0884ad80  a0 = craft+0x70                  ; the handling block
0884ad84  lwc1 f13,0x6c(a0)                ; track_gravity
0884ad88  mul.s f12,f13,f12                ; track_gravity * mass
0884ad8c  lwc1 f14,0x2b0(s0)               ; THIS frame's grounded fraction
0884ad90  mul.s f12,f12,f14
0884ad94  neg.s f12,f12                    ; the sign is in the code, not the data
0884ada4  lv.q C500,0x0(a1)                ; a1 == craft+0x140, the averaged normal
0884ada8  vscl.q C300,C500,S400            ; * the scalar above
0884adc4  a0 = craft+0x150                 ; (the averaged normal is also cached here)
0884add0  lwc1 f12,0x280(s0)               ; the magstrip blend
0884add4  sub.s f12,f20,f12                ; 1 - blend
0884adec  vscl.q C300,C500,S400            ; * that
0884ae08  a1 = craft+0x330                 ; the WORLD force accumulator
0884ae14  vadd.t C300,C300,C600            ; += it
```

`-track_gravity * mass * grounded * (1 - magLockBlend) * avgNormal`, with **no
scale of any kind** - the coefficient is `1`. Confidence **92**: every
instruction is in address order with nothing elided, and the PS2 four-corner
twin computes the same product through a different base pointer
([craft-update.md](../ps2-pulse-eu/craft-update.md#the-rest-of-the-four-corner-path)).

Two things fall out that had been open on this page for three passes:

- **The spring's calibration is not a curiosity.** It multiplies by
  `normal_gravity + track_gravity` because that is exactly the load it carries -
  gravity plus this term - which is why the resting compression is `1.25`
  whatever the two gravities are and whatever the craft weighs.
- **This is the missing pitch damping.** The damper is a multiplier on the
  spring magnitude, so its rate feedback is `0.1 * rebound * S` with `S` the
  spring force actually being carried. Without the downforce a reimplementation
  carries `normal_gravity` alone - `1/17` of the load on the shipped values -
  and gets `1/17` of the damping. Nothing else was ever missing.

The per-class gravity scale this interacts with is worth recording while it is
cheap: the table at `0x08ab0dcc` reads `1.0, 1.0, 1.0, 1.0` in the shipped image,
so the class never changes the load. Confidence **90** (a static read of four
words; no runtime check).

## `Ship_UpdateMagLock` rewrites the basis directly, and that is the missing mechanism

[cornering-ground-truth.md](../../../physics/cornering-ground-truth.md#the-refutation-is-scoped-the-inverted-section-was-carrying-it)
measures, from a whole lap, that something rotates the ship's basis **outside
`Body_Integrate`**, concentrated in Talon's Junction's inverted section, slaving
the ship's `up` to the track's own surface normal at `0.906x` the rate that
normal turns. It names `Ship_UpdateMagLock` (`0x0884ba0c`) as the suspect and
scores the attribution **45**, explicitly because "no instruction has been read
writing the basis outside the integrator".

**Those instructions are read here.** The function's last eighty instructions do
nothing else. With `a0 = body` (`craft+0x1cc`, reloaded at `0x0884c190`):

```text
0884c314  a1 = body+0x20                          ; row 2, forward
0884c32c  vdot.t   d = dot(forward, axis)
0884c394  vscl.q   (axis * d) * craft+0x280        ; * the mag-lock blend
0884c3d4  vsub.q   forward -= that
0884c3f4  sv.q     body+0x20 := forward            ; THE BASIS IS WRITTEN
0884c404  a0 = body+0x00                          ; row 0, right
0884c4b8  vsub.q   right -= axis * dot(right, axis) * craft+0x280
0884c4d8  sv.q     body+0x00 := right              ; AND AGAIN
0884c4f8  vcrsp.t  cross(forward, right)
0884c514  sv.q     body+0x10 := that               ; up := completion of the pair
0884c51c-0884c5ac                                  ; Gram-Schmidt, forward primary:
          forward := unit(forward)
          up      := unit(up - forward * dot(forward, up))
          right   := cross(up, forward)
```

Confidence **90** on the writes themselves - `sv.q` into `body+0x00`, `+0x10`
and `+0x20`, the three basis rows [`Body_Init` sets to
identity](rigid-body.md#body_init-0x0884de5c), followed by an explicit
re-orthonormalisation, which is what a function that has just skewed a rotation
matrix has to do. **It is not a torque, it never touches an accumulator, and it
therefore cannot appear in the momentum column** - which is exactly the shape
the lap measurement asked for. The attribution can go from **45** to **90**.

Three details make the match specific rather than merely available:

- **The axis is track-derived and interpolated along the path.** `sp+0x10` is
  built at `0x0884ba9c`-`0x0884bafc` as `unit(-(sample+0xB10))` off the object at
  `craft+0x1c4`, and a second candidate off the neighbouring sample is built the
  same way at `0x0884bcc0`-`0x0884bd18`. The two are then blended by normalised
  weights (`0x0884be2c`-`0x0884bea8`, each `1 - |d_i - h|` clamped at zero,
  divided by their sum) before either is used. So the axis the attitude is slaved
  to **turns continuously as the craft drives along the track**, which is the
  quantity the lap fit regressed against. **`+0xB10` is now identified rather than
  inferred - see the next subsection - and the weights are not distance weights.**
- **It is gated on exactly the field that fades the suspension out.**
  `craft+0x280` is the mag-lock blend; `0x0884ba84` returns immediately when it
  is zero, so on ordinary track this function does nothing at all. It ramps by
  `0.2` per frame (`DAT_08a7bd44`), clamped to `0..1`, driven by `craft+0x240` -
  which `Ship_CastHoverProbes` sets from a third raycast that accepts only
  surface type **3**, the mag floor. Ordinary track is untouched; a magstrip
  section gets full slaving in five frames. That is the localisation the lap
  measured, arrived at independently.
- **The same blend scales the hover spring to zero.** `Ship_HoverTwoPoint`
  multiplies every probe force by `1 - craft+0x280` (`0x0884ab48`). So the two
  are one design: on a magstrip the suspension hands the attitude over to a
  kinematic hold rather than competing with it. `crates/physics/src/hover.rs`
  implements the fade; the hold is `crates/physics/src/maglock.rs`, and the two
  halves are now both in the crate.

It also **repositions** the body: `0x0884c0dc`-`0x0884c18c` computes
`(0.8 * craft+0x2f0 - d)` along the blended axis, scales it by the blend and
passes it to `Body_Translate`, so the craft is held at four fifths of its hover
target height off the strip by direct displacement. And `0x0884c19c`-`0x0884c310`
projects the velocity off the axis and renormalises it to its original
magnitude - the one part of this function
[rigid-body.md](rigid-body.md#why-this-is-the-missing-228--fs) had already read,
in order to rule it out as the speed loss. That ruling stands; nobody had read
past it.

**A reimplementation must not model this as a torque.** Doing so puts it through
the momentum column, which is precisely the thing the lap capture proves the
original does *not* do.

### `+0xB10` is `SplinePt.down`, and the whole record is a located spline sample

The section above scored "`section+0xB10` is the surface normal" at **70**,
inference from use, and flagged it as a track-format question. It is settled at
**90**, and the answer is more specific than the guess: `craft+0x1c4` is the ship
**entity**, and `entity+0xaf0` and `entity+0xb60` are two `0x70`-byte
[`SplinePt`](../../../formats/track.md#control-points) records that
`AiTrack_LocatePosition` (`0x0887ce78`) fills in. `+0xB10` is `+0xaf0 + 0x20`,
which in that struct is **`down`: the unit vector *into* the track surface**. So
`unit(-(+0xB10))` is the surface normal, exactly as assumed, and the whole record
is available rather than one field.

Five legs, no one of which is decisive and which do not share an assumption:

| Leg | Where | What it shows |
| --- | --- | --- |
| The writer | `0x08842ae8`-`0x08842b00` in `FUN_08842a18` | calls `AiTrack_LocatePosition` with `a1 = entity+0xaf0` **and** `t1 = entity+0xb60`: both records are its output, and the second is the neighbour argument |
| A second writer | `0x0883ffe4`-`0x08840040` in `FUN_0883ff6c` | same call with `a1 = entity+0xaf0`, then copies `0x00`-`0x60` straight back out - the `SplinePt` stride, read as one object |
| The constructor | `0x08840dc0`-`0x08840e3c` in `FUN_08840c74` | initialises both records field by field in the `SplinePt` layout: four `vec4` at `+0x00/+0x10/+0x20/+0x30`, six floats at `+0x40`-`+0x54`, two bytes at `+0x60/+0x61` |
| The lift, undone | `0x0884bae0` area | the hold forms `sample+0x00 - 3.0 * unit(-(sample+0x20))`, and `AiTrack_LoadPathPoints` (`0x0887eba8`) lifted every control point by `pos -= 3.0 * down`. The two are inverses **only if `+0x20` is `down`** |
| The probe direction | `0x08849f..` tail of `Ship_CastHoverProbes` | the mag ray runs from `craft+0x1a0` to `craft+0x1a0 - 5 * (up - sample+0x20)`, which for an aligned ship is about ten units **into** the surface. With `+0x20` pointing out of it the ray would go nowhere |

Two corrections to the section above fall out of the same reading, and both
matter to a reimplementation:

- **The weights are height agreement, not path distance.** `h` is
  `dot(n1, craft+0x1a0 - craft+0x250)` - the craft's height above the *mag ray's
  own hit point*, along the first sample's normal - and `d_i` is its height above
  section `i`'s surface. Each weight is `1 - |d_i - h|` clamped at zero, both
  zero becomes `0.5/0.5`, and the pair is then normalised. The section whose
  surface best predicts the measured height wins.
- **There is a fallback branch nobody had recorded.** When
  `|h - d| > DAT_08a7bd48` (`5.0`, `0x40a00000`) the spline is abandoned outright
  and the axis becomes `craft+0x260` with `d = dot(craft+0x1a0 - craft+0x250,
  axis)` - the mag ray's own normal and hit. The second sample is skipped
  entirely when `sample2+0x40` reads `-1024.0`. That sentinel is written by the
  locator itself - `AiTrack_UpdateCursor` (`0x0887e464`) materialises `0xc4800000`
  at `0x0887e9d4` and stores it to `+0x40` of the record four instructions later -
  and **not** by the entity constructor, which zero-fills both records
  (`mtc1 zero, f20` at `0x08840d38`). `+0x40` is `SplinePt`'s `unk_0x40`, which
  the format page lists as undetermined.

`DAT_08a7bd44` is `0.2` (`0x3e4ccccd`) and is added or subtracted **per frame**,
with no `dt` anywhere near it.

### It is implemented, and measured against the lap it was predicted from

`crates/physics/src/maglock.rs` implements this, kinematically: the ramp, the
mag-floor probe, the two-sample axis blend with its fallback, the reposition, the
velocity projection and the basis rewrite in the binary's own operation order.
Nothing goes through an accumulator.

`crates/game/tests/maglock_ground_truth.rs` then measures it against
`data/traces/talons-junction-time-trial-lap-omega.csv`, one tick at a time from
each recorded pose, over the inverted stretch. On the USA PSP disc:

| | |
| --- | ---: |
| inverted poses (`up.y < 0.85`) with `Mag Floor` under them | **258 of 283** |
| upright poses (`up.y > 0.99`) with `Mag Floor` under them | **5 of 1,959** |
| residual `omega(basis) + body+0x150`, rms | `1.25` rad/s |
| what the hold produces from the same poses, rms | `2.30` rad/s |
| fraction of the residual it explains, nothing fitted | **`49.5 %`** |
| this crate's axis versus the recorded `up`, mean | `2.07` degrees |

The first two rows are the substantive new evidence: the probe finds a magstrip
where and only where the lap found the identity broken, and the two measurements
share no input. The last two are the open part - a full-blend hold snaps the ship
onto the axis in one tick and the original visibly does not sit exactly on it, so
either the blend is not `1.0` there or this crate's resampled spline is not quite
the original's evaluated one. **Neither is tuned away**; see the test.

## Contradictions with `docs/physics/README.md`

**`ride_height` does appear in the force law.** That page states it "never
appears in the force law. It is only the raycast length; the height the ship
settles at is emergent." The offset chain says otherwise:

- `HandlingXml_ParseAntigrav` stores `ride_height` at `+0x94`.
- `Ship_UpdateCraft` sets `craft+0x70 = stats + class * 0x80 + 0x94`, so
  `**(float**)(craft+0x70)` *is* `ride_height`.
- `Ship_UpdateCraft` then computes
  `craft+0x2f0 = (ride_height + craft+0x74 - leapAdjust) * (1 + 0.2 * magLockBlend) * K2`
  where `leapAdjust` is `min(craft+0x2e0, 4.0)` while that timer runs and `K2` is
  the global at `0x08ab0e1c`.
- `Ship_HoverTwoPoint` uses `craft+0x2f0` as the spring's target height:
  `(craft+0x2f0 - h) * ...`.

So `ride_height` is the primary term of the hover target height and scales the
spring force directly. Confidence **88**, resting on the parser evidence rather
than on a reading of the consumer. What `craft+0x74` is, and who writes it, is
**not determined** - it is an additive height offset, and `antigrav_height_adjust`
is an obvious suspect but nothing was found that copies it there.

**The four-corner selector is `DAT_08ab07e3 == 0 && DAT_08b31048 == 6`.** That
page lists "a four-corner hover variant exists alongside the two-point one; which
is used when was not established" as open. `Ship_UpdateHover` (`0x0884870c`) is
three lines: it calls `Ship_HoverFourCorner` under exactly that condition and
`Ship_HoverTwoPoint` otherwise, then `Ship_UpdateMagLock` unconditionally. The
same condition gates the brakes off and swaps the engine for an auto-speed law
driven by a counter at `craft+0x28c`, which reads like a game mode rather than a
ship or track property. Confidence **84** for the selector.

**The mode is Zone**, confidence **84** - raised from the 50 this paragraph used
to carry. The mode factory `Race_CreateModeObject` (`0x0882112c`) selects on
`(DAT_08ab07e3 == 0) ? DAT_08b31048 : 0`, and its `case 6:` allocates the object
whose constructor loads `Data\XML\Zone_HUD.xml`. That is the byte-identical
expression, so the four-corner hover, the disabled brakes and the auto-speed law
all belong to Zone. The counter at `craft+0x28c` is the zone number, **assigned**
into the craft by `Zone_Update` (`0x0882f5cc`) every ten seconds. See
[zone-mode.md](zone-mode.md).

Two corrections to the law as written above, both read from the instruction
stream at confidence **84**:

- **It is gated.** `(flags & 1) && !(flags & 2)`, with `T = 0.0` in the
  branch-likely delay slot when the test fails. Bit 0 is the ground-contact bit,
  the same one that gates the brakes.
- **The conversion is unsigned.** `bgez` plus `lui 0x4f80` is the standard
  `(float)(u32)` idiom, so it is `(float)(uint32)craft+0x28c`. The PS2 page
  already wrote `(uint)`; this page was the odd one out.

Both `g_autospeed_base` and `g_autospeed_step` are `.bss` and hold nothing in the
file: they are parsed at runtime from `<Handling><Global><Zone/></Global>` in
`Data\XML\HandlingStats.xml`.

**Groundedness is one frame stale for every control term.** That page records
`0.75 * grounded_prev + 0.25` for the hover spring's load factor, so the lag was
noticed there. It is not local to the spring: it splits the force law in two.

`Ship_UpdateHover` clears flag bit 0 (`flags &= 0xfffffffe`) as its first
statement, and `Ship_HoverTwoPoint` zeroes `craft+0x2b0` on entry - after reading
the old value for the load factor - then re-accumulates one half per contacting
probe and ORs bit 0 back in. Hover is **step 8** of fifteen. So, from the
ordering table above:

| Reads | Sees |
| --- | --- |
| `Ship_UpdateEngine` (`craft+0x2b0`, the 20 %-in-air blend) | **last frame** |
| `Ship_UpdateBrakes` (gated on `flags & 1` by the caller) | **last frame** |
| `Ship_ApplyQuadraticDrag` (`flags & 1` selects the coefficient) | **last frame** |
| inline gravity (`craft+0x2b0` blends normal against flight) | **last frame** |
| `Ship_UpdatePitch` (`flags & 1` selects air or ground gain) | **last frame** |
| `Ship_ApplyLateralGrip` (`craft+0x2b0`) | this frame |
| `Ship_ApplyWeathervaneTorque` (`flags & 1`) | this frame |

Confidence **84**, from the call order and the two clearing sites. The obvious
reimplementation - resolve contacts, then apply forces - gets a different answer
on every takeoff and landing frame, in the engine, the brakes, gravity, drag and
pitch simultaneously. Unlike the dead roll-levelling branch, this one changes
trajectories.

**The dead in-air roll-levelling term is confirmed**, and it is inline in
`Ship_UpdateCraft` rather than in a hover function: under `(flags & 1) == 0` it
computes `5 * right.y`, a comparison of `up.y` against zero, a `vcmovf`, a
`vscl.t` of the forward axis and a final `vadd.t` whose result is **never
stored**. Same conclusion, now with the location.

## Not determined

- **Almost nothing on this page is runtime-verified**, and the 84 cap for
  decompilation alone applied to every force-law claim in the first edition.
  Eight of those terms now have a second binary agreeing - see
  [the cross-platform section](#the-force-law-now-has-a-second-binary-leg) for
  which, and for the four places the two builds differ. The terms not on that
  list (brakes, airbrakes, rolling resistance, vertical damping, the dead
  roll-levelling branch) are still single-source and still capped at 84.
- **The call site of `Ship_UpdateCraft`.** No direct caller exists; the dispatch
  is presumably a vtable. Until it is found, "once per frame per craft, before
  integration" is structural inference.
- **The `craft+0x2a4` enum.** Values 0 through 8 are compared against. 0 disables
  the `rebound` parameter and multiplies drag by ~180x, 1 is required for turbo,
  3 skips the accumulator drain entirely, and 4, 5, 6 and 8 skip the control
  block. A front-end display mode and a racing mode are the obvious guesses, at
  confidence 40, so it is deliberately unnamed.
- **`FUN_088492bc`**, the gate on pitch input.
- **`FUN_08848f9c`**, which locates the craft's track section, updates lap
  bookkeeping, and adds a world force along the section's forward axis with a
  per-class magnitude from `0x08b36bc0`/`0x08b36bd0`. Its timer is re-armed every
  frame a section is found, so the force looks continuous rather than one-shot.
  Whether that is a rail assist, a catch-up mechanism or something else is a
  guess at confidence 45; it is described here and not named.
- **Any reader of `antigrav_height_adjust` (`+0x110`).** A search for scalar FPU
  loads at the block-relative offset `0x7c` found none in the craft path. That is
  a weak negative - it does not cover VFPU `lv.s` loads or a copy through a
  cached member - so "parsed but never consumed" is a hypothesis at confidence
  50, not a finding.
- **The `mass` at `+0xf4` versus the mass at `body+0x374`.** Every force term
  uses the latter. Whether ship construction copies one to the other, or scales
  it, was not traced.
- **The VFPU prefix encoding.** Three claims lean on it: gravity's negation,
  rolling resistance's negation, and the brake force's zeroed `y` lane. The first
  two are corroborated by an independent sign argument (above); the third is not,
  and sits at confidence 55.
- **`Ship_UpdateMagLock`'s writes.** The mechanical scan for accumulator base
  addresses truncated, so the "no commanded roll" negative does not cover it.
- **The `flags` word at `craft+0x1c0`.** Eleven bits are used across this path
  (`0x1, 0x2, 0x4, 0x8, 0x10, 0x20, 0x40, 0x80, 0x100, 0x200, 0x400, 0x800,
  0x1000, 0x2000`). Only bit 0 is established (grounded, set by the hover probes).
  The rest are described where they appear and not named.
- **Units.** Still open, and this page does not settle them.
- **Handedness.** Still open in the sense of world units and axis directions.
  What is *no longer* open, and was being confused with it, is the angular sign
  convention: `w_game = -w_physics`, established from the PS2 integrator and the
  steering measurement above. The basis itself is positively oriented under the
  ordinary component-wise cross product in both builds.

## Applied renames

| Address | Name | Conf |
| --- | --- | ---: |
| `0x0883945c` | `HandlingXml_ParseEngine` | 90 |
| `0x0883962c` | `HandlingXml_ParseBrakes` | 90 |
| `0x088398e0` | `HandlingXml_ParseTurning` | 90 |
| `0x08839a04` | `HandlingXml_ParseAirbrake` | 90 |
| `0x08839278` | `HandlingXml_ParseAntigrav` | 90 |
| `0x08838f50` | `HandlingXml_ParsePhysical` | 90 |
| `0x0883977c` | `HandlingXml_ParsePitch` | 90 |
| `0x0883a2f0` | `Handling_ParseStats` | 88 |
| `0x08849618` | `Ship_UpdateCraft` | 82 |
| `0x0884c5c8` | `Ship_UpdateEngine` | 84 |
| `0x08848788` | `Ship_UpdateSteering` | 84 |
| `0x0884c9a4` | `Ship_UpdateAirbrakes` | 88 |
| `0x088489d8` | `Ship_UpdateBrakes` | 82 |
| `0x08848d08` | `Ship_UpdatePitch` | 82 |
| `0x0884870c` | `Ship_UpdateHover` | 82 |
| `0x0884d4c8` | `Body_AddForceWorld` | 82 |
| `0x08839c68` | `HandlingXml_ParseAirbrakeGraphics` | 80 |
| `0x08848b78` | `Ship_ApplyLateralGrip` | 80 |
| `0x08848e28` | `Ship_ApplyQuadraticDrag` | 80 |
| `0x08848ed0` | `Ship_ApplyAngularDamping` | 80 |
| `0x08848f4c` | `Ship_ApplyRollingResistance` | 76 |
| `0x0884d5bc` | `Body_AddTorqueLocal` | 74 |
| `0x0884d604` | `Body_AddTorqueWorld` | 74 |
| `0x08848dc4` | `Ship_ApplyWeathervaneTorque` | 74 |
| `0x08b36bfc` | `g_handling_parse_class` | 85 |
| `0x08ab0dcc` | `g_class_gravity_scale` | 90 |
| `0x08b36bec` | `g_speedpad_jump` | 88 |

## Cross-platform

| Platform | Notes |
| --- | --- |
| PSP (Pulse) | This page |
| PS2 (Pulse) | **Located, and it agrees.** See [ps2-pulse-eu/handling-xml.md](../ps2-pulse-eu/handling-xml.md) |
| PSP (Pure) | Not located |

| Function | PSP | PS2 (`SCES_547.48`) |
| --- | --- | --- |
| `Handling_ParseStats` | `0x0883a2f0` | `0x0014e6f0` |
| `HandlingXml_ParseEngine` | `0x0883945c` | `0x0014d638` |
| `HandlingXml_ParseBrakes` | `0x0883962c` | `0x0014d7c0` |
| `HandlingXml_ParseTurning` | `0x088398e0` | `0x0014d8e0` |
| `HandlingXml_ParseAirbrake` | `0x08839a04` | `0x0014dc90` |
| `HandlingXml_ParseAntigrav` | `0x08839278` | `0x0014d4a0` |
| `HandlingXml_ParsePhysical` | `0x08838f50` | `0x0014d230` |
| `HandlingXml_ParsePitch` | `0x0883977c` | `0x0014d9d8` |
| `HandlingXml_ParseAirbrakeGraphics` | `0x08839c68` | `0x0014de78` |
| `g_handling_parse_class` | `0x08b36bfc` | `0x002da920` |
| `Ship_UpdateCraft` | `0x08849618` | `0x001596f8` |
| `Ship_UpdateHover` | `0x0884870c` | `0x0015aff0` |
| `Ship_HoverFourCorner` | `0x0884ae90` | `0x0015a940` |
| `Ship_HoverTwoPoint` | `0x0884a658` | `0x0015b978` |
| `Ship_UpdateMagLock` | `0x0884ba0c` | `0x0015b070` |
| `Ship_ApplyAngularDamping` | `0x08848ed0` | `0x0015c1b0` |
| `Body_AddForceWorld` | `0x0884d4c8` | `0x0015dac8` |
| `Body_AddTorqueLocal` | `0x0884d5bc` | `0x0015dda0` |
| `Body_AddTorqueWorld` | `0x0884d604` | `0x0015ddb8` |
| the enclosing per-entity update | `0x0884f70c`, not renamed | `World_StepBodies` `0x0015ded8` |

Part of the craft path has now been read in the PS2 build; see
[ps2-pulse-eu/craft-update.md](../ps2-pulse-eu/craft-update.md). What it corroborates:

- **The angular damping triple.** `(-pitch_damping, -5.0, k)` with `k = -5.0` in
  mode 0 and `-2.0` otherwise, reading `pitch_damping` from the same
  block-relative offset. Identical.
- **The four-corner selector**, including both globals and the magic value `6`,
  and `Ship_UpdateHover`'s three-line body. This page scores that at 84; a second
  binary agreeing raises it.
- **The four accumulators.** `body+0x100`, `+0x120`, `+0x130` are the same three
  offsets with the same world/local split, and the one-line helpers are the same
  one line.
- **The call-site shape.** The PS2 walks its body array and virtual-calls the
  craft update through a `{i16 this-adjust, void *fn}` vtable pair at
  `object+0x38`, clearing `*(object + 0x370)` immediately before - the same
  offset this page's disassembly shows. Different enclosing function, same
  dispatch idiom.
- **The angular sign convention**, which is the one item flowing the other way:
  the PS2's `Body_Integrate` explains this page's own runtime steering
  measurement instead of merely agreeing with a static reading. `w_game =
  -w_physics`; the surface-alignment and weathervane torques are correct as
  written. The PSP integrator has never been located, so this transfers at
  confidence **80** rather than the 88 it carries on the PS2 page.

### The force law now has a second-binary leg

The line below - "The force law below is untouched by it: no PS2 `Ship_Update*`
function has been located" - **is out of date.** Nine of the PS2
`Ship_UpdateCraft`'s sixteen callees have since been identified by constant
fingerprint and checked term by term. Six of this page's sections reproduce in
full:

| This page's term | PS2 | Result |
| --- | --- | --- |
| Engine | `0x0015c448` | Every element, **including the dead `gain`/`falloff` ramp** |
| Steering | `0x0015bf30` | Every element, including the reverse-controls blend |
| Pitch | `0x0015c2d8` | Every element; the gate is undecoded in both builds |
| Lateral grip | `0x0015c7c0` | Every element, and `(0.01 - slidegrip)` **as a literal expression** |
| Weathervane torque | `0x0015a290` | Both coefficients and the grounded selector |
| Track-section force | `FUN_0015a300` | Structure and the two per-class tables; still unnamed in both |
| Quadratic drag | `0x0015c3a0` | Three of four coefficients (see below) |
| Gravity | `0x0015a1d0` | Structure; two details differ (see below) |

**This is the second source the "decompilation alone caps at 84" note asks for**,
and it is a strong one: a different compiler targeting a different ISA. The
sections above should be read at **88** rather than 84, except where noted. Two
of this page's own weaker claims are specifically lifted:

- **The dead `Engine.gain`/`falloff` ramp**, scored 85 here from one binary and
  flagged as something an obvious reimplementation gets wrong. The PS2 emits the
  same dead store: it ramps `craft+0x2e8`, clamps it, then overwrites it with
  `controls->thrust` and computes the thrust from the input. **90.**
- **`slidegrip` as "percent of grip retained"**, scored 90 here as an
  interpretation derived from the load-time `1e-4` factor. The PS2 grip term is
  literally `airbrake * (0.01 - slidegrip) - 1.0`, with both endpoints falling
  out arithmetically. A derived reading reproduced as an explicit expression.
  **93.**
- **The `S221` register-carry oddity**, scored 65 here with a recommendation to
  write `accumulator.y = lift`. The PS2 builds an ordinary two-lane vector for
  thrust and lift. That does not explain the PSP's codegen, but it does confirm
  the intended semantics, so **the recommendation is right**. Raise the
  recommendation to 85; the explanation of the PSP's register use stays at 65.

Four differences, none of them corrections to this page - each needs the PSP
side re-read before anyone decides which build is the odd one:

- **Drag has no `-0.9` branch on the PS2.** Nothing there tests a mode enum. The
  other three coefficients and the `-0.2` threshold are identical. Whether the
  PS2 hoisted the case to its call site was not checked; confidence **55** that
  the PS2 genuinely drops it.
- **Gravity's per-class scale sits on `flight_gravity` on the PS2**, not on
  `normal_gravity` as this page states. One of the two pages is wrong.
- **PS2 gravity is scaled by `(1 - magLockBlend)`**, which this page records for
  vertical damping and bank-to-yaw but not for gravity. A fully mag-locked ship
  would then have no gravity at all.
- **PS2 lateral grip multiplies both grip coefficients by `1.5` under flag bit
  10** - the same bit that gates the turbo add. Not recorded here.

Two structural differences worth knowing before carrying any layout across:

- **The controls pointer is `craft+0x98` on the PS2**, against `craft+0x78`
  here. The member offsets inside it are unchanged: `+0x00` steer, `+0x04`
  thrust, `+0x10` pitch, `+0x44` buttons. **The struct moved; its contents did
  not.**
- **The PS2 has no craft-side accumulators.** `Body_ClearAccumulators`
  (`0x0015ca48`) zeroes `body+0x100`/`+0x110`/`+0x120`/`+0x130` directly and
  every term writes the body, so this page's `craft+0x320`/`+0x330`/`+0x340`/
  `+0x350` have no PS2 counterpart. Same four accumulators, one less copy.

**The ordering table is still not corroborated.** Every PS2 identification above
came from the callee list rather than from reading the call sequence, so the
fifteen-term order - and the stale-groundedness consequence that follows from it
- remains single-source. That is now the cheapest thing that would raise it.

Two things it does **not** corroborate, both recorded on that page: the ordering
of the fifteen force terms (the PS2 `Ship_UpdateCraft` was read only far enough
to find the hover and damping calls), and the bank-to-yaw coefficient, which is
`50.0` there against `30` here.

The `0x80` stride, all 32 field offsets, and all four load-time scale factors
reproduce **exactly** in the PS2 loader, from parsers built by a different
compiler for a different ISA. That is the second-binary leg this page's own
Cross-platform note asked for, so the parameter-block layout should be read as
corroborated rather than merely self-consistent. The force law has since gained
the same kind of leg - see below.

**The PS2 build answers this page's `stats_base + 0x90` question.** It has a
`<Misc>` element - not mentioned anywhere on this page - whose parser writes
`width 0x78`, `length 0x7c`, `height 0x80`, `easyshield 0x84`,
`mediumshield 0x88`, `hardshield 0x8c` and **`weight_distribution 0x90`**. That
identifies the per-team scalar `Ship_UpdatePitch` reads. Whether the PSP build
has the same element was not checked; if it does not, the two builds diverge
here and `docs/formats/handling-stats.md` needs to say which is which.

## History

- 2026-08-03, third pass: **`g_class_gravity_scale` (`0x08ab0dcc`) raised 78 -> 90**
  and decoded end to end. Its writer is `<GlobalClass><GravityMul airborne/>` and
  its two readers are the gravity term here, whose `vmul.p` chain is now written
  out above. The finding worth carrying: **the attribute is named `airborne` and
  the scale is applied to the grounded term.** Also **corrects two physics pages**
  which recorded the shipped table as `1.0` for every class - it is not, and
  `angular-velocity-column.md`'s rest-compression derivation used the wrong scale
  as a result.
- 2026-08-03, second pass: **`craft+0x2cc` corrected**. It was going to be
  implemented as a contact ratio at confidence 60; it is an unbounded accumulator
  of seconds since flag `0x200`, and the scale it gates is not ported. Also
  established this pass, both at 90: the two speed-pad tables are filled
  **unscaled**, and `craft+0x298` is **re-armed every tick inside a pad** rather
  than set on entry. A third gate on the statistics block, `racer+0x368 == 0`,
  recorded but not reimplemented - nothing identifies what it selects. Step 15 of
  `Ship_UpdateCraft` is now implemented in `crates/physics`; this page's "it
  cannot be the missing resistance, the lead is closed" still stands, and closing
  the lead is not the same as leaving the term out.
- 2026-08-03: two readings in the speed-pad section corrected from the trigger
  side (see [pads.md](pads.md)). What `FUN_08887144` returns is a pad's
  **matrix**, not a track "section", so the push direction is row 2 of that
  matrix rather than a "direction" field; and the block guarded by "section
  changed" is pad-visit statistics, not lap and sector counting. Its one real
  consequence, a **+100 Zone score per pad**, recovered at 85 via `Zone_Update`.
- 2026-07-26: first pass. Parameter block layout 90 from the XML loader; force
  law 74-85 from decompilation; nothing runtime-verified.
- 2026-07-27: the angular sign convention resolved from the PS2 integrator, with
  this page's steering measurement as one of its two legs; the handedness
  question separated from it. Later the same day, eight force-law terms gained a
  second-binary leg from the PS2 build, lifting most of them off the 84 cap and
  turning up four cross-platform differences.
- 2026-07-28: `Ship_UpdateMagLock`'s last eighty instructions read - the basis
  rewrite, attribution 45 -> 90. Later the same day its axis source was settled
  from the format side (`+0xB10` is `SplinePt.down`, 70 -> 90), two details of
  the blend corrected (height-agreement weights, and a raycast fallback branch),
  and the mechanism implemented and measured against the inverted stretch of the
  `omega_*` lap.
