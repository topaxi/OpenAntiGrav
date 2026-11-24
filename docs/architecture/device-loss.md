# A lost GPU device on a native build

What the game does when the driver resets, the GPU hangs or an eGPU is unplugged: wgpu marks the
device lost and every later call into it fails. The web build has its own handler
(`oag_game::web_fault`, [`docs/tools/web.md`](../tools/web.md), "Troubleshooting"); this page is the
native one, `crates/game/src/app/device_loss.rs` and `session/device_lost.rs`.

## What fires

wgpu offers two hooks and the game installs both on the one device it makes (`Gpu::bring_up`):

| Hook | What it does here |
| --- | --- |
| `Device::set_device_lost_callback` | **The one source of truth.** Records the reason and message in a flag shared with the frame thread. It runs on whatever thread wgpu was on, from inside `submit` or `poll`, so it does nothing else. |
| `Device::on_uncaptured_error` | Keeps wgpu's own rule for every error on a live device (a panic: a validation error is a bug we want to see). Once the flag is set it swallows what the dead device's other users keep reporting. |

wgpu-core 30.0.2 drops a lost device's own error before the uncaptured-error handler
(`ErrorType::DeviceLost => return, // will be surfaced via callback`), so an error is **never**
read as the loss by its text: a validation error that says "device is lost" stays fatal and does
not restart the game. The game never destroys its own device, so a `Destroyed` reason is a loss
here too (unlike the web build, where it is the game's own teardown). `Device::destroy()` is how a
test or the `lose-device` script step causes one on purpose: wgpu fires the callback with
`Destroyed` once the device is polled.

## What the frame loop does

`Session::frame` asks first thing, before it acquires an image. A dead device answers `Lost` to
the acquire for ever, and the loop that reconfigures on that never draws again (the frozen
window). On a loss it:

1. logs one `error` line with the reason;
2. captures the race on screen into the records and the ghost as leaving it would
   (`Session::capture_race_result`: a Speed Lap or Zone best lap, an abandoned Time Trial; guarded
   by `result_saved`);
3. winds the session down (`Session::wind_down`, the call every exit makes): the campaign
   position, the replays of the race on screen and of one parked behind the menus (a race of 300
   ticks or more, `race_stage::auto_replay`; **photo mode's rewind is left first**, since it takes
   the recording out of the race and only leaving puts it back), the records, the playlist's
   place, rumble off. `wind_down` runs once per session, whichever of the loss and the exit gets
   there first;
4. writes `<config dir>/oag/device-lost.txt`, the note the next run reads;
5. retitles the window, the one place a message can go with no GPU.

From then on every frame returns at the top: nothing ticks and nothing draws against the dead
device, and the event loop idles (`ControlFlow::Wait`) rather than spinning.

## What the player sees

`App::after_loss` decides, after the frame:

- **On a desktop the game starts itself again**, from whatever stage it was on: the session has
  saved, so `device_loss::restart_once` spawns the same executable (`$APPIMAGE` inside an
  AppImage, whose mount goes away with the old process) with the same arguments and
  `OAG_RELAUNCHED_AFTER_DEVICE_LOSS=1`, and this process leaves. One loss starts at most one
  process, however many frames see the flag. The new run brings up a fresh device and shows the
  main menu with a line over it, "THE GRAPHICS DRIVER STOPPED LAST TIME. YOUR PROGRESS IS SAVED"
  (the existing `Session::notice` band, strings `OAG_DEVICE_LOST_*`). A race in progress is over;
  its replay is in the REPLAYS page.
- **The restart is refused**, and the window stays up as it is with its new title until the
  player closes it, when:
  - the run carries the variable (a driver that loses the device again on a fresh start ends the
    run, so a bad driver does not loop);
  - an argument is not on the allowlist (`device_loss::restartable_args`): a player's own launch
    is a disc image or other positional path, `--source`, `--dlc`, `--launcher`, `--no-audio`,
    `--no-intro`, `--no-video`, `--original-menu`, `--pick-language` and `--script`. Anything
    else would run twice: `--race`, `--connect` and `--online` reopen a race or a lobby;
    `--dump-audio`, `--tap-audio` and `--screenshot` overwrite their file.
- **A phone** has no title bar to read and no second process to start: the run ends. What the
  player gets is the note: the next launch shows the line over the main menu. There is no toast:
  the Android glue (`crates/mobile`) has JNI only for launch arguments, and a toast built blind
  would be untested code on the device path.
- **The web** has its own handler and is unchanged.
- **A `--drive` run** (no window) ends with an error naming the loss and the last state, exit
  status 1; **a wallpaper** leaves its loop and winds down.

The note is removed when the menu puts the line up, not when the run opens: a run that never
reaches the menu (a `--race` run, a closed window) leaves it for the run that does.

A restart is a process restart, not a rebuilt device inside the process: every renderer, texture
and pipeline of the old session holds the dead device, and rebuilding each is the whole game's
start-up. That is what the front end already does on a fresh launch.

## What stays unrecoverable

- A loss **inside a race** loses the race itself: the replay up to the loss and the result as
  leaving it would are kept, the race is not resumable.
- **A hard kill of the process** (the driver taking the process down, the OOM killer) skips all of
  it. Nothing here writes on a timer ([`persistence.md`](persistence.md)).
- **wgpu issues the game cannot catch:**
  [#10222](https://github.com/gfx-rs/wgpu/issues/10222): after `VK_ERROR_DEVICE_LOST` from a
  submit, wgpu-hal drops resources the driver may still hold, which is wgpu's to fix;
  [#6279](https://github.com/gfx-rs/wgpu/issues/6279) and
  [#3043](https://github.com/gfx-rs/wgpu/issues/3043): "Parent device is lost" panics from
  `Queue::submit` on another thread reach the device-lost callback in wgpu 30.0.2, but a panic
  raised inside wgpu or a driver before it reaches either hook is not caught (the game's panic
  hook logs it and flushes, `oag_log::install_panic_hook`);
  [#10244](https://github.com/gfx-rs/wgpu/issues/10244): `request_device` answering `Lost` at
  start-up fails the bring-up (`Gpu::bring_up`'s "requesting the device" error), the same path as
  an adapter that will not serve, so a named adapter falls back to the default and the default
  ends the run with that message.

## Reproducing

`lose-device` is a `--drive` step and a `--script` entry (`SECONDS:lose-device`):

```sh
# headless, mid-race: ends with the loss said and the replay kept
oag-game <image> --no-audio --drive "boot; select racebox; press cross; until page=race; \
  select launch_race; press cross; until screen=picker; select 03_Track; press cross; \
  until screen.id=ship-select; select Feisar; press cross; until stage=race; ticks 400; lose-device"

# windowed, on the front end: restarts once, the new run shows the note
oag-game <image> --no-audio --script "45:lose-device"
```

Tests: `app::device_loss::tests` (the flag, an error is never the loss, a real device destroyed
with its later errors swallowed and failing under `OAG_REQUIRE_GPU` when there is no adapter, the
note, the argument allowlist both ways), and, under `just test-data`,
`drive_ground_truth::a_device_lost_mid_race_saves_the_replay_and_ends_without_a_panic` (replay,
records row and note written, no panic) and
`photo_mode_ground_truth::a_device_lost_during_a_rewind_still_keeps_the_replay`. Each fails on a
build without the part it names.
