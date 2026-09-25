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

- `MISSILEEXPSHIP`, `~QUAKETRAVEL`, `LEACHFAIL` and the Shuriken's own launch
  cue still have no recovered trigger and stay unwired - see each `Cue`
  variant's own doc comment (or, for the ones with no variant, the "leave"
  reasoning is on the RE page cited above).
- **Gameplay finding, not an audio-cue gap, left for whoever owns
  `crates/gameplay/src/projectile/flight.rs`**: `CannonPool_Update` reaps a
  Cannon round at `1.0 < age` (its own timer, distinct from the Rocket's
  `5.0 s`), but `flight.rs` has no `Weapon::Cannon`-specific branch the way
  it does for the Rocket (`rocket::LIFETIME_SECONDS`) - a Cannon round falls
  through to the generic `MAX_FLIGHT_SECONDS` (`10.0 s`) cap instead. The
  cue answer is unaffected either way (both ages are silent reaps), but the
  round itself likely lives roughly 9 seconds longer in this port than the
  original before it despawns.

## Next Steps

1. Recover a trigger for `MISSILEEXPSHIP`, `~QUAKETRAVEL`, `LEACHFAIL` or the
   Shuriken's launch cue, or record why each stays untriggered on the disc.
