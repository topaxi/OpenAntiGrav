# ADR-0067: A multi-window race is one device and one submission per window

## Status

Accepted (2026-10-14, lane `multi-window`). Relates to
[ADR-0007](0007-fixed-timestep-vs-original.md) (the simulation steps on its own clock),
[ADR-0036](0036-ui-composites-at-presentation-resolution.md) (the presentation target the
HUD and the grade meet in) and [ADR-0037](0037-dynamic-resolution-varies-a-viewport-not-an-allocation.md)
(a viewport, not an allocation, is what varies). Nothing is superseded.

## Context

Split screen puts up to eight people's views in one window. A multi-monitor desk wants the
other shape: one window per person, each the whole-screen view of one craft. The simulation
is not the problem (one `RaceSim`, one tick, a camera, HUD and input slot per person already
exist); the application is. `App` held one `Session` holding one `Gpu` holding one window and
one surface, and every event handler assumed it.

## Decision

1. **One process, one device, one `Session`.** A window is an `Output` (window, surface,
   configuration, present modes), the part of `Gpu` that is not the device. Window 1 owns the
   front end, the pause menu, the pointer and the performance overlay, exactly as before;
   every further window is a `Seat` (`Output`, its own `upscale::Framebuffer`, a grid slot)
   that `Session` owns beside it. Nothing about windows reaches a gameplay crate.
2. **Every window runs the whole single-window chain**: scene, resolve, whole-screen HUD,
   Pulse bloom, grade. The scene differs only in which camera it is drawn from
   (`Scene::render_view`, `Layer::Window`). Not the split's cut-up path, which would lose the
   bloom and the post chains for no reason: a window starts at the target's origin and fills
   it, which is exactly what those chains assume.
3. **One command encoder and one queue submission per window, for the whole chain.** A queue
   write lands before the submission that follows it, and the scene, the upscaler and the HUD
   all write uniforms and instance buffers per frame; two windows in one submission would
   draw with the last window's values.
4. **The windows share one scene allocation**, the largest any of them wants, and each draws
   into its own extent of it. The scene's depth and multisample attachments are one size and a
   render pass wants its attachments equal. Each window keeps its own presentation target at
   its own size.
5. **A window closing is a seat leaving.** Closing window 1 quits, as it always did. Closing
   a seat window gives up that slot's seat (`Assignment::unseat`), so
   `split_screen::follow_devices` flies the craft as the AI from the next tick, and a pad
   cannot take it back. `CloseRequested` and `Destroyed` are one case: under X11 with no window
   manager a window is destroyed outright.
6. **Focus is lost only when every window has lost it**, after a short grace, so clicking from
   one window into another does not pause the race for everybody. A seat window being resized
   to nothing or hidden never pauses.
7. **Seat windows ask for a present mode that does not block** (`Mailbox`, then `Immediate`).
   Window 1 follows the VSYNC row and paces the loop; a second blocking surface acquired from
   the same thread would make the loop wait on a second display's vblank in turn.

## Rejected

- *One window, one surface, N logical views composited into it.* That is split screen.
- *A thread and a device per window.* Doubles every texture and pipeline, and the race is one
  object: there is nothing for a second thread to own.
- *The split's `render_viewports` per window with a one-view layout.* Loses the Pulse bloom
  and HD's chain per window, and corrupts nothing it needs to.
- *Closing a seat window flips `World::controllers` directly.* `follow_devices` runs every tick
  and would flip it back while the pad is still seated.
- *Per-window dynamic resolution, motion blur and a temporal upscaler.* Each holds one
  camera's history; the windows alternate cameras every frame. Chosen off, not measured.

## Consequences

- A windowed race draws the scene once per window: CPU cost grows with the window count, and
  the camera-independent work of a frame (shadow gathers, scenery clocks, advert cards) is done
  per window rather than once, because each window is a whole frame. `render_viewports` shares
  it; windows do not.
- No motion blur, no FSR 3.1 history and no dynamic resolution while a race is windowed. FSR
  3.1 takes the rung down to FSR 1, as it does on a menu frame.
- A surface per window means one swapchain and one present per window. Real multi-display pacing
  could not be measured under a virtual display; the command to measure it is in
  `docs/rendering/multi-window.md`.
- Desktop only. The browser, Android and iOS give the program one surface; the flag is an error
  there.
