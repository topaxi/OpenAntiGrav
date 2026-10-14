# Multi-window local play

`--windows N` (2 to 8): one process, one race, one tick, and **N operating-system windows
instead of N views in one window** - each person's whole-screen view in a window of their
own, for a desk with more than one monitor. Split screen ([split-screen.md](split-screen.md))
is the other way to seat the same people; the two share every camera, HUD and input slot, and
differ only in where view `i` goes. The decision record is
[ADR-0067](../architecture/adr/0067-a-multi-window-race-is-one-device-and-one-submission-per-window.md).

**Chosen, not measured, all of it.** The original has no multi-window mode and nothing here is
read off it. First cut, 2026-10-14 (lane `multi-window`).

```sh
oag-game --race --windows 2                      # a race for two people, a window each
oag-game --windows 3                             # the front end on window 1; a race seats three
oag-game --race --windows 2 --windows-fullscreen # each window borderless on a monitor of its own
oag-game --race --windows 2 --screenshot shot.png --ticks 300   # headless: shot-window1.png, shot-window2.png
```

## Who sits where

Seated the way `--players N` seats people ([split-screen.md](split-screen.md), "Who plays which
slot"): the keyboard and the first pad on slot 0, each further pad on the next slot in the order
the pads connect. A slot nobody drives is flown by the AI. **The keyboard is one device whichever
window has the focus**; a per-window keyboard seat is open.

- **`--race --windows N`**: slots `0..N` are people from the first tick, window `k` shows slot
  `k - 1`.
- **A front-end run** (`--windows N` without `--race`): window 1 is the front end; windows 2 to N
  open on the first pass of the event loop once the title is known, and show a **holding frame**
  (black, the join screen's own word for a person and the seat's number: `PLAYER 2`) until a race
  starts. A race launched from the menus seats everyone who has a window - unless the join screen's
  lobby or a restart's kept seats say how many people race, or it is a campaign race, which is one
  person's - and fields opponents for the others to race. Back on the menus the windows go back to
  their holding frames. The text is the existing `OAG_JOIN_PLAYER`; no new string.
- **More people than windows** (the join screen seats three, two windows are open): the race is not
  windowed; it is the split screen in window 1, so everybody is seen. A window closed in the middle
  of the run ends the run of windows: slots past the gap are not seated.

`--windows` and `--players` do not combine (a clap conflict). On Wipeout HD, 2048 and Omega the
integrated-GPU rule is `--players`' own (`Title::split_on_integrated_gpu`,
`oag_game::split_screen::offered`): on an integrated GPU those titles open one window, said once in
the log, and Pulse and Pure open them all.

## What a window draws

Every window runs **the whole single-window chain on its own framebuffer** (`upscale::Framebuffer`,
at the window's own size): scene, resolve, whole-screen HUD, Pulse's bloom over the HUD, the
screen filter and the grade. Window 1 is `Session::draw` as ever; `Session::draw_seats` runs after
it and draws each further window the same way (`session/windows.rs`). A window is a full screen, not
a split view: the title's whole-screen HUD for that window's slot (`hud_views::draw` with the slot),
the countdown, and - on window 1 only - the track panel, the results and the end screens.

The scene is `oag_raceplay::Scene::render_view`, next to `render_viewports`, called by the window
and by the headless capture alike. It is a view of slot `s` through `Layer::Window`: it clears its
target and does all the camera-independent work itself, like a split's first view, and runs the post
chains (Pulse's bloom, HD's, Omega's tone map) like an unsplit race, because a window starts at the
origin and fills the target. The boost-zoom pulse is the player's own, so only the window that
follows the player's craft draws it.

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
- **The flyby and the end.** The pre-race flyby is the player's camera: window 1 plays it, and the
  other windows show their own craft's chase camera from the first tick. Once every person has
  finished, window 1 shows the end screens over the spectator camera
  (`Race::views_drawn` is one, as in a split) and the other windows keep their chase camera and HUD.
- **Pause.** The pause menu and the pause overlay are on window 1. Every window shows the frozen race
  behind it. **Open:** the pause overlay on every window.
- **Cost.** A frame draws the scene once per window, and the camera-independent work (shadow gathers,
  scenery clocks, advert cards) once per window rather than once, because each window is a whole
  frame. A split shares it between views. Measured below.

## Windows

`App::window_event` routes by `WindowId` (`windows::which`, a pure function with a test):

| Event | Window 1 | A seat window |
| --- | --- | --- |
| Close request | Quit, as ever | The seat goes to the AI, the window closes |
| `Destroyed` | (ignored) | The same as a close: under X11 with no window manager a window is destroyed outright |
| Resize | The surface; zero-sized pauses as ever | The surface; zero-sized is ignored and **never pauses** |
| Keyboard | The shared keyboard | The same keyboard |
| Pointer, touch, drops | The front end's | Ignored |
| Focus | See below | See below |

- **A closed seat is flown by the AI** even though its pad is still plugged in, and a pad that
  reconnects does not take it back. `Assignment::unseat(slot)` benches the pads on the slot, moves a
  keyboard that was seated there back to slot 0 and closes the slot to `reconcile`; the existing
  `split_screen::follow_devices` then flies the craft as `Controller::Ai` from the next tick. The
  pinning test is `pad_join_end_to_end::a_closed_window_hands_its_seat_to_the_ai_and_the_pad_cannot_take_it_back`.
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
  every `if windowed`) and the real windows below.
- **Headless per-window frames**: `--windows 2 --screenshot shot.png` writes `shot-window1.png` and
  `shot-window2.png`, each through `Scene::render_view` and the HUD of its slot. Read at 1440x816 on
  Pulse and HD at ticks 300 and 1200: each window shows its own craft from behind and its own times,
  speed and shield.
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
- **Tests**: `windows::tests` (routing, close, focus, the shared allocation, the windowed rule),
  `raceplay::tests::windowed` (the layout sees one view per window; each window does a frame's shared
  work itself and puts the viewpoint back), `input::pad::tests::an_unseated_*`, and the end-to-end
  close test above.

## Cross-title check

HD is looked at on both paths. **2048 and Omega**: the window path is title-blind (it calls
`Scene::render_view`, whose Omega branch runs `run_omega` per window), so *checked, applies*; an
Omega capture (below) ran.

- **Omega EU** (`data/extracted/ps4`, the default circuit), `--race --windows 2 --ticks 300`:
  both windows drew, each its own craft and its own whole-screen HUD, through Omega's tone map
  (`run_omega` per window): *checked, applies, works*. **2048**: same code path, no 2048 capture was
  taken here: *checked, applies, not captured*.

## Open

- **A menu row** for the window count. This cut has the flag only.
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
