# Pulse's weapon cues are read and mostly unwired - the wiring plan

2026-09-23. Landed the same evening the plan below was drafted: every "wire"
row is now in `crates/sound/src/sfx/cue.rs` (`Cue::ALL` grew from 14 to
28), with a push site in `crates/raceplay/src/weapons.rs`,
`crates/raceplay/src/weapons/single_instance.rs`,
`crates/raceplay/src/field/opponent_weapons.rs` or the sibling loop beside
`ignite_missile_bounces` in `crates/raceplay/src/tick.rs`. Per-cue evidence
and confidence live on each `Cue` variant's own doc comment and on the RE
page it cites (`rocket-visuals.md`, `missile.md`,
`cannon-quake-leachbeam.md`, `shuriken.md`, each carrying a 2026-09-23
"wired" note); `docs/overview/status.md`'s Pulse weapon-pieces table has the
per-weapon Audio cells, including the Quake row's correction (it used to
overclaim `QUAKELAUNCH`/`QUAKETRAVEL` as read on the page it cited).
`crates/game/tests/sfx_ground_truth.rs`, `sfx_weapon_ground_truth.rs` and
`race/tests/cues.rs` cover it, the first two run against real Pulse USA/EU,
PS2 EU, Pure USA/EU and HD discs.

**One correction the plan's own bank read got wrong**: `CANNONEXPLSHIP` is
not an empty cue. It owns exactly one command, opcode `0x05`, whose child
record indexes `CANNONEXPLWALL` directly - read straight off
`Bank::cue_children`/`Bank::resolve_child` against `pulse-psp-usa.chd`'s own
weapon bank, not inferred from a waveform count. So a Cannon's craft-hit
ending plays the same nine waveforms its wall-hit ending does, by the disc's
own construction - see `cannonexplship_is_a_child_reference_to_cannonexplwall`
in `crates/game/tests/sfx_weapon_ground_truth.rs`.

**2026-09-25: `LEACHENERGY`, `ROCKET`, `QUAKELAUNCH`, `~AUTOPILOT` and
`autopilot_eng` all wired**, and the missile.md/autopilot.md conflict over
`Ship_FireHeldWeapon` settled. `Cue::ALL` is now 33. See
`crates/sound/src/sfx/cue.rs`'s own per-variant doc comments,
`docs/ghidra/functions/psp-pulse-usa/autopilot.md`'s "`Ship_FireHeldWeapon`
opens both cues" section (the settling read), and
`docs/ghidra/functions/psp-pulse-usa/missile.md`,
`cannon-quake-leachbeam.md`, `rocket-visuals.md` and `docs/overview/status.md`
(each corrected to match). Tests: `a_locked_leachbeam_pulses_leachenergy_and_not_every_tick`
and `a_re_press_while_a_quake_wave_travels_still_raises_quakelaunch` in
`race/tests/cues.rs`, `the_autopilot_opens_a_held_voice_and_closes_it_when_the_pickup_expires`
in `sfx_weapon_ground_truth.rs` (moved there from `sfx_ground_truth.rs`
under the 1,000-line rule, alongside the shield's own equivalent test).

**2026-09-25: the Rocket/Cannon timeout-reap question is closed.**
`RocketPool_Update` (`0x0886de60`) and `CannonPool_Update` (`0x088582b0`)
were both read whole against `psp-pulse-usa`'s `BOOT.BIN`, including their
teardown's own callees (`FUN_088f3298`, `FUN_08939bcc`, `FUN_08939460` for
the Rocket - none reach `Sound_Play`). Both reach the same fork: `Sound_Play`
only ever branches on flag `0x10` (wall) versus `0x20` (craft) - there is no
third arm. The two functions gate entry into that block differently (the
Rocket's age check sets flag `0x4` first, then tests it; the Cannon's is an
`||` on the gate itself, `1.0 < age || (flags & 4) != 0`), but a round that
times out without recording a hit sets neither `0x10` nor `0x20` either way,
so the original plays nothing on that path, matching what this port already
did. Separately confirmed that this port's own `~ROCKETTVL` loop
(`TravelVoices::tick`) stops on the same tick a reap resets the slot,
independent of whether an `Impact` was written - not a gap the reap finding
exposed. See `rocket-visuals.md`'s "What a rocket hit spends" section and
`cannon-quake-leachbeam.md`'s Cannon section for the decompiled teardown and
the addresses.

## Open

- **2026-09-30 (pulse-weapon-audio lane): the four triggers were all recovered
  and wired, and the Cannon round now lives 1.0 s.** `Cue::ALL` is 38.
  `MISSILEEXPSHIP` (`MissilePool_Update`, craft-hit bit `0x20`) and the Missile's
  fuse cue - which the original's own pointer makes `SHURIKENEXPL` - fire from the
  impact loop; `LEACHFAIL` from `LeachBeam_InitUnlocked`'s unlocked arm; `SHURIKEN`
  from `Shuriken_Init`'s throw; `~QUAKETRAVEL` is held while a wave travels. The
  Shuriken's travel and hit cues moved onto the blade's own 300-unit emitter,
  which the same read showed exists. Evidence and addresses are on
  `missile.md`, `shuriken.md` and `cannon-quake-leachbeam.md`, each with a
  2026-09-30 section; tests are `race::tests::cue_endings`,
  `sfx_weapon_ground_truth::a_quake_wave_holds_its_travel_loop_for_as_long_as_it_lasts`
  and `projectile::tests::a_cannon_round_that_hits_nothing_is_reaped_at_one_second`.
- **`~QUAKETRAVEL` is one voice, not two**: the original opens one per road span
  (up to two, when the wave crosses a join). Chosen, not measured. **And it is
  flat-picked**: the cue authors two loops of one waveform at 30 and 330 degrees
  (the second bent by 2 semitones), and `TravelVoices::follow` picks one and plays
  it centred. The same flat pick applies to the other travel loops.
- **A Missile that spends its bounce budget on a wall plays nothing here**: its
  teardown plays `MISSILEEXPWALL` off bit `0x10`, whose setter (in `Missile_Update`)
  is unread. The fuse and craft-hit endings are wired.
- **HD**: its bank carries `SHURIKENEXPL`, `MISSILEEXPSHIP`, `~QUAKETRAVEL`,
  `LEACHFAIL` and `SHURIKEN`, so they play there on the strength of Pulse's binary
  alone; HD's own call sites are unread.
- **Not heard by a human yet**: none of the newly wired cues has been listened
  to; only the mixer's counts and the command lists were checked.
- **Not read**: whether an opponent's Quake or Shuriken launch sounds through the
  same call sites at the same volume (the port raises them for any craft).

## Next Steps

1. Listen to a Missile fuse ending (`SHURIKENEXPL`) once: if it sounds wrong
   against a PPSSPP capture, the pointer cell `0x08a7c950` is the place to look.
