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

So the Missile's reticle is four brackets around a box with a closed box at its
centre, and the LeachBeam's is four arrowheads pointing inward. That the same
model appears four times is why the placement below writes a **rotation** per
instance.

## `HudSight_Update`, the whole law

`HudSight_Update` (`0x0881dbcc`) runs once a frame with `dt`. Confidence **88**
on the arithmetic; the gate at the top is the one part that is not read (see
[The gate](#the-gate-is-the-one-part-not-read)).

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
projection - a second, softer range test on top of the `250.0` one.

All five widgets are shown when `visible` or while the extent is still easing,
and hidden otherwise, so the brackets are watched closing even after the target
is gone.

**The blink is a tint and never a hide**, which is the half of this that is easy
to get backwards: both arms above write a colour and neither drops the draw. A
port that gates the draw on the blink strobes the reticle off five times a
second. What is *not* recovered is which colour channel each arm lights, because
that depends on the byte order of the widget's colour word and nothing here has
read it - so `oag_game` carries the `0xc0 >> 8` brightness difference and leaves
the reticle white rather than inventing a hue.

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

**Which waveform each parameter value selects is inference at 55.** That the
parameter is `0` while seeking and `1` once locked is read; that those index the
cue's two waveforms in that order is the obvious reading and is not taken off the
bank's command list, whose selecting opcode is unread - the same gap
`oag_game::audio::sfx` records for every cue with alternates. If it is the other
way round the blip and the chime swap and nothing else does.

**This retires the "the lock tone is unidentified, `~ROCKLOCK` at 40" note.** It
is identified: `_DAT_00275ba4` is the pointer to that string and
`HudSight_UpdateTone` is the only thing that loads it.

## The gate is the one part not read

The projection block is guarded by a flag this page calls `gate`:

```c
gate = (hud->view->mode == 2);                    // hud + 0x3c, then + 0x48
if (!gate) {
    for (m in missile_pool) {
        if (m->owner_field == player->0x360 && m->target != 0
            && dot(normalize(m->velocity), normalize(m->target->pos - m->pos)) > 0.7) {
            gate = true;  break;
        }
    }
}
```

So the sight is drawn when some mode equals `2`, **or** when one of the player's
missiles is in the air and pointing at what it is chasing. Neither `hud->view`
nor `player+0x360` is identified, and the helper that returns the pool is at an
address Ghidra has not made a function of, so this is **read but not understood**
- confidence **50** on what it means, against 88 for everything below it.

Taken literally it would mean the reticle never appears while merely *holding* a
Missile, which does not match how the weapon plays. Either `mode == 2` is
commoner than it looks or one of the two reads is wrong. `oag_game` gates the
sight on "the held weapon locks and something is lockable" instead, and says so.

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
`HUD_Components_01.gtf`, unwired for the same reason Pulse's are.

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
