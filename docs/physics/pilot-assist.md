# Pilot Assist

HD/Fury and 2048 offer Pilot Assist, a guidance system for new players: a spring on the
track's own spline that yaws the craft back from a wall it is about to meet, paid for with a
few percent of thrust. This page is the behaviour; the instruction-level evidence for HD is
[pilot-assist.md](../ghidra/functions/ps3-hdfury-eu/pilot-assist.md).

## HD/Fury's law (confidence 85, read live on RPCS3)

Each tick, for a local player's craft with the save option on, racing (not on the grid), in
any mode but Zone and Detonator:

1. **Look ahead** `la = min(laDistConst + speed * laDistVelMul, laDistMax)` along the craft's
   forward axis.
2. **Probe the corridor** at that point (radius `2`) and at the craft (radius `5`): locate the
   nearest track-spline record, take the lateral offset `d` across it, and measure how far
   inside `radius` of either half-width edge the point is: `L = max(0, r - left - d)`,
   `R = min(0, right - r - d)`. Facing more than ~127 degrees back down the track reads as no
   push.
3. **Blend** toward `1` while any hull point is grounded and the craft is upright on the track
   (`dot(down, up) <= -0.5`), toward `0` otherwise: up at `2` per second, down at `20`.
4. **Yaw** `clamp((L + R)_ahead * torqueMul * blend, +/-maxTorque)` as a local torque, zero while
   the craft already spins faster than `maxAngVel`; **push** sideways along the craft's own right
   axis by `(L + R)_here * springMul * blend`.
5. **Charge for it**: a yaw over `25` starts a `penaltyDuration` timer, and the throttle is
   scaled by `thrustPercentOnUse` while it runs, `generalThrustPercentWhenEnabled` otherwise.

It never reads the steering input. All numbers are per speed class from
`/data/xml/handlingstats.xml`'s `<GlobalClass>` rungs.

The penalty timer decays even while the assist is off, and the throttle keeps the
`thrustPercentOnUse` scale until it reaches zero; switching it off mid-race does not refund
the last three seconds.

**Measured on RPCS3** (the table is on the evidence page): aimed at the right-hand wall at
100 units/s and let go, the craft hits it and drops to 16 units/s with the option off, and
is yawed back clear of it and holds 100-120 units/s with it on.

## The three levels, on every title

2048 offers Off, **Normal (its default)** and Extreme ("Super" in its own strings), and since
2026-10-10 (maintainer) every title offers the same three. Both levels run the one law above:

- **Extreme** is HD's law on a global table: `<GlobalClass><PilotAssist/>` plus
  `<PilotAssistPenalty/>`, at strength `1`.
- **Normal** is the same law on the ship's own `<Class><Assist/></Class>` block, which is far
  gentler (a torque cap of 50 against 600-1000). Its blend target is not `1` but a strength
  that ramps in with speed: `k * notInUseStrength`, `k = clamp((speed - min_speed) /
  ramp_up_range, 0, 1)` from the global `<SteerAssist min_speed="50" ramp_up_range="25"/>`;
  at or below `0.001` it does not run at all. It authors no general thrust percentage, so the
  throttle is untouched (100) until a correction starts the `0.25 s` penalty at 95. The two
  never run together. Evidence (static, confidence 65-72):
  [vita-2048-eu-v104/pilot-assist.md](../ghidra/functions/vita-2048-eu-v104/pilot-assist.md).

