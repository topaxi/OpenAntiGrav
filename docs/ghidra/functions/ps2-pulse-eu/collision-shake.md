# Collision feedback: sparks, a shield-bubble flash, and a camera shake - all three shared with the PSP

Functions in `SCES_547.48` (Wipeout Pulse, PS2, SCES-54748), image base
`0x00100000`.

Found chasing the PS2 `Aspect Ratio` camera-widen (see
[camera.md](camera.md)), reading `FUN_0013e280` in full turned up a
shake-decay block at its start. The PSP's own collision-response path had a
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
as "periods" - re-reading `FUN_0013e280`'s own consumption (below) makes
these unambiguous: a **three-keyframe, piecewise-linear falloff envelope**,
positions and values both authored by this function:

```c
void Camera_ArmShake(float magnitude, float duration, Camera *cam, int mode)
{
    if (currently-in-replay-mode-2) return;      // gated the same way FUN_0013e280 is
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
`oag_render::exhaust`'s own keyframed intensity ramps.

**The two arm-call scale constants, read directly**: `FUN_00154cf8` (above)
calls `Camera_ArmShake(DAT_0027e8dc * severity, DAT_0027e8e0, ...)`.
`DAT_0027e8dc` = `0x3e99999a` = **0.3** (magnitude scale), `DAT_0027e8e0` =
`0x3f19999a` = **0.6** (duration, in seconds - not scaled by severity at all).
The PSP's own `DAT_08ab0dfc`/`DAT_08ab0e00` are the **same bytes** at the same
role, confirmed by reading both binaries' `.data`. So a full-severity hit
(`severity = 1.0`) arms `magnitude = 0.3`, `duration = 0.6s`, every collision
on both platforms; only the severity scalar varies shake to shake.

`Rng_RangeF`-shaped: `FUN_001cc100(0.2, 0.8)` is the same "random float in a
range" shape `oag_render::exhaust`'s own `FLICKER`/`Rng_RangeF(0.75, 1.25)`
already reproduces for the engine flare, on a different function - not
renamed here, out of scope for this page.

## `FUN_0013e280` applies the shake every frame - still not renamed

See [camera.md](camera.md#the-aspect-ratio-options-widen-at-one-address-inside-a-much-bigger-function)
for why the containing function stays unnamed. **The shake-apply block at its
start was re-read in full** (not just at call-shape depth) to settle what it
actually perturbs:

- Bails immediately if `shake_timer <= 0.0` or this camera is not the process's
  single active one (`DAT_0027e7f0 != this`).
- Computes elapsed progress `p = 1.0 - shake_timer * shake_duration_recip`
  (`0..1` as the timer counts down from `duration`), optionally re-wraps it
  through a period (`+0x1c0`) and fractional scale (`+0x1c4`) when a period is
  armed, then walks the three-keyframe falloff table above to get the current
  envelope value.
- Drives **two** damped, phase-offset oscillations from it via `FUN_0020cf50`
  (a `sin`-shaped call, unread beyond that) - one at a base frequency
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

The rotation *axis* is not settled: Ghidra drops the same non-GPR argument at
every call level here (`FUN_0013e280` -> `FUN_0025cbb0` -> `FUN_0025ca48`),
so nothing in the decompiled C text names which register carries it, and
reading it out needs raw p-code/register dataflow rather than the decompiler's
one-function-at-a-time view. `mode`'s `1`/`3`/else split plausibly picks
between two different axes (matching the doc's earlier "front vs behind"
framing) rather than just negating a shared one, but that's a hypothesis, not
a read.

- Decrements `shake_timer` by a per-call constant (`+0x1d4`) on the way out.

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

- The rotation axis `FUN_0025ca48` builds its matrix from - Ghidra drops the
  carrying register from the decompiled signature at every call level; needs
  a p-code/register-dataflow read, not another decompile.
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
- **Nothing here was verified at runtime.**

## History

- 2026-09-03: first pass, from reading `FUN_0013e280` in full while writing up
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
  `FUN_0013e280` in full rather than by call shape alone, prompted by a
  reviewer catching that the earlier pass's pseudocode silently dropped two
  of `Camera_ArmShake`'s literal writes. That surfaced the exact three-key
  falloff envelope and the two arm-call scale constants (both read directly
  from `.data`), and - the bigger correction - that `FUN_0025cbb0` performs a
  matrix rotation of the camera's own basis, not a translation of a `+0x40`
  offset vector as the call shape alone had suggested. Cross-checked against
  the PSP's `func_0x002676b4`/`func_0x00267820`, which do the identical
  Rodrigues-rotation-then-`vmmul_t` in an independent binary.
