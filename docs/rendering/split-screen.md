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
`crates/game/src/split_screen.rs` (the flag and the lobby), `crates/game/src/main/session/picker/seat.rs` (Ship Select per seat), `crates/input/src/lobby.rs`
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

**Two views can be cut left and right instead** (`oag_render::viewport::Cut::Vertical`,
2026-10-13, `hd-authored-split-setup`): two full-height columns with the same 2 pixel gap.
**The choice between the cuts is the disc's** (HD's `SplitScreen` list, below); the
geometry is **chosen, not measured**. A cut means something for two views only: any other
count is cut in rows whatever was asked (`Cut::for_views`). `Race::cut` and
`Race::view_rects` are the one answer both the scene (`Scene::render_viewports`) and the HUD
(`hud_views::view_draws`, `placements`, `panel_viewport`) read, so a vertical scene cannot
get a horizontal HUD; the cut is view state, outside `RaceSim`, and the state hash does not
hold it. `--split-cut horizontal|vertical` sets it for `--players 2` on the command line;
on HD the race box's row sets it. A column is about 0.9 as wide as it is tall at 16:9, so
`fit_vertical_fov` widens the vertical field exactly as it does for any narrow view, and the
camera needs nothing else.

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
- **The flyby plays full screen and the split starts when it ends**
  (`Race::views_drawn` is one while `Race::in_intro`), with the track panel over the
  whole screen as in a one-player race. The extra cameras keep stepping through the
  flyby, so each is settled on its craft when the split appears. A flyby that never
  begins (a headless capture without `--intro-ticks`) leaves the split alone.
  Reported from play by the maintainer 2026-10-13; checked by
  `split_screen_ground_truth::the_flyby_plays_full_screen_and_the_split_starts_after_it`.
- Extra views are blended between ticks by the draw alpha like the player's, with
  their own previous pose, and re-snap on a respawn.

## Drawing

`Scene::render_viewports` calls the ordinary `Scene::render` once per view. Six
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

5. **What does not depend on the camera is done once, by the first view**
   (`Layer::Next` skips it, 2026-10-13, `split-render-cost`): the two shadow maps, the
   shadow geometry's gather (each view still uploads it with its own matrix), the
   scenery's animation clocks and the advert cards. They are functions of the craft
   and the race clock, so every picture is byte-identical to doing them per view, and
   they were more than half of a view's CPU on Pulse. See "Cost".

6. **HD's and Omega's post chains run once, after the last view.** Both draw the
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

Where the layout comes from. **One function answers it per view**
(`oag_hud::split_hud::source`, called by `hud_views::view_draws`), in this order, and
the first source that has a layout for the view's count, mode and shape wins:

1. **A player's override**: `Data\XML\SplitHud_<Title>.toml` under
   `overrides/<title>/` (see [asset-layers.md](../architecture/asset-layers.md#a-split-screen-hud-layout-file)).
2. **The disc's authored layout**: HD, Omega (and 2048, unwired), two views, Single
   Race, Tournament, Head2Head or Eliminator (**measured**, below).
3. **This project's file**, from the fallback layer: Pulse (PSP and PS2 pressings, one
   file), Pure and HD ship one in `assets/fallback/Data/XML/SplitHud_*.toml`, **chosen,
   not measured**.
4. **The whole-screen layout scaled** uniformly to fit the view, aspect kept, centred:
   the last resort, **chosen, not measured**. 2048 and Omega's non-authored cases are
   still here (no file written for either yet).

| | Source | |
| --- | --- | --- |
| HD / Fury and Omega, two views, Single Race, Tournament, Head2Head or Eliminator | the disc's own `splitscreen_hud` layouts, one per player | **measured** (below); Omega **ported, looked at** |
| Pulse (PSP, PS2), Pure and HD otherwise, any count 2 to 8 and every view shape | `SplitHud_Pulse.toml`, `SplitHud_Pure.toml`, `SplitHud_HD.toml` | **chosen, not measured** |
| 2048; Omega's other cases | the whole-screen layout scaled to fit the view | **chosen, not measured** |

**The file** (`schemas/v1/split-hud.tosd`, `oag_hud::split_hud`) is a list of
`[[layout]]` rules, each with `views`, `aspect` (a *view's* width over height, so
three, five and seven views, whose views differ in shape, pick a rule per view) and
`modes`, and its `[[layout.group]]`s: a list of the title's own widget names (a
trailing `*` is a prefix), the point of their authored bounding box to pin
(`anchor`: a corner, an edge midpoint or the centre), an `offset` in fractions of the
view, and a `scale` relative to the whole-screen HUD fitted to the view. It **only
places the title's own widgets**; a widget no group names is not drawn in a split,
which is how 5 to 8 views drop the secondary ones. Every number is chosen for
legibility at the size a player sees a view, never measured. Offsets and scales carry
no grid, so one file serves the PSP's 480x272 and the PS2's 640x448.

What each shipped file draws (lap, place, the held weapon, speed and shield, time
where there is room):

