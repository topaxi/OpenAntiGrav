# Replays: the REPLAYS page and the viewer

**Status: built, POSE VIEW included (stage 5 of [ADR-0087](../architecture/adr/0087-a-replay-and-photo-mode-share-one-timeline.md)).**
**Chosen, not measured**: nothing here is read off an original; the screens, the words and the
controls are this project's own. The mechanism is [the replay timeline](../architecture/replay-timeline.md);
the file is [the replay file](../formats/replay-file.md). POSE VIEW, past the last
second a build reproduces, plays the file's `POSE` chunk; see "What stops it" below.

## Recording

A race the session hands over is recorded from its first tick (`RaceStage::arm_auto_replay`, called
once the session has finished writing to the race: seats, windows, the AI choice) unless the
setting `[replay] record = false` (default on, `settings.tosd`). A network race records its
confirmed run ([networking](../networking/README.md#a-clients-recording-is-the-confirmed-run-2026-11-17)). A
race with a simulation mod is recorded too: the header's `mods` list names each module, its SHA-256
and its picks, and no ghost is offered from it
([determinism-and-state.md](../architecture/wasm-extensions/determinism-and-state.md#replays)). It is written to
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
| A race whose tick-zero hash disagrees (another mode written into `[setup]`; a 2048 campaign event, `Launch::Event`; a pack or roster this machine lacks; a state-hash layout change since the recording) and the file carries a `POSE` chunk | **opens in POSE VIEW from its first tick** with the build-mismatch notice: the header still builds the race, no tick is simulated and the recorded hulls are shown. A seek lands on the pose track's pose (`oag_raceplay` `tests::viewer::a_tick_zero_mismatch_with_a_pose_track_opens_in_pose_view`) |
| The same, with no `POSE` chunk | **loading ends, the menus reopen on the REPLAYS page and the row is dimmed with NOT THE RACE THAT WAS RECORDED, AND NO POSE TRACK TO SHOW**; nothing plays (`replay_viewer_ground_truth::a_recording_this_build_cannot_rebuild_is_refused_by_name`, `tests::viewer::a_tick_zero_mismatch_without_a_pose_track_is_refused_by_name`) |
| A header that no longer builds the race (an unknown launch setting, a module this build lacks) | refused as before, whatever the file carries, with the loader's reason |
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
tick-zero mismatch with no pose track ends the load there; with one, the viewer opens in POSE VIEW (table above).

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
(PLAY or PAUSE, -5 SEC, +5 SEC, the speed, PHOTO, EXIT) with the time and the keys under them, and under
those a thin bar (grey, the built part lighter, the played part blue, a white playhead) as wide as the chip
row less 20 units at each end. All of it is in the top band, inside the chip row's width: the bottom of the
picture is the HUD's own (current time, speed, shield) and its lap box and position reach in from the sides,
so nothing of ours is over any of them. Frames read on Pulse (1440x816 PSP aspect, PS2 aspect, 1024x768
free, tick ~660): no chip, key line or bar pixel is over a HUD readout
(`replay_view::tests::what_the_screen_draws_stays_in_the_top_band_inside_the_chip_row` holds the band and
the width). The bar used to run the full width along the bottom edge, and its dark band cut the foot of the
`current` time and covered the shield percentage; the mismatch banner (a state, not the normal screen) is
still full width, just under the bar, and on the PS2 aspect lies over the lap box while it is up. A click on
the bar seeks to where it lands, a click on a chip is its button.

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

Backing out of the REPLAYS page after that is the front end's main page, on every title. A watched
recording is never parked and the viewer has its own pause (Start, Cross), so the title's pause
screen with its RESTART RACE and QUIT RACE rows is never built for it
(`replay_viewer_ground_truth::leaving_a_recording_never_lands_on_the_pause_screen`: Escape, Circle
and a click on EXIT each leave, and none lands on the pause page). The end-of-race WATCH REPLAY row
opens the same viewer through the same path.

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

### Touch

While a finger spoke last the viewer draws `replay_view::touch_layout` instead of the key legend: EXIT top
left; PHOTO, CAMERA (the next camera view, as SELECT does in a race) and HUD top right, with the craft
arrows under them; along the bottom a full-width bar (a tap seeks, a held finger scrubs, one seek every
four ticks because a seek rebuilds the race, and a last one on lift) and above the time, -5 s, PLAY, +5 s
and SPEED, all at least 0.13 of the height inside `oag_input::touch::SAFE_X`/`SAFE_Y`. **Chosen, not
measured.** A playing viewer hides the controls after three seconds without a touch (a thin progress line
stays along the edge), and a tap on the picture shows or hides them. A double tap on the left third of the
picture skips back, on the right third forward, in the middle plays or pauses. A one-finger drag, pinch or
twist on the picture (not on the bar) opens photo mode over the viewer with PLAY on when the viewer was
playing: that is the replay's free camera ([photo mode](photo-mode.md), "Touch"). A recording that is
only poses (POSE VIEW) has no race to fly, so the gestures do nothing there. The racing overlay does not
read fingers while the viewer is open.

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

WATCH REPLAY follows VIEW RESULTS AGAIN on Pulse's `EndRace Menu` list and on HD/Fury's blocks, for a
race that was recorded: it writes the file now and opens it. No original has the row
(`offers_watch_replay`, `crates/game/src/app/race_stage/endrace.rs`).

**HD/Fury** authors one `<Block>` per option, so the row is a block of ours: `view_again`'s own box,
look and label widget, one step further down, the step being the screen's own from `return_to_menu`
to `view_again` (`oag_ui_screens::endrace::hd`). On every copy of the screen on the Fury disc (five)
and on the plain PSN install that is `375, 385, 520x40`, with no text, image or other option's box
under it (`watch_replays_added_block_covers_nothing_on_*`, `crates/hd/tests/endrace_screens_ground_truth.rs`).
It sits last so the disc's three rows keep their places. Up and Down step it in screen order, a mouse
hover selects it and a click confirms, and a tap selects it and a second tap confirms, as on every
other block.

It is **not offered**:

- on a race that was not recorded (`[replay] record = false`, or the recording failed);
- while a Tournament has a leg left to race, on either title: watching replaces the finished race,
  and NEXT RACE with it, and launching the cell again starts at leg 1;
- while a network link is up (LAN or online), on either title. Watching does not drop the link, and
  the loader takes its network branch first, so it would join the room with the recording's race
  (read in `finish_loading`, not run). The recording is on the REPLAYS page once the room is left;
- under `--original-menu`, so a compare shot lists what the original lists. The `--menu-page
  endrace-menu` still never draws it;
- on 2048, whose `EndRace` pages end in a strip of two icon tiles, not wired; Pure and Omega draw no
  end-of-race menu. There the REPLAYS page is the way in.

**Leaving the viewer lands on REPLAYS, not back on the end menu**: the menu belongs to the finished
race, and loading the recording replaces that race. Nothing is lost by it. A campaign medal is saved
on the tick the race ends and the loyalty award as the screens are built, both before the menu is up.
HD checked live (2026-11-27, headless, `--autopilot`): a time trial and a campaign single race each
reached the row, played the race and landed on REPLAYS
(`replay_viewer_ground_truth::hd_end_of_race_menu_watches_the_race_just_finished` is the time trial).

## A screenshot opens the viewer

Maintainer's request, 2026-11-16: a screenshot of ours (`docs/formats/shot-png.md`) opens replay
mode on its race, positioned at the point of the shot. The `oaGs` chunk is a replay file cut at the
shot's tick, so it goes through the viewer above and nothing new draws the race
(`session::shot_open`, `replays::open_viewer`; `oag_raceplay::viewer::Viewer::starting_at`).

- **Where it opens: paused on the shot's last tick, state `ended`.** The stream stops at the shot,
  so the picture on screen is the shot's frame (the HUD, the craft and the camera are the race's at
  that tick; the viewer's chips and bar are over it). The "END OF REPLAY" banner is not drawn on
  that tick, since it would cover the picture the player came for; it is drawn once the playhead
  has moved. Until the opening pass has built that far (about a second of a 700-tick shot, longer
  for a long run) the viewer says OPENING REPLAY. CROSS or a click on PLAY plays the race from tick
  0 (as at any end of a replay), the bar seeks back, CIRCLE or EXIT leaves for **the main menu**
  (a shot is no row of the REPLAYS list).
- **A replay file (`.oagr`) takes the same two ways in**, found by its `OAGR` magic rather than its
  name: `--replay-png FILE.oagr` (also spelt `--watch`) or a drop on the main menu or a demo. It
  plays from tick 0 as WATCH does, and leaving returns to the main menu; the same refusals apply
  (another title, a header that cannot be rebuilt, modules by name, a file that will not read).
  The REPLAYS page stays the way to browse the files in `replays/`. Checked in
  `a_dropped_screenshot_opens_the_viewer_on_its_tick_...` (a `.oagr` cut from the shot's stream).
- **The way in, on the command line:** `oag-game --replay-png FILE.png [DISC]` starts the game
  and opens the file as soon as the main menu is up (the disc is found as for any run; the flag
  refuses `--race`, `--screenshot`, `--from-png`). `--from-png` stays the headless re-render.
- **The way in, on the window:** dropping the file on the window while the **main menu** (the
  front end's top page, nothing over it) or an **attract demo** (not a wallpaper, which ignores
  input) is up. A demo is left as any input leaves it, then the viewer loads. A drop anywhere else
  (a race being driven, the loading screen, any page below the main menu, a prompt, the launcher
  but for its own disc-key drop) is ignored. winit's Wayland backend reports no drops, so on a
  Wayland session a drop needs the X11 backend (the same limit as the launcher's disc drop); the
  command line always works.
- **A refusal is shown, and nothing leaves the screen.** Everything is decided before the load
  starts, so a refused drop leaves the menu or the demo where it was with an orange band over it
  (`replay_view::notice_draw`) that a click, a tap, a button or a key dismisses (the press is
  spent, so it does not also act on the row beneath it) or that goes after eight seconds:
  not a PNG of ours; a PNG of ours with no stream (the text's `no_stream_because` is shown, so a
  Race End Photo says it was taken past the recording); a shot taken on another title (names it:
  open that disc); a header that cannot be rebuilt; **simulation modules this machine lacks or has
  another build of, by name** (`oag_raceplay::extensions::preflight_recorded`, from the modules on
  offer and their bytes, with nothing compiled). What only the load can find (the rebuilt race is
  not the recorded one at tick 0, a module whose compiled layout differs) comes back to the main
  menu with the same band. The shot's own state hash is compared the first time the race stands on
  its tick; a build that does not reproduce it says so in the band (`OAG_SHOT_OTHER_HASH`).
- **Evidence:** `replay_viewer_ground_truth::a_dropped_screenshot_opens_the_viewer_on_its_tick_and_a_refused_one_says_why`
  (`--drive`, no window: the `drop` step calls the window's own `Session::file_dropped`): a
  `--race --screenshot` shot of 300 ticks opens with `replay_at=300/300/300` and the state `hash`
  of the shot, the refusals are the band's text, a drop on the RACEBOX page loads nothing, CROSS
  plays and CIRCLE returns to the main page.
  `a_dropped_screenshot_is_ignored_in_a_race_and_taken_in_a_demo`: no viewer and no band after a
  drop mid-race; a drop in a `--demo` ends the demo and opens the viewer. Viewer unit tests:
  `tests::viewer::a_viewer_started_at_a_tick_opens_paused_there` and
  `..._started_at_the_end_...`.
- **Fixed on the way:** a `--race --screenshot` shot's header carried neither the opponent driver
  it flew (`[options] ai`) nor the circuit the load had chosen, so a viewer that rebuilt the race
  by the session's loader flew this project's driver and desynced at the first stored hash (tick
  60), where the headless `--from-png` (which re-applies the command line's defaults) matched.
  The capture now notes the driver in the header (`race_capture::shot::Facts::note_ai`) and the
  load records the circuit it settled on (`Setup::request.track`).
- **Checked against the other title of the pair:** the viewer and the drop are title-agnostic;
  **ported** by construction, not run on 2048 or Omega (their `--race` shots re-simulate in
  `replay_ground_truth`; Omega's racing is incomplete, `omega-status.md`). A shot from another
  title is refused with that title's name rather than switching disc: the session boots one title.

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
- **The web build** has no file system to keep a replay in: the page is empty there (`replays::WRITES`). **A phone
  records and lists replays** into the app's files directory (`<external files>/data/oag/replays`), reached over USB
  ([Android](../tools/android.md)); on Android 10 and later the page's EXPORT row copies the replay to `Download/OpenAntiGrav` (`oag_mobile::export`), where the file manager and the share sheet reach it, and a notice says whether it went. A share sheet of our own and an import are still open (`replays::REACHABLE` is the desktop-only half).
- **Take-over, SAVE REPLAY**: stage 7.
