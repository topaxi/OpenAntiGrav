# The projection, measured against the original's own frames

**Our craft mesh is the right size. Our projection is too narrow, and the error
tracks speed.** This page records the measurement that settles both halves,
because the first half retires a blocker that three passes worked on under the
opposite assumption, and the second half turns a `_q`-grade decompiled store
into a confirmed one.

Read [`camera.md`](../ghidra/functions/psp-pulse-usa/camera.md) first for the fov
chain in the original - this page is the pixel-side evidence that lands on it.

## What was believed, and why it was wrong

From 2026-08-04 to 2026-08-09 the record said our craft renders **too large**:
the wingtip lamp centroids sat `189.3 px` apart in ours against `108.8 px` in the
original from the original's own recorded camera pose, a ratio of `1.740`, and
`1 / 0.75^2 = 1.7778` is 2.2 % away - so "the original scales the mesh twice"
became the live hypothesis. `Race::ship_model_matrix`'s doc comment carried it,
and so did four sections of `HANDOVER.md`.

**The clause that made it a craft-specific error was the one that was false.**
That measurement was recorded as being taken "with the track and buildings behind
aligning". They do not align. At the authored fov the tower, the barrier and the
horizon are all discontinuous across a checkerboard interleave of the two frames,
and the far scenery registers at **1.1195**, not `1.000`. Everything in the frame
is magnified, and the craft was only the part anyone measured.

The second failure was the instrument. The lamp-centroid metric is **not linear
in the quantity it measures** - model scales `1.0 / 0.75 / 0.5625` read
`1.740 / 1.191 / 1.067` where linear would give `1.305 / 0.979` - so it could not
tell one candidate factor from another even in principle. Four later attempts
(hue silhouette, saturation, luma profile, red-dominance clusters) each failed on
the original's side for a different reason; they are catalogued in `HANDOVER.md`
and are not repeated here.

## The instrument that works

**Similarity registration on high-passed gradient magnitude.** The craft is the
same mesh with the same texture in both frames, so its internal structure - panel
lines, intake grilles, the yellow spine, the canopy edge - is a rich non-emissive
pattern. Find the uniform screen zoom `s` about the image centre, plus
translation, that maximises masked normalised cross-correlation of our frame onto
the original's.

**There is no brightness threshold anywhere in it.** That is the whole point: the
high-pass (sigma 6) removes exactly the low-frequency bloom difference that makes
thresholds untransferable between the two renderers, and taking gradient
magnitude makes it indifferent to a patch being dark-on-light in one frame and
light-on-dark in the other. Pointed at a box containing only background, the same
routine measures whether the *background* scales - which is how "our mesh is too
big" is told apart from "our projection is too wide".

Three validations, all of which the lamp metric fails:

