# Split screen

`oag-game --race --players N` (1 to 8, default 1) draws one race as `N` views of
one window: one `RaceSim`, one 60 Hz tick, `N` cameras. **An enhancement of this
project's, not a recovery**: the PS2 front end authors split-screen race rows
(`FE_RACE_CAM_SS`, `FE_RACE_CAM_SSH`) and the HUD sets carry `splitscreen_hud`
variants, but no capture of the original's split screen has been read, so every
choice below is **chosen, not measured** and carries no confidence score.

Code: `crates/render/src/viewport.rs` (layout), `crates/raceplay/src/viewpoint.rs`
(cameras), `crates/raceplay/src/scene/viewports.rs` (drawing),
`crates/game/src/split_screen.rs` (the flag). Tests: the three modules' unit tests
and the disc-backed `crates/game/tests/split_screen_ground_truth.rs`.

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

The full-screen HUD is **hidden while split** (`Race::hud_shown` is false): one HUD
for slot 0 across four views reads as every player's. The countdown, which is part
of the HUD draw, goes with it - the gantry's own `GO` board is still in the scene.

## Cost

Draws submitted per frame on Pulse's Talon's Junction, `--mode single_race`
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

## Open

- A per-player HUD laid out per view (the next piece of work).
- Slot `i > 0` is still flown by the grid's driver: only `World::primary_slot` is
  stepped from a snapshot, which is the remote-slot-input lane's work. Marking the
  slots `Controller::Local` is the declaration `--players` makes; the views already
  show them flying.
- Pad to slot assignment UI, and the front-end entry (the PS2's `Team SelectionSplit`
  and `Track CreationSplit` screens): `--players` is a command-line switch only.
- Per-team camera rigs, per-view bloom (an origin in `oag_post::bloom`), motion blur
  and the temporal upscaler under a split, and a check of HD's and Omega's chains.
- Pulse PS2, Pure and the other titles are the same code path and unchecked; the
  2048 and Omega check is **checked, applies, not wired**: the viewport and camera
  code knows no title, and only Pulse's frames were looked at.