| View shape (a view's width/height) | Where it occurs | Pulse and Pure | HD |
| --- | --- | --- | --- |
| 2.4 and wider (`strip`) | two views, the top of three | every group at its corner, larger | every group |
| 1.4 to 2.4 (`landscape`) | four views, the top of five | every group | every group |
| 1.0 to 1.4 (`square`) | five to seven views' lower rows | lap, place, speed, shield (Pulse only: Pure draws none), held weapon, current time | no best-time group |
| under 1.0 (`portrait`) | seven and eight views' lower rows | the same, larger | the same, larger |

Each group is its own draw (`ViewDraw`): the layout is cut once at load into one
sub-layout per group (`SplitHudAssets`), and the reticle, which is not a widget of any
group, is one more draw per view, placed as the whole-screen HUD would be. A layout with
no widgets draws nothing whatever the readout holds (`a_layout_with_no_widgets_draws_nothing_from_a_full_readout`),
which is what keeps a group from drawing the whole readout again.

**Per-view scissor.** `Frame` carries a `Span` per view (where its draws start in the
four lists and its rectangle), and `Renderer::set_scissors` turns them into one
scissored draw per view in the one pass, so a widget straddling a view's edge is cut at
it rather than painting over the neighbour. A whole-screen frame has no spans and is
drawn as it always was.

**The authored layouts** (HD, `oag_hd::hud::SPLIT`). `/data/xml/splitscreen_hud/`
holds `splitscreen_hud_{0,1}.xml` and `splitscreen_elimination_hud_{0,1}.xml`,
composed like any HD HUD (16 and 15 files, 109 and 108 sprites, no include missing,
nothing skipped). Both are authored in the whole 1920x1080 screen, and the `_1` is
the `_0` moved down exactly 540: the extents of their widgets differ by 540 in y.
So player `i` is a top-and-bottom half with its layout sitting in it, which is the
shape `oag_render::viewport::layout` cuts for two views. **The `vert_` twins are wired
(2026-10-13)** for a left-and-right cut: `vert_splitscreen_hud_{0,1}.xml` (`DATA03`) and
`vert_splitscreen_elimination_hud_{0,1}.xml` (`DATA00`), each pair one view (960) apart in x
and the same height (`oag_title::hud::VerticalLayouts`, `oag_hd::hud::SPLIT.vertical`; the
four compose with no include missing, 109 sprites each in the arcade pair, the layout
reaching y 1,048 against the horizontal family's 734). `SplitAssets` carries the step to
the next player (`step`, `region(slot)`): down one view's height for the rows, right one
view's width for the columns, and the reticle's view is 960x1080. The arcade vertical
file's own comment asks for "separate sights" and includes the whole-screen sights, so a
vertical view's reticle is the arcade one placed into the column. Single Race, Tournament,
Head2Head and the Eliminator read these on a vertical cut; Time Trial, Speed Lap and Zone
(no authored pair) draw this project's `SplitHud_HD.toml` by view shape, as they do in rows.
The family covers two players, so any other count never reads it. The reticle is authored view-relative
(`splitscreen_sights.xml` places its models off screen and the runtime places
them), so the race projects it into a 1920x540 view and the draw moves it down by
the view's own offset. **2048 and Omega ship the same four files**
(`hd_2048_and_omega_ship_the_same_split_layouts_540_apart`): 2048 as
`Data\XML\SplitScreen_hud\SplitScreen_{,Elimination_}HUD_{0,1}.xml` in
`data.psarc`, Omega as `Data/xml/splitscreen_hud/splitscreen_{,elimination_}hud_{0,1}.xml`
in `data00.psarc`. All three titles compose them to the same 109 sprites and 63
labels (108 and 67 for Elimination), widest 1895 (2208), lowest 734 and 1274 (778
and 1318): the second player is 540 lower in each. **Omega: ported, looked at** -
its HUD grid is HD's 1920x1080, so `oag_omega::hud::SPLIT` is the same row, and
`split_hud_ground_truth` composes the roots that row names and checks them against
HD's; a two-view Single Race on `data/extracted/ps4` (2026-10-13) draws each player's
own lap panel, place, time and speed arc in its half, as HD's does. **2048: checked, differs** - the
files are authored in HD's 1920x1080 while 2048's HUD space is the Vita's 960x544,
so its copy would need scaling first (as `oag_hud::assist::splice` scales a
fragment). The PS2 pressing of Pulse names only
`Arcade_HUD.xml` and `TimeTrial_HUD.xml` as HUD paths in its executable and has no
split layout file (**checked, differs**: its split is the camera's own aspect
change, `Camera_SubmitScene`'s viewport count, see
[aspect-ratio.md](../ps2/aspect-ratio.md)); Pure and the PSP Pulse author none.

**Chosen, not measured:**

- **The scaled whole-screen layout** for a case with no file and no authored layout:
  uniform scale to fit, aspect kept, centred. In a two-up strip (about 3.5:1) the HUD
  is height-limited and sits in the middle of the strip, so its corner widgets are not
  at the strip's corners. Pulse, Pure and HD no longer reach it; 2048 and Omega's
  other cases do.
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
`PLAYER 2`) in Pulse's own multiplayer table or HD's grid, and write no record. With two
people on Pulse the end screens are `EndRace Results Split` and `EndRace Rewards Split` (the
PS2's own, borrowed by the PSP) and each person banks their own team's loyalty; any other
split banks none. The authored data, the chosen rules and what is open are in
[split-end-race.md](../ui/split-end-race.md).
`--autopilot` flies every local seat, so `--players 2 --autopilot --mode single_race`
reaches the end headless.

## Cost

Measured on the workstation (AMD Radeon RX 7800 XT, RADV, Vulkan, a release build with
`--features perf-probe`), headless at 1280x720 on Pulse's default circuit,
`--mode single_race`, tick 300 and tick 1800, median of 60 frames per run and of 3 runs.
The load average is the machine's, not this process's: the numbers are a floor under
contention, which is why every row carries it.

**How to measure.** `OAG_RENDER_BENCH=N` and `OAG_RENDER_GPU_BENCH=N` on a
`--screenshot --players N` run of a `perf-probe` build (`crates/game/src/race_capture/bench.rs`;
a split race goes through `run_split`). It prints, per run:

- `render_viewports CPU (record + submit)`: one whole frame's CPU cost, the GPU idled first;
- `render_viewports submit-to-idle`: the same plus the wait for the GPU, serialised;
- `sum of N scene pass(es), GPU`: the timestamp pairs of each view's scene pass, summed;
- `pipelined per frame`: frames submitted back to back with one wait at the end, which is
  the cost a frame budget is read against (it hides the smaller of CPU and GPU behind the larger);
- with `OAG_RENDER_PERF=1`, the last CPU run's per-phase spans and allocations.


The previous table here (6,507 and 7,172 draws at eight views, tick 300 and 1800) does not
reproduce: Pulse's default circuit with the default culling submits 3,762 and 3,700 draws at
eight views, and `--pvs false` 7,923 at tick 1800, so the old figures were most likely taken
with the visibility partition off; the cause is not known. The draws column below is the
`frame at tick` line of the same capture, at the default culling.

**Verdict, on the discrete GPU: a split is CPU-bound.** Before the hoist below, Pulse at
eight views cost 19 ms of CPU per frame (over the 16.7 ms of 60 Hz) while the eight scene
passes together cost 1.1 ms on the GPU, about 0.13 ms each. The pipelined cost followed the
CPU number to the decimal, so the GPU hid entirely behind the recording. There, lowering the
render scale buys nothing and dynamic resolution has nothing to act on; the lever is what the
CPU does per view. **That does not carry to a weaker GPU**: see the Raphael iGPU table below,
where the scene passes cost 4 to 7 ms and the same split is GPU-bound at one and two views.

**Where a view's CPU went** (Pulse, two views, per view, `OAG_RENDER_PERF=1`): the shadow
geometry's gather 1.2 to 1.6 ms (a hull projection that casts a ray per vertex onto the
floor, on Pulse's default `original` tier), `encoder.finish` 0.4 ms (the recorded commands are
encoded there, not where they are recorded; it looks proportional to the draw count: 1.1 ms of HD's
1.9 ms per view at twice Pulse's draws), the
craft and shield uniform writes 0.3 ms, scenery clocks and adverts 0.1 ms, the rest under
0.1 ms each. Recording the draws themselves is 0.04 ms.

**What the first view now does alone** (`Layer::Next` skips it): the two shadow maps
(`render_shadow_passes`), the shadow gather (`upload_shadow_geometry`; every view still
uploads it with its own matrix), the scenery's animation clocks and the advert cards.
Each is a function of the craft and the race clock, never the camera: the casters are fixed
buffers and ranges, the coverage fit spans every craft (the `mapped` fit is per view since
2026-10-13, below), the sun-occlusion fit is each craft's own, the adverts draw through their own authored cameras. Every picture was
byte-identical to before the hoist (the `mapped` tier at two or more views has changed since, below): 22 stills (six titles at one view, Pulse at two, four and eight,
Pure, HD, Omega, 2048 and Pulse PS2 at two, and the `mapped` and `original` shadow tiers on
Pulse and HD) plus seven at tick 1200 (up to eight views) `cmp` equal, each pair taken with the
shadow tier that reaches the hoisted code. **Not hoistable**, each reads the view's matrix,
eye, fog sample or LOD: the scene uniform (`ship_scene` carries the eye's fog and the
refraction matrix), craft, shield, flame and pad uniform writes, `select_lod`, behind-glass,
particles, clouds, exhaust and the per-view sort.

Three runs each, the median of each run's median, before (main `1fcfd1255`) then after
the hoist, taken one binary after the other. The machine drifts by 10 to 20 percent over
minutes under other lanes' builds, so a row is only comparable inside itself, never against
another row's wall time: the load column is the 1-minute load average at each run's start.

| Title | Views | Tick | Draws | CPU ms before -> after | Pipelined ms before -> after | GPU scene passes ms | Load (before / after) |
| --- | ---: | ---: | ---: | --- | --- | ---: | --- |
| pulse | 1 | 300 | 478 | 2.41 -> 2.78 | 2.47 -> 2.86 | 0.23 | 6 9 10 / 6 6 7 |
| pulse | 2 | 300 | 964 | 4.81 -> 3.63 | 4.80 -> 3.71 | 0.36 | 6 9 10 / 6 6 7 |
| pulse | 4 | 300 | 1887 | 9.64 -> 5.71 | 9.77 -> 5.78 | 0.62 | 7 8 9 / 6 7 7 |
| pulse | 8 | 300 | 3762 | 19.14 -> 9.42 | 19.12 -> 9.48 | 1.06 | 7 8 9 / 5 7 7 |
| pulse | 1 | 1800 | 569 | 2.63 -> 3.07 | 2.60 -> 3.00 | 0.25 | 9 8 9 / 8 8 7 |
| pulse | 2 | 1800 | 1019 | 4.99 -> 3.89 | 5.00 -> 3.98 | 0.35 | 8 9 9 / 8 8 6 |
| pulse | 4 | 1800 | 1755 | 9.85 -> 5.84 | 10.52 -> 5.73 | 0.55 | 8 9 9 / 8 7 6 |
| pulse | 8 | 1800 | 3700 | 19.17 -> 9.57 | 19.68 -> 9.36 | 1.07 | 9 8 8 / 8 7 6 |
| 2048 | 2 | 300 | 1494 | 1.82 -> 1.97 | 1.85 -> 2.09 | 0.34 | 7 / 8 |
| hd | 1 | 300 | 904 | 1.86 -> 2.38 | 1.96 -> 2.31 | 0.57 | 7 6 6 / 7 6 6 |
| hd | 2 | 300 | 1811 | 3.81 -> 3.55 | 3.75 -> 3.34 | 0.81 | 7 6 6 / 6 6 8 |
| hd | 4 | 300 | 3479 | 7.48 -> 5.30 | 7.47 -> 5.23 | 1.39 | 6 6 6 / 6 7 9 |
| omega | 2 | 300 | 2436 | 1.20 -> 1.56 | 1.10 -> 1.25 | 1.11 | 6 / 8 |
| pure | 2 | 300 | 722 | 0.75 -> 0.75 | 0.79 -> 0.84 | 0.24 | 7 6 6 / 7 7 6 |
| pure | 4 | 300 | 1429 | 1.53 -> 1.51 | 1.55 -> 1.57 | 0.32 | 7 6 6 / 7 7 6 |
| pure | 8 | 300 | 2867 | 3.03 -> 2.99 | 3.12 -> 3.17 | 0.48 | 7 6 6 / 7 7 6 |

`GPU scene passes` is the sum of the views' scene passes after the hoist (the hoist does not
move it). The one-view rows are an unchanged code path and read 15 percent apart from drift alone: an
interleaved A/B (the two binaries alternating, six rounds each) gives Pulse one view 2.70 to
3.23 ms before against 2.67 to 3.37 after, and HD one view 2.27 to 2.39 against 2.04 to
2.46, the same population. The same alternating A/B for the split rows, the median of each
binary's runs, with the load average read at the start of each run:

| Title | Views | Rounds | CPU ms before | CPU ms after | Pipelined ms before | Pipelined ms after | Load during the runs |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| Pulse | 2 | 5 | 4.85 | 3.21 | 4.80 | 3.23 | 6 to 8 |
| Pulse | 4 | 2 | 9.45 | 4.84 | 9.58 | 4.90 | 6 |
| Pulse | 8 | 4 | 20.08 | 11.26 | 21.27 | 9.09 | 4 to 13 |

At eight views the quiet runs (load 4 to 5) read 18.2 to 20.9 ms before and 8.1 ms after; the
machine was shared with other lanes' builds for the rest, which is the spread. Earlier
alternating runs without the per-run load (uptime readings of 7 to 14 around them) agree:
Pulse two, four and eight views 5.81, 11.39 and 22.59 ms of CPU before against 3.74, 5.78
and 9.76 after, and HD four views 8.16 against 5.43.

**2048 and Omega** (the shared-lineage check): the hoist is title-agnostic, so both ran it
(**ported**, nothing title-specific to port). Pictures are `cmp` equal at two and four views
(Omega) and two and four views (2048), and each was timed once at two views (table above;
single runs, so only the order of magnitude: 1.2 to 2.0 ms of CPU, which is the GPU's
business more than the CPU's on Omega, whose scene passes cost 1.1 ms).

**The shadow gather per tick, and the `mapped` map per view** (2026-10-13, `shadow-gather-per-tick`).
Both are for the Raphael iGPU and the discrete card alike; the numbers follow.

*The gather.* The `original` tier's hull gather (`oag_raceplay::hull_gather`) is a ray per hull
vertex onto the floor, and it ran every **rendered** frame although the craft move once per
60 Hz tick. Profiled on Pulse's Single Race grid (eight craft, release build, one run of 300
ticks): the placement rays 10 us, `hull_triangles` 20 us and `conform_to_floor` (the per-vertex
floor rays, about 240 vertices a craft) **1,722 us a tick**, so the floor rays are the whole
gather. It is now taken at the tick's own poses, kept with the tick before it, and a draw
between ticks lerps the two by the draw alpha (`Scene::original_hulls`,
`scene/frame/hull_cache.rs`):

- **At alpha 1.0 the picture is main's, bit for bit.** Every capture, trace and screenshot
  draws there: 47 of 49 stills `cmp` equal to main's binary (six titles at one view at ticks 300,
  1200 and 2400 and at two views at tick 300; Pulse at two, four and eight views at ticks 300 and
  1200; the `original`, `blob`, `mapped` and `off` tiers of Pulse and HD at one view; Pulse's `blob`
  at two and four views; `--draw-alpha` 0.25, 0.5 and 0.75 at one and four views, which draw the
  live gather because a single capture has no predecessor to blend from); the other two are the
  intended `mapped` change below.
