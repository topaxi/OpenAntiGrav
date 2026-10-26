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
