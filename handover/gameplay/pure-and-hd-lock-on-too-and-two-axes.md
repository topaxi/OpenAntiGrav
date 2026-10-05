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
- ~~**The LeachBeam's distance-breakpoint table is unnamed.**~~ **Corrected
  and ported, 2026-09-16.** There was no table: `hud-sight.md`'s own
  2026-09-15 pass had misread three TOC-relative literal loads as a
  weapon-stats pointer index; they are three static floats, exact quarters of
  the shared `0.5` s hold constant. `oag_race::sight::Sight::hold_progress`
  and `oag_game::hud::sight_draw::leach_reveal_draws` now drive HD/2048's
  three-ring reveal off it, scaled onto this engine's own `HOLD_SECONDS`
  rather than HD's absolute seconds - see `lock-sight.md`'s own
  "Colour and blink, resolved" pass for the sibling colour finding from the
  same session.
- **Pure's Eliminator tuning is one file, and nothing reads the axis's second
  row.** `oag_title::weapons::Weapons::elimination` is carried and unread:
  `Race::load` opens the race table for every mode on every title.
- ~~**Pure ships weapons this build does not model**~~ **Done 2026-09-15.**
  The roster is compared in `docs/formats/weapon-stats.md`'s Pure dialect
  section (ten weapons, not nine - the fuse-less Bomb was the tenth and the
  decoder used to skip it); the `Disruptor` is decoded, fired, flown and
  landed - `oag_weapons::projectile::disruptor`, `oag_weapons::disruption`,
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
- ~~**The far-target alpha ... is still unreproduced.**~~ **Read in full,
  2026-09-16, on the PSP side - not a tuning constant at all.** Pulse's
  "near" test samples the rendered frame's own Z-buffer at the reticle's
  previous screen position (`Hud_SampleSightDepth`, `g_hud_sight_depth_sample`
  - see `lock-sight.md`'s new section). Still not ported: reproducing it needs
  a depth buffer inside `oag_race`, which the dependency rules forbid. **HD's
  own blink is still unread** - whether `Image_SetVertexColours`'s alpha byte
  on that binary follows the PSP's 96/255 ratio remains unchecked.

## 2026-09-16: the reticle's colour, off byte order and gameplay-hash checks

Reported from play: "the lock-on mechanisms for missile and leech beam are
implemented, but do not look exactly like the original." Closed the three
measured-but-unreproduced gaps this thread and `lock-sight.md` already named,
plus the LeachBeam reveal port `hud-sight.md`'s 2026-09-15 pass had found but
not wired up:

- **The reticle's hue is recovered and built.** `HudSight_Update`'s packed
  colour word, read against this project's own established byte order (the
  same one `Loading_DrawWave`'s ramp fixed): locked is red, seeking
  alternates yellow (full brightness) with white ([`BLINK_TINT`] the
  brightness). `oag_race::sight::Sight::tint` now returns `[r,g,b]`;
  `Sight::brightness` keeps the old scalar for HD's own-coloured concentric
  widgets, which are deliberately not given this hue - HD authors its own
  ring colours and nothing has read whether its blink shares this mechanic.
  Screenshotted on `pulse-psp-eu.chd`: yellow while seeking, white on the
  alternating blink phase, red once locked - see the lane report for paths.
- **The far-target alpha step is not a tuning constant.** It samples the
  rendered frame's own Z-buffer at the reticle's previous screen position, a
  one-frame-lagged occlusion test - `oag_race` cannot reach a depth buffer
  (`CLAUDE.md`'s dependency rules), so this stays unported, now for a
  structural reason rather than an unread global.
- **HD/2048's LeachBeam reveal is ported.** `Sight::hold_progress` (0..1,
  scaled onto this engine's own `HOLD_SECONDS`) drives
  `sight_draw::leach_reveal_draws`, replacing the old all-four-together draw
  with the measured one-ring-at-a-time reveal, at reduced confidence because
  the port scales HD's absolute quarter-seconds onto a different hold
  constant. Screenshotted on `hdfury-ps3-eu-dec.iso`.
- **`Sight` carries no `World`/hash coverage** - it lives on `Race`'s own
  view state, not `sim.world` (`crates/game/src/race/weapons.rs`), so none of
  the above moved a determinism hash. Checked directly, not assumed.
- **`HOLD_SECONDS` untouched, deliberately** - still the PSP's `0.8` on every
  title; the HD-vs-PSP hold-time question above remains unresolved by this
  pass, on purpose.

New tests: `oag-game`'s `reticle_tests.rs` gained a hue test and a
progressive-reveal test with the fixture's own rect-size assertions since the
BG and Outer widgets are authored at the same size and only the count (not
the identity) is checkable for that stage; two existing "all four together"
LeachBeam tests were rewritten for the new behaviour rather than deleted. All
ten `lock_sight_ground_truth` tests still pass against real discs.

## Next Steps

- Check whether `Hud_UpdateMissileSight`'s own `LockedOnLines`/
  `LockedOnMiddle` toggle is the same threshold-reveal mechanism as the
  LeachBeam's or a flat all-or-nothing pair-add - `hud-sight.md`'s 2026-09-16
  pass narrowed this to confidence 60 ("closer to all-or-nothing") without
  fully tracing `Hud_UpdateMissileSight`'s own accumulator
  (`param_2+0xec`); not adopted into the engine's still-additive reading at
  70 pending that trace.
- Decide whether HD's measured `0.5` s hold becomes a per-title
  `oag_race::sight` constant or stays the PSP's `0.8` on every title; either
  is a gameplay-crate change and a hash move, not this lane's.
- Read HD's own blink arithmetic (if any) to say whether
  `Sight::brightness`'s reused PSP scalar is right for the concentric
  dialect or just an unverified placeholder that happens not to be wrong yet.
- `FUN_0890906c` (`0x0890906c`, `psp-pulse-usa`) is read only as far as "it
  samples the sight's own depth and also does something else with a
  neighbourhood count" - below 50 confidence on the second half, not named.
  A full read would settle what `*(float*)(DAT_08b62d34+0x74)` feeds.
- Finish the Disruptor's three unbuilt effects and its visuals; see the
  struck-through item above for the addresses.
