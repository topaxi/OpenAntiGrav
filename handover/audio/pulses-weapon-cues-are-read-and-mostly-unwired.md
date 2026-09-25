# Pulse's weapon cues are read and mostly unwired - the wiring plan

2026-09-23. Landed the same evening the plan below was drafted: every "wire"
row is now in `crates/game/src/audio/sfx/cue.rs` (`Cue::ALL` grew from 14 to
28), with a push site in `crates/game/src/race/weapons.rs`,
`crates/game/src/race/weapons/single_instance.rs`,
`crates/game/src/race/field/opponent_weapons.rs` or the sibling loop beside
`ignite_missile_bounces` in `crates/game/src/race/tick.rs`. Per-cue evidence
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
`crates/game/src/audio/sfx/cue.rs`'s own per-variant doc comments,
`docs/ghidra/functions/psp-pulse-usa/autopilot.md`'s "`Ship_FireHeldWeapon`
opens both cues" section (the settling read), and
`docs/ghidra/functions/psp-pulse-usa/missile.md`,
`cannon-quake-leachbeam.md`, `rocket-visuals.md` and `docs/overview/status.md`
(each corrected to match). Tests: `a_locked_leachbeam_pulses_leachenergy_and_not_every_tick`
and `a_re_press_while_a_quake_wave_travels_still_raises_quakelaunch` in
`race/tests/cues.rs`, `the_autopilot_opens_a_held_voice_and_closes_it_when_the_pickup_expires`
in `sfx_weapon_ground_truth.rs` (moved there from `sfx_ground_truth.rs`
under the 1,000-line rule, alongside the shield's own equivalent test).

## Open

- **What the original plays when a Rocket or a Cannon round times out
  without hitting anything.** `rocket-visuals.md`'s "What a rocket hit
  spends" section reads the teardown as branching on the round's own
  `0x10`/`0x20` flags, and a reap sets neither - so the branch this project
  has read plays neither `ROCKEXPLWALL` nor `ROCKEXPLSHIP` on that path, and
  this port plays nothing there either. Whether the original's own teardown
  call has a third arm, or genuinely plays nothing, is unread.
- `MISSILEEXPSHIP`, `~QUAKETRAVEL`, `LEACHFAIL` and the Shuriken's own launch
  cue still have no recovered trigger and stay unwired - see each `Cue`
  variant's own doc comment (or, for the ones with no variant, the "leave"
  reasoning is on the RE page cited above).

## Next Steps

1. Read the Rocket/Cannon pool-reap teardown call itself (not yet located)
   to settle whether the original's timeout plays a third cue or is
   genuinely silent.
