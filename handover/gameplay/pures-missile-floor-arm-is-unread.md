---
status: re-first
titles: [pure]
updated: 2026-11-25
summary: "Pure's Rocket and Plasma floor law is measured and wired; its Missile still runs Pulse's arms, its own Missile_Update unread"
---

# Pure's Missile floor arm is unread

2026-11-25, lane `psp-floor-law`. Pure's Rocket and Plasma ride their launch normal
(`FloorArms::LaunchNormal`, `docs/ghidra/functions/psp-pure-usa/floor-follow.md`); every other
Pure flyer keeps Pulse's arms through `FloorArms::for_other_weapons`.

## Open

- Pure's `Missile_Update` (`0x0885cf30`): it stores `+0xd0` at `0x0885d410`, so unlike the Rocket's
  its carried normal may move. Read its two raycasts and floor arms the way `floor-follow.md` reads
  the Rocket's; one live PPSSPP flight settles it (weapon id 2 in `pure_floor_probe`'s method).
- A Pure volley fired at Vineta K's first corner never hit the floor (`v0 = 0` on every probe, the
  rockets fell to y -532). The craft had stopped against the corner's wall; whether the volley left
  the floor ahead or the probe misses there for another reason is not separated.
- Pure's steepest bank is not censused: `oag-trace track` refuses a Pure image. The 15 degree floor
  on Sebenco Climb is the steepest a shot met.

## Next Steps

1. Read Pure `Missile_Update` in Ghidra (`/pure/BOOT-psp-pure-usa.BIN`), compare with Pulse's
   `Missile_Update` (`0x0885a918`); about 30 minutes.
2. Fire one Missile on Sebenco Climb's restart state and log `+0xd0` per update; about 20 minutes
   with the method on `floor-follow.md`.
3. If it differs, give the Missile its arm in `oag_weapons::projectile::launch_normal` and a chained
   row in `crates/raceplay/tests/projectile_bank_ground_truth.rs`.
