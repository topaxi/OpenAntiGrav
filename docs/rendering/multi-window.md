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

**The flags.**

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
- **The flyby and the end.** The pre-race flyby is the player's camera: the window that holds the
  player (window 1) plays it full screen where a split would, and the other windows show their own
  craft's chase camera from the first tick. Once every person has finished, the window that holds
  the player shows the end screens over the spectator camera (`Race::window_drawn` is the player alone,
  as `views_drawn` is one in a split) and the other windows keep their chase cameras and HUD.
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
- **Placement.** Seat windows are asked to sit beside window 1 at creation (X11); the compositor
  places them under Wayland, which cannot place a plain window. `--windows-fullscreen` puts window
  `k` borderless on the platform's `k`-th monitor (they share when there are fewer), which Wayland
  honours. **`--windows-fullscreen` was not exercised** (no second monitor, and no `xrandr
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

## Open

- **The layout picker**: a menu to choose the windows and the seats per window, per screen, and a
  per-window cut chosen there. This cut has the two flags only (`--windows`, `--window-seats`); the
  model (`Race::windows`, `WindowSeats`) is what a picker would write.
- **The authored two-player HUD when the layout mixes counts**: the load packs the title's authored
  two-seat HUD only when told two views (`split_screen::views_for` says two when any window holds
  exactly two), so a layout of `4,3` has no window using it and a layout with a two-seat window uses it
  for that window alone; every other count uses the split-HUD files or the scaled whole-screen layout.
- **A keyboard per window**: the keyboard seat follows the focused window, so two people can share one
  machine with a keyboard each (or a keyboard and a pad). Today the keyboard is slot 0's.
- **The pause overlay on every window**, and the end screens on a seat's own window.
- **`--window-monitor`** (a monitor per window by name) and remembering the windows' places.
- **Per-window motion blur, FSR 3.1 and dynamic resolution**: each needs its own history per camera
  (a `MotionState` and a temporal record per window) and a timer per window.
- **Camera-independent work once per frame** across windows, as `render_viewports` does for a split.
- **Pacing on real displays**, the command above.
- **Monitor hot-plug** and a window dragged between monitors of different scale are not exercised.
- **Seat windows' own settings** (render scale, screen filter per window): they follow the profile.
