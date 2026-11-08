# Photo mode

What each title's photo mode is, read off its executable and its data (2026-11-06, lane
`photo-read`). Scores follow the [rubric](../reverse-engineering/confidence-rubric.md). The
functions and their evidence are on each binary's page:
[Pulse EU](../ghidra/functions/psp-pulse-eu/photo-mode.md),
[HD](../ghidra/functions/ps3-hdfury-eu/photo-mode.md),
[2048](../ghidra/functions/vita-2048-eu-v104/photo-mode.md),
[Omega](../ghidra/functions/ps4-omega-eu/photo-mode.md).

## By title

| Title | Has one | Entry | Shot | Conf. |
| --- | --- | --- | --- | ---: |
| Pulse PSP | yes | Pause (`EXT_PHOTO`, single-player screens only); SELECT on `Race End Photo` | 480x272 JPEG 4:4:4, quality argument 90, `/PICTURE/WIPEOUT_PULSE/` | 90 |
| Pulse PS2 | no save path | See [below](#pulse-ps2) | none | 70 |
| Pure | **no** | none | none | 90 |
| HD/Fury | yes | Pause, including split screen; SQUARE on `Race End Photo` | JPEG at display resolution (1920x1080 measured), quality argument 90, exported to the XMB | 90 |
| 2048 | yes, its own | Pause and `Race End Photo`: `PhotoModeTouchButton` to `InGame VitaPhoto` | JPEG at the display surface, `ScePhotoExport` | 84 |
| Omega | yes, cut down | Pause; SQUARE at the finish per the XML, the string says OPTIONS | The console's Share function, no file of its own | 80 |

## Pure has none

Confidence 90. No string containing `photo` (any case) in `BOOT.BIN`, EU or USA, nor `.jpg`. All
1,241 entries of `Data.wad`, `FE.wad` and `FEData.wad` of the EU disc, extracted and searched: no
`photo` in any, including the language files (nine files with `Entry ID=`, up to 909 rows each).
`InGame_Definition.xml` authors no photo screen, and the pause screens START opens (`InGame Main Menu`,
`InGame NoRestart Main Menu`, `InGame Free Play Menu`) have no photo row: walked live on PPSSPP (USA) in Single Race,
Time Trial, Tournament, Free Play and Zone, `docs/ui/pause-menu.md`, "Pure". Short of 94: the USA disc's archives
were not searched, only its binary. This build's PHOTO MODE row follows CONTINUE on every Pure pause screen.

## Pulse

Screens, all in `Data\Plugins\PI001\GUI\InGame_Definition.xml` (`31b50f2e`), `Photo Mode Top Level`:

| Screen | Task | Purpose |
| --- | --- | --- |
| `InGame Photo` | `Pause` | The two rows and the legends |
| `InGame Taking Photo` | none | Empty, so no menu is in the shot; 30 frames, then the save |
| `InGame Taking Photo Saving` | `Pause` | `TRC_SAVING`, waits on the writer thread |
| `InGame Photo Message` | `Pause` | Result dialog, `FE_CONTINUE` back to `InGame Photo` |
| `InGame Race End Photo` | `Pause` | A prompt, `ER_PHOTO_TAKE2`, then `InGame Taking Photo`. No `goto` to it in this file, and only the camera gate names it in code |

**Entry from pause.** `InGame Pause SP`, `SP Time Trial`, `SP Speed Lap`, `SP Tournament` carry
`EXT_PHOTO` as row 3 and `goto="InGame Photo"`. `InGame Pause MP` and `MP Tournament Host` do not:
no photo mode in multiplayer. `GSDisableEntriesBitField="0x04"` strikes the row under Game Share.

**Entry from the finish.** `Race End Photo`'s `Race End Photo Redirect`, `forward="select"`, default
`InGame Photo`, `GSDisable="1"`. The redirect's task flips from `run` to `Pause`.

Rows:

| Row | Values | Default |
| --- | --- | --- |
| `Photo Camera Mode` (`ER_PHOTO_CAM`) | `OPT_INT` internal, `OPT_CLOSE` close, `ER_PHOTO_CAM_FRONT` front, `ER_PHOTO_CAM_ABOVE` above, `ER_PHOTO_CAM_TRACK` track | internal |
| `Photo Hud Display` (`ER_PHOTO_HUD`) | `FE_OFF`, `FE_ON` | `FE_OFF` |

Controls: CROSS takes (the redirect's forward), CIRCLE leaves. The camera:

| Row | Camera | Stick, no digital button held | Digital |
| --- | --- | --- | --- |
| internal | Mode 1: 5 ahead of the nose, looking forward | none | |
| close | Mode 4: 12 behind, 3 above | pitch and yaw | Triangle resets both |
| front | Mode 3: 12 ahead, 3 above, facing back | pitch and yaw | Triangle resets both |
| above | Mode 2: 6 behind, 2.5 above (`camera.md`: "above" is not above) | none | |
| track | Mode 7: the authored `Camera` node nearest the craft | X zooms `+0x270`, `[3, 40]` | L `-5`, R `+5` |

Pitch `+= stick * 0.05` per frame, clamped to **`[-1.0, 0.6]`** radians; stick up is negative, lifting
the eye. Yaw `+= stick * 0.05`, free, wrapped at `+-2 pi`. Confidence 90, live. Reset on a mode change
into close or front, and on leaving.

**There is no free camera on Pulse, and the track does not limit it.** The eye is a rigid offset from
the craft turned by two angles, or an authored node. `|eye - craft|` read 12.3693 at every pitch and
yaw, and at pitch `0.6` the eye sits under the road (the frame shows the hull from below). Confidence 90.
Track "zoom" is the field of view: `+0x270 = z` draws `z + 0.06 * (25.1 - z)` degrees at 112 units
(six values matched; the target is the node framing).

## HD/Fury

Rows, ranges and laws: [the HD page](../ghidra/functions/ps3-hdfury-eu/photo-mode.md#photomode_update).
Screen tree as Pulse's, plus `InGame Taking Photo Transition Screen`, `Pause`, `delay="1"`, so the
menu is gone a second before the shot. Rows, top to bottom: camera, HUD, speed blur, exposure,
saturation, effects, then aperture, focus or shutter for the chosen effect.

| Row | Range, step | Opens on |
| --- | --- | --- |
| Camera | internal, close, track | internal |
| HUD | off, on | off |
| Speed blur | 0 to 100, 5 | 0 |
| Exposure | 0 to 100, 1 | `(int)(33.3 - 33.3 g)`, `g` the race's grade: 21 on Vineta K, live |
| Saturation | 0 to 100, 1 | 20, which is neutral; live |
| Photo effects | off, depth of field; motion blur track and ship in race phases 2 and 3 | off |
| Aperture, focus | 0 to 100, 1 | 40, 20 |
| Shutter speed | 0 to 100, 1 | 20 |

A depth of field shot accumulates 7 more frames, a motion blur shot 120. `close` orbits freely on two
axes, tilts, zooms `[10, 100]` from 65, pans `+-5` (pan static only); `track` zooms `[3, 40]` from 20
and tilts; L1, R1 change the craft. Confidence 90 for the rows that were read live, 84 for the rest.

## 2048

A touch screen, `InGame VitaPhoto`, with three effect toggles (motion blur track, motion blur ship, depth
of field) and a preview before saving. HD's `InGame Photo` code remains in the binary, unreachable.
`MSC_PHOTO_CANCEL` is the Glory Moment, a front-camera picture sent to other racers, not photo mode.
`MSC_PHOTO_CHANGE_CAMERA` and `MSC_PHOTO_CYCLE_*` are text on the help image: the camera toggle is
button 5 between close and internal, the cycle is L and R. Its depth of field law differs from HD's.
Evidence: [the 2048 page](../ghidra/functions/vita-2048-eu-v104/photo-mode.md).

## Omega

**Checked, differs** against 2048 and HD
([page](../ghidra/functions/ps4-omega-eu/photo-mode.md)). HD's `InGame Photo`, not 2048's touch screen,
losing speed blur and saturation and gaining a separate motion blur list, and the shot is the console's
Share. Not run:
shadPS4 has no memory readout, so nothing here has a live number.

## Pulse PS2

`SCES_547.48` holds `InGame Photo`, `InGame Taking Photo`, `InGame Taking Photo saving` and `Photo Mode`
in the camera gate shape, and no `ER_PHOTO_*` state strings and no `.jpg`. PS2 has no save path in its
binary; its XML was not read. Confidence 70. Inherited from PSP if a PS2 port wants it.

## Split screen and multiplayer

Pulse: none, the row is absent from `InGame Pause MP` and `MP Tournament Host`. HD: **yes**, the `Photo`
block is on `InGame Pause Split Player 1` and `2`, the tournament pair and the Zone pair, from either
pad. Read, not run. Omega: same blocks, same screens. 2048: not checked in a split.

## What entering stops

**Pulse, measured (PPSSPP, Pulse USA, 90).** In `InGame Photo`, `Ship_UpdateCraft` and
`RaceManager_Update` take no hits, the craft stands to the bit, and only `Camera_UpdateSpectatorView`
runs. The same holds for `InGame Pause`, so photo mode is the pause plus a camera. The race resumes
where it stopped. Numbers: [Pulse EU, Live](../ghidra/functions/psp-pulse-eu/photo-mode.md#live-ppsspp-pulse-usa).
**Audio stops with it.** The engine note, the cue sequencer and the music callback take no hits in pause or
photo mode and the SAS voices stand still, while the mixer keeps ticking (80).

**HD, measured (RPCS3, 90).** Same: the craft stands to the bit, Continue resumes. Audio not measured.
[HD, Live](../ghidra/functions/ps3-hdfury-eu/photo-mode.md#live-rpcs3).

**Neither title's camera stops at the track.** Pulse: 12.3693 from the craft at every angle, under the
road at pitch `0.6`. HD: at orbit 1.34 the frame looks up through the road. A port needs no collision
to be faithful.

**Omega: not checkable** live, shadPS4 has no memory readout.

## What ours does

One photo mode for every title, on by default (Pure included, which has none in the original).
Code: `oag_raceplay::photo` (the camera), `oag_game::photo` (the screen and the files),
`oag-game`'s `session::photo` (the session half). Everything here that is not a number measured
off an original is **chosen, not measured**; where Pulse's and HD's laws differ the wider one is
taken, and a value one title measured is used on the rest unchanged.

**Entry.** The pause menu's PHOTO MODE row. **On Pulse that is the title's own pause screen**
(`EXT_PHOTO`, the disc's row on the screens that author one, ours after OPTIONS on a screen that
does not; [the pause menu](pause-menu.md)), opened by START or Escape. **On the other titles it is
the front end's main page over the race parked behind it** (Escape, or Back on a phone): the row is
parked off the page unless a race with
one view is held, as a choice with nothing to offer is (`Menu::offer`). The built-in mod
`oag-extras` attaches it, so it is on every title's main page (after OPTIONS' own rows; on HD and
Omega's strip it is the tab right of OPTIONS). On Pulse's `Race End Photo`, SELECT opens it once
the screen takes input (61 ticks after the finish), and leaving returns to the screen. Left out:
split screen (no single picture, no single pad: a follow-up), a LAN or online race (the server's),
and a build that cannot write a file (a page, a phone).

**While it is open** the race does not tick in the frame loop (`oag_game::photo::ticks`), whatever
state it is in, so its state hash does not move and the race resumes where it stood (a rewind moves
the race on screen through its past and puts it back, "Accumulated shots and rewind" below); the audio is stepped as under
the pause. The camera is a render-side override (`CameraOverride`), put back on leaving. Leaving spends
Circle, Cross, Start, Select and the shoulders, so none reaches the race, and **the tick that leaves is still the
mode's**: the frame loop read the devices into a snapshot before the mode spent its presses, so the leave tick used
to run `Race::tick` with CIRCLE as an absorb press
(`leaving_photo_mode_with_circle_hands_the_race_no_press`, tapped and held). Escape (and Back, and the
window losing focus, which does nothing then) leaves the mode and goes no further; the next Escape
opens the menus.

**Lenses** (the camera row): `internal` (5 ahead of the nose, Pulse's measured), `close` (12 behind
and 3 above, free to orbit; Pulse's measured offset, **chosen** on the others), `track` (the
circuit's authored `Camera` node nearest the craft, aimed at it; only where a circuit authors
one, left out of the cycle otherwise). The mode opens on `internal`, as both originals do.

**No visibility culling while it is open** (maintainer's decision: a shot must never lose
scenery). On every title the section set (`Race::visible_set`: Pulse, Pure, 2048 and Omega
circuits that author sections) is every section, including over the director's station camera,
and HD's chunk set is not built; the draws still go through the frustum test. Leaving the mode
restores the cull on the next frame. Test:
`tests::photo::photo_mode_draws_every_section_and_leaving_restores_the_cull` (red without the
rule); the director case is `tests::wreck_finished::photo_mode_over_the_directors_camera_is_not_masked_by_the_directors_station`.
Why: with the cull following the subject (the section a driver's index gives equals a full
nearest-sample scan's), PVS on against off dropped the background skyline on one Pulse subject
(`03_Track`, seed 7, tick 600, R once) and thin coplanar slivers on the rails. Before any change,
photo mode on another craft was culled to the seat's section where the camera could be placed and
drew everything where it could not (more than 96 samples away), so the symptom was not
reproduced as missing geometry on `main`. With the rule, PVS on is pixel-identical to PVS off
on Pulse (7 subjects) and HD (7 subjects). The weather's section and the boost lens follow the
framed craft (`Race::framed_slot`; an opponent's `Ship::segment` is never written, so its
driver's index is mapped through the racing line). 2048 and Omega: **checked, applies, not run
as a frame**: the same `visible_set`, and a circuit with no sections is never culled.
Frame time with the mode open, 1280x800, debug build, drive `screenshot` frames (CPU record and
submit, no GPU wait), twelve frames each, median: Pulse 7.3 ms, HD 6.6 ms (a quiet-machine
number it is not: single frames spread 3.7 to 12.6 ms).

**Controls**, all on one stick and the buttons a keyboard has (the abstract input has one stick):

| Input | Does |
| --- | --- |
| stick, arrow keys | orbit `close` (yaw and pitch, each free, wrapped at +-2 pi, 0.05 rad a tick: HD's law); zoom and tilt `track` |
| SELECT held + stick | `close`: zoom (up and down, field 10 to 100 from 65) and tilt (left and right) |
| SQUARE held + stick | `close`: pan, +-5 |
| SELECT tap | next lens |
| SQUARE tap | HUD on or off (off on opening) |
| L, R | previous, next craft, wrapping, skipping craft out of the race |
| TRIANGLE | reset the orbit, tilt, pan and fields |
| START | the help, then the effects panel, then neither |
| CROSS | take the shot (refused for 0.15 s after opening: HD's guard) |
| CIRCLE, right click | leave |

A click on the five chips along the top (camera, HUD, EFFECTS, TAKE PHOTO, BACK) does the same, and a
click with the help hidden shows it. A chip says its row and its value, and the value alone where the
row's word is too long for it (Pulse's own `PHOTO CAMERA MODE`). Not ported: HD's separate tilt triggers (L2, R2 are the airbrakes'
axes here), the right-stick zoom, the touch gestures of 2048 (a follow-up).

**The shot.** The frame the player sees, at the size they see it (`Framebuffer::output`, the
presentation target: the picture at window size, **before** the display grade and anything drawn
over the surface), written as a PNG to `photos/` under the user's data directory
(`<data dir>/oag/photos/oag-<title>-<unix seconds>-<n>.png`, `n` making the name unique). The
screen's text is drawn over the surface, so it is not in the file; the race's HUD is, only when
its row is on, and the end screens and track panel never are. The result message ("PHOTO SAVED")
shows and the mode stays open on the same camera. `crates/game/tests/photo_mode_ground_truth.rs`
checks the file is pixel-identical to a `--drive` screenshot of the same tick with the help
hidden, and of the same size.

**Words.** One id per row, `OAG_PHOTO_*` in `assets/ui/strings/english.toml`; where a title's disc
words a row the disc's own text is played instead (`oag_title::photo::Strings`: Pulse's
`ER_PHOTO_CAM` and `ER_PHOTO_HUD`; HD's and Omega's `MSC_PHOTO_HUD`, `_HELP`, `_ROT`, `_PAN`,
`_RESET` and `_SHIP`), checked to be in each disc's string table by the ground-truth test. The
rest is ours, with French named under `[untranslated]`.

**Checked against the other lineage.** 2048 and Omega both run this mode (driven to a race, the
mode opened from the pause menu, orbited, the HUD row on, a shot taken and read): ported, with
`photo_mode_ground_truth` on Pulse and a `--drive` walk on each of the five titles. `Race End Photo`
exists on Pulse only in this build (HD's end-race flow does not open on it), so SELECT at the finish
is Pulse's alone. 2048's own touch screen,
its preview step and its Glory Moment are not used.

**Picture controls** (`photo-effects`; `oag_post::photo_fx`, `oag_raceplay::photo::effects`). The
EFFECTS chip, or START once, opens a panel under the chips: exposure, saturation, speed blur, depth of
field, and aperture and focus while depth of field is on. With it up the stick moves its cursor (up and
down) and the row (left and right, repeating while held) and the camera stands still; a click on a row
puts the cursor on it and the arrows either side of the value change it. HD's rows, ranges and
laws on every title (`docs/ghidra/functions/ps3-hdfury-eu/photo-mode.md`, "`PhotoMode_Update`"):

| Row | Range, step | Opens on | Does |
| --- | --- | --- | --- |
| Exposure | 0 to 100, 1 | 33 | `2^((v - 33) * 0.03003)` on linear light, up is brighter |
| Saturation | 0 to 100, 1 | 20 | `v * 0.05`: 20 is the colour as it is, 0 is grey, 100 is five times |
| Speed blur | 0 to 100, 5 | 0 | HD's zoom ring held at `v * 0.01` (`oag_post::hd_zoom`), on every title |
| Depth of field | off, on | off | HD's focus curve over the scene's depth |
| Aperture | 0 to 100, 1 | 40 | the widest blur, `0.025 * v / 100` of the picture's height |
| Focus | 0 to 100, 1 | 20 | focus distance `d` from 3 to 493 (HD's curve); sharp from `d - c` to `d + 2c`, `c = d^0.55`, widest blur at `c^1.7` further out |

**The game's own motion blur is off while the mode is open, whatever the setting says**, on every
title (it smeared a frozen race whenever the camera moved); leaving puts the player's setting back, and
`the_games_motion_blur_never_reaches_a_photo` checks a shot is the same file with it `high` as `off`. The pass
runs on the finished frame (after the glow of HD, 2048 and Omega, before the HUD), so a shot and the screen show the same picture. **A control at its opening value is not
applied at all, and a pass that is not asked for builds nothing:** the shot with every control at its
opening is the one the mode saved before anything was touched, byte for byte
(`photo_mode_ground_truth::each_picture_control_changes_the_saved_shot_and_at_neutral_it_is_the_unprocessed_one`,
which also moves each control and reads the file: exposure up raises the mean luma, saturation 0 leaves no
colour, the speed blur lands at the edge and not the middle, depth of field softens the far field and
leaves the craft). The values live on the mode and die with it. After leaving, the picture is **byte-identical to the one
a visit that moved nothing leaves** (`a_race_through_photo_mode_hashes_the_same_at_every_tick_as_one_without`
compares the two, with the same state hash at every tick), and to the one the previous build, which had no
picture controls, leaves (a `--drive` run of both on Pulse: 0 of 1,175,040 pixels differ). **It is not
comparable byte for byte with a screenshot taken before entering:** the race keeps running into the pause
menu, and two screenshots of a race that never opened the mode, a frame apart, already differ in 350,590
pixels (503,404 across a visit, on the previous build and this one alike). Measured by
`--drive`, 2026-11-06.

**Chosen, not measured:**

- the **exposure curve**: HD's slider falls as its value rises and what its shader does with the number was
  never read, so the slider here runs the other way (up is brighter) and its step is stops of linear light;
- **`g` is 0** on every title: no title here models the race grade HD opens its exposure on
  (`(int)(33.3 - 33.3 g)`, 21 on Vineta K), so it opens on 33;
- the aperture's reach, and the gather (56 taps over the blur circle; a tap behind the pixel is held to the
  pixel's own radius, so a sharp craft keeps a hard edge and the blurred background does not halo over it);
- **speed blur is the boost pulse only.** HD's `Photo Blur` writes `v * 0.01` into both `FunkLayer+0x64`
  and `+0x68`, the ring's boost `E` and damage `P` pulses
  ([`FUN_00678f18`](../ghidra/functions/ps3-hdfury-eu/photo-mode.md#speed-blur-target-and-depth-of-field-targets), 85); `P`
  tints the streak red, so a photographer's speed blur here leaves it at zero;
- blur and depth of field are two independent rows (Omega's two lists), because the pass runs both;
- **HD's ring, in a dark scene, is faint** (a speed blur of 100 on the default race's opening grid: 38% of the
  pixels differ by more than 10 levels, 22 at the edges and 0.6 in the middle, against Pulse's far
  brighter streak); the ring is HD's own pass, drawn the way HD draws it;
- **the ring lives under its own tick** while the mode is open: the race does not step, so the history draw
  steps once per mode tick.

Where the pass is not run: the glow Pulse and Pure add **over the HUD** (their composite is after the HUD,
`bloom.md`) is outside it, so the exposure does not scale and the depth of field does not blur that glow.
GL is not built or run here (the WebGPU and Vulkan backends are). `cargo check -p oag-raceplay --target
wasm32-unknown-unknown` builds with the pass.

## Accumulated shots and rewind

Motion blur (track and ship), accumulated depth of field and a rewind, on the titles whose originals
have the first two (`oag_title::photo::Accumulation`: HD and 2048) and on every title for the rewind
([ADR-0087](../architecture/adr/0087-a-replay-and-photo-mode-share-one-timeline.md), stage 6;
code: `oag_raceplay::photo::{rewind, shot}`, `oag_raceplay::timeline::Keeper`, `oag-game`'s
`session::photo_rewind`).

**Rows.** In the effects panel: MOTION BLUR (off, track, ship), SHUTTER (shown with a blur chosen,
0 to 100, opens on 20) and REWIND (the race's clock at the moment on screen, `M:SS.s`) and PLAY (on or off). The first two
are on offer where the title's `photo_shot` is set, the last two in a race that has keyframes and was
entered from the pause menu (not from `Race End Photo`, which keeps its board), or over the replay
viewer. A click on the row's arrows moves them as the stick does.

**PLAY** plays the race's recording on from the moment REWIND shows, a tick a frame and silent, and
turns itself off at the live tick (or the end of the recording). Over a live race it steps the same
`Rewind` the scrub moves, so leaving still puts the race back to the bit
(`photo_mode_ground_truth::a_live_race_played_from_a_scrub_and_left_hashes_as_it_was`, and the fixture test
`playing_on_from_a_scrub_follows_the_recorded_run_to_the_live_tick`). Photo mode never offers a take-over.

**Over the replay viewer** ([ADR-0088](../architecture/adr/0088-the-replay-viewer-and-photo-mode-switch-both-ways.md)).
The viewer's PHOTO chip or SELECT opens the mode at the tick on screen. REWIND is then the viewer's seek over the whole
recording and PLAY plays the viewer on; leaving continues the replay from where the row stands, playing if it
was playing when the mode opened or PLAY is on. The blur rows are not offered over it (the viewer has no
keyframes of the race it moves) and the mode is refused in POSE VIEW.
`replay_viewer_ground_truth::photo_mode_and_the_viewer_switch_both_ways_at_the_tick_the_player_chose`.

| Title | Depth of field | Motion blur | Ship shutter | Evidence |
| --- | ---: | ---: | ---: | --- |
| HD | 7 frames | 120 | `0.05` of the track's | `ps3-hdfury-eu/photo-mode.md`: `this+0x100` and `+0x10c` |
| 2048 | 7 | 130 (`0x82`) | `0.05` | `vita-2048-eu-v104/photo-mode.md`, `VitaPhoto_Update` |
| Pulse, Pure | none | none | | no accumulating shot in the original |
| Omega | **checked, applies, not wired** | | | its `InGame Photo` has a motion blur list of its own (above); its frame counts and shutter were not read and shadPS4 has no memory readout |

The shutter is `(v * 0.01)^2 * 0.25` seconds, at most 15 ticks. **HD's ship factor is 2048's `0.05`,
chosen, not measured**: HD's two blurs were read as one shutter, and at `1.0` the craft crosses a held
camera in a fraction of the window and leaves no picture (read: the ship blur shot of the lane's
first run had no craft in it). At `0.05` the window is 0.75 tick at the longest shutter.

**A departure from the ADR's wording, named.** ADR-0087 decision 5 has the probe `Race` moved and the live
race neither ticked nor written. Drawing a second `Race` through the one `Scene` was not what the replay
viewer does (it moves the race on screen), so this does the same: the race on screen **is ticked and
written** by a rewind, and put back to the bit when the mode closes. The proofs are the hash and text ones
below; the price is that the view (camera spring, exhaust, particles) is rebuilt, not kept (after leaving a
rewind of 5 s on HD the picture one tick later differs from a visit that did not rewind by 0.9 percent MAE,
at five ticks 0.3, at twenty 0.1; before `Rewind::end` ran the pre-roll to the live tick it was 10.3, 7.3
and 1.4). The `Timeline` probe holds the keyframes and nothing is drawn from it.

**Rewind** is the race on screen moved through its own recording: the keyframe a pre-roll (60 ticks,
the viewer's own) before the tick, then the ticks up to it through `Race::tick`, drawn and silent, so
the camera spring and the exhaust have settled there. The live race is **put back to the bit** on
leaving: entering takes `Race::snapshot_sim()` and sets the recording aside
(`Race::pause_recording`), leaving restores both. `Keeper` keeps a keyframe a second from the race's
first tick (a few microseconds a second) and starts the second `Race` it seeks with only when a
rewind first asks. With no recording (a race that records nothing) a seek stops at the whole
keyframe at or before the tick and a motion blur is not offered. Every race records now (the REPLAYS
page's `[replay] record`, on by default), so a Single Race rewinds and blurs; with the setting off, or in a
modded race, the keyframe rule applies.

**What this rests on, the maintainer's two conditions** (kept by ADR-0088, which records this departure from decision 5). Photo mode never offers a take-over: it shows the
past and cannot play from it (a take-over is the replay viewer's, decision 9), and the race resumes exactly
where it was, to the bit (the proofs below).

**The shot** is made of frames, one drawn each frame of the window behind a progress bar over the
picture (not in the file), and **any input abandons it** and writes nothing (2048's own state 3).
Each frame is read back from the presentation target before the grade, as a single shot is, and the
frames are averaged in 16 bits a channel. A drive draws a frame only when asked, so `--drive` asks
for one while a shot is under way (`photo_job` in the state, `drawn/frames`).

- **Motion blur**, chosen, not measured: the frames are spread evenly over the shutter, from
  `shutter` seconds before the shot's tick to the tick, each drawn from the race at that moment,
  between the two ticks around it (`between_ticks`). **Track blur**: the camera rides the craft, so
  the scenery smears and the craft stays sharp. **Ship blur**: the camera holds the pose it has at
  the shot's tick and the craft smears.
- **Depth of field**, chosen, not measured: the same tick drawn 7 times with the eye moved across the
  lens (a centre sample and a ring of six) and aimed at the focus plane, so that plane stays sharp
  (`LensShift`, applied after the blend in `Race::view`). The ring's radius is half of the widest
  blur the single-frame pass draws at the aperture; each frame keeps the other half as that pass. The
  intent is a total as wide as the preview's; the saved shot was not compared with the preview.
- **Both on**: the frames of the blur, the lens samples taken in turn.

**Measured** (HD, Time Trial, Anulpha Pass, close lens, tick 702, debug profile (opt-level 2), the
discrete GPU, headless `--drive`, one frame drawn and read back per tick, the 1-minute load average 6.4 to
6.7 on 24 threads): a 120-frame track blur takes **1.97 s at 1280x800, 2.68 s at 1920x1080 and 3.20 s at
3840x2160**, 16 to 27 ms a frame. (An earlier run waited on the shot with `until`, which holds each frame to
16.7 ms: 2.03, 2.11 and 2.82 s, so the wait step sets the floor of that one. At 1280x800 the unpaced figure
sits on that floor too, so the cost there may be lower.) A 60 Hz window draws a frame a tick, so a shot
takes at least 2 s of wall clock there whatever it costs. 2048 (130 frames, 1440x816): 2.2 s paced. Depth
of field, 7 frames: 0.12 to 0.23 s. A rewind seek: 0.6 to 1.3 ms a push. `Setup::clone` at race build, for
the keeper: 0.8 ms on HD, 1.1 ms on 2048, 1.0 ms on Pulse, resident memory +0, +0 and +1.2 MiB (its large
parts are shared, not copied). Frames were read as pictures:
a track blur leaves the hull sharp and the scenery smeared (detail kept, by horizontal gradient:
hull 0.87, scenery 0.27), a ship blur the reverse (hull 0.49, scenery 1.00)
(`photo_mode_ground_truth::a_motion_blur_shot_smears_what_it_should_and_any_input_abandons_it`).

**Not done, named.** The shot is 8-bit frames averaged, so a dark gradient the blur flattens shows
contour bands; the single plain frame shows them in the same places (a rewound shot of the start
tunnel does), so they are the frame's. Rewind is not offered from `Race End Photo` or in a split
screen. Omega's blur and 2048's own depth of field law (`(a * 0.01)^2 * 900 + 100`; HD's focus curve
is used) stand as before. Also left open, as before: split screen, the photo
counter stat, touch gestures, saving on a page or a phone, HD's real `close` rest offset, and a shot at
the scene's own pixels (the file is the presentation target, upscaled from the scene drawn at its
configured render size: the dynamic resolution is held there while the mode is open).

**Checked against the other lineage.** Speed blur's target (HD) is the zoom ring `oag_post::hd_zoom`
already ports; 2048's `FUN_8112ea02` carries HD's focus formula (the two agree, as above) and Omega's two
effect lists are what the independent rows follow: ported, the same pass runs on all five (screenshots of
each on a race of its own).