**1. Against known factors.** The original's own pixels resized by `magick
-distort SRT`, an independent resampler:

| true | measured | error |
| ---: | ---: | ---: |
| 0.70 | 0.6995 | -0.07 % |
| 0.85 | 0.8481 | -0.22 % |
| 0.95 | 0.9525 | +0.26 % |
| 1.00 | 1.0000 | 0.00 % |
| 1.05 | 1.0525 | +0.24 % |
| 1.15 | 1.1494 | -0.05 % |
| 1.25 | 1.2498 | -0.02 % |

Worst error **0.26 %** over a 1.8x range. Resampling softens edges, so this is a
harder test than the real one.

**2. A free control that cannot move.** The HUD is a 2D layer at a fixed pixel
size; no camera parameter can scale it. A band dominated by it (`y 470..544`)
registers at **0.997**, while the 3D scene in the same frame registers at 1.12.

> **The same property makes the HUD a contaminant, and it bit immediately.** A
> second session registering the band left of the craft (`0,200,380,430`) got
> **1.0106**, against 1.1119 for its mirror image on the right - a real global
> correlation peak, not a weak one, and it reads exactly like a scene that
> disagrees with this page. Cropping the two frames and *looking* at them
> explains it in one glance: a strip of green HUD text sits inside the bottom of
> that box, and the original's road surface there is blown out to white by a
> bloom we do not have - so the HUD, which cannot scale, was one of the few
> strong gradient features left in the band. **Excluding a 35-pixel-tall strip
> moves the answer from 1.0106 to 1.1394**, and the band's upper half alone
> gives 1.1375. All three background regions agree once the HUD is out.
>
> Two rules follow. **Exclude the HUD from every registration box** - it pulls
> the answer toward 1.000, which is the answer that looks like "no problem
> here". And **look at the crop when a band disagrees**: the anomaly was
> invisible in the correlation curve and obvious in the picture.

**3. Linearity in the quantity measured.** A fixed eye with a varying fov is an
exact screen-space zoom about the principal point, so `--camera-fov` is ground
truth with no code change. Screen offsets scale as `1/tan(fov/2)`, so
`measured * tan(fov/2)` must be constant:

| fov | craft | far scenery | craft x tan(fov/2) |
| ---: | ---: | ---: | ---: |
| 54 | 1.2634 | 1.2695 | 0.6437 |
| 57 | 1.1882 | 1.1927 | 0.6452 |
| 60 | 1.1188 | 1.1197 | 0.6460 |
| 63 | 1.0503 | 1.0554 | 0.6436 |
| 65.75 | 0.9988 | 1.0009 | 0.6451 |
| 69 | 0.9393 | 0.9417 | 0.6455 |
| 72 | 0.8857 | 0.8918 | 0.6435 |

**Constant to 0.39 %** over a 1.43x induced range.

**4. A free precision check: sweep the same band at two step sizes.** Two
sessions reported the HUD-excluded left band 1.5 % apart, which is six times the
instrument's validation figure. The cause is the sweep **step** and not the
search radius (`--search 70` and `90` agree exactly), and running the clean bands
the same way turns an annoyance into a control:

| band | step 0.01 | step 0.005 | spread |
| --- | ---: | ---: | ---: |
| sky (`0,20,960,200`) | 1.1195 | 1.1180 | 0.13 % |
| trackside right (`580,200,960,430`) | 1.1119 | 1.1116 | 0.03 % |
| left, HUD excluded (`0,200,380,430`) | 1.1394 | 1.1567 | **1.5 %** |

**Step sensitivity measures peak sharpness**, so it says which bands are
measurable. The clean bands are step-insensitive to ~0.1 %; the left band stays
unreliable even with the HUD removed, because the original's road there is still
blown out and there is little structure left to correlate. So the left band's
right answer is "1.11-1.16, and this band should not be quoted to three digits" -
which is enough to kill the 1.0106 reading and not enough to refine anything.

**Run every band at two steps and treat disagreement beyond 0.26 % as the band
declaring itself unmeasurable.** It costs one extra invocation and it is the
check that would have caught this before two sessions compared third digits.

Note `--render-scale` is the wrong knob and was checked as such: it is internal
resolution and does not change on-screen size at all.

## The measurement: the craft is not the error

Registering **far scenery** (`y 20..200` - buildings, tower, sky, at effectively
infinite distance, which **no mesh scale can move**) against registering the
**craft**, over `data/traces/pad0-boost.csv` and `data/shots/pad0-boost/`:

| tick | speed | far scenery | craft | craft / far |
| ---: | ---: | ---: | ---: | ---: |
| 0 | 71.84 | 1.1195 | 1.1186 | 0.999 |
| 10 | 79.48 | 1.1251 | 1.1173 | 0.993 |
| 20 | 87.39 | 1.1388 | 1.1352 | 0.997 |
| 30 | 93.88 | 1.1504 | 1.1421 | 0.993 |
| 40 | 133.25 | 1.2092 | 1.1919 | 0.986 |
| 50 | 150.81 | 1.2508 | 1.2328 | 0.986 |
| 60 | 149.94 | 1.2550 | 1.2424 | 0.990 |
| 70 | 145.18 | 1.2485 | 1.2346 | 0.989 |
| 80 | 141.54 | 1.2364 | 1.2226 | 0.989 |
| 90 | 139.50 | 1.2329 | 1.2189 | 0.989 |
| 100 | 137.99 | 1.2329 | 1.2153 | 0.986 |
| 110 | 94.64 | 1.1625 | 1.1446 | 0.985 |
| 120 | 65.26 | 1.1156 | 1.1072 | 0.992 |
| 130 | 53.17 | 1.0652 | 1.0806 | 1.014 |
| 140 | 42.76 | 1.0722 | 1.0647 | 0.993 |
| 148 | 40.95 | 1.0703 | 1.0569 | 0.987 |

**`craft / far` = 0.992 +/- 0.007 (sd) across 16 frames spanning a 1.17x range of
absolute ratio.** Our craft is the same size as our scene; it inherits the zoom
like everything else.

*The table is the original pass's run.* Re-taken independently on the committed
script, tick 0's far scenery reproduces exactly (**1.1195**) and the craft box
quoted in "Reproducing" below reads **1.1211** rather than the table's 1.1186,
for `craft / far` = **1.0014** instead of 0.999 - the craft column was measured
on a different box. Both readings say the same thing, which is the point: the
conclusion is that this ratio is 1, and it is 1 either way.

Confirmed directly rather than only by ratio: rendered at the fov that zeroes the
background for tick 0 (`--camera-fov 65.75`), three **craft-only** boxes
containing no background at all register at **0.9989 / 0.9985 / 0.9987**. The
craft mesh is correct to **0.15 %**.

So `1 / 0.75^2 = 1.7778` is **refuted**, and the decision recorded in
`ship_model_matrix` - ship the confirmed `0.75`, leave the remainder visible
rather than cancelling it - was the right call for the wrong reason. The
remainder was never in the mesh.

## The projection error is the original's speed-proportional fov term

Converting each frame's far-scenery ratio to the fov that would zero it
(`tan(fov/2) = ratio * tan(30 deg)`) gives a fov that rises monotonically with
speed across the whole capture. `camera.md` records a store, read at instruction
level at the top of `FUN_088455ec`:

```c
craft[0x790] = dot(fwd, *(shipNode + 0x140)) * 0.075 + craft[0x7c];
```

with `0.075` a constant at `0x08a7b6a0`, and `craft+0x790` added to **both**
tripod fovs every frame in `Ship_UpdateCameraRigs`' tail. That is
`authored + 0.075 * forward_speed + craft[0x7c]`, in **additive degrees**.

Fitting exactly that form against the pixels - `dot(fwd, vel)` computed from the
capture's own `fwd_*` and `vel_*` columns, not the `speed` magnitude column:

```
fov_deg = 60.181 + 0.07685 * dot(fwd, vel)     residual rms 0.407 deg, max 0.888 deg
```

| quantity | recovered from the executable | fitted from 16 frames of pixels | 95 % interval |
| --- | ---: | ---: | --- |
| coefficient | **0.075** | 0.07685 | [0.0710, 0.0827] |
| intercept | **60** (`<ExternalCameraFar fov>`, this ship) | 60.181 | [59.54, 60.82] |

**Both recovered constants sit inside the pixel fit's interval**, from two
completely independent routes - a disassembler and a photometric registration of
16 emulator frames. That is the confirmation the store did not have.

Three checks that could have broken it:

- **`speed_ramp` is excluded as the driver**: fitting against it gives an
  intercept of **62.0 deg**, which misses the authored value.

**The pixels mildly argue *against* pure forward velocity, and this page has to
say so.** `dot(fwd, vel)` and the `speed` column agree to three decimal places at
**13 of these 16 ticks** - the craft goes where it is pointing for most of the
capture. They separate only at ticks 110, 120 and 130 (86.76/94.64, 61.35/65.26,
50.47/53.17), which are also the three worst residuals. Fitting the `speed`
column instead is marginally better (rms 0.344 vs 0.407 deg, intercept 59.96).

Nesting both in one fit is the discriminator, since
`speed = dot(fwd, vel) + off_axis`. If the driver is forward velocity the
off-axis coefficient is 0; if it is speed magnitude, the off-axis coefficient
equals the forward one:

```
fov = 60.0 + 0.07879 * dot(fwd, vel) + 0.11657 * off_axis
                +/- 0.00247            +/- 0.04694   (t = +2.48)
