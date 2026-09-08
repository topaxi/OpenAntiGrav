# HD's engine trail is one of four ribbons, and a craft flying through one sparks

2026-08-24, both from a player's sighting rather than a sweep. `WO_TRAIL_HITSHIP` is **wired** - chain, colour select and the three approximations on [engine-trail.md](../../docs/ghidra/functions/ps3-hdfury-eu/engine-trail.md) and `Race::advance_trail_hits`. `WakeTrail` is **documented only** - [trail-ribbon.md](../../docs/rendering/trail-ribbon.md). **Not on either page**: the psys inventory ground truth reads the PSP and PS2 discs only, so the two new HD effect names are checked by nothing there; and a sighting settling a code ambiguity (which craft the burst lands on) is evidence this project had not used before - cheaper than reading an SPU job, and worth reaching for again.

## Open

- `WakeTrail` is documented only, not wired
- **Correction, 2026-08-25:** the "extend the psys inventory ground truth to
  cover `WakeTrail`" framing below was wrong. `WakeTrail` is an SPU ribbon
  manager, structurally `TrailEffectManager`'s twin - it has no `.pob`
  particle-system asset at all (confirmed: no `waketrail.pob` anywhere across
  HD's seven archives), so a POB inventory test cannot cover it categorically,
  not just as a matter of scope. Wiring it is [trail-ribbon.md](../../docs/rendering/trail-ribbon.md)'s
  "not implemented, and what it would take" section, unrelated to this thread.
- **The inventory ground truth now reads HD, narrowly.**
  `crates/game/tests/psys_inventory_ground_truth.rs`'s `mod hd` pins that
  every `RACE_EFFECTS` name (including both `WO_TRAIL_HITSHIP` variants) is
  really on the disc, and that `WO_TRAIL_HITSHIP_RED` is authored only in
  `DATA06` - corroborating engine-trail.md's "red variant on a Fury skin"
  reading from the disc's own archive membership, independent of the
  executable's variant-select instruction. **It deliberately stops short of
  the three-bucket completeness sweep the PSP and PS2 discs get**: HD authors
  82 distinct particle systems against `RACE_EFFECTS`' wired count (see
  correction below for that number), and the rest include whole unbuilt game
  modes (Detonator, Nitro - see
  `docs/ghidra/functions/ps3-hdfury-eu/mode-manager.md`), per-damage-tier hull
  states, and several `Leach Beam`/`Plasma`/weapon sub-effects new to HD.
  Bucketing each on the strength of "that system isn't built" would be the
  plausibility-over-evidence shortcut this file's own doc comment forbids for
  the two discs it already covers - each needs its own executable
  evidence, the same bar `NO_TRIGGER_RECOVERED`'s existing 27 entries hold to.
- **Correction, 2026-09-08: the "7 wired" / "75 remaining" arithmetic above
  was already stale when written.** `RACE_EFFECTS` (`crates/game/src/race/effects.rs`)
  is 15 long today - Plasma, Shuriken (both `_HEAD` and `_BOUNCE`) and Quake
  landed 2026-09-02, two weeks after this bullet's 2026-08-24 note counted 7 -
  and the two counts drifted independently in three places (this file said 7,
  `psys_inventory_ground_truth.rs`'s own `mod hd` doc comment said 11, neither
  matched the other or the real 15) until cross-checked against the source
  directly this session. All 15 names being present on the disc is already
  what `mod hd`'s own `every_wired_effect_is_on_the_disc` test asserts
  (`just test-data`, or `cargo nextest run -p oag-game --run-ignored all -E
  'binary(psys_inventory_ground_truth)'`), so this correction rests on
  existing, reproducible coverage rather than a fresh one-off scan.
  **82 - 15 wired = 67 unbucketed**,
  a number worth re-deriving from `RACE_EFFECTS.len()` rather than trusting
  literally, the same lesson this correction exists to teach.
