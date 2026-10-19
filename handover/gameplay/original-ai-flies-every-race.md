# The original AI flies every race with opponents: what is left

2026-10-13, lane `original-ai-default`. Maintainer's decision: until
`oag_ai::Driver` is solid, every race with opponents flies the recovered
original driver, with `oag-game --ai ours` as the developer switch back. Built
in `oag_game::ai_choice`; the rules, the chosen level mapping and the log lines
are in [gameplay/ai.md](../../docs/gameplay/ai.md) ("Which AI flies").

## Open

- **A lagging human makes the leader fall on `13_Track`: the original's does too
  (measured 2026-10-13, PPSSPP, two boots; docs/gameplay/ai.md, "The original's
  own field falls ...").** Falls at the authored jump in every capture. Speed at
  spline row 19: falls 66 to 103.6, clears 101.8 and up; the leaders' held thrust
  is 47.1 and their speed 89.6 to 103.6 when they fall (16 of 39 entries at 47.x),
  this engine's 91 to 100. What clears is the speed pad just before the jump (pad
  6, 45 units ahead of it, lateral -1.9 to -9.8): at held thrust 47, 20 of 20
  on-pad entries cleared, 16 of 19 off-pad fell. The original's fall loops end
  (13 in a row at skill level 0, at most 2 at level 2 before the finish); this
  engine's slot 3 looped 129 times. **Hypothesis only:** our leader sits on the
  pad's off side. Not measured here: our signed lateral at the pad and whether
  our pad test boosts an AI craft there. `05_Track` (4 resets parked, 1 with the
  default-skill autopilot, a reset volume on lap 4) was not captured on the original.
- **A second reproduction of an original-driver jump fall: Pure, Chenghou
  Project** (2026-10-19, lane `campaign-ai-weapons`). Single race, VENOM, seed
  1, player parked, `use_original_ai(None, 1)`. Slot 2 climbs the ramp at line
  index ~1694-1719 on lap 3 at speed 169-191 (held thrust 44), goes airborne
  around tick 8031 and never lands: it falls (speed up to 324), strikes at tick
  8098 (shield 63 to 55) and is rescued at tick 8139 after 1.75 s airborne.
  Nothing hit it in the 3,100 ticks before. It surfaced once opponents started
  firing (`docs/gameplay/ai.md`, "An original opponent never fired, and now
  does"), which changed the field state; `pure_original_ai_ground_truth`'s
  Chenghou walk now allows 1 respawn a craft and names this cause.
- ~~**The absorb half of `WeaponAi_DecideFireOrAbsorb`**, reverted~~ **Re-landed
  2026-10-19 (lane `ai-absorb-turbo`)** on top of a fixed Turbo gate, its
  cadence read (every world step, 85). See `docs/gameplay/ai.md`, "The absorb
  half, re-landed" and "The Turbo gate reads the plan". Still open from it:
  - **More lip stops on de Konstruct Black with absorb**: forward lip stops
    per million craft-ticks 42.8 to 51.5 (seeds 4-23). About half follow a
    barrel roll, which absorbed energy makes more frequent (1,380 to 1,706
    rolls). Next: `Driver::wants_to_roll` before the first jump (samples
    ~190-207 of `05_Track`); the other half is not attributed.
  - **The Turbo gate's 220 is Pulse physics only**; HD, 2048, Omega and Pure
    run it unmeasured.
  - **Whether the original driver compensates for a Turbo** (its thrust law
    during a boost) is unread; an original-driven opponent runs this
    project's Turbo gate too.
- **A forked circuit still learns the speed plans** when the original flies:
  the route plans gate which routes the coin offers
  (`Race::steer_branching`) and are learned on the ring plan's reference hull.
  Skipping them for the original field means deciding what offers a route
  there (every route, or a rule of the original's own), which changes how an
  original opponent goes round a fork, so it was not done. Everything else
  skips them: `Race::start` no longer learns them, `Race::ensure_speed_plans` does.
- **Races with more than one human keep this project's driver**
  (`ai_choice::Reason::MultipleHumans`): the original field's slot, place and
  rubber-band handling with several humans is unread.
- **Classes without an original table keep opponents off** (RACE REMIX on Super
  Phantom): `ai.md`'s class coverage is unchanged by this lane.
- Ghosts recorded before this change raced opponents flown by our driver; they
  still play as poses.

- **The autopilot flies the original's player driver (2026-10-19, lane
  `autopilot-original-ai`; docs/gameplay/ai.md, "The autopilot flies the
  original's player driver").** Left open there:
  - **Maintainer question:** an opponent on this project's driver "switching to
    the original autopilot" while holding the pickup has no original behaviour
    behind it: the original never fires the Autopilot outside an Eliminator
    (fire index `0`, `g_weapon_ai_rate[0] = 0`). Wanted anyway, as chosen?
  - In an Eliminator an opponent can fire it (`0.012` a decision); what
    `Autopilot_Fire`'s running bit does to a craft its driver already flies is
    unread.
  - 2048 and Omega steer the player's driver with the pilot assist's gains
    (airbrake `1`/`0`, a `1.73 |x|` lookahead floor, a `libm` call); not read
    whole, the opponents' controller flies it.
  - What sets `craft+0x1d4` (the pad-to-driver blend) is unread; the handover is
    one tick here.
  - HD scales the player's `100` by `SplitScreenMultiplier` with more than one
    person; not applied.
  - Pure's player lane `0` is inherited from Pulse (`0x08919b6c` unread for it).

## Next Steps

1. Compare this engine with the capture: log the signed lateral (the `lat`
   axis of `oag-trace track`) of the original-driver leader at spline row 3073
   (pad 6) and each zone entry's speed, parked player, VENOM, seed 1. If it
   sits at +1.4 to +2.6 (off the pad, where the original's crafts at +2.6 are
   never boosted), the difference is the lane (wander/spread) or the pad test
   for an AI craft, not the formation law or the jump; fix that, then re-count
   respawns against the original's (13 in a row at most, 2 at level 2). Only if
   it still loops, decide with the maintainer on a floor this project chooses
   (labelled chosen, not measured).
2. Decide what offers a route to the original field without the plan gate, then
   skip the plans on forked circuits too.
3. ~~Add a round-the-lap ground truth over every Pulse circuit for the original
   field with an `--autopilot` player.~~ Done 2026-10-19 with weapons off
   (`autopilot_original_ground_truth::the_original_autopilot_finishes_every_pulse_circuit_without_a_rescue`,
   12 of 12, no rescue); the autopilot now flies the original's player driver.
   With weapons on, `02_Track` wrecks the player on lap 3 (docs/gameplay/ai.md,
   "The autopilot flies the original's player driver").
4. Read the original field with several humans (slot, place, rubber band), then
   lift `Reason::MultipleHumans`.
5. De Konstruct Black's extra lip stops under absorb: count `wants_to_roll`
   rolls taken between samples ~190 and 207 of `05_Track` over seeds 4-23,
   then decide (with the maintainer) whether this project's roll policy should
   hold off before a gap. The original's own roll use by opponents is unread.
