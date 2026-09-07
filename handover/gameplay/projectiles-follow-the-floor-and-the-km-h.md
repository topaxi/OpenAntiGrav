# Projectiles follow the floor, and the km/h fix - both now landed

2026-08-11, [rocket-visuals.md](../../docs/ghidra/functions/psp-pulse-usa/rocket-visuals.md) has the mechanism and isolation detail. `Rocket_Update` (`0x0885d2a8`) deflects off geometry rather than flying straight and detonating on first contact; authored speeds are km/h and `launch` now divides by 3.6 too, via `oag_core::math::SPEED_TO_KMH`. **The invented trail and blast are gone, and both were the thread's whole point.** `race::weapons::visuals` rides `ROCKET_FLARE_EFFECT` on the projectile from `Rocket_Init` (`0x0885cdb8`), plays `TRACK_BLAST_EFFECT`/`CRAFT_BLAST_EFFECT` per impact branch through `Race::blast_for`, and draws **nothing** for a modelled rocket rather than a billboard on top of the real flare. The pooling block is gone too: `psys::MAX_INSTANCES` is 24 pools of `MAX_PARTICLES` each, allocated up front. **What is still open is one RE question: whether the Cannon follows the floor** - the Missile does, the Cannon is unread.

## Open

- Whether the Cannon follows the floor like the Missile does is unread. `Rocket_Update` (`0x0885d2a8`) is the read that settled it for the Missile; the Cannon's equivalent has never been opened.

## Next Steps

- Read the Cannon's update in `psp-pulse-usa` `BOOT.BIN` and settle whether it deflects off geometry the way `Rocket_Update` does, or flies straight. That is the only thing left on this thread.
