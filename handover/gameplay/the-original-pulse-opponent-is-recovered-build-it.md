# The original Pulse opponent is recovered: build `OriginalDriver` for the campaign

2026-10-11, lane `orig-ai-pulse` (RE only, nothing built). The maintainer's
decision of the same day: the campaign gets a third AI mode, the original's own
opponent with its rubber banding; race box, remix and multiplayer keep ours.

The specification is [ai.md, "The original driver, recovered"](../../docs/gameplay/ai.md#the-original-driver-recovered-2026-10-11);
the formulas, addresses and the live measurement are on
[ai-controls.md](../../docs/ghidra/functions/psp-pulse-usa/ai-controls.md).
Do not requote either here.

What is settled, in one breath: an opponent writes the player's control record
and runs the player's physics (measured), a kinematic rail moves its body
toward its planned lane every tick (measured, small steady-state move), its
thrust law reproduces 1,140 live samples exactly at two skills (measured), and
its steering and lane planner are read at instruction level (not traced).

## Open

- **The rail's place in the frame.** `AiManager_Update` has no static caller
  (vtable); whether it runs before or after `Body_StepWorld` is not read.
  The `RailCorrection` must be applied at the same point.
- **~~`SplinePt.unk_0x5c/0x5d/0x66` census~~** done, see Next Steps 2. What
  remains: no live leg shows `07_Track`'s band or `06_Track`'s zeros acting on an
  opponent, and `06_Track`'s other `+0x66` values (at or above `128`) are a use
  of the byte nobody has read.
- **The steering and the lane planner have no live leg.** Read whole, scored
  85 and 80.
- **What the thrust samples cannot discriminate** (unity multipliers,
  `WhenBehind` below its clamp, the ship-state gate) and **how much of the line
  the rail holds** against the steering: both listed with their one-run
  falsifiers on `ai-controls.md`'s "Measured" section.
- **The campaign branch of `AI_ResolveSkillScale` was not run live**: the
  measurement used Single Races at two `g_skill_level` values.
- **`StartBoost`** at the launch is a separate, already-documented consumer
  (`ai-stats.md`), not wired by this lane.
- **Other titles.** HD's thrust law is Pulse's with four changes; its steering,
  lane planner and rail are unread. The tables are read on HD, 2048 and Omega
  (identical to Pulse's plus `SplitScreenMultiplier`), and Pure's are another
  schema, refused by name ([formats/ai-tables.md](../../docs/formats/ai-tables.md)).

## Next Steps

1. ~~Extend `oag_tables::ai_race_stats` to read the whole file and add an
   `AIControlStats.xml` reader.~~ **Done 2026-10-12 (`ai-tables`).**
   `ai_race_stats::parse_class_race` returns a `ClassRace` (spread, grid,
   both rubber-band rows, the skill curve's spread multiplier, the three
   pack-swap rows, HD's `SplitScreenMultiplier`); `ai_control_stats` returns a
   `Controller` per class with an absent attribute as `None` (`xtrackMax` is
   never authored, so the executable's default is the driver's to apply). Both
   refuse by name, and real-disc ground truth covers Pulse PSP USA and EU,
   PS2, HD disc and PSN, 2048 EU and USA, Omega and Pure
   ([formats/ai-tables.md](../../docs/formats/ai-tables.md)).
2. ~~Add `unk_0x5c`, `unk_0x5d`, `unk_0x66` to `SplinePoint` and run the
   census.~~ **Done 2026-10-12.** `Option<u8>`, `None` on 2048's short points.
   `07_Track` is the only circuit with a lane band (311 points, two runs a
   direction); `06_Track` is the only one authoring `+0x66`, and only the
   zeros (80 points) sit below `35`. Both pressings identical
   ([formats/track.md](../../docs/formats/track.md)).
3. Build `oag_ai::original::OriginalDriver` to the sketch in `ai.md`: thrust law
   first (it has 1,140 samples to test against: port the cases
   `scripts/psp-ai-thrust.py --predict` checks as unit tests with synthetic
   tables), then the lane planner and ring, then the steering, then the pitch.
4. Give `oag-physics` an explicit body write for the rail, and apply it at the
   frame point the original does (Open, first bullet: measure it first with one
   PPSSPP run breaking on `AiManager_Update` and `Body_StepWorld` alternately).
5. Wire it into the campaign only, from the mode, never a setting; keep
   `Session::resolve_campaign_ai_skill_scale` for the skill scale and pass the
   integer difficulty for the shuffle.
6. Rewrite the doc comments the build falsifies, in the same change: the
   `oag-ai` `lib.rs` header ("their speed is not"), and `oag_raceplay::finished_thrust`'s
   "the opponents' `spread` wander this project does not model".
7. Falsify on the disc: a campaign race with the original driver, opponents'
   thrust logged, compared against a PPSSPP capture of the same cell with
   `scripts/psp-ai-thrust.py`. Then check 2048/Omega and HD per CLAUDE.md's
   cross-title rule.

## What the `OriginalDriver` build can rely on (2026-10-12)

- `ClassRace` and `Controller` are complete and refuse by name; a field a file
  does not author is `None` (controller) or a refusal (race), never a guess.
  `ClassRace::thrust` is the `AiRaceStats` the finished-player law already uses.
- `SplinePoint::unk_0x5c/0x5d/0x66` are `Option<u8>`: `0x5c`/`0x5d` read as the
  forced `0xff`/`0x00` before version `0x105`, `None` on 2048's short points.
  `oag-trackfile`'s GLB does not carry them (an export drops them and a read gives `None`),
  and so a custom track has no band or floor today.
- HD, 2048 and Omega can use the same `ClassRace` with no title branch; HD-era
  files add `split_screen`, the only structural difference.
