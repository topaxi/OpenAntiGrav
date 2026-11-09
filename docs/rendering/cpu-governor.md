# The CPU governor

`oag_present::governor` is a closed loop beside dynamic resolution ([`dynamic-resolution.md`](dynamic-resolution.md)).
DRS sheds pixels when the GPU is the slow side. The governor acts when the frame thread's CPU time
(the `CPU` row: frame time minus swapchain waits) is over the frame limiter's period while the
smoothed GPU time is under 0.9 of it, and it steps down an ordered list of this project's own
extras. It restores them one at a time, with hysteresis, when the CPU plus that knob's measured
saving fits in 0.8 of the budget for 6 s. A step whose measured saving was under 0.2 ms is undone and,
on the second weak reading, skipped for that title, track and mode until the game quits (see "A step that saved nothing is undone"). Thresholds are chosen, not measured. It holds when the GPU
is the bound side, when there is no GPU reading, and when nothing paces the frame (vsync `on`, or an
unlimited loop).

- **Runtime only.** `Session::governed_profile` hands the draw a copy; `render_profiles` is never
  written. Captures and `--screenshot` run headless and never reach it.
- **Knobs** (`session/cpu_governor.rs`): `shadows mapped` down to the title's authored tier
  (`original`, or `off` where the title authors none), then `model_detail` `maximum` to `high` to
  `original`. Nothing goes below what a title authors, so a default profile has no steps.
- **Setting:** `graphics.cpu_governor` (CPU GOVERNOR on the advanced graphics page), **on by default** (maintainer decision 2026-10-27, from the unlocked Deck numbers below).
- **Log:** one `cpu governor:` info line per change (what, CPU and GPU ms, budget); `--frame-log`
  lines end in `governor=N`.

## Deck numbers (2026-10-27, HD, EU PS3 image, Anulpha Pass, `--render-scale 50`, `--no-intro`)

Native under the Deck's gamescope, release `dist` znver2, load 0.5 to 1.5. Ms per frame from the
`frame-phases` total minus `present`; two runs each, 25 s, first 4 lines dropped. Identical runs
differ by up to 0.4 ms.

| Knob (against base) | CPU change |
| --- | --- |
| `shadows mapped` (base `original`) | +0.1 to +0.2 |
| `lod maximum`, `high`, `texture_detail maximum` | none beyond noise |
| `reconstruction off` | none |
| `motion blur off` (not used: HD authors it) | -0.2 |
| `shadows off` (not allowed: authored tier) | -1.4 |

Pulse default circuit: `mapped`, `lod maximum`, a PSP screen filter, `motion blur low`, `msaa 4x`
all within noise (3.5 to 3.5 ms). Our extras are cheap on the CPU; the lever that is not ours, the
authored shadow passes, is the big one and the governor must not touch it.

Governor on against off, extras on (`shadows mapped`, `model_detail maximum`), `vsync off`,
`frame_limit 165` (budget 6.06 ms), 40 s, three interleaved rounds:

| | total | submit | p99 | steps |
| --- | --- | --- | --- | --- |
| off | 7.74, 7.71, 7.76 | 3.12 | 11.2 | - |
| on | 7.57, 7.49, 7.49 | 3.04 | 11.2 to 11.45 | 3 in about 16 s |

That capped run is the first measurement; it cannot show a win (see the next section).
A race at its target (vsync `smooth`, limit 90, extras on) logged `governor=0`
throughout and no change line.

Frame counts in the controller are in budget periods, so at a presentation rate below the budget
the waits last longer in wall time (about 1.8 s per shed streak here).

## Unlocked Deck numbers (2026-10-27, `governor-unlocked`): the verdict is on

The capped run above could not show a win: gamescope holds presentation at 90 Hz whatever the CPU
does. This run removes the cap. Same Deck, same release `dist` znver2 build (`oag_present::governor`
at `dcf5025ff`), the maintainer's settings copy with `vsync off`, extras on (`shadows mapped`,
`model_detail maximum`), `--render-scale 50 --no-intro --autopilot --seed 1`, 60 s per run, the first
4 frame-log lines dropped, off and on interleaved, three rounds, load 0.7 to 2.1.

**Getting unlocked under gamescope.** With `ENABLE_GAMESCOPE_WSI=1` (the gaming-mode path), `vsync off`
and `frame_limit unlimited` still held 90.0 fps (a 20 s probe). `ENABLE_GAMESCOPE_WSI=0` (the X11 path
under the same gamescope) ran 146 fps with the same settings. All unlocked rows use WSI=0; the capped
check at the end uses WSI=1, the player's path. (Since the refresh clamp that run reads a 60 Hz display on the
Deck and the governor's budget is 16.7 ms; "A step that saved nothing is undone" below has the nested-gamescope
recipe that restores the 165 Hz budget.)

**The budget needs a finite limit.** With `frame_limit unlimited` the governor is inert by design (nothing
paces the frame), so the runs use `frame_limit 165` (budget 6.06 ms) for HD and 500 (2.0 ms) for Pulse.
Neither is reached: HD runs about 155 fps, Pulse about 300 to 330.

