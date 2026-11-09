# Replays: the REPLAYS page and the viewer

**Status: built, POSE VIEW included (stage 5 of [ADR-0087](../architecture/adr/0087-a-replay-and-photo-mode-share-one-timeline.md)).**
**Chosen, not measured**: nothing here is read off an original; the screens, the words and the
controls are this project's own. The mechanism is [the replay timeline](../architecture/replay-timeline.md);
the file is [the replay file](../formats/replay-file.md). POSE VIEW, past the last
second a build reproduces, plays the file's `POSE` chunk; see "What stops it" below.

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

**The screen** (`oag_game::replay_view`, `Session::viewer_screen`): six chips in a row along the top, centred
(PLAY or PAUSE, -5 SEC, +5 SEC, the speed, PHOTO, EXIT) with the time and the keys under them, and a thin bar
along the bottom edge (grey, the built part lighter, the played part blue, a white playhead).
The chips and their two lines of keys sit between the lap box (top left) and the position (top
right) of Pulse's HUD at 1440x816 on the PSP aspect, on the PS2 aspect and in a 1024x768 free window
(frames read: no rectangle of one touches the other's). **The bar does touch the HUD at the bottom**
on the PSP aspect: its dark band starts at about 783 px of the 816 px frame, which
clips the foot of the `current` lap time (the HUD's digits end at about 790 px) and covers the shield
percentage at bottom right. A click on the bar seeks to where it lands, a click on a chip is its
button.

**Two clocks are on screen.** The viewer's `0:10 / 0:29` is the recording's tick over 60, from tick 0
and so including the start-line countdown (`oag_race::COUNTDOWN_TICKS`, 272 ticks). The HUD's
`current` is the lap clock the live race shows (`Race::readout`, `lap_ticks`): ticks since the
player's craft first crossed the start line, which on lap 1 is later than the release (Pulse
Venom, a pad-flown Single Race: the line is crossed at tick 413, 141 ticks after the release; a
second run crossed at about tick 406). So `0:11` on the bar (tick 660 to 719) reads `0.04.1` to
`0.05.1` on the HUD, not 11 s, and that is right: `crates/raceplay/src/tests/viewer.rs`
(`the_hud_clocks_after_a_seek_are_the_live_races_at_that_tick`) and
`replay_timeline_ground_truth` (`*_seeks_reproduce_a_pad_flown_race`, on Pulse) assert the HUD's lap
and race clocks after a seek equal the live race's at that tick.

| Input | Does |
| --- | --- |
| Cross, Start, Enter | play or pause; from the end, play from the start |
| Left, Right | five seconds back or on (a seek) |
| Up, Down | the next speed: 0.5X, 1X, 2X, 4X |
| L1, R1 | follow the previous, next craft, wrapping, skipping craft out of the race (photo mode's rule, `oag_raceplay::photo::step_among`): the picture, the HUD, the sound and the camera follow it (`Race::set_seat`, view-only: the race on screen stays on the recorded run at every tick with the craft changed throughout, `tests::viewer::following_another_craft_moves_the_picture_and_not_one_tick_of_the_run`). A split recording shows one view and cycles all the craft |
| Square | the race's HUD on or off (the chips and the bar stay) |
| L2, R2 | scrub the playhead back and on while held, 1 to 30 ticks a frame by how far it is pulled; silent, and the sound is rebuilt where it stops. Read apart from the buttons, so neither is L1 or R1 whatever the trigger mode |
| Right stick | jog: its horizontal axis moves the playhead 1 to 10 ticks a frame; the vertical axis does nothing |
| Select, PHOTO | photo mode at the tick on screen ([photo mode](photo-mode.md), "Over the replay viewer") |
| Circle, Escape, a right click, EXIT | leave for the REPLAYS page |

The **speed**: at 1X the race sounds as a race does (`sound::race_tick`); at any other speed the
effects are muted (the cues of the ticks that ran are dropped); the music is the race's own
playlist and a seek does not touch it. **Split screen**: a split recording opens in one view of the first seat; L1 and R1 cycle every
craft in the race, not the seats alone (the seats' own windows are not built).

**While the first keyframe is not in**, `prefix()` is `None` for 0.2 to 1.1 s (the plans a keyframe
carries): the grid is held at tick 0, the bar is empty and OPENING REPLAY is on the picture.
ADR-0087 decision 3's "plays from tick 0 at once" is not literally true. After that the bar's
lighter part is the built prefix and playback waits at its edge until the pass has built the next
second (a frame of a 6,142-tick Pulse Time Trial opened from the end-of-race row showed the built
part at about 77 percent three seconds in).

## What stops it: a build mismatch

The `Timeline`'s pass ends at the first stored hash that disagrees (`Pass::Desynced`; a recorded
event of a kind a later build wrote, `Pass::Refused`, is the same to the player). The viewer
plays and seeks the agreeing part exactly and draws a banner: **THIS REPLAY WAS RECORDED ON AN
EARLIER VERSION AND STOPS MATCHING AT 0:25**. Then:

- **With a pose track** (`Replay::pose_track`, every file this build writes) the playhead goes on in
  **POSE VIEW** (`replay=pose_view`; the banner's second line says so): every craft is put at its
  recorded pose for the tick (`Race::show_poses`: the positions are written over the race on
  screen's state, the roll and lean are zero because the pose has them folded in, the velocity is
  zero, the chase camera follows, and no tick of the simulation runs), the HUD is not drawn, and
  nothing sounds. A pose track has no weapons, pickups or effects, so none are shown. A seek past
  the last agreeing second restores the last agreeing state and then places the poses. The
  picture is not a verified state: it can never start a take-over (decision 9).
- **At the end of the track** (`replay=pose_ended`) it holds. The track is capped at an hour
  (216,000 ticks): when the recording is longer, the end is **named** (END OF THE POSE TRACK: IT
  HOLDS AT MOST AN HOUR), not shown as the end of the replay, so the hulls do not freeze silently.
- **Without a pose track** (a file written before the chunk) it stops at the last agreeing second
  (`replay=stopped`, IT STOPS HERE: THIS VERSION CANNOT REBUILD THE REST).

`replay_viewer_ground_truth` changes one tick of a recorded throttle, writes the file back and
watches it: `pose_view` begins at `1500/.../1500`, runs to `pose_ended` at the end of the recording
(frames read: the banner, the craft moving with the camera on it, no HUD). The fixture tests are
`tests::viewer::past_the_last_agreeing_second_a_pose_track_plays_in_pose_view` (every craft at its
recorded pose at the end and after a seek) and `..._stops_at_the_last_agreeing_second` (no track).

## The end-of-race row

WATCH REPLAY follows VIEW RESULTS AGAIN on the **Pulse dialect** (`EndRaceDialect::Pulse`, the list
the PSP titles' `EndRace Menu` draws), for a race that was recorded: it writes the file now and opens it.
HD's blocks and 2048's buttons are the disc's own and have no such row, and Pure and Omega draw no
end-of-race menu, so on those the REPLAYS page is the way in. **Chosen, not measured.** A row on a
dialect that cannot draw it would be a cursor stop with nothing on it.

## Per title

Each title: a Time Trial flown by the Autopilot, 1,794 to 2,401 ticks, recorded through the menus
and watched through the list, frames of the page, a playing replay, and seeks of 5 seconds (`--drive`, no window): Pulse (PSP, EU), Pure (PSP, EU), HD (PS3, EU),
2048 (Vita, EU, extracted package, from the race box), Omega (PS4, EU). All five rebuild at tick zero, play, seek and pause.
2048's recording through its own campaign event is refused at tick zero (the table above).

**Checked against the other title of the pair** (CLAUDE.md): 2048 and Omega were each run from their
own files through the same path as the other three. **ported**: nothing in the viewer is per title.
Omega's light backdrop needed the band behind the detail lines; 2048's value column is ten letters.

## Open

- **The shared keys with photo mode** (ADR-0088's binding plan) are built: L1 and R1 craft, SQUARE the HUD, L2 and R2 scrub, the
  right stick jogs ([photo mode](photo-mode.md), "Controls"). The speeds of the scrub and the jog, and PLAY on a full pull of R2
  in photo mode alone, are **chosen, not measured**. A scrub seeks every frame (a keyframe restore and a 60-tick pre-roll),
  so a held trigger runs at the seek's cost, not at a fixed rate.

- **Split screen**: one view per window.
- **The Steam Deck** (ADR-0087 decision 3) is still unmeasured; the opening pass's wall time on this desktop is in the timeline page.
- **A modded race and a network race record nothing** (decision 7).
- **The web build and a phone** have no directory a player can reach: the page is empty there (`replays::SAVES`).
- **Take-over, SAVE REPLAY**: stage 7.
