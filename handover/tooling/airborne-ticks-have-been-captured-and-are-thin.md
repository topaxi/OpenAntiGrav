# Airborne ticks *have* been captured - 65 of them, and they are thin in a specific way

**Corrected 2026-09-07.** The title used to be "Nothing airborne has ever been
captured", and that is false: two committed captures on `16_Track` contain
airborne ticks, and one of them was already load-bearing for a different finding
before anybody noticed.

| capture | rows | non-grounded | of which `grounded == 0` | events |
| --- | ---: | ---: | ---: | --- |
| `data/traces/talons-junction-clean-lap.csv` | 2,977 | 31 | 8 | 432-439 (8), 1571-1579 (9), 1595-1608 (14) |
| `data/traces/talons-junction-autopilot.csv` | 3,020 | 34 | 6 | 432-445 (14), 1588-1594 (7), 1611-1623 (13) |

Both are the same three features of Talon's Junction at slightly different
phases. The longest event is 14 ticks (0.23 s) and the longest run with **both**
probes off is 8 ticks (0.13 s). `data/traces/head-pad*.csv` read `grounded 0` on
every one of their 110 rows, which is a constant column and not airborne data -
do not count them.

**These ticks already paid for themselves once**:
`crates/trace/tests/hover_contact_ground_truth.rs` uses them to show our hover
contact test reproduces the recording's whole `grounded` column, 2,976 of 2,976
ticks, `0.5` runs included - see
[`docs/physics/README.md`](../../docs/physics/README.md).

## What 65 thin ticks can and cannot settle

- **`grip_air` is genuinely excited, and a coefficient still cannot be *fitted*
  off this.** The term is `grip_air * dot(vel, right) * k * (1 - grounded)`
  (`crates/physics/src/airbrake.rs:303`), so it wants airborne ticks with lateral
  velocity and airbrake deflection - and that is exactly what these are: through
  1602-1608 the right airbrake is pinned at `100`, lateral velocity runs -9.3 to
  -4.7 units/s, and `grounded` is `0`. (Tick numbers as the columns print them.
  `grounded(t)` describes the pose at `t - 1` - see
  `crates/trace/tests/hover_contact_ground_truth.rs` - so the airborne window is
  smeared by one tick against the velocity columns, which does not move anything
  said here about a seven-tick window.) The problem is the *confound*, not the
  excitation: the craft is yawing at 0.25 to 1.18 rad/s through the same ticks,
  so the body frame the lateral component is measured in rotates through the
  sample, and the steer column ramps monotonically across the whole event. Seven
  ticks is not enough to separate a grip coefficient from a rotating frame. What
  these ticks *can* do is **falsify**: a `grip_air` far from the authored value
  would move `dot(vel, right)` visibly over those seven ticks, so the pose-walk
  technique in `hover_contact_ground_truth.rs` applied to the airbrake term would
  bound it, which is worth more than the fit and is not blocked on any capture.
- **The airborne pitch gain cannot be read off *any* committed capture, and a
  longer jump would not change that.** `Pitch::pitch_air` is "a plain gain on the
  input axis" (`crates/physics/src/params.rs:221`), and **no capture records a
  pitch input axis at all**: `scripts/psp_trace_fields.py` reads `throttle`
  (`craft+0x2b8`), `brake` (`+0x2bc`), `steer` (`+0x2c0`) and the two airbrakes
  (`+0x2c4`/`+0x2c8`), and there is no pitch field in that list. So this one is
  blocked on **locating the pitch-axis craft field and adding it to
  `psp_trace_fields.py`**, not on reaching `13_Track`. That is a much cheaper
  next step than grinding five Race Campaign grids, and it has to happen first
  either way.
- **The `-0.3` airborne weathervane is the marginal one.** The angular columns
  are there (`omega_*`), and across the crest `omega_y` swings from -0.95 to
  +1.18 rad/s. But the steer input ramps monotonically through every one of those
  ticks, so commanded yaw and any weathervane term are being asked for at the
  same time and in the same direction. A capture with the **stick centred** while
  airborne would separate them in a handful of ticks; a longer jump under a
  ramping input would not.