HD Anulpha Pass (CPU row is `cpu_ms - present_ms` from the frame-log line; phases is `frame-phases` total
minus present):

| Run | fps | frame_ms | p99 | CPU row | phases | gpu_ms | governor lines |
| --- | --- | --- | --- | --- | --- | --- | --- |
| off 1 | 152.0 | 6.60 | 8.56 | 6.10 | 5.98 | 2.14 | 0 |
| off 2 | 152.7 | 6.57 | 8.50 | 6.09 | 5.96 | 2.13 | 0 |
| off 3 | 152.9 | 6.57 | 8.50 | 6.07 | 5.94 | 2.13 | 0 |
| on 1 | 158.1 | 6.34 | 8.11 | 5.85 | 5.73 | 2.17 | 3 |
| on 2 | 157.4 | 6.37 | 8.18 | 5.87 | 5.75 | 2.18 | 3 |
| on 3 | 158.8 | 6.31 | 8.07 | 5.81 | 5.68 | 2.16 | 3 |

Off to on: fps 152.5 to 158.1 (+3.7 %), mean p99 8.52 to 8.12 ms (-0.4 ms), CPU row 6.09 to 5.84 (-0.25 ms),
submit 3.09 to 2.98. The spread inside a group is under 1 fps and 0.06 ms of p99, so the gap is well beyond
noise. The worst single-run p99 did not improve (on 2 reached 10.27 ms, above every off run), so the claim is
the mean p99. The governor stepped three times in about 7 s, in the same order each run: `shadows original`
(CPU 7.2 ms), `model detail high` (6.6), `model detail original` (6.8 to 6.9).

Pulse (EU PSP image, default circuit, extras `--shadows mapped --lod maximum`):

| Limit | | fps | frame_ms | p99 | CPU row | gpu_ms | governor lines |
| --- | --- | --- | --- | --- | --- | --- |
| 300 (3.33 ms budget) | off | 292.8, 292.1, 293.6 | 3.42 | 4.25 | 2.98 | 0.74 | 0 |
| 300 | on | 292.4, 292.9, 293.4 | 3.42 | 4.25 | 2.96 | 0.74 | 0 |
| 500 (2.0 ms budget) | off | 298.9, 298.3, 301.8 | 3.35 | 4.58 | 3.00 | 0.74 | 0 |
| 500 | on | 328.0, 330.4, 328.4 | 3.05 | 4.26 | 2.73 | 0.64 | 3 |

At limit 300 Pulse's CPU (3.0 ms) fits the budget and the governor holds: the same numbers either way.
At limit 500 it steps the same three knobs and fps goes 299.7 to 328.9 (+10 %), CPU row -0.27 ms, p99 -0.3 ms
(the one 16 ms outlier in off 3 is a single frame).

**Capped 90 Hz (WSI=1, vsync `smooth`, limit 90, HD Anulpha, extras on, two runs each):** off and on both
90.0 fps, p99 11.19 to 11.24 ms, CPU row 6.8 against 6.7, and `governor=0` with no change line in either on run.
Nothing was shed, so nothing visible was lost where the race holds its target.

**Verdict: default on (maintainer decision 2026-10-27, unlocked Deck numbers).** fps and mean p99 improve beyond
run-to-run noise when the frame rate is unlocked (HD +3.7 %, Pulse +10 % where the CPU is over budget), and in a
capped race that holds its target the governor never acts. A settings file with no `cpu_governor` key reads as on. The earlier "0.25 ms is not worth trading tiers" reading was a capped
measurement; the same 0.25 ms is 3.7 % of frame time once the cap is gone. The cost is real: when it acts it
trades the player's own shadow map and model detail tiers for a few percent, which is why a profile without
extras, and a race at its target, are untouched.

## A step that saved nothing is undone (2026-11-05, `governor-check`)

After a shed's cooldown the governor reads what the step saved (smoothed CPU before the shed minus after
the cooldown). A reading under `MIN_SAVING` (0.2 ms, chosen, not measured) undoes the step and counts a weak
reading against that knob; the second weak reading in a row also **skips** the knob: it is never shed again
for that combination (below), and the next shed jumps over it. A reading at or above the floor clears the knob's
count. Knobs are identified by a stable id (`Knob::id` in `session/cpu_governor.rs`: shadows 0, model detail
`high` 1, model detail `original` 2), not by position, so a profile with a different set of steps keeps each
skip on its own knob.

- **Why 0.2 ms.** The Deck's CPU row repeats to about 0.1 ms run to run. Every reading taken on HD Anulpha
  (below, plus a probe) falls on one side of 0.2 ms: steps that bought nothing read 0.00, 0.00, 0.00, 0.04, 0.00,
  0.00 and once 0.15; steps that helped read 0.24, 0.35, 0.36, 0.36, 0.37, 0.39 and 0.53. The floor sits above the
  largest weak reading and below the smallest real one. The thread's example of 0.05 ms (or 1 % of the period)
  is under the noise and would have kept the weak step on any reading that happened to come out positive.
