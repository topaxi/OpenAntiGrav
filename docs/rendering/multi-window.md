# Multi-window local play

One process, one race, one tick, and **the views dealt across operating-system windows
instead of all cut up in one window** - for a desk with more than one monitor. **A window holds a
list of seats**: one seat is that person's whole-screen view and HUD, several are cut up inside the
window exactly as a split screen cuts its views (`oag_render::viewport::layout` over the window's
area, the split-HUD rules picked by that window's own view count and cut). So 2 windows of 4 players,
4 and 2, 1 and 1 and 1 are all one model: a list of windows, each `{ seats, cut }`. Split screen
([split-screen.md](split-screen.md)) is the one-window case; the two share every camera, HUD and input
slot, and differ only in where view `i` goes. The decision records are
[ADR-0067](../architecture/adr/0067-a-multi-window-race-is-one-device-and-one-submission-per-window.md)
(the device, the submission, the lifecycle) and
[ADR-0068](../architecture/adr/0068-a-window-holds-a-list-of-seats.md) (the seat list).

**Chosen, not measured, all of it.** The original has no multi-window mode and nothing here is
read off it. First cut 2026-10-14 (lane `multi-window`); seat lists the same day.

```sh
oag-game --race --windows 2                      # two people, a window each
oag-game --race --windows 2 --players 6          # six people, three in each window (cut in thirds)
oag-game --race --window-seats 4,2v              # window 1: four seats in quarters; window 2: two side by side
oag-game --windows 3                             # the front end on window 1; a race seats three, a window each
oag-game --race --windows 2 --windows-fullscreen # each window borderless on a monitor of its own
oag-game --race --window-seats 4,2 --screenshot shot.png --ticks 300   # headless: shot-window1.png, shot-window2.png
```

**The flags.** (The layout screen, below, writes the same layout.)

- `--windows N` (2 to 8): N windows, the people dealt across them as evenly as they divide
  (`split_screen::deal`: 3 over 2 is `2,1`, 6 over 4 is `2,2,1,1`, 2 over 4 is `1,1,0,0` - a window
  nobody is dealt shows a holding frame). One person per window without `--players`; `--players P`
  says how many people. The cut of a two-seat window is `--split-cut`.
- `--window-seats COUNTS`: the seats per window spelled out, a comma-separated list with one entry per
  window, each a count of 1 to 8 and optionally `h` or `v` for the way two seats are cut
  (`4,2v` = four seats in window 1, two side by side in window 2; a count without a letter takes
  `--split-cut`). At least two windows (one window is `--players`), at most eight seats in all; each
  bad entry is refused by name. Not with `--windows` or `--players`
  (`split_screen::parse_window_seats`, tested).
- `--windows-fullscreen`: see Windows below.

## Who sits where

Seated the way `--players N` seats people ([split-screen.md](split-screen.md), "Who plays which
slot"): the keyboard and the first pad on slot 0, each further pad on the next slot in the order
the pads connect. A slot nobody drives is flown by the AI. Slots are dealt to windows in order,
window 1 taking the first: `--window-seats 4,2` is slots 0 to 3 in window 1 and 4 and 5 in window 2.
**The keyboard is one device whichever window has the focus**; a per-window keyboard seat is open.

- **`--race` with a layout**: the people are in their seats from the first tick (`apply_windows`).
- **A front-end run** (a layout without `--race`): window 1 is the front end; the further windows
  open on the first pass of the event loop once the title is known, and show a **holding frame**
  (black, the join screen's own word for a person and the seats the window is planned to hold:
  `PLAYER 2` for one, `PLAYER 5 - 6` for two; the existing `OAG_JOIN_PLAYER`, no new string) until a
  race starts. A race launched from the menus seats everyone the layout plans for - unless the join
  screen's lobby or a restart's kept seats say how many people race, or it is a campaign race, which
  is one person's - and fields opponents for the others to race. **A head count other than the one
  spelled is dealt, not refused** (`split_screen::layout_for`: the join screen seating three over a
  `4,2` layout is `2,1`). Back on the menus the windows go back to their holding frames.
- **Fewer people than windows**: the windows nobody is dealt show their holding frame.

On Wipeout HD, 2048 and Omega the integrated-GPU rule is `--players`' own
(`Title::split_on_integrated_gpu`, `oag_game::split_screen::offered`): on an integrated GPU those titles
open one window, said once in the log, and Pulse and Pure open them all.

## What a window draws

Every window runs **the whole single-window chain on its own framebuffer** (`upscale::Framebuffer`,
at the window's own size): scene, resolve, HUD, Pulse's bloom over the HUD, the screen filter and the
grade. Window 1 is `Session::draw` as ever; `Session::draw_screens` runs after it and draws each
further window the same way (`session/windows.rs`). **What a window draws is the seats the race gives
it** (`Race::windows`, window `i` of the race is window `i` of the run; `Race::window_drawn`):

- **One seat**: a full screen, not a split view. The title's whole-screen HUD for that slot
  (`hud_views::draw` with the window), the countdown, and - on window 1 only - the track panel, the
  results and the end screens. The scene is `oag_raceplay::Scene::render_view`, next to
  `render_viewports`: a view of slot `s` through `Layer::Window`, which clears its target and does all
  the camera-independent work itself like a split's first view, and runs the post chains (Pulse's
  bloom, HD's, Omega's tone map) like an unsplit race, because a window starts at the origin and fills
  the target. The boost-zoom pulse is the player's own, so only the window that follows the player's
  craft draws it.
- **Several seats**: cut up inside the window by the window's own cut, exactly as a split screen
  (`Scene::render_window_views` is `render_viewports` over the window's slots; both are
  `render_slots`, so a one-window split is byte-identical to what it was). The HUD is the split HUD
  (`overlay.draw_views`) with the rules picked by **this window's** view count and cut
  (`split_hud::source`: the title's authored two-player layout for a window of two seats, this
  project's split-HUD files, or the whole-screen layout scaled into each view). As in a split, no
  Pulse bloom and HD's and Omega's chains run once over the window after its last view.
- **Nothing**: a window with no seat to show draws the holding frame.

- **One encoder, one submission per window, for the whole chain.** A queue write lands before the
  submission that follows it, and the scene, the upscaler and the HUD all write per frame; two
  windows in one submission would draw with the last one's values. `Scene::render_viewports` has the
  long form of the argument.
- **One scene allocation, one extent per window.** The scene's depth and multisample attachments are
  one size and a pass wants its attachments equal, so every window's framebuffer is allocated at the
  largest target any window wants (`windows::shared_allocation`) and draws into its own extent of it
  ([ADR-0037](../architecture/adr/0037-dynamic-resolution-varies-a-viewport-not-an-allocation.md)).
  A 1080p window beside a 4K one renders at its own size, inside the 4K allocation.
- **Off while windowed**, chosen, not measured: motion blur, the jitter and the temporal record (so
  FSR 3.1 falls to FSR 1, as on a menu frame) and dynamic resolution. Each holds one camera's history
  or times one pass, and the windows alternate cameras every frame. A split drops the same three.
  Pulse's bloom and HD's and Omega's chains are *kept*, which a split cannot do.
- **The flyby and the end.** The pre-race flyby is the player's camera and **every window that
  holds a seat plays it** (2026-10-14, maintainer's request): `Race::window_drawn` answers the
  player's slot alone for each such window while `in_intro()` holds, so each draws the same flyby
  camera full window at its own size and aspect (`Scene::render_view`, as window 1 always did),
  and each goes back to its own seats, cut by its own cut, on the tick the flyby ends. A window
  with no seat keeps its holding frame. **Chosen, not measured**; it is view-side only, nothing the
  tick reads or the hash holds (`World::tick` stays 0 under the flyby). The flyby and its skip are
  one race state, so a skip (a held Cross, or a click on window 1; the seat windows' pointer is
  ignored, as everywhere) ends it on every window at once, and the HUD, which `Race::hud_shown`
  already hid race-wide through it, is off on every window. The track panel (circuit name and
  blurb) is part of the flyby but belongs to window 1, so the other windows show the bare camera.
  Once every person has finished, the window that holds the player shows the end screens over the
  spectator camera (`Race::window_drawn` is the player alone, as `views_drawn` is one in a split)
  and the other windows keep their chase cameras and HUD. **Not exercised live:** a finish with
  several windows (a headless run seats people nobody drives, so no one finishes, and a windowed run
  under software Vulkan would need a four-minute race at 8 fps); read from `Race::finished` (every
  *flown* person), `window_drawn` and `RaceStage::draw_hud`/`draw_window_hud` only.
- **Pause and the resume countdown** (`oag_game::pause`, a multiplayer race only). The race is held
  for the whole `pause::State::frozen` - `Paused` and `Resuming` alike, the gate `Session::draw` reads
  for window 1 - so every window draws the tick exactly (no between-ticks blend) while held and blends
  again once it runs. The pause line ("PLAYER 1 PAUSED - ANYONE CAN RESUME IN 0:28") is on window 1.
  **The resume countdown is drawn in every window**, one picture per view the window cuts (the split's
  one-per-view rule, per window): one over the whole of a one-seat window. Looked at under
  Xvfb in a `--race --windows 2` race, Start on window 1 to pause and again to resume: window 2 froze on
  its frame (timer unchanged across the pause and the countdown), drew the countdown art beside its
  craft while window 1 did, and ran again when the countdown ended. A title with no countdown art
  (Pure, HD, 2048, Omega) says the count as text in its `HUD` face in the same places, with the split
  placement per window's own views (30% down each view of a window of several seats; the authored
  63% in a window of one, `pause::anchor_y`), see [split-screen.md](split-screen.md). Looked at on
  HD under Xvfb, `--window-seats 2,1` (the one-seat window is the whole-screen case) and `2,2` (four
  views in two windows): `3`, `2`, `GO` in every view. **Open:** the pause line on every window.
- **Cost.** A frame draws the scene once per window, and the camera-independent work (shadow gathers,
  scenery clocks, advert cards) once per window rather than once, because each window is a whole
  frame. A split shares it between views. Measured below.

## The layout screen

**Chosen, not measured, all of it.** After the join screen's START (two or more people seated, a
desktop build, a title that offers a split on this adapter), the **WINDOW LAYOUT** screen opens
before Track and Ship Select; its START goes on to the race box exactly as the join screen's did,
and its BACK returns to the join screen with everybody still seated. It is `oag_ui_screens::window_layout`
(model, drawing, pad, keyboard and pointer) over the join screen's rows, on the title's own menu
skin and font (HD's and Omega's row blocks where the title draws them), driven by
`session/window_layout.rs`.

| Row | What it does |
| --- | --- |
| WINDOWS | 1 to the head count (8 at most). 1 is today's split screen in one window. A new window takes one seat from the fullest window and the next monitor; a window that goes hands its seats to the one before it |
| WINDOW MODE | WINDOWED or FULLSCREEN: the DISPLAY page's own `display.window_mode`, applied (with window 1's monitor) at START and only when it changed, so the two pages never disagree. Seat windows follow it (`windows_fullscreen`) |
| WINDOW | Which window the rows below edit (more than one window only); a click on a box in the preview picks it too |
| SEATS | How many seats that window holds, shown as `PLAYER 1 - 3`. Moving one takes from or gives to the next window; every window keeps one seat, and the seats add up to the people |
| CUT | HORIZONTAL or VERTICAL, **only for a window of exactly two seats** (the one count a cut means anything for). With one window and two people it is the split screen's own cut |
| MONITOR | Which monitor that window sits on, from winit's list (`Window::available_monitors`, names as the DISPLAY page spells them). Only when the machine has more than one. Window 1's is the DISPLAY page's `display.monitor` |

The preview is a box per window (16:9), its seats cut up by `oag_render::viewport::layout`
(`split_screen::seat_cells`, so it is the picture a window draws), numbered by seat, the edited window
lit, with the window's monitor under it. **Input**: up and down move the cursor, left and right change
the value under it, confirm steps it on (and acts on START and BACK), START goes on, circle or Escape
goes back; the pointer hovers rows, a click steps a value or acts, the wheel over a row changes it, a
click on a preview box picks that window, the secondary button goes back (a touch tap is a click).

**What START writes is what `--window-seats` writes**: `Session::window_layout` (one `WindowSeats` per
window, `split_screen::layout_of`), `windows_wanted` so the event loop opens the windows on its next pass
(`Session::screens_to_open`, which now also tops up windows already open; windows past the chosen count
are dropped with no seat handed to anyone, they only had holding frames), `window_monitors` (each further
window's monitor, by name) and window 1's cut. One window clears the layout, so the race is the split it
always was. A window for the monitor window 1 is on goes beside window 1 (a request: X11 honours it,
Wayland places it); a window for another monitor is centred on it, stepped aside for each window already
there; `--windows-fullscreen` or WINDOW MODE FULLSCREEN puts it borderless on its monitor.

**A chosen layout is for its split race only.** `Session::window_layout_chosen` keeps the menu race
seating (`seat_the_windows`, which seats everyone a `--windows` layout plans for) from reading it: a later
one-person race from RACEBOX stays one person in window 1, and the windows still open show their holding
frames. A restart from the end screens keeps the seats and so the layout. A race parked behind the pause
menu keeps its seats (existing behaviour, `unseat_after_split_race`), so a race started from that menu is
still the split; walked on Pulse. Choosing fewer windows while a race is parked closes the dropped windows
through `close_screen` (their seats go to the AI and they leave the parked race's layout); with no parked
race they only had holding frames and are dropped as they are. A launch in the same frame as START (a
title with neither Track nor Ship Select) counts the windows asked for, not only those already open
(`Session::open_windows`).

**Remembered with no save slot**, as the RACE page's choices are: `[race] window_seats` (the
`--window-seats` spelling, `"3,1"`, `"4,2v"`, empty for one window) and `window_monitors` (names) in
`settings.toml`. The screen starts from the layout this run already has (`--windows`), else the remembered
one, else one window; it is **re-validated each time it opens**: another head count is dealt anew over the
same windows keeping each cut and monitor (`Picker::new`), more windows than people is cut down, and a
monitor that is gone is monitor 1. The command line still wins for its own run.

**Where it is not offered.** The browser, Android and iOS get the one surface the platform makes (the
opener is compiled out), so the screen is **skipped** there and the join screen's START goes straight on.
On an integrated GPU the HD, 2048 and Omega splits are not offered at all
(`Title::split_on_integrated_gpu`), so neither the join screen nor this one is reachable; Pulse and Pure,
which the rule exempts, get the screen with every window count.

`--menu-page window-layout[:PEOPLE[:WINDOWS[:MONITORS[:EDITED]]]]` draws it as a still (4 people, 1
window, 1 monitor, the first window by default); the monitors are made up (`MONITOR n`), the rest is the
live screen's. **Looked at**: Pulse EU, Pure EU, HD EU, Omega EU and 2048 (Vita) stills at 1440x816 with
2 to 8 windows and 1 to 2 monitors; and live under Xvfb with software Vulkan, one keyboard and three
virtual pads (`scripts/uinput-pads.py`, `OAG_PADS=3`): four people joined, WINDOWS 2 seated them 3 and 1
(window 1 one seat over two, window 2 whole), START opened window 2 (`PLAYER 4`) while Track and Ship
Select ran, the race logged `multi-window: 2 window(s) holding 3, 1` and each window drew its own
craft; a second run laid 2 and 2 with a vertical cut chosen by mouse clicks (window 1 stacked, window 2 side
by side); a fresh process re-opened on the remembered 3 and 1; `--windows 2` then WINDOWS 1 closed window 2
and wrote an empty `window_seats`. **Not exercised**: more than one real monitor (Xvfb's RandR keeps one,
`xrandr --setmonitor` is not kept), FULLSCREEN, and a monitor that has gone since the layout was saved
(the clamp is a unit test); Wayland placement.

## Windows

`App::window_event` routes by `WindowId` (`windows::which`, a pure function with a test):

| Event | Window 1 | A seat window |
| --- | --- | --- |
| Close request | Quit, as ever | Every seat it held goes to the AI, the window closes |
| `Destroyed` | (ignored) | The same as a close: under X11 with no window manager a window is destroyed outright |
| Resize | The surface; zero-sized pauses as ever | The surface; zero-sized is ignored and **never pauses** |
| Keyboard | The shared keyboard | The same keyboard |
| Pointer, touch, drops | The front end's | Ignored |
| Focus | See below | See below |

- **A closed window's seats are flown by the AI** even though their pads are still plugged in, and a
  pad that reconnects does not take one back. For each slot the window held `Assignment::unseat(slot)`
  benches the pads on it, moves a keyboard that was seated there back to slot 0 and closes the slot to
  `reconcile`; the existing `split_screen::follow_devices` then flies the craft as `Controller::Ai`
  from the next tick. The window leaves the race's layout (`Race::remove_window`) and the planned one,
  so the windows after it keep their seats; the race keeps its cameras and slots. The pinning test is
  `pad_join_end_to_end::a_closed_window_hands_all_its_seats_to_the_ai_and_the_pads_cannot_take_them_back`.
- **Focus.** With one window a lost focus is acted on at once, as it always was. With seats it is
  only "away" when **no window of the run has it** (`windows::Focus`), after a 250 ms grace (winit
  reports `Focused(false)` for one window and `Focused(true)` for the other, not always in one pass
  of the loop). Clicking from window 1 into window 2 does not pause the race.
- **Every window opens the same way** (fixed 2026-10-14, reported from play on niri, where window 1
  floated and the seat windows were tiled). Window 1 and each further window are built by one
  function, `window::attributes` (title, size, icon, app id, fullscreen mode and monitor, and the
  resizable rule): **a windowed window is fixed size** (`with_resizable(false)`, min = max, the
  signal a tiling compositor floats a window on, measured under niri), a borderless one is
  resizable and fullscreen. The mode is the profile's WINDOW MODE, or `--windows-fullscreen` / the
  layout screen's FULLSCREEN for the run (`Session::seat_mode`). Checked under Xvfb: both windows
  of `--windows 2` carry `WM_NORMAL_HINTS` minimum = maximum = 1440x816, where the build before
  the fix gave window 2 no min or max at all.
- **Monitors.** Each further window goes on the monitor the layout screen chose for it
  (`window_monitors`, by name; a name that is gone is the first monitor). With no layout chosen
  (`--windows`) the windows are **spread over the monitors in winit's order, starting from the one
  window 1 is on** (the profile's `display.monitor`, the first by default) and wrapping, so window
  1 on monitor 1 puts window 2 on monitor 2 and window 3 back on monitor 1. Borderless covers that
  monitor. Under a nested weston with two outputs, `--windows 2 --windows-fullscreen` put window
  2 borderless over all of the second output (1280x720).
- **Windowed placement is a request, and the platforms differ.** X11 and Windows honour
  `with_position`: a windowed window for window 1's monitor opens beside it, one for another
  monitor is centred there and stepped aside for each window already on it. **Wayland has no
  protocol for a position**: the compositor places every windowed window (a tiling one floats it
  because it is fixed size) and only the monitor of a borderless one is honoured. **`--windows-fullscreen` was not exercised** (no second monitor, and no `xrandr
--setmonitor` run under Xvfb); it is the winit calls `Fullscreen::Borderless(Some(monitor))` per window
and nothing more. **Open:** `--window-monitor`, a monitor per window by name.
- **A window that cannot open** is said in the log and skipped; the run goes on with the windows it
  has.

## Platforms

Desktop only. The browser, Android and iOS give the program the one surface the platform makes, so
`--windows` there is **an error** (`DriverArgs::window_count`, behind `cfg(wasm32 | android | ios)`)
rather than a flag silently ignored, and `Gpu::open_output`, the seat opener and the present modes
are compiled out. Checked: `cargo check -p oag-game --example oag_web --features web --target wasm32-unknown-unknown`
builds (no warning of this lane's). **Not checked: Android and iOS** (no NDK or iOS SDK here).

## Pacing (chosen, not measured on real displays)

Presenting several FIFO surfaces from one thread waits on each display's vblank in turn, so with
two 60 Hz monitors a FIFO window 2 would cap the whole loop at the slower display and add a blocking
wait before window 1's own. The cut avoids it by construction: **window 1 follows the VSYNC row
and paces the loop; every further window asks for `Mailbox`, then `Immediate`** (`window::seat_present_modes`),
neither of which blocks, and falls to `Fifo` - and says so in the log - only when the surface offers
neither. Every `windows::PACING_LOG_FRAMES` (600) frames the log says each window's mean swapchain
acquire wait and present time and its present mode:

```
windows: frame 16.7 ms, window 1 acquire+present 9.80 ms, window 2 acquire 0.02 ms present 0.31 ms (Mailbox)
```

**What was measured (Xvfb, software Vulkan `lvp`, two windows, `vsync = "off"`):** window 2 got
`Mailbox` (the X11 software surface offers it) and its acquire wait stayed at 0.03 to 0.12 ms on
every logged interval, against window 1's 0.12 to 1.23 ms of acquire plus present, including
intervals on a machine at load average 77 where the whole frame took 380 ms. A seat window never
made the loop wait. That shows the design does not add a *blocking* wait on a seat; it says nothing
about a real display's refresh, which `lvp` does not have.

**What could not be measured here.** A virtual display has no vblank, so the per-display wait cannot
be seen under Xvfb (and RADV cannot present on it at all). On the maintainer's AMD RADV desk with two
real monitors:

```sh
RUST_LOG=warn,oag=info oag-game --race --windows 2 --no-audio 2>&1 | grep 'windows:'
```

Read window 2's `acquire` and the mode in parentheses. An acquire near zero and `Mailbox` is the
design working. An acquire near a refresh period, or `Fifo` in the parentheses, means the surface
offered no non-blocking mode and window 1 is waiting on window 2's display too; then try
`vsync = "off"` for window 1's row, or set the second monitor's refresh to match.

## Verification

- **Single-window and split frames are byte-identical to `main`** (`201dae2ab`): `--race --screenshot`
  of one player, `--players 2` and `--players 4`, on Pulse (PSP EU) and HD/Fury (PS3 EU), at ticks 300
  and 1200, PNGs compared with `cmp`: twelve of twelve identical. This proves the headless capture
  path (`race_capture`), which both share with the window for the scene (`Scene::render` and
  `Scene::render_viewports` are untouched in what they draw), the HUD and the readback. **It does not
  cover `session/draw.rs` or `app.rs`**, which `--screenshot` never runs; for the window path there is
  no byte-level tool, and the check is the code (a race that is not windowed takes the old branch in
  every `if windowed`) and the real windows below. **Re-run after the seat-list change** (which
  refactored `render_viewports` into `render_slots` and `hud_views::draw` to read a window): the same
  twelve against `main`'s `201dae2ab` PNGs, merged with the later `main`: twelve of twelve identical.
  A `--windows 2` capture (one seat per window) is byte-identical to the first cut's.
- **Headless per-window frames**: `--windows 2 --screenshot shot.png` writes `shot-window1.png` and
  `shot-window2.png`, each through `Scene::render_view` and the HUD of its slot. Read at 1440x816 on
  Pulse and HD at ticks 300 and 1200: each window shows its own craft from behind and its own times,
  speed and shield.
- **Seat lists, headless**: `--window-seats 4,2v --screenshot` on Pulse and HD at ticks 300 and 1200,
  read at 1440x816: window 1 holds four seats in quarters, window 2 two side by side; each view has its
  own craft and its own HUD, and on HD window 2's two-seat window uses the title's authored two-player
  layout while window 1's four use the split-HUD rules for four.
- **Seat lists, real windows**: Xvfb + `lvp`, `--race --window-seats 3,2`: window 1 held three seats
  (one over two), window 2 two stacked, window title `PLAYER 4 - 5`; closing window 2 logged
  `window 2 closed: slot(s) [3, 4] flown by the AI` and window 1 kept running its three.
- **The flyby on every window** (2026-10-14, lane `window-flyby`). Headless, `--race --window-seats
  2,1 --intro-ticks 100 --screenshot` on Pulse EU (Talon's Junction White): window 2's frame, which
  drew its own craft's chase camera through the flyby before, is now the flyby camera, the same
  shot window 1 draws (window 1 adds the track panel); window 1's frame is byte-identical to the
  build before, and so are `--players 1`, `2` and `4` at flyby ticks 100 and 600 (six of six, `cmp`).
  Live under Xvfb with software Vulkan (`pause_on_focus_loss = false`, since with no window manager
  the first focus loss froze the race before the flyby ran), `--race --window-seats 2,1` on Pulse EU,
  one root capture holding both windows per frame (so the frames are simultaneous): both play the
  flyby through its shots at the same time; **a click on window 1 skipped it and the next frames
  show window 1 in its two views (top and bottom) and window 2 whole in the same capture**, then the
  `3`, `2`, `1`, `GO` countdown in every view of both windows and the craft run. This project loads
  a flyby only for Pulse PSP (`start_grid.vex`, [race-intro.md](race-intro.md)): on HD EU
  `begin_intro` is false and a capture at `--intro-ticks 100` is the grid, so `--window-seats 2,1`
  there is as before, window 1 split and window 2 whole, read live too. The disc-backed pin is
  `raceplay::window_flyby_ground_truth` (every window, a window with no seat, the split after, and a
  skip ending it everywhere); both tests fail on the build before this change (window 2 answers its
  own seats `[2, 3]` through the flyby).
- **Real windows** under Xvfb with software Vulkan (`lvp`), two windows, a race on Pulse: both windows
  draw their own craft with their own HUD; window 1's craft sat on the grid (its keyboard idle) while
  window 2's, flown by the AI, ran at 554 km/h; closing window 2 (`xdotool windowclose`, which is a
  `Destroyed`) logged `window 2 closed: slot 1 is flown by the AI` and left window 1 running.
- **The front end and a menu-launched race**, Xvfb + `lvp`, `--windows 2` with no `--race`: the
  front end ran on window 1 and window 2 showed the holding frame (`PLAYER 2`) through the language
  picker, the title screen and the main menu; a race launched from RACEBOX (Time Trial, Talon's Junction
  White, keys sent to window 1) seated two people, took window 2 from the holding frame to its craft,
  and flew slot 1 as the AI (`split screen: slot 1 is now Ai`, no second device); Escape parked the race
  behind the pause menu on window 1 and window 2 froze on the parked race (two captures five seconds
  apart were pixel-identical in window 2's timer). **Not walked:** leaving the parked race for the menus
  to see window 2 go back to its holding frame (it is `windowed_race()` going false, no more).
- **Tests**: `windows::tests` (routing, close, focus, the shared allocation, a window's name),
  `split_screen::tests` (`deal`, `parse_window_seats`, `layout_for`, `views_for`),
  `raceplay::tests::windowed` (each window cuts its own seats by its own cut; a one-seat window is
  whole; a seat with no camera is skipped; removing a window; each window does a frame's shared work
  once and puts the viewpoint back, for one seat and for several), `input::pad::tests::an_unseated_*`,
  and the end-to-end close test above (a window of two seats).

## Cross-title check

HD is looked at on both paths. **2048 and Omega**: the window path is title-blind (it calls
`Scene::render_view`, whose Omega branch runs `run_omega` per window), so *checked, applies*; an
Omega capture (below) ran.

- **Omega EU** (`data/extracted/ps4`, the default circuit), `--race --windows 2 --ticks 300`:
  both windows drew, each its own craft and its own whole-screen HUD, through Omega's tone map
  (`run_omega` per window): *checked, applies, works*. **2048**: same code path, no 2048 capture was
  taken here: *checked, applies, not captured*.

- **The layout screen** is title-blind (the title's skin, font and row blocks only): *checked, applies*
  on Pulse, Pure, HD, Omega and 2048, each looked at as a still. Omega's light page inks its text black,
  so the preview's unselected seat numbers are drawn white there (`window_layout::legible`); 2048's
  placeholder menu leaves no room under the title, so the hint goes under the rows as the join screen's
  does. Only the live walk above ran on Pulse; the others share the code and were not walked.

## Open

- **The authored two-player HUD when the layout mixes counts**: the load packs the title's authored
  two-seat HUD only when told two views (`split_screen::views_for` says two when any window holds
  exactly two), so a layout of `4,3` has no window using it and a layout with a two-seat window uses it
  for that window alone; every other count uses the split-HUD files or the scaled whole-screen layout.
- **A keyboard per window**: the keyboard seat follows the focused window, so two people can share one
  machine with a keyboard each (or a keyboard and a pad). Today the keyboard is slot 0's.
- **The pause overlay on every window**, and the end screens on a seat's own window (the flyby is on every window since 2026-10-14).
- **`--window-monitor`** on the command line (the layout screen chooses monitors; the flags still do not) and remembering the windows' sizes and places.
- **Per-window motion blur, FSR 3.1 and dynamic resolution**: each needs its own history per camera
  (a `MotionState` and a temporal record per window) and a timer per window.
- **Camera-independent work once per frame** across windows, as `render_viewports` does for a split.
- **Pacing on real displays**, the command above.
- **Monitor hot-plug** and a window dragged between monitors of different scale are not exercised.
- **Seat windows' own settings** (render scale, screen filter per window): they follow the profile.
