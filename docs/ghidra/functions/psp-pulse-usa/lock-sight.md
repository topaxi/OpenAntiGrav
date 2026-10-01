# The lock-on sight, and the tone that goes with it

**Binary:** `pulse-psp` `BOOT.BIN`, image base `0x08804000`.

**Status:** read end to end. The reticle the Missile and the LeachBeam put over
the craft they are locking is **nine HUD widgets over three models**, and the
function that drives it also writes the flag `Ship_FireHeldWeapon` gates the lock
on - so this page answers a question [missile.md](missile.md) left open: **the
lock is not instant.** It takes `0.8` seconds of holding a target on screen.

| Address | Name | Confidence |
| --- | --- | --- |
| `0x0881dbcc` | `HudSight_Update` | 88 |
| `0x0881e8c8` | `HudSight_UpdateLeachBeam` | 92 |
| `0x0881b478` | `HudSight_RotateAboutCentre` | 92 |
| `0x0881b604` | `HudSight_Bind` | 90 |
| `0x0881b34c` | `HudSight_UpdateTone` | 85 |
| `0x0883b358` | `Hud_ResolveLockTarget` | 80 |

Read [missile.md](missile.md) first for `Ship_AcquireLock` and what a lock is
*for*; this page is what the player sees and hears while one is being taken.

## Nine widgets, three models, and the difference matters

`HudSight_Bind` (`0x0881b604`) looks nine widgets up by name under the `"HUD->"`
path and stores each into its own slot on the HUD object:

| Slot | Widget | Model it instances |
| --- | --- | --- |
| `+0xd8` | `missile_sight_1` | `Data\HUD\missile_sight_outer.vex` |
| `+0xdc` | `missile_sight_2` | the same |
| `+0xe0` | `missile_sight_3` | the same |
| `+0xe4` | `missile_sight_4` | the same |
| `+0xec` | `missile_sight_inner` | `Data\HUD\missile_sight_inner.vex` |
| `+0x108` | `leachbeam_sight_1` | `Data\HUD\leachbeam_sight.vex` |
| `+0x10c` | `leachbeam_sight_2` | the same |
| `+0x110` | `leachbeam_sight_3` | the same |
| `+0x114` | `leachbeam_sight_4` | the same |

**`+0xe8` is skipped by the bind and is not a tenth widget** - it is the *hold
timer* `HudSight_Update` accumulates, sitting in the middle of the slot run. See
below.

**The widget names are in `BOOT.BIN`; the model names are only in the layout
XML**, and conflating the two is easy: `docs/ui/hud.md` and
`oag_game::hud::widget::Model` both named the models (`missile_sight_inner` /
`_outer`, `leachbeam_sight`) and were **correct about them**. What is new here is
that there are *nine widgets over three models* - four instances of one bracket,
four of one arrowhead, one inner - which is what makes the reticle a reticle.

All nine are authored in `Arcade_HUD.xml`'s first `<Mode3D>` block, which
declares `mode="orthographic"`, and **all nine carry the same
`x="-240" y="136.0" z="0" ztest="0"`**. An identical authored position on nine
widgets is a placeholder: the runtime writes `widget+0x98`/`+0x9c` every frame.

### What the models are

Each is a single textured quad, `8.0` units square, centred on the origin, four
vertices as one triangle strip with full `0..1` UVs - checked by decoding the
batches rather than by looking. What makes them a reticle is the texture:

- `missile_sight_outer.vex` is **one corner bracket**, an `⌐` shape.
- `missile_sight_inner.vex` is a **closed rounded box** outline.
- `leachbeam_sight.vex` is a **hollow triangle**, an arrowhead.

