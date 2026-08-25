# HD's engine trail is one of four ribbons, and a craft flying through one sparks

2026-08-24, both from a player's sighting rather than a sweep. `WO_TRAIL_HITSHIP` is **wired** - chain, colour select and the three approximations on [engine-trail.md](../docs/ghidra/functions/ps3-hdfury-eu/engine-trail.md) and `Race::advance_trail_hits`. `WakeTrail` is **documented only** - [trail-ribbon.md](../docs/rendering/trail-ribbon.md). **Not on either page**: the psys inventory ground truth reads the PSP and PS2 discs only, so the two new HD effect names are checked by nothing there; and a sighting settling a code ambiguity (which craft the burst lands on) is evidence this project had not used before - cheaper than reading an SPU job, and worth reaching for again.

## Open

- `WakeTrail` is documented only, not wired
- The psys inventory ground truth reads only the PSP and PS2 discs, so the two new HD effect names (`WO_TRAIL_HITSHIP`, `WakeTrail`) are checked by nothing there

## Next Steps

- Extend the psys inventory ground truth to cover the two new HD effect names
