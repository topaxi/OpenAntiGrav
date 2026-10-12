# Pure's campaign is built as a list; what stays open

## Open

- **2026-10-12 `pure-campaign`: landed.** `RACE CAMPAIGN` on Pure opens a list of 32 events a class (tournaments, single races, time trials, Zone); each launches as a campaign cell through the Pulse/HD path, its medal records into `[[campaign]]`, and the disc's 17 `<Unlock>` rows gate events and two teams. Permanent write-up: `docs/formats/pure-campaign.md`; the recovered law: `docs/ghidra/functions/psp-pure-usa/campaign.md`.
- **The class gate.** A fresh profile's `Class Selection` lists Vector and Venom only (PPSSPP, 2026-09-10); that gate is in code, not in `Definition.xml`. The list shows all five classes (chosen). `Unlock Phantom Class` (`0x08a7c848`) is the one named hook; `TeamSelection_ApplySelection`'s Phantom branch (confidence 60, see `pure-splits-the-race-box-into-a-chain.md`) is probably the same gate.
- **`MedalCount`'s counter.** `Medal_Evaluate` raises gold/silver/bronze counters (`+0x7c`/`+0x78`/`+0x74`) on every award, repeat or not; whether the `MedalCount` check sums them and whether a replay counts again was not traced. The list counts each event's best medal once (chosen). A live PPSSPP check: earn one medal twice and read `MedalCount`'s operand.
- **Pure's own opponent in the campaign.** The lane left `Title::race.original_opponent` alone by instruction: the planner and rail are not built (`psp-pure-usa/ai-controls.md`). A Pure campaign race is already marked as a campaign cell, so flipping it later needs no screen work. Time trials and Zone have no opponents; races and tournaments are where it shows.
- **The race box does not gate.** Pure's Track Selection and Team Selection filter by the same unlock rows in the original in every mode. Only the campaign list and `Team Selection` (via `pure_locked_teams`) read them here; the race box's circuit list does not drop a Classic or Zone circuit until its medal count is met. `oag_game::unlock::Gate` is the place if the maintainer wants it.
- **The tournament leg medal is judged on the last leg only** (as `Medal_Evaluate` does, `race index == race count - 1`), through `Session::tournament` unchanged from Pulse; a Pure tournament abandoned midway records nothing. Not walked windowed beyond the first race.
- **Time trial laps.** `RaceTimes` is a whole-race figure; the list races the mode's default (three). The autopilot's 111.78 s against 105/108/111 on Vineta K agrees; a Pure capture of a time trial's lap counter would settle it.
- **No windowed walk to a recorded medal.** The window path was walked on an Xvfb through the list, class step, hover and click, Team Selection and a Zone launch; a finished race's medal was walked on the real simulation without the window (`pure_campaign_walk_ground_truth`: a time trial and a four-leg tournament under the autopilot, judged, recorded, the list re-read). A hand-driven finish through the end-of-race screens is unplayed.
- **No sound on the list's own events beyond the menu cues** (`Nav::Moved`, `Stepped`, `Accept`, `Decline`); Pure's `FE` name voices are not wired.

## Next Steps

1. Live-check the `MedalCount` counter semantics on PPSSPP (see above); then drop the "chosen" label in `pure_list`'s doc and `campaign.md`.
2. Decide whether Pure's race box should gate circuits and classes like the original (`oag_game::unlock::Gate`); the unlock evaluation is `oag_tables::pure_campaign::Unlock::met`, reusable.
3. Find the class gate (`Unlock Phantom Class`, `0x08a7c848`) and read it as data.