- **Between ticks it is not the same picture, and is not meant to be.** The old gather projected
  the hull from the blended pose; this lerps the projected vertices of the two ticks. On Zone's
  circuit (three 60-tick windows, 540 frames at alpha 0.25, 0.5 and 0.75, 168,480 vertices,
  `hull_gather_ground_truth`) the lerped vertex sits from the live one by a median 0.0006 world
  units, p90 0.013, p99 0.053, p99.9 0.147 and **at worst 0.237**; a craft is some 13 units long
  and a pixel at the chase camera about 0.03 units, so the p99 is under two pixels and the worst
  vertex (a floor seam under one ray) under eight. **Chosen, not measured** as acceptable.
- **Anything the blend cannot represent is gathered live, as before**: a craft that gained or
  lost its shadow over the tick, a previous gather that is not this tick's predecessor
  (a respawn or a jump past `SNAP_DISTANCE` draws the current gather, as the hull does). Under
  60 Hz the race runs several ticks between draws, so no draw has a predecessor and the cache
  would only double the work: after two misses it stops seeding, and a frame is one gather, as
  before (seeded again on one draw in sixteen).
- The cache key is a pose generation (`Race::pose_generations`, moved by every tick path and by
  `settle_poses`, unique across races) plus each craft's tick model matrix, not `World::tick`,
  which the flyby and cosmetic ticks hold still. Tests: `tests::split_shadows` (once per tick
  at four draws a tick, once per frame under 60 Hz), `hull_gather::tests` (the lerp, a changed
  hull, a snap).

*The bench must say how many frames share a tick.* It draws one captured tick over and over, so a
per-tick cache hits on every frame but the first. `OAG_RENDER_BENCH_FRAMES_PER_TICK=K` (default 1)
makes every K-th benched frame a new tick: **1 is a 60 Hz window (the cost before the cache), 4 a
240 Hz one, and a very large number is the upper bound** where the cache never misses. Pulse,
tick 300, medians of three runs, load average 11 to 14 (the machine was shared), RX 7800 XT,
`render_viewports` CPU and pipelined, ms:

| Views | before (main) CPU / pipelined | 60 Hz (K=1) | 240 Hz (K=4) | upper bound |
| ---: | --- | --- | --- | --- |
| 1 | 2.98 / 3.07 | 3.09 / 3.04 | 1.52 / 1.73 | 1.26 / 1.29 |
| 2 | 3.53 / 3.81 | 3.60 / 3.86 | 2.28 / 2.59 | 2.02 / 2.15 |
| 4 | 5.44 / 5.83 | 5.34 / 5.78 | 3.86 / 4.41 | 3.82 / 4.03 |
| 8 | 8.10 / 8.17 | 8.11 / 8.19 | 6.68 / 7.04 | 6.74 / 6.72 |

