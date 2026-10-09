# Collision feedback: sparks, a shield-bubble flash, and a camera shake - all three shared with the PSP

Functions in `SCES_547.48` (Wipeout Pulse, PS2, SCES-54748), image base
`0x00100000`.

Found chasing the PS2 `Aspect Ratio` camera-widen (see
[camera.md](camera.md)), reading `Camera_SubmitScene` (`0x0013e280`, named
2026-09-05 once this page's shake block and camera.md's own frustum-plane
reading were both complete) in full turned up a shake-decay block at its
start. The PSP's own collision-response path had a
"camera shake" reading retracted for calling no camera API at all
([contact-response.md](../psp-pulse-usa/contact-response.md#fun_088418e0s-contact-loop-drives-three-separate-reactions-and-one-of-them-is-a-particle))
- checked closely enough to be right about the function, and wrong about the
conclusion: the retraction rested on an *unread* second call it guessed was a
sound cue. It's `Camera_ArmShake` there too, same as here - see the
correction below.

Also found on the same pass, from the same contact loop: `ShipShield_Hit`, the
shield-pickup bubble's hit-flash, already named on the PSP
([shield-pickup.md](../psp-pulse-usa/shield-pickup.md)) and matched here by
structure rather than by any shared search - **this is the shield-bubble
pickup's own hit reaction, not the HUD's persistent energy bar**, which is a
different, still-open question
([hud.md](../../../ui/hud.md#the-place-and-the-total-time-are-authored-at-one-anchor-so-one-of-them-has-to-go)).

**The names below are applied**, from [names.tsv](names.tsv).

| Address | Name | Conf |
| --- | --- | ---: |
| `0x0013eec0` | `Camera_ArmShake` | 92 |
| `0x001eced0` | `ShipCollisionFx_Trigger` | 90 |
| `0x00169af8` | `ShipShield_Hit` | 85 |

## The severity clamp is the same formula, the same constant, a different binary

`FUN_00155188`, a large per-tick per-craft function (not otherwise
characterised here - it also dispatches an eight-way per-state switch nothing
in this repository has named yet), walks the craft's contact ring and computes,
per contact:

```c
fVar15 = 1.0;
if (fVar16 * 0.0125 <= 1.0) {
    fVar15 = fVar16 * 0.0125;
}
```

`fVar16` is `sqrt(...)` of a squared distance computed two statements above -
the same shape as `Body_RecordContact`'s impulse magnitude on the PSP side.
This is **the exact `min(|p| * 0.0125, 1.0)` clamp** `contact-response.md`
recovered for `Ship_DispatchCollisionFx`'s own severity argument, same
literal `0.0125`, in an independent binary. `fVar15` is then passed as the
first argument to `FUN_00154cf8(fVar15, craft, contactRecord)`.

## `FUN_00154cf8`: the nearest contact, a spark, and - only here - a shake

Not named: it does two distinct things and neither name would cover both.
**Decompiled in full.** It finds the nearest of up to ten `Ship Collision Fx`
slots (`craft + 0xd20`, distance to the contact point) exactly as the PSP's
`Ship_DispatchCollisionFx` does, then:

1. Unconditionally calls `ShipCollisionFx_Trigger(severity, thatInstance, 0, depth > 0.0)`
   - the spark spawn, confirmed by the literal strings inside it:
   `"WO_SHIP_COLL_SPARK_DAMAGE"` and `"WO_SHIP_SPARK_DAMAGE_LEACHBEAM"`,
   chosen by the same fourth-argument branch. This is the PS2 side of the
   hull-spark mechanism this project already implements against the PSP's
   `ShipCollisionFx_Trigger` (`0x089246b4`) - same name, same role, a
   different address on this binary.
2. **Only if the reacting craft is the local player**, and a state read
   (`FUN_001526d0(craft) == 1`) passes, tests whether the contact point is
   ahead of or behind the craft (a dot product against forward) and calls
   `Camera_ArmShake(DAT_0027e8dc * severity, DAT_0027e8e0, Camera_ForCraft(craft), mode)`
   - `mode = 3` ahead, `mode = 1` otherwise. `DAT_0027e8dc` and `DAT_0027e8e0`
   are an unread magnitude and duration scale, both in the same tight
   `0x0027e7xx`-`0x0027e8xx` global cluster [camera.md](camera.md) already
   places the per-player camera state in.

**Corrected same day: the PSP has the identical call in the identical slot.**
This page originally read `contact-response.md`'s retraction ("nothing in
`Ship_DispatchCollisionFx`'s body calls a camera API") as meaning the PSP
genuinely lacks this shake. It doesn't - the retraction was checking the
right function and drawing the wrong conclusion from an *unread* second call,
which `contact-response.md` had guessed was a sound cue at confidence 40 and
never actually decompiled. It is `Camera_ArmShake` there too (`0x08878750`,
renamed and documented on that page now), called with the same severity, the
same `[0.2, 0.8]` phase randomiser, and the same front/behind mode branch as
here. So this is not a platform difference: both binaries arm and apply the
same shake from the same collision event. `FUN_00159268(craft)` (not named - a
plain struct-field walk, low value to name on its own) is read as "the camera for
this craft" purely from being self-consistent with `Camera_ArmShake`'s own
writes, below.

## `Camera_ArmShake` (`0x0013eec0`)

**Decompiled in full**, a single-purpose setter. **Corrected 2026-09-03**: an
earlier pass here missed two literal writes and mislabelled `+0x198`/`+0x19c`
as "periods" - re-reading `Camera_SubmitScene`'s own consumption (below) makes
these unambiguous: a **three-keyframe, piecewise-linear falloff envelope**,
positions and values both authored by this function:

```c
void Camera_ArmShake(float magnitude, float duration, Camera *cam, int mode)
{
    if (currently-in-replay-mode-2) return;      // gated the same way Camera_SubmitScene is
    cam->shake_timer          = duration;          // +0x194
    cam->shake_duration_recip = 1.0 / duration;     // +0x190
    cam->falloff_value[0]     = magnitude * 0.25;   // +0x198, at position 0.0
    cam->falloff_value[1]     = magnitude * 0.125;  // +0x19c, at position 0.3
    cam->falloff_value[2]     = 0.0;                // +0x1a0, at position 1.0
    cam->falloff_pos[0]       = 0.0;                // +0x1a4
    cam->falloff_pos[1]       = 0.3;                // +0x1a8
    cam->falloff_pos[2]       = 1.0;                // +0x1ac
    cam->falloff_count        = 3;                  // +0x1bc
    cam->shake_magnitude      = magnitude;          // +0x1cc
    cam->shake_mode           = mode;               // +0x1c8
    cam->shake_phase_rng      = Rng_RangeF(0.2, 0.8);  // +0x1d0, via FUN_001cc100
    // precomputes 1/(pos[n]-pos[n-1]) into +0x1b0+n*4 for the falloff table's lerp
}
```

So the shake's strength is a fixed envelope over its own elapsed-progress
fraction, independent of `magnitude`'s own value: full strength
(`magnitude * 0.25`) at the moment of impact, down to `magnitude * 0.125` by
30% of the way through the shake's duration, down to `0.0` at the end - three
points, linearly interpolated between them, same table shape as
`oag_fx::exhaust`'s own keyframed intensity ramps.

**The two arm-call scale constants, read directly**: `FUN_00154cf8` (above)
calls `Camera_ArmShake(DAT_0027e8dc * severity, DAT_0027e8e0, ...)`.
`DAT_0027e8dc` = `0x3e99999a` = **0.3** (magnitude scale), `DAT_0027e8e0` =
`0x3f19999a` = **0.6** (duration, in seconds - not scaled by severity at all).
The PSP's own `DAT_08ab0dfc`/`DAT_08ab0e00` are the **same bytes** at the same
role, confirmed by reading both binaries' `.data`. So a full-severity hit
(`severity = 1.0`) arms `magnitude = 0.3`, `duration = 0.6s`, every collision
on both platforms; only the severity scalar varies shake to shake.

`Rng_RangeF`-shaped: `FUN_001cc100(0.2, 0.8)` is the same "random float in a
range" shape `oag_fx::exhaust`'s own `FLICKER`/`Rng_RangeF(0.75, 1.25)`
already reproduces for the engine flare, on a different function - not
renamed here, out of scope for this page.

## `Camera_SubmitScene` applies the shake every frame

This page's own reading of the shake-apply block, plus
[camera.md](camera.md#the-aspect-ratio-options-widen-at-one-address-inside-camera_submitscene)'s
reading of the frustum-plane block further down the same function, is what
earned `FUN_0013e280` its name on 2026-09-05 - see that page for the naming
rationale and confidence. **The shake-apply block at its start was re-read in
full** (not just at call-shape depth) to settle what it actually perturbs:

- Bails immediately if `shake_timer <= 0.0` or this camera is not the process's
  single active one (`DAT_0027e7f0 != this`).
- Computes elapsed progress `p = 1.0 - shake_timer * shake_duration_recip`
  (`0..1` as the timer counts down from `duration`), optionally re-wraps it
  through a period (`+0x1c0`) and fractional scale (`+0x1c4`) when a period is
  armed, then walks the three-keyframe falloff table above to get the current
  envelope value.
- Drives **two** damped, phase-offset oscillations from it via `FUN_0020cf50`
  (read as `sin`-shaped here at first; **it is the cosine** - measured 2026-09-30, see "Measured against the original") - one at a base frequency
  (`DAT_0027e7e0`), one at `1.5x` that frequency and phase-shifted by the
  armed random `shake_phase_rng`.
- `shake_mode` (`1`/`3`/else) selects which of three ways the envelope and the
  two oscillation terms combine into one or two angles, each applied via
  `FUN_0025cbb0(angle, param_1 + 0x40)`.

**Corrected 2026-09-03: `FUN_0025cbb0` rotates the camera's basis matrix - it
does not translate a position.** `param_1 + 0x40` is not a scratch offset
vector; it is the *first row of the camera's own 3x4 basis matrix*
(`+0x40`/`+0x50`/`+0x60`), read a few dozen lines further down this same
function to rebuild the view frustum. `FUN_0025cbb0` forwards its float angle
argument (dropped from Ghidra's shown signature, live in `$f12`, the same
class of gap the PSP relocation trap already trained us to expect) to
`FUN_0025ca48`, which builds a **Rodrigues axis-angle rotation matrix** from
that angle and an axis - the same `(1 << 7) >> 7` fixed-point-to-turn wrap
and `vcos`/`vsin` pair `func_0x00267820` uses on the PSP side (see
`contact-response.md`) - then `FUN_0025cbb0` itself `vmulabc`/`vmaddabc`s that
rotation into the camera's basis rows, matrix-multiply shaped exactly like
the PSP's `vmmul_t` at `func_0x002676b4` (`0x08a6b6b4`). **Both binaries agree
independently: the shake is an angular perturbation of the camera's
orientation, not a positional jitter of the eye.**

**Settled 2026-09-05: the rotation axis is not fixed at all - it is a live
snapshot of one of the camera's own three basis rows, picked by `shake_mode`,
and each mode applies *two* sequential rotations, not one.** `Camera_SubmitScene`'s
disassembly (not just its decompile) around the `FUN_0025cbb0` call sites
shows a fresh `lq a1,<offset>(s2)` immediately before every call, with nothing
between that load and the `jal` that could change `$a1` - the register the
call passes straight through into `FUN_0025ca48`'s `in_a1_qw`, whose `mtc1`/
`dsrl32`/`pcpyud` triplet (confirmed by decompile) fans exactly that
register's three lanes into the Rodrigues axis. Reading the register origin
this way needed no p-code beyond what the disassembly already shows:

```text
mode == 1: FUN_0025cbb0(angle = f[0x60(sp)] + shake, axis = s2+0x50)   // row1
           FUN_0025cbb0(angle = shake,                axis = s2+0x60)   // row2
mode == 3: FUN_0025cbb0(angle = -(f[0x60(sp)] + shake), axis = s2+0x50) // row1, negated
           FUN_0025cbb0(angle = shake,                   axis = s2+0x60) // row2
else:      FUN_0025cbb0(angle = shake,                   axis = s2+0x50) // row1
           FUN_0025cbb0(angle = osc2 * 0.06,              axis = s2+0x40) // row0 - the basis's own row, already rotated by the first call
```

(`s2+0x40`/`+0x50`/`+0x60` are the same three basis rows this page already
placed - see above.) Confidence **92**: every step is a direct disassembly
read with no interpretation, on one binary.

**Cross-checked on the PSP the same day, independently, at confidence 95.**
`Camera_SubmitScene` (`0x08878874`, [camera.md](../psp-pulse-usa/camera.md))
decompiles the equivalent block *without* the dropped-argument gap the PS2
copy has - the PSP's wrapper (`func_0x00266f04`, `0x08a6af04`, not yet named)
shows its axis as an explicit third pointer argument, not a register Ghidra
had to be walked past:

```c
// shake_mode == 1
func_0x00266f04(fVar13 + fVar22, param_1 + 0x40, param_1 + 0x50);  // row1
func_0x00266f04(fVar22,          param_1 + 0x40, param_1 + 0x60);  // row2
// shake_mode == 3
func_0x00266f04(-(fVar13 + fVar22), param_1 + 0x40, param_1 + 0x50); // row1, negated
func_0x00266f04(fVar22,             param_1 + 0x40, param_1 + 0x60); // row2
// else
func_0x00266f04(fVar22, param_1 + 0x40, param_1 + 0x50);            // row1
func_0x00266f04(fVar19 * fVar21 * fVar20 * 0.1 * 0.6, iVar9, iVar9); // row0 (iVar9 == param_1+0x40)
```

Same three offsets, same per-mode selection, same "second call's axis is the
matrix's own row0" shape in the `else` branch, in an independently-written
binary. This is not a hypothesis about which of "local right/up/forward" the
shake picks - the disc's own code does not select a fixed local axis at all;
it re-reads whichever basis row the current mode names, including reading
back the very row the first call in the `else` branch just rotated.

What this means for a from-scratch reimplementation: **a single fixed
rotation axis cannot reproduce this.** The two rotations per mode are about
different vectors (row1 then row2, or row1 then row0), composed on top of
each other, and the axes are the camera's own live orientation state, not
constants - so reproducing this needs the camera's current basis passed into
whatever plays the shake, not a module-level constant. `Shake::rotation`'s
own doc comment is corrected to say so rather than name a stand-in axis; see
`crates/render/src/camera/shake.rs`.

**What each row means physically (right/up/forward vs. the camera.md
transposition reading) is deliberately left open, at a much lower
confidence (~55) than the finding above and not needed to settle it.**
`positional-audio.md` established, for the same class of active-camera
object, that "the camera's world axes are the stored matrix's *columns*",
which would make a single *row* here the local representation of a world
axis rather than a local right/up/forward vector outright. One piece of
in-function corroboration for that reading: `Camera_SubmitScene` at
`0x0013e784`-`0x0013e7c8` sums the squares of lane 2 (the third element)
across all three rows and gates on the total - the squared length of
*column* 2, which only reads as a meaningful degeneracy guard if the
columns, not the rows, are the basis's real axes. Whether `s2` here is
provably the same struct camera.md examined was not chased - the axis
finding above does not depend on it.

- Decrements `shake_timer` by a per-call constant (`+0x1d4`) on the way out.

## Measured against the original, PSP Pulse USA (2026-09-30)

**Method.** PPSSPP v1.20.4 under Xvfb, Pulse PSP (USA) in a Talon's Junction Time Trial, its websocket
debugger. `Camera_SubmitScene` (`0x08878874`) is the PSP's apply site. Only the most recently armed
breakpoint fires on this build, so two breakpoints were swapped within one call: one at the entry
(read the camera struct, `0x130` bytes from the `a0` camera pointer: the basis rows `+0x40/+0x50/+0x60`
and the shake fields), one at `0x08878af0`, after the shake block and before the copy-out
(read `+0x40..+0x70` again). The pair is the shake's own input and output for one call, so a variable
timestep and the craft's motion do not enter. Scripts and raw captures: a scratch directory, not kept
(`cam_capture2.py`, `fit4.py`; `cap2`, `cap5`, `cap7`).

PSP camera struct offsets, the PS2's `+0x190..+0x1d4` block at its PSP place: `shake_timer +0xe4`, reciprocal
duration `+0xe0`, falloff values `+0xe8`, positions `+0xf4`, reciprocal gaps `+0x100`, count `+0x10c`,
period/fraction `+0x110/+0x114` (`0`, `1.0` on every capture), mode `+0x118`, magnitude `+0x11c`,
phase `+0x120`, per-call decrement `+0x124`; base frequency `0x08ab1098` (`30.0`).

**Captures.** `cap2`: a real wall scrape, `cross`+`left`, 300 calls, the shake armed on nearly every call
at magnitude `0.0009` to `0.025`. `cap5` and `cap7`: a second boot each of a stationary craft with the shake
fields **forced** at the entry breakpoint (`0.3` mode 1 and `0.15` mode 3; `0.2` mode 1 and `0.3` mode 3),
the whole envelope table written as `Camera_ArmShake` writes it, so a full-strength rotation and the
whole `0.6` s decay are measured. A forced arm measures the apply side only.

**Findings**, each against the model in `crates/render/src/camera/shake.rs` fit to every active call (72
per forced capture, 280 on the scrape), error the largest absolute difference of a basis component:

| Model | Error (forced, `0.3`) |
| --- | ---: |
| cosine oscillator, `Rodrigues(axis, -angle)`, second axis re-read after the first rotation - **the shipped one** | **3e-7** (float noise; worst 5.5e-7) |
| the same, second axis kept from before the first rotation | 2.8e-4 mean, 3.1e-3 worst |
| sine oscillator (what the code had) | 1.0e-2 |
| right-handed sense (`+angle`) | 4.7e-2 |

1. **The oscillator is a cosine, not a sine.** At progress `0` the original's second rotation is `0.1 *
   magnitude` and its first `(0.25 + 0.1) * magnitude`, read straight off the rotation vector between the two
   basis snapshots (`0.3500`, `0.0999` times magnitude, at every magnitude). The call the block makes
   at `0x0897e030` is Ghidra's `cosf` (`names.tsv`, `particle-system.md`), so this was a misread of the call
   shape, not of the arithmetic. Plain radians, argument `progress * 30`.
2. **The sense is opposite to the right-handed Rodrigues formula.** The basis rows are
   `(left, up, forward)` and `cross(row0, row1) = row2` on every recorded tick
   ([engine.md](../psp-pulse-usa/engine.md)), so the numbers are a right-handed world and this is a fact
   about which way the camera turns. Mode 1: the camera rotates about its own up by `-(envelope +
   oscillator)`; mode 3 (`Ahead`): `+(envelope + oscillator)`; the second rotation, by `-oscillator`, in both.
3. **The two axes are the camera's own up (`+0x50`) and then its forward (`+0x60`) in world coordinates,
   the second as the first rotation left it** - the two-rotation shape the 2026-09-05 section recovered,
   now with the handedness fixed. The basis rows are the camera's up and forward, **not** a view matrix's
   rows: a right-handed view matrix's rows are right, up and *back*, so a port reading `view.row(2)` has the
   wrong sign, and feeding world-space axes into a view-space multiply mixes frames.
4. **No accumulation.** With the craft at rest the entry basis of call `n + 1` is identical to call `n`'s to
   `9e-5` (hover drift) while the copy-out differs from it by up to `0.1`: the camera update rebuilds the
   basis every frame, so the in-place rotation is per-frame only. This closes "whether the basis-row writes
   are read back before or after the copy-out" for the PSP: the shake is applied before the copy-out
   (`0x08878af0` precedes the `lv.q` at `0x08878af4`), and does not persist into the next frame. The
   PS2's `FUN_0013e280` was not checked at runtime.
5. **Timing.** The per-call decrement (`+0x124`) is the frame's measured `dt` (`0.0167 +- 0.0003`), not a
   constant. `Camera_ArmShake` runs during the craft update, before the same frame's submit, so the first
   frame after an arm sees the full `0.6` s timer (progress `0`), and the decrement comes after the use.
   `Camera_ArmShake` itself (`0x08878750`), broken on during a scrape: `f12` the magnitude, `f13 = 0.6` on
   every one of 40 calls, `a1` the mode. It is called on nearly every frame of a scrape, at small severity
   (`0.0015` to `0.025` magnitude, so severity `0.005` to `0.08`): the visible shake of an ordinary
   scrape is under a degree, and only a hard hit (severity near `1`) reaches `0.3`, six degrees.

**Not covered:** the third `shake_mode` combination (nothing arms it), a hard wall hit (a real hit at
severity near `1` was not produced; the forced arm stands in for it), the PS2 binary at runtime, and the
internal (cockpit) view, whose submit goes through the same function.

## `ShipShield_Hit` (`0x00169af8`) - the same mechanism the PSP has, at a different address

**Decompiled in full**, four stores:

```c
void ShipShield_Hit(ShieldBubble *shield)
{
    shield->colour   = HIT_COLOUR;   // +0xc0, a vec4 read from .bss (unreadable statically - a
                                      // runtime-written static initialiser, same trap as PSP's)
    shield->swell    = 0x3f8ccccd;   // +0xe4, the literal 1.1
}
```

Matches the PSP's own `ShipShield_Hit` (`0x0885eb04`, confidence 80,
[shield-pickup.md](../psp-pulse-usa/shield-pickup.md)) closely enough to carry
the name across with only a small confidence discount for the untraced colour
constant: same literal `1.1`, same "colour vec4 plus a swell/duration scalar"
shape, and **the same two-caller count** - `FUN_00153328` and `FUN_00155188`
(the latter the same per-tick function the shake severity above is computed
in) are this binary's only two callers, matching "a whole-image scan finds its
two known callers" on the PSP page exactly. This is the shield **pickup
bubble's** hit-flash - the temporary invincibility item's model re-flashing and
swelling when it absorbs a hit - not the HUD's persistent energy-bar colour,
which stays a separate, open question.

## Not determined

- ~~What each basis row physically represents~~ **Answered for the PSP, 2026-09-30**: the rows are
  `(left, up, forward)` in world coordinates and the shake rotates those vectors - see "Measured against
  the original".
- ~~Whether `s2+0x40..0x60` is mutated in place by the shake before the copy-out, and whether it
  accumulates~~ **Answered for the PSP, 2026-09-30**: mutated in place before the copy-out, and it does
  **not** accumulate - see "Measured against the original". Not checked on the PS2 at runtime.
- `FUN_0020cf50`/`FUN_0025cbb0`/`FUN_0025ca48`/`FUN_00159268`/`FUN_001cc100` -
  read only for the one call shape each was seen in here (`FUN_0025cbb0` and
  `FUN_0025ca48` now decompiled in full, the other three not), none renamed.
- `FUN_00155188`'s own eight-way state switch - unread beyond what this page
  needed.
- Whether `+0x1c0`/`+0x1c4` (the optional period/fractional-scale re-wrap) are
  ever actually armed by anything on the collision path, or only by some other
  caller of the shared apply block - not traced.
- `ShipShield_Hit`'s colour constant (`+0xc0`, a `.bss` vec4) - unreadable
  statically, same static-initialiser trap as the PSP's own three colour
  vec4s; not chased further since the PSP page already reads it as cyan
  `(0, 1, 1, 1)` and the point here was the structural match, not the value.
- `FUN_00153328`, `ShipShield_Hit`'s other caller - not read at all.
- **Verified at runtime on the PSP 2026-09-30** (the section above); the PS2 binary was not.

## History

- 2026-09-03: first pass, from reading `Camera_SubmitScene` in full while writing up
  [camera.md](camera.md)'s `Aspect Ratio` section. `ShipShield_Hit` added the
  same day, prompted by a maintainer's own play-testing observation of a
  shield element flashing on a hit - traced to the pickup bubble's mechanism,
  not the HUD bar the observation may also have meant. Later the same day, a
  maintainer's memory of playing PSP/Pure ("this happens there too") sent the
  search back to `contact-response.md`, which corrected its own "camera
  shake... retracted" conclusion in place once its unread second call turned
  out to be the PSP's own `Camera_ArmShake` - the platform-difference framing
  earlier on this page did not survive that.
- 2026-09-03, same day, before implementing: re-read `Camera_ArmShake` and
  `Camera_SubmitScene` in full rather than by call shape alone, prompted by a
  reviewer catching that the earlier pass's pseudocode silently dropped two
  of `Camera_ArmShake`'s literal writes. That surfaced the exact three-key
  falloff envelope and the two arm-call scale constants (both read directly
  from `.data`), and - the bigger correction - that `FUN_0025cbb0` performs a
  matrix rotation of the camera's own basis, not a translation of a `+0x40`
  offset vector as the call shape alone had suggested. Cross-checked against
  the PSP's `func_0x002676b4`/`func_0x00267820`, which do the identical
  Rodrigues-rotation-then-`vmmul_t` in an independent binary.
- 2026-09-05: the rotation axis, this page's own longest-standing "not
  determined" item, is settled - a live basis row (`+0x40`/`+0x50`/`+0x60`),
  picked by `shake_mode`, applied as two sequential rotations per frame, not
  a fixed constant. Read straight off `Camera_SubmitScene`'s disassembly (no p-code
  needed - the register origin was visible from the `lq a1` right before each
  call) and cross-checked independently on the PSP's `Camera_SubmitScene`,
  whose decompile shows the same axis as an explicit pointer argument at the
  same three offsets. `oag_render::camera::shake`'s doc comment is corrected
  to stop presenting a fixed "local right" axis as this module's own
  placeholder for an unconfirmed reading - the disc's own code does not pick
  a fixed axis at all, so no single constant is a faithful stand-in, and the
  module's rotation arithmetic is left as a known simplification rather than
  rewritten to a shape (axis input into `Shake::rotation`) this pass did not
  scope.
- 2026-09-30: measured against the running PSP original (see "Measured against the original"). The
  oscillator is a cosine, the rotation sense is opposite to the right-handed formula, the second axis is
  re-read after the first rotation, the basis is rebuilt each frame (no accumulation), and the first frame
  after an arm sees the full timer. `Shake::rotation` was wrong on the first two and on which vectors it
  was given, `Race::view` on frames and on the order of advance and arm; both are fixed and pinned by
  `crates/render/tests/shake_ground_truth.rs`.
