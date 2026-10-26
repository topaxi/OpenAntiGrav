---
status: blocked-hardware
titles: [all]
updated: 2026-10-14
summary: "Web multi-window: wire a popup canvas after desktop multi-window lands; real pad, hidden opener, second monitor, Firefox"
categories: [frontend, rendering]
---

# Web multi-window: a second canvas in a popup, spike done, wiring open

2026-10-14, maintainer's request. The spike ran the same day and answers both
Open items: **it works**, with one design constraint (the opener's frame loop,
below). Nothing is wired into the game: `--windows` is still refused on wasm,
and the in-game run needs an edit to `open_screens` in `session/windows.rs`, the
desktop multi-window lane's file (see "Wiring", below).

## The idea

On the web, a "window" is a `<canvas>` in a same-origin popup (`window.open`). The
popup shares the page's thread and event loop, so the canvas is an ordinary
`HtmlCanvasElement` (what winit's `with_canvas` and a wgpu surface take) and one
WebGPU device can present to both. Pure or Pulse first: the web build already
holds a Pulse PSP race at the 60 fps cap ([web.md](../../docs/tools/web.md)); HD
and Omega follow the integrated-GPU rule as on the desktop.

## What the spike measured

Chromium 153 (`/usr/bin/chromium`, WebGPU on, hardware adapter in headless;
software Vulkan in the headful Xvfb runs), 2026-10-14. Two harnesses, neither
the game:

1. **A plain JS page** (`scripts/web-popup-probe.html`): one `GPUDevice`, two
   canvases (the page's and one made in the popup's own document), a clear
   colour per frame into each, a `requestAnimationFrame` loop per window or a
   single one, `keydown` and `getGamepads()` per window.
2. **A Rust wasm crate on this workspace's exact `winit` 0.30.13 and `wgpu`
   30.0.1** (not committed, about 150 lines): winit window 1 on the page's
   canvas, window 2 on the popup's canvas through `with_canvas`, one device, a
   clear per window per `RedrawRequested`, and every `WindowEvent` logged per
   window. It was built with the stable toolchain, no threads, and takes the
   same calls the game's `gpu::web::canvas` and `Gpu::open_output` make. Its
   essential part, so the claim can be rebuilt:

   ```rust
   // The page opens the popup from a click, makes a <canvas> in popup.document, and
   // calls start(canvas, ..) on the module. The canvas is another realm's object.
   #[wasm_bindgen]
   pub async fn start(popup_canvas: JsValue) -> Result<(), JsValue> {
       let popup_canvas: web_sys::HtmlCanvasElement = popup_canvas.unchecked_into(); // never dyn_into
       // one instance, adapter and device, requested as the game's gpu::web::prepare does
       event_loop.spawn_app(app); // winit 0.30, ApplicationHandler
   }
   // in ApplicationHandler::resumed, for each of the two canvases:
   let attrs = Window::default_attributes()
       .with_canvas(Some(canvas.clone()))
       .with_prevent_default(true)
       .with_focusable(true);
   let window = Arc::new(event_loop.create_window(attrs)?);
   let surface = instance.create_surface(window.clone())?; // or SurfaceTarget::Canvas(canvas)
   // inner_size() is 0x0 here: configure the surface on WindowEvent::Resized, which follows.
   // per RedrawRequested: get_current_texture(), a clear pass, queue.submit, queue.present(frame),
   // then window.request_redraw().
   ```

The popup is `window.open('', name, 'popup,width=640,height=360')`: an
`about:blank` document inherits the opener's origin and its cross-origin
isolation (`popup.crossOriginIsolated` is true and `popup.opener === window`),
so COOP/COEP does not sever it. A same-origin URL served by
`scripts/serve-web.py` would carry the same headers; a URL without them would
make COOP sever the opener link.

### 1. Can one device present to a popup canvas? Works (high confidence)

- **wgpu**: `canvas.getContext('webgpu').configure({ device })` on a canvas in
  the popup's document, with the device requested in the opener, presents.
  Both canvases showed their own clear colour in screenshots of each page.
  Through Rust, `Instance::create_surface(SurfaceTarget::Canvas(c))` and
  `create_surface(winit_window)` both worked on the popup canvas: winit 0.30
  hands wgpu a `RawWindowHandle::WebCanvas` pointer (`wgpu` takes it with
  `unchecked_into`), so no document lookup happens.
- **winit**: `WindowAttributes::with_canvas(Some(popup_canvas))` is accepted as
  a second window on the same event loop (`WindowId(2)`), with its own
  `Resized`, `Focused`, `Occluded`, keyboard and mouse events. winit's resize
  observer (created in the opener) observes the popup's canvas and reports
  its size (`inner_size` reads 0x0 at creation and `Resized 640x360` comes
  next, as for window 1, so configure the surface on that event).
- **The trap**: a popup's canvas is an instance of the *popup's*
  `HTMLCanvasElement`, so wasm-bindgen's `dyn_into::<HtmlCanvasElement>()`
  returns `Err` (`c2 instanceof HTMLCanvasElement` was false in the opener's
  realm, true against `popup.HTMLCanvasElement`). `gpu::web::canvas` ends in
  `.ok()`, so a `dyn_into` on the popup canvas would silently give `None` and
  winit would make a canvas of its own in the opener: it looks like it works
  and does not. Use `unchecked_into`.
- **Rate** (headless, two windows each presenting every frame): 59.97 fps in
  the page and 59.97 in the popup with winit's single loop (both counted from
  the one callback); 59.96 and 60.13 with a `requestAnimationFrame` loop in
  each window of the JS probe. In a real X11 window manager (openbox on Xvfb,
  headful, software Vulkan) both held 60.0 while visible, whichever window had
  focus, and the popup canvas did change between two screenshots of its window
  taken 0.37 s apart.
