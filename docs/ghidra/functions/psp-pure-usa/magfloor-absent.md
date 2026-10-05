# Wipeout Pure has no magstrip effect (2026-10-05)

A clean negative for the Pulse magfloor effect
([magfloor-fx.md](../psp-pulse-usa/magfloor-fx.md)): Pure neither builds the
effect nor has the collision class that would trigger it. No function is
named, so this page adds no `names.tsv` rows.

## Evidence

All read off `BOOT.BIN` strings (`strings -a`), both pressings.

- Pure USA and EU contain no `visual_effects` string and no `MagEffect`
  string. Pulse's `BOOT.BIN` has both. Confidence 90 that Pure's code cannot
  name either file: a path could be assembled from fragments, but Pure's
  `%s`-built paths are all model or track names (`%s\track.vex`,
  `Data\Defaults\start_grid_%d.vex`).
- Pure's complete list of `Data\...` effect paths is `Data\Psys\WO_*.POB` (26
  weapon, ship and track effects) plus `Data\Weapons\*.vex` (bomb, mine,
  rocket, shield, disruptor, plasma). None is a magstrip or floor effect.
- `Mag Floor Collision` and `Cage Collision` are absent from Pure's
  executable, so Pure ships no magstrip surface class to trigger an effect.
  Already pinned by `class_table_ground_truth.rs`
  (`the_two_titles_share_one_class_name_table`), which now also asserts the
  two effect strings are absent from Pure and present in Pulse. Confidence 85.
- Pure's `Data.wad` (both regions) has neither MagEffect hash, matching the
  code side.

## Consequence

`WeaponModels::mag_floor` stays `None` for Pure as a finding, not a gap.
Nothing is wired. Not tried: a Pure runtime watch (no object to watch for).
