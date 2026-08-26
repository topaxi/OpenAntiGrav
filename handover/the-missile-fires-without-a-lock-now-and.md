# The Missile fires without a lock now, and the lock-on indicator is nine widgets nobody draws

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

**The lock-on sight and its tone are the user-visible gap this did not close**, and
the by-catch says why. `BOOT.BIN` holds one contiguous run of `<Mode3D>` model
names at `0x08a79cd4`: `missile_sight_1` … `missile_sight_4`,
`missile_sight_inner`, then `leachbeam_sight_1` … `leachbeam_sight_4`. **There is
no `missile_sight_outer` string in the executable**, which `docs/ui/hud.md` and
`oag_game::hud::widget::Model` both claimed; both corrected. Four brackets and a
centre is a lock-on reticle, not an inner/outer pair, so these are anchored on the
craft `Ship_AcquireLock` picked rather than parked on the screen - and `oag-game`
parses all nine and draws none, because the `<Mode3D>` layer has never had a
projection pass. That is the whole reason there is no lock-on graphic.

**A dead end worth not repeating**: `entity+0x85c`, the lock target, has exactly
three consumers in the executable - `Craft_Construct_q` sets it to `-1`,
`Ship_AcquireLock` writes it, `Ship_FireHeldWeapon` reads it. Nothing draws off
it. Whatever drives the sight reads the mirror `Ship_AcquireLock` writes to
`weapon_record+0x1b4`, and `0x1b4` is a common enough offset that a plain operand
sweep returns mostly noise - it needs the sweep scoped to the weapon-record
functions.

**Verified**: `just` green, and **five** disc-backed tests in
`crates/game/tests/missile_ground_truth.rs` - the four that were there plus
`a_missile_with_no_lock_still_flies_a_real_circuit_and_ends_itself`, which fires
from every slot on the shipped starting grid, proves at least one shot went up
unguided, and proves none of them outlives three seconds on real geometry. The
world hash did not move: no `Projectile` field was added, and the age is still
derived from `lifetime`.

## Open

- **The lock-on sight is not drawn.** Nine `<Mode3D>` models parse and none
  renders; the layer needs its own projection pass, and the sight additionally
  needs anchoring on the locked craft rather than on the screen.
- **The lock tone is not identified.** `~ROCKLOCK` (`0x08a79b98`) is the only
  plausible-looking string, no instruction references its pointer, and it sits at
  confidence **40** - below the threshold at which this project writes a name down.
- **What drives the sight is unread.** `weapon_record+0x1b4` is the mirror to
  sweep; `entity+0x85c` is a dead end and its three consumers are enumerated above.
- **`entity+0x860 & 1`** - the lock *flag* `Ship_FireHeldWeapon` gates on - has no
  writer identified. If the original builds a lock over time rather than
  instantly, that flag is where it lives, and this engine locks instantly.
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

- Build the `<Mode3D>` overlay projection, then drive `missile_sight_1..4` and
  `missile_sight_inner` off the lock so the player can see what a missile will
  chase. That is the single biggest user-visible gap this change leaves.
- Sweep `weapon_record+0x1b4` scoped to the weapon-record functions to find what
  reads the lock mirror - it should reach both the sight and the tone.
- Find the writer of `entity+0x860` before assuming the lock is instant.
- Decide the blast in one place: linear falloff on force, the struck craft
  excluded, damage on the direct hit alone. It moves every weapon, so it wants
  its own change and its own hash move.
