---
status: measure
titles: [hd, pulse]
updated: 2026-11-25
summary: "S21 HD: ranked perf plan. Pass count and Mali's backend thread look like the limit, not GPU compute; run the pass-cost toggle first"
categories: [rendering, tooling]
size: L (40)
difficulty: medium (40)
---

# The S21's HD frame: ranked proposals, and what to measure first

A read-only review of the 2026-11-25 S21 profile (`docs/tools/android.md`, "What the frame thread
waits for" and "Where an HD frame goes on the S21"; leads in
[hd-on-the-s21-is-gpu-bound-and-the-advert-cards-cost-3-ms.md](hd-on-the-s21-is-gpu-bound-and-the-advert-cards-cost-3-ms.md)).
No code changed. Baseline: Galaxy S21 (Mali-G78), HD Vineta K forward, render scale 25 (476x270),
REFLECTION RESOLUTION `auto` at quarter: 26.3 ms from a cool start, 34.8 ms warm.

## What the review found (re-derived from that lane's own captures)

1. **The frame is not bound by GPU compute.** Across that lane's five baseline runs, the frame
   follows the CPU cap at a constant GPU clock (858 MHz: 26.3 ms with the A78s at 2080 MHz, 29.9 ms at
   1456 MHz) and barely follows the GPU clock (A78s at 1456 MHz: 29.9 ms at 858 MHz, 34.8 ms at
   403 MHz; the scene row 9.3 to 10.7 ms). About 4 to 5 ms of the frame scales with the GPU clock.
   Single runs with different start temperatures: confidence 50.
2. **Mali's backend thread sits in series with the GPU.** In the same Perfetto capture
   (`sched_slice` of `mali-cmar-backe` between matched presents) it is on the CPU 10.7 ms a frame in
   35 slices, on the A78 cores, and 4.2 ms of that falls in the gap after the previous frame's fence
   (gap 3.7 to 7.1 ms, p10 to p90). In the 60 s `simpleperf` record 41 % of its CPU time is the
   job-submit `ioctl` (`cmarp_submit_atoms_to_base`), 14 % reading completions. 35 slices a frame is
   about one per render pass: the frame has about 34. So a render pass costs about 0.3 ms of serial
   driver time here, whatever it shades.
3. **"The advert cards cost pixels" was shown on a Radeon 610M, not on the Mali.** On the S21 the
   eight card passes (1.05 M pixels) read 1.3 to 1.6 ms and HD's bloom chain (14 to 16 passes, about
   0.2 M pixels) reads 2.8 to 3.2 ms: the same 0.16 to 0.23 ms a pass. Single-pass rows read lower
   (0.05 to 0.12 ms), so this is confidence 50, not a law.
4. **The originals redraw every card every frame** (Pulse: GE dump, confidence 92,
   `docs/ghidra/functions/psp-pulse-usa/billboards.md`; HD: `Billboard_UpdateAndRender`, confidence
   82, `docs/ghidra/functions/ps3-hdfury-eu/billboards.md`). "Draw a still card once" is already
   measured and dropped (`docs/rendering/frame-thread-cpu.md`, "Why `submit` is the bottleneck").
