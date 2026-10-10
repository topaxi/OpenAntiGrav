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

## 2048: Normal and Extreme (confidence 65-72, static)

2048's three settings are Off, **Normal (its default)** and Extreme ("Super"). Extreme is HD's
law above, read from 2048's own global `<PilotAssist>` rungs, which are softer than HD's.
Normal is the same law run on a second object, read from the ship's own `<Assist>` block,
which is far gentler (a torque cap of 50 against 600-1000). Its strength ramps in with speed
between `SteerAssist.min_speed` and `min_speed + ramp_up_range`. The two never run together.
Evidence: [vita-2048-eu-v104/pilot-assist.md](../ghidra/functions/vita-2048-eu-v104/pilot-assist.md).

## The HUD indicator

Both HD and 2048 author four widgets: a background shown while the assist is enabled, a main
icon shown for 3 s after it acts, and a left and a right arrow that blink at 4 Hz for 1.5 s
on the side of the wall it steered away from.

## Other titles

- **Omega: ported.** Its executable carries 2048's whole set of names, and its own
  `Data/xml/handlingstats.xml` authors `<PilotAssist>` with 2048's numbers on all four rungs
  (`pilot_assist_ground_truth::which_titles_author_pilot_assist`), so the generic race load
  hands Omega's own table to the same law. Its HUD indicator is not wired
  (`oag_omega::hud`'s `assist: None`).
- **Pulse, Pure**: no Pilot Assist in either executable and no table on either disc. They
  get none unless the maintainer rules otherwise: no recovered law means no assist.

## In this engine

- **The law**: `oag_physics::pilot_assist`, called from `forces::evaluate` once the hover has
  set this tick's contact, with the throttle scaled straight after `controls::update` the way
  `Craft_UpdateThrottle` scales it. Its state is `ShipState::pilot_assist` (HD's `craft+0x380`),
  hashed only once it leaves its default, so a race with the option off steps and hashes
  exactly as it did before the assist existed.
- **The numbers**: `oag_tables::handling::Global::pilot_assist`, per class, from the title's
  own `<GlobalClass>` rungs; `oag_raceplay`'s global loader reports whether the class has one.
- **The corridor**: the race's spline table (`oag_raceplay::pilot_assist`). The located record
  is the nearest sample; the fork sibling is the nearest sample on another path within 20
  units of it, **chosen, not measured** (the original takes it off its junction graph).
- **Who gets it**: the player's craft only. Opponents never do (maintainer: the AI flies the
  player's physics), pinned by `pilot_assist_ground_truth::no_opponent_ever_gets_the_assist`.
  Off in Zone, as in the original, and on the grid.
- **The switch**: `[controls] pilot_assist`, the CONTROLS page's PILOT ASSIST row, applied to
  a running race at once. **On by default on Android only** (maintainer, 2026-10-10: touch
  controls are much harder than a pad; a desktop player is assumed to know Wipeout). A
  headless capture or trace takes `--pilot-assist on|off` only, default off.
- **The HUD**: `oag_hud::assist` draws HD's four `AssistIndicator*` widgets by the original's
  rule; 2048's `PilotAssist` icon is up while the switch is on.

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

The regression gate (`race_ground_truth::a_lone_craft_gets_round_the_circuits_it_is_known_to_get_round`)
reads the same before and after: twelve clean laps, no respawns.
