# Pure and HD lock on too, and it cost two axes rather than any new recovery

2026-08-26, [lock-sight.md](../../docs/ghidra/functions/psp-pulse-usa/lock-sight.md)
and [weapon-stats.md](../../docs/formats/weapon-stats.md) - the pages to read, not
this row. **All three titles now take a lock and draw a reticle**, and the
recovered law did not change once: `HudSight_Update`'s placement, its `0.8`-second
hold, its chase and its `w > 0` guard are engine code with no title in them. What
was in the way was per-title *data* wearing Pulse's spelling.

**Pure could not find its weapons.** `Race::load` opened
`Data\XML\WeaponStats_Race.xml` on every title; Pure names one lower-cased
**`Data\XML\weaponstats.xml`** and ships no Eliminator variant. Found in one
string search of `/psp-pure-usa/BOOT.BIN` - `0x08a445a0`, three strings before the
`"WeaponStats"` and `"Weapon"` element names its own parser matches, which is what
says it is that parser's file and not some other table. Now
`oag_title::weapons::Weapons`, an axis because the shapes differ: two files chosen
by race mode against one that every mode reads.

**And its table would not have parsed anyway.** Pure authors a single
`speed="950"` per weapon where Pulse authors `venomspeed`/`flashspeed`/
`rapierspeed`/`phantomspeed`, and **no `launchSpeed` at all**. Both accepted now:
the per-class spelling wins where present, one `speed` folds into all four, and a
missing `launchSpeed` is zero. Folding one into four is not a stand-in - Pure
genuinely flies every class at the same weapon speed, so there is nothing per
class to lose.

**HD had the lock all along and drew the reticle nowhere near the craft.** Its
`WeaponStats_Race.xml` parses, so `Ship_AcquireLock`, the hold and the unguided
shot already ran there. Two things were wrong. Its reticle is not the PSP's:
**six concentric `<Image>` sprites** off `Data\HUD\Textures\missile_reticule.gtf`
at authored sizes 128/108/80/64, a red outer and a green inner, with a separate
`LockedOn` pair - where the PSP titles instance one corner-bracket model four
times and rotate each. That is `oag_title::hud::Sights`, the second axis.

**The other was the screen, and it is the one that reads as a bug rather than a
gap.** `sight::SCREEN` was a constant 480x272 while HD's HUD is authored in
1920x1080, so HD's reticle projected into the top-left ninth of the frame and sat
*inside its lap counter* - a stray widget, not a missing one. `Sight` now carries
the grid it works in, off `hud::Assets::space`.

**The placeholder idiom is what confirms the whole reading**, and it is the same
on all three: every sight widget on every title is authored centred on
`(-width/2, +height/2)` of that title's own screen - `(-240, 136)` on the PSP,
`(-960, 540)` on HD. Same arithmetic, different number, all overwritten every
frame.

**One thing is read off names rather than out of code**: which of HD's six is the
seeking set and which two the locked one. `MissileSightLockedOnLines` and
`MissileSightLockedOnMiddle` are unambiguous about *what* they are and silent
about whether the seeking set stays up underneath, so they are drawn
**additively** - every authored widget shown rather than some hidden on a guess.
Confidence 70, and nothing of HD's own HUD code has been disassembled for it.

**Verified**: `just` green, five disc-backed tests in
`crates/game/tests/lock_sight_ground_truth.rs` covering all three titles, four
new draw tests for the concentric dialect and the unread one, and a capture per
title showing the reticle over a craft ahead.

## Open

- ~~**Which of HD's six widgets is up when is read off their names**, at
  70.~~ **HD's own sight update is disassembled now, 2026-09-15** - see
  [hud-sight.md](../../docs/ghidra/functions/ps3-hdfury-eu/hud-sight.md).
  It found something the name-reading did not predict: the Missile and the
  LeachBeam are each driven by their **own** per-tick update function
  (`Hud_UpdateMissileSight`, `Hud_UpdateLeachBeamSight`), not one function
  switching on held weapon, and HD's shared hold time is **`0.5` s**, not the
  PSP's `0.8` - a real, unadopted divergence (`oag_race::sight::HOLD_SECONDS`
  is still the PSP's `0.8`, shared and title-blind, and changing it is a
  simulation-behaviour change outside a presentation-only lane's scope).
  The `LockedOn` pair's own "additive" reading is now suspect rather than
  confirmed: the LeachBeam's equivalent mechanism is a distance-gated reveal
  across its four widgets, not a bare on/off, and whether the Missile's pair
  is the same mechanism is unchecked - see the page's "Not read this pass".
