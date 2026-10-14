---
categories: [frontend, rendering]
---

# Web multi-window: a second canvas in a popup, spike not started

2026-10-14, maintainer's request. A plan, nothing built. It follows the desktop
multi-window lane (a list of windows, each holding seats and a cut; see section 2
of [the split-screen and network thread](split-screen-multi-window-network-play-in-order.md)),
and should not start before that lands.

## The idea

On the web, a "window" is a `<canvas>` in a same-origin popup (`window.open`). The
popup shares the page's thread and event loop, so the canvas is an ordinary
`HtmlCanvasElement` (what winit's `with_canvas` and a wgpu surface take) and one
WebGPU device can present to both. Pure or Pulse first: the web build already
holds a Pulse PSP race at the 60 fps cap ([web.md](../../docs/tools/web.md)); HD
and Omega follow the integrated-GPU rule as on the desktop.

## Input: poll from whichever window has focus (maintainer, 2026-10-14)

A browser may withhold gamepad data from an unfocused document, but one of the
windows will be focused, and every pad is mapped to a seat in one place
(`oag_input::pad::Assignment`), so it does not matter which window receives
input. Poll `getGamepads()` on the focused window's own `navigator` (the opener
can reach `popup.navigator`), and attach keyboard listeners to both documents.
winit's web backend attaches its listeners to the page's own `window`, so popup
keys likely need listeners of our own.

## Open (for the spike)

- Whether wgpu and winit accept a canvas from another document at all.
- That a focused popup gets live pad data through the route above. The web build
  has never been tried with a gamepad (web.md).
- Placement: a popup needs a click to open, fullscreen needs a click per window.
  Placing on a second monitor needs the Window Management API (Chromium only, a
  permission prompt); elsewhere the player drags the popup themselves.
- Pacing: `requestAnimationFrame` drives every frame and a canvas offers only
  `Fifo`; two monitors at different refresh rates may stutter.
- Fallback if popups fail: one canvas, split layout, which works today.

## Next Steps

- After desktop `multi-window` merges: a half-day spike on Pure, two canvases,
  answering the first two Open items before anything is wired. Wiring after it
  is about 1-2 days.
