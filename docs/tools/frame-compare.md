# Frame comparison: one frame of theirs, one frame of ours, same state

Replaying inputs into the emulator can never reproduce a run - the original
integrates measured frame durations, and two runs of the same script from
identical pinned starts diverge from tick 0
([the debugger page](../reverse-engineering/ppsspp-debugger.md#measured-a-pinned-pose-is-necessary-and-nowhere-near-sufficient)).
This pipeline sidesteps that entirely: instead of replaying *inputs* and hoping
the trajectories agree, it captures the original's **state** - ship pose,
camera pose, a window screenshot, all per tick - and renders our engine from
that exact state, so any difference between the two frames is a renderer or
camera-model difference and never an accumulation of physics divergence.

What it has already settled, first: **the camera fov unit is degrees**
(measured at the RMSE minimum of a sweep against the original's own frame,
bounded to about two degrees -
[camera.md](../ghidra/functions/psp-pulse/camera.md#the-fov-unit-is-degrees-measured-against-the-originals-own-frame)),
and **the camera node stores its rotation transposed**, which the one-shot
probe that discovered the node could not see
([camera.md](../ghidra/functions/psp-pulse/camera.md#the-live-pose-is-capturable-per-tick-and-the-node-stores-it-transposed)).

## The pieces

| Step | Tool | What it produces |
| --- | --- | --- |
| Capture | `psp-trace.py --camera --shot-every N --shot-dir D` | per-tick CSV with `cam_*` columns, window shots named `tick%05d.png` |
| Target | `just pads --before 50` | pad centres, push axes, and on-spline approach points |
| Teleport | `just drive place` | the craft at a chosen point, settled, pose printed |
| Render | `just frame-shot trace.csv TICK out.png` | our frame from that row's ship **and camera**, 480x272 |
| Compare | `just frame-compare theirs.png ours.png` | side-by-side and difference images |

The capture format is `oag_trace::trace::CAMERA_COLUMNS` - twelve optional
columns, all-or-nothing, negated-eye already undone at capture time - and the
transposed-store reconciliation lives in one place,
`oag_trace::replay::camera_orientation_of`. `--pose-from` in `oag-game` applies
the row's ship basis through the same `Basis::LeftUpForward` reading every
comparison uses, and the recorded camera through a `Race::view()` override that
the PVS eye and the fog eye follow automatically.

## A targeted screenshot, end to end

```sh
just pads --before 50 > /tmp/pads.csv
just drive place --pads-csv /tmp/pads.csv --pad 0 --before 50 --speed 120 --settle 30
uv run --with websocket-client scripts/psp-trace.py \
    --ticks 20 --camera --out data/traces/pad0.csv \
    --shot-every 5 --shot-window <niri window id> --shot-dir data/shots/pad0
just frame-shot data/traces/pad0.csv 0 /tmp/ours.png
just frame-compare data/shots/pad0/tick00000.png /tmp/ours.png
```

Needs a PPSSPP with its websocket debugger in a race (`just drive menu` walks
there from any screen). Captures and shots are derived game data: they live
under `data/traces/` and `data/shots/`, both gitignored, and are never
committed.

## What to trust, and how far

- **Geometry and framing: trust.** From a captured row, our render reproduces
  the original's framing to the point where an fov sweep's RMSE minimum lands
  on the authored value. A wrong camera reading is *loud* - the row-vs-column
  finding showed up as a visibly different view of the same corridor.
- **Pixels: compare, do not diff to zero.** Different renderer, different
  filtering, different anti-aliasing; the difference image is for spotting
  *structural* disagreement (missing mesh, wrong culling, fog at the wrong
  depth), not for a byte match.
- **Moving shots carry a small phase.** The window shows the last presented
  frame; measured within about two ticks of the paused row and unresolvable
  below the cross-renderer noise floor at low speed. A stationary craft
  (`place --speed 0`, generous `--settle`) removes the question entirely - use
  one for anything quantitative, the fov measurement being the worked example.
- **A `.stale.png` shot must not be cited.** It means the compositor clipboard
  could not be told apart from its previous content - usually a genuinely
  unchanged frame, occasionally the
  [one-shot clipboard lag](../reverse-engineering/ppsspp-debugger.md#screenshots-at-the-breakpoint-and-the-clipboards-one-shot-lag).

## Open ends

- The shot-vs-row phase could be pinned exactly by a fast-moving per-tick
  capture; nothing has needed it.
- The fov measurement is one ship, one view, one track section. A second team
  (different authored fov) and the internal view would turn confidence 85 into
  a closed question.
- `place` always writes the basis rows *and* the `+0xc0` transpose; whether the
  transpose alone would be rebuilt in time was never isolated.
