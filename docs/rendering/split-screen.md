# Split screen

`oag-game --race --players N` (1 to 8, default 1) draws one race as `N` views of
one window: one `RaceSim`, one 60 Hz tick, `N` cameras. **An enhancement of this
project's, not a recovery**: the PS2 front end authors split-screen race rows
(`FE_RACE_CAM_SS`, `FE_RACE_CAM_SSH`) and HD, 2048 and Omega ship `splitscreen_hud`
layouts (see "HUD" below), but no capture of the original's split screen has been
read, so every choice below is **chosen, not measured** and carries no confidence
score, except where a section says it is the disc's own data.

Code: `crates/render/src/viewport.rs` (layout), `crates/raceplay/src/viewpoint.rs`
(cameras), `crates/raceplay/src/scene/viewports.rs` (drawing),
`crates/game/src/split_screen.rs` (the flag and the lobby), `crates/input/src/lobby.rs`
and `crates/input/src/pad.rs` (who plays which slot), `crates/ui-screens/src/join.rs`
and `crates/game/src/main/session/join.rs` (the join screen),
`crates/game/src/hud_views.rs` and `crates/raceplay/src/slot_hud.rs` (the per-view
HUD). Tests: the modules' unit tests, `crates/game/tests/pad_join_end_to_end.rs` and
the disc-backed `crates/game/tests/split_screen_ground_truth.rs` and
`split_hud_ground_truth.rs`.

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

`Scene::render_viewports` calls the ordinary `Scene::render` once per view. Five
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

