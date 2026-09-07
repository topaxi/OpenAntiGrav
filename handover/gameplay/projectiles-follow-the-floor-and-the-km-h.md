# Projectiles follow the floor, and the km/h fix - both now landed

2026-08-11, [rocket-visuals.md](../../docs/ghidra/functions/psp-pulse-usa/rocket-visuals.md) has the mechanism and isolation detail. `Rocket_Update` (`0x0885d2a8`) deflects off geometry rather than flying straight and detonating on first contact; authored speeds are km/h and `launch` now divides by 3.6 too, via `oag_core::math::SPEED_TO_KMH`. **The in-flight smoke trail and impact blast remain invented**: all three rocket `.pob` effects parse and play whole, but nothing is wired to them - a maintainer's verdict **"much better, not faithful"**. What's blocking it is pooling, not transcription: a `psys::System` is sized for one effect (256 particles) and a full grid firing wants ~60 live particles per rocket across 24 rockets. Do the explosions first - most visibly wrong. **Still open: whether the Cannon follows the floor** - the Missile does, the Cannon is unread.

## Open

- In-flight smoke trail and impact blast are invented, not the real `.pob` effects, though those effects parse and play whole
- Wiring the real effects is blocked by pooling: `psys::System` is sized for 256 particles, a full grid firing wants ~60 live particles per rocket across 24 rockets
- Whether the Cannon follows the floor like the Missile does is unread

## Next Steps

- Wire the real rocket explosion effects first (most visibly wrong), which requires fixing `psys::System` pooling for ~60 particles/rocket x 24 rockets
