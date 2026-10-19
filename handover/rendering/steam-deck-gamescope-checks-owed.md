# Steam Deck under gamescope: the fixes landed, the Deck confirmations are owed

2026-10-14, the split-screen render-perf drive. The Deck went to sleep before
the last two lanes could confirm on it. Everything below was fixed and tested
on this host. The parts that only the Deck's own gamescope shows (its game
Xwayland reports 100mm x 160mm, so winit reads scale 6; a local gamescope
reports 1016x572 mm and scale 1) still need a run there. Evidence:
`docs/tools/packaging.md` "Under gamescope", `docs/rendering/multi-window.md`,
`docs/rendering/split-screen.md` "Cost".

## Open

- **Borderless first size (scale 6).** winit reports a freshly fullscreened X11
  window at monitor x scale (23040x12960). Every surface size is now held to the
  monitor under gamescope (`window::held_to_monitor`, `window::surface_size`).
  On the Deck one single-view launch on an earlier build of the fix was clean.
  Four more single-view launches and one `--players 2` launch on the final
  build were not run.
- **The maintainer's original report is unreproduced.** "FPS tanks the moment
  CUT goes VERTICAL on WINDOW LAYOUT" did not show on the screen itself, on the
  race START leads to, or on a CLI split, on the Deck or here. What was found
  instead: the stale BLUR overlay row, the 6x window under gamescope (5 fps
  windowed), and 4K output cost (now a 1920x1080 pixel-budget default under
  gamescope).
- **The CLI AI-order fix is not confirmed on the Deck.** On the iGPU the frame
  cost did not move (22.17 -> 22.27 fps), so no Deck change is expected.
- **Deck leftovers.** `~/oag-perf/borderless-6x` is still on the Deck; remove it.
- **HD at 4K on the Deck** still drops to 28 fps on its worst stretch under the
  pixel budget. The budget is not tuned per title. 2026-10-19 (`docs/rendering/high-resolution-cost.md`,
  "Steam Deck, 4K"): scene only, HD at the 1080p budget is 8.5 / 11.7 ms p50 / p99, so the 28 fps is
  HUD, bloom and compositor on top; the budget itself was judged right and left as it is.
- **RENDER SCALE row** shows no "auto" label while the gamescope budget is in
  effect.

## Next Steps

1. Build `just appimage-deck` from main. In a worktree, first replace the
   `data/appimage` symlink with a real directory. Copy it to
   `deck@steamdeck:oag-perf/<lane>/` and run it there with
   `--appimage-extract`, under `flock -o ~/.cache/oag/deck.lock`, with its own
   `XDG_CONFIG_HOME`/`XDG_STATE_HOME` and `pause_on_focus_loss = false`, on
   `DISPLAY=:1` with a Steam-like environment copied from a gamescope child's
   `/proc/<pid>/environ`. Never through Steam and never with virtual input
   devices.
2. Run 4 borderless single-view launches and 1 with `--players 2`, each with
   `--frame-log` under `timeout`. Each log must show
   `inner 3840x2160 (winit reports 23040x12960)`, no texel-limit warning, and
   only `surface=3840x2160`. Then remove the directory and the leftover one.
3. Ask the maintainer to launch from Steam with `--frame-log` in the launch
   options and pick VERTICAL. If the drop recurs, the log's `stage=` and `cut=`
   fields place it.