5. **HD's and Omega's post chains run once, after the last view.** Both draw the
   scene into a linear float target and then *encode* it into the frame with a pass
   that clears the whole frame, sampling the linear target from its top-left whatever
   the view's origin is. Run once per view (as `Scene::render` first did), each
   encode wiped every earlier view and every later view was graded from the first
   view's pixels: a two-player HD race drew a black top view and the bottom view
   showed the first view's (slot 0's) picture. 2048 draws through HD's chain and had the same black
   view; Omega's tone map does not clear, so its bottom view showed the top view's
   picture twice. The views now only fill the linear target, and
   `Scene::encode_split_post` (`crates/raceplay/src/scene/viewports.rs`) runs the
   chain once over the whole area, which is exactly what one view passes.
   **Chosen, not measured**: one exposure and one glow for the whole picture (glow
   bleeds a few pixels across the 2 pixel gap); HD's boost zoom streak is not drawn
   (it holds the primary craft's pulse) and Omega's distortion offsets are not drawn
   (the pass reads a depth attachment only the last view still has). Both open.
   `crates/game/tests/split_screen_ground_truth.rs` pins HD two and four views,
   Omega and 2048 two views: every rectangle drawn, no two the same. 2048 and Omega
   share the path: **ported**, each with its own test.

## HUD

**One HUD per view, from that view's slot.** `Race::readout_for(slot)` builds the
`Readout` of the view that follows `slot` (lap, place, lap times, speed, shield,
held weapon, the Eliminator column, Head2Head's gap, wrong way); `readout()` is
`readout_for` of the primary slot, so one view is what it always was. The bars that
animate from the shield (flash and blink, HD's whole-percent twin, 2048's lagging
bar and thrust chase, the Pilot Assist indicator) live per slot in
`crates/raceplay/src/slot_hud.rs` and advance for every viewed slot, for no other
slot, and for the primary exactly as before.

`oag_game::hud_views::draw` draws every view's HUD in **one** list, painted in the
passes one HUD takes. The scene needs a submit per view because
`queue.write_buffer` lands before the submission; the HUD renderers write their
uniform and quad buffers the same way, so one draw per view in one encoder would
show every view the last one's numbers. Each view's draw list is moved into its
rectangle by an `oag_hud::Placement` and anything outside the view is dropped, so
a layout that authors a widget off-screen cannot paint over a neighbour.

Where the layout comes from, per title and view count:

| | Source | |
| --- | --- | --- |
| HD / Fury, two views, Single Race, Tournament, Head2Head or Eliminator | the disc's own `splitscreen_hud` layouts, one per player | **measured** (below) |
| everything else (any other count, mode or title) | the title's whole-screen layout, scaled uniformly to fit the view, aspect kept, centred | **chosen, not measured** |

**The authored layouts** (HD, `oag_hd::hud::SPLIT`). `/data/xml/splitscreen_hud/`
holds `splitscreen_hud_{0,1}.xml` and `splitscreen_elimination_hud_{0,1}.xml`,
composed like any HD HUD (16 and 15 files, 109 and 108 sprites, no include missing,
nothing skipped). Both are authored in the whole 1920x1080 screen, and the `_1` is
the `_0` moved down exactly 540: the extents of their widgets differ by 540 in y.
So player `i` is a top-and-bottom half with its layout sitting in it, which is the
shape `oag_render::viewport::layout` already cuts for two views. A `vert_` twin of
each does the same with 960 in x for a left and right split, and the family covers
two players, so it is shipped and not wired. The reticle is authored view-relative
(`splitscreen_sights.xml` places its models off screen and the runtime places
them), so the race projects it into a 1920x540 view and the draw moves it down by
the view's own offset. **2048 and Omega ship the same four files**
(`hd_2048_and_omega_ship_the_same_split_layouts_540_apart`): 2048 as
`Data\XML\SplitScreen_hud\SplitScreen_{,Elimination_}HUD_{0,1}.xml` in
`data.psarc`, Omega as `Data/xml/splitscreen_hud/splitscreen_{,elimination_}hud_{0,1}.xml`
in `data00.psarc`. All three titles compose them to the same 109 sprites and 63
labels (108 and 67 for Elimination), widest 1895 (2208), lowest 734 and 1274 (778
and 1318): the second player is 540 lower in each. **Omega: checked, applies, not
wired** - its HUD grid is HD's 1920x1080, so `oag_omega::hud::LAYOUTS` could carry
the same row; no Omega race could be looked at on this machine's extraction (the
short-read note in `omega_race_ground_truth`). **2048: checked, differs** - the
files are authored in HD's 1920x1080 while 2048's HUD space is the Vita's 960x544,
so its copy would need scaling first (as `oag_hud::assist::splice` scales a
fragment). The PS2 pressing of Pulse names only
`Arcade_HUD.xml` and `TimeTrial_HUD.xml` as HUD paths in its executable and has no
split layout file (**checked, differs**: its split is the camera's own aspect
change, `Camera_SubmitScene`'s viewport count, see
[aspect-ratio.md](../ps2/aspect-ratio.md)); Pure and the PSP Pulse author none.

**Chosen, not measured:**

- **The scaled whole-screen layout** for every case with no authored one: uniform
  scale to fit, aspect kept, centred. In a two-up strip (about 3.5:1) the HUD is
  height-limited and sits in the middle of the strip, so its corner widgets are not
  at the strip's corners. The layout source for those cases is meant to be data
  (a TOML layout per view count and aspect through the fallback layer); this is the
  stand-in until it exists.
- **One answer per screen for the intro**: the HUD shows for the primary's flyby and
  `hud_delay`, for every view, since the other views have no flyby. The
  track-description panel draws into the first view's rectangle.
- **The countdown** (the HUD overlay's fallback, drawn only where the circuit has no
  gantry) is drawn once, centred on the screen. The gantry's own count is in the
  scene and so in every view.
- **No message lines, split gap or time-trial pace on a view that is not the
  primary's.** They compare laps against the primary's stored best and push a
  `MESSAGE` cue; per-person records are not read yet. The Zone colour grade is one
  per scene, so every view reads the primary's speed class.
- **The reticle of a view that is not the primary's is display only.** It is
  projected through that slot's own chase camera and field of view
  (`SlotHud::sight`, fed by `Race::sight_target(slot)`) at the title's HUD aspect, so
  a narrow view sees a slightly wider vertical field than it assumes. The weapons
  code reads the primary's `RaceView::sight`, so another person's fire does not wait
  for their lock (open).

## The end of the race

A person who crosses the line keeps their view until the last person does: the lap
counter holds at the target, the clocks at their crossing, and the layout's last
message line reads `FINISHED`. When the last person finishes one view is drawn
(`Race::views_drawn`) under the end screens, which name every person (`PLAYER 1`,
`PLAYER 2`) in Pulse's own multiplayer table or HD's grid, bank no loyalty and write
no record. The authored data (the PS2's `EndRace Results Split`), the chosen
rules and what is open are in [split-end-race.md](../ui/split-end-race.md).
`--autopilot` flies every local seat, so `--players 2 --autopilot --mode single_race`
reaches the end headless.

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
- **A local slot with no device on it is flown by the grid's driver** (chosen, not
  measured; `split_screen::follow_devices`, once a tick in the windowed session). A
  race ends when every person-flown slot has finished, so a slot nobody can steer -
  a pad unplugged mid-race, or `--players 4` on a machine with one pad - would hold
  the end-of-race screens for ever. The slot goes back to `Controller::Ai` and returns
  to `Local` when a pad takes it again; remote slots are never touched. It writes
  `World::controllers` from outside the tick, so a replay or a network client would
  have to carry it as an input (open). Headless captures never run it.
- **A keyboard that did not join still drives slot 0**, merged with that seat's
  pad: it has to reach the pause key and the menus. A keyboard that did join holds
  its seat, so no pad that connects later takes it.
- **A pad plugged in when the join screen seats its players, and not joined, is
  benched** until it disconnects (`Assignment::bench`): a pad on the desk does
  not take a seat the instant the race starts.
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

A split screen writes **no result and no ghost** (`records.toml` and the ghost
store are for the one person the result is about), and it **fields opponents
whatever the mode**, so Time Trial, Speed Lap and Zone are reachable as split races
from the menus. Restarting a race from the 2048 end-of-race screen, which skips the
menus, keeps the split.

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
assignment's rules, are `oag-input`'s unit tests. Not checked, and the lead's to run with two pads: the
session wiring has no test of its own (the `seat_split_race` call in the race
build, the merge on returning to the menus, the lobby dropping on Track Select's
back), and the windowed walk only reached Track Select with the keyboard seated
alone. To run: `just play data/images/pulse-psp-eu.chd --no-audio`, SPLIT SCREEN,
press cross on each pad, START, pick a track and ship, and steer each pad; then
leave the race and check a pad moves the menu cursor again.

## Open

- **A launch must set `race::Options::views`** (the CLI's `--players` does, in
  `prepare.rs`) for any race that will call `split_screen::apply(.., 2)`: without it
  HD reads no authored layout and the reticle projects into the full screen.
- **HD draws only the last view's scene** in a `--screenshot` of `--players 2` or
  `4` (the earlier views are black), on `main` as well: the post chain writes
  `view` per call and was never verified under a split. The HUDs are drawn.
- **The HUD overlay has no per-view scissor.** `Placement::clip` drops a draw that
  lies wholly outside its view, so a sprite or label straddling a view's edge still
  paints across it by its overhang.
- A split HUD layout as data (a TOML layout per title, view count and aspect, with
  the authored one first), for Pulse, Pure, the PS2 and the counts and aspects the
  authored family does not cover. HD's Wo3 and 2097 skins, its Zone layout and
  the `vert_` left-and-right family are shipped and not wired; 2048 and Omega's are
  the same data and not wired. Per-person messages and records, and a lock that
  gates another person's fire.
- A second player's ship choice: the join screen goes on to one Track Select and
  one Ship Select, so only slot 0 picks a team (the PS2's `Team SelectionSplit`
  carries a set per player).
- Two real pads: the assignment is unit-tested over stand-in ids and the keyboard
  path runs end to end, but no second pad has been plugged in.
- Per-team camera rigs, per-view bloom (an origin in `oag_post::bloom`), motion blur
  and the temporal upscaler under a split, and a check of HD's and Omega's chains.
- ~~The results scoreboard and the end-race flow still key on slot 0 and end every
  view when slot 0 finishes.~~ **Done, 2026-10-13**: see "The end of the race". Open
  from it: the PS2's authored `EndRace Results Split`, per-person rewards and
  records, a live walk with two real pads.
- Resolution holds its scale while split: the scene pass is not timed (timing one
  view of `N` would read as a cheap frame), so dynamic resolution gets no reading.
- `--players N > 1` **changes `RaceSim::state_hash`**, because `World::controllers`
  is hashed; the cameras, views and draw do not.
- Not a 2048 or Omega finding: the viewport and camera code is title-agnostic, but
  only Pulse's frames were looked at, so Pure, Pulse PS2, HD, 2048 and Omega are
  unchecked under a split.
