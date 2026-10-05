# Both lockable weapons draw their reticle now, and the black background behind it is gone

2026-08-26, [missile.md](../../docs/ghidra/functions/psp-pulse-usa/missile.md) - the
page to read, not this row. **A press with no lock used to do nothing and keep the
pickup, and that was ours rather than the original's.**
`Ship_FireHeldWeapon` (`0x08844ae8`) branches on the lock and calls
`Weapon_RequestFire` on **both** arms - the target's address on one, a null and an
index of `-1` on the other - and `Weapon_FireMissile` (`0x088685cc`) tests the
target not at all, clearing `craft+0x1bc` *before* it checks whether the pool has
room. So the pickup is spent even on a full pool. `Missile_Update` already skipped
its whole guidance block on `self+0xe0 == 0`, which
[missile.md](../../docs/ghidra/functions/psp-pulse-usa/missile.md#the-target-is-fixed-at-launch)
recorded months ago and nothing acted on: an unlocked missile rides the floor,
glances off walls up to five times, and flies ballistically.

**What ends it was the missing half, and it is recovered.** `MissilePool_Update`
(`0x08869588` - renamed from `Projectiles_Update_q`, and re-scored 75 -> 88, since
three things say it is the *Missile's* pool and not projectiles in general) runs a
second pass over every live slot and takes the destroy branch on `3.0 < age`. The
age is `missile+0x50`, **the same field `Missile_SpeedNow` ramps on**, which is
what puts the constant at 90 rather than at "a float compared against 3.0". Bit
`4` is shared with the wall and craft paths, so the expiry is a **detonation**
rather than a reap. Ported as `missile::SELF_DETONATE_SECONDS`.

**And the detonation spends no blast, which is measured rather than chosen.** The
damage (`FUN_08869054`) and the blast force (`Missile_ApplyBlastForce`,
`0x08868ea4`) have exactly two callers each - the per-tick craft-hit test and the
network handler - by exhaustive operand search over all 524,719 instructions. The
pool's teardown reaches **neither**; it reaches `Missile_SpawnExplosion`
(`0x08868d50`, fourcc `MIEX`) alone. So a missile that hits nothing looks like it
went off and hurts nobody. Carried as `Impact::blast`, a flag rather than a second
array so the visual side still plays the explosion.

**Two live claims fell over on the way and are corrected in place but not
fixed.** (1) `oag_weapons::projectile`'s `Impact` says a flat blast is ours
because "nothing has been read that says" the original falls off. Something has:
`Missile_ApplyBlastForce` adds
`normalize(d) * (1 - |d|/blastradius) * blastforce` and **excludes the craft
struck**, so the force falls off linearly and the *damage* does not go through
that function at all. (2) `MAX_FLIGHT_SECONDS` calls itself "ours, a safety net";
the Rocket's own pool (`RocketPool_Update`, `0x0886de60`) caps at `5.0 < rocket+0x48`
and reaps without an explosion - so the cap is recovered and only the number is
ours. Neither is changed here: both move how every weapon lands, and this change is
about the Missile.

**The lock-on reticle and its tone are in too**, recovered whole on their own page:
[lock-sight.md](../../docs/ghidra/functions/psp-pulse-usa/lock-sight.md).
`HudSight_Update` (`0x0881dbcc`) turned out to answer a question `missile.md` had
open for months - **it is what writes `entity+0x860 & 1`**, the flag
`Ship_FireHeldWeapon` gates the lock on, and it only writes it after **0.8
seconds** of holding a target on screen *and* the brackets catching up. So the
lock is not a property of geometry: it takes time.

The rest of that function is the reticle. Four corner brackets at `±extent` with
the four quarter-turn rotations the original writes to `widget+0xac`, an inner box
that leads them by at most `0.4` of the extent, an extent easing between `30`
(idle), `9.6` (seeking) and `6.0` (locked) times `clamp(30/dist, 1, 5)`, a chase
whose vertical counts `0.7`, a `w > 0` behind-camera guard and a `250`-unit draw
range. All ported into `oag_race::sight`, all asserted.

**The art is nine widgets over three models**, and the distinction is the finding:
`missile_sight_1` … `_4` all instance `missile_sight_outer.vex` - a single corner
bracket - `missile_sight_inner` has its own closed box, and the LeachBeam's four
share a hollow arrowhead. Each model is one 8-unit textured quad, so the
`<Mode3D>` block needs no 3D pass at all; the quads go through the existing 2D UI
pipeline with a rotation added to it. `docs/ui/hud.md` and
`oag_game::hud::widget::Model` had the three *model* names right and said nothing
about the four-to-one instancing; both corrected.

**The tone is `~ROCKLOCK`**, `hud.bnk` cue 6, two waveforms, 0.11 s, neither
looping - which is exactly what `HudSight_UpdateTone`'s seeking/locked parameter
selects between. Fired here as two edges rather than one parameterised voice,
because this mixer has no cue parameters; at 0.11 s the audible result is the same
pair of blips.

**A dead end worth not repeating**: `entity+0x85c`, the lock target, has exactly
three consumers in the executable - `Craft_Construct_q` sets it to `-1`,
`Ship_AcquireLock` writes it, `Ship_FireHeldWeapon` reads it. Nothing draws off
it. The sight reads its target through `Hud_ResolveLockTarget` (`0x0883b358`) off
the mirror at `weapon_record+0x1b4` instead.

**Verified**: `just` green at **2,490**, and eight disc-backed tests. Five in
`crates/game/tests/missile_ground_truth.rs` - the four that were there plus
`a_missile_with_no_lock_still_flies_a_real_circuit_and_ends_itself`, which fires
from every slot on the shipped starting grid, proves at least one shot went up
unguided, and proves none of them outlives three seconds on real geometry. Three
new ones in `crates/game/tests/lock_sight_ground_truth.rs`: the layout authors
nine sight widgets over three models, the art decodes out of those models into
the HUD sheet, and a Missile held on a real circuit locks a real craft after the
recovered hold. **And it was looked at**: a `--race --opponents --give Missile`
capture shows the reticle over a craft ahead. The world hash did not move - no
`Projectile` field was added, the age is still derived from `lifetime`, and the
reticle is render-only state on `Race`.

## Open

- **Whether an opponent's craft has its own `entity+0x860` at all is
  unrecovered, and it changes what "guided AI missile" means.** `HudSight_Update`
  is the sole writer this thread found, and nothing read says it runs for
  anything but the craft the HUD is drawn for. If it never runs for an AI craft,
  that craft's copy of the bit - if the field is even per-craft rather than a
  single global - is never set, and `Ship_FireHeldWeapon` takes the no-lock arm
  for every missile an opponent fires in the original, regardless of what
  `Ship_AcquireLock` found. This engine's opponents already run the geometric
  lock unconditionally (a deliberate, already-documented deviation - see
  `Race::fire_missile`'s doc comment), and the hold gate just landed leaves that
  choice untouched: only `slot == 0`'s candidate is gated on `sight::Sight`,
  because that is the only craft this project has found a reticle for. If the
  hypothesis above is right, this engine's opponents are strictly better shots
  with the Missile than the original's are. Not chased here - it needs
  `Ship_FireHeldWeapon`'s and `HudSight_Update`'s callers resolved, and both
  currently return no xrefs from Ghidra's static analysis (probably reached
  through a function-pointer table).
- ~~**HD authors a LeachBeam and cannot draw its reticle.**~~ **Wired
  2026-09-15**: `oag_title::hud::Sights::Concentric` gained a `leach` field
  (the same `Option`-of-four shape `Brackets::leach` already used) and
  `hud::sight_draw` now draws HD's and 2048's own
  `LeachBeamSightBG`/`Outer`/`Middle`/`Inner` for a held LeachBeam rather than
  nothing. Which of the four shows when is **chosen** (all four together) and
  not measured - HD's own `Hud_UpdateLeachBeamSight`
  (`docs/ghidra/functions/ps3-hdfury-eu/hud-sight.md`) turns out to reveal
  them one at a time as the lock progresses rather than all-or-nothing, so
  the chosen behaviour is known to be an approximation, not just unverified.
  That page also found HD's hold time is `0.5` s where the PSP's is `0.8`,
  unadopted pending a decision on a per-title constant.
- **Where the LeachBeam's four widgets go is inferred, not read.**
  `HudSight_Update` is read end to end and writes **five** widgets - the
  Missile's four plus its inner. Nothing yet read writes the bind's
  `+0x108` … `+0x114`. `oag_race::sight` gives the LeachBeam's four the
  Missile's corners and rotations, which is **chosen, not measured** and carries
  no confidence score: the grounds are that the two sets are the same shape and
  that one arrowhead makes four corners no other way. A capture checks the
  result, not the rule. Finding the writer of `+0x108` settles it.
- ~~**The sight quads draw on an opaque black square, and always have.**~~
  **Fixed 2026-09-07, and the hypothesis was right.** The three models' own
  `pass_mask` was read and is `0x120e` on all three - the `0x200` bit, so
  `BlendClass::Additive` - and their textures are named `gunsight_ADD.tga`,
  `gunsightdot_ADD.tga` and `LeachBeamSight_ADD_nomip.tga` with alpha at
  250/255 across every texel, so the shape is in the colour channels over a
  black field and an alpha blend could only ever draw a black tile. The class
  is now read off each model's batch at load and carried to the draw; both
  weapons' reticles sit on the track on Pulse **and on Pure**, whose sight
  models declare the same class. The whole reading and both titles' numbers
  are in [hud.md](../../docs/ui/hud.md), "A `<Mode3D><Model>` quad carries its
  own blend". Nothing was chosen: `oag_game::race::hud::quad_blend` tabulates
  no model name.
- **`HudSight_Update`'s gate is read and not understood** - `hud->view->0x48 == 2`
  or "one of my missiles is homing". Taken literally the reticle would never
  appear while merely holding a Missile. `oag-game` gates on "the held weapon
  locks and something is lockable" instead and says so.
- **The far-target alpha is not reproduced.** The original dims a distant
  reticle to 96/255 against a global this engine has no equivalent of; every
  drawn target is treated as near.
- **The 480x272 projection aspect is ours.** The reticle projects at the virtual
  screen's shape rather than the window's, so it is exact at the original's own
  aspect and drifts slightly as the display aspect is taken away from it.
- **The blast is flat and includes the firer**, and both are now known to
  disagree with the original: linear falloff on the force, the struck craft
  excluded from it, and no damage through it at all.
- **The Rocket's recovered `5.0` cap** is not adopted; it keeps this engine's
  `MAX_FLIGHT_SECONDS = 10.0`.
- **The network path disagrees with the local one about the expiry blast**:
  `MissilePool_DestroyRemote` (`0x08868a10`) calls the blast force on its
  "died on nothing" branch where `MissilePool_Update` does not. Unresolved.
- **A cue that reads wrong**: the expiry branch plays `_DAT_00278950`, which
  resolves to `"SHURIKENEXPL"` where its neighbours resolve to
  `"MISSILEEXPWALL"` and `"MISSILEEXPSHIP"`. Checked twice. Confidence 60 that it
  is a copy-paste in the original; not load-bearing here.

**The gate landed.** `Race::fire_missile` now passes the geometric candidate
through only when the *firing craft's own* hold has caught up -
`self.sight.locked()` for `slot == 0`, since `sight::Sight` is the reticle and it
is drawn for one craft. A press before the brackets close still fires - the
pickup is still spent and the missile still flies - it just carries no
candidate, same as this thread's first half already made an unlocked press do.
Every other slot's candidate is used exactly as before this gate: unchanged,
because nothing here says an opponent's craft has a hold to check at all - see
the new `## Open` line below. Of the file's five tests, only
`a_missile_turns_toward_the_craft_it_locked` needed a real hold before firing to
keep asserting on a genuine lock; `a_missile_never_locks_its_own_firer` was
primed the same way so its self-lock invariant still gets exercised against
slot 0 and not only against the opponents. The other three needed no change -
neither fires expecting a *specific* lock outcome from slot 0's shot.

**The LeachBeam's reticle landed on 2026-09-07** and its four widgets are
driven off its own authored window. `oag_tables::weapons::LeachBeamStats`
decodes `absorb` and the lock pair at `stats+0x114`/`+0x118` and nothing else -
the beam's other six attributes have no consumer, and `range` is deliberately
left alone because it is authored at the same figure as `HudSight_Update`'s own
code literal and conflating the two would manufacture a finding.
`missile::lock_window` takes the window directly, `lock` is a one-line wrapper
over it, and no call site moved: the two weapons differ by **two numbers**, the
`0.9` cone and the 1.4 screen being shared. `sight::Held` carries which weapon
the reticle wears so the draw path picks art off the *held weapon* and never off
its target; Pulse's `Sights::Brackets` gained a `leach` set and Pure's is `None`,
measured against its own layout rather than inferred. **Two findings**: the
LeachBeam's far bound is shorter than the Missile's on all four shipped tables
(near bounds agree), and it is the only decoded block authoring no
`slowdown_time` - a claim `RocketStats::slowdown_time` had made universally and
now qualifies. The world hash did not move: no `World` field, no `Projectile`
change, the reticle is still render-only state on `Race`.

## Next Steps

- **Find the writer of the LeachBeam bind's `+0x108` … `+0x114`.** With the
  blend settled, this is the reticle's last *chosen, not measured* thing: the
  LeachBeam's four arrowheads are given the Missile's corners and rotations
  because the two sets are the same shape, and `HudSight_Update` writes only
  the Missile's five. A capture checks the result and not the rule, so the
  arrowheads could be at the right places for the wrong reason - and pointing
  outward where the original points them inward would look wrong to a player
  and pass every test in the tree. This replaces the `pass_mask` step, which
  landed on 2026-09-07.
- **Pure's sight quad is 12 units where Pulse's is 8**, and
  `hud::sight_draw::SIGHT_SIZE` is a hardcoded `8.0` for both. Read off the
  loader report on `pure-psp-usa.chd`: `missile_sight_inner.vex` measures
  `quad Some([11.999471, 11.999471])` there against `[7.9978027, 7.9978027]`
  on Pulse, and `crate::sprite::Placed::quad_extent` already carries the real
  number that `model_draw` uses and `bracket_draws` does not. Small, and it is
  a title difference being flattened rather than a missing reading.

**Pure and HD both lock and draw now** - see
[pure-and-hd-lock-on-too-and-two-axes.md](pure-and-hd-lock-on-too-and-two-axes.md),
which is where the two rows that used to sit here went.
- Decide the blast in one place: linear falloff on force, the struck craft
  excluded, damage on the direct hit alone. It moves every weapon, so it wants
  its own change and its own hash move.
