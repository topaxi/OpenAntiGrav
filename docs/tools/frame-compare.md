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
[camera.md](../ghidra/functions/psp-pulse-usa/camera.md#the-fov-unit-is-degrees-measured-against-the-originals-own-frame)),
and **the camera node stores its rotation transposed**, which the one-shot
probe that discovered the node could not see
([camera.md](../ghidra/functions/psp-pulse-usa/camera.md#the-live-pose-is-capturable-per-tick-and-the-node-stores-it-transposed)).

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
    --shot-every 5 --shot-dir <shot-dir>
just frame-shot data/traces/pad0.csv 0 /tmp/ours.png
just frame-compare <shot-dir>/tick00000.png /tmp/ours.png
```

Needs a PPSSPP with its websocket debugger in a race (`just drive menu` walks
there from any screen). Captures and shots are derived game data: they live
under `data/traces/` and `data/shots/`, both gitignored, and are never
committed.

**`place` and the capture that reads its speed must be one shell invocation.**
Nothing keeps the CPU paused between `psp-drive.py place` exiting and the next
command connecting - it free-runs with no throttle held in between, and this
game decelerates hard with no thrust (150 -> 104 units/s over the 30-tick
settle alone). A gap of a few seconds of wall-clock tool-call overhead between
the two commands is enough to coast a `--speed 150` placement down to a
standstill (measured: 150 -> 0.017 units/s) before the capture ever starts,
which reads exactly like a low-speed capture rather than a botched one. Chain
`place` and `psp-trace.py`/`just trace` with `&&` in one call.

**A raw Xvfb shot is the virtual screen's own resolution, not the PSP's.**
`import -window root` under `OAG_SHOT_DISPLAY` (see
[ppsspp-debugger.md](../reverse-engineering/ppsspp-debugger.md#running-without-a-real-display-xvfb-works-no-compositor-needed))
grabs whatever size the `Xvfb` invocation declared - 1280x720 in the
documented recipe - not 480x272. `just frame-compare` already resizes
`theirs` before comparing (`magick -resize '480x272!'`); any ad-hoc pixel
comparison against a raw shot must do the same or it is comparing two
different coordinate spaces and every region will look wrong for reasons that
have nothing to do with the game.

## What to trust, and how far

- **Geometry and framing: trust at rest, and only at rest.** From a captured
  row, our render reproduces the original's framing to the point where an fov
  sweep's RMSE minimum lands on the authored value. A wrong camera reading is
  *loud* - the row-vs-column finding showed up as a visibly different view of
  the same corridor.

  **But that measurement was taken on a stationary craft, and it had to be**
  (see the moving-shot bullet below, which recommends exactly that). The
  original adds `0.075 * dot(fwd, vel)` **degrees** to its fov every frame, so
  at `speed = 0` the term is zero and the authored value is the whole answer -
  which is why the calibration landed cleanly *and* why the speed term went
  unnoticed for months, until a comparison at 150 units/s put the whole frame
  1.26x out. See
  [projection-vs-the-original.md](../rendering/projection-vs-the-original.md).

  **So: any comparison at speed is misregistered until we implement the term.**
  Pass `--camera-fov` computed from the capture's own forward velocity, or
  compare at a low-speed tick. The general form of the lesson is worth more than
  the fix: *a calibration performed at one point of a parameter's range confirms
  the value there and says nothing about the slope.*
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

- **A fast-moving per-tick capture was tried (2026-08-25) and did not pin the
  shot-vs-row phase to one number - it narrowed the question instead of
  closing it.** Setup: `place --speed 150` on a pad approach (settles to
  ~100-120 units/s) followed in the same invocation by
  `psp-trace.py --camera --shot-every 1`, under Xvfb (no compositor
  available). **The instrument itself is sound**: a render-vs-render control
  - candidate rows scored against row 10's own render as the reference,
  instead of against a shot - peaks cleanly and correctly at the self-match
  (NCC 1.000) and falls off smoothly either side (0.529, 0.583, **1.000**,
  0.710, 0.630 for rows 8-12), confirming `frame-register.py`'s masked
  gradient-NCC resolves one tick of camera motion at this speed. **Scored
  against the actual emulator shots, only one of three sampled ticks gave a
  clean, high-margin peak**: tick 10 picks row 9 over every neighbour by
  0.10+ NCC (phase -1, i.e. the shot reflects the state one tick before the
  breakpoint pause). The other two (ticks 15, 18) gave weak, non-monotonic
  peaks whose registration shift moved with the search window's width -
  exactly what a scene with periodic structure (this capture sat in a tunnel
  with repeating wall panels) looks like when the search is absorbing the
  inter-tick motion instead of measuring it, not what a real signal looks
  like. **Suspect the screenshot path itself, not a non-constant phase**:
  this session had no compositor, so it captured through Xvfb's
  `import -window root` (`OAG_SHOT_DISPLAY`), never through the niri
  clipboard path the existing ~2-tick bound was originally measured with -
  the two have never been shown to share a lag, and an inconsistent one would
  produce exactly this pattern (one clean reading, two noisy ones, same
  capture, same speed, same box).
  **Next**: repeat under a real or niri-emulated compositor to see whether
  the inconsistency goes with it, or drive a yawing section instead of a
  straight corridor so the phase reads off `dx` as a linear ramp across
  candidate rows rather than off a peak height - a ramp does not need the
  shift search tightened against a periodic scene to stay honest.
- The fov measurement is one ship, one view, one track section. A second team
  (different authored fov) and the internal view would turn confidence 85 into
  a closed question.
- `place` always writes the basis rows *and* the `+0xc0` transpose; whether the
  transpose alone would be rebuilt in time was never isolated.
