# Replays: the REPLAYS page and the viewer

**Status: built (stage 5 of [ADR-0087](../architecture/adr/0087-a-replay-and-photo-mode-share-one-timeline.md)).**
**Chosen, not measured**: nothing here is read off an original; the screens, the words and the
controls are this project's own. The mechanism is [the replay timeline](../architecture/replay-timeline.md);
the file is [the replay file](../formats/replay-file.md). **Open: POSE VIEW** (the viewer past the
last second a build reproduces), which waits for the `POSE` chunk; see "What stops it" below.

## Recording

A race the session hands over is recorded from its first tick (`RaceStage::arm_auto_replay`, called
once the session has finished writing to the race: seats, windows, the AI choice) unless the
setting `[replay] record = false` (default on, `settings.tosd`), the race is a network race, or
a simulation mod runs in it (`Race::start_recording` returns early). It is written to
`<data dir>/oag/replays/auto/oag-<title>-<unix seconds>-<n>.oagr` **when the stage is dropped**,
which every way out of a race goes through (the finish, QUIT RACE, RESTART, leaving the game), and
at WATCH REPLAY. A race under 300 ticks (five seconds) is not kept. **A race a test, a trace, a
capture or a server builds records nothing**: arming is the session's alone
(`replay_viewer_ground_truth::a_race_a_capture_builds_leaves_no_replay`).

**The ring** (`oag_game::replays`): `auto/` holds the newest 20 files and at most 32 MiB, oldest
deleted first by the number in its name (never its modification time), the newest kept whatever its
size; `saved/` is never pruned. `replays::tests::the_ring_holds_twenty_files_...` and `..._thirty_two_mebibytes_...`
run the two limits, the second with the real 32 MiB.

**A bug this found.** `Header::seed` is drawn over all of `u64` (`race_seed::fresh`) and a TOML
integer is an `i64`: a seed above `i64::MAX` was written as it stood and made a header no reader
could parse, so about half of the races a player recorded (ghosts too, since ADR-0055) could not be
read back. The seed is now stored as the same 64 bits as an `i64` (`file::tests::a_seed_over_the_signed_range_survives_the_file`).

**What the header needed beyond ADR-0087's tables**, each found by the tick-zero hash refusing a
front-end race the first time:

- **The AI choice.** The session flies the opponents with the original driver
  (`ai_choice::apply_noting`); a rebuild must too, so `[options]` carries `ai = "original"`,
  `ai_level` and `ai_skill` (`Launch::original_ai`), applied after the seats.
- **DLC packs.** `opponent_teams` may name a team only a mounted pack ships. A rebuild without the
  pack fields another hull there. The viewer mounts this machine's own packs (`race_options.dlc`);
  whether they are the recording's is the tick-zero hash's to say.
- **A pad leaving a seat** (`follow_devices`) is the `controller` event (`Kind::Controller`, code 3):
  `Race::set_controller` writes and records it, the applier writes it between the same two ticks.
  `timeline::tests::a_seat_handed_to_the_driver_reproduces_and_without_its_event_desyncs`: with the
  event the recording reproduces, with it stripped the first stored hash after it disagrees.
  Chosen over "arm nothing on a split race": a pad leaving cannot be predicted, so that would never
  record one.

## The REPLAYS page

A REPLAYS row on the main page (`oag-extras`, `assets/mods/oag-extras/menu.toml`, after
MULTIPLAYER) opens `assets/ui/menu.toml`'s `replays` page: REPLAY (a stepper over `saved/` then
`auto/`, newest first, read each time the menus open: `ValueSource::Replays`), WATCH, KEEP, DELETE,
BACK. KEEP copies an `auto/` file into `saved/` and puts the row on the copy; DELETE removes the
file. The list reads each file's `HEAD` only (`Replay::read_head`, a few hundred bytes).

- **The row** is the day of the month and the time the file was written, `*` first for a kept one
  (`07 22:16`): ten letters is what 2048's face has room for.
