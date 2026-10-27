# The CPU governor

`oag_present::governor` is a closed loop beside dynamic resolution ([`dynamic-resolution.md`](dynamic-resolution.md)).
DRS sheds pixels when the GPU is the slow side. The governor acts when the frame thread's CPU time
(the `CPU` row: frame time minus swapchain waits) is over the frame limiter's period while the
smoothed GPU time is under 0.9 of it, and it steps down an ordered list of this project's own
extras. It restores them one at a time, with hysteresis, when the CPU plus that knob's measured
saving fits in 0.8 of the budget for 6 s. Thresholds are chosen, not measured. It holds when the GPU
is the bound side, when there is no GPU reading, and when nothing paces the frame (vsync `on`, or an
unlimited loop).

- **Runtime only.** `Session::governed_profile` hands the draw a copy; `render_profiles` is never
  written. Captures and `--screenshot` run headless and never reach it.
- **Knobs** (`session/cpu_governor.rs`): `shadows mapped` down to the title's authored tier
  (`original`, or `off` where the title authors none), then `model_detail` `maximum` to `high` to
  `original`. Nothing goes below what a title authors, so a default profile has no steps.
- **Setting:** `graphics.cpu_governor` (CPU GOVERNOR on the advanced graphics page), **on by default** (since the unlocked Deck measurement below; a profile with no extras has nothing to shed, and a race that holds its target never moves a knob).
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
check at the end uses WSI=1, the player's path.

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

**Verdict: recommend default on; the default stays off by maintainer decision** until there are more
knobs or cases with stronger impact. fps and mean p99 improve beyond run-to-run noise when the frame rate is
unlocked (HD +3.7 %, Pulse +10 % where the CPU is over budget), and in a capped race that holds its target the
governor never acts. A settings copy with no `cpu_governor` key stays off. The earlier "0.25 ms is not worth trading tiers" reading was a capped
measurement; the same 0.25 ms is 3.7 % of frame time once the cap is gone. The cost is real: when it acts it
trades the player's own shadow map and model detail tiers for a few percent, which is why a profile without
extras, and a race at its target, are untouched.

**Does a step that saved nothing get undone? No.** `governor.rs` records each shed's saving but only uses it
to time a restore. In the HD logs the `model detail original` step came at 6.8 to 6.9 ms CPU after `model
detail high` at 6.55, and the frame-log lines at level 3 read about the same CPU as level 2, so it bought nothing and
stayed. Open item in the perf handover thread (lane 6), not implemented here.

## Open

A CPU lever worth shedding would have to be one of ours that costs real CPU: the post-chain pass
count (10 bloom downsamples, 7 blur passes) and per-view shadow passes in split screen are not
yet knobs. Measure those on the Deck before adding any.
