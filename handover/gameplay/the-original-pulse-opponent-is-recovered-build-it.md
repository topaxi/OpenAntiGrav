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
- **`SplinePt.unk_0x5c/0x5d/0x66` census** on Pulse's tracks: whether any
  circuit authors a lane band or a speed floor. `oag_vex::track::SplinePoint`
  does not carry the bytes yet.
- **The steering and the lane planner have no live leg.** Read whole, scored
  85 and 80.
- **`StartBoost`** at the launch is a separate, already-documented consumer
  (`ai-stats.md`), not wired by this lane.
- **Other titles.** HD's thrust law is Pulse's with four changes; its steering,
  lane planner and rail are unread; Pure, 2048 and Omega are unread.

## Next Steps

1. Extend `oag_tables::ai_race_stats` to read the whole file (`SpreadDist`, the
   six `RubberBanding` floats, `AIPackSwapping1..3`) and add an
   `AIControlStats.xml` reader, each with a real-disc ground truth. Never a
   `const` of disc values.
2. Add `unk_0x5c`, `unk_0x5d`, `unk_0x66` to `SplinePoint` and run the census;
   record it in `docs/formats/track.md`.
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
6. Falsify on the disc: a campaign race with the original driver, opponents'
   thrust logged, compared against a PPSSPP capture of the same cell with
   `scripts/psp-ai-thrust.py`. Then check 2048/Omega and HD per CLAUDE.md's
   cross-title rule.
