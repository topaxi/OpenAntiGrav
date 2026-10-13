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

**2026-10-13, read from `vulkaninfo` on the workstation (RX 7800 XT, RADV, Mesa
26.2.4, niri on Wayland) and wgpu 30.0.1: the hint is probably a no-op there.**
wgpu-core clamps `desired_maximum_frame_latency` to the range
`(min_image_count - 1)..=(max_image_count - 1)` that wgpu-hal reports
(`wgpu-core/src/device/resource.rs`, `wgpu-hal/src/vulkan/swapchain/native.rs`).
This driver reports `minImageCount` 4 on the base Wayland surface and 3 on the
X11 one (per present mode: Fifo and Immediate 3, Mailbox 4). So a latency of 1
becomes 2 on X11 and 3 on Wayland, the same as the old default, and Mailbox's 2
becomes 3. Unverified: that wgpu-hal reads the base capabilities and not a
per-mode query, and how deep Mesa's Wayland Fifo really queues (its present
throttling may cap it below the image count). The fix for "one frame fewer" on
this driver is not the hint but the present-time wait or the acquire order below.

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
- ~~**Where "input sampling" is.** The simulation ticks at a fixed 60 Hz inside
  a faster render loop; the call must sit right before the frame's
  `InputSnapshot` is built, which the loop does not yet treat as one point.~~
  **Answered 2026-10-13 (read from the code, not measured).** Keys are latched
  when winit delivers them (`app.rs`, `set_key`); the pads are polled and the
  merged snapshot built by `self.controls.player_snapshots()`
  (`session/frame.rs`, in the tick loop right before `Race::tick`), once per
  tick (`poll_players` drains the gilrs events itself). Input is therefore
  already sampled as late as a tick allows. A `vkAntiLagUpdateAMD(INPUT)` call
  belongs before `now` is read at the top of the fixed-timestep block, not per
  tick and not just before `self.clock.advance`: otherwise the driver's sleep
  falls outside this frame's `elapsed` and the ticks are computed from a
  pre-sleep time. The `about_to_wait` limiter waits as well, so with anti-lag
  active one of the two waits has to go.
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
  **Who is exposed, and what to watch** (added 2026-10-13). It applies to `on`
  and to `smooth` when it falls back to `Fifo`; `off` (`Immediate`) and `smooth`
  on `Mailbox` do not block there, so the maintainer's own setting is
  unaffected where Mailbox is offered. **Unchecked on the Deck**: if gamescope's
  surface does not offer Mailbox, `smooth` falls back to `Fifo` at latency 2
  (`present_mode` warns only once), and the cost there is up to two queued
  frames, not one. Pitfalls for the change: an early return in `frame` would
  drop the held image, a load stall would hold it for hundreds of milliseconds,
  `Outdated` and `Lost` need the reconfigure path, the `present_cost` meter
  times the acquire inside `draw`, and the web build has no blocking acquire.
- **Reach**: Vulkan on AMD only (RADV >= 26.3; AMD's Windows driver). A no-op
  elsewhere, so it must stay optional and detected, never required.
- ~~**Local Mesa is 26.2.4** (2026-10-09), so nothing here is testable on the
  workstation yet; SteamOS's Mesa version is unchecked.~~ **Corrected
  2026-10-13: it is testable now.** Mesa 26.2.4 ships the implicit layer
  `VK_LAYER_MESA_anti_lag`, and `vulkaninfo` lists `VK_AMD_anti_lag` revision 1
  as a layer-device extension on the RX 7800 XT (and on the iGPU and llvmpipe).
  That layer's algorithm is the basic one (submit-to-execution delay only); the
  RADV-native pacing is 26.3. So the API wiring can be tried today and the
  pacing quality compared when 26.3 lands. SteamOS's Mesa version is still
  unchecked.
- **Present modes on the workstation.** Wayland (niri) offers `Mailbox` and
  `Fifo` only, so `off` resolves to `Mailbox` there, not `Immediate`; X11 (xcb,
  xlib) offers all four. `smooth` gets `Mailbox` on both, so the workstation's
  `smooth` is not exposed to the acquire-order cost. The Deck is still unchecked.
- **Implicit layers are installed and will skew a measurement.** MangoHud,
  vkBasalt, Steam's overlay, gamescope's WSI layer and lsfg-vk (a frame
  generation layer that fails to load: "Failed to find 'vkGetInstanceProcAddr'
  in layer liblsfg-vk-layer.so"). Disable them, or set
  `VK_LOADER_LAYERS_DISABLE`, for any latency number.
- **No latency measurement exists.** `perf::Meter` measures frame time, not
  input-to-photon. Measure before and after either change.

## Next Steps

1. Hoist the swapchain acquire (Open, above), measured with step 3's probe
   if it exists by then. An hour or two; any GPU.
1b. On the Deck, look for the log line "this surface offers ..., so vsync smooth
   falls back to Fifo" (or run `vulkaninfo` for the present modes). Two minutes
   once someone is at the Deck; it decides how much `smooth` is exposed to the
   acquire-order cost.
2. When Mesa 26.3 is installed: check `vulkaninfo | rg anti_lag`, then check
   wgpu-hal 30 for a device-creation hook that takes extra extensions. 30 min.
3. Build an input-to-photon probe (or use an external camera / LDAT-style
   reading) so the effect is a number. Half a day.
4. Wire `vkAntiLagUpdateAMD` behind detection, `smooth` and `on` first. About
   a day if wgpu-hal allows the extension, longer if it needs a fork or patch.
