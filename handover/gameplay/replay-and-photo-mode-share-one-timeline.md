---
status: blocked-maintainer
categories: [frontend, tooling]
titles: [pulse, pure, hd, 2048, omega]
updated: 2026-11-08
summary: "ADR-0087: one Timeline for replay viewer, photo rewind and take-over; stages 1 to 4 and 6 landed, stages 5 and 7 open"
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
- **Whether a fresh `Race` derives `capture_results` and `wreck_ended_tick`** at a take-over after a finish. The plan
  refuses those moments; a stage-7 lane may lift that.
- **Stage 2 landed** ([replay file](../../docs/formats/replay-file.md)): `HEAD` names its schema, `[setup]` and `[options]` rebuild the
  race (`oag_raceplay::rebuild`; tick-zero hash proved for Time Trial, Speed Lap and Single Race on all five titles), `EVNT`
  carries `pilot_assist` and `give`, `[fork]` is read. **Not in `[setup]`, so not claimed:** Zone, Eliminator, Tournament,
  Head to Head, split-screen and multi-window (`seat_picks`, several people), Race Remix, DLC and foreign tracks, campaign
  events (`grid_teams`), capture aids (`pose`, `field`), network and modded races. A recording of one of these may be written
  and its rebuild is refused by the tick-zero hash, never played wrong.
- **Stage 3 landed** ([the timeline](../../docs/architecture/replay-timeline.md)): `Timeline`, keyframes, the ceiling, the
  opening pass, the desync state. A seek reproduces a pad-flown Single Race on all five titles, and split screen on Pulse (two
  people, Head to Head) once `start_recorded` seats `Header::slots`. Its page's "Open" lists what stage 5 inherits: the probe's
  `results` and `wreck_ended_tick` after a seek back, quiet `step`, and `follow_devices` as an unrecorded writer of `controllers`.
- **Stage 4 landed** ([the replay file](../../docs/formats/replay-file.md), "The pose track"): `POSE`, every
  craft refined to 0.05 unit and 0.5 degree, captured a segment at a time. 156 to 176 B a craft a second at Venom on all
  five titles (Phantom, first 6,000 ticks: 197 to 263; 4.5 percent over the ADR's Venom row: the sweep stored no key index), a respawn bounded to the same error on all five,
  an unknown chunk skipped. `PoseTrack::pose(craft, at: f64)` is the reader stage 5 builds POSE VIEW on.
- **Stage 6 landed** ([photo mode](../../docs/ui/photo-mode.md), "Accumulated shots and rewind"): `Keeper`
  (keyframes from the first tick, the probe on the first visit), `Rewind` (the race on screen moved through its
  recording and put back to the bit), the shot plan (HD 120 and 2048 130 frames, 7 of depth of field) and the
  scrub row. It plays the race's own recording, which on `main` only a Time Trial or Speed Lap has; the REPLAYS
  lane's "record every race" is what makes it every race.
- **Unstaged:** replays of modded races, replays written by `oag-server`, multi-seat take-over.

## Next Steps

1. ~~Stages 1 to 4~~ landed. The modes the replay file does not claim stay unclaimed: nothing has shown them reproducing.
2. ~~Stage 4 `pose-track`~~ landed; ~~stage 6 `photo-rewind`~~ landed; stages 5 and 7 as the ADR orders them.