- **The detail lines** under the rows are CIRCUIT, FINISH, RACE (mode and class), NOTE, DATE (UTC:
  there is no time-zone source) and SIZE, in that order, cut from the end to the room the title's
  skin leaves (Pulse two lines, Pure one, HD and Omega all). The REPLAYS page draws a dark band
  behind them: Omega's backdrop is light and its detail lines are white. The shared drawing half is
  `oag_ui_screens::prompt::record_row_draw`, which RECORDS and MODS also use.
- **A file this build will not play is listed dimmed with the reason** (the note slot, as a mod that
  did not load is): RECORDED BY A NEWER VERSION (a header naming another kind or a later major),
  DAMAGED FILE, CANNOT BE REBUILT: NO LAUNCH REQUEST, RECORDED ON <TITLE>: OPEN THAT DISC. The note
  takes the first detail line's place.
- **2048 has no list of rows**: its front end is the disc's own touch grid, so REPLAYS is the fourth
  of this build's tiles on `GameModeChoice` (`EXTRA_TILES` is four now, the fourth 80 units tall to
  fit under the grid's 544; a mod's REMIX tile still fits).

## What a player sees for a mode the file does not claim

[The replay file](../formats/replay-file.md) proves the rebuild for Time Trial, Speed Lap and Single
Race on all five titles and Head to Head with two people on Pulse. The list cannot know a tick-zero
result from `HEAD`, so **any other mode is listed with NOTE: MODE NOT VERIFIED and WATCH tries it**:

| What the header names | What WATCH does |
| --- | --- |
| Zone, Eliminator, Head to Head with one person, Speed Lap (Pulse, run through WATCH, 752 ticks each) | rebuilds, the pass ends complete, plays to the end |
| A race whose tick-zero hash disagrees (another mode written into `[setup]`; a 2048 campaign event, `Launch::Event`; a pack or roster this machine lacks) | **loading ends, the menus reopen on the REPLAYS page and the row is dimmed with NOT THE RACE THAT WAS RECORDED**; nothing plays (`replay_viewer_ground_truth::a_recording_this_build_cannot_rebuild_is_refused_by_name`) |
| A file that will not read at all | the same dimmed row with the reader's reason |

Not run: Tournament, a campaign cell on Pulse, Race Remix, foreign tracks, split screen beyond Pulse's two
people. Their tick-zero hash is the judge: they play only if it agrees, and a later disagreement
is the mismatch banner.

## The viewer

WATCH (or WATCH REPLAY at the end of a race) reads the file, builds `oag_raceplay::Options::for_replay` over the
disc the session booted from, and loads the race through the **ordinary loader and scene build**
(`Session::start_race_loading`, the half of a launch a race box and a recording share). At the
hand-over `Viewer::open` starts the `Timeline` over a copy of the loaded `Setup` and `Viewer::begin`
puts the header's launch settings and seats on the race on screen, stops its recording and ghost. A
tick-zero mismatch ends the load there.

`oag_raceplay::viewer::Viewer` moves the race on screen with `Race::tick` (drawn), and the timeline's
probe only restores keyframes: a seek restores the keyframe at or before `target - 60` into the race
on screen (`Race::restore_for_viewing`, which also clears the board and the wreck tick the view
keeps), then runs the ticks to the target as **drawn, silent ticks** (the pre-roll: the camera
spring, exhaust and particles settle), and the session rebuilds the sound from the state
(`oag_game::sound::rebuild`, `Audio::rebuild_race_sfx`). `replay_viewer` fixture tests
(`crates/raceplay/src/tests/viewer.rs`) compare the race on screen with the recorded run's
`RaceSim` text at every tick of a play and at every 7th tick of a shuffled seek order.

**Nothing a watched race does is a record, a ghost or a replay**: the frame loop hands the race to
`Session::tick_viewer` and `continue`s past the pause, the finish and escape arms, `result_saved`
is set, `RaceStage::watching` keeps it out of the parked-race slot, and its recorder is off.
`replay_viewer_ground_truth::a_recorded_race_is_watched_and_a_mismatch_is_shown` compares
`config/` (`records.toml`, `settings.toml`) and `ghosts/` byte for byte, and the file list of
`auto/`, before and after a watch to the end.

