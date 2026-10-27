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
- **Setting:** `graphics.cpu_governor` (CPU GOVERNOR on the advanced graphics page), **off by default**.
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

A win of about 0.25 ms (3 %) of CPU. p99 does not move because gamescope holds presentation to the
panel's 90 Hz (fps 90.0 in both), so the over-budget case is only over the 165 budget, not
dropping frames. A race at its target (vsync `smooth`, limit 90, extras on) logged `governor=0`
throughout and no change line. Hence the off default.

Frame counts in the controller are in budget periods, so at a presentation rate below the budget
the waits last longer in wall time (about 1.8 s per shed streak here).

## Open

A CPU lever worth shedding would have to be one of ours that costs real CPU: the post-chain pass
count (10 bloom downsamples, 7 blur passes) and per-view shadow passes in split screen are not
yet knobs. Measure those on the Deck before adding any.