5. **The pass rows under-report on this GPU**: every skip toggle reads 2 to 6 times its row.
   Anything keyed on a row (REFLECTION RESOLUTION `auto`'s 10 % rule) is biased low on a tiler.
6. **`submit` 8.1 ms** is `finish` 5.3 ms plus `Queue::submit` 2.8 ms. Of the thread's CPU time:
   Mali's own entry points about 25 %, wgpu validation proper 8 to 10 %, `write_buffer` and its
   staging buffers 6 %. The driver's `memset` is per-draw command memory, not descriptor or uniform
   churn. None of it is the limit for HD today; it is for Pulse on this phone.
7. **The track's shading costs 30 to 50 times the cards' per pixel on Mali** (65 to 90 ns a pixel
   from the glass target's size steps and the render-scale steps; 4.4 times on an RX 7800 XT), and it
   does not follow the GPU clock. HD's DXT textures are decoded to RGBA8 here. Nobody has looked
   inside the scene pass on this GPU.

## Open

Ranked. Gains are S21 frame ms on the baseline; "RDNA" is the RX 7800 XT at 1080p, where the whole
scene is 1.5 ms. Every "fidelity none" item passes the byte-identity or pair checks before it merges.

**Do now**

1. **Draw an advert card only on frames where a view samples it** (`crates/raceplay/src/adverts.rs`,
   `scene/frame.rs`). Test each card's placeholder draws against the PVS and frustum of the main and
   behind-the-glass views; draw all cards when there is more than one view. A card is a pure
   function of the scenery clock and is cleared on each draw, so the picture is identical by
   construction. S21 -1.5 to -2.3 ms, RDNA -0.1 ms, Radeon 610M -1.0 to -1.3 ms; 5 h; confidence 60.
2. **Fewer small passes** (`crates/post/src/hd_bloom.rs`). Fold the luminance ladder's seven to nine
   halvings into 8x8-box passes (same mean of the same texels, a third of the passes); build the
   cards and the glass target without the velocity output they discard. S21 -0.8 to -1.8 ms, RDNA
   -0.05 ms; 8 h; fidelity none intended, the ladder's rounding must be checked against the HD
   pairs; confidence 45.
3. **One presentation-size pass** (`crates/present/src/upscale.rs`, `crates/game/src/render.rs`).
   The upscale, the HUD (drawn with a `Load` of 2.5 M pixels) and the composite are three full-size
   passes. The earlier single-run attempt read slower while the toggles price the three at 13.5 ms
   on the flyby: re-measure in one run before believing either. S21 -1.0 to -2.5 ms, RDNA -0.03 ms;
   3 h to re-measure, 6 h to land; fidelity none; confidence 35.
4. **Stop frame N's buffer writes waiting on frame N-1** (`crates/gpu/src/writes.rs`). wgpu replays
   every `write_buffer` as a copy at the head of the submission, into buffers the previous frame's
   last passes still read; on a deferred-job driver that can hold the whole frame back. A ring of
   destination buffers removes it. Only after the freeze-writes experiment below. S21 0 to -4 ms,
   RDNA 0; 3 h experiment, 16 h to land; fidelity none; confidence 35.
5. **Measure the track's shading on Mali** (`crates/mesh/shaders/`). Arm's offline compiler on the
   circuit's specialised fragment shader, a toggle that drops the top two mips of the decoded
   textures (memory against arithmetic in one run), and one stub toggle per lighting term. S21 -3 to
   -5 ms possible, RDNA -0.1 ms; 8 h to diagnose; confidence 25.

**Later**

6. One upload per frame instead of a staging buffer per `write_buffer`: HD frame 0 (its thread -0.5
   ms), Pulse on the S21 -0.5 to -0.9 ms; 6 h; same plumbing as item 4.
7. Submit and present on their own thread: Pulse on the S21 -3 to -5 ms if its 6.5 ms present block
   is the driver's submit; HD 0; 14 h; confidence 40.
8. The cards' size as a setting (`original` default, `auto` on the reflection controller): only if
   their pixels turn out to matter on Mali; setting-gated; 4 h.
9. Race pass: store velocity and depth only when a pass reads them: -0.5 to -1.0 ms at render scale
   100 on the S21, nothing at 25; 5 h.
10. ASTC where the device has no BC: frame gain unknown until item 5; GPU memory 2.96 GB to about
    0.6 GB ([texture-compression-only-covers-bc-astc-is-the.md](texture-compression-only-covers-bc-astc-is-the.md)).

Not proposed, each already measured: dropping the depth prepass on a tiler (it pays), skipping the
glass target when no glass shows, a third swapchain image, a portrait or pre-rotated surface.

## Next Steps

1. **The pass-cost toggle, about 1 hour on the phone.** Three runs with the skip-toggle protocol
   (`docs/tools/android.md`, "Where an HD frame goes on the S21"): `adverts` as it is; a new skip
   name that keeps the eight card passes and their clears and drops the draws; a new one that adds
   16 one-pixel clear passes at the head of the frame. It gives ms per pass and says whether the
   cards cost pixels or passes, which sizes items 1, 2, 3 and 8.
2. **The card census, desktop only.** Count per frame of one autopilot lap how many cards have a
   placeholder draw that survives the PVS and frustum (Vineta K, Anulpha, Moa Therma). Above 5 of 8
   on average, item 1 moves to "later".
3. **Freeze-writes.** A toggle that drops every `Writes::write_buffer` and `write_texture` while it
   is on, under the fence-timeline capture (`scripts/android-fence-timeline.py`): if the present
   returns before the previous fence while frozen, item 4 is real; if nothing moves, close it.
4. Log one timestamp pair from the first pass's begin to the last pass's end as its own frame-log
   field, so the rows' sum has a denominator.
5. Repeat the cool and warm baselines three times each and regress the frame on the GPU and A78
   clocks `scripts/android-perf-poll.sh` already logs, to firm up finding 1.
6. When a finding above is confirmed by a run, write it into `docs/tools/android.md` with its
   command; this thread is the only place findings 1, 2, 3 and 7 are written down today.
