---
status: ready-to-wire
categories: [frontend, tooling]
titles: [pulse, pure, hd, 2048, omega]
updated: 2026-11-08
summary: "ADR-0087: stages 1 to 6 landed, the viewer and photo mode switch both ways and share their keys (ADR-0088); stage 7 take-over is next"
size: XL
difficulty: hard
---

# A replay and photo mode share one `Timeline`

[ADR-0087](../../docs/architecture/adr/0087-a-replay-and-photo-mode-share-one-timeline.md) is **accepted** (2026-11-07),
with its take-over hold decided (decision 9: at most one second after GO, ended by the stick or an airbrake). Every size
and cost is measured on all five titles by `crates/raceplay/src/tests/replay_timeline_cost.rs`, an `#[ignore]`d sweep run
with `OAG_SWEEP=1 OAG_SWEEP_TITLE=<title>`.

## Open

- **Replay and photo mode switch both ways: landed** (`replay-photo-switch`,
  [ADR-0088](../../docs/architecture/adr/0088-the-replay-viewer-and-photo-mode-switch-both-ways.md)). The viewer's PHOTO chip or SELECT
  opens photo mode at its tick; photo mode's PLAY row (effects panel, under REWIND) plays the recording on from the
  scrub over a live race or a replay; leaving a mode opened from a replay continues the replay from the scrub; a live
  race resumes to the bit. The CIRCLE leak (the leave tick ran `Race::tick` on a stale snapshot) is fixed.
  **The binding lane landed** (`replay-photo-bindings`, ADR-0088's plan; [photo mode](../../docs/ui/photo-mode.md), "Controls",
  and [replays](../../docs/ui/replays.md)): L1/R1 previous and next craft on both screens (the viewer follows it through
  `Race::set_seat`, view-only), SQUARE the HUD in the viewer, L2/R2 a view-side scrub (photo mode: a full R2 is PLAY),
  the right stick read view-side (photo mode zooms and tilts, the viewer jogs), none of it in `InputSnapshot`. **ADR-0088
  misnames the stick layout as HD's** (right orbits, left pans and zooms); HD's measured layout is built (left stick and arrows
  orbit, SQUARE pans, right stick Y zooms; right stick X tilt is chosen), in one function (`oag_raceplay::photo::roles`).
  The scrub speeds, the jog and PLAY on a full R2 are chosen. A scrub seeks every frame, so a held trigger runs at the seek's
  cost. Not run on a real pad (the right stick and triggers are injected by `--drive`: `rstick`, `lstick`, `pull`).
- **A rewind whose pre-roll starts from the first keyframe hashes differently from the live run** (landing at 101 with
  `--autopilot`, 102 with the throttle held, a Pulse Single Race, no mod mounted; landings from 131 on matched). Probably the
  first keyframe's plans (it is cut before the first tick learns them); not shown. Found by the PLAY test, which scrubs
  past it; the viewer verifies every stored hash. **Also:** the stage-6 rewind hash test replays no recording while the
  repository's mods are mounted (the race records nothing, `recorded_events` absent), so only keyframes are checked.
  `docs/ui/photo-mode.md`, "Not done, named". Find why before a rewind to the start is offered as exact.
- **Over a live race PLAY is photo mode's playback, not the viewer's screen** (ADR-0088, decision 1); a viewer over a
  live race is not built.
