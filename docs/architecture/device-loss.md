# A lost GPU device on a native build

What the game does when the driver resets, the GPU hangs or an eGPU is unplugged: wgpu marks the
device lost and every later call into it fails. The web build has its own handler
(`oag_game::web_fault`, [`docs/tools/web.md`](../tools/web.md), "Troubleshooting"); this page is the
native one, `crates/game/src/app/device_loss.rs` and `session/device_lost.rs`.

## What fires

wgpu offers two hooks and the game installs both on the one device it makes (`Gpu::bring_up`):

| Hook | What it does here |
| --- | --- |
| `Device::set_device_lost_callback` | Records the reason and message in a flag shared with the frame thread. It runs on whatever thread wgpu was on, from inside `submit` or `poll`, so it does nothing else. |
| `Device::on_uncaptured_error` | Before a loss it keeps wgpu's own rule (a panic: a validation error on a live device is a bug we want to see). An error whose text names a lost device counts as the loss itself, for the case where it arrives before the callback. After a loss it swallows what the dead device keeps reporting. |

wgpu has no typed "device lost" error: the lost state reaches a caller as a validation error
carrying wgpu-core's `DeviceError::Lost` text, "Parent device is lost", so the second hook reads
that text. The game never destroys its own device, so a `Destroyed` reason is a loss here too
(unlike the web build, where it is the game's own teardown). `Device::destroy()` is how a test
or the `lose-device` script step causes one on purpose: wgpu fires the callback with `Destroyed`
once the device is polled.

## What the frame loop does

`Session::frame` asks first thing, before it acquires an image. A dead device answers `Lost` to
the acquire for ever, and the loop that reconfigures on that never draws again (the frozen
window). On a loss it:

1. logs one `error` line with the reason;
2. winds the session down (`Session::wind_down`, the same call every exit makes): the campaign
   position, the replays of the race on screen and of one parked behind the menus (a race of
   300 ticks or more, `race_stage::auto_replay`), the records, the playlist's place, rumble off;
3. writes `<config dir>/oag/device-lost.txt`, the note the next run reads;
4. retitles the window, the one place a message can go with no GPU: the title says the device
   was lost, progress is saved, close the window and start again;
5. sets `quit`. The event loop leaves at the next `about_to_wait`.

`wind_down` runs once per session, whichever of the loss and the exit gets there first.

## What the player sees

- **On the front end** (boot sequence, menus, the disc chooser, with no race parked and no
  network race) the game starts itself again, once: `device_loss::relaunch` spawns the same
  executable with the same arguments and `OAG_RELAUNCHED_AFTER_DEVICE_LOSS=1`, and the old
  process leaves. The new run brings up a fresh device and shows the main menu with a line
  over it, "THE GRAPHICS DRIVER STOPPED LAST TIME. YOUR PROGRESS IS SAVED" (the existing
  `Session::notice` band, strings `OAG_DEVICE_LOST_*`). A run that carries the variable does not
  restart a second time: a driver that loses the device again on a fresh start ends the run.
- **In a race**, or on a phone or the web (no second process to start), the run ends. The window
  stays up with its new title until the player closes it. The next launch, by hand, shows the
  same line over the main menu, from the note in step 3.
- **A `--drive` run** (no window) ends with an error naming the loss and the last state, exit
  status 1.

A restart is a process restart, not a rebuilt device inside the process: every renderer, texture
and pipeline of the old session holds the dead device, and rebuilding each is the whole game's
start-up. That is what the front end already does on a fresh launch.

## What stays unrecoverable

- A loss **inside a race** loses the race itself: the replay up to the loss is kept (what the
  auto-replay ring holds), the race is not resumable. A Speed Lap or Zone best lap set since the
  last exit from the race is kept by `wind_down` like any exit.
- **A hard kill of the process** (the driver taking the process down, the OOM killer) skips all of
  it. Nothing here writes on a timer ([`persistence.md`](persistence.md)).
- **wgpu issues the game cannot catch:**
  [#10222](https://github.com/gfx-rs/wgpu/issues/10222): after `VK_ERROR_DEVICE_LOST` from a
  submit, wgpu-hal drops resources the driver may still hold, which is wgpu's to fix;
  [#6279](https://github.com/gfx-rs/wgpu/issues/6279) and
  [#3043](https://github.com/gfx-rs/wgpu/issues/3043): "Parent device is lost" panics from
  `Queue::submit` on another thread are what the uncaptured-error handler turns into a flag, but a
  panic raised inside wgpu or a driver before it reaches the handler is not caught (the game's
  panic hook logs it and flushes, `oag_log::install_panic_hook`);
  [#10244](https://github.com/gfx-rs/wgpu/issues/10244): `request_device` answering `Lost` at
  start-up fails the bring-up (`Gpu::bring_up`'s "requesting the device" error), the same path as
  an adapter that will not serve, so a named adapter falls back to the default and the default
  ends the run with that message.
- **A second loss in a restarted run** ends it; see above.

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

Tests: `app::device_loss::tests` (the flag, the verdict on an error, a real device destroyed with
its later errors swallowed, the note), and `drive_ground_truth::a_device_lost_mid_race_saves_the_replay_and_ends_without_a_panic`
(`just test-data`; fails on a build whose frame loop does not ask).
