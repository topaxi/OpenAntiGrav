# The Missile fires without a lock, and the lock-on reticle and its tone are in

2026-08-26, [missile.md](../docs/ghidra/functions/psp-pulse-usa/missile.md) - the
page to read, not this row. **A press with no lock used to do nothing and keep the
pickup, and that was ours rather than the original's.**
`Ship_FireHeldWeapon` (`0x08844ae8`) branches on the lock and calls
`Weapon_RequestFire` on **both** arms - the target's address on one, a null and an
index of `-1` on the other - and `Weapon_FireMissile` (`0x088685cc`) tests the
target not at all, clearing `craft+0x1bc` *before* it checks whether the pool has
room. So the pickup is spent even on a full pool. `Missile_Update` already skipped
its whole guidance block on `self+0xe0 == 0`, which
[missile.md](../docs/ghidra/functions/psp-pulse-usa/missile.md#the-target-is-fixed-at-launch)
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
fixed.** (1) `oag_gameplay::projectile`'s `Impact` says a flat blast is ours
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
[lock-sight.md](../docs/ghidra/functions/psp-pulse-usa/lock-sight.md).
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
range. All ported into `oag_game::race::sight`, all asserted.

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

- **The 0.8-second hold does not gate firing yet.** `Ship_FireHeldWeapon` passes
  a target only when `entity+0x860 & 1` is set, so in the original a press before
  the reticle locks fires a **dumb** missile - which is the same behaviour the
  first half of this change built. Wiring the two together closes the loop and
  changes which shots are guided, so it wants its own change: the five disc-backed
  missile tests call `Race::fire_missile` directly after 30 ticks and would all
  start getting unguided missiles.
- **Pure authors the Missile's five sights and has no weapon table to drive
  them.** No archive on the Pure disc holds `Data\XML\WeaponStats_Race.xml`, so
  there are no lock distances and no Missile at all. The widgets and their art
  are already there and already load; finding Pure's weapon data is the whole
  job. `pure_ships_no_weapon_table_so_its_sights_have_nothing_to_drive_them`
  fails the day it is found.
- **HD locks but draws no reticle.** Its Missile parses with a real lock window,
  so the hold and the unguided shot work there already; its sights are `<Image>`
  sprites named `MissileSight*`/`LeachBeamSight*` off `missile_reticule.gtf`
  rather than `<Mode3D>` models, so `sight_draws` finds nothing to place. A
  second naming table and the ordinary sprite path.
- **The LeachBeam's reticle is not driven.** Its four widgets are bound and its
  model is in the sheet; what is missing is upstream - `oag_formats::weapons`
  parses no `<Weapon type="LeachBeam">`, so there are no `lock_min_dist` /
  `lock_max_dist` at `stats+0x114`/`+0x118` to run the window against. Parsing
  that block and adding an arm to `Race::sight_target` is the whole job.
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

## Next Steps

- Gate `Race::fire_missile` on the reticle's lock, so firing early gives the dumb
  missile the original gives. Re-time the five disc-backed missile tests to hold
  past `sight::HOLD_SECONDS` first.
- Parse `<Weapon type="LeachBeam">` and give its four sight widgets the same
  treatment the Missile's five just got.
- Find where Pure keeps its weapon data. Its sights are authored and loading and
  are waiting on nothing else.
- Give HD's `MissileSight*` sprites the placement law; the lock behind them
  already runs.
- Decide the blast in one place: linear falloff on force, the struck craft
  excluded, damage on the direct hit alone. It moves every weapon, so it wants
  its own change and its own hash move.