A 60 Hz window gains nothing (and loses nothing: the rows are inside the machine's own spread);
a 240 Hz window saves 1.5 ms of CPU at one view and 1.25 to 1.6 ms at two, four and eight (the
gather costs the same however many views share it, since the earlier hoist). The GPU sum was 0.31, 0.49,
0.85 and 1.46 ms in every column. On the Raphael iGPU (load 3 to 5, N = 1, 2, 4; the eight-view
rows were taken at load 33 to 65 and are not quoted) the CPU column moves the same way
(2.21 -> 2.20 -> 1.16 ms at one view, 3.10 -> 1.84 at two, 4.58 -> 3.82 at four, K = 1, 4) and
the **pipelined time does not**: 4.89, 5.81 and 7.92 ms before against 4.75, 5.69 and 7.95 at
K = 4, because the iGPU is bound by its 4.8 to 7.3 ms of scene passes. Pure draws no shadows and
did not move on the iGPU (pipelined 4.58, 4.93, 5.65, 6.11 ms before; 4.65, 4.88, 5.46, 5.83 after,
at one, two, four and eight views, load 9 to 26).

*2048 and Omega* (the shared-lineage check): both draw through the same `original_hulls` and
`render_shadow_passes` code, **ported**, nothing title-specific: their default tier is `off`
and `original` draws no hull there (no authored occluder), so their stills at one and two views
are `cmp` equal to main's, and the `mapped` redraw per view is the same code on every title.

*Open from it:* the floor rays are the cost; **done since, below** ("The floor rays ask the road once
per point").

*The `mapped` map per view.* The depth map is fitted on the craft its view follows
(`Race::viewed_slot`), not on slot 0 for every view: each later view redraws the map itself, in
its own encoder (each view is its own submission, and a queue write lands before the submission
that follows it, so no view's fit reaches another's draws). The alternative, one map that fits
every viewed craft, loses texels as the craft spread (a 140 unit box becomes the whole spread
plus 70 each side), and the race's craft are track-lengths apart within seconds. **Chosen, not
measured** for the picture; the cost is measured. Pictures: view one of a two-view race is pixel
equal to main's (ImageMagick `compare -metric AE` 0 over its half), views two and up now carry
their craft's shadow where main's had none; N = 1 is `cmp` equal. Pulse, tick 300, medians of
three, ms:

| Views | RX 7800 XT before -> after (CPU / pipelined) | load | iGPU before -> after (pipelined) | load |
| ---: | --- | --- | --- | --- |
| 1 | 1.13 / 1.16 -> 1.12 / 1.15 | 5 | 7.35 -> 7.21 (unchanged path) | 6 to 8 |
| 2 | 2.30 / 2.27 -> 2.31 / 2.33 | 4 to 5 | 7.80 -> 9.40 | 5 to 6 |
| 4 | 4.27 / 4.23 -> 4.56 / 4.66 | 4 to 6 | 9.74 -> 13.72 | 3 to 4 |
| 8 | 8.19 / 8.02 -> 9.49 / 8.96 | 6 to 10 | 11.74 -> 18.86 | 4 to 5 |

On the discrete card a redraw costs about 0.1 to 0.2 ms a view; on the iGPU **1 to 1.6 ms of GPU
time a view past the first**, because the depth pass drew the whole circuit into every map (the
track was one caster, not culled to the map's box). The shared-map alternative costs what main's
single map does (the "before" column), so that is the price of views two and up getting their
shadows. The `mapped` tier is opt-in and the iGPU split is Pulse and Pure only, and eight views
at 18.9 ms missed 60 Hz. **Culled since, below** ("The track caster is culled to each map's box").

*The floor rays ask the road once per point* (2026-10-13, `shadow-floor-rays`). The hull gather's
cost was `conform_to_floor`: one `Race::floor_above` ray (a `raycast_all` over the circuit's BVH)
per output vertex, twelve per coarse hull triangle. A triangle has six distinct points (three
corners, three midpoints), each shared by three or four of the twelve, and the ear-clipped fan of
a hull ring shares its corners and diagonals across neighbouring triangles, so most rays repeated
an earlier one. `conform_to_floor` now keeps the answer of each `(position, normal)` bit pattern
for the one hull and asks the road once per distinct point. `floor` is a function of exactly
those bits, so **the vertices are the same, bit for bit**: no approximation, no change to
`oag-physics` (the floor query itself is untouched, `just gate-affected` did not select a
simulation crate). The cache lives for one hull in one call, never across ticks (the poses move
every tick, so it would never hit).

- **Proof**: a probe over Pulse's eight-craft Single Race grid, 300 ticks on autopilot, hashes
  every position, normal and colour of the 496,800 gathered vertices through `HullGather::at_tick`:
  `906fd5eb7311ca08` on main and on the branch. 26 stills `cmp` equal to main's binary (Pulse
  `mapped` and `original` at one, two, four and eight views at ticks 300 and 1200; HD `mapped` and
  `original`, Omega and Pulse PS2 and Pure `mapped` at one and two views, tick 300). 2048 has no
  source this checkout can read (its `.pkg` is PFS-encrypted, no `.vpk`), so it was not run; its
  `original` tier draws no hull, so the change cannot reach it. Unit tests: `a_point_shared_by_triangles_is_asked_of_the_road_once`
  counts `floor` calls on a two-triangle quad (24 vertices, 9 distinct points) against a copy of
  the old per-vertex loop and asserts the two outputs bit-equal; it fails if the memo is dropped.
- **Cost**: `HullGather::at_tick`, release build, three runs each, load average 14 to 16: **1,196
  us a tick before, 417 after** (2.9 times). What is left is the rays that are distinct, in
  `raycast_all`'s per-collider `Vec` and sort; answering them from a cheaper query was not done
  because `floor_above` sees only the **first four** hits in collider and triangle order
  (`MAX_HITS`), non-hoverable and coplanar duplicates included, and a nearest-per-collider query
  would not (a semantics change, to be agreed rather than slipped in).

RX 7800 XT, Pulse, `original` tier, tick 300, medians of three runs, load average 1.2 to 1.5, ms,
`render_viewports` CPU (pipelined in brackets), before (main) -> after:

| Views | 60 Hz (K=1) | 240 Hz (K=4) |
| ---: | --- | --- |
| 1 | 2.44 (2.25) -> 1.46 (1.48) | 1.03 (1.40) -> 1.04 (1.17) |
| 2 | 3.24 (3.27) -> 2.22 (2.27) | 1.83 (2.21) -> 1.83 (1.97) |
| 4 | 4.86 (4.91) -> 3.85 (3.89) | 3.46 (3.83) -> 3.45 (3.58) |
| 8 | 8.11 (8.15) -> 7.09 (7.12) | 6.68 (7.06) -> 6.71 (6.85) |

A 60 Hz window now saves 1.0 ms of CPU a frame at every view count (it paid the whole gather every
frame, since the per-tick cache never hits there); a 240 Hz window already paid it once a tick
and saves 0.2 to 0.25 ms of pipelined time. On the Raphael iGPU the `original` tier is bound by
the scene passes (4.8 to 8.0 ms) and the pipelined column does not move (4.83 -> 4.99, 5.68 ->
5.72, 7.67 -> 7.58, 9.83 -> 9.37 at K=1, inside the spread); its CPU column falls the same way
(2.17 -> 1.50, 2.98 -> 2.15, 4.38 -> 3.66, 7.38 -> 6.62 ms).

*The track caster is culled to each map's box* (same lane). `render_depth_map` draws the track,
every craft and nothing else into the `mapped` tier's depth map, and the map covers a box of
`MAPPED_RADIUS` (70) around one craft. Each opaque track draw carries its own world-space
bounding sphere (`DrawCall::bounds`, the one the scene pass frustum-culls with), so the pass now
tests every sphere against the box's own frustum (`Frustum::from_view_projection(fit.matrix())`,
the matrix the pass renders with) and records only the draws that touch it
(`Drawable::opaque_ranges_in`). A draw an `Anim Transform` moves has no trustworthy bound and is
always kept. Nothing is approximated: the sphere test is conservative, and the pass has no
depth clamp, so a draw wholly outside the box was clipped away whole before. Per map, so each
view of a split culls to its own craft. Kept of the track's opaque draws: Pulse 465 of 1,737
(ticks 300 and 1200), Pure 123 of 1,179, HD 286 to 287 of 1,835.

- **Proof**: the same 26 stills `cmp` equal (the `mapped` ones are the claim), plus 16 more `mapped`
  stills at ticks 1200 and 2400 (Pulse, Pure, HD and Omega at one and two views) equal to main's.
  Stills can only catch an over-cull (with no cull the pass draws main's whole circuit), so
  they were checked to be sensitive: a throwaway build whose test sphere has radius 0 differs from
  main on all four stills tried (9,595 pixels of Pulse's two views at tick 300), and `mapped`
  differs from `off` by 25,057. **The cull being there** is the unit test
  `the_mapped_depth_map_records_only_the_track_draws_in_its_box` (`tests/split_shadows.rs`): the
  test circuit is one triangle at the origin, one view's box holds it and the other's clears it,
  and the pass's own count (`FrameWork::track_ranges_in_depth_map`) must read 1 and 0; with the
  cull replaced by "every range" the second reads 1 and the test fails. Also
  `only_the_draws_that_touch_the_light_box_are_kept` (inside, a sphere reaching in at the side,
  outside, behind the far plane, an unbounded moving draw).
- **Cost**, `mapped` tier, Pulse, tick 300, medians of three, before -> after. RX 7800 XT, K=1
  (60 Hz), load 1.0 to 1.4, CPU / pipelined ms: one view 1.11 / 1.14 -> 1.07 / 1.09, two 1.98 /
  2.01 -> 1.89 / 1.93, four 3.77 / 3.79 -> 3.62 / 3.64, eight 7.38 / 7.43 -> 7.05 / 7.05 (0.05 to
  0.4 ms: the discrete card was never limited here). Raphael iGPU, pipelined ms (the rows were
  taken with other lanes' work running, the per-run load average ranged from 1 to 8, so read them
  as a direction): Pulse K=1 one view 7.33 -> 6.60, two 9.70 -> 9.04, four 13.47 -> 12.93, eight 18.45
  -> 16.55; K=4 7.34 -> 7.06, 9.11 -> 9.28, 13.91 -> 12.12, 18.89 -> 16.39. Pure (`mapped`), K=1: 6.32 ->
  5.85, 8.47 -> 7.15, 10.82 -> 9.57, 14.61 -> 12.90. So the single-view tier gains about 0.3 to
  0.7 ms on the iGPU and eight views about 2 ms, **less than the 1 to 1.6 ms a view the map cost**:
  what remains is the 2048 x 2048 depth clear, the craft casters, and the 465 draws Pulse's box
  still touches (the track's draws are big, a sphere is loose around them; a tighter bound is
  not authored in the model).
- **2048 and Omega** (shared lineage): the cull is the `Drawable`'s own and title-agnostic
  (**ported**): HD and Omega's `mapped` stills at one and two views are `cmp` equal; 2048 not
  runnable here, as above.

*Open from this lane:* culling the hull gather to the craft some view can see (with eight craft
and two views most hulls are projected for nothing: needs the visibility to be exact, and the
two-tick cache wants every craft's gather on the tick before); the Steam Deck numbers above
(the maintainer's).

**Steam Deck.** Not measured: the Deck is the maintainer's. Steps, Pulse and Pure only (HD,
2048 and Omega stay off integrated GPUs in a split):

1. Build the bench binary on the Deck (desktop mode, repo checked out, the two images under
   `data/images/`): `cargo build --release -p oag-game --features perf-probe`. Plugged in,
   nothing else open.
2. For N in 1, 2, 4, 8, three runs each, from the repo root:
   ```sh
   mkdir -p /tmp/oag-bench; for n in 1 2 4 8; do for rep in 1 2 3; do
     echo "== pulse n=$n rep=$rep load=$(cut -d' ' -f1 /proc/loadavg)"
     XDG_CONFIG_HOME=/tmp/oag-bench/config XDG_DATA_HOME=/tmp/oag-bench/data XDG_STATE_HOME=/tmp/oag-bench/state \
     OAG_RENDER_BENCH=60 OAG_RENDER_GPU_BENCH=60 \
       ./target/release/oag-game data/images/pulse-psp-eu.chd --race --no-audio --mode single_race \
         --players $n --ticks 300 --size 1280x800 --screenshot /tmp/oag-bench/shot.png 2>&1 \
       | grep -E '^bench (split|render_viewports CPU|sum of|pipelined)|frame at tick'
   done; done | tee /tmp/oag-bench/pulse.txt
   ```
   Then the same with `pure-psp-eu.chd` for Pure. The private `XDG_*` directories keep it off
   the Deck's own settings and saves; nothing makes a sound or opens a window.
3. Read, per run, the median (microseconds): `bench split: ... on <adapter>` must name the
   Deck's GPU (an AMD RADV device), not llvmpipe; **`pipelined per frame` is the number to hold
   under 16.7 ms (60 Hz) or 25 ms (40 Hz)**; `render_viewports CPU` is the CPU's share;
   `sum of N scene pass(es), GPU` is the GPU's. If `pipelined` sits near `CPU` and far above
   the GPU sum, the split is CPU-bound there too; if the GPU sum approaches `pipelined`, the
   Deck's GPU is the limit and a lower render scale would help.
4. Send the medians of the three runs per N with the load averages.

**The Ryzen 9 7900's own Raphael iGPU (RDNA2, 2 CUs, selected with
`[graphics] renderer = "vulkan: AMD Ryzen 9 7900 12-Core Processor (RADV RAPHAEL_MENDOCINO)"`).
An APU proxy, weaker than the Deck's GPU and paired with a far stronger CPU, so it brackets the
Deck from one side and says nothing about its number.** Same runs, tick 300, three each, medians,
load average 8 to 12:

| Title | Views | Draws | GPU scene passes ms | CPU ms before -> after | Pipelined ms before -> after | Bound by (after) |
| --- | ---: | ---: | ---: | --- | --- | --- |
| Pulse | 1 | 478 | 4.25 | 3.14 -> 2.84 | 4.86 -> 4.76 | GPU |
| Pulse | 2 | 964 | 4.88 | 5.69 -> 4.00 | 5.98 -> 5.48 | GPU |
| Pulse | 4 | 1887 | 6.53 | 11.25 -> 5.97 | 11.03 -> 7.49 | both |
| Pulse | 8 | 3762 | 7.07 | 22.08 -> 9.37 | 22.27 -> 9.65 | CPU |
| Pure | 1 | 363 | 5.07 | 0.74 (unchanged path) | 4.49 | GPU |
| Pure | 2 | 722 | 5.49 | 1.36 | 4.84 | GPU |
| Pure | 4 | 1429 | 6.20 | 2.37 | 5.36 | GPU |
| Pure | 8 | 2867 | 6.61 | 4.49 | 6.23 | GPU |

Pure was run after the hoist only (it draws no shadows, so the hoist does not touch it: 0.75 ms
before and after on the discrete card). Its four rows were taken at a load average of 86 to 163,
another lane's test run, so its CPU column is inflated by contention; the GPU sum and the
pipelined column, which are what the iGPU is bound by, were steady across the three runs
(for example 4.99, 5.07 and 5.09 ms of scene pass at one view). The adapter was confirmed from
the bench's own `bench split: N view(s) on AMD Ryzen 9 7900 12-Core Processor (RADV
RAPHAEL_MENDOCINO) (IntegratedGpu, Vulkan)` line; the setting reaches the headless capture path.

So on an APU the same split flips: the scene passes cost 4 to 7 ms, the hoist is worth
the most at four and eight views where the CPU was the limit, and at one to two views (and
Pure at any count) the GPU is the limit, where a lower render scale would help and the CPU
work does not matter. Eight views hold 16.7 ms on this machine either way: Pulse at 9.7 ms
and Pure at 6.2 ms pipelined, 1280x720 in all.

**Product rule (decided by the maintainer, not a proposal here): split screen on an
integrated GPU is offered for Pulse and Pure only; HD, 2048 and Omega stay off it.** What the
Raphael numbers say about the two that are offered: the GPU cost is set by the pixels and barely
by the view count (Pulse 4.3 ms at one view, 7.1 at eight; Pure 5.1 to 6.6), because the views
share one frame's pixels, so **no view-count cap is implied at 720p**: every count up to eight
holds 60 Hz with 7 ms or more to spare. What the iGPU's budget is made of is resolution: a
frame is about 4.3 ms of scene pass (Pulse) at 1280x720, so the same split at 1920x1080 would
be roughly 2.25 times that by pixel count (an estimate, not measured), still under 16.7 ms. If a
slower APU or a larger window ever pushes a split over, render scale is the lever for these two
titles and a detail budget (draw distance, LOD) is the second; neither is needed on the
evidence here.

**Next steps, ranked by what the numbers say** (recommendations; none is built):

1. **The shadow gather, at any view count.** Still the largest single CPU item on Pulse's
   default tier (1.2 to 1.6 ms a frame, half of one view's cost): a ray per hull vertex onto
   the floor, redone every rendered frame though the craft moves once per 60 Hz tick. Caching
   it per tick, or per craft, is a view-independent win that also helps one view; it needs a
   check that the picture at an interpolated draw alpha does not move.
2. **The Deck number decides the next lever, and it is the maintainer's to take.** The
   workstation's discrete card is CPU-bound and the Raphael iGPU is GPU-bound at one and two
   views (and Pure at every count) and CPU-bound at eight on Pulse; the Deck pairs a stronger GPU than Raphael's with a weaker CPU,
   so it can land on either side.
   - **If the Deck reads CPU-bound** (`pipelined` near `CPU`): a per-view draw-distance and LOD
     rule tied to the view count, which belongs to the graphics-settings cascade design thread.
     `encoder.finish` is the largest per-view cost left (0.4 ms on Pulse, 1.1 ms
     of HD's 1.9 ms) and looks proportional to the draws a view submits.
   - **If it reads GPU-bound** (the scene-pass sum near `pipelined`): resolution is the lever, and
     `Scene::render_viewports`' per-view timers already exist, so summing them into dynamic
     resolution's reading for a split is small; it holds its scale while split today.
3. **Per-view uniform slices** in one buffer with dynamic offsets, so the views stop
   overwriting each other's `queue.write_buffer` data. By itself it saves little (one
   encoder and submit instead of N: about 0.1 ms a view), but it is the prerequisite for
   the item below, and it lets the craft, shield and pad uniform writes (0.3 ms a view
   today) move out of the per-view path.
4. **Recording the views on several threads** (wgpu encoders are `Send`), after item 3.
   The views are independent once they stop sharing uniform buffers; on a CPU-bound split
   this divides the per-view CPU by the cores used. The largest change and the largest
   possible gain; do it only if items 1 and 2 leave the Deck short.
5. Per-view bloom and the other effects the split omits are a picture question, not a cost
   one, and cost the GPU little on the workstation.

**Limits of these numbers.** The bench re-renders one static frame (the race held at tick 300
or 1800), so it does not see tick-to-tick variation; every view carries an unexplained 0.5 to
1.3 ms of extra CPU on its first spans after the GPU is idled, in both binaries, which was
not chased (the pipelined numbers, with no idle in between, agree with the CPU ones); a
`perf-probe` build adds marks inside the pass that can shift small spans.

## Who plays which slot (chosen, not measured)

Every rule here is ours. The PS2 build takes its players from the pads in port
order and has no join step to read; its authored flow (`FE_RACE_CAM_SS` and
`FE_RACE_CAM_SSH` route to `Split ScreenV`/`Split ScreenH`, and `Team SelectionSplit`
carries both players' widget sets, `docs/formats/race-setup.md`) is a camera choice
and a per-player team pick, not a device join, so it is not what the entry below
follows. Confidence scores do not apply.

**Any seated player pauses; only they resume.** Start pauses from every seated
device, not the keyboard's slot alone (`Controls::take_presses`): with devices seated
a pad's presses land on its own slot, and before 2026-10-13 a Steam Deck's own Start
did nothing in a split race (reported from play). The pause then belongs to the player
who pressed: only their Start resumes and only their Circle leaves the race
(maintainer's rule, 2026-10-13; `session::pause`). A pause nobody pressed for, the
window losing focus, is anyone's to lift. A one-player race is unchanged, since every
press is slot 0's there.

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
every title that shows the extras, and a tile on 2048's grid (see "Where the entry is,
per title"). `oag_input::lobby::Lobby` is the model:

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

### Where the entry is, per title (2026-10-13, `split-entry-hd-2048-omega`)

Walked windowed on an own Xvfb with two virtual pads (`scripts/uinput-pads.py`): menu,
SPLIT SCREEN, both pads join, START, the title's own screens, a two-view race.

| Title | Where the entry is | Authored by the original? |
| --- | --- | --- |
| Pulse, Pure | SPLIT SCREEN row after REMIX, under RACEBOX (the `oag-extras` mod) | PS2 Pulse authors `Team SelectionSplit` and the split cameras but no join step; the row is **chosen, not measured** |
| HD | **The race box's own rows, as the disc authors them** (2026-10-13, `hd-authored-split-setup`): on the RACE page after AI DIFFICULTY, `NUMBER OF PLAYERS` (`1`, `2`) then `SPLIT SCREEN` (`HORIZONTAL`, `VERTICAL`), read from `racebox_definition.xml`'s `NumberOfPlayers` and `SplitScreen` lists (`RB_NUMP`, `RB_SPLSCR`, `RB_HORIZ`, `RB_VERT`). The cut row is greyed while the count is one (the authored `IDCantFocusString="FE_OFF"`: this build greys it and keeps the stored cut, it does not draw `FE_OFF` over it). START with two people opens the join screen **for exactly two seats** and refuses START until both are taken (chosen: the authored count is the head count), then HD's Track Select, `Ship Select - Player 1` and `Player 2`, then the race in the chosen cut. **These rows replace our generic SPLIT SCREEN row on HD** (the authored place wins; `Definition::drop_rows_picked_on_screen`); the row stays on Pulse, Pure, Omega and 2048. Two people is the authored maximum, so HD has **no route to three or more** from the menus; `--players N` still cuts up to eight. On an integrated GPU HD offers no split, and the two rows go with the row (`Definition::drop_split_screen`). The padlock beside `NumberOfPlayers` is a trial-build lock, not a progression unlock: [racebox-padlocks.md](../ghidra/functions/ps3-hdfury-eu/racebox-padlocks.md), confidence 72, so two players are unlocked here |
| Omega | **The race box's own rows, as the disc authors them** (2026-10-13, `split-entry-mirror-hd`): Omega's real `racebox_definition.xml` (`data09`, 15,805 bytes) carries HD's `NumberOfPlayers` (y 420, with its padlock) and `SplitScreen` (y 470, `IDCantFocusString="FE_OFF"`) lists with the same ids and English text, so `FrontEnd::local_split` names it (`LocalSplit::Authored`) and the same code HD uses puts both rows on Omega's RACE page and drops our generic row (the revealed row under RACEBOX now holds REMIX alone). START with two people opens the two-seat join screen, then Track Select, loading and a two-view race in the chosen cut with Omega's HD-shaped HUD (**HD: ported**, own test `selection_screens_ground_truth::omega_race_box_offers_the_discs_own_players_and_cuts`). **Ship Select per seat: checked, applies, not wired** (corrected the same day, the old "no Ship Select" and "not located" came through `scripts/psarc.py`): Omega's `data09` ships `team_selection_definition.xml` with `Team Selection`, `Team Selection Player 1` and `Team Selection Player 2`, the same names and redirects as HD's, in HD's dialect (HD's `picker::hd` reader parses it as it stands, with Omega's own numbers: `ShipModel` origin 960,540, labels at y 120, 14 teams). It is not wired because `FrontEnd::team_select` is `None` on Omega, and setting it also opens Ship Select for one person and drops the TEAM, VARIANT and TRACK rows from the RACE page, which changes Omega's one-player flow (open, see the handover thread); so Omega's seats fly the settings team. Omega ships HD's `vert_` files too, in `data00`, and now that its race box authors the cut they are **wired** (`oag_omega::hud::SPLIT.vertical`, the same four names and 960 column; **ported**, test `split_hud_ground_truth::omega_wires_hds_vert_layouts_one_column_apart`); walked windowed 2026-10-13 with two pads: join, Track Select, a left and right two-view race |
| 2048 | **HD's two rows, borrowed** (2026-10-13, `split-entry-mirror-hd`): the RACE page (reached by the RACEBOX tile) carries `NUMBER OF PLAYERS` and `SPLIT SCREEN` exactly as HD's, with the cut greyed at one person; START with two people opens the two-seat join screen, then the race in the chosen cut. 2048's own `GameModeChoice` third tile (SPLIT SCREEN) is gone, as our generic row is on HD: the rows are the entry. `LocalSplit::Borrowed` in `oag_2048` holds HD's lists (`1`, `2`; `RB_HORIZ`, `RB_VERT`) and the words come from 2048's own English table, which carries HD's ids word for word (`RB_NUMP`, `RB_SPLSCR`, `RB_HORIZ`, `RB_VERT`, `FE_SHIPSEL_P1`/`P2`, and `MAN_2_SPL`, whose text sends the reader to the Racebox: "In Racebox, set the number of players to 2, and choose between vertical or horizontal split screen"). The load report says `split setup borrowed from Wipeout HD/Fury`. A disc-backed test holds the borrowed lists equal to HD's and Omega's authored ones (`wipeout_2048_borrows_the_lists_hd_and_omega_author`). **No Ship Select**: 2048 authors one `team` screen (`Team_Definition.xml`, a touch and 3D screen, `type="team2048"`) and no `Team Selection Player 1`/`2` pair and no Track Select (no `Track_Selection`, no `racebox_definition.xml` in the base, patch or DLC archives), and this build reads none of them for a race, so seats fly the settings team and circuit and the strings `FE_SHIPSEL_P1`/`P2` have no screen. 2048 is **not authored by the original**; the whole entry is **chosen, not measured**, **labelled borrowed**. Omega: **checked, differs** (Omega authors the lists itself in a race box that also lists 2048's own game) |

HD's other multiplayer files (`multiplayer_definition.xml`, `DATA02`) are the online
screens (`Join Game`, `Lobby`, `Network Sync`), not local. The cross-check of the lane: the
2048 tile is **checked, differs** on Omega (its front end is HD's strip, so the row there
was the same revealed row as HD's), and is gone now: Omega's authored rows replace it as
HD's do (**ported**) and 2048 borrows them; the join screen's block rows are **ported** (one
reader serves HD and Omega, the same skin numbers).

**The join screen on HD and Omega** takes HD's own `<List>` geometry (origin, pitch,
text scale, block height and gap from `oag_title::MenuList` and `ListBlocks`) with a label
block and a value block per row, drawn through `oag_ui::menu::block` in the frame's ink, so
it reads like HD's own RACE page and is legible on Omega's light page (white text with no
box was invisible there). The value block is twice HD's list value width (**chosen**: a
device's name is longer than any value HD's lists hold). Before this change the rows were
mirrored off the strip's column (800 on a 1920 grid, the title at 194), which put the
device column 320 units from the names and drew "PLAYER 1" over a pad's name. The hint
goes under the rows there, and also on a skin whose row origin is ours (2048's
placeholder menu), where it overlapped the title. Pulse and Pure are unchanged (their
rows and title share one margin; a test pins the right edge).

### Integrated GPUs (2026-10-13)

The maintainer's product rule (see "Cost"): on an integrated GPU a split is offered for
Pulse and Pure only. It is `Title::split_on_integrated_gpu` (`true` on those two, `false` on
HD, 2048 and Omega: data on the title, no title-name branch), read by
`oag_game::split_screen::offered(title, wgpu::DeviceType)`: only `IntegratedGpu` is gated;
a discrete, virtual, software (lavapipe) or unknown adapter offers every title. Where it
acts, once the adapter is known (`Gpu::device_type`, fixed for the run):

- **The menus**: `Definition::drop_split_screen` removes the row (and `trim_tiles` the 2048
  tile) when a `Shell` is installed (`Session::drop_split_screen_row`, from the windowed
  boot and from 2048's placeholder route; a language reload clones the trimmed
  definition). REMIX and every other row stay.
- **A windowed `--race --players N`** on such a title logs `--players N: split screen is
  not offered for <title> on an integrated GPU, so one view` and runs one view
  (`players_offered`). **Headless captures and the render bench are not gated**: they are
  diagnostics and the Raphael numbers in "Cost" were taken that way.
- The adapter is chosen at launch (`graphics.renderer` takes effect on a restart), so the
  row never changes mid-run. Unit-tested over stand-in device types; no real integrated
  adapter was walked (Xvfb offers lavapipe only), so the gate on a real iGPU is read, not
  looked at.

A split screen writes **no result and no ghost** (`records.toml` and the ghost
store are for the one person the result is about), and it **fields opponents
whatever the mode**, so Time Trial, Speed Lap and Zone are reachable as split races
from the menus. Restarting a race from the 2048 end-of-race screen, which skips the
menus, keeps the split.

START goes on to the race box's own Track Select and Ship Select, as the RACE
page's START does; backing out of Track Select drops the lobby. **Each seat picks its
own craft** - see [Ship Select per seat](#ship-select-per-seat). The race build
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
assignment's rules, are `oag-input`'s unit tests. The session wiring (the
`seat_for_launch` call in the race build, the merge on returning to the menus, the
lobby dropping on Track Select's back, the front end's seat picks reaching
`Options::seat_picks`) has no test of its own: it was **walked live on 2026-10-13**
with two virtual pads (`scripts/uinput-pads.py`, below) on PSP and PS2 Pulse EU in an
Xvfb, through the join screen, Track Select, each Ship Select and a race, and a
change that dropped the `launch_from_settings` copy of the picks would fail no test.
The walk, to repeat it: start an Xvfb, `oag-game data/images/pulse-psp-eu.chd
--no-audio` windowed under the member-rules env prefix, and drive the pads with
`uv run --with evdev python3 scripts/uinput-pads.py < fifo` (commands in its
docstring): `a press cross`, `b press cross` at the join screen, `a press start`, and
on.

## Ship Select per seat

Track Select is one pick, any device. Ship Select then belongs to the seats, each
driven by its own device alone (`session::picker::seat::input` reads the seat's
device out of `Controls::device_presses`, so two pads pressing at once never move
each other's pick):

- **PS2 Pulse, two seats: the disc's own page.** `Team SelectionSplit` authors both
  players' Ship Selects on one page, top and bottom; both lists move at once and a
  confirm holds a seat (`READY`) until the other confirms too. Read, drawn and
  pointed by the same code as the single-player screen
  (`oag_ui_screens::picker::split`). Evidence and the widget inventory:
  [selection-screens.md](../ui/selection-screens.md#the-split-screens-team-selection-2026-10-13).
- **Everywhere else, and any count above two: one screen per seat, in seat order**,
  the order HD authors (`Team Selection Player 1`, then `Player 2`). Titled
  `FE_SHIPSEL_P<n>` where the source's string table has it, `SHIP SELECT - PLAYER <n>`
  otherwise. **Chosen, not measured.**

A seat's pick (team, variant, skin) never touches `settings.race`, which is the first
player's and persists: seat 0 writes the settings as it always did, the others go to
`oag_raceplay::Options::seat_picks` (`SeatPick`), so the 2048 end-of-race restart keeps
them. The race build gives slot `i + 1` its seat's team and paint
(`load::roster::apply_seat_picks`, `oag_livery::load`'s per-slot `Paint`) and moves the
opponent that would have flown the same team onto a team a person left, so the opponents
keep every other team once; two people may take the same team (**chosen**). A seat's
handling is its team's own (`opponent_handling[slot]`), not slot 0's. `--seat-team
TEAM[:SKIN]` (repeatable) is the headless way to set them.

Verified live on PS2 and PSP Pulse EU in an Xvfb with two virtual pads
(`scripts/uinput-pads.py`): each pad moved only its own list, a confirm held and a held
seat ignored the other pad, back on the second seat while the first held took both to
Track Select with the hold dropped, the mouse clicked a half's arrows and craft and its
wheel stepped the half it was over. The race that followed showed the pad-picked
Qirex and EG-X on PSP (live, both views); the PS2 pair's AG Systems is the headless
`--players 2 --team Qirex --seat-team AG_Systems` race (the live PS2 frame cut it off
at the view's edge). Both seats confirming launches at once, so a "both done" still
does not exist: the race frames are that evidence.

## Open

- **A launch must set `race::Options::views`** (the CLI's `--players` does, in
  `prepare.rs`) for any race that will call `split_screen::apply(.., 2)`: without it
  HD reads no authored layout and the reticle projects into the full screen.
- ~~**HD draws only the last view's scene** in a `--screenshot` of `--players 2` or
  `4` (the earlier views are black).~~ **Fixed by the once-over-the-area post chain
  (see "Drawing")**; seen again 2026-10-13 (`split-render-cost`): HD at two and four
  views, tick 300 and 1200, draws every view.
- HD's Wo3 and 2097 skins and its Zone layout are shipped and not wired (and the Wo3 and
  2097 `vert_` copies with them). 2048's authored two-view HUD is **checked, differs** (1920x1080
  files in a 960x544 HUD space, needs scaling first) and has no file of ours;
  Omega's other cases and 2048's draw the scaled whole-screen layout.
- The split-HUD files were looked at in Single Race at every count, and in Zone and
  the Eliminator at four views on Pulse and Zone at four on HD; Time Trial and the
  Eliminator on Pure and HD, and every mode at five to eight views other than Single
  Race, were checked by name against the layouts (`the_shipped_split_files_name_only_widgets_their_titles_author`)
  and not by eye. The PS2 pressing draws its text smaller against its bars than the
  PSP does (that is how its whole-screen HUD is authored here, one file serves both),
  so its split HUD reads smaller; a per-platform scale is open. HD's Eliminator
  column, lap times and Zone ladder are placed by name and have not been judged on a
  real display. Per-person messages and records, and a lock that
  gates another person's fire.
- ~~HD's front end shows no SPLIT SCREEN row, so its authored one-after-the-other Ship
  Select is unreachable by menu there.~~ **It always had one** (the `oag-extras` row,
  revealed under RACEBOX); walked 2026-10-13, join to `Ship Select - Player 2`. 
  **Done, 2026-10-13 (`hd-authored-split-setup`, then `split-entry-mirror-hd` for Omega and 2048)**: HD's authored `NumberOfPlayers` and
  `SplitScreen` rows are wired (see "Where the entry is, per title") and the vertical cut
  exists. Open from it: the authored row draws `FE_OFF` when the cut cannot be focused (this
  build greys the row and keeps the stored cut); the join screen's hint line on HD and Omega
  is small (0.6 of the row scale, under the rows); the vertical cut is HD's alone, since no
  other title authors a choice (`--split-cut` reaches every title and Pulse, Pure, Omega and
  2048 draw their scaled or file layouts in the columns, not judged by eye).
- **Omega's seats pick no ship (open, a decision)**: the walk goes join, Track Select, then
  loading and the two-view race, and nobody picks a ship. Omega does author the screens
  (`Team Selection Player 1`/`2` in `data09`'s `team_selection_definition.xml`, read with the
  digest-matched Rust reader; the earlier "unread" came through the broken script), and HD's
  reader parses them. Wiring is one line (`team_select: Some(..)` in `oag_omega::frontend`) but
  it also gives Omega's one-player RACE START a Ship Select and drops the TEAM, VARIANT and TRACK
  rows, so it is a flow change the maintainer decides.
- **2048's seats pick no ship either**, for a different reason: it authors no per-player pair.
- A seat past the second on the PS2 (the authored page holds two) and every other
  title take Ship Select in turn; a third and later seat's pick is the same
  machinery, but only two seats were walked live.
- A team of a variant family on HD (`Feisar_c1`) is compared by exact id when a seat's
  pick is moved out of the opponents' draw, so a seat on `X_c1` can still face an
  `X` that `follow_player` then moves onto the player's variant.
- Replays: the game does not record one yet (`Race::start_recording` is called only by
  tests), but when it does, the header carries slot 0's `team` alone, so a split race's
  seat picks would not be in it and playback would desync.
- Stick navigation on a seat's screen: a seat reads its device's pressed buttons
  (the d-pad, the keys), as the join screen does, so a pad with only a stick
  cannot move a pick.
- Two real pads: the assignment is unit-tested over stand-in ids and the keyboard
  path runs end to end, but no second pad has been plugged in.
- Per-team camera rigs, per-view bloom (an origin in `oag_post::bloom`), motion blur
  and the temporal upscaler under a split, and a check of HD's and Omega's chains.
- ~~The results scoreboard and the end-race flow still key on slot 0 and end every
  view when slot 0 finishes.~~ **Done, 2026-10-13**: see "The end of the race", with
  `EndRace Results Split` and per-person rewards for two people on Pulse. Open from it:
  per-person records, a live walk with two real pads.
- Resolution holds its scale while split: the scene pass is not timed (timing one
  view of `N` would read as a cheap frame), so dynamic resolution gets no reading.
  The render bench does time every view (`Scene::render_viewports`' `view_timers`);
  it also shows resolution would not help a split here: the GPU is not the limit
  (see "Cost").
- ~~**The `mapped` shadow tier follows slot 0 only**~~ **Done, 2026-10-13**: each later
  view redraws the depth map fitted on its own craft (see "The shadow gather per tick,
  and the `mapped` map per view" under "Cost"). Open from it: the track is drawn whole
  into every one of those maps, 1 to 1.6 ms of iGPU time a view.
- `--players N > 1` **changes `RaceSim::state_hash`**, because `World::controllers`
  is hashed; the cameras, views and draw do not.
- Not a 2048 or Omega finding: the viewport and camera code is title-agnostic, but
  only Pulse's frames were looked at, so Pure, Pulse PS2, HD, 2048 and Omega are
  unchecked under a split.
