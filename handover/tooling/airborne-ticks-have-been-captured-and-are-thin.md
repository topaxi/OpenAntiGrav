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
- The airborne pitch gain has no runtime leg **and no input column** - blocked on
  the craft's pitch-axis offset, not on a track.
- The `-0.3` airborne weathervane has no runtime leg - blocked on a capture that
  centres the stick while airborne, not on a longer jump.

## Next Steps

- **Cheapest first: find the pitch-axis craft field.** `steer` is `craft+0x2c0`
  and the airbrakes are `+0x2c4`/`+0x2c8`, so the input block is contiguous and
  small; the pitch axis is very likely inside it or beside it. Add it to
  `scripts/psp_trace_fields.py` and every future capture carries it.
- **Then re-capture Talon's Junction with a script that centres the stick over
  the 1595 crest.** No track unlock is needed - `16_Track` is one of the three
  always-free entries - and the crest is dated to the tick by
  `crates/trace/tests/hover_contact_ground_truth.rs`. That is the whole
  weathervane experiment, on a circuit already reachable.
- **`13_Track`, the circuit with the clean authored jump, is not reachable from a
  fresh profile - measured, not assumed.** 2026-08-26.
  `Data\Plugins\PI001\Definition.xml`'s `PI_Track name="13_Track"` carries
  `<Unlock Grid="Grid6">`, and Race Campaign's own progression screen showed only
  Grid 1 of 16 unlocked, needing 12 of 24 points to open Grid 2 - so five more
  grids stand between a fresh profile and 13_Track. Reaching it means either
  grinding five Race Campaign grids first (real races, real placements, not a
  Time Trial capture) or editing the profile's own progression state, neither of
  which is a next step to take unattended.
- **RaceBox's own Custom Race → Track Select is not a free track picker.**
  Confirmed live (PPSSPP SDL under Xvfb, `47811`/websocket debugger, screenshots
  via `magick import -window root`): it is a wrapping list of exactly **three**
  entries, moved with **up/down**, not left/right
  (`docs/reverse-engineering/ppsspp-debugger.md` already said so; that session
  re-derived it by watching `right` do nothing). Those three are exactly the
  `PI_Track` entries with **no `<Unlock>` element at all** - `16_Track` (Talon's
  Junction, always first), `03_Track` (Moa Therma) and `18_Track` (Metropia
  reversed) - everything else needs its own `<Unlock Grid="N">` cleared first.
  None of the three has a clean authored jump, which is what made this thread
  look blocked; the three events above are what it has instead.
- **The nearer locked target is `10_Track`, not `13_Track`.** Its own `<Unlock
  Grid="Grid0">` is the shallowest gate of any locked track (`Grid0` is the
  currently-active grid, not a future one), and `docs/gameplay/ai.md`'s own table
  already flags 40 of its 72 unsupported-line samples as the same "nothing within
  sixty reaches" shape as `13_Track`'s jump - a partial rather than a clean case,
  but real and much closer. Reaching it still needs at least one Race Campaign
  cell cleared (a real race, opponents and weapons on, placed well enough for the
  cell's medal), which `scripts/psp-autopilot.py` was not built or tested for -
  it flies a Time Trial line, not a contested race.
- Whichever track is reached, the loop is: `oag-trace track --source <image>
  --track 'Data\Environments\<NN>_Track\track.vex'` for the spline (accepts any
  track, not just the default), `scripts/psp-autopilot.py --spline ...
  --trace-out data/traces/<name>.csv` for the capture, then read the airborne
  terms off the airborne ticks the same way
  `docs/physics/cornering-ground-truth.md`'s `scripts/trace-cornering.py` reads
  the grounded ones.

## From the HANDOVER.md index (moved 2026-09-25)

**the old title was false, corrected 2026-09-07.** `talons-junction-clean-lap.csv` holds **31** non-grounded ticks over three events (8 with both probes off, longest event 14 ticks) and `talons-junction-autopilot.csv` another **34** - both on `16_Track`, both already committed. `head-pad*.csv` read `grounded 0` on all 110 of their rows, which is a constant column, not airborne data. They already paid for themselves: `crates/trace/tests/hover_contact_ground_truth.rs` uses them to show our contact test reproduces the whole `grounded` column, 2,976 of 2,976. What they **cannot** settle is more specific than "they are thin": `pitch_air` is a gain on an input axis and **no capture records a pitch axis at all** (`scripts/psp_trace_fields.py` has throttle/brake/steer/airbrakes and nothing else), so that one is blocked on locating the craft's pitch-input offset, not on reaching `13_Track`. The `-0.3` weathervane is blocked on a capture that **centres the stick** while airborne - which `16_Track` can give, no unlock needed - not on a longer jump. `grip_air` is genuinely excited (right airbrake pinned at 100 through 1602-1608, lateral velocity -9.3 to -4.7) but confounded by 1 rad/s of yaw across seven ticks: falsifiable off what exists, not fittable