- **Throttling, measured with a real window manager** (headless cannot show it:
  `document.hasFocus()` is true for every page there, and Playwright emulates
  focus; the runs below launch Chromium directly and move windows with
  `xdotool`):

  | state (geometry from `xdotool getwindowgeometry`) | opener rAF | popup rAF | visibility |
  | --- | --- | --- | --- |
  | popup as the browser opened it: 700x534 at 1,22 over the opener's 700x500 at 0,0, popup focused | **0** | 60 | opener hidden |
  | popup at 300,150 or 100,50 or 40,40 (partly over the opener), popup or opener focused | 60 | 60 | both visible |
  | popup beside the opener, no overlap, either focused | 60 | 60 | both visible |
  | popup fully covered by the opener, or minimised | 60 | **0** | popup hidden |
  | opener minimised, popup visible and focused | **0** | 60 | opener hidden |

  The popup opened at the opener's size, not at the `width=640,height=360` the
  call asked for: that run launched Chromium with `--window-size=700,500`, which
  applied to the popup as well, so the first row is a popup that covers the
  opener (all but a 1 pixel column), not what a default player sees. What it
  shows is the rule: **a window that is fully covered or minimised is hidden
  and stops its own `requestAnimationFrame`; a partial overlap does not hide
  it** (the three overlap rows ran 60/60 with each window focused in turn).
  winit's single loop runs on the *opener's*: with the opener hidden or
  minimised the whole game, including a visible popup, stops (the winit spike
  read 0.0 fps for both windows and `Occluded true` on both). With the popup
  hidden the loop keeps drawing into it (the frame counter ran at 60) for no
  benefit, and `getCurrentTexture` did not fail. How often a player's popup
  lands fully over the opener (a maximised popup on the same monitor) depends
  on the browser's and compositor's placement: untested beyond openbox.

### 2. Input: works (gamepad on a virtual pad, not a real one)

- **Gamepad.** A uinput Xbox-shaped pad (`scripts/uinput-pads.py`, one pad)
  was visible to `navigator.getGamepads()` in the opener *and* on
  `popup.navigator`, with identical button and axis values, and
  `gamepadconnected` fired in both windows. With real focus changes (openbox):
  both navigators carried live data whichever window had focus, so **focus is
  not what gates it, visibility is**: a hidden document's navigator lists the
  pad but reports no pressed buttons (popup covered or minimised: no pressed
  sample in the popup, the opener still live; opener minimised: the reverse).
  The pad appears only after its first button press, as in any Chromium page.
  So "poll the focused window" is sound, and "poll any visible window" is the
  simpler rule that is also true. Real pads: **untested** (the command to try
  one is at the end).
- **gilrs on the web polls only the opener**: `gilrs-core` 0.6.8's wasm backend
  calls `web_sys::window().navigator().get_gamepads()` every poll, no document
  argument. With the opener visible that is enough (both navigators agree); with
  it hidden it reads zeros, which only matters once the frame loop outlives a
  hidden opener (below).
- **Keyboard.** winit's own listeners sit on each *canvas*, so a key reaches the
  right window's `WindowEvent::KeyboardInput` with no listener of ours, **once
  the canvas has focus** (a click on it, or `focus()`; `with_focusable(true)`
  gives it a tabindex). Checked with real X11 key events to the focused popup,
  then the opener, then the popup again: each landed in its own window's events.
  A key pressed in a popup before its canvas has focus goes to the body only;
  a `keydown` listener on `popup.document` (also tried) catches those.
- `WindowEvent::Focused` works per window (`popup: Focused true`, `main:
  Focused false` as the focus moved), so `windows::Focus` can follow it.

### Noted, not fixed

