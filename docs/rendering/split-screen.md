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
`PLAYER 2`) in Pulse's own multiplayer table or HD's grid, bank no loyalty and write
no record. The authored data (the PS2's `EndRace Results Split`), the chosen
rules and what is open are in [split-end-race.md](../ui/split-end-race.md).
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
buffers and ranges, the `mapped` fit reads slot 0's model matrix, the sun-occlusion fit is
each craft's own, the adverts draw through their own authored cameras. Every picture is
byte-identical to before: 22 stills (six titles at one view, Pulse at two, four and eight,
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
| Pure | 2 | 722 | 4.90 | not run before | 4.85 | GPU |
| Pure | 8 | 2867 | 5.67 | not run before | 6.05 | GPU |

So on an APU the same split flips: the scene passes cost 4 to 7 ms, the hoist is worth
the most at four and eight views where the CPU was the limit, and at one to two views (and
Pure at any count) the GPU is the limit, where a lower render scale would help and the CPU
work does not matter. Eight views hold 16.7 ms on this machine either way.

**Next steps, ranked by what the numbers say** (recommendations; none is built):

1. **The shadow gather, at any view count.** Still the largest single CPU item on Pulse's
   default tier (1.2 to 1.6 ms a frame, half of one view's cost): a ray per hull vertex onto
   the floor, redone every rendered frame though the craft moves once per 60 Hz tick. Caching
   it per tick, or per craft, is a view-independent win that also helps one view; it needs a
   check that the picture at an interpolated draw alpha does not move.
2. **The Deck number decides the next lever, and it is the maintainer's to take.** The
   workstation's discrete card is CPU-bound and the Raphael iGPU is GPU-bound at one and two
   views and CPU-bound at eight; the Deck pairs a stronger GPU than Raphael's with a weaker CPU,
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
- HD's Wo3 and 2097 skins, its Zone layout and the `vert_` left-and-right family are
  shipped and not wired. 2048's authored two-view HUD is **checked, differs** (1920x1080
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
- HD's front end shows no SPLIT SCREEN row, so its authored one-after-the-other Ship
  Select (wired, `Team Selection Player 1`/`2`) is unreachable by menu there.
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
  view when slot 0 finishes.~~ **Done, 2026-10-13**: see "The end of the race". Open
  from it: the PS2's authored `EndRace Results Split`, per-person rewards and
  records, a live walk with two real pads.
- Resolution holds its scale while split: the scene pass is not timed (timing one
  view of `N` would read as a cheap frame), so dynamic resolution gets no reading.
  The render bench does time every view (`Scene::render_viewports`' `view_timers`);
  it also shows resolution would not help a split here: the GPU is not the limit
  (see "Cost").
- **The `mapped` shadow tier follows slot 0 only** (`mapped_fit(ship_model_matrix_of(0))`,
  a 140 unit box ahead of the first craft): in a split, views two and up get a map that
  covers their craft only when it is near the first. Read from the code, not looked at;
  unchanged by the hoist below.
- `--players N > 1` **changes `RaceSim::state_hash`**, because `World::controllers`
  is hashed; the cameras, views and draw do not.
- Not a 2048 or Omega finding: the viewport and camera code is title-agnostic, but
  only Pulse's frames were looked at, so Pure, Pulse PS2, HD, 2048 and Omega are
  unchecked under a split.
