---
status: blocked-maintainer
categories: [frontend, tooling]
titles: [pulse, pure, hd, 2048, omega]
updated: 2026-11-07
summary: "ADR-0087 (proposed) designs one Timeline for replay viewer, photo rewind and take-over; stage 1 (share the snapshot track half) starts on accept"
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
  rows, the 3-2-1 countdown and the two-second hold at a take-over, and v1 being single-seat. The hold lets go of the
  recorded stream up to two seconds after "here" (decision 9); say if that is a departure to refuse.
- **The Steam Deck is not measured.** The command is in the ADR (decision 3). The design needs `tick_sim_only` at or
  under about 280 us there.
- **A respawn in the pose track is argued, not shown**: the sweep's Autopilot races have none. Stage 4 needs one.
- **Whether a fresh `Race` derives `capture_results` and `wreck_ended_tick`** at a take-over after a finish. The plan
  refuses those moments; a stage-7 lane may lift that.
- **Split-screen recordings** are not shown to reproduce.
- **Unstaged:** replays of modded races, replays written by `oag-server`, multi-seat take-over.

## Next Steps

1. Stage 1, `shared-track-half`: put the constant half of `RaceSim` (collision, spline, racing line, ai order, routes,
   pads, weapon tables, speed and hull plans, title laws) behind an `Arc`. Full gate; no golden hash moves. Re-run the
   sweep for the snapshot table.
2. Stage 2, `replay-header-v1`, in parallel: `toml-schema` in `HEAD`, `[setup]`, options read back, `EVNT`, `[fork]`.
3. Then stage 3 `timeline-core` and stage 4 `pose-track`, then 5 to 7 as the ADR orders them.
