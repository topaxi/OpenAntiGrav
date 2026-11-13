---
status: ready-to-wire
titles: [pulse, pure, hd, 2048, omega]
updated: 2026-11-13
summary: "The attract demo runs; its Demo InGame overlay, per-output wallpaper cameras and the rest of the measured demo rules are open"
categories: [rendering]
difficulty: medium (45)
---

# The attract demo's leftovers

`docs/ui/attract-demo.md` is what runs. Open:

## Open

- **The `Demo InGame` overlay is not drawn.** Pulse's original shows `demo_logo.mip` and a
  pulsing `BOOT_PRESS_ANY2` over the HUD-less race (`Demo_Definition.xml`, measured by `demo-re`);
  HD authors the same state (`demo_logo_HD.gtf`, `BOOT_PRESS_ANY`, two lines animating in). Our
  demo draws the bare race. The screens are authored data: parse and draw them, do not stand in.
- **Measured demo rules not wired**: no track description, no Now Playing, no camera shake, no
  profile stats, AI weapon use doubled, class Flash with three laps (we keep the player's class
  and the race's laps), and the `Reset Demo` fade between demos (we cut through the loading
  screen).
- **A wallpaper per output with its own camera.** The user's wish: a window per monitor, each with
  an individual camera. The multi-window layer (`docs/rendering/multi-window.md`) deals *seats*
  across windows, and a demo has none. It needs camera-only views that follow a chosen AI craft
  (`oag_raceplay::viewpoint`) per window, then `--wallpaper --windows N`. A true desktop-background
  surface (Wayland layer-shell, X11 root, Windows WorkerW) is separate, platform work.
- **2048 and Omega read `DemoConfig.xml`'s circuit and timeouts but not its demo music
  (`Data\Music\07`) or ship.** Pulse's selection was measured on Pulse PSP only: Pure, HD and the
  rest inherit it.
- **No pointer or touch test beyond the drive verbs** (`mouse`, `click`); a real winit key event
  sets `Session::demo.key` in `app.rs` and is checked by hand only.

## Next Steps

1. Draw the `Demo InGame` state (logo and pulsing text) over a demo, per title, from the disc's XML.
2. Add camera-only views for wallpaper windows.
3. Skip the track panel and Now Playing in a demo, and the camera shake.
