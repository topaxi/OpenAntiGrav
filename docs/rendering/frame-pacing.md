# Frame pacing: present mode and swapchain depth

How the VSYNC row becomes a present mode and a queue depth, and why a fullscreen window
queues one frame more than a window does. Code: `crates/game/src/main/window.rs`
(`present_modes`, `frame_latency`, `queued_frames`) and `Output::configure` in
`crates/game/src/main/gpu.rs`.

## The decision

| VSYNC | Present mode, best first | Frames queued in a window | Fullscreen |
| --- | --- | --- | --- |
| `off` | `Immediate`, `Mailbox` | 1 (`Immediate`), 2 (`Mailbox`) | at least 2 |
| `on` | `Fifo` | 1 | at least 2 |
| `smooth` | `Mailbox`, `Fifo` | 2 | 2 |

A swapchain holds the queued frames plus one image (`desired_maximum_frame_latency`; Metal's
`maximumDrawableCount`, Vulkan's `minImageCount`). The window column is **chosen, not
measured** (2026-10-09: one queued frame under `on` and `off` to save a frame of latency).
The fullscreen column is measured (below).

Every configure (creation, a resize, a VSYNC change) logs the decision once per change:

```text
surface: present mode Immediate (offered [Immediate, Mailbox, Fifo, FifoRelaxed]), 2 frame(s) queued, fullscreen
```

Fullscreen means `crate::window::is_fullscreen`: the saved borderless mode, F11, or gamescope.
An F11 toggle reaches the decision through the resize it causes.

## Why fullscreen needs the third image

A compositor that copies a window's image into its own frame lets the image go at once, so
two images are enough in a window. A display that scans a fullscreen surface out directly
(macOS direct-to-display, gamescope, an unredirected X11 or Wayland window) keeps the shown
image until the next one replaces it at a vblank. With two images, the image for the frame
after next is the one on screen, so acquiring it waits for that vblank. A frame whose GPU time
is over one refresh then costs two or three refreshes: 120 Hz becomes 60 or 40, whatever the
render resolution, because the wait follows the refresh and not the pixels.

Evidence, GitHub issue 7 (macOS, 120 Hz panel, Wipeout HD, VSYNC `off`, FRAME LIMIT 120, the
reporter's performance overlay):

| | Window | Render | FPS | FRAME | SCENE+BLOOM+BLUR as printed | PRESENT |
| --- | --- | --- | --- | --- | --- | --- |
| windowed 960x544 | 2144x1376 | 1920x1088 | 102 | 9.8 ms | 11.2 ms | 5.09 ms |
| borderless, FULLSCREEN RESOLUTION 960x544 | 3456x2168 | 960x544 | 61 | 16.3 ms | 11.4 ms | 8.85 ms |

The GPU work is the same, the frame is 6.5 ms longer and the extra time is in `PRESENT`, which
holds the acquire's wait (`Session::present_cost`). The reporter's own summary was 120 fps
windowed against 40 fullscreen at both 960x544 at 25 % and 1600x900 at 100 %.

Under gamescope VSYNC `off` already ran `Mailbox` with two queued (gamescope's WSI offers
`[Mailbox, Fifo]`, no `Immediate`), so there only `on` changes. A headless gamescope
(`gamescope --backend headless`) composites and never scans out, so it does not reproduce the
wait; the workstation check is the decision line above.

## The Steam Deck's 60 (a different cause)

A second report (maintainer, 2026-10-26: a Steam Deck OLED at 90 Hz reading 60 fps with a
spiky graph and an 8 ms `CPU` row) is **not** this mechanism. Settings there were
`vsync = "smooth"` with `frame_limit = "90"`, which already queues two frames, and gamescope's
WSI keeps three images either way. Native under the Deck's gamescope with `vsync = "on"` at
200 % render scale, the build before and after this change wrote byte-identical frame logs.

The cause is the frame thread's CPU on one circuit. The maintainer raced HD's Anulpha Pass:
`frame-phases` total 9.4 ms (submit 5.1, scene 2.5, ticks 1.0). Past 11.1 ms a frame takes two
vblanks, so frames alternate one and two: 60 fps, p99 22.3 ms. Autopilot on the same Deck and
circuit: submit 5.0 to 5.3 ms, total 7.5 to 7.8, still 90; on `--race`'s default circuit submit
is 2.5 and total 5.1. The present wait of about 6.6 ms under `smooth` is normal under gamescope
and shows at a flat 90 too. A profile of the submit puts wgpu-core's per-element texture
tracking of the binding-array group on top (`crates/mesh/src/mesh_render/bindless.rs`); see
[`frame-thread-cpu.md`](frame-thread-cpu.md) for the Deck method.
