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
`crates/game/src/split_screen.rs` (the flag), `crates/game/src/hud_views.rs` and
`crates/raceplay/src/slot_hud.rs` (the per-view HUD). Tests: the modules' unit
tests and the disc-backed `crates/game/tests/split_screen_ground_truth.rs` and
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
- Slot `i > 0` is still flown by the grid's driver: only `World::primary_slot` is
  stepped from a snapshot, which is the remote-slot-input lane's work. Marking the
  slots `Controller::Local` is the declaration `--players` makes; the views already
  show them flying.
- Pad to slot assignment UI, and the front-end entry (the PS2's `Team SelectionSplit`
  and `Track CreationSplit` screens): `--players` is a command-line switch only.
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
