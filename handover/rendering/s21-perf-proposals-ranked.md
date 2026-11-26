---
status: measure
titles: [hd, pulse]
updated: 2026-11-26
summary: "S21 HD perf plan: cards culled, row kinds folded, wgpu trunk pinned for #10319 (-2.4 to -12 ms; back to crates.io at wgpu 31); ASTC opt-in landed"
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

1. ~~Draw an advert card only on frames where a view samples it~~ **landed** (`s21-passes`):
   S21 -4.2 and -5.6 ms in-run, Deck -0.4 to -0.8 ms, host -4.8 % user instructions, picture
   byte-identical (`docs/rendering/frame-thread-cpu.md`, "Advert cards: drawn only when shown").
   Culls with a mod shader attached too (Pulse's official `slipstream`); a portal view may show a
   card stale or blank, accepted by the maintainer.
2. **Fewer small passes.** A render pass costs 0.14 to 0.41 ms on the S21, about 0.015 to 0.03 ms
   on the Deck (`docs/tools/android.md`, "What a render pass costs").
   - **The bloom ladder fold is parked** on branch `bloom-ladder-fold-parked` (maintainer: the
     ladder is the original's own 256x256 R16F halving chain): it saves about 1.1 ms on the S21
     (two in-run toggles; 2.2 ms on a third) and moves 1-2/255 on up to 7.7 % of pixels in 12 of
     16 HD pairs. A version replaying the halvings with half-precision rounding in one pass was
     not byte-identical either.
   - **Velocity-less cards and glass: not worth it.** With the cards culled, leaving every card
     out reads 28.9 against 28.3 ms on the S21 (noise), which bounds what their velocity
     attachment can cost; the glass target at quarter is about 14 k pixels.
3. ~~One presentation-size pass~~ **measured and dropped**: drawing the resolve, HUD and blooms
   straight onto the surface (composite skipped, neutral grade, no filter) read slower on the S21
   in both in-run toggles, 26.0 against 26.8 ms and 26.9 against 28.2 ms
   (`docs/tools/android.md`, "What a render pass costs").
4. ~~**Fewer per-frame writes**~~ **landed as a git pin of wgpu trunk** (`wgpu-trunk`). The cost was the barrier-plus-copy
   pair wgpu-core 30.0.2 records per `write_buffer` (gfx-rs/wgpu#6673), fixed by gfx-rs/wgpu#10319, which is on trunk only.
   The tree now takes `wgpu` and `naga` from gfx-rs/wgpu at rev `60cef2be828e` (`docs/architecture/dependency-upgrades.md`,
   "Git pins"). S21 HD Vineta K, three interleaved rounds against 30.0.2: **-2.4 to -12.0 ms per frame** over whole races
   (31.0/30.5/29.9 to 19.0/25.8/27.5 ms), all of it in the present wait; HD, Pulse and Omega host frames byte-identical
   (`docs/tools/android.md`, "wgpu trunk (the git pin) against 30.0.2"). One write a frame stays not worth building (the
   copies cost about 0 ms with #10319). Open, each measured by the S21 protocol of that section:
   - **Back to crates.io when wgpu 31 ships**: `wgpu = "31"` and `naga` the same in the root `Cargo.toml`, drop the rev, re-run
     the six-frame picture check and one S21 round. `gh release list -R gfx-rs/wgpu` shows when; trunk past the pin may add
     breaks of its own.
   - **Trunk's two speeds on the S21**: about 18 ms for a stretch, then about 27 ms (rounds switched at racing second 70, 20,
     or never). The slow mode is #10319's 2.4 to 3.1 ms; what gives the fast mode (a later trunk change or GPU clocking, one
     fast run read 858 MHz against 403) is unknown. A perfetto fence trace per mode would show which wait shrinks.
   - **`hd_zoom` pass timestamp** reads 1.6 to 2.9 ms on trunk against 0.1 to 0.2 ms on 30.0.2 while the frame got faster;
     probably the hoisted barrier landing inside that timestamp pair. Unproven.
   - **Host GPU p50 at 1080p about +5 %** on trunk (791/779/793/799 against 734/753/793/760 us, four rounds of
     `just bench-render hd --size 1920x1080 --binary <build>`), frame and CPU unchanged; inside 30.0.2's own spread in
     one round. Re-measure on an idle host, or with `OAG_RENDER_GPU_BENCH` per pass, before calling it real.

5. ~~Measure the track's shading on Mali~~ **measured, one fix landed** (`s21-shading`): the rich
   tenth of the circuit's draws costs half the scene pass (`track-rich` 11.1 to 5.6 ms of scene
   GPU, 4.3 ms of frame); the plain nine tenths read 0.8 ms of pass but 3.5 ms of frame, and
   texture bandwidth (`mips`) 0.8 and 1.6 ms. One run each; the frame reading is the safer one.
   `circuit_lighting = simplified` (plain surfaces only) is unmeasured on the S21, bounded by about
   3.5 ms of frame. `malioc`: the
   rich shader spilled 512 bytes of stack at 64 registers and 50 % occupancy. Folding the row kinds
   a circuit's table lacks (`program_codes`, `rate_codes`) took it to 304 bytes: S21 scene GPU -2.4
   to -3.0 ms and fence period -3.2 to -3.7 ms in-run, frame -0.6 to -1.6 ms; Deck -8 % (1080p) and
   -10 % (4K); byte-identical on 26 shots across HD, Omega, 2048 and Pulse, Zone included
   (`docs/rendering/hires-gpu-levers.md`, "A circuit's row kinds fold where its table has none").
   **Still open**: 304 bytes of stack remain (208 with every kind folded); the rest of the spill is
   the uber-shader's live state. A `malioc` bisect over `lit_colour`'s blocks (the scratch method:
   stub a function, re-run `malioc --core Mali-G78`) is the next read; 4 h; confidence 30.

**Later**

6. One upload per frame instead of a staging buffer per `write_buffer`: HD frame 0 (its thread -0.5
   ms), Pulse on the S21 -0.5 to -0.9 ms; 6 h; same plumbing as item 4.
7. Submit and present on their own thread: Pulse on the S21 -3 to -5 ms if its 6.5 ms present block
   is the driver's submit; HD 0; 14 h; confidence 40.
8. The cards' size as a setting (`original` default, `auto` on the reflection controller): only if
   their pixels turn out to matter on Mali; setting-gated; 4 h.
9. Race pass: store velocity and depth only when a pass reads them: -0.5 to -1.0 ms at render scale
   100 on the S21, nothing at 25; 5 h.
10. ~~ASTC where the device has no BC~~: **landed** (`astc-optin`) as the opt-in TEXTURE COMPRESSION
    row (`astc`, default `original`), encoded by astcenc `medium` into an on-disk cache on a background
    thread; S21 numbers with the real encode in `docs/rendering/texture-compression.md`.

Not proposed, each already measured: dropping the depth prepass on a tiler (it pays), skipping the
glass target when no glass shows, a third swapchain image, a portrait or pre-rotated surface.

## Next Steps

1. ~~The pass-cost toggle~~ and ~~the card census~~: done (`s21-passes`), numbers in
   `docs/tools/android.md` ("What a render pass costs") and `docs/rendering/frame-thread-cpu.md`.
2. ~~Freeze-writes~~: done (`s21-shading`), item 4 re-scoped above.
3. Log one timestamp pair from the first pass's begin to the last pass's end as its own frame-log
   field, so the rows' sum has a denominator.
4. Repeat the cool and warm baselines three times each and regress the frame on the GPU and A78
   clocks `scripts/android-perf-poll.sh` already logs, to firm up finding 1.
5. When a finding above is confirmed by a run, write it into `docs/tools/android.md` with its
   command; this thread is the only place findings 1, 2, 3 and 7 are written down today.