**Which is authored and which is chosen** (the loader says which in its report, one line per
level, and `oag_raceplay::load::global::pilot_assist` is the one place the rule lives: play the
disc's numbers where it authors them, a labelled stand-in only where it authors nothing):

| Title | Extreme | Normal |
| --- | --- | --- |
| 2048 | authored (global `<PilotAssist>`) | authored (ship `<Assist>`, global `<SteerAssist>`); the twelve HD-derived guest teams author no `<Assist>`, so they run the native block, chosen |
| Omega | authored (2048's numbers on all four rungs) | authored (its 20 native craft, as 2048) |
| HD/Fury | **authored** (the law recovered live on RPCS3, confidence 85) | **chosen**, 2048's block and ramp |
| Pulse | **chosen**, 2048's Extreme rung per class | **chosen**, 2048's block and ramp |
| Pure | **chosen**, 2048's Extreme rung per class, `VECTOR` too | **chosen**, 2048's block and ramp |

**Chosen, not measured**, with no confidence score: neither Pulse nor Pure carries assist code
or a table (no `PilotAssist`, `laDist` or `springMul` string in either executable), and HD
authors no ship `<Assist>`. The chosen numbers are 2048's own, so they are re-derivable:
`pilot_assist_levels_ground_truth::every_native_craft_authors_the_same_assist` reads all 100
class blocks of 2048's 20 native craft and all 100 of Omega's and finds **one** block, the
per-ship values (`laDistConst 10, laDistVelMul 0.4, laDistMax 75, springMul -12, torqueMul 10,
maxTorque 50, maxAngVel 2, thrustPercentOnUse 95, penaltyDuration 0.25, notInUseStrength 0.1`),
so "which ship" has one answer: all of them. `titles_without_a_level_run_2048s_and_labelled_so`
pins every chosen set to 2048's, per class.

**Speed units need no scaling.** A Venom held on thrust peaks at 123, 116, 124, 117 and 125
units/s on Pulse, Pure, HD, 2048 and Omega (Phantom 201, 175, 204, 211, 211), so 2048's
`min_speed 50` and `ramp_up_range 25` mean the same speed on all five: factor `1.0`.

**Not recovered for Normal**, stated rather than guessed at: the multiplier the original reads
at `(*(craft+0x8c))+0x10` is taken to be `notInUseStrength` (the only candidate, not tied to
the attribute in the code), and which speed `craft+0x5a8` holds is taken to be the craft's
speed. Neither moves under the assist-off gate.

## The HUD indicator

Drawn at **Extreme only**: 2048 ties its background and icon to the Extreme byte, and Normal
runs on a state object the indicator never reads. Pulse and Pure draw none yet: their art waits on
the asset-layer lane (the original has none to draw). Both HD and 2048 author four widgets: a background shown while the assist is enabled, a main
icon shown for 3 s after it acts, and a left and a right arrow that blink at 4 Hz for 1.5 s
on the side of the wall it steered away from.

## Other titles

- **Omega: ported.** Its executable carries 2048's whole set of names, and its own
  `Data/xml/handlingstats.xml` authors `<PilotAssist>` with 2048's numbers on all four rungs
  (`pilot_assist_ground_truth::which_titles_author_pilot_assist`), so the generic race load
  hands Omega's own table to the same law. Its HUD indicator is not wired
  (`oag_omega::hud`'s `assist: None`).
- **Pulse, Pure**: no Pilot Assist in either executable and no table on either disc; they run
  the chosen sets in the table above.

## In this engine

- **The law**: `oag_physics::pilot_assist`, called from `forces::evaluate` once the hover has
  set this tick's contact, with the throttle scaled straight after `controls::update` the way
  `Craft_UpdateThrottle` scales it. Its state is `ShipState::pilot_assist` (HD's `craft+0x380`),
  hashed only once it leaves its default, so a race with the option off steps and hashes
  exactly as it did before the assist existed.
- **The numbers**: `oag_tables::handling::Global::pilot_assist` and `steer_assist` and
  `Class::assist`, from the title's own files; `oag_raceplay::load::global::pilot_assist` picks
  the law per level and reports the source (authored, or chosen and from what).
- **The corridor**: the race's spline table (`oag_raceplay::pilot_assist`). The located record
  is the nearest sample; the fork sibling is the nearest sample on another path within 20
  units of it, **chosen, not measured** (the original takes it off its junction graph).
- **Who gets it**: the player's craft only. Opponents never do (maintainer: the AI flies the
  player's physics), pinned by `pilot_assist_ground_truth::no_opponent_ever_gets_the_assist`.
  Off in Zone, as in the original, and on the grid.
- **The switch**: `[controls] pilot_assist = "off" | "normal" | "extreme"`
  (`oag_physics::pilot_assist::Level`), the CONTROLS page's PILOT ASSIST row (mouse and touch
  as the other choice rows), applied from the next race, as the control scheme is: a mid-race
  switch is not in the input stream a replay records. A recording's header carries
  `pilot_assist = "normal"|"extreme"` when it was on (`oag_game::ghosts::race_options`); the
  older `"true"` read as Extreme. **Normal on Android, off everywhere else** (maintainer,
  2026-10-10: touch controls are much harder than a pad; a desktop player is assumed to know
  Wipeout). A file from when this was a switch holds `true`/`false`, read as Extreme/Off so a
  saved choice keeps what it ran. A headless capture or trace takes `--pilot-assist
  off|normal|extreme` (`on` still means Extreme), default off. The row's values are the
  tokens: the menu has no per-title value strings, so 2048's own "Super" shows only in its
  `OptionsPilot` list.
- **2048's own list**: `OptionsPilot` (`Frontend::pilot_choice`) sets the level at Launch
  Game: `FE_OFF`, `FE_NORMAL`, `FE_SUPER` are Off, Normal, Extreme.
- **The HUD**: `oag_hud::assist` draws HD's four `AssistIndicator*` widgets by the original's
  rule; 2048's `PilotAssist` icon is up while Extreme is. Normal and the Pulse and Pure
  titles draw no indicator.

## Measured in this engine

HD, Talon's Junction, time trial, Venom, a new player who holds thrust from the start and
never touches the stick, 1800 ticks (`pilot_assist_ground_truth::a_new_player_meets_fewer_walls_with_pilot_assist_on`):

| Assist | Wall contacts | Ticks touching a wall | Distance round the lap |
| --- | ---: | ---: | ---: |
| off | 16 | 1297 | 706 |
| on | 0 | 0 | 2857 |

The same run as a player sees it: `oag-game --race --mode time_trial --class venom --hold
cross --pilot-assist off|on --ticks 1000 --screenshot <png>` against the HD image shows the
unassisted craft scraping the left-hand wall at 79 km/h and the assisted one mid-track at
441 km/h, with the indicator's arrow blinking on the side of the wall it was turned from.

Pilot Assist at all three levels, the same run, 1800 ticks, Venom
(`pilot_assist_levels_ground_truth::hd_levels_off_normal_extreme` and `pulse_levels_off_normal_extreme`):

| Title | Level | Wall contacts | Ticks touching a wall | Distance round the lap |
| --- | --- | ---: | ---: | ---: |
| HD | off | 16 | 1297 | 706 |
| HD | normal | 13 | 1293 | 717 |
| HD | extreme | 0 | 0 | 2857 |
| Pulse | off | 4 | 1333 | 722 |
| Pulse | normal | 5 | 1332 | 723 |
| Pulse | extreme | 2 | 4 | 2864 |

Normal does not sit clearly between Off and Extreme: it is within noise of Off (Pulse's 5 contacts
against Off's 4 is not between them; its distance is). It is gentle by design (a torque cap
of 50, blend target `0.1` at full speed), and a craft held on thrust with the stick never
touched is the hardest case for it. Making `notInUseStrength` `1.0` instead (a trial, not
kept) moved HD to 17 contacts / 1285 ticks / 725 and Pulse to 6 / 1139 / 1021, so the small
effect is the `<Assist>` numbers, not that multiplier. Nothing was tuned.

The regression gate (`race_ground_truth::a_lone_craft_gets_round_the_circuits_it_is_known_to_get_round`)
reads the same before and after: twelve clean laps, no respawns.