- **2026-09-08: six names confirmed, all executable-checked rather than
  inferred.** Internal `SYSP` names read at the `0x10 + slots * 4` name field
  (`just psarc cat … | xxd`); trigger absence checked two independent ways -
  `strings -a` over `EBOOT.elf` and Ghidra's own defined-string table
  (`search_strings`, once a live instance became available mid-session) -
  both agreeing on zero hits for every name below:
  - `NUMBERS`, `TEST_BOMBSPIKES` (both `DATA02`-only): disc-authored
    debug/test assets. **The control that matters**: this isn't just "unwired
    means no string" - a `Data\Psys\...POB` load-path scan finds 46 of the 82
    disc names as a string, including plenty this engine hasn't built at all
    (`WO_LEACHBEAM_CHARGING`, `WO_BOMB_EXPLO_DETONATOR`, every `WO_DAMAGE_*`
    tier), so the original executable's string table tracks its own load
    paths, not OAG's wiring - these two sitting outside that 46 says
    something about the original game, not about this project's scope.
  - `WO_BLUE_WELDER`, `WO_MODESTO_STEAM_A` (`DATA02`), `WO_UNDERWATER_GODRAYS`
    (`DATA02`, new to HD) and `WO_DustMotes` (`DATA00`, the only mixed-case
    name in the corpus, paired with its own `dustmotes_4x4.gtf` texture in
    the same archive - which is why it reads as a real shipped asset despite
    the odd casing, not a second debug leftover): environment effects, no
    recovered placement trigger. `WO_BLUE_WELDER` and `WO_MODESTO_STEAM_A` are
    exactly the two names the PSP's own `NO_TRIGGER_RECOVERED` list already
    carries under that reason - this confirms it independently on HD's own
    executable instead of assuming it carries over unchanged, and the other
    two extend the same category to names new to HD.

  Confidence 82 for all six (not higher: for the first two, 36 authored names
  are *also* missing from the 46-string load-path scan, so absence alone
  isn't proof; for the environment four, no code path was traced for a track
  format that might place them some way that never puts the name in a
  string). Full writeup: `docs/formats/pob.md`'s HD corpus section. Still no
  code-side change - six of 82 doesn't justify assembling an HD-side
  `NO_TRIGGER_RECOVERED` const yet (see
  `crates/game/tests/psys_inventory_ground_truth.rs`'s `mod hd` doc comment).

## Next Steps

- Bucket HD's other 61 distinct particle systems (82 total minus 15 wired
  minus the 6 confirmed above) into "no trigger recovered, with why" or a
  genuinely new trigger, one name at a time against the executable - not by
  inference from an already-established "that weapon/mode isn't built" fact,
  which is not the same as reading *that name's own* trigger.
  `stesparkstest.pob` is confirmed already, incidentally, while measuring the
  inventory count above: its internal `SYSP` name field is not `STESPARKSTEST`
  at all but `WO_SHIP_COLL_SPARK_DAMAGE` - a leftover copy of the wired
  collision spark under a test filename, not a fourth debug name to chase.
  `DATA02` carries a second such leftover, `wo_ship_explosion_lightshafts.pob`
  internally named `WO_SHIP_EXPLOSION`, which is a `NO_TRIGGER_RECOVERED`-style
  entry in its own right once someone's ready to write one (it is already
  known to be the same name as the wired-but-unbuilt `WO_SHIP_EXPLOSION`
  reason, just a second file, not a second effect). The remaining 61,
  unread this session, include the `WO_LEACHBEAM_*` family (9 names),
  `WO_PLASMA_*` non-head files, both `_DETONATOR` bomb variants, both
  `WO_QUAKE_DETONATOR*` names, `WO_NITRO_*`, `WO_REPULSER*`, every
  `WO_SHIP_*` damage/death/explosion variant beyond the wired collision
  spark, and a handful with no obvious sibling to reason from
  (`WO_CANNON_HOTSPOT`, `WO_CANNON_MUZZLEFLASH`, `WO_MAGSTRIP_LIGHTNING`,
  `WO_LIGHTBARRIER_EXPLO`, `WO_PULSECANNON_MUZZLE`) - see
  `docs/ghidra/functions/ps3-hdfury-eu/weapons.md` for what's already read
  about the weapon table before assuming any of these needs fresh RE from
  zero. Once enough of the 61 are read to be worth assembling, an HD-side
  `NO_TRIGGER_RECOVERED` const in `psys_inventory_ground_truth.rs`'s
  `mod hd` - mirroring the PSP/PS2 one - is the natural place to enforce the
  bucketing the same way those two discs already are, rather than leaving it
  as prose here indefinitely.
