# Race rules specification

How a race starts, how laps are counted, what each mode ends on, and how a
finish is decided. Pulse (PSP) is the reference. The material is thinner than
the physics and weapons pages in one respect: most per-title rules are
**unmeasured and inherited**, and the table at the end says so.

Confidences are 0-100 ([rubric](../reverse-engineering/confidence-rubric.md)).
No score means chosen, not measured. Tuning values authored on the disc
(Zone's start speed, increment and recharge) are not reproduced. A few
campaign and race-box figures the evidence pages already quote (kill targets,
the Head2Head medal) are restated.

## Timing

The simulation runs a fixed 60 Hz. The original runs a variable `dt`; lap times
it records are not frame counts, because it accumulates `dt` and interpolates
the crossing inside the tick. Compare lap counters, not tick counts.

## Start

1. **Flyby lock.** The front end shows the circuit's camera animation. Nothing
   can end it before a 60-tick counter reaches zero; thrust held through it ends
   it on that tick (measured, 3 runs).
2. **Countdown: 272 ticks** from the end of that lock, summing to 4.5377 s of
   `dt` (identical to five figures on three runs of one binary, 90).
3. **Release.** On tick 272 the throttle state steps from `0` to the held value
   with no ramp; the lap clock and the race clock leave `0.0` and take their
   first `dt`; the gantry shows `GO` on the same tick (90 for the clocks, 85 for
   `GO`). Every time the game shows or scores excludes the countdown.
4. Holding thrust through the countdown is **not** a false start (measured: the
   stun and slowdown timers stay `0` for the whole capture).

The 272-tick figure is Pulse Time Trial's own. It is not measured on Single
Race with a full grid, Zone, Pure, HD or 2048. Zone's clocks also hold through
the countdown, read statically only (65).

## Laps

**The ring.** A circuit is a ring of path segments followed through each
junction's primary exit; alternate paths are shortcuts selected by an
"excluded path" argument and are off the ring (88). Progress is distance along
that ring.

**Where the lap begins** (88). Once at race-manager construction:

```text
1. locate the authored "Start Position" on the spline
2. step 154.0 units along that sample's tangent, in a straight line
3. locate that point on the spline; its arc position is the line
```

The 154 is a literal and applies to every circuit. The original spawns the
craft about 16.5 units behind the line on the circuit it was measured on and
lets the first crossing start the race.

**How a crossing counts** (the original, 92, read statically; not watched live
with a reversed craft). Per craft per tick:

- Progress is the authored normalised arc position `t` times a load-time length,
  **unwrapped**: each tick picks the wrap (`0`, `+L`, `-L`) that moves progress
  least.
- A crossing count `wraps + (line < arc)` rises at each forward crossing and
  falls at each reverse crossing. A lap completes when the count rises by
  exactly one and equals the count the next lap is expected on.
- The first forward crossing sets "started" and records no lap; the HUD lap is
  `next_expected - 1`, so the grid reads "Lap 1".
- **Reversing over the line never lowers the lap**: the target is only ever
  raised. Reversing across and re-crossing earns nothing.
- On a lap the time is stored in centiseconds in a 20-entry array, compared with
  the best, checked against records, flagged perfect if no wall was hit that lap.

**Laps per class** (90; a flat census over 236 campaign cells plus a live read
at every rung for Time Trial):

| Speed class | Venom | Flash | Rapier | Phantom |
| --- | --- | --- | --- | --- |
| Laps (Time Trial, Single Race, Tournament leg) | 3 | 4 | 4 | 5 |

The count is configuration in the original (`laps="%d"` in the race setup).
Pure's ladder has a fifth class below Venom and 2048's a fifth above Phantom;
their lap counts are not measured.

## Modes

| | Time Trial | Speed Lap | Zone | Single Race | Eliminator | Tournament | Head2Head |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Opponents | none | none | none | full grid | full grid | full grid | one |
| Weapons | off | off | off | on | on, second table | on | off, locked |
| Damage | recovers | recovers | on | on | on | on | unmeasured |
| Ends | class laps | never | craft destroyed | class laps | kill target | after the last leg | class laps |
| Free turbo per lap | yes | yes | no | no | no | no | no |
| HUD layout | TimeTrial | TimeTrial | Zone | Arcade | Elimination | Arcade | n/a |

Opponents and weapons are consequences of the mode, not options: the original's
Custom Race screen sets WEAPONS to OFF and AI to N/A when Time Trial, Speed Lap
or Zone is chosen (measured on screen for all seven race types). The disc's text
says Time Trial and Speed Lap recover ship energy automatically (so shield never
ends them); a free turbo per lap is the disc's text corroborated by a HUD layout
authoring a turbo icon and no other (85).

### Time Trial and Speed Lap

Time Trial ends after the class's laps. A run past the last lap starts a fresh
attempt in the original; this engine stops instead. Speed Lap reads `Lap 1 of 7`
and never ends on its own: its pause menu has a seventh row, `END SESSION`,
that no ending mode has (75; the lap-8 crossing was never watched).

### Zone

(84 identification, from `Zone_Update`.)

- The zone number steps every **10.0 s** of accumulated `dt`, on nothing else
  (84). The accumulator resets to zero rather than subtracting, so each zone
  discards the overshoot (82).
- **Speed law.** Thrust is `start + increment * zone`, replacing the throttle,
  uncapped (88). `start` and `increment` are authored (`<Global><Zone>`). The
  speed law runs only while grounded and not on the grid. The step selects no
  speed class: the names above Phantom are labels on one scale.
- **Score:** +1 per tick, +500 for a zone finished without touching anything
  (which also restores shield by authored `recharge`), +100 on entering a new speed pad
  (85 for the three numbers).
- **Ending:** the run ends when the craft is destroyed (80; the disc's
  `"Ship destroyed on zone"` and `"survive for as long as possible"` strings).
  A campaign Zone event adds a target zone count per cell.
- Zone flies its own environment and a four-corner hover (see
  [physics](physics.md#hover-and-track-following)).

### Single Race

A destroyed craft takes "Ship destroyed" as its result in place of a place and
does not come back (75: the results screen's field list excludes the alternatives
`Racing`, `Ship destroyed`, `1st`..`8th`). The race ends for the player on their
own last crossing; an opponent finishing first does not end it.

### Eliminator

- Race for kills, not position. No lap target. The race ends the moment **any**
  craft's kill count reaches the target (80).
- **Target:** a campaign cell's gold figure when one is in play (10, 7 or 5 on
  the disc); otherwise the race box's `KILLS` row, whose first entry is **5**
  (90; widget identification 78). The list's authored default of 10 is not
  honoured by the running screen.
- **Weapons:** a second table (`WeaponStats_Elimination.xml`), so weapons "do
  more damage" is data (90). Pads refresh in `elimination_refresh_time`.
- **No absorb energy.** A press spends the held weapon on a one-second Shield
  (80).
- **Health:** each completed lap adds 20 % of the pool maximum to the player
  (`Ship_RefillLapShield`; score on the shield page, see Evidence).
- **Death:** a destroyed craft waits a 1.5 s dwell after its half-second
  explosion, then a respawn wait (1.0 s for the player, 0.8 s for others),
  and is counted as a death (the opponent wait measured live on PSP; respawn
  site 40).
- A wrong-way craft has its airbrakes forced to 100 and its engine killed
  (Pulse only; measured absent on Pure and HD; unscored in the evidence).
- **Kill credit** is chosen: the last craft to land a direct weapon hit, cleared
  by a wall hit or another cause. The original's attribution was not traced.

### Tournament

A series of Single Race legs (up to twelve; four on the PSP disc) scored as one
event.

- A leg races exactly like a Single Race.
- **Points by finishing place: 8, 6, 5, 4, 3, 2, 1, 0** for 1st to 8th (88,
  `g_tournament_points_by_position`). A craft that did not finish scores 0 only
  if destroyed; a craft still racing when the leg ends scores its running rank,
  which the original writes every tick (90). No fastest-lap or kill bonus.
- Totals carry across legs. The final standing rank by total points, not the raw
  total and not the last leg's place, feeds the campaign medal threshold.
- HD authors no tournament standings table; the other titles inherit this table
  and nothing measured says otherwise.

### Head2Head

- The field is the player plus **one** AI opponent (measured: `AICount="1"` on
  all 23 cells and the per-mode grid-size table).
- Weapons are locked off (all 23 cells say off).
- Laps follow the per-class table; the medal is win or nothing (gold target 1,
  silver and bronze 0).
- The two craft start on the **two front grid places**: the AI on place 1 and the
  player on place 2 (PSP, one person); two people on PS2 take places 1 and 2 by
  slot. A field of two does not pack to the back.

## Finish

- **The flag is the player's own last crossing** of the class's lap count. An
  opponent crossing first records its finish time and stops being ordered by
  distance; it does not end the race.
- **Order.** The original ranks every craft each tick (`Race_UpdatePositions`,
  90). This engine orders finishers by finish tick, then racers by distance, then
  by slot.
- **After the line.** The original keeps the race running behind the end-of-race
  panels with the player's craft flown by the opponent driver. A wreck, an
  Eliminator target and a Zone ending stop the world.
- **The results board** is a snapshot fixed at the flag and never revised.
- Lap times and the total exclude the countdown; the clock starts at the release
  tick.

## Per-title differences

| Rule | Pulse | Pure | HD / Fury | 2048 | Omega |
| --- | --- | --- | --- | --- | --- |
| Countdown 272 ticks | M | unmeasured, I:Pulse | unmeasured, I:Pulse | unmeasured, I:Pulse | unmeasured, I:Pulse |
| Lap-line rule | M (88) | I:Pulse | I:Pulse | I:Pulse | I:Pulse |
| Laps 3, 4, 4, 5 | M | five-rung ladder, laps unmeasured | I:Pulse | five-rung ladder, laps unmeasured | unread |
| Tournament points | M | I:Pulse | no table authored, I:Pulse | I:Pulse | I:Pulse |
| Eliminator wrong-way kill | runs | absent, M | absent, M | I:HD | I:HD |
| Zone speed ladder | 1 scale, names | no colour grade, speed alone | 14 thresholds, M 86 | 17 thresholds, 78 | unread |

## Where this engine differs from the original

Chosen, not measured: lap 1 is timed from the first line crossing, not the
release, because our spawn sits on the grid slot and is further behind the line
than the original's (the original times lap 1 from the release); a lap is a
wrap of ring distance gated by passing the near and then the far half of the
circuit since the last crossing (blocks rocking over the line); whole-tick lap
times, no sub-tick interpolation; the split table, perfect-lap flag and
twenty-lap record are not modelled; Eliminator refills every craft per lap
where the original reads the player's slot alone; progress on the far side of a
fork is read from a route at the same ring fraction; the unfinished-opponent
case of a wrecked opponent at the exact tick a leg ends is not zeroed.

## Open

- Where an Eliminator respawn happens is not found in the executable (40).
- Speed Lap's end at lap 8 and a Time Trial's fresh-attempt rule are unwatched.
- The 272-tick countdown on a full-grid race and on every other title.
- The per-title Tournament points beyond Pulse.

## Evidence

- Pages: [lap counting](../gameplay/lap-counting.md),
  [race modes](../gameplay/race-modes.md),
  [after the finish](../gameplay/after-the-finish.md),
  [race intro](../gameplay/race-intro.md),
  [leaving the track](../gameplay/leaving-the-track.md)
- Ghidra: [race progress](../ghidra/functions/psp-pulse-usa/race-progress.md),
  [zone mode](../ghidra/functions/psp-pulse-usa/zone-mode.md),
  [zone start](../ghidra/functions/psp-pulse-usa/zone-start.md),
  [tournament](../ghidra/functions/psp-pulse-usa/tournament.md),
  [head2head](../ghidra/functions/psp-pulse-usa/head2head.md),
  [eliminator kill target](../ghidra/functions/psp-pulse-usa/eliminator-kill-target.md),
  [race campaign](../ghidra/functions/psp-pulse-usa/race-campaign.md),
  [shield](../ghidra/functions/psp-pulse-usa/shield.md)
- Code: `crates/race/src` (`course`, `state`, `mode`, `zone`, `tournament`,
  `standing`)
