---
status: blocked-maintainer
categories: [frontend, tooling]
titles: [pulse, pure, hd, 2048, omega]
updated: 2026-11-07
summary: "ADR-0087 designs one Timeline for replay viewer, photo rewind and take-over; stage 2 (replay header v1) landed, stage 1 in flight, stages 3 to 7 open"
size: XL
difficulty: hard
---

# A replay and photo mode share one `Timeline`

[ADR-0087](../../docs/architecture/adr/0087-a-replay-and-photo-mode-share-one-timeline.md) is **proposed**, with every size
and cost measured on all five titles by `crates/raceplay/src/tests/replay_timeline_cost.rs` (an `#[ignore]`d sweep, run
with `OAG_SWEEP=1 OAG_SWEEP_TITLE=<title>`). Nothing is built. The maintainer's decisions of 2026-11-07 are marked in the
ADR and are not open.

## Open

- **Accept or amend the ADR.** What is a proposal, not the maintainer's: the 60-tick keyframe, the 16 MiB ceiling, the
  `auto/` ring (20 files, 32 MiB), the pose track at 10 Hz refined to 0.05 unit and 0.5 degree, the REPLAYS page and its
  rows, the 3-2-1 countdown at a take-over, and v1 being single-seat.
- **A question for the maintainer (decision 9):** is a frozen 3-2-1 with the recorded stick on screen enough, or should the seat
  keep its recorded input after GO until the pad engages (at most two seconds)? The hand-off table says 12 to 45 percent of
  points hit a wall inside two seconds with nothing held. The hold departs from "the stream stops at tick T", so it is
  asked, not taken.
- **The Steam Deck is not measured.** The command is in the ADR (decision 3). The design needs `tick_sim_only` at or
  under about 280 us there.
- **A respawn in the pose track is argued, not shown**: the sweep's Autopilot races have none. Stage 4 needs one.
- **Whether a fresh `Race` derives `capture_results` and `wreck_ended_tick`** at a take-over after a finish. The plan
  refuses those moments; a stage-7 lane may lift that.
- **Stage 2 landed** ([replay file](../../docs/formats/replay-file.md)): `HEAD` names its schema, `[setup]` and `[options]` rebuild the
  race (`oag_raceplay::rebuild`; tick-zero hash proved for Time Trial, Speed Lap and Single Race on all five titles), `EVNT`
  carries `pilot_assist` and `give`, `[fork]` is read. **Not in `[setup]`, so not claimed:** Zone, Eliminator, Tournament,
  Head to Head, split-screen and multi-window (`seat_picks`, several people), Race Remix, DLC and foreign tracks, campaign
  events (`grid_teams`), capture aids (`pose`, `field`), network and modded races. A recording of one of these may be written
  and its rebuild is refused by the tick-zero hash, never played wrong.
- **Split-screen recordings** are not shown to reproduce.
- **Unstaged:** replays of modded races, replays written by `oag-server`, multi-seat take-over.

## Next Steps

1. Stage 1, `shared-track-half`: put the constant half of `RaceSim` (collision, spline, racing line, ai order, routes,
   pads, weapon tables, speed and hull plans, title laws) behind an `Arc`. Full gate; no golden hash moves. Re-run the
   sweep for the snapshot table.
2. ~~Stage 2, `replay-header-v1`~~ landed. Stage 3 also owes the unclaimed modes above.
3. Then stage 3 `timeline-core` and stage 4 `pose-track`, then 5 to 7 as the ADR orders them.
