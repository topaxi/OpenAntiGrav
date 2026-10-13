# Split screen

`oag-game --race --players N` (1 to 8, default 1) draws one race as `N` views of
one window: one `RaceSim`, one 60 Hz tick, `N` cameras. **An enhancement of this
project's, not a recovery**: the PS2 front end authors split-screen race rows
(`FE_RACE_CAM_SS`, `FE_RACE_CAM_SSH`) and the HUD sets carry `splitscreen_hud`
variants, but no capture of the original's split screen has been read, so every
choice below is **chosen, not measured** and carries no confidence score.

Code: `crates/render/src/viewport.rs` (layout), `crates/raceplay/src/viewpoint.rs`
(cameras), `crates/raceplay/src/scene/viewports.rs` (drawing),
`crates/game/src/split_screen.rs` (the flag and the lobby), `crates/input/src/lobby.rs`
and `crates/input/src/pad.rs` (who plays which slot), `crates/ui-screens/src/join.rs`
and `crates/game/src/main/session/join.rs` (the join screen). Tests: the modules'
unit tests, `crates/game/tests/pad_join_end_to_end.rs` and the disc-backed
`crates/game/tests/split_screen_ground_truth.rs`.

## Layout (chosen, not measured)

`oag_render::viewport::layout(count, area)` cuts `area` into two rows, the top
holding `count / 2` views and the bottom the rest, each row divided evenly, with
a 2 pixel gap. One view is `area` itself, untouched.

| Views | Top row | Bottom row |
| ---: | ---: | ---: |
| 1 | 1 (the whole area) | - |
| 2 | 1 | 1 |
| 3 | 1 | 2 |
| 4 | 2 | 2 |
| 5 | 2 | 3 |
| 6 | 3 | 3 |
| 7 | 3 | 4 |
| 8 | 4 | 4 |

Two players split top and bottom at full width: a wide strip keeps the authored
vertical field and sees more to the sides, which suits a forward-looking racing
camera better than two tall slivers. View `i` follows grid slot `i`, left to right
then top to bottom. Every count tiles the area exactly (a unit test over 1 to 8
and three odd sizes), and 1, 2, 4 and 8 are screenshot-verified on Pulse.

**Field of view needs no new code.** The aspect comes from the view's rectangle and
`oag_render::camera::fit_vertical_fov` already keeps the authored vertical field at
or above the PSP's 16:9-ish shape and widens it below, so a wide strip sees more to
the sides and a quadrant keeps the full-screen shape.

## Cameras

View 0 is the player's own camera and takes the code path it always took: one view
is byte-identical to before (four Pulse frames compared with `cmp`). Views `i > 0`
each have a `Chase` spring in `RaceView::viewpoints`, advanced on the tick right
after the player's, from slot `i`'s craft. Camera state is view-side: no tick reads
it back, and a unit test pins `RaceSim::state_hash` equal with and without a split
while every camera is read.

The draw selects a view with `Race::set_viewpoint(slot)` (a `Cell`, because
`Scene::render` takes `&Race`) and every camera accessor - view, field of view,
visibility sections, LOD eye, the chunk set - follows it. `Race::reset_viewpoint`
puts the player's back. Chosen, not measured:

- Every extra view flies the **player's own `<ExternalCameraFar>` rig**, whatever
  team its craft is: the load keeps one team's camera blocks.
- An extra view has **no impact shake**, no intro flyby, no finish or destroy camera
  and no cockpit view. The speed widen of its field follows the viewed craft; the
  boost kick does not.
- Extra views are blended between ticks by the draw alpha like the player's, with
  their own previous pose, and re-snap on a respawn.

## Drawing

`Scene::render_viewports` calls the ordinary `Scene::render` once per view. Four
things make that more than a loop:

1. **One encoder and one submit per view.** `Scene::render` writes every
   drawable's uniforms with `queue.write_buffer`, which lands before the
   *submission's* commands. Two views in one encoder would both draw with the last
   view's matrices.
2. **Only the first view clears.** The scene pass clears the whole attachment
   whatever the viewport is. Later views load the colour; depth is cleared each
   time. Under MSAA the resolve rewrites the whole target
   (`crates/render/tests/msaa_resolve_viewport.rs`), so the multisampled colour is
   what is loaded, and `crates/render/tests/msaa_load_keeps_view.rs` pins that a
   loading pass keeps an earlier view.
3. **No motion blur, no camera jitter, no temporal upscaler input.** The motion
   state and the upscaler's frame record hold one camera's history.
4. **No Pulse bloom.** `oag_post::bloom`'s `prepare` reads the glow mask from the
   top-left of the target whatever the viewport's origin is, so a view elsewhere
   would glow from the first view's pixels. Skipped rather than drawn wrong.

Wipeout HD's and Omega's post chains are not verified under a split.

## HUD

The full-screen HUD and the track panel over the pre-race flyby are **hidden while
split** (`Race::hud_shown` and `Race::track_panel_progress`): one HUD
for slot 0 across four views reads as every player's. The countdown, which is part
of the HUD draw, goes with it - the gantry's own `GO` board is still in the scene.

## Cost

