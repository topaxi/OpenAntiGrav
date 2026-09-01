# Positional audio

How a cue fired somewhere in the world becomes a volume and a stereo position.

[`sound.md`](sound.md) walks a cue name down to `sceSasSetVoice`; this page is
the other axis - what the game multiplies into that voice because the thing
making the noise is 80 units away and to the left. It is the reading
[`oag_game::audio::sfx`](../../../../crates/game/src/audio/sfx.rs) was missing
when it said "only the player's craft is audible here".

**Everything below is static reading of `psp-pulse-usa`'s `BOOT.BIN`, except the
pan table, which is read out of five shipped executables and matches a closed
form exactly.** The law itself is now runtime-verified - see
["Runtime verification"](#runtime-verification-2026-09-01) below - 2,910 live
samples across two sessions, 2,910 exact matches on both outputs.

## Read this before trusting an address on this page

This import carries **unrelocated address constants**: a `jal` prints its target
as `0x001351c4` and a global as `_DAT_002bde10`. **Add the image base
`0x08804000`** to get the real address - `0x001351c4` is `0x089391c4`. The same
trap is on [HANDOVER](../../../../HANDOVER.md) and it also silences every xref
tool in the bridge, so callers here were found with
`search_instructions(operand_pattern = "0x001351c4")`, not `get_xrefs_to`.

## The cast

| Address | Name | Confidence |
| --- | --- | --- |
| `0x089391c4` | `SoundEmitter_Init` | 90 |
| `0x08939720` | `SoundEmitter_Update` | 90 |
| `0x08939b84` | `SoundInstance_StillPlaying` | 80 |
| `0x08939c00` | `SoundEmitter_ComputeVolumeAndAngle` | 90 |
| `0x08939e58` | `SoundInstance_UpdateSpatial` | 86 |
| `0x089394dc` | `SoundEmitter_ServiceRequests` | 85 |
| `0x0893a2b0` | `SoundManager_Update` | 88 |
| `0x0893a5ec` | `SoundManager_LinkEmitter` | 85 |
| `0x0893a7e8` | `SoundManager_BuildVolumeCurve` | 94 |
| `0x0893a87c` | `SoundManager_VolumeCurve` | 94 |
| `0x08995a9c` | `Scream_PanVolumePair` | 85 |
| `0x08ac4a2c` | `g_scream_pan_table` (data) | 94 |

(`SoundInstance_StillPlaying` and `SoundEmitter_ServiceRequests` were found
2026-09-01 chasing why the zero-volume gate never fired live - see
["The zero-volume gate is dead code"](#the-zero-volume-gate-is-dead-code-2026-09-01)
below.)

`Sound_Play` (`0x089392b0`) and `Scream_PlaySoundByName` (`0x08991b20`) were
already named by [`sound.md`](sound.md); this page settles two of their
arguments.

## The shape of it

```text
SoundManager_Update  (once per frame)
  |
  +- copies the listener frame off the active camera object (DAT_08ab10b0)
  +- for each emitter in the manager's list:
       SoundEmitter_Update            distance, direction, cone angle, doppler delta,
                                       and the +0x5c bit 0 in-range/out-of-range latch
       if queued request count == 0: idle cleanup
       else: SoundEmitter_ServiceRequests, per queued request:
         if emitter+0x5c bit 0 is set (out of range): skip this request entirely
         else, if SoundInstance_StillPlaying:
           SoundInstance_UpdateSpatial
             SoundEmitter_ComputeVolumeAndAngle   -> volume 0..1024, angle 0..359
             Scream_SetSoundPan / Volume / Pitch  (or Scream_PlaySoundByName on the first frame)
               ...
             Scream_PanVolumePair               -> the two hardware volumes
```

## The emitter is a 0x70-byte record, and every craft owns one

`SoundEmitter_Init` (`0x089391c4`) is called on a `0x70`-byte allocation. Its
whole body is the defaults, so the record's field meanings are pinned three
ways: what `Init` writes, what `SoundEmitter_Update` writes, and what
`SoundEmitter_ComputeVolumeAndAngle` reads. All three agree.

| Offset | Meaning | `Init` default |
| --- | --- | --- |
| `+0x00` | emitter world position, `vec4` | - (copied each frame) |
| `+0x10` | `listener_pos - emitter_pos`, `vec4` | - |
| `+0x30` | distance to the listener | `-1.0`, a "no previous frame" sentinel |
| `+0x34` | distance **change** since last frame (the doppler term) | `0` |
| `+0x38` | **radius** | `200.0` (`0x43480000`) |
| `+0x40` | cone half-angle, radians | `pi/2` (`0x3fc90fdb`) |
| `+0x48` | angle from the cone axis to the listener, radians | `FLT_MAX` (`0x7f7fffff`) |
| `+0x4c` | `u8`, cone enabled | `0` |
| `+0x4d` | `u8`, skip this emitter entirely | `0` |
| `+0x50` | the **scene node** the position is read off | set by the caller |
| `+0x54` | queued request count | `0` |
| `+0x58` | queued request list head (`0x30`-byte nodes) | `0` |
| `+0x5c` | flags: bit 0 out of range, bit 1 disabled | `0` |
| `+0x60` / `+0x64` | manager list prev / next | `0` |
| `+0x68` / `+0x69` | `u8` edges: came into range / went out of range | - |

`Init` ends by calling `SoundManager_LinkEmitter` (`0x0893a5ec`), which pushes
the record onto the manager's list at `mgr+0x88` and bumps the count at
`mgr+0x164`. So **an emitter is live from construction**; nothing has to
register it.

**There are fourteen construction sites**, found by searching for
`jal 0x001351c4`:

| Site | What owns the emitter |
| --- | --- |
| `Craft_Construct_q` `0x088417d0` | **every racing craft** |
| `ExhaustFlare_Init` `0x0890570c` | the `~ENGINE` note, one per craft |
| `Missile_Init` `0x0885a898`, `Rocket_Init` `0x0885d118` | projectiles |
| `Mine_Init` `0x08859c68` | already named, not previously cross-referenced here |
| nine more, all unnamed | `0x08858410` and `0x088584b8` (same function, two sites), `0x0885bdc8`, `0x08863298`, `0x08873e70`, `0x08875390`, `0x08877560`, `0x0891d924`, `0x08925a84` - none has a static caller (all 8 owning functions show zero xrefs, consistent with a class/table dispatch this codebase uses elsewhere). One lead, not yet confidence 50: the owner of `0x08858410`/`0x088584b8` frees its own emitter immediately after one `Sound_Play` call each, at `+0x38` radius `300.0`, and the sound-descriptor pointer for the first branch resolves to the string `CANNONEXPLWALL` - suggestive of a shared weapon-impact-effects pool rather than per-projectile construction, but the second branch's descriptor did not resolve as cleanly and this was not chased further. |

`Craft_Construct_q` is the one that matters most here, and it is four
instructions:

```asm
088417bc  jal   0x00142ce4          ; alloc(0x70)
088417c0  _li   a0, 0x70
088417d0  jal   0x001351c4          ; SoundEmitter_Init
088417dc  sw    s5, 0x50(s0)        ; craft+0x50 = emitter
088417e0  lw    a0, 0x794(s0)       ; the craft's scene node
088417e8  sw    a0, 0x50(s5)        ; emitter+0x50 = that node
```

`*(craft + 0x794)` is the ship's scene node, which
[`camera.md`](camera.md) already recovered independently ("ship node =
`*(craft + 0x794)`, a 4x4 at `+0x00`, rows at `+0x00`/`+0x10`/`+0x20`"), and
`SoundEmitter_Update` reads its translation at `+0x30`. Two subsystems, one
struct, same offsets.

**The craft emitter never overrides `+0x38`, so its radius is the `200.0`
default.** The engine flare's does: `ExhaustFlare_Init` writes `0x42480000` =
`50.0`. That is the whole reason a passing opponent's engine is quieter than its
collisions.

## The listener is the camera, and the manager copies its frame every frame

`SoundManager_Update` (`0x0893a2b0`) opens by copying four quadwords off
`DAT_08ab10b0`, the active camera object - the same global
[`exhaust.md`](exhaust.md) gates `Trail_DrawRibbon` on:

```text
mgr+0x40..0x4c  <-  camera+0x40..0x4c      rotation row 0
mgr+0x50..0x5c  <-  camera+0x50..0x5c      rotation row 1
mgr+0x60..0x6c  <-  camera+0x60..0x6c      rotation row 2
mgr+0x70..0x7c  <-  -(camera+0x70..0x7c)   negated
```

**This corroborates [`camera.md`](camera.md)'s transposition finding from a
completely different subsystem.** That page measured, off one live capture at
confidence 80, that "the camera's world axes are the stored matrix's *columns*
... which with the separately-stored negated eye is the shape of a view matrix
kept in two halves". The sound manager assumes exactly that and nothing else: it
negates `+0x70` to get a world-space eye, and - see below - it reads the world
right axis as `(m[0][0], m[1][0], m[2][0])`, the first **column**. A second
subsystem written by different people, agreeing on both halves, is worth more
than re-reading the camera code. Treat camera.md's finding as corroborated.

**The manager is reached through a global pointer, not held at a fixed
address itself** - the same "pointer at a fixed address" shape as
`G_STATE_MACHINE` (`scripts/ppsspp_debugger.py`) and the active-camera global
above. The decompiler renders it as `_DAT_002bde10`, one of this import's
unrelocated data-global constants (see the warning at the top of this page);
adding the image base gives `0x08ac1e10`, but that address holds a *pointer*
to the manager (confirmed in disassembly: `lui s4,0x2c` / `lw a0,-0x21f0(s4)`
loads the pointer, then `addiu a0,a0,0x40` indexes into what it points at) -
dereference it to get `mgr`. Runtime-confirmed 2026-09-01: `*0x08ac1e10` read
live as `0x08fb65f0`, a plausible heap pointer, and `mgr+0x168`/`mgr+0x16c`
off that address held exactly the gamma and volume-curve table this page
already read statically.

Other manager fields this page needs:

| Offset | Meaning |
| --- | --- |
| `+0x88` | emitter list head |
| `+0x8d` | `u8`, enable the doppler term this frame |
| `+0x94` | `24.0` - the listener-jump threshold that gates `+0x8d` |
| `+0x124` | `0x40` bytes of bank pointers, indexed by `Sound_Play`'s second argument |
| `+0x164` | emitter count |
| `+0x168` | `1.7` - the volume-curve gamma |
| `+0x16c` | 256 floats, the volume curve |

### A trap: the decompiler folds the teleport guard into nothing

The decompiler renders the guard as

```c
auVar1 = vsub_q(*(... *)(param_1 + 0x70), *(... *)(param_1 + 0x70));   /* zero */
*(bool *)(param_1 + 0x8d) = length(auVar1) < *(float *)(param_1 + 0x94);
```

which reads as dead code - a subtraction of a value from itself. **It is not.**
The disassembly uses two different stack slots and only one of them is the
post-write value:

```asm
0893a300  lv.q  C400, 0x0(a1)      ; a1 = mgr+0x70, the PREVIOUS frame's listener position
0893a304  sv.q  C400, 0x0(sp)      ; saved to sp+0x00 before anything is written
...
0893a360  sv.q  C400, 0x0(a1)      ; mgr+0x70 = the new listener position
0893a374  sv.q  C400, 0xa0(sp)     ; and to sp+0xa0
0893a378  lv.q  C300, 0x0(sp)      ; the OLD one
0893a384  vsub.q C320, C300, C310  ; old - new
```

So `mgr+0x8d` is **"the listener moved less than 24 units this frame"** - a
camera-cut guard that suppresses the doppler term across a hard cut. Both stack
slots trace back to a load of `mgr+0x70`, which is why the decompiler unified
them. Confidence **84**; the instruction order is unambiguous, the `24.0` is
read off the constructor.

## The law

`SoundEmitter_Update` (`0x08939720`), per emitter per frame:

```text
e[0x00]  = node[0x30..0x3c]                        ; the node's world translation
e[0x10]  = mgr[0x70] - e[0x00]                     ; emitter -> listener
e[0x30]  = |e[0x10].xyz|                           ; distance
if cone enabled (e[0x4c]):
    e[0x48] = acosf(clamp(dot(normalize(e[0x10].xyz), -node[0x10..0x1c]), -1, 1))
if previous distance >= 0 and mgr[0x8d]:
    e[0x34] = distance - previous distance
e[0x5c] bit 0 latches when the distance crosses e[0x38]; e[0x68]/e[0x69] are its edges
```

`SoundEmitter_ComputeVolumeAndAngle` (`0x08939c00`), per queued request:

```text
d     = e[0x30]
atten = (e[0x38] - d) / e[0x38]                    ; 1 - d/radius, LINEAR
if e[0x4c]: atten *= 1 - e[0x48] / e[0x40]         ; cone falloff, also linear
atten = min(atten * 1.25, 1.0)

right = (mgr[0x40], mgr[0x50], mgr[0x60])          ; column 0 = the listener's world right
c     = clamp(dot(normalize(e[0x10].xyz), right), -1, 1)
angle = degrees(acosf(c)) - 90
if angle < 0: angle += 360                         ; 0..359

volume = d > e[0x38] ? 0    ; DEAD CODE - see below, the caller never reaches
       : (int)( curve(atten * request_volume) * 1024.0 )  ; this function with d >= e[0x38]
```

Three things worth saying plainly:

- **The falloff is linear in distance, not inverse-square, and the `1.25`
  matters.** `atten * 1.25` clamped at `1.0` means an emitter is at *full*
  volume out to `0.2 * radius` and then falls linearly to nothing at `radius`.
  For a craft that is full volume within **40 units** and silent past **200**.
- **Beyond the radius the sound is not merely quiet, it is gated off** - but
  not, it turns out, by this function's own `d > radius` line. That line is
  provably unreachable; see
  ["The zero-volume gate is dead code"](#the-zero-volume-gate-is-dead-code-2026-09-01).
  The actual gate is one level up, in `SoundEmitter_ServiceRequests`, which
  refuses to call into this function at all once `+0x5c` bit 0 latches. And
  `Sound_Play` itself refuses to *start* a cue on an emitter whose `+0x5c` bit 0
  is latched, unless its sixth argument forces it. Out of range is a state, not
  a volume.
- **The angle loses front from back**, because `acos` is even. A source dead
  ahead and a source dead behind both produce `0`. That is correct for a machine
  with two speakers and is not an approximation introduced here.

### The volume curve is a gamma curve, built at boot

`SoundManager_BuildVolumeCurve` (`0x0893a7e8`) fills the 256 floats at
`mgr+0x16c`:

```c
mgr[0x168] = gamma;                 /* 1.7, from the constructor at 0x08939fe0 */
mgr[0x16c] = 0.0f;                  /* entry 0 */
for (i = 1; i < 256; i++)
    curve[i] = powf(i * (1.0f/255.0f), 1.0f / gamma);
```

and `SoundManager_VolumeCurve` (`0x0893a87c`) is the lookup: clamp to `[0,1]`,
index by `(int)(v * 255.0f)` - truncated, not rounded. Confidence **88**: the
formula is literally in the loop and the `1.7` is a literal in the manager's
constructor, but neither is runtime-verified.

### Doppler

`SoundInstance_UpdateSpatial` (`0x08939e58`), with `dt` as its float argument:

```c
pitch = (short)( (int)( -(e[0x34] / dt) * inst[0x0c] * 1536.0f ) + (int)inst[0x04] );
```

`inst[0x04]` is the instance's base pitch - for the engine note that is the
`base + speed_kmh * 5.0` value [`exhaust.md`](exhaust.md) recovers - and
`inst[0x0c]` is a per-instance doppler scale. The `1536.0` is read; **the unit
is not**, and it is the same unrecovered pitch unit `~ENGINE` already reads as
cents. Confidence **78**.

### The two hardware volumes, and the table that makes them

`SoundInstance_UpdateSpatial` hands the volume and the angle to SCREAM, which
carries them through an interpolator (`Scream_SetSoundPan`, `0x0898d05c`, whose
`0x168`/`0xb4` constants confirm the unit is **degrees**) and finally into
`Scream_PanVolumePair` (`0x08995a9c`):

```text
combined = p1 * p2 * p4 * p6 * 0x102 / 0x7f^3        ; 0..0x7ffe, some terms squared per a mode mask
if combined == 0: both volumes 0

a = wrap360(angle + per-voice offsets)
a = a < 270 ? a + 90 : a - 270

if a <= 179:  (L, R) =  (tbl[a].0, tbl[a].1) * combined / 0x3fff
else:         (L, R) = ~(tbl[a-180] swapped, one negated) * combined / 0x3fff
```

`tbl` is `g_scream_pan_table` at **`0x08ac4a2c`** (reached through a pointer at
`0x08ac4a28` that points at the word after itself): **180 entries of two
`s16`**.

**The table is `cos` and `sin` at half-degree steps, exactly.** All 360 values
satisfy

```text
tbl[i] = ( floor(16383 * cos(i * 0.5 degrees)),
           floor(16383 * sin(i * 0.5 degrees)) )
```

with **zero deviation** on every entry - `floor`, not `round`; entry 1 is
`(16382, 142)` where rounding would give `143`. So the pan is **equal-power**,
and the 180 entries span a quarter turn.

### The two rotations cancel, and the whole pan collapses

Write `phi = degrees(acos(c))`, the angle produced above, so `phi` is in
`[0, 180]`.

- `phi >= 90` gives `angle = phi - 90` in `[0, 90]`, and then `a = angle + 90 = phi`.
- `phi <  90` gives `angle = phi + 270` in `[270, 360)`, and then `a = angle - 270 = phi`.

**`a == phi` always.** The `- 90` in the emitter and the `+ 90` in SCREAM are
inverses, and the wrap exists only to keep the intermediate in `[0, 360)`. So
the entire pan law, from the dot product to the two speaker gains, is

```text
c = clamp(dot(normalize(listener_pos - emitter_pos), listener_right), -1, 1)
L = cos(acos(c) / 2) = sqrt((1 + c) / 2)
R = sin(acos(c) / 2) = sqrt((1 - c) / 2)
```

The half-angle identities remove the transcendental entirely: **the recovered
pan needs one square root and no `acos`, `sin` or `cos`.** That matters here
beyond tidiness - see [determinism](../../../architecture/determinism.md) and
`scripts/check-transcendentals.py`.

Sanity, with `c = dot(emitter -> listener, right)`:

| Source is | `c` | `phi` | `L` | `R` |
| --- | --- | --- | --- | --- |
| to the listener's **left** | `+1` | `0` | `1` | `0` |
| dead ahead **or** dead behind | `0` | `90` | `0.7071` | `0.7071` |
| to the listener's **right** | `-1` | `180` | `0` | `1` |

The sign reads backwards at first glance and is right: `e[0x10]` points from the
emitter *to* the listener, so a source on the right produces a vector pointing
left.

The `a > 179` branch phase-inverts one channel for the rear half of the circle.
**The game's own emitters never reach it** except at exactly hard right
(`phi = 180`, `a = 180`), because `phi` is capped at `180` by `acos`. Its sign
bookkeeping - which reads the previous frame's output signs - is therefore
unexercised on this path and was not chased.

## A consequence the numbers do not advertise

**Two seconds after the flag, the field is already out of engine range.** The
engine emitter's radius is `50.0` and the AI opens a gap immediately, so a race
left to itself renders the *player's* engine and nothing else - measured, not
predicted: `sfx_ground_truth::the_whole_grid_is_audible_and_not_all_from_one_place`
first asserted a stereo spread across the grid and got `0.00009`, because seven
of the eight voices were sitting at gain zero. It now moves a rival to the
camera's right on purpose.

That is what the recovered numbers say, and it is worth flagging rather than
tuning away: a "field of eight engines" is not what this law produces. The
collision and pad cues, at `200.0`, carry four times the range and are what a
player actually hears from the field. If a live capture ever shows rivals
audible further out than this, the radius or the `1.25` is where to look
first - both are read off one function and neither has been seen running.

## Runtime verification, 2026-09-01

**Nothing about this page was runtime-verified before this pass.** A live
Pulse USA session (`PPSSPPSDL` under Xvfb, SINGLE RACE, a full grid, player
held at full throttle to spread the field) was driven with
`scripts/psp-watch-soundemitter.py`, which arms one execution breakpoint at a
time - `SoundEmitter_ComputeVolumeAndAngle`'s entry, then its return address,
swapped back and forth, because **v1.20.4 only ever fires the
most-recently-armed execution breakpoint** and the function's two out-params
are stack slots in its caller that go stale once something else runs. At
entry it reads the emitter and manager fields the law consumes; at return it
reads the actual `volume`/`angle` the game computed; a Python reimplementation
of the closed form (float32 throughout) computes what it should have
produced.

**1,610 live hits, across five capture runs, 1,610 exact matches on both
outputs** (`/tmp/soundemitter*.csv`, not committed - regenerate with the
script above). Coverage was two of the three attenuation regimes: 283 hits
with `d < 0.2·radius` (clamped to full volume) and 1,327 with
`0.2·radius <= d < radius` (the linear ramp), `d/radius` up to `0.9994`. The
third regime - `d >= radius`, the separate zero-volume gate - was **not**
directly caught in **2,910 live hits total across two sessions**, including
one that deliberately targeted an emitter already reading `distance = 1009`,
`radius = 50`, queued-request-count `1` for over a minute of real time
without ever firing. That is not a sampling gap - see below, it cannot fire.

### The zero-volume gate is dead code, 2026-09-01

Chasing why a persistently-queued, persistently-out-of-range emitter never
once reached `SoundEmitter_ComputeVolumeAndAngle`'s own `if (radius < d)
volume = 0` line led to `SoundManager_Update`'s full per-emitter dispatch,
previously not decompiled on this page:

```c
// SoundManager_Update (0x0893a2b0), the per-emitter loop, addresses relocated
SoundEmitter_Update(emitter);                      // 0x00135720 + base = 0x08939720
if (emitter->queued_count == 0) { /* idle cleanup */ }
else SoundEmitter_ServiceRequests(mgr_dt, emitter); // 0x001354dc + base = 0x089394dc
```

`SoundEmitter_ServiceRequests` (`0x089394dc`) walks the emitter's queued
request list and, for each request still in the "keep" state, gates the
actual spatial update on the exact same flag `SoundEmitter_Update` just set
or cleared **one call earlier in the same tick**:

```c
if ((emitter->flags_0x5c & 1) == 0) {          // NOT out-of-range-latched
    if (SoundInstance_StillPlaying(request))    // 0x08939b84
        SoundInstance_UpdateSpatial(mgr_dt, request);  // 0x00135e58 + base = 0x08939e58
    // else: unlink and free the finished request
}
// if the latch IS set: nothing happens to this request at all - not even reaped
```

Both nested call targets resolve, after adding the image base, to addresses
this page had already named independently (`SoundInstance_UpdateSpatial` at
`0x08939e58`, and `SoundEmitter_Update` itself, called immediately before this
dispatcher, at `0x08939720`) - two exact matches, not a guess.

**`SoundEmitter_Update`'s own latch write happens before this gate is
checked, in the same frame:**

```c
if (!cone_enabled) {
    if ((flags_0x5c & 1) == 0) {              // currently in range
        if (radius <= distance) { flags_0x5c |= 1; edge_0x69 = 1; }   // -> latch NOW
    } else if (distance < radius) {
        flags_0x5c &= ~1; edge_0x68 = 1;                              // back in range
    }
}
```

So on the very tick `distance` first reaches `radius`, the latch is set
**before** `SoundEmitter_ServiceRequests` runs for that emitter this same
frame - there is no one-tick window where `SoundInstance_UpdateSpatial`, and
therefore `SoundEmitter_ComputeVolumeAndAngle`, gets called with `d >= radius`.
The latch also **clears** the moment `distance < radius` again, so a craft
re-entering range resumes normally - it is a live gate, not a one-way trap -
but the specific `if (radius < d) volume = 0` line inside
`SoundEmitter_ComputeVolumeAndAngle` is unreachable through this call chain in
every state the latch can be in. `SoundEmitter_ComputeVolumeAndAngle` has no
other caller on this page's own call graph.

**This resolves the open question rather than leaving it open**: the earlier
framing ("not yet observed, corroborating but not direct") undersold it. The
2,910-hit live capture matches what static reading now proves: the branch
cannot fire, so no capture will ever catch it firing. What *does* gate an
out-of-range emitter to silence is `SoundEmitter_ServiceRequests` refusing to
call into it at all - one level up from where this page originally placed the
gate.

**Two things fell out for free.** `inst[0x0c]`, the per-instance doppler
scale documented as "read as a field, never as a value", read as exactly
`0.0005` on every one of the 1,610 hits (all against the engine note, the only
cue that was continuously queued in this capture - not yet checked against a
collision or pickup cue). And `e[0x4c]`, the cone-enabled flag, read `False`
on all 1,610 - real negative evidence (not just "not looked at") that nothing
in a normal race turns the cone on, though it does not rule out a `.vex`
trigger this capture never exercised.

## Confidence, and what would raise it

| Claim | Score | Why not higher |
| --- | --- | --- |
| The pan table is `floor(16383 * cos/sin(i/2 deg))` | **94** | 360 exact values in five executables; the rubric caps data agreement at 94 |
| Linear falloff and the `1.25` clamp | **92** | runtime-exact on 1,610/1,610 live hits across the clamped and ramp regimes |
| Actual out-of-range gating is `SoundEmitter_ServiceRequests` refusing the call, not `d > radius` inside `ComputeVolumeAndAngle` | **90** | full call-chain decompilation, two independent exact address matches, and 2,910/2,910 live hits consistent with "cannot fire" |
| Emitter record layout | **90** | three functions agree, `Craft_Construct_q` matches camera.md's struct, and every field this page reads off it reproduced the game's own output live |
| Listener is the camera, columns are the world axes | **92** | camera.md measured it live from the other side, and the recovered angle law reproduced the game's own output exactly using this axis assignment |
| Volume curve `pow(v, 1/1.7)` over 256 steps | **94** | fingerprinted directly off live memory: gamma exactly `1.7`, table max deviation `~8e-8` (float32 precision), and it reproduced 1,610/1,610 exact volumes |
| Doppler `-(dd/dt) * scale * 1536` | 78 | the `1536` unit is still unrecovered; the per-instance scale is now a read value (`0.0005`) rather than an unread field, not yet a confirmed unit |
| The teleport guard is `|old - new| < 24` | 84 | untouched by this capture - different function |

**The cheapest upgrade this page named is done, and so is its own follow-up
question.** What is left in the same shape: the doppler unit (needs an
audible reference, not a register read), and EU cross-verification of the
code (below).

## Cross-title: four of six discs carry the same table, and the PS2 does not

The 720-byte table was searched for byte-for-byte in every executable under
`data/extracted/`, in both byte orders:

| Executable | Result |
| --- | --- |
| `psp/pulse-usa/.../BOOT.BIN` | little-endian at file offset `0x2c0aac` |
| `psp/pulse-eu/.../BOOT.BIN` | little-endian at `0x2be6ec` |
| `psp/pure-usa/.../BOOT.BIN` | little-endian at `0x289c6c` |
| `psp/pure-eu/.../BOOT.BIN` | little-endian at `0x28346c` |
| `ps3/hdfury-eu/.../EBOOT.elf` | **big-endian** at `0x90ea9e` |
| `ps2/pulse-eu/SCES_547.48` | **absent, in both byte orders** |

Pure and Wipeout HD carrying the identical curve is what lets
`oag_game::audio::sfx` pan on all three titles without that being a Pulse
reading applied to another game - the caveat that module's doc comment carries
about *triggers* does not extend to this. HD holding it byte-swapped is the same
pattern the rest of that disc shows.

**The PS2 absence is a measured negative, not an unchecked one.** A second scan
looked for any 64-entry run of `floor(k * cos/sin(i * step))` pairs over
`k` in `{16383, 16384, 32767, 4095, 4096, 1023, 255, 127}`, `step` in
`{0.25, 0.5, 1.0}` degrees and both byte orders - 48 combinations, no hit. So
the PS2 build either computes its pan or tabulates it in a form none of those
describe. Unexplained, and worth a look by whoever next opens `SCES_547.48`.

## Not determined

- **Nine of the fourteen emitter construction sites** (2026-09-01: `Mine_Init`
  turned out to already be named, just not cross-referenced here, so the
  count moved from ten). Only craft, engine flare, missile, rocket and mine
  are identified; the remaining nine have no static caller each (indirect/
  table dispatch), and one - see the site table above - has a weapon-impact
  lead (`CANNONEXPLWALL`) that was not chased to a confident name.
- **Which class sets `+0x4c`.** The cone is implemented and defaulted off;
  2026-09-01's live capture read it `False` on all 1,610 samples (real
  negative evidence, not just "not looked at" - see
  ["Runtime verification"](#runtime-verification-2026-09-01)), but never
  exercised a `.vex` `soundcone` `0x3e9` in the field, so which class turns it
  on is still open - see the [class table](../../../formats/vex.md).
- **`inst[0x0c]`, the per-instance doppler scale, read `0.0005` live on every
  sample** (2026-09-01) - but only against the engine note, the only
  continuously-queued cue in that capture, and no construction site was
  traced, so a collision or pickup cue may carry a different value.
- **The mode mask.** `SoundInstance_UpdateSpatial`'s call passes `10`, which
  squares two of the four gain terms in `Scream_PanVolumePair`. Which two are
  which is read; *why* those two is not.
- **The `a > 179` sign bookkeeping**, unexercised on this path.
- **EU cross-verification of the code.** The table is confirmed in
  `psp-pulse-eu`; none of the functions is.
- **The doppler `1536` unit** is the one claim 2026-09-01's live capture did
  not reach - needs an audible reference, not a register read. (The
  `d > radius` zero-volume branch is no longer on this list: it is closed,
  provably dead code - see
  ["The zero-volume gate is dead code"](#the-zero-volume-gate-is-dead-code-2026-09-01).)
