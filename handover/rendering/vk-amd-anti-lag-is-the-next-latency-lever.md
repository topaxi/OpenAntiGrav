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
- **Hoist the swapchain acquire to before the tick loop** (maintainer,
  2026-10-13), a vendor-neutral latency win that needs no extension. Input is
  sampled once per tick in the frame's tick loop (`player_snapshots`), and the
  swapchain is acquired afterwards, as the first call in `Session::draw`
  (`crates/game/src/main/session/draw.rs`, `get_current_texture`). Under a
  blocking present mode (`Vsync::On`, or `smooth` falling back to Fifo) the
  acquire waits for a free image, so input sampled before that wait is up to a
  frame old when drawn. Move the acquire above the frame's `now`/elapsed read
  in `crates/game/src/main/session/frame.rs` (before `self.clock.advance`), so
  the wait counts toward this frame's elapsed and the ticks, the input and the
  draw alpha all describe the moment after it. The between-ticks draw keeps
  this easy: its alpha is read only in the race's render call, after the
  acquire, and nothing between the advance and the acquire depends on it
  (checked 2026-10-13; see `drawing-between-ticks-what-is-still-stepped.md`).
  This is also the natural place for `VK_ANTI_LAG_STAGE_INPUT_AMD` above.
- **Reach**: Vulkan on AMD only (RADV >= 26.3; AMD's Windows driver). A no-op
  elsewhere, so it must stay optional and detected, never required.
- **Local Mesa is 26.2.4** (2026-10-09), so nothing here is testable on the
  workstation yet; SteamOS's Mesa version is unchecked.
- **No latency measurement exists.** `perf::Meter` measures frame time, not
  input-to-photon. Measure before and after either change.

## Next Steps

1. Hoist the swapchain acquire (Open, above), measured with step 3's probe
   if it exists by then. An hour or two; any GPU.
2. When Mesa 26.3 is installed: check `vulkaninfo | rg anti_lag`, then check
   wgpu-hal 30 for a device-creation hook that takes extra extensions. 30 min.
3. Build an input-to-photon probe (or use an external camera / LDAT-style
   reading) so the effect is a number. Half a day.
4. Wire `vkAntiLagUpdateAMD` behind detection, `smooth` and `on` first. About
   a day if wgpu-hal allows the extension, longer if it needs a fork or patch.