off-axis 95 % CI [0.0152, 0.2180]  - excludes 0, contains 0.07879
```

So the pixels **reject a pure forward-velocity driver at about 95 %** and are
consistent with speed magnitude. Three things stop that from being a conclusion:

- It rests on **3 of 16 ticks**. Everywhere else the two columns are the same
  number and carry no information about which is the driver.
- The point estimate is **1.5x the forward coefficient**, where speed magnitude
  predicts exactly 1.0x - so it does not cleanly match that model either.
- Tick 130 keeps a **-0.9 deg** residual under *both* models, so neither is the
  whole story. Otherwise there is no residual structure: 7 sign runs over 16
  points against ~9 expected by chance.

**And the disassembly is not ambiguous**: the store reads a dot product with the
craft's forward axis. A page that overrode an instruction-level read on the
strength of three ticks would be repeating this thread's original mistake in the
opposite direction.

### Resolved: the field is the plain body velocity, and the three ticks are the instrument

**A smoothed velocity was the proposed reconciliation. It is refuted.**
`FUN_088455ec` reads `iVar12 = *(craft + 0x794)` and dots the craft's forward
row with `iVar12 + 0x140`. The rigid body's own field map - measured at runtime
and recorded in `scripts/psp_trace_fields.py` - puts `right`/`up`/`fwd`/`pos` at
`+0x00`/`+0x10`/`+0x20`/`+0x30` and **`vel` at `+0x140`**, and the same function
reads `+0x10` off that pointer as the up axis for the camera's height offset.
So `shipNode` *is* the rigid body, `+0x140` *is* its instantaneous velocity, and
the store computes exactly `dot(fwd, vel)` - the same quantity the CSV records
and the fit already used. **There is no lag anywhere in it.**

**Which leaves the three ticks, and they turn out to be the instrument.** The
disagreement lives entirely at ticks 110, 120 and 130. Those are the three ticks
where the craft is yawing hardest, by a wide margin:

| tick | yaw rate (rad/s) | slip angle | residual |
| ---: | ---: | ---: | ---: |
| every other tick | 0.007 - 0.036 | 0.1 - 5.5 deg | within +/-0.6 |
| 110 | **0.803** | 16.9 deg | +0.89 |
| 120 | **0.605** | 12.0 deg | +0.67 |
| 130 | **0.379** | 9.1 deg | -0.88 |

`|residual|` correlates with yaw rate at **+0.781** and with slip angle at
+0.702. That is not a coincidence and it has a mechanism: **this metric fits a
similarity transform - uniform zoom plus translation - and has no rotation term
at all.** When the craft yaws hard, the two frames differ by an in-frame
rotation and a parallax sweep that the model cannot represent, so the error goes
somewhere, and where it goes is the zoom estimate.

**And the confound is exact.** The craft slips *because* it is yawing, so
"off-axis" and "the registration is degraded here" are not merely correlated -
they are the same three frames. The discriminator's off-axis regressor therefore
cannot separate a genuine speed-magnitude driver from a registration artefact,
and the `t = +2.48` carries no weight against an instruction-level read.

**So the driver is `dot(fwd, vel)`, at confidence 85.** The pixel objection is
withdrawn - explained, not overruled. What would have made it survive is a
capture with hard yaw *and* an independent check on the registration at those
ticks; the honest statement is that this instrument does not measure a
hard-turning frame.

*Method note, since it is the general point.* The question that resolved this is
the second of the pair on
[methodology.md](../reverse-engineering/methodology.md#rules-learned-the-expensive-way):
not "is my result significant" but "**what reading would let the harder evidence
be right anyway?**" - and the answer came from one decompile and one column of
the capture nobody had looked at.
- **Boost contributes nothing extra.** Adding a `boost_timer` term gives a
  coefficient of **-0.0087** and barely moves the rms. Ticks 40-70 (boost active)
  sit on the same line as ticks 80-148 (boost expired). The widen is present with
  no boost at all, which is why `BoostFovKick` never accounted for it.
- **Robust to the frame/trace lag.** `data/shots/pad0-boost/measurements.md`
  records a +0..+2 tick ambiguity. Refitting at each lag moves the coefficient
  over `0.0769 / 0.0759 / 0.0752` and the intercept over `60.18 / 60.29 / 60.47`
  - every one of them still bracketing `0.075` and `60`. The lag does not decide
  the constants.
- **It is a fov error and not an aspect error.** A 2D search over independent
  `(sx, sy)` peaks at `sy/sx = 1.000` (craft) and `0.989` (far scenery), within
  one grid step of isotropic. `AUTHORED_ASPECT = 480/272` is exactly the 960x544
  capture aspect, so `fit_vertical_fov` is the identity here and is not
  implicated.

**A note that used to claim more than it should.** Tick 60 rendered at
`--camera-fov 71.85` gives far scenery **0.9995** and craft-only boxes
0.9891 / 0.9899, and this was written up as out-of-sample validation of the fit.
**It is not.** The recovered law puts tick 60 at `71.246` and the pixel fit at
`71.704`; `71.85` is above both, so that render was made at a value neither
model produced - it was swept until the background zeroed. What it actually
measures is the residual below.

### A residual of 0.4-1.1 % survives the correction, and it is not the fov

Applying the **recovered** fov leaves a small, consistent over-scale:

| tick | forward speed | recovered fov | far scenery after correction |
| ---: | ---: | ---: | ---: |
| 148 | 40.9 | 63.064 | 1.0051 |
| 0 | 71.5 | 65.364 | 1.0043 |
| 62 | 148.9 | 71.166 | 1.0093 |
| 60 | 149.9 | 71.246 | ~1.010 (as the 71.85 sweep above implies) |

**Do not sweep the fov to absorb this.** The fov is now known exactly from the
executable - 0.0007 degrees over 19 live samples - so a swept value that zeroes
the background is fitting the *camera* to cancel an error that is somewhere
else, which is precisely the move this whole page exists to warn about. Passing
the recovered fov is correct even where a swept one registers better.

What the residual is not: it is not the camera position (a `--pose-from` render
installs the capture's own eye and orientation), and it is not the frame-to-trace
lag (at tick 62 the fov moves 0.07 degrees over two ticks, where 0.5 would be
needed). It is roughly twice the instrument's 0.26 % validation figure at slow
ticks and four times it at fast ones, and it is **not** monotonic in speed, so
"a second speed term" does not fit it either. Unexplained, small, and recorded
rather than absorbed.

For comparison, the error this page's main finding removed was **12-26 %**. A
1 % residual does not threaten any conclusion here; it is worth chasing only
because it is the next thing in the way of a genuinely exact matched pose.

### What this settles that reading alone could not

`camera.md`'s correction section rates the **decomposition** of the additive term
into `0.075 * speed` and `craft[0x7c]` at confidence **0**, because a live series
of `+9.4` at 130 units/s, `+6` at 120, `+3` at 100 and `+0.5` at 75 does not fit a
line. The pixels do fit a line, over 148 ticks and a 41-151 units/s range, with an
intercept indistinguishable from the authored fov. So:

- **`craft+0x7c` is 0 to within +/-0.64 degrees throughout a clean run.** The
  additive fov term is the speed-proportional store and nothing else.
- **That live series was therefore not this term.** It is the `Hud_Update` shake
  path decaying after the craft was placed - which is what "a decaying
  oscillation" described, and it decays faster than speed does because it is not
  a function of speed at all. The two measurements were never of the same thing.
- **`FUN_088455ec` runs while an external view is selected**, which `camera.md`
  names as the obvious next check and leaves open. Our capture is the external
  chase view, and the term is plainly present in it. This is inference from
  effect rather than from execution, so it is worth the cheap direct
  confirmation: an exec breakpoint on `0x088455ec` with an external view
  selected.

### Confidence

| Claim | Score | On what |
| --- | ---: | --- |
| Our craft mesh is the correct size (0.15 %) | 85 | An instrument validated to 0.26 % against known factors, a control that cannot scale reading 0.997, and craft-only boxes at 0.999 with the background zeroed. Held below 90 for sampling, not for the instrument: one capture, one ship, one circuit |
| The original's fov carries a speed-proportional additive term, coefficient `0.075` degrees | 88 | A store read at instruction level, whose two constants are independently recovered from 16 frames of the original's own pixels and both fall inside the fit's 95 % interval |
| That term's driver is `dot(fwd, vel)` specifically | 85 | `shipNode` is the rigid body and `+0x140` is its velocity, matched against the body field map measured at runtime; the same function reads `+0x10` off the same pointer as the up axis. The pixel objection is explained as a confound - it lives only in the three hardest-yawing ticks, where a rotation-free registration model degrades - rather than overruled |
| `craft+0x7c` is ~0 during a clean run | 75 | One capture, one circuit, one ship. The residual is 0.41 deg rms against a term that would have to be several degrees to matter |
| `1 / 0.75^2` scales the drawn mesh | **refuted** | Far scenery, which no mesh scale can touch, carries the same ratio as the craft in all 16 frames |

## Settled on the running game: the law is exact

**2026-08-09, and it ends the thread.** `g_camera_fov_degrees` (`0x08b34310`) is
the live projection's own fov with a single writer, so reading it is the
quantity in dispute with no pixels and no instrument in between. Breaking at
`Ship_UpdateCraft` and reading the global alongside the body's own `fwd` and
`vel` at `+0x20` and `+0x140`:

```
fov_degrees(frame N) = 60 + 0.075 * dot(fwd, vel) evaluated at frame N-1
worst residual over 19 consecutive samples: 0.0007 degrees
```

That is float32 precision. Not a fit - the **law reproduces exactly**, over a
sweep from 122 to 146 units/s taken while the craft accelerated and then
settled. Three things follow immediately:

- **`craft+0x7c` is exactly `0`**, not merely small. The intercept is the
  authored `60` to three decimals at every sample.
- **The one-frame offset is a publish order, not a lag in the law.**
  `Camera_PublishTripod` runs later in the frame than `Ship_UpdateCraft`, so the
  global read at that breakpoint is the previous frame's. Anything sampling the
  fov and the body together must account for it.
- **The driver is `dot(fwd, vel)` and nothing else, settled by a case the pixels
  could never reach.** During a placement transient one sample had forward
  velocity of **-76.5** units/s while `|vel|` was 69.4, and the fov read
  **54.26** - *below* the authored 60. A speed-magnitude driver cannot produce a
  fov below the authored value at any speed; a dot product can, and predicts
  exactly that number. The off-axis question is closed by construction.

**The pixel measurement was right to 0.6 %.** At tick 0 registration said the
fov that zeroes the background is `65.75`; the executable says
`60 + 0.075 * 71.52 = 65.364`. That is the whole method validated end to end
against the machine it was inferring about.

**Confidence 94** for the law, its coefficient and its driver - a runtime trace
of the exact quantity, which the [rubric](../reverse-engineering/confidence-rubric.md)
caps at 94 until a second binary agrees. The confidence table below is superseded
by this section wherever they differ.

### What this retires

- **The `craft+0x7c` decomposition question**: answered, it is zero on a clean
  run.
- **`FUN_088455ec` runs in an external view**: no longer an inference. The
  capture above is the external chase view and the term is plainly live in it.
- **The off-axis / speed-magnitude objection**: moot. Kept below because the
  *reason* it was wrong is worth more than the result.
- **`exhaust.md`'s 136-edge world fit**: definitively the broken instrument, not
  merely the less-validated one.
- **The B2 tower's "90-95 degrees"**: refuted. The fov's actual ceiling on this
  circuit is ~71 degrees at 146 units/s, and it is reached by speed rather than
  by a boost.

## Two things in this repository still contradicted it, and the live read settled both

Recorded rather than resolved, because a page that only lists its supporting
evidence is the failure this whole thread was about.

**1. The 136-edge world fit says the background is the right size.**
[exhaust.md](../ghidra/functions/psp-pulse-usa/exhaust.md) fits the horizontal
displacement of 136 matched world edges against distance from the screen centre
on a matched-pose frame and gets `offset = +0.00405 * (x - 480) + 1.37`, RMS
residual 9.9 px, "implied world scale ours/theirs = 1.0041" - and concludes the
track, the camera and our projection are all right.

**That is numerically incompatible with a 1.1195 far-scenery zoom**, which is a
slope of ~0.12 - thirty times the fitted one - and would put the frame edges
+/-57 px out. A 9.9 px residual cannot hide that, so one of the two instruments
is broken.

**The registration side has since been reproduced by a second session working
independently**, on the committed script: sky `1.1180` against this page's
`1.1195`, and trackside right of the craft `1.1119`. The band left of the craft
needed the HUD excluded (see above), after which it reads `1.1394`. Three
background regions, two sessions, one instrument with three validations - against
one fit with none. The refutation is now written into `exhaust.md` beside the fit
itself.

The likeliest mechanism, and it is testable: **nearest-neighbour edge pairing
absorbs a uniform zoom.** Under a 12 % zoom the correct partner for an edge near
the frame edge is ~57 px away, further than the spacing between adjacent rails
and barrier posts - so the pairing step silently matches each of our edges to a
*different* original edge than it should, and a fit over wrong pairs collapses
toward zero slope with a residual that looks like noise. The registration metric
has no pairing step at all, which is why it is not exposed to this, and it
carries three validations the edge fit has none of. That is the reason to back
registration - not that it is newer.

**2. The B2 tower reads 1.4-1.5x, not 1.17x.** exhaust.md separately measures the
tower's width on **the original's own frames** at different boost ages and finds
it ~1.4-1.5x smaller at boost age 0.3-0.7 s than at 1.3 s+, an fov sweep landing
the boosted frame at "roughly 90-95 degrees against the authored 60". That is
original-versus-original, so it involves no cross-renderer registration at all -
and it is **independent corroboration that the original's fov moves with speed**,
which is worth more than its disagreement costs.

But it does not reconcile in magnitude. This page's law over the same speed range
(150 down to ~40 units/s) predicts a tangent ratio of
`tan(35.65) / tan(31.5) = 1.171`, not 1.4-1.5, and 90-95 degrees is far above
`60 + 0.075 * 150 = 71.3`. Same sign, same driver, wrong size. Either the tower
measurement conflates the zoom with the boost's own motion blur and the craft's
changing distance to it, or the law is missing a term that only shows at boost
speeds. Unresolved.

### The one read that settles both, and it costs a single breakpoint

`g_camera_fov_degrees` (`0x08b34310`) is the live projection's own fov, and
`Camera_PublishTripod` is its only writer. **Read it at the exact tick
`data/shots/pad0-boost/tick00000.png` was captured** - speed 71.84 units/s:

- **~65.4** (`60 + 0.075 * 71.5`) confirms this page directly, from the
  executable's own state rather than from pixels, and disposes of the edge fit.
- **~60** means the original's fov was authored-flat at that moment, this page's
  *interpretation* is wrong even if the registration is sound, and the zoom has
  some other cause.

Nothing else in the thread has this property: it reads the quantity in dispute,
in the frame in dispute, with no instrument in between. Do it before spending
effort on either contradiction above.

## What follows from this

**1. Do not apply any further factor to `ship_model_matrix`.** The mesh is right.

**2. `Race::projection` implements the term, as of 2026-08-09.**
`SPEED_FOV_GAIN_DEG` in `crates/raceplay/src/lib.rs`, added to the authored degrees
before the player's fov setting, which is where `Ship_UpdateCameraRigs` adds it.
It composes with `BoostFovKick` - still this project's own invention - and does
not replace it: this one adds degrees, that one multiplies a tangent.

Two tests pin it. One asserts the field widens with forward speed at 40, 100 and
150 units/s and **narrows below the authored value when the craft moves
backwards**, which is the case that distinguishes the driver from speed
magnitude. The other was rewritten: it used to compare the boost kick against a
tick-0 reference, which this term breaks for a reason unrelated to the kick, and
now compares two races at the same tick so the kick is isolated from the speed
widen.

**A matched-pose comparison still needs `--camera-fov`**, because `place_at`
resets the body and a posed craft therefore has zero velocity - the term
contributes exactly `0` in that path, by construction.

**3. Every matched-pose pixel comparison taken before this is suspect**, and the
reason is worse than a constant offset: the frames were misregistered by
1.12-1.26x depending on the tick's *speed*, so it is not a factor that divides
out. Re-take them with `--camera-fov` computed from the capture's own forward
velocity, or at a low-speed tick where the error is smallest. Named instances are
tracked on `HANDOVER.md`.

## Reproducing

The metric is [`scripts/frame-register.py`](../../scripts/frame-register.py)
(numpy and pillow only - scipy does not install here). It was written as a
scratch script under `data/`, which is gitignored and therefore not durable; it
is committed because it is the instrument for **every** future comparison
against the original, not a note about this one.

```sh
cargo build --release -p oag-game
# from the repo root, so data/ resolves:
target/release/oag-game --race --pose-from data/traces/pad0-boost.csv \
    --pose-tick 0 --ticks 0 --screenshot ours_t0.png --size 960x544