**The screen** (`oag_game::replay_view`, `Session::viewer_screen`): five chips in a row along the top, centred
(PLAY or PAUSE, -5 SEC, +5 SEC, the speed, EXIT) with the time and the keys under them, and a thin bar
along the bottom edge (grey, the built part lighter, the played part blue, a white playhead).
The top and bottom are where every title's HUD leaves room (lap box top left, position top
right, times and speed bottom). A click on the bar seeks to where it lands, a click on a chip is
its button.

| Input | Does |
| --- | --- |
| Cross, Start, Enter | play or pause; from the end, play from the start |
| Left, Right | five seconds back or on (a seek) |
| Up, Down | the next speed: 0.5X, 1X, 2X, 4X |
| Circle, Escape, a right click, EXIT | leave for the REPLAYS page |

The **speed**: at 1X the race sounds as a race does (`sound::race_tick`); at any other speed the
effects are muted (the cues of the ticks that ran are dropped); the music is the race's own
playlist and a seek does not touch it. **Split screen**: a split recording opens in one view of the first seat; L and R do not
cycle craft yet (**open**).

**While the first keyframe is not in**, `prefix()` is `None` for 0.2 to 1.1 s (the plans a keyframe
carries): the grid is held at tick 0, the bar is empty and OPENING REPLAY is on the picture.
ADR-0087 decision 3's "plays from tick 0 at once" is not literally true. After that the bar's
lighter part is the built prefix and playback waits at its edge until the pass has built the next
second (a frame of a 6,142-tick Pulse Time Trial opened from the end-of-race row showed the built
part at about 77 percent three seconds in).

## What stops it: a build mismatch

The `Timeline`'s pass ends at the first stored hash that disagrees (`Pass::Desynced`). The viewer
plays and seeks the agreeing part exactly, stops at its last second (`replay=stopped`) and draws a banner:
**THIS REPLAY WAS RECORDED ON AN EARLIER VERSION AND STOPS MATCHING AT 0:25** and IT STOPS HERE:
THIS VERSION CANNOT REBUILD THE REST. `replay_viewer_ground_truth` changes one tick of a recorded
throttle, writes the file back, and watches it: the viewer reports `stopped` at `1500/1500`, the
last agreeing hash. **POSE VIEW is not built**: with the `POSE` chunk the playhead would go on
past that second with the hulls at their recorded poses and no HUD (decision 4); with a file
that has none, which is every file today, it stops there, which is what this build does.

## The end-of-race row

WATCH REPLAY follows VIEW RESULTS AGAIN on the **Pulse dialect** (`EndRaceDialect::Pulse`, the list
the PSP titles' `EndRace Menu` draws), for a race that was recorded: it writes the file now and opens it.
HD's blocks and 2048's buttons are the disc's own and have no such row, and Pure and Omega draw no
end-of-race menu, so on those the REPLAYS page is the way in. **Chosen, not measured.** A row on a
dialect that cannot draw it would be a cursor stop with nothing on it.

## Per title

Each title: a Time Trial flown by the Autopilot, 1,794 to 2,401 ticks, recorded through the menus
and watched through the list, frames of the page, a playing replay, and seeks of 5 seconds (`--drive`,
`data/scratch/replay-viewer/ev-<title>/`): Pulse (PSP, EU), Pure (PSP, EU), HD (PS3, EU),
2048 (Vita, EU, extracted package, from the race box), Omega (PS4, EU). All five rebuild at tick zero, play, seek and pause.
2048's recording through its own campaign event is refused at tick zero (the table above).

**Checked against the other title of the pair** (CLAUDE.md): 2048 and Omega were each run from their
own files through the same path as the other three. **ported**: nothing in the viewer is per title.
Omega's light backdrop needed the band behind the detail lines; 2048's value column is ten letters.

## Open

- **POSE VIEW** (decision 4) waits for the `POSE` chunk.
- **Split screen**: the craft L and R cycle, one view per window.
- **The Steam Deck** (ADR-0087 decision 3) is still unmeasured; the opening pass's wall time on this desktop is in the timeline page.
- **A modded race and a network race record nothing** (decision 7).
- **The web build and a phone** have no directory a player can reach: the page is empty there (`replays::SAVES`).
- **Take-over, SAVE REPLAY, photo rewind**: stages 6 and 7.