**Both sets are driven as of 2026-09-07**, and the difference between them is
the whole of what a title has to author: four names and, for the Missile alone,
a fifth. **Where the LeachBeam's four go is read, 2026-10-01**: by their own function,
`HudSight_UpdateLeachBeam` - see
[The LeachBeam's reticle is its own function](#the-leachbeams-reticle-is-its-own-function). `oag_race::sight::Held` carries which of the two is up and
`oag_title::hud::Sights::Brackets` carries the names, its `leach` field `None`
for a title that authors no such widget. What had kept the LeachBeam's four dark
was entirely upstream - `oag_tables::weapons` parsed no
`<Weapon type="LeachBeam">`, so `Ship_AcquireLock`'s second pair of offsets had
nothing behind them. See [The LeachBeam's own window](#the-leachbeams-own-window).

So the Missile's reticle is four brackets around a box with a closed box at its
centre, and the LeachBeam's is four arrowheads pointing inward. That the same
model appears four times is why the placement below writes a **rotation** per
instance.

## The LeachBeam's own window

`Ship_AcquireLock` (`0x08844784`) selects `stats+0x114`/`+0x118` when the held
weapon id is 10 and `stats+0x50`/`+0x54` when it is 1 - see
[missile.md](missile.md#the-lock) for the switch itself. **Everything after that
selection is shared**: the same `0.9` cone, the same 1.4 along-track screen, the
same nearest-by-longitudinal-distance tie-break, the same "not the firer, and
racing" skip. So the two lockable weapons differ by **two numbers and nothing
else**, which is why `oag_gameplay::projectile::missile::lock_window` takes the
numbers rather than a second stats type.

**And the two numbers differ, which is the finding.** Measured 2026-09-07
against `WeaponStats_Race.xml` and `WeaponStats_Elimination.xml` on both the USA
and the EU pressing: the LeachBeam's near bound equals the Missile's on all
four, and its **far bound is shorter on all four**. So the LeachBeam reaches
less far than the Missile does and its reticle takes a target later. No value is
quoted here or asserted anywhere, per
[ADR-0006](../../../architecture/adr/0006-no-copyrighted-content.md); the
*relation* is what `crates/tables/tests/weapons_ground_truth.rs` pins, and it
is the assertion that would catch a decoder reading one block for both.

Two further things the block says:

- **It authors no `slowdown_time`**, alone among the blocks
  `oag_tables::weapons` decodes, on all four tables. A weapon that fastens onto
  a craft and drains it is not one that charges a fixed slowdown on impact, so
  the absence reads as design; recorded because the module's own docs had
  claimed the attribute was universal.
- **It authors a `range` at the same figure as this page's own `250.0` draw
  range**, and that is a coincidence rather than the same number twice: the
  `250.0` above is a code literal in `HudSight_Update` and the `range` is
  tuning data for a beam nothing here fires. `LeachBeamStats` deliberately does
  not decode it, so nothing can grow a dependency between the two.

### The LeachBeam's reticle is its own function

**2026-10-01, confidence 92.** `Hud_Update` (`0x0881bf50`) calls `HudSight_Update`
and, when it returns no lock (`iVar2 == 0`), `FUN_0881e8c8` - **named
`HudSight_UpdateLeachBeam`** - which writes `+0x108` .. `+0x114`. This section
used to say nothing read those four and gave them the Missile's law as "chosen,
not measured". It is a different law, read at instruction level and **replayed
against 298 frames of the running game** (below).

```c
// gate: the held weapon, with a target, not flagged 0x1000
view = hud->view;                                   // hud + 0x3c
if (view->held_plus_one == 11 && view->leach_target != 0 &&    // +0x48, +0xec
    !(view->leach_target->flags & 0x1000) && (node = target->node)) {
    (sx, sy) = 240 + 240 * clip.x / clip.w, 136 + 136 * clip.y / clip.w;
    if (clip.w > 0 && dist < 250 && 0 <= sx < 480 && 0 <= sy < 272) {
        visible = true;  centre = (sx, sy);          // the raw projection: no chase,
        if (!view->seen) hud->extent = 30.0;         // no 2*new-old; first sight snaps open
    }
}
want, ref = visible ? (6.0, 6.0) : (30.0, 9.6);       // 9.6 = 6.0 * 1.6
step = dt * 50;  if (extent > ref) step *= 1.4;  if (!visible) step *= 1.5;  // DAT_08ab0a78
extent = ease(extent, want, step);                    // hud + 0x27c
locked = (extent == ref);                              // hud + 0xf8: arrival is the lock
spin = visible ? spin + dt * (locked ? 4.0 : 2.0)      // hud + 0x104; DAT_08ab0a7c, a80
               : spin - 2.0 * dt;
spin = fmodf(spin, 2*pi);
corner[i] = rotate(centre + (+-extent, +-extent), about centre, spin);   // FUN_0881b478
piece[i].angle = spin + {pi/2, pi, 0, 3pi/2}[i];       // widget + 0xac
if (!visible && extent == 30.0) hide the four;         // else show them
alpha = visible ? 6.0 / extent : 1.0 - extent / 30.0;  // min(a * 255, 255), truncated
colour = alpha << 24 | (locked ? 0x0000ff : 0x00ffff); // red locked, yellow seeking
view->seen = visible;                                  // view + 0xf1
view->tone = visible || (craft->fire_word & 0x8000);   // view + 0xf0
```

What it settles:

- **The hide rule is the gate, and the shot closes it.** `view+0x48` carries the
  held weapon id plus one (read live: `0` with nothing held, `11` the tick after
  the id `10` was written into the weapon record, `2` for a Missile).
  `Weapon_FireLeachBeam` sets the craft's held-weapon slot to `-1`
  ([cannon-quake-leachbeam.md](cannon-quake-leachbeam.md)), so the gate fails on
  the fire tick, the arrowheads open at `75` to `105` units a second and are gone
  in about `0.24` seconds, **while the beam is still live** (read live: the
  extent was `11.5` four frames after the shot and `30.0` on the eighteenth, the
  beam flag held for 44 frames).
- **There is no hold timer.** The lock is the extent arriving at `6.0`, `0.34`
  seconds after a first sighting from `30.0`, against the Missile's `0.8`.
- **The whole figure spins**, `2.0` rad/s while seeking, `4.0` locked, and runs
  back at `-2.0` rad/s while open. That is the part the Missile's law does not
  have and the port did not draw.
- **The colour's alpha byte carries the fade** and its RGB is yellow or red with
  no blink, where the Missile scales RGB under an opaque alpha.

**Verified live, 2026-10-01** (PPSSPP v1.20.4, a Single Race, the LeachBeam id
written into the player's weapon record, an opponent held `60` units ahead
through the sight's own breakpoint, then the unlocked arm fired):
`scripts/psp-leach-sight-capture.py capture` reads `hud+0x27c`, `+0x104`,
`+0xf8`, `view+0xf1` and the frame's `dt` at every `HudSight_Update` entry, and
`replay` runs the arithmetic above on each step: **159 and 139 steps, worst extent
error under `1e-5`, worst spin error under `1e-6` rad, no `locked` disagreement,
across the first sighting, the lock, the spin at both rates, the shot, the
opening and the hide.** The control that the replay can fail: with the opening
rate's `1.5` taken as `1.0` it reports an extent error of `0.59`, and with the
return spin `-3.0` rad/s a spin error of `0.017`. The constants (`30.0`, `1.5`,
`4.0`, `2.0`) were read out of memory at `0x08ab0840`, `0x08ab0a78`,
`0x08ab0a7c` and `0x08ab0a80`; the literals are in the instructions.

**The tone is its own voice.** `Hud_Update` calls `HudSight_UpdateTone(hud, 0)` and
`(hud, 1)`: slot `0` reads `0x08ab0a54` (the Missile's state) and slot `1`
`0x08ab0a58`, which `HudSight_UpdateLeachBeam` writes `1` for a held target and `2`
for a lock. So a held LeachBeam plays its own `~ROCKLOCK` voice and **reaches the
locked tone about `0.34` s after a first sighting, not at the Missile's `0.8`**;
`oag_game`'s one tone follows `Race::sight_state`, which now reports exactly that.

**Not obtained: a picture.** The pinned-target capture runs the emulator one
frame per breakpoint stop, and its window showed no reticle for the LeachBeam or
for a Missile held the same way (the pinned craft sat behind the pillar the player
was parked facing), so no frame pair of the arrowheads exists from this pass, and
their art and size are unverified here beyond what the previous page said. Ours
was not seen in a played race either: an autopilot race with `--give LeachBeam`
never put a craft squarely in the window. What was checked is that the draw's
`rotation` and the corners' turn are the same clockwise matrix (`ui.wgsl`'s
`(cos, sin), (-sin, cos)` on a y-down screen), so the figure turns rigidly and
the arrowheads keep pointing inward at every spin. The
numbers above do not depend on it.

**Not a claim about Wipeout HD.** HD's own LeachBeam reticle is
`Hud_UpdateLeachBeamSight` and keeps the Missile's law in `oag_race::sight`
(`Sight::set_leach_law` is left off for the concentric dialect).

**Implemented** in `oag_race::sight::leach` and switched on by
`oag_game::race::Race::set_sight_leach_law` for a title whose brackets carry a
LeachBeam set (`Sights::Brackets { leach: Some(..) }`, which is Pulse). The
tests pin the closing, the lock on arrival, the three spin rates, the opening
steps (`7.25`, `8.5`, `9.75`, `11.5` at 60 Hz), the fade and tint, and that a
shot takes the reticle down while the beam is live
(`crates/game/src/race/tests/leach_beam.rs`).

**HD authors the block too, and that is measured**, contrary to what this page
implied until 2026-09-07: its weapon table carries a `<Weapon type="LeachBeam">`
with a lock window, so a held LeachBeam is reachable on HD as well as on Pulse.
Its own four sprites are `LeachBeamSightBG`, `LeachBeamSightOuter`,
`LeachBeamSightMiddle` and `LeachBeamSightInner` - the same four-part naming
`MissileSight*` uses - off `HUD_Components_01.gtf`. They are **not** drawn:
`Sights::Concentric` carries one set of names and nothing has read which of the
four is up when, so `oag_game::hud::sight_draw` draws nothing for a LeachBeam on
that dialect rather than lending it `MissileSight*`. Both halves are pinned by
`crates/game/tests/lock_sight_ground_truth.rs`.

**Pure authors neither.** Its `Data\XML\weaponstats.xml` carries no
`<Weapon type="LeachBeam">` and its `Data\XML\Arcade_HUD.xml` no
`leachbeam_sight_*` - measured on `pure-psp-usa.chd`, not inferred from the
Missile - so `oag_pure::hud::ART` names `leach: None` and a LeachBeam there
would draw nothing rather than borrow the Missile's brackets.

## `HudSight_Update`, the whole law

`HudSight_Update` (`0x0881dbcc`) runs once a frame with `dt`. Confidence **88**
on the arithmetic; the gate at the top is the one part that is not read (see
[The gate](#the-gate-read-it-is-the-held-weapon)).

```c
player->lock_flags &= ~1;               // entity + 0x860, cleared every frame
if (hud->sight[0] == 0) return 0;       // no widgets bound, nothing to do

step   = dt * 30.0;
x      = 240.0;  y = 136.0;             // screen centre, 480x272
visible = false;
```

### Projecting the target

```c
target = hud->view->target;                        // hud + 0x3c, then + 0xe4
if (target && gate && !(target->flags & 0x1000) && (node = target->node)) {
    world = node->transform.position;
    eye   = view_matrix * world;                   // vtfm4_q
    dist  = length(eye.xyz);
    clip  = projection_matrix * eye;
    if (clip.w > 0.0 && dist < 250.0) {
        sx = 240.0 + 240.0 * clip.x / clip.w;
        sy = 136.0 + 136.0 * clip.y / clip.w;
        px = 2.0 * sx - prev_x;                    // one-frame extrapolation
        py = 2.0 * sy - prev_y;
        if (0 <= px && px < 480 && 0 <= py && py < 272) {
            visible = true;  x = px;  y = py;
        }
    }
}
```

Three things worth naming:

- **`clip.w > 0.0` is the behind-camera guard, and it is recovered rather than
  added.** A craft behind the camera projects to a mirrored on-screen point that
  looks plausible in a still and is wrong in play. The `0.9` lock cone makes it
  rare, not impossible.
- **`dist < 250.0`** is a hard range on the *sight*, not on the lock, and it is
  much shorter than the Missile's authored `lock_max_dist`.
- **The `2*new - old` extrapolation is tested for being on screen, not drawn.**
  What gets drawn is the smoothed centre below. A target that is about to leave
  the screen stops being `visible` a frame early.

### The hold timer, and where it lives

```c
hud->hold = visible ? hud->hold + dt : 0.0;        // hud + 0xe8
```

That is the slot the bind skips. It is what makes the lock take time.

### Chasing the target

The reticle centre (`hud+0xc0`, `hud+0xc4`) is moved toward the projected point
rather than snapped to it, at a rate that depends on how far it has to go:

```c
zoom = clamp(30.0 / dist, 1.0, 5.0);
dx   = x - cx;
dy   = (y - cy) * 0.7;                             // y weighted by 0.7
d    = sqrt(dx*dx + dy*dy);

rate = 1.0;
if (visible) {
    rate = (d * 0.12 + 1.5) * 1.1;
    if (hud->hold > 0.8 && was_locked) rate = 15.0;
    rate *= zoom;
}

locked = false;
if (d < step * rate) {                             // it arrived this frame
    cx = x;  cy = y;
    if (visible && hud->hold > 0.8) locked = true;
} else {
    t   = step * rate / d;
    cx += dx * t;
    cy += dy * t / 0.7;                            // the 0.7 divided back out
}
```

**`0.7` weights the vertical**, so the reticle is judged to have arrived sooner
in `y` than in `x`. The weight is applied to the distance and divided back out of
the step, which is the original's own arrangement rather than a simplification -
the two are not the same thing once the step is clamped.

**A lock is taken only on the frame the reticle arrives**, and only after `0.8`
seconds of continuous visibility. That is the whole acquisition rule, and it is
what `entity+0x860 & 1` carries.

### The box, and where the five widgets go

The half-extent (`hud+0x280`) is eased toward one of three values:

```c
extent_target = visible ? (locked ? 6.0 : 9.6) : 30.0;
extent_target *= zoom;
// eased at dt * 50.0 per second, 1.4x faster while the current extent is above
// the target *before* the zoom - so the threshold is 6.0 once locked and 9.6
// before it, not a fixed 9.6
```

so the brackets sit wide open at `30`, close to `9.6` once they have something,
and to `6.0` on the lock. Then:

```c
sight[0]->pos = (cx - e, cy - e);  sight[0]->rotation = 1.5707964;   // pi/2
sight[1]->pos = (cx + e, cy - e);  sight[1]->rotation = 3.1415927;   // pi
sight[2]->pos = (cx - e, cy + e);  sight[2]->rotation = 0.0;
sight[3]->pos = (cx + e, cy + e);  sight[3]->rotation = 4.712389;    // 3pi/2
```

**Rotations, not mirrors.** The original writes an angle to `widget+0xac` for
each of the four, which is the only way one corner bracket makes four corners
without four models. Written as the bit patterns `0x3fc90fdb`, `0x40490fdb`, `0`
and `0x4096cbe4`.

The inner is pulled off the smoothed centre toward the *true* projected point,
clamped to `0.4` of the extent - so it lags visibly while the brackets chase:

```c
d2 = (x - cx, y - cy);
if (|d2| > 0.4 * e) { d2 *= 0.4 * e / |d2|; }
inner->pos = (cx + d2.x, cy + d2.y);
```

### Colour and blink

```c
alpha_target = visible ? (near ? 255.0 : 96.0) : 0.0;
// eased at dt * 1000.0 per second
// a 10 Hz square toggle runs the whole time (0.1s period)
// while NOT locked, the blink alternates the colour between two tints
```

`near` is `depth < _DAT_002aca4c + 500.0`, where the depth comes out of the same
projection - a second, softer range test on top of the `250.0` one. **This
description is superseded** - `_DAT_002aca4c` is a base-0 address from before
the 2026-09-07 relocation fix (see `docs/ghidra/workflow.md`), and what it
resolves to is not a tuning constant at all. See
[The far-target alpha step is a live depth-buffer sample](#the-far-target-alpha-step-is-a-live-depth-buffer-sample-not-a-tuning-constant).

All five widgets are shown when `visible` or while the extent is still easing,
and hidden otherwise, so the brackets are watched closing even after the target
is gone.

**The blink is a tint and never a hide**, which is the half of this that is easy
to get backwards: both arms above write a colour and neither drops the draw. A
port that gates the draw on the blink strobes the reticle off five times a
second. **Which colour channel each arm lights, and the far-target alpha's
"near" test, are both resolved below** - see
[Colour and blink, resolved](#colour-and-blink-resolved-the-byte-order-and-the-two-tints)
and
[The far-target alpha step is a live depth-buffer sample](#the-far-target-alpha-step-is-a-live-depth-buffer-sample-not-a-tuning-constant).

## The tone: `~ROCKLOCK`, one voice with two states

`HudSight_Update` publishes a small state to a global,
`DAT_002aca54[player]`:

| Value | When |
| --- | --- |
| `0` | no target (written at the top of every frame) |
| `1` | `visible` |
| `2` | `locked` |

`HudSight_UpdateTone` (`0x0881b34c`) turns that into sound:

```c
if (state == 0 && previous != 0) { stop(voice); voice = 0; }
else {
    if (state != 0 && voice == 0) voice = play("~ROCKLOCK", 0x400);
    if (state == 1) set_param(voice, 0, 0);
    else if (state == 2) set_param(voice, 0, 1);
    previous = state;
}
```

**One cue, started once and parameterised**, rather than two cues. `~ROCKLOCK`
is a real cue in the disc's own banks - `oag-wad sounds` puts it in `hud.bnk` as
cue 6 at 2 waveforms, 0.11 s, neither looping - and the parameter is what picks
which of the two plays. The `~` prefix marks it the way `~MISSILETVL` and
`~LEACHATTACH` are marked.

**What the parameter selects is read, 2026-09-30.** The cue's list is
`[0x15, guard(param 0 == 0), key-on (delay 30), guard(param 0 == 1), key-on
(delay 15), 0x16]`, so the parameter picks a **tempo** rather than one of two
waveforms: a beep every 30 master ticks (116 ms) while seeking, every 15 (58 ms)
once locked, both keying the same 0.052 s waveform. The earlier "two waveforms,
which is which, inference at 55" reading is retired; see
[`sound.md`](sound.md#cue-parameters-the-guard-operand-and-the-loop-back-flag-2026-09-30)
for the guard's operand, the parameter store (`0x0898daf0`) and the loop-back.

**This retires the "the lock tone is unidentified, `~ROCKLOCK` at 40" note.** It
is identified: `_DAT_00275ba4` is the pointer to that string and
`HudSight_UpdateTone` is the only thing that loads it.

## The gate, read: it is the held weapon

**Resolved 2026-10-01** (it was "the one part not read", confidence 50 on what it
meant). The projection block is guarded by

```c
gate = (hud->view->held_plus_one == 2);           // hud + 0x3c, then + 0x48
if (!gate) {
    for (m in missile_pool) {
        if (m->owner_field == player->0x360 && m->target != 0
            && dot(normalize(m->velocity), normalize(m->target->pos - m->pos)) > 0.7) {
            gate = true;  break;
        }
    }
}
```

and `view+0x48` is **the held weapon id plus one**: the player's status record
`hud+0x3c` reads `0` with nothing held, `2` with a Missile (id `1`) and `11` with
a LeachBeam (id `10`), live on PPSSPP. So the Missile's sight is drawn while a
Missile is held **or** while one of the player's missiles is in the air and
pointing at what it chases - which is the behaviour the previous reading found
"does not match how the weapon plays" and does; the `mode == 2` it could not place
was the held weapon. The LeachBeam's own function gates on `11` the same way
([above](#the-leachbeams-reticle-is-its-own-function)). `oag_game` gates the sight
on "the held weapon locks and something is lockable", which is the same
condition with the lock window folded in.

## The other two titles

Measured 2026-08-26 and pinned by `crates/game/tests/lock_sight_ground_truth.rs`.
**All three lock, and two dialects draw it.** The law on this page is engine code
with no title in it; what differs is only what each disc authors.

| | Weapon table | Sight widgets | Reticle |
| --- | --- | --- | --- |
| Pulse | two, per mode | 9 `<Mode3D>` models | four brackets + inner |
| Pure | **one**, `weaponstats.xml` | 5 `<Mode3D>` models, the Missile's | the same |
| HD/Fury | two, per mode | 6 `<Image>` sprites, **0** models | concentric rings |

**Pure is the PSP dialect with two spellings of its own.** Its `Arcade_HUD.xml`
carries `missile_sight_inner` and `missile_sight_1` … `_4` off the same two
models at the same placeholder position - and *not* the LeachBeam's four, the
disc agreeing that is a Pulse weapon. What kept it dark was upstream: Pure names
one lower-cased `Data\XML\weaponstats.xml` where Pulse names two, and authors a
single `speed` per weapon where Pulse authors four with a `launchSpeed`. Both are
now axes - see [weapon-stats.md](../../../formats/weapon-stats.md).

**HD draws the same law with different art.** Its arcade HUD composes to zero
`<Mode3D>` models; the reticle is concentric `<Image>` sprites off
`Data\HUD\Textures\missile_reticule.gtf`:

| Widget | Size | Colour |
| --- | --- | --- |
| `MissileSightBG` | 128 | white |
| `MissileSightOuter` | 128 | red |
| `MissileSightInner` | 108 | green |
| `MissileSightMiddle` | 80 | white |
| `MissileSightLockedOnLines` | 128 | white |
| `MissileSightLockedOnMiddle` | 64 | red |

Nothing rotates and nothing is offset - the sizes are authored and only the
centre is the reticle's. Which four are the seeking set and which two the locked
one is read **off the names**, at confidence 70: they are unambiguous about what
each widget is and silent about whether the seeking set stays up underneath.
`oag_game` draws them additively, which shows every authored widget rather than
hiding some on a guess. HD also authors four `LeachBeamSight*` off
`HUD_Components_01.gtf` - `BG`, `Outer`, `Middle`, `Inner` - and its weapon
table authors the `<Weapon type="LeachBeam">` block to go with them, measured
2026-09-07. They are still unwired, but no longer for Pulse's old reason: what
is missing is a second widget set on `Sights::Concentric` and a reading of which
of the four is up when. Until then a held LeachBeam draws nothing here. See
[The LeachBeam's reticle is its own function](#the-leachbeams-reticle-is-its-own-function).

**The placeholder idiom is what says the reading is right.** Every sight widget
on every title is authored centred on `(-width/2, +height/2)` of that title's own
screen - `(-240, 136)` on the PSP's 480x272, `(-960, 540)` on HD's 1920x1080. Same
arithmetic, different number, and all of them overwritten every frame.

**Which screen also has to be per title**, and that is the one thing that reads
as a stray widget rather than a missing one when it is wrong: a reticle
projecting into 480x272 while the layout draws in 1920x1080 lands in the top-left
ninth of an HD frame - inside its lap counter - and never leaves.

## What this settles elsewhere

- **`entity+0x860 & 1` has a writer.** It is here, and it means "the reticle has
  settled on a target it has held for 0.8 s". `Ship_FireHeldWeapon` reads it to
  decide whether to pass the target through - see [missile.md](missile.md).
- **The `<Mode3D>` layer's first real consumer is 2D.** Every one of these nine
  widgets is a screen-space quad with a rotation, drawn in an orthographic block.
  Nothing here needs a 3D pass.

## Colour and blink, resolved: the byte order and the two tints

2026-09-16. Reads a fresh decompile of `HudSight_Update` off the now-relocated
`psp-pulse-usa` database (the 2026-09-07 relocation fix - see
`docs/ghidra/workflow.md` - postdates this page's original pass, which is why
its addresses in this section were still on the base-0 numbering: what this
page called `_DAT_002aca4c`/`DAT_002aca54` resolve to `g_hud_sight_depth_sample`
(`0x08ab0a4c`) and `0x08ab0a54` in the live database, not new addresses).

The widget's colour word is written by `Image_SetVertexColours`
(`0x089122b4`, confidence 78 - it resolves a widget's mesh instances the way
`HudSight_Bind`'s own lookup helper does and stamps the given `u32` into each
at `+0x6c`, called with `iVar20`/`iVar14`/`iVar15`/`iVar21`/`iVar9` - the five
sight widget pointers - immediately after the arithmetic below). The literal
construction, straight off the decompile:

```c
uVar16 = uVar18 | 0xff000000;          // uVar18 = the eased alpha, 0..255
if (!locked) {
    uVar12 = uVar18 * 0xc0 >> 8;       // 0.75x - the recovered BLINK_TINT
    if (!blink) {
        uVar16 = uVar16 | uVar18 << 8;
    } else {
        uVar16 = uVar12<<8 | uVar12<<0x10 | uVar12 | 0xff000000;
    }
}
Image_SetVertexColours(widget, uVar16);
```

**The byte order is this project's own established one, not a fresh
assumption.** `Loading_DrawWave`'s vertex-colour ramp from `0xff000000` to
`0xff808080` (`crates/render/src/loading.rs`, `docs/formats/psp-texture.md`)
already fixed byte0 = R, byte1 = G, byte2 = B, byte3 = A for this engine's own
32-bit packed colour word, and `HudSight_Update`'s `0xff000000`/`0xc0` literals
are the identical idiom. Reading `uVar16`'s construction in that order:

| State | R | G | B | A |
| --- | --- | --- | --- | --- |
| Locked | alpha byte | `0` | `0` | `0xff` |
| Seeking, blink off | alpha byte | alpha byte | `0` | `0xff` |
| Seeking, blink on | alpha byte `* 0.75` | same | same | `0xff` |

So: **locked is red**, brightness driven by the eased alpha value; **seeking
alternates yellow at full brightness with white at [`BLINK_TINT`] the
brightness** - never a hue this page's earlier "not recovered" note
anticipated, but not a coincidence either: yellow-to-red on lock is the same
warm-to-hot shift a targeting reticle in most other games of this era uses.
Confidence **88**, matching the surrounding arithmetic this page already
carries at that score - the construction is read directly, not inferred.

**The colour word's own alpha byte is always `0xff`.** The eased fade this
project already carries as `Sight::alpha()` scales the RGB channels instead of
a blend alpha - consistent with the sight models' `Additive` blend class
(`crates/game/src/hud/sight_draw.rs`): under `dst + src.rgb * src.a`, fixing
`a = 1` and scaling `rgb` is the same final colour as fixing `rgb = white` and
scaling `a`, so this is not a second, uncounted fade layered on top of the one
already ported.

**Ported** to `oag_race::sight::Sight::tint` (returns `[r, g, b]` now, not a
scalar) and `crates/game/src/hud/sight_draw.rs::bracket_draws`. Wipeout HD's
concentric dialect is deliberately **not** given this hue - HD authors each
ring's own colour (a red outer, a green inner) and nothing has read whether
HD's own sight code spends a blink the same way, so `Sight::brightness()`
keeps the pre-existing scalar-only behaviour for that dialect. See
[the other two titles](#the-other-two-titles) below for what nothing here
answers about HD's own colour writes.

## The far-target alpha step is a live depth-buffer sample, not a tuning constant

2026-09-16. `HudSight_Update`'s `near` test -
```c
bVar1 = g_hud_sight_depth_sample < local_60 + 500.0;
```
where `local_60` is the target's own projected depth, reshaped into roughly a
16-bit Z-buffer's units (`((clip.z/clip.w)*0.5+0.5) * -63945.0 + 64946.0`) -
reads a global this page's earlier pass could not resolve
(`_DAT_002aca4c`, a base-0 address from before the relocation fix; see
above). In the live, relocated database it is `g_hud_sight_depth_sample`
(`0x08ab0a4c`), and it is not a fixed tuning constant.

**It is written once a frame, from a live read of the rendered frame's own
Z-buffer, at the reticle's own previous on-screen position.**
`Hud_SampleSightDepth` (`0x08819b24`) is its only writer:

```c
void Hud_SampleSightDepth(int zbuffer)
{
    g_hud_sight_depth_sample =
        (float)*(ushort *)(zbuffer + ((0x110 - DAT_08ab0a48) * 0x200 + DAT_08ab0a44) * 2);
}
```

`DAT_08ab0a48`/`DAT_08ab0a44` are `HudSight_Update`'s own smoothed reticle
centre (`cy`/`cx`), cast to `int` and stored at the very end of that function -
so this reads back a 16-bit Z-buffer texel (`0x200` = 512, the buffer's row
stride) at wherever the reticle was drawn *last* frame, one frame lagged.

**`Hud_SampleSightDepth`'s only caller does double duty**, which is worth
recording even though the second half is out of this page's scope:
`FUN_0890906c` (`0x0890906c`, left unnamed - confidence on its own full
purpose is below 50) gates on `param_1 == 0x200` and a non-null Z-buffer
pointer, calls `Hud_SampleSightDepth` unconditionally, and then separately
counts nearby texels below a threshold (`0x3b3`) into
`*(float*)(DAT_08b62d34+0x74)` - a small neighbourhood readback that looks
like a depth-of-field or fog focus metric for an entirely different
subsystem, sharing the pass rather than being sight-specific. The sight's own
read is the unconditional call at the top; the rest is not read further here.

**So "near" is really "is the target's own depth closer than whatever
geometry the reticle was sitting over a frame ago"** - a one-frame-lagged,
screen-space occlusion test, not a softer range band layered on
[`DRAW_RANGE`] the way this page's own earlier "near/far" framing suggested.
Confidence **85** on this reading: the writer, its caller, and the field
layout (`0x200`-wide row stride, `0x110` = 272 = screen height, the flip
matching `HudSight_Update`'s own `y`-down convention) are all read directly;
what is not read is `FUN_0890906c`'s own broader purpose or exactly when in
the frame it runs relative to `HudSight_Update`.

**Not ported.** Reproducing this needs the rendered frame's own depth buffer,
which `oag_race` structurally cannot reach - `CLAUDE.md`'s dependency rule 1
forbids a gameplay crate depending on `oag-render`. `oag_race::sight::Sight`
keeps treating every drawn target as near, same as before this pass - now
because the alternative needs a renderer in a crate that must not have one,
rather than because the constant was unsampled.

## Does Pulse have anything like HD's staged reveal? No - checked

2026-09-16. HD's `Hud_UpdateLeachBeamSight` reveals one of three rings at a
time as the hold progresses (see `docs/ghidra/functions/ps3-hdfury-eu/hud-sight.md`).
Rereading `HudSight_Update`'s own five-widget placement above with that
question: **the Missile's own "reveal" is continuous, not staged.** The
extent (`hud+0x280`) eases from `30.0` open through `9.6` seeking to `6.0`
locked at one rate the whole time - a single easing curve, never a discrete
widget swap - so what reads as "the brackets closing in" is one number moving,
not a sequence of different widgets being shown and hidden the way HD's rings
are. No widget is ever hidden or added at a hold-time threshold anywhere in
this function.

**The LeachBeam on Pulse has a reveal law, and it is not staged either**
(2026-10-01): `HudSight_UpdateLeachBeam`, above, closes the four arrowheads
continuously from `30.0` to `6.0` and takes the lock on arrival - no widget is
hidden or added at a threshold there either. It is the closing the earlier text
here left to the Missile's law.
