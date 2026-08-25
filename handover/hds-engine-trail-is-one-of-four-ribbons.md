# HD's engine trail is one of four ribbons, and a craft flying through one sparks

2026-08-24, both from a player's sighting rather than a sweep. `WO_TRAIL_HITSHIP` is **wired** - chain, colour select and the three approximations on [engine-trail.md](../docs/ghidra/functions/ps3-hdfury-eu/engine-trail.md) and `Race::advance_trail_hits`. `WakeTrail` is **documented only** - [trail-ribbon.md](../docs/rendering/trail-ribbon.md). **Not on either page**: the psys inventory ground truth reads the PSP and PS2 discs only, so the two new HD effect names are checked by nothing there; and a sighting settling a code ambiguity (which craft the burst lands on) is evidence this project had not used before - cheaper than reading an SPU job, and worth reaching for again.

## Open

- `WakeTrail` is documented only, not wired
- **Correction, 2026-08-25:** the "extend the psys inventory ground truth to
  cover `WakeTrail`" framing below was wrong. `WakeTrail` is an SPU ribbon
  manager, structurally `TrailEffectManager`'s twin - it has no `.pob`
  particle-system asset at all (confirmed: no `waketrail.pob` anywhere across
  HD's seven archives), so a POB inventory test cannot cover it categorically,
  not just as a matter of scope. Wiring it is [trail-ribbon.md](../docs/rendering/trail-ribbon.md)'s
  "not implemented, and what it would take" section, unrelated to this thread.
- **The inventory ground truth now reads HD, narrowly.**
  `crates/game/tests/psys_inventory_ground_truth.rs`'s `mod hd` pins that
  all 7 `RACE_EFFECTS` names (including both `WO_TRAIL_HITSHIP` variants) are
  really on the disc, and that `WO_TRAIL_HITSHIP_RED` is authored only in
  `DATA06` - corroborating engine-trail.md's "red variant on a Fury skin"
  reading from the disc's own archive membership, independent of the
  executable's variant-select instruction. **It deliberately stops short of
  the three-bucket completeness sweep the PSP and PS2 discs get**: HD authors
  82 distinct particle systems against 7 wired, and the other 75 include
  whole unbuilt game modes (Detonator, Nitro - see
  `docs/ghidra/functions/ps3-hdfury-eu/mode-manager.md`), per-damage-tier hull
  states, and several `Leach Beam`/`Plasma`/weapon sub-effects new to HD.
  Bucketing each on the strength of "that system isn't built" would be the
  plausibility-over-evidence shortcut this file's own doc comment forbids for
  the two discs it already covers - each of the 75 needs its own executable
  evidence, the same bar `NO_TRIGGER_RECOVERED`'s existing 27 entries hold to.

## Next Steps

- Bucket HD's other 75 distinct particle systems (82 total minus the 7
  wired) into "no trigger recovered, with why" or a genuinely new trigger,
  one name at a time against the executable - not by inference from an
  already-established "that weapon/mode isn't built" fact, which is not the
  same as reading *that name's own* trigger. `numbers.pob` and
  `test_bombspikes.pob` look like disc-authored debug/test assets rather than
  shipped effects and are a reasonable place to start, since confirming that
  is a small, bounded Ghidra string search rather than a weapon-system read.
  `stesparkstest.pob` is confirmed already, incidentally, while measuring the
  inventory count above: its internal `SYSP` name field is not `STESPARKSTEST`
  at all but `WO_SHIP_COLL_SPARK_DAMAGE` - a leftover copy of the wired
  collision spark under a test filename, not a fourth debug name to chase.
  `DATA02` carries a second such leftover, `wo_ship_explosion_lightshafts.pob`
  internally named `WO_SHIP_EXPLOSION`, which is a `NO_TRIGGER_RECOVERED`-style
  entry in its own right once someone's ready to write one (it is already
  known to be the same name as the wired-but-unbuilt `WO_SHIP_EXPLOSION`
  reason, just a second file, not a second effect).