- ~~**HD's four `LeachBeamSight*` are unwired**~~ **Wired 2026-09-15** for
  both HD and 2048 (`oag_title::hud::Sights::Concentric::leach`,
  `crates/game/src/hud/sight_draw.rs`) - drawn as all four together whenever
  the LeachBeam reticle is up, **chosen, not measured**: `hud-sight.md`'s
  finding above is that the original reveals the four one at a time as the
  lock progresses, which this engine does not reproduce. Pure still authors
  none, unchanged.
- **The LeachBeam's distance-breakpoint table is unnamed.** Both
  `Hud_UpdateMissileSight` and `Hud_UpdateLeachBeamSight` index a
  weapon-stats-shaped pointer by small negative offsets to decide which
  widget's hide bit to clear; naming that table is what would turn "chosen"
  above into "measured".
- **Pure's Eliminator tuning is one file, and nothing reads the axis's second
  row.** `oag_title::weapons::Weapons::elimination` is carried and unread:
  `Race::load` opens the race table for every mode on every title.
- ~~**Pure ships weapons this build does not model**~~ **Done 2026-09-15.**
  The roster is compared in `docs/formats/weapon-stats.md`'s Pure dialect
  section (ten weapons, not nine - the fuse-less Bomb was the tenth and the
  decoder used to skip it); the `Disruptor` is decoded, fired, flown and
  landed - `oag_gameplay::projectile::disruptor`, `oag_gameplay::disruption`,
  `docs/ghidra/functions/psp-pure-usa/weapons.md`. **Still open on it**: the
  three effects that land without a force (Drunk - its `amount`'s input scale
  in `Ship_ApplySteeringTorque` `0x0892edfc`; Rubber Ship - `FUN_0892dea8`'s
  hover damping `* 0.2`; Drunk Camera), every visual and sound
  (`WO_DISRUPTOR_HEAD`, `~DISRUPTORTVL`, `DISRUPTOREXPWAL`/`SHP`,
  `disruptor_effect.vex`, `disruptor_cockpit.vex`), the pad-time roll (this
  engine rolls at the press), and a sight for it so the player's bolt can
  home the way `DisruptorPool_Fire` (`0x0884fc14`) passes `craft+0x194`.
- ~~**Pure's Bomb is not decoded**~~ **Settled 2026-09-15**: `BombStats::
  timetodie` is an `Option`, `None` is a charge that sits until tripped
  (`Drop::fuse = NO_FUSE`), and `damageradius` (`0x08b1786c`) has **no
  reader** in the executable - the evidence page's Bomb section.
- **The far-target alpha and HD's own blink** are still unreproduced, as on
  Pulse. `hud-sight.md`'s colour writes go through `Image_SetVertexColours`
  with a computed alpha byte; whether any of that arithmetic is the PSP's
  same 96/255 ratio is unchecked.

## Next Steps

- Name the distance-breakpoint table `Hud_UpdateLeachBeamSight` (and
  `Hud_UpdateMissileSight`) index, to settle the reveal order the widgets
  fill in and replace the engine's current all-four-together choice with the
  measured one.
- Check whether `Hud_UpdateMissileSight`'s own `LockedOnLines`/
  `LockedOnMiddle` toggle is the same distance-reveal mechanism as the
  LeachBeam's, rather than the flat "additive" reading at 70.
- Decide whether HD's measured `0.5` s hold becomes a per-title
  `oag_race::sight` constant or stays the PSP's `0.8` on every title; either
  is a gameplay-crate change and a hash move, not this lane's.
- Finish the Disruptor's three unbuilt effects and its visuals; see the
  struck-through item above for the addresses.
