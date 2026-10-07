# The Cannon flew a floor probe the original does not, and left its muzzle on a bank

2026-10-07, [projectile-floor.md](../../docs/gameplay/projectile-floor.md) ("The
Cannon is not in this table at all") - the page to read. Maintainer report: on a
tilted track (Moa Therma's loop) the cannon fires with an offset to the side of
the craft. The cause was the flight, not the spawn: the round ran the generic
floor-follower seeded with world up. `Cannon_UpdateRound` (`0x0886593c`) has no
probe, ride or fall; it now sweeps straight, ends on a wall and reflects off a
floor with a 3.0 push-off. Missile, Shuriken and Plasma are now born riding the
craft's up, as `Missile_Init`, `Shuriken_Init` and `Plasma_Launch` store it.
Moa Therma, 496 rounds: worst off-muzzle error 7.05 units before, 0.00 after.

## Open

- **No banked frame was captured.** `--autopilot` fires nothing and a hand-held
  `--race` cannot reach the loop, so the screenshots are flat-track only
  (`data/scratch/weapon-tilt/shots/`); the bank is covered by
  `cannon_tilt_ground_truth` alone. A `--give`-style autopilot-with-fire flag would
  let a picture show it.
- **The Cannon's flight against a live PPSSPP is unchecked.** The decompile is
  confidence 82 (VFPU-dense); the reflect arm is read at the shape of its
  arithmetic only. Nobody has watched a round glance off a floor in the original.
- **HD inherits Pulse's Cannon flight** (HD authors a Cannon and runs the
  shared code; no HD `Cannon_UpdateRound` read). **Not checked:** whether
  2048's `weaponstats_Race_2048.xml` or Omega's `WeaponStats_Race.xml` carry a
  Cannon block; if so they inherit the same flight.
- The Plasma's recovered seed is `-craft->up`; whether `craft+0xb10` is the
  craft's up or its contact normal was never separated (the Rocket's note says
  the same), so all four weapons use `body.up()`.

## Next Steps

1. Capture a cannon shot on Moa Therma's loop in PPSSPP (software, muted) and
   compare the round's path with the craft's orientation, to confirm the straight
   flight at the bend.
2. Read HD's Cannon update and record whether it matches Pulse's.