- **Fullscreen** needs a click in the window that goes fullscreen:
  `popupCanvas.requestFullscreen()` from a click inside the popup resolves
  (`popup.document.fullscreenElement` set, the opener's not); from a click in
  the opener it rejects with `TypeError: Permissions check failed`. winit's own
  fullscreen helpers read the *opener's* `document` (`Canvas::document` is the
  page's), so its `fullscreen()` getter would be wrong for a popup canvas. The
  game already routes web fullscreen through the page (`oagFullscreen` in
  `web/main.js`); that function needs a per-window form, called from the
  popup's own click, and a pad button cannot trigger it (same rule as the main
  window: [web.md](../../docs/tools/web.md)).
- **Placement** on another screen needs the Window Management API
  (`getScreenDetails()`): present in Chromium (`'getScreenDetails' in window`
  true in both windows, permission state `prompt`), a permission prompt, absent
  in Firefox and Safari. Without it the popup opens where the browser puts it
  and the player drags it, so `seat_placement`'s per-monitor spread has no web
  equivalent beyond that. Untested with a second monitor.
- **Pacing.** Two `requestAnimationFrame` loops (one per window) and one loop
  driving both canvases both held the display's 60 in these runs. The
  difference that matters is the one in the table above: one loop on the opener
  stops everything when the opener is hidden; one loop per window stops only the
  hidden window. Two monitors at different refresh rates were not tried.
- **Popup blockers.** `window.open` needs a user gesture, so the popup opens
  from a button click on the page, before or after the disc is picked, not from
  the game. `--disable-popup-blocking` (not a player's setting) was used for
  the unattended headful runs.

## Wiring (what is left, not done)

1. **In-game run on Pure or Pulse: not done.** The desktop path is
   `Session::open_screens` -> `Gpu::open_output(event_loop, Look, modes)`, which
   builds window 2 with `window::attributes`, then
   `instance.create_surface(window.clone())`. Reading it, the changes are small
   but the hook point, `open_screens`, lives in `session/windows.rs`, the desktop
   multi-window lane's file, so none was made (the spike's brief kept it to the
   web entry point, the page and a probe):
   - drop the `cfg(not(wasm32))` from `open_screens`, `open_output`,
     `seat_placement` (monitors are empty on the web, so it returns
     `(None, None)` on its first lookup), `seat_mode`, `window_title` and
     `seat_present_modes` (the web surface offers `Fifo` only; `apply_modes`
     already falls back to it and says so);
   - a web branch in `open_output` that takes the popup canvas from a page
     function (`oagTakePopupCanvas`, wasm-only, next to `gpu::web::canvas`),
     `unchecked_into`, and applies the same `inner_size = None` /
     `fullscreen = None` / `with_canvas` / `with_focusable` / `with_prevent_default`
     the first window gets;
   - `web.rs` accepting extra CLI arguments (`--windows 2 --players 2`) from the
     page so a spike run reaches a split race without the join screen;
   - `web/main.js`: a button that opens the popup and builds its canvas from the
     click, and a per-window fullscreen helper.
2. **The frame loop must not live only on the opener's rAF.** winit 0.30 has no
   hook to drive its loop from another document's `requestAnimationFrame`, so
   either the opener stays visible (a "game is in the popups" page that shows a
   status and is never covered), or the game runs with `ControlFlow::Poll`
   (winit then schedules through a `MessageChannel`, which hidden pages still
   run; not tried) and presents from there. Unresolved: it decides whether a
   hidden opener is a supported state.
3. **gilrs** needs the popup's navigator when the opener is hidden, or the
   opener kept visible (point 2).
4. Seat windows' framebuffers, HUD per window and the layout screen are desktop
   code that compiles on the web once the cfgs go; not run in a browser.

**Estimate**: 1 to 2 days for points 1 and 3 on top of the desktop multi-window
code, as the first plan said; point 2 is a decision plus about a day if
`ControlFlow::Poll` works. **Fallback if the opener-hidden state is not
acceptable**: one canvas, split layout, which works today.

## Try it with a real pad

```sh
python3 scripts/serve-web.py --dir scripts --port 8000   # COOP/COEP, as the page needs
chromium --enable-unsafe-webgpu --enable-features=Vulkan http://127.0.0.1:8000/web-popup-probe.html
```

Click the button (the popup opens), press a button on the pad, and read the
page: it prints each window's `getGamepads()`, `hasFocus()` and
`visibilityState`, both frame counts, the last keys and the fullscreen result
from a click in the popup. Try covering and minimising each window. `?mode=single`
drives both canvases from one loop.

## Open

- A real pad (the spike used a virtual one on uinput).
- The in-game run, once the desktop multi-window code lands (Wiring, 1).
- Whether `ControlFlow::Poll` keeps the game running with a hidden opener.
- A second monitor at a different refresh rate, and `getScreenDetails()`.
- Firefox and Safari (Chromium only here; Firefox draws slowly, web.md).

## Next Steps

- After desktop `multi-window` merges: wiring 1 and 3 above, then the in-game run
  on Pure with two windows, then settle point 2.