# the free control, which is not optional - far scenery no mesh scale can move:
python3 scripts/frame-register.py data/shots/pad0-boost/tick00000.png \
    ours_t0.png --box 0,20,960,200 --range 0.70,1.40,0.01 --search 70
#   => 1.1195

# the craft, for comparison against it:
python3 scripts/frame-register.py data/shots/pad0-boost/tick00000.png \
    ours_t0.png --box 440,330,525,430 --range 0.70,1.40,0.01 --search 70
```

**The script refuses a peak that lands on an end of the sweep**, rather than
returning it. That is not a nicety: an earlier sweep capped at 1.25 returned
exactly `1.2500` for the three fastest ticks of this capture and those numbers
were written up as measurements. `--allow-clamped` exists for deliberate
one-sided probes and should be rare.

**The `--range` floor has the identical failure mode and bites the command
above.** The script sweeps the zoom applied to MOV, which is the *reciprocal* of
the reported ratio, so an uncorrected fast tick at ratio `1.2550` needs
`1/1.2550 = 0.797` - just under the `0.80` floor. Both sessions re-taking this
hit the refusal on their first run. **Use `--range 0.70,1.40` for an uncorrected
frame** and narrow it only once the answer is bracketed. The ceiling trap is
famous here because it produced fake numbers; the floor is the same trap and
only looks different because the guard catches it.

The reference frames are `data/shots/pad0-boost/`, and `data/` is gitignored - so
`rg` and `fd` return nothing there with exit code 0 whether it is full or empty.
Use `/bin/ls`, `find`, `grep` or an absolute path.
