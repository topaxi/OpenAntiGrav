# Collision feedback: sparks are shared with the PSP, the camera shake is not

Functions in `SCES_547.48` (Wipeout Pulse, PS2, SCES-54748), image base
`0x00100000`.

Found chasing the PS2 `Aspect Ratio` camera-widen (see
[camera.md](camera.md)), reading `FUN_0013e280` in full turned up a
shake-decay block at its start. The PSP's own collision-response path was
read twice and found to call no camera API at all - "the 'camera shake'
reading this page previously carried was a guess from the `0.0125` scale
alone and is **retracted**"
([contact-response.md](../psp-pulse-usa/contact-response.md#fun_088418e0s-contact-loop-drives-three-separate-reactions-and-one-of-them-is-a-particle)).
The PS2 binary's equivalent function does something the PSP's was verified not
to: it calls into a real, working camera-shake mechanism, in the same
conditional slot where the PSP's plays an audio cue instead.

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

**Where the PSP's equivalent stops at a sound cue, this one also arms a
shake.** `contact-response.md` is explicit that `Ship_DispatchCollisionFx`'s
body calls no camera API, checked by a full decompile specifically looking for
one; the PS2 function occupies the *same* slot (spark first, an unconditional
call; then, gated on local-player, a per-hit-direction branch) with a shake
call where the PSP has a sound call instead. That is either a genuine
platform difference or the PSP's shake (if it exists at all) is wired
somewhere this function's own call graph never reaches - not yet
distinguished, see Open below. `FUN_00159268(craft)` (not named - a plain
struct-field walk, low value to name on its own) is read as "the camera for
this craft" purely from being self-consistent with `Camera_ArmShake`'s own
writes, below.

## `Camera_ArmShake` (`0x0013eec0`)

**Decompiled in full**, a single-purpose setter:

```c
void Camera_ArmShake(float magnitude, float duration, Camera *cam, int mode)
{
    if (currently-in-replay-mode-2) return;      // gated the same way FUN_0013e280 is
    cam->shake_timer          = duration;          // +0x194
    cam->shake_period_a       = magnitude * 0.25;   // +0x198
    cam->shake_period_b       = magnitude * 0.125;  // +0x19c
    cam->shake_duration_recip = 1.0 / duration;     // +0x190
    cam->shake_magnitude      = magnitude;          // +0x1cc
    cam->shake_mode           = mode;               // +0x1c8
    cam->shake_phase_rng      = Rng_RangeF(0.2, 0.8);  // +0x1d0, via FUN_001cc100
    cam->falloff_write        = 0;                  // +0x1a0, +0x1a4 - envelope-table reset
    // precomputes 1/(key[n+1]-key[n]) into +0x1b0+n*4 for the falloff table's lerp
}
```

`Rng_RangeF`-shaped: `FUN_001cc100(0.2, 0.8)` is the same "random float in a
range" shape `oag_render::exhaust`'s own `FLICKER`/`Rng_RangeF(0.75, 1.25)`
already reproduces for the engine flare, on a different function - not
renamed here, out of scope for this page.

## `FUN_0013e280` applies the shake every frame - still not renamed

See [camera.md](camera.md#the-aspect-ratio-options-widen-at-one-address-inside-a-much-bigger-function)
for why the containing function stays unnamed. The shake-apply block at its
start, now read in light of `Camera_ArmShake`'s writes:

- Bails immediately if `shake_timer <= 0.0` or this camera is not the process's
  single active one (`DAT_0027e7f0 != this`).
- Computes a normalised progress `p = shake_timer * shake_duration_recip`
  (`0..1` as the timer counts down), looks up a keyframed falloff envelope
  from it (the `+0x1a4` table `Camera_ArmShake` reset and precomputed lerp
  factors for), and drives **two** damped, phase-offset oscillations from it
  via `FUN_0020cf50` (a `sin`-shaped call, unread beyond that) - one at the
  base rate, one at `1.5x` and phase-shifted by the armed random `shake_phase_rng`.
- `shake_mode` (`1`/`3`/else) selects which of three ways the two oscillation
  terms combine and which axis of a `+0x40` offset vector they're added into,
  applied twice per call (`FUN_0025cbb0`, not identified - a vec3-component
  accumulator by its call shape) - consistent with `Camera_ArmShake`'s
  `mode = 3` (ahead) vs `mode = 1` (elsewhere) branch.
- Decrements `shake_timer` by a per-call constant (`+0x1d4`) on the way out.

None of that offset vector's own consumer was traced in this pass - whether
`+0x40` reaches the eye position `FUN_0013e280` later uploads to the GS
display stack, or something else entirely, is unconfirmed.

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

- Whether the PSP disc genuinely lacks this shake (a real platform
  difference) or has it wired through a path `contact-response.md`'s search
  didn't reach - `Ship_DispatchCollisionFx`'s own body was the thing checked
  there, not the PSP's per-frame camera-update function itself for an
  internal shake-apply block matching this one's shape.
- `+0x40`'s consumer inside `FUN_0013e280` - not traced past the accumulator
  calls.
- `FUN_0020cf50`/`FUN_0025cbb0`/`FUN_00159268`/`FUN_001cc100` - read only for
  the one call shape each was seen in here, none renamed.
- `DAT_0027e8dc`/`DAT_0027e8e0` (the shake magnitude/duration scale constants)
  and `FUN_00155188`'s own eight-way state switch - unread beyond what this
  page needed.
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
  not the HUD bar the observation may also have meant.