- **One reading cannot disable a knob.** The load can rise during a cooldown (the 0.15 probe reading and the
  6.5 to 6.6 ms CPU at the undo lines are such moments), so a single low reading only undoes the step. The knob
  is shed again once the CPU is still over, and only a second weak reading in a row skips it. A good reading in
  between clears the count. The price of a useless knob is therefore two extra short sheds (about 4 s each) before
  it is skipped.
- **An undo is not a restore.** It sets no relapse window and does not double the next restore's patience.
- **Skips are per combination, per launch.** What a knob saves depends on what is on screen, so the weak-reading
  counts and the skips are kept per combination of **title, track and mode** (maintainer decision 2026-11-05).
  `Session::feed_governor` names the combination with `Governor::enter`, hashed from the race's `result_key`
  (`session/cpu_governor.rs`, `combination`: title, circuit entry name, mode; a reversed run is its own track, the
  speed class is not part of it). The generic governor sees only the opaque `u64` and knows no title. A combination
  seen for the first time starts with nothing skipped and at full quality; going back to one restores its skips and
  counts without new readings. A stage restart of the same combination, `reset()` and the governor switched off and
  on keep the current combination's skips, as before. Nothing is written to settings: a new launch learns again.
  The store is `MAX_COMBINATIONS` (32) combinations that learned something beside the current one; one that learned
  nothing takes no room, and past the bound the combination entered longest ago is forgotten and learns again
  from nothing (two short sheds per useless knob). `enter` with the combination already named does nothing, so
  the per-frame call touches no store and allocates nothing.
- **Log.** `cpu governor: <knob> saved N ms` after each cooldown, and `undone, it saved too little: <knob>` for an
  undo (debug level).

### Deck, HD Anulpha, with against without the check (2026-11-05)

`main` at `eb9c2dda2` ("before") against this branch ("after"), release `dist` znver2 builds from plain
worktrees (`target-lf-before`, `target-lf-batch1-base`), `--no-default-features --features extensions,dev-cli`.
`settings-gov-on.toml` for both (governor on, `vsync off`, `frame_limit 165`, extras `shadows mapped`,
`model_detail maximum`), `--render-scale 100 --no-intro --autopilot --seed 1 --no-audio`, 60 s, the first 4 frame-log
lines dropped, before and after interleaved, three rounds, load 1.9 to 2.3.

**The 2026-10-27 recipe no longer reaches the budget.** Since the refresh clamp the governor's budget is the
lower of the frame limit and the display's refresh. Under the Deck's gamescope on the X11 path (`ENABLE_GAMESCOPE_WSI=0`)
the refresh reads 60 Hz, so the budget is 16.7 ms and the CPU (8.2 ms) is never over it: three rounds of
`ab.sh 3 60 anulpha` at scale 50 logged no governor line in either arm. These runs therefore start the game inside
a nested `gamescope --backend headless -W 1280 -H 800 -r 165`, which reports 165 Hz (`display refresh in force: 165 Hz`)
and gives the 6.06 ms budget again. The numbers are not comparable with the 4K ones above. At 4K scale 100 under
the Deck's own gamescope GPU time was 6.2 ms (over 0.9 of 6.06 ms), so the governor holds there by design.

| Arm | fps | frame_ms | p99 | CPU row | gpu_ms | knobs down at the end |
| --- | --- | --- | --- | --- | --- | --- |
| before 1, 2, 3 | 151.6, 151.2, 151.7 | 6.64 | 8.47 | 5.86 | 2.79 | 3, 3, 3 |
| after 1, 2, 3 | 151.3, 151.0, 151.2 | 6.66 | 8.55 | 5.87 | 2.79 | 2, 2, 2 |

Same fps within noise (151.5 against 151.2; the spread inside a group is 0.5 fps), same CPU row, same p99,
one knob fewer stepped down. In every "after" run the governor shed `shadows original` (saved 0.24 to 0.37 ms), shed
`model detail high` (saved 0.00 to 0.04 ms), undid it, shed it again (0.00), undid it and skipped it, then shed
`model detail original` straight from `maximum` (saved 0.36 to 0.53 ms). So on this device the middle detail rung
bought nothing and the player keeps it: `model detail high` is the step that no longer happens, and the last step now
lands on the one that pays. The A/B binary carried a 0.15 ms floor; all fourteen readings above fall on the same side of
0.2 ms, so the decisions are the same.

Checks against the other titles: the governor is title-agnostic and the knobs are the same on every title (ported;
Pulse and Pure share this code). Omega and 2048 profiles reach it through the same `steps` list, not measured here
(not checkable without their own Deck runs).

## Open

A CPU lever worth shedding would have to be one of ours that costs real CPU: the post-chain pass
count (10 bloom downsamples, 7 blur passes) and per-view shadow passes in split screen are not
yet knobs. Measure those on the Deck before adding any.
