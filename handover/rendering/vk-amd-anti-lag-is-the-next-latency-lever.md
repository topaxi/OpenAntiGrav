# VK_AMD_anti_lag is the next latency lever, and Mesa 26.3 brings it to RADV

Opened 2026-10-09 at the maintainer's request. The maintainer plays with
`vsync = "smooth"` on an AMD workstation and an AMD Steam Deck, so this is the
setup that matters first.

## What landed with this note

`crates/game/src/main/window.rs::frame_latency` sets
`SurfaceConfiguration::desired_maximum_frame_latency` per resolved present
mode (before this, wgpu's default 2 everywhere). Vulkan's swapchain holds that
plus one images:

- `on` (`Fifo`) and `off` (`Immediate`): **1**, one queued frame fewer.
- `smooth` (`Mailbox`, or its `Fifo` fallback) and `off` falling back to
  `Mailbox`: **2**, unchanged. Mailbox needs a third image to discard into,
  and `smooth`'s promise is never halving.

**So the maintainer's own `smooth` setting is unchanged by it.** Chosen, not
measured: no input-to-photon number exists for any mode.

## What VK_AMD_anti_lag would add

The driver measures frame time and queuing delay and sleeps the app at
`vkAntiLagUpdateAMD` so input is sampled as late as the GPU allows. RADV's
implementation (Mesa 26.3, built on the new `vk_frame_pacer` runtime helper)
computes `delay = avg_frame_time + queuing_delay - min(deviation, 1 ms)` from
the previous input begin, and replaces the older `MESA_LAYER_ANTI_LAG`.

It helps where frames queue: `on`/`smooth`, and any GPU-bound frame (the
Deck, heavy HD/Omega circuits). Under `off` below the limiter's cap there is
little to remove: the limiter already waits *before* a frame
(`crates/game/src/main/app.rs`, `about_to_wait`), which is the same idea.

## Open

- **wgpu 30 does not expose it.** It needs the device extension enabled at
  device creation through wgpu-hal's Vulkan backend, then a raw `ash` call to
  `vkAntiLagUpdateAMD` with `VK_ANTI_LAG_STAGE_INPUT_AMD` before input is
  read and `VK_ANTI_LAG_STAGE_PRESENT_AMD` around present. Whether wgpu-hal 30
  lets a caller add a device extension is unchecked.
- **Where "input sampling" is.** The simulation ticks at a fixed 60 Hz inside
  a faster render loop; the call must sit right before the frame's
  `InputSnapshot` is built, which the loop does not yet treat as one point.
- **The swapchain acquire hoist landed 2026-10-13** (vendor-neutral, no
  extension): `Session::frame` acquires the image (`Gpu::acquire`,
  `crates/game/src/main/gpu.rs`) before its `now`/elapsed read, so a blocking
  acquire's wait counts toward the frame's `elapsed` and the ticks, the pad
  input they sample (`player_snapshots` polls gilrs there) and the draw alpha
  describe the moment after it. Keyboard events that arrive during the wait
  still reach the next frame (the event loop is blocked in it). See
  `docs/architecture/engine-lifecycle.md`, "Frame structure". **Not
  measured**: no member's sandbox can present (no lavapipe installed, RADV
  needs DRI3, which Xvfb lacks), so the before/after acquire wait under Fifo
  is still to be read on a real display with the `frame timing` debug line
  (`RUST_LOG=info,oag_game=debug`; acquire wait and where the frame's input
  sample sits against it, once a second). `VK_ANTI_LAG_STAGE_INPUT_AMD`
  belongs right after that acquire, before the clock read.
- **Reach**: Vulkan on AMD only (RADV >= 26.3; AMD's Windows driver). A no-op
  elsewhere, so it must stay optional and detected, never required.
- **Local Mesa is 26.2.4** (2026-10-09), so nothing here is testable on the
  workstation yet; SteamOS's Mesa version is unchecked.
- **No latency measurement exists.** `perf::Meter` measures frame time, not
  input-to-photon. Measure before and after either change.

## Next Steps

1. Read the hoist's effect on a real display: the `frame timing` debug line
   on a build of `edc07a0c6` (the counter on the old order) and one of
   `b0ec1779a` or later (the hoist), `vsync = "on"`, a minute of racing each.
   Ten minutes, maintainer.
2. When Mesa 26.3 is installed: check `vulkaninfo | rg anti_lag`, then check
   wgpu-hal 30 for a device-creation hook that takes extra extensions. 30 min.
3. Build an input-to-photon probe (or use an external camera / LDAT-style
   reading) so the effect is a number. Half a day.
4. Wire `vkAntiLagUpdateAMD` behind detection, `smooth` and `on` first. About
   a day if wgpu-hal allows the extension, longer if it needs a fork or patch.
