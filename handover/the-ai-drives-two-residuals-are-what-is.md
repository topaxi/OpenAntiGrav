# The AI drives; two residuals are what is left

2026-08-11 verified against the real disc (`the_ai_drives_the_field_along_the_track`, `a_driven_field_replays_identically`, disc-backed). Full account on [ai.md](../docs/gameplay/ai.md). Two traps paid for while landing it: (1) steering originally fed the **torque** accumulator, so a proportional term on line offset was a limit cycle at any gain - closed one derivative in (curvature -> target turn rate -> error against actual turn rate); (2) a three-point curvature estimate must divide by the chords' full length, not the distance between midpoints (half as much), or every craft takes every corner 1.41x too fast (`curvature_approximates_one_over_the_radius` pins it). Each opponent needs its **own** `Environment` - sharing the player's flies seven craft against the player's piece of track. Still open: reaction latency (deliberately unbuilt), and craft-to-craft collision's pending impulse at `entity->0x4c + 0x110` has no writer, which is what would arm the stun.

## Open

- Reaction latency is deliberately unbuilt
- Craft-to-craft collision's pending impulse at `entity->0x4c + 0x110` has no writer (this is what would arm the stun)

## Next Steps

- Add a writer for the pending impulse at `entity->0x4c + 0x110` to arm the stun
- Build reaction latency
