---
status: measure
titles: [pulse, hd]
updated: 2026-11-11
summary: "Read HD's Cannon update; no banked Cannon frame; Black rescues a craft ten times a race (Moa Therma capture done 2026-11-11)"
---

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

The de Konstruct AI-field ceilings raised with it (`ai_dekonstruct_black_ground_truth`
forward 6 -> 8, `ai_dekonstruct_symptoms_ground_truth` 05 FLASH 1 -> 3) were
attributed in lane `ai-loss-split`: 7 of the 10 extra deaths are weapon-dominant,
3 wall-dominant at the old scrape spots, deaths over 160 more races 110 -> 110,
a Cannon hit applies no impulse, and no AI change is owed. The ceilings stay.
`crates/raceplay/tests/ai_loss_attribution_board.rs` is the diagnostic
(`OAG_SWEEP=1`, release, `--ignored --nocapture`).

**2026-10-08 follow-up (lane `projectile-tilt`).** The maintainer asked whether
Rocket, Missile, Plasma and Shuriken share the fault ("across all titles"). They
do not: `projectile_tilt_ground_truth.rs` rides each on a bank on Pulse and HD
and all stay within 15 degrees of the floor under them. One real fault, the
Plasma falling along world `-Y` instead of its carried normal, is fixed
([projectile-floor.md](../../docs/gameplay/projectile-floor.md), last section).
Banked frames now exist.

## Open

- **Forward Black rescues a craft about ten times a race** (882 teleports in 80
  seven-craft races with weapons off, 791 with them on), and a late-lap craft is
  killed by wall scrape alone now and then (4 of 128 deaths scraped 95 or more,
  3 of 126 before the change; laps 5-6, in the changed build at reversed idx
  1926-1941 and 359).
  Neither moved with the Cannon; both are the AI's, not this thread's.

- **No banked Cannon frame.** `--pose x,y,z` puts a craft on the loop and
  `--give <weapon> --hold cross --press square` fires from there (that is how
  the 2026-10-08 rocket and missile frames were taken); the Cannon needs its
  hold, not captured yet.
- ~~The Cannon's flight against a live PPSSPP is unchecked.~~ **Checked 2026-11-11
  (`weapon-bank-follow`)** on Moa Therma's 85 degree bank: 24 of 24 updates of ours land on the
  original's to 0.000 unit, a floor glance included (`cannon-quake-leachbeam.md`, 2026-11-11
  section). Two differences found there, neither wired: the round's speed is 2.7 times ours, and
  the collision cage ends a round; both are in
  [projectiles-leave-the-track-the-original-cages-them.md](projectiles-leave-the-track-the-original-cages-them.md).
- **HD inherits Pulse's Cannon flight** (HD authors a Cannon and runs the
  shared code; no HD `Cannon_UpdateRound` read). **Not checked:** whether
  2048's `weaponstats_Race_2048.xml` or Omega's `WeaponStats_Race.xml` carry a
  Cannon block; if so they inherit the same flight.
- ~~Whether `craft+0xb10` is the craft's up or its contact normal was never separated.~~
  **Separated 2026-11-11:** it is the located track point's down, so the seed is the track's up,
  not `body.up()` (item 4 of the cage thread above). Not wired.

## Next Steps

1. ~~Capture a cannon shot on Moa Therma's loop in PPSSPP.~~ Done 2026-11-11 (above).
2. Read HD's Cannon update and record whether it matches Pulse's.
