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

**Half the Rocket/Cannon timeout question settled cheaply, from the port's
own code rather than new RE work; the other half is still open, below.**
`crates/gameplay/src/projectile/flight.rs`'s timeout-reap branches for both
weapons reset the slot without ever writing an `Impact`, so
`crate::race::tick`'s impacts loop never sees a `struck: None` from a
timeout - only from a real wall hit, which is what made wiring
`RocketHitWall`/`CannonHitWall` safe with no new disambiguation. **What the
*original* plays on that same reap is not settled** - see Open.

**One correction the plan's own bank read got wrong**: `CANNONEXPLSHIP` is
not an empty cue. It owns exactly one command, opcode `0x05`, whose child
record indexes `CANNONEXPLWALL` directly - read straight off
`Bank::cue_children`/`Bank::resolve_child` against `pulse-psp-usa.chd`'s own
weapon bank, not inferred from a waveform count. So a Cannon's craft-hit
ending plays the same nine waveforms its wall-hit ending does, by the disc's
own construction - see `cannonexplship_is_a_child_reference_to_cannonexplwall`
in `crates/game/tests/sfx_weapon_ground_truth.rs`.

## Open

- **What the original plays when a Rocket or a Cannon round times out
  without hitting anything.** `rocket-visuals.md`'s "What a rocket hit
  spends" section reads the teardown as branching on the round's own
  `0x10`/`0x20` flags, and a reap sets neither - so the branch this project
  has read plays neither `ROCKEXPLWALL` nor `ROCKEXPLSHIP` on that path, and
  this port plays nothing there either. Whether the original's own teardown
  call has a third arm, or genuinely plays nothing, is unread.
- **`LEACHENERGY`.** Needs the LeachBeam ribbon's own scroll-cursor wrap.
  The `leachbeam-gfx` lane's 2026-09-23 ribbon re-read added exactly that
  edge - `oag_render::beam::Ribbon::advance` now returns a `bool` for it -
  but it is read and discarded inside
  `crates/game/src/race/weapons/visuals.rs`'s own `advance_leach_beam_ribbon`,
  a file this lane does not own. Surfacing it (store the return on
  `RaceView`, or have that function return it) is a small change for
  whoever owns that file next; once it exists, wiring `Cue::LeachEnergy` off
  it is what the audio side would do.
- **`missile.md` (around lines 174-176) names `Ship_FireHeldWeapon`
  (`0x08844ae8`) as the opener of `ROCKET`, `QUAKELAUNCH` and
  `_AUTOPILOT`/`autopilot_eng`.** `autopilot.md` and `Cue::Disengaging` say
  the Autopilot's opener is unlocated. One of the two is wrong; if
  `missile.md` is right, both the Autopilot's audio and `QUAKELAUNCH` gain a
  real call site. Not chased this pass - reading `Ship_FireHeldWeapon` end to
  end is the next step.
- `MISSILEEXPSHIP`, `QUAKELAUNCH`, `~QUAKETRAVEL`, `LEACHFAIL` and the
  Shuriken's own launch cue still have no recovered trigger and stay
  unwired - see each `Cue` variant's own doc comment (or, for the ones with
  no variant, the "leave" reasoning is on the RE page cited above).

## Next Steps

1. Read `Ship_FireHeldWeapon` (`0x08844ae8`) end to end to resolve the
   Autopilot/`QUAKELAUNCH` conflict above.
2. Once `leachbeam-gfx`'s own ribbon-cursor edge is surfaced outside
   `weapons/visuals.rs`, wire `LEACHENERGY` off it.

## From the HANDOVER.md index (moved 2026-09-25)

2026-09-23; landed same day - `Cue::ALL` grew 14 to 28, every "wire" row pushed; still open: what the original plays on a Rocket/Cannon timeout reap, `LEACHENERGY`, and a `Ship_FireHeldWeapon` read-through
