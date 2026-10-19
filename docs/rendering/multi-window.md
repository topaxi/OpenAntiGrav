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
- **F11** in any window toggles that window alone between borderless fullscreen on the monitor it is
  on right now and windowed (back at its previous size and position). Drag each window to its monitor,
  press F11 in each. Per window and per session: the saved WINDOW MODE setting is not rewritten
  (**chosen, not measured**). F11 is never a game input and no keyboard-seat routing sees it. Checked
  under openbox on Xvfb (the window filled the screen and returned); a second monitor is the
  maintainer's to try. On the web F11 is the browser's own.

**Measuring without a person at it (2026-10-14, lane `split-preseat`).** Three dev switches, off by
default, none a setting (`oag_game::devrun`, **chosen, not measured**):

- `--preseat`: with `--players N`, `--windows N` or `--window-seats` and no `--race`, seat those
  people on placeholder devices (seat 0 the anonymous keyboard, the rest keyboards no key ever
  matches - no pads, no virtual input) and open the WINDOW LAYOUT screen as soon as the menus are
  up, pre-filled from the flags (`--players 2` is one window of two seats, so the CUT row is
  there). It is the real path: `mp_devices` seeded, `open_join`, then the join screen's START.
  **Explicit rather than implied by the flags**, because `--windows 2` has always opened its
  windows and waited for a person. **Landed on directly** rather than walked to (a walk is the
  boot's taps plus MULTIPLAYER, LOCAL MULTIPLAYER and two presses; direct is one frame and cannot
  drift with the menu's row order). A real pad still takes a free seat the normal way after BACK to
  the join screen; with every seat dealt (`--players 8`) it cannot until a seat is freed there.
  `--windows`/`--window-seats` still open their extra windows at boot, before the screen.
- `--script SCRIPT`: `SECONDS:BUTTON,...` taps (`--press`'s names, plus `quit`), in a normal
  windowed run - no `--screenshot`. `+SECONDS:BUTTON` counts from where `--preseat` landed, and the
  plain taps that got through the boot (`3:start,6:start,9:cross,12:start`) are dropped once it
  has, so a layout walk does not depend on the boot's length. Wall clock, one frame down and one
  up per tap.
- `--frame-log` (or `OAG_FRAME_LOG=1`): one `info` line a second on every stage, menus included,
  from the overlay's own meters: `frame-log stage=menu:window-layout(windows=1,cut=h) fps=
  frame_ms= p99_ms= cpu_ms= gpu_ms= present_ms= target_hz= window=WxH surface=WxH render=`, a `-`
  where a reading does not exist (the GPU scene pass on a menu), and one `size change` line
  whenever the window or surface size moves. Terminal and log file, with no overlay.

```sh
cargo build --release -p oag-game
oag-game data/images/pulse-psp-eu.chd --no-audio --players 2 --preseat --frame-log \
  --script "3:start,6:start,9:cross,12:start,+4:down,+5:down,+6:right,+16:right,+26:quit"
```

## Who sits where

Seated the way `--players N` seats people ([split-screen.md](split-screen.md), "Who plays which
slot"): the keyboard and the first pad on slot 0, each further pad on the next slot in the order
the pads connect. A slot nobody drives is flown by the AI. Slots are dealt to windows in order,
window 1 taking the first: `--window-seats 4,2` is slots 0 to 3 in window 1 and 4 and 5 in window 2.
The LOCAL MULTIPLAYER flow's **seat assignment screen** (below) can put any person in any pane: the slots a
window shows are then the ones its panes were given (`split_screen::window_slots`), the slots themselves,
and so the grid order, stay the join order.
**Keyboards are seats** (2026-10-14, lane `keyboard-seats`, **chosen, not measured**; it replaces the
`window-keyboard` lane's "the keyboard follows the focus", which put a pad player's craft under the
keyboard of whoever's window had the focus). In a race dealt across windows a keyboard is a seat like a
pad is, and a key goes to the keyboard seat of **the window it arrived on**
(`Assignment::route_key`, `Controls::window_key`):

1. Window 1's seat holds the keyboard the front end uses, as it always did.
2. **A key press in a window that holds no keyboard seat claims the window's lowest free seat** (a seat
   below the seated head count, not closed, with no pad and no keyboard on it; a pad that connects
   later takes the lowest seat that is not claimed). The press has to be a bound key (a modifier or an F-key claims
   nothing; Tab is Select, so it does). A release claims nothing.
3. **A window that holds no keyboard seat and has none to claim ignores keys.** This is the flaw it
   fixes: a keyboard player in window 1 and a pad player in window 2, and the focus moves to window 2:
   the keys go nowhere (window 2's only seat is the pad's) instead of onto the pad player's craft.
4. A single window (the front end, a race of one window, a split screen in one window) sends every key
   to its first keyboard seat as before; Escape and the pause menu stay window 1's.

What tells a keyboard apart depends on what the platform reports (**the capability table**):

| Platform | Keyboards told apart by | Two keyboards drive two seats | One keyboard, two windows |
| --- | --- | --- | --- |
| Wayland, a compositor with a `wl_seat` per window (sway `seat`) | The window each seat's keyboard has focus in | **Yes**, each window's player flies at once (live, below) | A key claims the seat of the window it arrives in |
| Wayland, one seat (niri, GNOME, KDE) | Nothing: winit reports one dummy device id | No: one keyboard has one focus | **Only the focused window's keyboard player moves** (stated limit); each window's player is a seat claimed by pressing there |
| X11 | The raw key event's source device (`DeviceEvent::Key`, XInput2 `sourceid`) | **Yes**, from the moment a second keyboard presses a bound key, whichever window has the focus (live, below) | Until a second keyboard speaks, as the one-seat Wayland row |
| Windows | The raw input device handle on `DeviceEvent::Key` | Implemented the same way, **not tested here** (recipe below) | As the X11 row |
| macOS | Nothing (no per-keyboard id) | No | As the one-seat Wayland row |
| Android, iOS, web | Nothing | No | One window |

**Why raw events.** winit 0.30.13 reports a per-keyboard id only on `DeviceEvent::Key`; the window's
`WindowEvent::KeyboardInput` carries the virtual core keyboard on X11 (`x11/event_processor.rs`,
`VIRTUAL_CORE_KEYBOARD`) and a constant on Windows (`windows/event_loop.rs`, `DEVICE_ID`), and
Wayland's id is a unit dummy. A raw event names no window, so it is read against the window with the
focus; it carries a `PhysicalKey` only, which `oag_input::keys::key_at` reads by its place on a QWERTY
board (**limit**: AZERTY's `Z` sits where QWERTY has `W`, so the per-device path reads by position and
the window path by letter; `Y` and `Z` are both Circle, so QWERTZ agrees). Per-device routing is
**inert until a second keyboard presses a bound key** (`Controls::device_key`): with one keyboard the
window's own events carry the keys, so a single keyboard can still claim a seat in each window as
above. From then on the window's key events are the same presses again and a windowed race drops
them (`Controls::per_device_keys`); each keyboard is bound to the seat it pressed in (the earlier one
keeps what it was driving and the keys it holds, a later one claims the focused window's free seat, or
the lowest free seat of the race) and keeps it whichever window has the focus. Raw key events arrive
while one of our windows has the focus (winit's `DeviceEvents::WhenFocused`), not while another
application does. Escape, typing and a rebind stay on the window path.

Rules that carry over from `window-keyboard` and what became of them:

- **Every window's first seat counts as driven** (`Assignment::set_keyboard_reach`, which
  `Assignment::drives` reads), **kept**: a seat nobody has pressed a key at yet idles rather than being
  handed to the AI by `follow_devices`, and a pause owner's claim does not lapse. **Consequence:**
  `--window-seats 1,1` seats *two people*: the window nobody has pressed in sits idle (it used to be
  AI-flown), and the race's end screens wait for both to finish. A window's later seats are the AI's
  unless a pad or a press seats them.
- **A key held as a window loses focus is let go** (`Controls::let_go_window`, `Keyboard::let_go`),
  **kept, per window now**: the keyboard seats that take that window's keys release, and each key stays
  released until it is seen to come up, because X11 reports held keys to the window that gains focus as
  synthetic presses and keeps delivering the auto-repeat as ordinary presses. The other windows' seats
  keep their keys (on a compositor with a seat per window the other window's player is still typing),
  and so does a seat bound to its physical keyboard. In a windowed race synthetic and repeated key
  events are dropped; when no window has focus for a quarter of a second every keyboard is released and
  the next press counts again.
- **Dropped:** moving the keyboard to the focused window's seat (`keyboard_seat`, `keyboard_to`,
  `sync_keyboard_seat`, the "keyboard is now on slot N" log line) and with it the single keyboard that
  roamed on X11 and Wayland alike. One keyboard now holds one seat per window it has pressed in; on a
  one-seat compositor only the focused window's seat moves.
- A race parked behind window 1's pause menu lets go of the other seats' keys every frame
  (`Controls::let_go_extra_seats`), because the menu reads window 1's keyboard only and a key released
  meanwhile would come back held on resume.
- **Pointer taps** (a click is a tap on the keyboard's latch) stay on the first keyboard seat, window 1's.
- **The flyby's skip is any flown person's** (2026-10-14, `flyby-panel`): `tick_intro` ends the flyby
  while any slot a person flies (`World::person_flown`, plus the primary) holds Cross, so a keyboard or
  pad seat at window 2 skips it, as a click on window 1 does. **Chosen, not measured**; it is view-side
  (the flyby holds `World::tick` at 0) and a single person's race reads the same slot as before. A
  single-window split screen's join
  screen seats one keyboard (`lobby::Device::Keyboard`); two physical keyboards in one window's split
  screen are not wired (open).

The log says what happened: `keyboard seat: a key in a window with a free seat claims slot N (Shared)`,
`keyboard seat: slot N is now bound to Device(KeyboardId(i))` and
`keyboards: a second keyboard spoke, race keys now come per physical keyboard`.

**Windows, test recipe (not run here).** Two USB keyboards, `oag-game --race --window-seats 1,1
--no-audio` with `RUST_LOG=warn,oag=info`. Press X on keyboard A with window 1 focused: no
`keyboard seat` line is expected, window 1's craft thrusts. Press X on keyboard B: the log must say
`a second keyboard spoke` and `claims slot 1 (Device(KeyboardId(1)))`, window 2's craft thrusts while
window 1's keeps thrusting, whichever window has the focus. If window 1's craft keeps thrusting after B
lets go of X, Windows delivers a key's window event before its raw event (measured the other way round on
X11) and the engagement in `Controls::device_key` needs reversing there. If A and B drive one craft, Windows reports
both keyboards under one raw device handle (a remote desktop, a KVM, or a keyboard that presents as a
hub) and the table's Windows row is wrong for that setup.

- **`--race` with a layout**: the people are in their seats from the first tick (`apply_windows`).
- **A front-end run** (a layout without `--race`): window 1 is the front end; the further windows
  open on the first pass of the event loop once the title is known, and show a **holding frame**
  (black, the join screen's own word for a person and the seats the window is planned to hold:
  `PLAYER 2` for one, `PLAYER 5 - 6` for two; the existing `OAG_JOIN_PLAYER`, no new string) until a
  race starts. A race launched from the menus seats everyone the layout plans for - unless the join
  screen's lobby or a restart's kept seats say how many people race, or it is a campaign race, which
  is one person's - and fields opponents for the others to race (not in a Time Trial of several people, which fields the people only, [mp-time-trial.md](../ghidra/functions/psp-pulse-usa/mp-time-trial.md#the-port)). **A head count other than the one
  spelled is dealt, not refused** (`split_screen::layout_for`: the join screen seating three over a
  `4,2` layout is `2,1`). Back on the menus the windows go back to their holding frames.
- **Fewer people than windows**: the windows nobody is dealt show their holding frame.

## What a window draws

Every window runs **the whole single-window chain on its own framebuffer** (`upscale::Framebuffer`,
at the window's own size): scene, resolve, HUD, Pulse's bloom over the HUD, the screen filter and the
grade. Window 1 is `Session::draw` as ever; `Session::draw_screens` runs after it and draws each
further window the same way (`session/windows.rs`). **What a window draws is the seats the race gives
it** (`Race::windows`, window `i` of the race is window `i` of the run; `Race::window_drawn`):

- **One seat**: a full screen, not a split view. The title's whole-screen HUD for that slot
  (`hud_views::draw` with the window), the countdown, the track panel while the flyby plays and fades
  (every window, see "The flyby and the end") and, once the race is over, the results and the end screens (every window, same section). The scene is `oag_raceplay::Scene::render_view`, next to
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
  project's split-HUD files, or the whole-screen layout scaled into each view). As in a split,
  Pulse's bloom runs once per view after the last and HD's and Omega's chains once over the window
  after its last view (see [split screen](split-screen.md), rules 4 and 6).
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
- **Motion blur, FSR 3.1 and dynamic resolution run per window while it holds one seat**
  (2026-10-14, lane `window-render-cost`, **chosen, not measured**; this replaces "off while
  windowed", which dropped all three because each holds one camera's history or times one pass).
  Each window now has its own:
  - **Camera history**, `Scene::history`: the previous tick's matrices the blur measures against
    (`MotionState`), the camera-jitter phase, the last-frame record the temporal upscaler reads and
    the cut watch, one set per window (`motion::MAX_HISTORIES`, 8). A shared set had three faults, each
    silent: a second window at the same tick and draw alpha is "the same frame" to `MotionState` and
    read window 1's previous view-projection; the jitter phase advanced once per render call, so
    with two windows each saw every other phase of an even sequence; and the cut watch alternated
    cameras. A window viewing another seat takes its camera cuts from that seat's respawns
    (`Race::viewed_camera_cuts`), not the player's.
  - **FSR 3.1 history**, in the window's own `Framebuffer`, fed that window's `Scene::temporal(window)`.
  - **Timers**, `session::timing::Rings`: the five GPU timestamp rings window 1 owns on `Session`
    (scene pass, blur chain, HD bloom chain, the two halves of FSR 3.1), claimed, abandoned and
    resolved by the same rules, read into a `drs::Cost` by the same `assemble` rule.
  - **One controller, one scale.** The windows' passes are serial on the one queue and the frame period
    is their sum, so the controller is fed the cost **summed over the windows of a frame**, once every
    window has come back for it (`Session::feed_windows`: a frame waits up to four frames for a late
    window and is then dropped, in order), and its one scale is applied to each window's own
    ceiling. The controller's model, cost goes as the pixel count, holds for a sum of windows each
    scaled by one fraction. (Independent controllers per window were the alternative: each would learn
    the others' cost as unmeasured fixed work and the windows would ratchet down together.)
  - **A window of several seats is a split and keeps dropping all of it**: its views are N cameras in
    one frame (a history holds one) and its scene pass has no one pair to bracket. While any window
    holds several seats every window holds its scale at the ceiling (`Session::drs_runs`, as a split
    screen does), and blur and FSR 3.1 stay on in the windows of one seat. A split screen in one
    window is unchanged. Pulse's bloom and HD's and Omega's chains are *kept*, which a split cannot do.
- **The flyby and the end.** The pre-race flyby is the player's camera and **every window that
  holds a seat plays it** (2026-10-14, maintainer's request): `Race::window_drawn` answers the
  player's slot alone for each such window while `in_intro()` holds, so each draws the same flyby
  camera full window at its own size and aspect (`Scene::render_view`, as window 1 always did),
  and each goes back to its own seats, cut by its own cut, on the tick the flyby ends. A window
  with no seat keeps its holding frame. **Chosen, not measured**; it is view-side only, nothing the
  tick reads or the hash holds (`World::tick` stays 0 under the flyby). The flyby and its skip are
  one race state, so a skip (any flown person's held Cross, or a click on window 1; a seat window's pointer is read
  only by the end screens, below) ends it on every window at once, and the HUD, which `Race::hud_shown`
  already hid race-wide through it, is off on every window. The track panel (circuit name and
  blurb) is drawn **on every window that plays the flyby**, in the title's own panel art
  (`oag_game::track_panel::Overlay`, one per race), laid out against that window's own first view
  (`RaceStage::draw_window_track_panel`: `Race::window_drawn(window)` and `window_cut(window)`, so
  the panel's 480x272 box fits the window's own size and aspect and letterboxes in a tall one). It
  runs on the one clock (`Race::track_panel_progress`, the flyby's ticks, then the world's while it
  fades), so every window fades it together behind its own seats. Window 1 and single-window frames
  are unchanged. Skipping is any flown person's, above.
  **The end screens are on every window that holds a seat** (2026-10-16, maintainer's request:
  "show the end race screen on all windows on multi window"; **chosen, not measured**, view-side only).
  Once every flown person has finished, `Race::window_drawn` answers the player's slot alone for every
  window with a seat, exactly as for the flyby (the spectator camera the end screens sit over), and each
  window draws `RaceStage::draw_end_screens` over it at its own size and aspect: `EndRace Results`,
  `Rewards`, `Menu` and `Race End Photo` for a title with the screens, the built-in results table
  for one without (Pure, a multi-person 2048 race, a source that did not read). A window with no seat keeps
  its holding frame. **One shared menu**: the flow is the one `RaceStage::endrace`, drawn into each window
  in turn (one submission per window, as ever), so there is one cursor and one set of rows.
  - **Pad and keyboard**: a press made on any seat's slot drives the menu. `Controls::fold_presses`
    hands every other slot's Up, Down, Left, Right, Cross, Start and Circle to the keyboard's slot (the one
    the menus read) in the finished arm of `Session::frame`, a tick later through the keyboard's latch,
    as a click's press is. Only there: in a running race those presses are thrust and weapons.
  - **Pointer**: each further window now has its own pointer latch (`Screen::pointer`, fed by that
    window's cursor, button, wheel and touch events; a left click is still also that seat's Start for the
    pause). The end screens read **one pointer per tick**: window 1's if it did anything, else the first
    further window's that did (`session::pointer::end_screen_pointer`), mapped through that window's own
    aspect rectangle into the screens' grid, so hit-testing is in the layout the window draws. A click in
    any window confirms Results and Rewards, and picks a menu row. The further windows' latches are
    taken every tick, so a click made mid-race is spent and cannot confirm the results. **No cursor is
    drawn on a further window** (window 1 draws its own arrow, `draw.rs`); the OS cursor is the player's
    there, and a hovered row highlights.
  - **Leaving**: RETURN TO MENU is `leave_finished_race` (the stage becomes the menus, nothing is parked),
    so every further window goes back to its holding frame the next frame; RACE AGAIN is `finish_launch`
    and the windows take their seats at the load as at any launch. Nothing stays on the end screen
    because there is no per-window state: a window shows the end screens only while the race on
    screen is finished.
  **Looked at (2026-10-16)**: headless `--race --players 2 --windows 2 --mode single_race --autopilot
  --screenshot s.png --ticks 12000` (Pulse PSP EU, a 3-lap finish at tick about 7,700, 1440x816 each): `s-window1.png`
  and `s-window2.png` both show the built-in results table over the spectator camera (before the change
  window 2 kept its chase camera and HUD). **Live under Xvfb on lavapipe** (`--race --players 2
  --windows 2 --autopilot`, resumed by a click, the race took about 8 minutes at about 0.3x speed): the
  race finished, window 1 and window 2 each drew the table at their own 1440x816 over their own spectator
  camera, and **a click in window 2 left the race** (the `--race` run quits, as the table says), which is
  the window 2 pointer reaching the screen. **Not walked: the disc's own EndRace screens (`Results`,
  `Menu`) in a second window.** That needs the front end (no `--race`) walked to a multi-window race
  and flown to its end, which this lane did not manage on lavapipe; the screens are drawn by the same
  `EndRaceRuntime::draw` with the window's own rectangle, which `--menu-page endrace-*` shows for window 1
  alone. RETURN TO MENU and RACE AGAIN with several windows are read from the code only.
  Checked against the title lineage: nothing here is title-specific (the end screens are the title's, the
  routing is generic), so 2048 and Omega take the same path (2048's touch pages are skipped in a multi-person
  race, so it draws the built-in table in every window).
- **Pause and the resume countdown** (`oag_game::pause`, a multiplayer race only). The race is held
  for the whole `pause::State::frozen` - `Paused` and `Resuming` alike, the gate `Session::draw` reads
  for window 1 - so every window draws the tick exactly (no between-ticks blend) while held and blends
  again once it runs. The pause line ("PLAYER 1 PAUSED - ANYONE CAN RESUME IN 0:28") is **on every window that holds a
  seat** (since 2026-10-14, `window-keyboard`; each draws it on its own whole-window grid over its
  graded frame). **Start on any window pauses with that window's seat as the owner** (the keyboard is
  at that seat), and **a left click on a window is that window's first seat's Start** under the owner
  rule (`windows::click_slots`; window 1's click was slot 0's and still is, a click on window 2 is
  its seat's, so it lifts player 2's pause and not player 1's); a seat window's pointer is otherwise
  ignored. There is no pause *menu* on a multiplayer pause. Escape parks the race behind window 1's
  front-end menus, and every other window keeps showing the frozen race **under the menus' own dim
  tint** (`PAUSE_OVERLAY`, 0.55 black, the alpha window 1's menu is drawn over the parked race
  with; a stopgap for the menu, **chosen, not measured**), HUD still readable beneath it. Looked at
  under Xvfb (Pulse EU, `--windows 2`, menus walked to a Racebox race, Escape on window 1): window 1
  the menu over its craft as ever, window 2 its craft and HUD dimmed and frozen. The menu itself on
  the other windows stays open.
  **The resume countdown is drawn in every window**, one picture per view the window cuts (the split's
  one-per-view rule, per window): one over the whole of a one-seat window. Looked at under
  Xvfb in a `--race --windows 2` race, Start on window 1 to pause and again to resume: window 2 froze on
  its frame (timer unchanged across the pause and the countdown), drew the countdown art beside its
  craft while window 1 did, and ran again when the countdown ended. A title with no countdown art
  (Pure, HD, 2048, Omega) says the count as text in its `HUD` face in the same places, with the split
  placement per window's own views (30% down each view of a window of several seats; the authored
  63% in a window of one, `pause::anchor_y`), see [split-screen.md](split-screen.md). Looked at on
  HD under Xvfb, `--window-seats 2,1` (the one-seat window is the whole-screen case) and `2,2` (four
  views in two windows): `3`, `2`, `GO` in every view.
- **Cost.** A frame draws the scene once per window, and **the camera-independent work runs once
  per frame however many windows draw** (2026-10-14, lane `window-render-cost`): the two shadow
  maps and the shadow gather, the scenery clocks and the advert cards. `Scene::render_window`'s
  `shared` flag says an earlier window of this frame already drew; it is the gate a later view of a
  split already used (`Scene::does_frame_work`), set by the first window that draws
  (`RaceStage::shared_frame_work`, cleared once a frame by `Session::begin_scene_frame`), so a window 1
  that draws nothing leaves the work to window 2. That window's encoder is submitted before the next
  window draws, so the maps are the textures it wrote. The `mapped` tier's depth map is fitted on the
  craft each window follows, so every window still redraws it. Particles, clouds, mist and the other
  effects advance per tick, not per render call (the clouds are gated by the tick), so there was no
  per-window simulation to hoist; what each window still does is camera-dependent (uniforms, LOD, sort,
  the particle and exhaust geometry built against its own right and up). Measured below.

## The layout screen

**Chosen, not measured, all of it.** After the join screen's START (two or more people seated, a
desktop build, a title that offers a split on this adapter), the **WINDOW LAYOUT** screen opens
before Track and Ship Select; its START opens the seat assignment screen below (unless the layout has a single view in all, which has nothing to
choose between and goes straight on to the RACE page), and its BACK returns to the join screen with everybody
still seated. It is `oag_ui_screens::window_layout`
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
Every title gets the screen with every window count, on every GPU class.

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

## The seat assignment screen

**Chosen, not measured, all of it** (2026-10-14, lane `mp-seat-assign`, step 5 of the LOCAL
MULTIPLAYER flow in `split-screen.md`). After the layout screen's START, when the layout has **two or
more views in all** (`split_screen::has_panes_to_choose`: several windows, or **one window holding a split**,
2026-10-14 `seat-assign-split`, so person 2 can take the top view), the **WHO SITS WHERE** screen opens before
the RACE page: which pane of the chosen layout each seated person flies in. Only a layout with a single view in
total skips it. The browser and the phones have no layout screen (`session::join` goes straight on there), so
the screen is not reachable on them: `open_seat_assign` needs the layout stage BACK returns to, and a split
there keeps the join order. **Open**: a `WindowLayoutStage` built from `remembered_cut` would open it; not wired
because it cannot be run here. It is `oag_ui_screens::seat_assign` over the join screen's rows
and the layout screen's preview (one shared `window_layout::draw_preview`, so the boxes and the cuts are the
picture the race draws), on the title's own skin and font, driven by `session/seat_assign.rs`.

- **A row per person** (`PLAYER 3  WINDOW 1 VIEW 2`; a window of one pane says just `WINDOW 2`), then AUTO,
  START and BACK. The preview's cells say which person is in each pane. The grid has room for eight rows, so
  **from six people up the person rows are left out** (AUTO, START and BACK only) and the numbered preview says who
  is where; with them, up to five people fit.
- **Each person moves their own marker with their own device**: left or up steps to the pane before, right or
  down to the pane after (window order, a window's panes in the order the race cuts them, wrapping), and
  confirm says "I sit here". Each device's presses are told apart as the join screen's are
  (`Controls::device_presses`, `Lobby::seat_of`; a raw keyboard that no seat names is the anonymous keyboard a
  click seated). A pane holds one person, so **moving onto a held pane swaps** the two and keeps everyone placed;
  whoever moved, and whoever was swapped, has to confirm again (dimmed until they do). START, or confirm from
  the last person to confirm, goes on; with somebody unconfirmed it is refused (the decline sound). Circle,
  Escape or the secondary button goes back to the layout screen with its choice as it was.
- **A fresh screen opens everybody confirmed**, on the last assignment where the devices are still seated and in
  join order elsewhere, so OK is enough. AUTO puts everybody back in join order, confirmed.
- **The mouse** has no seat of its own: a click on a pane (or a person's row) picks that person up, a click on
  another pane puts them there (swapping), and that click is the confirmation of both; a click on START goes
  on once everybody is confirmed.
- **What START writes**: `Session::pane_people`, who sits in each pane. A race built from the menus deals the
  windows by it (`split_screen::deal_windows(race, layout, order)`, `window_slots`): `--window-seats 2,1` with
  `pane_people = [0, 2, 1]` is window 1 showing slots 0 and 2 and window 2 slot 1, logged as
  `multi-window: 2 window(s) holding slot 0 + slot 2, slot 1`. **Grid order is untouched** (a person's craft is
  the slot their join order gave them, `rank_from_back`); only the pane that shows it moves, so a pad, and
  the keyboard of a window, drive the craft the pane shows. An order that is not every slot once is ignored.
  The windows' titles and holding frames name the people they hold (`PLAYER 1, 3`, `Session::planned_seats`).
  A race again, and a RETURN TO MENU race, skip the screen and keep `pane_people` (cleared when the flow
  ends, `Session::sync_multiplayer`, or the join screen is left).
- **One window holding a split** is dealt the same way: `load.rs` deals the views by pane whenever several windows
  are open **or the seat assignment ran** (`split_screen::deals_the_panes`), as a one-window `deal_windows`
  (`layout_for` over one window is `deal(people, 1, cut)`, the layout screen's `split_cut` for two seats), so
  pane `k` of the screen is view `k` of the race: the screen's `seat_cells` and the race's `window_rects` are the
  one `viewport::layout`. `--players N` leaves `pane_people` empty and keeps the plain split (view `i` follows
  slot `i`); the plain split does not carry an order, which is why an assigned single window takes the dealt
  path even for the identity order, so P1-on-top and P2-on-top draw by the same path. What `windowed()` changes
  for one window: window 1's views drive the HUD and end screens as in a multi-window race, a window of several
  seats is a split for the resolution controller (`Session::drs_runs`: scale held, as a split holds it), and
  keys are not read per window (`keys_by_window` needs further windows). The multi-seat window draws no blur, jitter or
  upscaler history (`RaceStage::render_window`: `effects` is for a window of one seat), as the plain split never did.
  Every LOCAL MULTIPLAYER race on a desktop with one window is now dealt this way (join refuses fewer than two people). Tests:
  `pad_join_end_to_end::one_window_split_puts_the_chosen_person_in_the_chosen_view` (2 horizontal, 2 vertical, 3
  and 4 views, order reversed: the race's rectangles equal the screen's cells, person 2 first),
  `split_screen::tests` (the skip rule and the deal gate).
- **Remembered on disk** with the layout: `[race] window_panes`, one device key per pane in pane order
  (`split_screen::device_key`: `keyboard`, `keyboard 2`, a pad's name, `name 2` for a second pad of that name;
  a `GamepadId` is per run). The next screen puts each device back in its pane
  (`split_screen::remembered_panes`); a device that is gone or new fills the panes left, in join order.

`--menu-page seat-assign[:SEATS[:ORDER[:UNCONFIRMED]]]` draws it as a still (`2,1` seats, join order by
default). **Looked at**: Pulse EU and HD EU stills at 1440x816; and live under Xvfb with software Vulkan and
virtual pads (`scripts/uinput-pads.py`, `OAG_PADS=3`): **Pulse PSP EU**, three pads joined, WINDOWS 2 seated
2+1; pad 3 stepped left (it and player 2 swapped and dimmed), pad 3 and pad 2 confirmed, the RACE page,
Track and Ship Select, and the race logged `seat assignment: pane 1 = player 1, pane 2 = player 3, pane 3 =
player 2` then `multi-window: 2 window(s) holding slot 0 + slot 2, slot 1`; holding pad 3's throttle drove the
craft in window 1's **lower** pane, holding pad 2's drove window 2's, window 1's upper pane stood still; a fresh
process opened the screen on the remembered assignment with everybody confirmed (OK went on) and logged the
same layout; window 2's title changed from `PLAYER 3` to `PLAYER 2` with the assignment. **HD EU**, two pads and
the keyboard: the keyboard's Left moved player 3's marker and swapped it with player 2's, START was refused
while they were dimmed, and both confirmed. The join screen's START row no longer shows the head count.
**Also looked at**: 8 people on Pulse and HD (`seat-assign:4,4`, `2,2,2,2`) and 5 on HD (`2,3`) as stills; a
first version drew a row per person for every head count and AUTO, START and BACK ran over the preview at 6 or
more (found by review, not by the 3-person walks). **The pointer, live (Pulse PSP, window 1)**: a click on pane 1
picked player 1 up, a click on pane 3 put them there and swapped player 3 into pane 1 (rows `PLAYER 1 WINDOW 2`,
`PLAYER 3 WINDOW 1 VIEW 1`), a click on AUTO put everybody back; **the keyboard as a person**: pad, pad and keyboard
joined, the keyboard moved its marker twice to the window-2 pane (`seat assignment: pane 1 = player 2 (xbox 360
controller), pane 2 = player 3 (xbox 360 controller 2), pane 3 = player 1 (keyboard)`, `multi-window: 2 window(s)
holding slot 1 + slot 2, slot 0`).
**One window holding a split, looked at (2026-10-14, `seat-assign-split`)**: `--menu-page seat-assign:2` and
`seat-assign:4` as stills on Pulse EU and HD EU (the `WINDOW 1 VIEW n` rows, a stacked pair and a 2x2 preview, 1440x816), and
live on Pulse PSP EU under Xvfb with software Vulkan and two virtual pads: both pads joined, the layout screen left at
WINDOWS 1 (HORIZONTAL), START opened the screen, pad 2's Left put it in view 1 (rows `PLAYER 1 WINDOW 1 VIEW 2`,
`PLAYER 2 WINDOW 1 VIEW 1`, both dimmed until confirmed, START refused), both confirmed, `seat assignment: pane 1 =
player 2 (xbox 360 controller 2), pane 2 = player 1 (xbox 360 controller)`, Track and Ship Select, the race logged
`multi-window: 1 window(s) holding slot 1 + slot 0`, and the frame at the start showed pad 2's purple Qirex in the **top**
view and pad 1's blue Assegai below; holding pad 2's throttle moved the top view to 72 km/h while the bottom stayed at 0.
**The end screens**: `Session::reenter_multiplayer`'s lobby and page walk is `multiplayer::reenter`, tested on the shipped
`menu.toml` (`return_to_menu_lands_on_the_race_page_with_the_same_people`: the menu lands on RACE over MULTIPLAYER, where
`has_left_the_flow` is false, so `sync_multiplayer` does not clear `pane_people`; `..._without_two_people_is_a_plain_menu`),
and a race again is `finish_launch` -> `launch_race`, which clones `self.race_options`, so it carries `seat_picks` (read, not
run: it never goes through `launch_from_settings`, which refills them from the empty `split_picks`, nor `drop_split_lobby`,
which `handle_menu`/Track Select BACK reach only while `multiplayer` is false). The three
fields a RETURN TO MENU keeps (`pane_people`, `window_layout`, `split_cut`) have no writer on the path: `open_menus` writes
none (the `window_layout.remove` in `windows.rs` is the failed-window-open path), `sync_multiplayer` is the one clearing `pane_people` and it ends the flow only off the RACE page. There is no
`Session` without a window and a GPU, so those two are pinned at the functions the path calls, not as one run. No race
finished on lavapipe, so neither end-screen path was walked (the 2026-10-16 lane's live walk finished a race, see "The flyby and the end").
**Not walked**: a race again and a RETURN TO MENU race (see above), **the keyboard player flying a pane in a race on Xvfb**
(a held `x` moved nothing, with the keyboard in window 1 under the join-order assignment as well as in window 2
under the chosen one, so it is the Xvfb/XTEST keyboard and not this screen; the routing is
`a_window_whose_slots_are_not_a_run_still_routes_to_the_keyboard_seat`), Pure, Omega and 2048 stills (2048's `.pkg`
is PFS-encrypted here), a second real keyboard.

## Windows

`App::window_event` routes by `WindowId` (`windows::which`, a pure function with a test):

| Event | Window 1 | A seat window |
| --- | --- | --- |
| Close request | Quit, as ever | Every seat it held goes to the AI, the window closes |
| `Destroyed` | (ignored) | The same as a close: under X11 with no window manager a window is destroyed outright |
| Resize | The surface; zero-sized pauses as ever | The surface; zero-sized is ignored and **never pauses** |
| Keyboard | The first keyboard seat (window 1's), or in a race dealt across windows the keyboard seat of the window it arrived on ("Who sits where") | The keyboard seat of that window, claimed by a press if it has a free seat, else ignored |
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

## Under gamescope

gamescope (a Steam Deck in gaming mode) maps every window of a client full-output at 0,0 and
shows one, so seat windows would sit on top of each other (measured: two windows, both
3840x2160+0+0 and viewable). A `--windows` / `--window-seats` layout is folded into one window
holding every seat, cut like a split screen (`window::one_window_under_gamescope`, **chosen, not
measured**). Details: `docs/tools/packaging.md`, "Under gamescope".

**The WINDOW LAYOUT screen offers one window under gamescope (2026-10-14, lane `gamescope-menu`,
maintainer's decision, **chosen, not measured**).** The composition root hands
`oag_ui_screens::window_layout::Picker` one bit of plain data, `.one_window_only(under_gamescope)`,
and the screen never reads the environment. With it set: every seat is dealt into window 1; the
WINDOWS row stays but reads 1 and cannot change (pad, click and wheel all end in a no-op); the mode
row stays and reads FULLSCREEN, locked (gamescope opens every window borderless,
`window::effective_mode`); the per-window MONITOR row is gone, and so is the monitor caption on the
preview box; the hint line is replaced by the reason (`OAG_WINDOW_LAYOUT_ONE_WINDOW`: "ONE WINDOW
ONLY: THIS DISPLAY SHOWS ONE WINDOW AT A TIME"); the locked rows are drawn at half alpha. The CUT
and SEATS rows and the preview work as on a desktop. START still goes through `apply_window_layout`,
whose own fold (`one_window_under_gamescope`) is then a no-op. On the desktop none of this is set
and the screen is unchanged.

**`--windows-fullscreen` under gamescope** has no per-monitor meaning: the layout is one window and
that window is borderless at the output whatever the flag says, so it does nothing there.
**CLI:** `--windows N` and `--window-seats` are not refused; one `info` line at start says why they
fold ("gamescope shows one window at a time: the N requested windows are folded into one window
holding all M seats", `DriverArgs::gamescope_fold_note`, logged once from `main_body`).

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

## Cost of a windowed frame (2026-10-14, lane `window-render-cost`)

**How to measure.** The render bench reaches a windowed race (`crates/game/src/race_capture/bench.rs`,
`perf-probe` build, headless, no window): each window is its own encoder and submission, as the
frame loop makes them, and each seat's scene pass is timed.

```sh
# on the maintainer's RADV desk, a real GPU (CPU and GPU separate), a dev build with the probe
cargo build -p oag-game --bin oag-game --features perf-probe
OAG_RENDER_BENCH=60 OAG_RENDER_GPU_BENCH=60 target/debug/oag-game data/images/pulse-psp-eu.chd \
  --race --window-seats 2,2 --screenshot /tmp/bench.png --ticks 300 --no-audio | grep '^bench'
# the same with --motion-blur medium prices the effect each window now draws; swap the
# disc for data/images/hdfury-ps3-eu.iso for HD; --window-seats 1,1 and 4,4 are the other rows
# (add VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json for software Vulkan)
# a live two-monitor run: the pacing line, and the controller with graphics.target_fps set:
RUST_LOG=warn,oag=info oag-game --race --windows 2 --no-audio 2>&1 | grep 'windows:'
```

`render_viewports CPU` is one whole frame's CPU, record plus submit, the GPU idled first;
`pipelined per frame` is frames back to back with one wait at the end, the number a frame budget is
read against. **Relative numbers, not real-GPU numbers**: measured on this workstation (RADV on an RX
7800 XT, headless, and lavapipe), a dev build (opt-level 2, not the release builds the split's table
below uses, so absolute CPU is several times higher), at a load average of **8 to 78 from other
lanes** (the load each run started under is in the scratch log; it is what makes the medians
bimodal, one population at about half the other). Each row is before (`b5412435f` plus the harness)
against after, **interleaved, three repetitions of 60 runs** (HD on lavapipe: 5 runs, it is 0.4 s a
frame); the table is the best repetition's p10, which is the least contended run, and the scene passes on
RADV are 0.5 to 3 ms in all rows (the GPU is not the limit, as for a split).

| Title (RADV) | Windows | CPU ms before -> after | Pipelined ms before -> after |
| --- | --- | --- | --- |
| Pulse | 1,1 | 12.5 -> 11.1 | 13.2 -> 11.9 |
| Pulse | 2,2 | 22.3 -> 20.3 | 25.8 -> 21.7 |
| Pulse | 4,4 | 47.0 -> 43.7 | 47.1 -> 48.4 |
| HD | 1,1 | 38.7 -> 31.2 | 37.8 -> 35.2 |
| HD | 2,2 | 57.7 -> 43.9 | 61.9 -> 50.9 |
| HD | 4,4 | 84.3 -> 81.4 | 90.2 -> 88.3 |

Lavapipe (software, CPU and GPU are the same cores, so only the pipelined column means anything):
Pulse 56 -> 52, 93 -> 86, 128 -> 134 ms; HD 392 -> 327, 614 -> 562, 898 -> 830 ms.

**Reading it.** The hoist saves a few percent to about 20 percent of a windowed frame's CPU: the
shadow maps' encoding, the scenery clocks (0.65 ms in a dev build) and the advert cards, once instead of
once per window. It is small because the **shadow gather was already shared between the windows of a
frame** (the per-tick hull cache of `shadow-gather-per-tick`) and because what each window still does
is camera-dependent: uniform writes, LOD, sort, particle and exhaust geometry, and `encoder.finish`,
which scales with the draw count. At 4,4 the saving is inside the noise (3 to 7 percent against a spread
of 2x between repetitions). **What a window costs is what it draws**, so a windowed race is CPU-bound as a split is: on RADV the
GPU spends 0.5 to 3 ms in the scene passes against 11 to 84 ms of CPU. The effects each window now
draws cost, on the same rows with `--motion-blur medium` (median over three, noisy): Pulse 1,1 +1.4 ms,
2,2 -4 ms (noise), 4,4 +12 ms; HD 1,1 +13 ms, 2,2 -3 ms (noise), 4,4 +23 ms, so about 1 to 3 ms per window in a dev build;
they are scene-side only, and the FSR 3.1 chain is a `Framebuffer` pass outside the bench.
**What is not measured:** the live frame's HUD, resolve and composite per window (identical before and
after), real-display pacing, and the controller on a GPU that can hold its target (lavapipe cannot, and
the log says so).

## The WINDOW LAYOUT screen's CUT row costs nothing (2026-10-14, lane `split-preseat`)

The maintainer saw the frame rate fall on the WINDOW LAYOUT screen itself the moment CUT went to
VERTICAL, on a Steam Deck in gaming mode at 3840x2160. It is **not reproduced** by the path below,
on either machine. The reading of the code agrees (a Fill and a Text or two either way;
`apply_window_layout` runs only at START); this is the measurement.

How: `--players 2 --preseat --frame-log --script ...` (above), Pulse (PSP, EU), a fresh profile
(defaults: perf overlay off, no screen filter, no upscaler), the script walking the real screen:
CUT horizontal 6 s, tap RIGHT to vertical for 10 s, RIGHT back to horizontal 10 s. Release builds.
Locally the window is `gamescope --backend headless -W 3840 -H 2160` (the Deck's compositor, on
RADV, no display; `MESA_VK_DEVICE_SELECT` picks the GPU) with `window_mode = "borderless"`; Xvfb
and lavapipe are for mechanism checks only and are not a GPU number. One line a second, mean over
the meter's window.

| Machine, GPU | vsync | CUT horizontal | CUT vertical | window = surface |
| --- | --- | --- | --- | --- |
| desk, RX 7800 XT (RADV) | off | 120.0 fps, 8.33 ms, CPU 0.66 ms, present 0.20 ms | the same | 3840x2160 throughout |
| desk, Raphael iGPU (RADV) | off | 87.6 fps, 11.41 ms, CPU 6.6 ms, present 6.3 ms | 87.7 fps, 11.40 ms, CPU 6.5 ms | 3840x2160 throughout |
| desk, RX 7800 XT | on | 60.0 fps, 16.67 ms | the same | 3840x2160 throughout |
| **Deck** (Van Gogh, RADV, Mesa 26.2.0-devel; znver2 dist build) | off | 120.0 fps, 8.33 ms, CPU 1.0 ms, present 0.22 ms | 120.0 fps, CPU 1.05 ms | 3840x2160 throughout |
| **Deck** | on | 60.0 fps, 16.67 ms, p99 16.8 ms | 60.0 fps, p99 16.8 ms | 3840x2160 throughout |

Load average when each run started: desk 4 to 10 (other lanes; the iGPU row's present time is a
constant, not a trend), Deck 0.6 to 0.9. The front end's own limit is 120 fps (`frame limit in
force: 120`), so "120" is the cap, not what the GPU could do. No `size change` line follows the one
at start, so neither the window nor the surface moves on CUT. START on the screen, with CUT vertical,
goes on to the seat assignment screen and the RACE page (the log names each stage); the race is
measured in the next section.

What this leaves: the drop the maintainer saw needs something the probe does not have - a real
profile (a screen filter, an upscaler, motion blur), real pads, or the Steam launch path. Launch
the game from Steam with `--frame-log` in its launch options (or `OAG_FRAME_LOG=1`) and read the
log file: the line names the stage, so the CUT toggle shows up with its own fps, CPU and present time.

**Found on the way, `gamescope-window`'s area (not fixed here).** On the Deck's gaming-mode
gamescope (`DISPLAY=:1`, 3840x2160) a requested window size arrives **six times too large**:
`window_mode = "windowed"` at 1440x816 opened 8640x4896; at 3840x2160 it panicked in
`Surface::configure` (`Requested was (23040, 12960), maximum extent for either dimension is
16384`) on all three tries. `borderless` is clamped to 3840x2160, but about half the launches still
panicked the same way and a retry worked. The measurements above are borderless with a retry.

## The race the WINDOW LAYOUT screen leads to costs the same either way (2026-10-14, lane `menu-race-deck`)

The same drop might have lived further along: START on WINDOW LAYOUT, seat assignment, Ship
Select, the RACE page's launch row, the race. Walked by script on the **Deck** (gaming mode
gamescope, `DISPLAY=:1`, 3840x2160 borderless, vsync off, RADV Van Gogh, Mesa 26.2.0-devel, znver2
dist build of `main` at `7e6bce9ac`, Pulse PSP EU, fresh profile with `pause_on_focus_loss = false`,
`--no-audio`). The script is the `--players 2 --preseat` walk above; the cut is one `+5:right`
entry before START (CUT vertical), then a `cross` on seat 0 and `cross@1` on seat 1 every 3 s
through the seat assignment, Ship Select and RACE pages (the walk reaches the race on its own:
stages `menu:window-layout`, `menu:seat-assign`, `menu:race`, `menu:picker`, `loading`, `race`).
Horizontal is the same script without `+5:right`. A first run on an empty cache spent 90 s
transcoding the boot movies and ate the boot taps; every run below is on the warm cache.

Mean of the one-line-a-second `stage=race` readings, the first 5 s of the race dropped (74 s
of race for the menu walk, 44 s for the CLI), each run on its own, three per cut:

| Start | Cut | Run | fps | frame | p99 | CPU | GPU scene | present |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| menu walk | horizontal | 1 / 2 / 3 | 66.8 / 67.9 / 66.9 | 15.00 / 14.74 / 14.98 ms | 17.4 / 16.8 / 17.4 ms | 14.9 / 14.7 / 14.9 ms | 13.8 / 13.5 / 13.5 ms | 5.3 / 5.1 / 5.3 ms |
| menu walk | vertical | 1 / 2 / 3 | 67.8 / 69.8 / 68.8 | 14.77 / 14.34 / 14.55 ms | 17.3 / 16.7 / 16.9 ms | 14.7 / 14.3 / 14.5 ms | 13.5 / 13.4 / 13.4 ms | 5.2 / 4.8 / 5.0 ms |
| CLI `--race --players 2 --split-cut X` | horizontal | 2 runs | 50.6 / 50.8 | 21.4 ms | 27.8 ms | 21.3 ms | not read | 13.0 / 12.6 ms |
| CLI | vertical | 2 runs | 50.9 / 52.1 | 21.4 / 21.0 ms | 27.7 / 26.8 ms | 21.3 / 21.0 ms | not read | 12.6 / 12.3 ms |

Window, surface and render size are 3840x2160, 3840x2160 and 3812x2160 in every run, and the one
`size change` line is the start-up one: **no size moves on the way into the race, in either cut.**
Load average when the runs started was 1.0 to 1.5.

Reading: **vertical is not dearer than horizontal on the Deck, by either route.** The three-run
spread inside a cut (66.8 to 67.9 horizontal, 67.8 to 69.8 vertical) is as wide as the gap between
the cuts, and vertical is, if anything, 1 to 2 fps ahead. The menu-started race and the CLI race
are not the same race: the CLI one is about 17 fps slower in both cuts (CPU 21 ms against 15 ms,
present 12.5 ms against 5 ms, the GPU scene time not read). That is a difference of the race that
was set up (the CLI `--race` defaults against the RACE page's choices), **not** of the cut, and
it was not chased here (**open**, not part of this question). The craft stand still in these
walks (nobody drives), so a moving race is a few CPU-ms dearer than either column.

So the maintainer's drop is not on the Pulse path from WINDOW LAYOUT to the race with a default
profile. What is left is what the probe does not have, as above: a real profile, real pads, or the
Steam launch path - run the game from Steam with `--frame-log` and read the log's `stage=` and
`cut=` fields. Two things seen on the Deck while measuring, both `gamescope-window`'s area and
unchanged: the window request still arrives at scale 6 (`inner 23040x12960`, clamped by the
surface to 16384x12960 with a warning, then to the 3840x2160 output), and the first launch on a
fresh cache is the movie transcode, so a timed script must run a second time.

### Why the CLI race measured 17 fps slower (2026-10-14, lane `cli-race-cost`)

Two real differences, one fixed, one a property of the measurement.

- **Fixed: `--race --players 2` picked the opponents' AI before the second person was seated.**
  `ai_choice::apply` reads the number of humans, and the CLI route (and the `--screenshot`
  capture the render bench runs) called it before `split_screen::apply` added the second one, so
  the original driver flew a two-human race (log: `AI: the original driver flies the opponents`)
  where the menu route logs `AI: this project's driver flies (more than one human is racing)`.
  Both routes now choose after the seating. This is a rule bug, not the frame cost: with
  render, cut and race the same, the A/B (`before`/`after` binaries, Raphael iGPU, 3812x2160,
  two views) gave 22.2 fps and 45.6 ms CPU (waiting on the GPU) before, 22.3 fps and 45.4 ms after.
- **A property of the walk, not a cost: the CLI race has a driver in seat 2.** With one keyboard
  or one pad nothing drives seat 2, so `split_screen::follow_devices` hands it to the AI
  (`split screen: slot 1 is now Ai`, a few seconds in; on the Deck it is in the CLI log and not in
  the menu log, where `--preseat` seats a placeholder keyboard that keeps the seat `Local`).
  That craft then flies, so the CLI views move through the track while the menu walk's views
  show craft standing on the grid. On the iGPU the CLI race swings between 13 and 29 fps with
  the position on the track; the menu-started one holds 28 fps standing still. A frame rate
  taken from a stationary start is a best case for either route.
- Settings, render target (3812x2160), render profile and frame limit match between the routes;
  the only frame-log field that differs is `gpu_ms`: a split race with one window draws one scene
  per view and times none of them (`session/draw.rs`, chosen so resolution holds its scale while
  split), a one-window multi-window race times its window, so the Deck's menu log has a GPU
  figure and the CLI log has `-`.
- On current main under gamescope with no player-chosen render scale the frame is drawn within a
  1920x1080 budget (`deck-4k`), on both routes; the 3812x2160 figures above are with
  `--render-scale 100`, which is the case the 2026-10-14 Deck numbers were.

Not re-measured on the Deck: it was unreachable (`No route to host`) from the moment the fix was
built, so the Deck pair `before`/`after` is **open**; the iGPU A/B above is the evidence.
`render-bench` is affected by the first item only for its `2h`/`2v` cases (the capture path
chose the AI the same way); its numbers measure the frame, which the AI choice did not move
here, so they stand.

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
- **A seat's own effects in a window** (2026-10-19, lane `local-seat-contact-fx`): each seat's wall
  sparks, shake, boost kick, boost pulse and flaps are its own (`docs/rendering/split-screen.md`,
  "Cameras"). `--window-seats 2,2 --screenshot shot.png --ticks 810 --force-wall-slide 800:3`
  against `main`'s build with the same flag: window 2's lower view (seat 3) shows the sparks at its
  craft and the shaken view, `main`'s the same wall hit with neither. One live `--windows 2` run on
  an Xvfb with software Vulkan opened both windows and drew both seats (the hit itself needs a
  keyboard seat, so the hit is the headless frames').
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
- **The track panel on every window and the any-person skip** (2026-10-14, lane `flyby-panel`).
  Headless, Pulse EU (Talon's Junction White), `--race --window-seats 2,1 --intro-ticks 100|400|800`
  against `main` (`028d1f232`): window 1 byte-identical at all three, window 2 differs and now carries
  the panel (the name over the shot, the blurb under it, the wipe clipping the blurb at tick 100, as
  window 1's does); `--players 1|2|4` at flyby ticks 100|100|600 and `--players 2 --intro-ticks 3000
  --ticks 30` (the panel's fade) byte-identical; `--window-seats 1,1 --size 700x1000`: window 1
  identical, window 2's panel fitted and letterboxed in the tall frame. Live, Xvfb + `lvp`, `--race
  --window-seats 2,1`: both windows draw the panel through the flyby in step; `--window-seats 1,1`
  with `xdotool keydown x` focused on window 2 skipped the flyby (both windows were on the race, window 2
  thrusting, on the next capture, 2 s later). The pin is
  `raceplay::window_flyby_ground_truth::a_second_persons_held_cross_skips_the_flyby`, which fails
  with the old primary-only read. The panel's drawing has no test of its own beyond the capture
  diff (a GPU draw).
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
  a flyby only for Pulse PSP (`start_grid.vex`, [race-intro.md](../gameplay/race-intro.md)): on HD EU
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
- **A keyboard per window and the pause line on every window** (2026-10-14, lane `window-keyboard`; the keyboard-follows-focus part is superseded by "Keyboards are seats" above, the pause line and click parts stand).
  Pulse EU, Talon's Junction White, `--race --window-seats 1,1`, Xvfb with software Vulkan
  (`pause_on_focus_loss = false`), real input through XTEST (`xdotool windowfocus`, `keydown x`),
  one root capture holding both windows per frame: with window 2 focused and X held, window 2's
  craft left the grid (24, 34, 49 and 56 to 63 km/h over successive frames) while window 1's sat at
  0 km/h with its timer running, and the log says only `window keyboard: the keyboard is now on slot 1`
  (no `slot 0 is now Ai`, so window 1 stayed a person). With X still physically held, focus moved to
  window 1: window 1 stayed at 0 km/h (before the let-go fix it thrust from the re-delivered press,
  seen in the same capture), and a fresh press then drove it. Space on window 2 paused with owner
  slot 1: both windows drew `PLAYER 2 PAUSED - ANYONE CAN RESUME IN 0:27` (window 2's scene, black in
  a first cut that cleared under the line, was intact); a click on window 1 and Space with the focus
  on window 1 did not lift it (slot 0 is not the owner); a click on window 2 did, into the resume
  countdown and the race. **Not exercised:** a pad beside the keyboard (the pad join is
  `pad_join_end_to_end`'s), a finish, more than two windows.
- **Keyboards are seats, live** (2026-10-14, lane `keyboard-seats`). Pulse EU, Talon's Junction White,
  `--race --window-seats 1,1`, software Vulkan, sixty-tick samples of both craft's speed from a
  temporary log line (not kept), keys held from the start. Nothing here touched the maintainer's session:
  a private headless sway (`WLR_BACKENDS=headless`) with two seats, a `zwp_virtual_keyboard_v1` per seat
  (`scripts/multiseat-vk`, a 90-line client over `wayland-protocols-misc`; a uinput keyboard would not have reached it and would
  have typed into the real session), each seat's cursor clicked into its own window (`seat seatN cursor
  press button1`), `swaymsg -t get_seats` showing `seat0` focused on window 1 and `seat1` on window 2.
  A virtual keyboard's first key reaches a client only after winit has bound that seat's `wl_keyboard`
  (a capability that appears with the keyboard), so every script starts with a second's sleep.
  - *Seat 1 only* (window 2): slot 1 to 59 km/h with window 2's readout showing it, slot 0 at 0.02 and
    window 1 on the grid. *Both at once* (seat 0 holds X, seat 1 holds X and Right): slot 0 148 km/h
    while slot 1 steered off and read `WRONG WAY`, two different pictures in one capture. *One seat
    moving between windows* (the niri case): a press with the seat focused on window 1 flew slot 0
    only; after the seat's cursor was clicked into window 2 a press claimed slot 1 (`keyboard seat: ...
    claims slot 1 (Shared)`) and flew it (34 km/h) while slot 0 coasted at 0.15.
  - *X11*: the same sway's Xwayland exposes one X keyboard per seat (`xwayland-keyboard:53` and `:54`,
    two XInput2 slaves under the one `Virtual core keyboard`), which is the two-physical-keyboards case
    (a plain Xvfb has one XTEST keyboard and `xinput create-master` made nothing there). Window 1 had the
    focus throughout. Temporary log lines (not kept) gave the order of one press's two events: the raw
    event of A's first X came ahead of its window event, and B's Right likewise. Keyboard B alone flew
    window 1's seat (one keyboard, the window path); then A holding X and B holding X and Right made the
    seat list `[(Device(KeyboardId(0)), 0), (Device(KeyboardId(1)), 1)]`, and slot 0 reached 152 km/h
    while slot 1 steered into the walls. A first cut that let go of the held keys when the second
    keyboard spoke left the first keyboard's craft idle for the rest of the hold; now it keeps them
    (`the_first_keyboard_keeps_its_held_key_when_a_second_one_speaks`).
  - *Not exercised*: a pad player in the other window (a pad cannot be made headless; the rule is
    `pad::keyboards::tests::a_window_whose_seats_are_pads_ignores_keys`), Windows, more than two windows.
- **Per-window effects, the shared work and the summed controller** (2026-10-14, lane
  `window-render-cost`). Pictures: `--race` PNGs of `--players 1|2|4` and `--window-seats
  1,1|2,1|2,2|4,4`, Pulse EU and HD EU, shadows `original` and `mapped`, ticks 300 and 1200, against the
  build before (`main` plus the bench harness, which draws the same): **88 of 88 `cmp` identical**, for
  the hoist build and again for the final build. Blur: `--window-seats 1,1 --autopilot --ticks 600
  --motion-blur high` on Pulse: both windows differ from blur off and read as blur along their own travel (the
  road streaks run out from each camera's own vanishing point, the HUD stays sharp, no ghost of
  the other window's camera); a capture primes each window's history with its own tick-before-last view.
  Live under Xvfb with lavapipe, `--race --window-seats 1,1` with `target_fps = 60`, `reconstruction =
  fsr3`, `motion_blur = medium` (Pulse PSP profile in a scratch settings file): both windows draw the
  flyby with blur; the log says `dynamic resolution: 60 fps is out of reach here - 51.553 ms of this
  frame is fixed cost` (the two windows' FSR 3.1 presented halves summed: a software rasteriser cannot
  hold 60); with `reconstruction = off`, `dynamic resolution: 1080x612 (75% of 1440x816) - scalable 77.7 ms`
  then `720x408 (50%)` (the two windows' passes summed, the one scale reaching both), then the same
  out-of-reach line at the floor. **Not exercised:** a real GPU's controller, two monitors, the FSR
  3.1 picture itself under lavapipe at speed. Pins: `raceplay::tests::windowed` (the work once per frame
  across four windows and again on the next frame; a window told the work is done does none; the
  `mapped` map still per window; each window's previous camera, jitter phase and reset are its own,
  which fails with one shared history) and `session::timing::tests` (the cost rule, and a frame fed
  only once every window has come back, in order, given up on after four frames); checked by
  mutation: reading `shared` as always false and keying the history on window 0 each fail them.
- **Tests**: `oag-input` (`pad::keyboards::tests`: a window whose seats are pads ignores keys, a press claims the lowest free seat, a pad does not take a claimed seat, nothing is free merged or closed, a device keeps its seat whichever window has the focus, a device-bound seat takes no window events, unseating drops the seat; `tests`: a key flies the seat of the window it arrived on, no keyboard seat ignores keys, a single window is unchanged, a window's focus loss lets go of its own seat only, two keyboards fly two seats, the first keyboard keeps its held key when a second speaks, the extra seats let go while parked; `keys::every_candidate_is_reachable_from_a_physical_key`; checked by mutation: sending a window with no free seat to claim to the first seat fails four of them, dropping the device rule fails three), the reach tests in `pad::tests`, `windows::tests` (`first_seats`, `Focus::current`, `click_slots`), `pause::tests::a_seat_in_a_window_the_keyboard_can_reach_is_present`,
  `windows::tests` (routing, close, focus, the shared allocation, a window's name),
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
- ~~**A second keyboard cannot join from the join screen** (2026-10-14, `keyboard-seats`)~~ **Done on X11 and
  Windows, 2026-10-14 (`mp-menu-flow`)**: `lobby::Device::Keys(KeyboardId)` is a physical keyboard named by the raw key
  events (`Controls::lobby_key`, fed by `Session::device_key` while the join screen is open), the join screen shows a seat
  per keyboard (`KEYBOARD 2`), a raw press is one device and not also the window's anonymous keyboard (the dedupe in
  `Controls::player_snapshots`), and the seated keyboards fly their seats from the first tick of the race, also in one
  window (`Controls::sync_device_keys`, `Assignment::claim_keyboard_device`). **Open: Wayland and macOS**, where winit names
  no keyboard: the windows open only after the layout screen, so a second keyboard there can only claim a seat by a press
  in its own window (this page, "Who sits where"), never as a join row. A single-window race with two keyboards on
  X11 is covered by unit tests only (a plain Xvfb has one XTEST keyboard).
- ~~The end screens on a seat's own window~~ (2026-10-16, see "The flyby and the end"); open from it: the disc's own EndRace screens in a second window were not walked live, a drawn cursor on the seat windows, and **the pause menu on the seat windows** (Escape's menu is window 1's; the other windows only show the dim tint).
- **`--window-monitor`** on the command line (the layout screen chooses monitors; the flags still do not) and remembering the windows' sizes and places.
- ~~Per-window motion blur, FSR 3.1 and dynamic resolution; the camera-independent work once per frame~~
  (2026-10-14, `window-render-cost`, see "What a window draws"). Open from it: **per-view history for a
  window of several seats** (a split's views are N cameras in one frame; blur, the temporal record and
  timing there want one history per view, which a split screen in one window wants too); the
  summed-cost controller on **real displays** (the command in "Cost", below); and FSR 3.1 and the
  blur under lavapipe could be looked at but not timed meaningfully.
- **Pacing on real displays**, the command above.
- **Monitor hot-plug** and a window dragged between monitors of different scale are not exercised.
- **Seat windows' own settings** (render scale, screen filter per window): they follow the profile.