## Open

- `grip_air` has no runtime leg. Falsifiable off the existing captures by a pose
  walk over the airbrake term; not fittable off them.
- ~~The airborne pitch gain has no runtime leg and no input column.~~ **Closed
  2026-10-20 (`pulse-air-pitch`).** The axis was already located
  (`*(craft+0x78) + 0x10`, engine.md "The pitch axis, read at runtime"); every
  capture now writes it as `stick_pitch`, with `stick_steer` (`+0x00`). Four drop
  tests (`psp-trace.py --place`, Talon's start straight) put the pitch residual at
  `+0.008`, `+0.008` and `+0.001` rad/s² centred, up and down against a `1.92`
  input (`docs/physics/airborne-ground-truth.md`, "The air pitch laws against a
  drop test"). Gain, sign, scale, bias and in-air pitch damping hold.
- **The `-0.3` airborne weathervane is still open.** The drops were placed with
  velocity along the nose, so its yaw term is `0.16` rad/s² rms against a `0.23`
  rms yaw residual; the fit gives `-0.16` or `-0.37` depending on the constant.
- ~~The measured airborne pitch residual on the 1595 crest.~~ **Closed
  2026-10-20: a hover escape off a wall at 1594, not an air pitch law** (same
  page, "The 1595 crest residual is a hover escape off a wall"). Ported; seeded at
  1594 the nose stays within `1.5` mrad and the landing is the original's tick.
  The `--reseed 60` run keeps its `32` mrad because it reaches 1594 with `+1.7`
  units/s of vertical velocity from 1580-1584.
- **2026-10-21 (`pulse-wall-escape-census`): the 1579 landing escape is closed; the
  two probes' escapes add.** `Ship_HoverTwoPoint` measures both heights off points cached
  before its loop, so the moves sum (`1.698` recorded, `1.691` summed, `1.221` the larger).
  Ported (`Rig::TWO_POINT.sum_escapes`). The one-step census
  (`crates/trace/tests/height_miss_census.rs`) splits the old `391` misses: Autopilot rail
  writes, the magstrip hold (which a one-step walk cannot run;
  `crates/raceplay/tests/magstrip_jump_census.rs` runs it), and 1579. Under player input, 3,597
  weave rows with 1,950 wall contacts show no wall escape on either side. Evidence:
  `docs/physics/airborne-ground-truth.md`, "The two-point escapes sum".
- **Open from the census.** (a) With the landing closed, a roll from 1578 shows the nose
  drifting `-6` mrad between the 1584 cage tick and 1594 (position restored each tick, not
  with omega restored): an angular residual on the cage response. It now makes the free roll
  miss the 1594 wall escape. (b) Fort Gale player ramp `1190` tick 58: ours escapes `0.21`
  off the rear probe right after a wall hit, the original does not. (c) Magstrip hold at full
  blend: the original sits a median `+0.034` higher (282 rows), the spline locator's height.
  (d) Weave run 700, tick 804: a wall-contact row where the original moves the body `0.053`
  down along up.
- **A `MagFloor` hit does not escape (chosen).** Pulse's shape tag would pass
  it; with it on, a single-seeded pad replay (`pad_crossing_ground_truth`, 1284)
  drifted under a magstrip and escaped seven times. Settle it with a live tick of
  a probe within a unit of a magstrip (de Konstruct's first jump is one).
- The air pitch residual on the clean lap's own ten fully airborne crest rows is
  a steady `-0.34` rad/s² (against `+0.008` on the drops), and `-0.23` on its
  six half-grounded rows. Small, and the opposite sign to the old free-run gap;
  unattributed.

## Next Steps

- **The weathervane: a drop with sideslip.** `psp-trace.py --place` takes the
  velocity along the tangent it faces; give it a velocity off the nose (a
  `--place` variant, or a write of `body+0x140` after the place) and hold the
  stick centred. Ten degrees of slip at `110` units/s is about `0.6` rad/s² of
  airborne weathervane yaw, several times the residual. Run it with
  `crates/trace/tests/crest_pitch_pose_walk.rs` (`OAG_CREST_TRACE=<csv>`), whose
  `ROW` lines carry the yaw residual and the weathervane's yaw term.
- **The 1584 cage tick's angular response** (census item (a), 1-2 hours). Roll from 1578
  with `OAG_ROLL_FROM=1578 OAG_ROLL_TO=1612` (`crest_pitch_pose_walk.rs`): the `position` row
  drifts `-1.5` to `-6.1` mrad from 1585 to 1594 and the `omega` row does not, so ours'
  angular impulse on the 1584 cage tick differs. Compare `oag_gameplay::cage::pass`'s
  angular term against the original's `Body_ResolveContact` at 1584
  (`docs/ghidra/functions/psp-pulse-usa/collision-cage.md`).
- **The `--reseed 60` crest's vertical velocity.** It enters 1594 `+1.7` units/s
  fast: `+0.3` to `+0.7` after the 1579 landing escape (a `1.7`-unit lift, now summed
  as the original does, 2026-10-21) and `+1.33` more on the 1584 cage tick. One step from each recorded row matches
  there (`0.18` units/s from 1583), so this is accumulated state; roll forward
  from 1575 with one component restored per tick (the sweep's second test,
  `OAG_ROLL_FROM`) to find which.
- **Drop-test caveat.** Placed `20` units up the craft was back on the track by
  tick 0; at `9` it flew. The reset was not traced (the cage top is the
  suspect).
- `13_Track`/`10_Track` reachability notes below stand, and matter less now
  that a drop test gives clean airborne ticks on any always-free circuit.
- Whichever track is reached, the loop is: `oag-trace track --source <image>
  --track 'Data\Environments\<NN>_Track\track.vex'` for the spline,
  `scripts/psp-autopilot.py --spline ... --trace-out data/traces/<name>.csv`
  (`--hold-near X,Y,Z,R,BUTTONS` holds an input over one feature) for the
  capture, then the crest sweep above.
- **`13_Track`, the circuit with the clean authored jump, is not reachable from a
  fresh profile - measured, not assumed** (2026-08-26): `<Unlock Grid="Grid6">`;
  Race Campaign showed Grid 1 of 16 open. RaceBox Custom Race offers exactly
  `16_Track`, `03_Track` and `18_Track` (the three with no `<Unlock>`), moved with
  up/down. The nearer locked target is `10_Track` (`<Unlock Grid="Grid0">`).

## From the HANDOVER.md index (moved 2026-09-25)

**the old title was false, corrected 2026-09-07.** `talons-junction-clean-lap.csv` holds **31** non-grounded ticks over three events (8 with both probes off, longest event 14 ticks) and `talons-junction-autopilot.csv` another **34** - both on `16_Track`, both already committed. `head-pad*.csv` read `grounded 0` on all 110 of their rows, which is a constant column, not airborne data. They already paid for themselves: `crates/trace/tests/hover_contact_ground_truth.rs` uses them to show our contact test reproduces the whole `grounded` column, 2,976 of 2,976. What they **cannot** settle is more specific than "they are thin": `pitch_air` is a gain on an input axis and **no capture records a pitch axis at all** (`scripts/psp_trace_fields.py` has throttle/brake/steer/airbrakes and nothing else), so that one is blocked on locating the craft's pitch-input offset, not on reaching `13_Track`. The `-0.3` weathervane is blocked on a capture that **centres the stick** while airborne - which `16_Track` can give, no unlock needed - not on a longer jump. `grip_air` is genuinely excited (right airbrake pinned at 100 through 1602-1608, lateral velocity -9.3 to -4.7) but confounded by 1 rad/s of yaw across seven ticks: falsifiable off what exists, not fittable
