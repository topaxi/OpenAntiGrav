---
status: ready-to-wire
titles: [pulse, pure, hd, 2048, omega]
updated: 2026-11-13
summary: "Pulse's demo and per-output wallpaper landed; HD's screen, other titles' stations, the wallpaper on niri/X11/Windows open"
categories: [rendering]
difficulty: medium (45)
---

# The attract demo's leftovers

`docs/ui/attract-demo.md` is what runs. Open:

## Open

- **HD, 2048 and Omega draw no `Demo InGame` screen.** Pulse's (PSP and PS2) is wired and Pure inherits it (its
  `demo_logo.mip` is a 0-byte entry, so it draws the rules and the text); HD authors the same state in
  `demo_definition.xml` (`demo_logo_HD.gtf`, `BOOT_PRESS_ANY`, two lines animating in) on its
  1920x1080 grid, which `oag_raceplay::demo_overlay` does not read yet (its `Frontend` space and a
  `.gtf` through `Archives::read_texture`). 2048 and Omega take HD's.
- **The demo camera on Pure, HD, 2048 and Omega.** `Camera::Spectator` is inherited from Pulse, but
  `finish_camera` needs the circuit's stations, read for Pulse's PSP disc only, so those titles
  keep the chase camera.
- **`Reset Demo`'s fade between demos** (we cut through the loading screen); HD authors it
  (0.4 s fade, 0.7 s hold).
- **Why the original's demo never finishes on laps** (its finish path in mode 2) is unread; the
  behaviour is measured and kept (`UNENDING_LAPS`).
- **Pulse's demo grid and laps are not forced.** `Rules::craft` (8) is pinned by the Pulse ground
  truth, not applied (a Single Race grid is already eight there); on a title whose grid differs
  the demo takes its own. `Rules::laps` (3) applies only to a demo that ends on laps, which the
  original's does not.
- **The wallpaper's leftovers** (the desktop background per output, a camera each, landed:
  `docs/ui/attract-demo.md`, "A desktop background"):
  - **Not run on niri live**: the maintainer runs
    `cargo run --release -p oag-game -- data/images/pulse-psp-eu.chd --no-audio --wallpaper` from the
    main checkout on the niri session and checks a
    background on each output, distinct cameras, and that drawing stops while the screen is locked
    (CPU of the process near zero).
  - **Fractional scale** (`wp_fractional_scale_v1` plus `wp_viewporter`): drawn at the next integer
    scale up today.
  - **A desktop background on X11 (root window), Windows (WorkerW) and macOS (desktop-level
    window)**: not built; there `--wallpaper --windows N` gives N ordinary windows, a camera each (checked on X11
    under Xvfb).
  - **A further output's chase camera stays on its craft once it is wrecked** (seen on sway: a
    burnt `shipwreck.vex` hull filling a 1920x1080 output); moving it to a live craft is open.
  - Window 1's own camera is the spectator director, which can frame the craft a further output
    chases for a while; the further outputs never share a craft with each other.
- **2048 and Omega read `DemoConfig.xml`'s circuit and timeouts but not its demo music
  (`Data\Music\07`) or ship.** Pulse's selection was measured on Pulse PSP only: Pure, HD and the
  rest inherit it.
- **2048's idle count.** Its main screen is the touch front end (`Stage::Frontend`); `tick_demo`
  only counts on `Stage::Menu`, so an untouched 2048 front end starts no demo (`--demo` works).
  The authored 30 s is read and ready.
- **Pure and Pulse PS2 were not driven through the idle path**, and Pure's demo is limited to
  the player's own team (its team gates are read from the archives, which a demo may not open on
  the frame thread).
- **No pointer or touch test beyond the drive verbs** (`mouse`, `click`); a real winit key event
  sets `Session::demo.key` in `app.rs` and is checked by hand only.

## Next Steps

1. Read HD's `demo_definition.xml` screen (grid, `.gtf` logo) through `oag_raceplay::demo_overlay`.
2. The maintainer runs the wallpaper on niri (command in Open).
3. Read the spectator stations for the titles other than Pulse PSP, then their demos watch through them.