- **The viewer's photo mode** offers no blur rows (no keyframes of the race it moves) and is refused in POSE VIEW.
- **Stage 7, `take-over`, is next** (decision 10's text). Stages 3 and 5, its dependencies, have landed.
- **An ADR is owed for photo mode's rewind.** Stage 6 moves the live race on screen and restores it to the bit, departing
  from decision 5's probe. The maintainer accepted that on two conditions, both on `docs/ui/photo-mode.md`: photo mode
  offers no take-over, and the race resumes exactly where it was. ADRs are immutable, so a short new ADR records the
  departure for photo mode alone.
- **Campaign replays (fixed 2026-11-08, `replay-fix`).** Five campaign recordings from the maintainer's Steam Deck could not
  be watched. `Mode::from_name` skips Tournament, so `Mode::from_recorded_name` reads a header. The campaign launch's AI
  tuning and finished-craft thrust were never recorded, so `[options] campaign_skill` carries them. There is no back-compat
  (unreleased; the maintainer deletes the old files). `campaign_replay_ground_truth` verifies the five Deck files with the key
  inserted by name, from `data/reference/replays/pulse-tournament/` (local, never committed).
  `replay_viewer_ground_truth::a_campaign_race_recorded_from_the_menus_reproduces` passes with `data/dlc` mounted.
  **Docs not updated:** `docs/formats/replay-file.md` does not yet name `campaign_skill` or Tournament as claimed.
- **Viewer blur (fixed, `replay-fix`).** "LOD and blur all over the place" was motion blur history surviving a seek, so the
  landing frame smeared the whole screen. A seek now restarts the history and counts as a camera cut. No LOD difference
  was found. Open: at 2X and 4X a frame spans 2 to 4 ticks, so blur is stronger there. The maintainer's idea of rendering
  the landing frame from ticks X-3 to X is not built: the frame lands sharp instead.
- **Seen in the viewer's frames, unchecked:** the transport rows (PAUSE, -5 SEC and the rest) overlap the HUD's lap and
  position readouts. The HUD's `current` lap time read `0.04.7` at replay time `0:11 / 0:29`; confirm which clock each shows.
- **The Steam Deck tick is not measured.** The command is in the ADR (decision 3). The design needs `tick_sim_only` at or
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
- **Stage 5 landed** ([the replay page](../../docs/ui/replays.md)): every race the session hands over is recorded to `auto/`
  (a 20-file, 32 MiB ring), the REPLAYS page (WATCH, KEEP, DELETE), the viewer over a `Timeline` with a bar showing the built
  prefix, the mismatch banner, `Audio::rebuild_race_sfx`, the 60-tick pre-roll, WATCH REPLAY on the Pulse end-of-race list, and
  `[replay] record`. POSE VIEW (the viewer past the last agreeing second, on the `POSE` chunk, named end at the one-hour cap) is built. Open: split-screen craft cycling in the viewer, and the end-of-race row on HD, 2048, Pure and Omega
  (their dialects author no such row). Found on the way: a seed above `i64::MAX` made a header unreadable (about half of all
  recordings, ghosts included); a front-end race needs the original AI's request in `[options]`; `follow_devices` is now the
  `controller` event; a DLC pack mounted on the machine fields other hulls, so a rebuild mounts the reader's own.
- **Stage 4 landed** ([the replay file](../../docs/formats/replay-file.md), "The pose track"): `POSE`, every
  craft refined to 0.05 unit and 0.5 degree, captured a segment at a time. 156 to 176 B a craft a second at Venom on all
  five titles (Phantom, first 6,000 ticks: 197 to 263; 4.5 percent over the ADR's Venom row: the sweep stored no key index), a respawn bounded to the same error on all five,
  an unknown chunk skipped. `PoseTrack::pose(craft, at: f64)` is the reader stage 5 builds POSE VIEW on.
- **Stage 6 landed** ([photo mode](../../docs/ui/photo-mode.md), "Accumulated shots and rewind"): `Keeper`
  (keyframes from the first tick, the probe on the first visit), `Rewind` (the race on screen moved through its
  recording and put back to the bit), the shot plan (HD 120 and 2048 130 frames, 7 of depth of field) and the
  scrub row. It plays the race's own recording, which every race has now (REPLAYS lane); a race with
  recording off or a modded race rewinds to whole keyframes.
- **Unstaged:** replays of modded races, replays written by `oag-server`, multi-seat take-over.

## Next Steps

1. ~~Stages 1 to 5~~ landed. Zone, Eliminator and one-person Head to Head reproduced 752 ticks on Pulse through WATCH; the
   replay file still claims only the table in its page.
2. ~~Stage 6 `photo-rewind`~~ landed.
3. Stage 7 `take-over`, as the ADR orders it.
4. ~~The photo-mode rewind ADR~~ landed as ADR-0088; `replay-file.md` for `campaign_skill` and Tournament is still owed.
5. ~~Leaving photo mode with CIRCLE reaches the race as an absorb press~~ fixed (ADR-0088, decision 5).
6. ~~The binding lane~~ landed; then stage 7.