Draws submitted per frame on Pulse's default circuit, `--mode single_race`
(frame counts from the capture's own `frame at tick` line, not a GPU time):

| Views | Tick 300 | Tick 1800 |
| ---: | ---: | ---: |
| 1 | 465 draws, 31,970 triangles | 276, 25,007 |
| 2 | 955, 63,916 | 1,254, 89,572 |
| 4 | 2,738, 177,383 | 2,430, 185,365 |
| 8 | 6,507, 415,226 | 7,172, 552,440 |

Cost scales with the views' own content: the player's view at the start line is a
cheap one and the others are in the open, so four views cost 5 to 9 times one.
There is no 8-up frame budget yet.

## Who plays which slot (chosen, not measured)

Every rule here is ours. The PS2 build takes its players from the pads in port
order and has no join step to read; its authored flow (`FE_RACE_CAM_SS` and
`FE_RACE_CAM_SSH` route to `Split ScreenV`/`Split ScreenH`, and `Team SelectionSplit`
carries both players' widget sets, `docs/formats/race-setup.md`) is a camera choice
and a per-player team pick, not a device join, so it is not what the entry below
follows. Confidence scores do not apply.

**The assignment** is `oag_input::pad::Assignment`, generic over the pad's id so
the rules are unit-testable (`gilrs::GamepadId` has no public constructor):

- `Seating::Merged` is the default and the old behaviour exactly: every pad and
  the keyboard drive slot 0, a pad with no entry of its own too.
- `Assignment::seated(n)` is a split screen of `n` people. The keyboard drives
  slot 0. A pad nobody assigned takes the lowest of slots `0..n` no other pad
  holds, in the order the backend lists them, so the first pad shares slot 0 with
  the keyboard and the next pads take slots 1, 2, and so on.
- It runs on every poll (`Assignment::reconcile`), not on connect events: a pad
  plugged in mid-race takes the lowest free slot, and a pad that went away frees
  its slot, whose craft then flies on an empty input.
- **A pad past `n`, with no slot free, drives nothing.** Merging it into slot 0
  would put two people on one craft.
- `--players N` seats the default way (`Controls::seat_default`). Leaving a split
  race for the menus merges again (`Controls::merge_devices`), so any device
  drives the one menu cursor; a race parked behind the pause menu keeps its
  seats.

**The join screen** is the second way to seat people. Entry: a SPLIT SCREEN row
under RACEBOX in the built-in `oag-extras` mod (an action, `split_screen`), on
every title that shows the extras. `oag_input::lobby::Lobby` is the model:

| Device | Press | Does |
| --- | --- | --- |
| no seat | confirm (cross) | takes the next seat |
| seated | back (circle) | gives its seat up, the later seats move down |
| no seat | back (circle) | leaves the screen |
| seated | start | begins, with the seated count |

Seat `i` flies slot `i`, so the seated slots are always `0..count`. A pad that
disconnects while the screen is open loses its seat. The keyboard, the mouse and
touch are one device (a click arrives as a keyboard tap): a click on the empty
row joins it, a click on a taken seat gives that seat up, START and BACK are
rows, and the secondary button backs out. Escape is the keyboard pressing back.
Per-device presses come from the backend's own press events for pads
(`Pad::take_presses`, so a press shorter than a tick counts and two pads are told
apart before either has a slot) and from the keyboard's newly pressed bits
(`Controls::device_presses`).

START goes on to the race box's own Track Select and Ship Select, as the RACE
page's START does; backing out of Track Select drops the lobby. The race build
drains it (`split_screen::seat`): slots `0..count` become local, the screen is
cut into `count` views and each device goes on the slot it joined. One seat, or
none, is an ordinary race and changes nothing. A split race fields opponents
whatever the mode, as `--players` does, because a split of one craft is one view.

`--menu-page join[:N]` draws the screen with `N` seats taken (the keyboard first,
then pads named `GAMEPAD n`), off the same labels and string table as the live one.
Verified on Pulse (EU): the row between REMIX and OPTIONS, the screen empty, with
one seat and with three, START opening Track Select, and Escape from Track Select
back on the main menu.

**What is checked.** `crates/game/tests/pad_join_end_to_end.rs` runs the real
`Controls` -> `Race::tick` path with the keyboard assigned to slot 1: slot 1's
craft turns and slot 0's does not, and the same key on slot 0 does the reverse.
A pad reading on slot 1 reaching slot 1's snapshot, and the lobby's and the
assignment's rules, are `oag-input`'s unit tests. Not checked: two real pads on
real hardware (no pad on the test host), and the windowed flow with a second
device.

## Open

- A per-player HUD laid out per view (the next piece of work).
- A second player's ship choice: the join screen goes on to one Track Select and
  one Ship Select, so only slot 0 picks a team (the PS2's `Team SelectionSplit`
  carries a set per player).
- Two real pads: the assignment is unit-tested over stand-in ids and the keyboard
  path runs end to end, but no second pad has been plugged in.
- Per-team camera rigs, per-view bloom (an origin in `oag_post::bloom`), motion blur
  and the temporal upscaler under a split, and a check of HD's and Omega's chains.
- The results scoreboard and the end-race flow still key on slot 0 and end every
  view when slot 0 finishes.
- Resolution holds its scale while split: the scene pass is not timed (timing one
  view of `N` would read as a cheap frame), so dynamic resolution gets no reading.
- `--players N > 1` **changes `RaceSim::state_hash`**, because `World::controllers`
  is hashed; the cameras, views and draw do not.
- Not a 2048 or Omega finding: the viewport and camera code is title-agnostic, but
  only Pulse's frames were looked at, so Pure, Pulse PS2, HD, 2048 and Omega are
  unchecked under a split.
