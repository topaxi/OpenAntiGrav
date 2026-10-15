# Wipeout HD's race frame: where the GPU time goes (2026-10-15, `hd-gpu`)

The question was why HD is the title that runs out of GPU first (14.0 ms on the
Raphael iGPU at 1280x800, 28 fps on the Steam Deck's worst stretch) and what can
come off it without changing the picture. **One cut landed (craft outside the
frustum are not drawn, byte-identical, 1 to 3 % of the GPU); the largest one found
is a design decision left open (the velocity attachment, about 10 % of the race
pass); and three suspects were measured and cleared.** Everything is `chosen`
workload, not measured against the original.

## The machine

A notebook, not the AMD desk machine the earlier numbers came from: Intel Arrow
Lake iGPU (`Intel(R) Graphics (ARL)`, Mesa anv) plus an NVIDIA RTX PRO 2000
Blackwell laptop dGPU. Every figure says which. Two traps on this host:

- **`MESA_VK_DEVICE_SELECT` does not filter NVIDIA's own ICD**, so the first "iGPU"
  run was on the NVIDIA card. `VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/intel_icd.json`
  selects the iGPU (the Vulkan loader then only sees it); the adapter is printed at
  the end of every `bench:` line.
- **The first bench after a rebuild read 2x the CPU time** (`cpu p50 4,805 us`, then
  2,472 us on the same code a run later). Throw away the first run after a build, or
  run the same build twice.

**This iGPU is not GPU-bound at 1280x800.** Whole-race HD there is frame 4.3 ms with
the GPU passes 2.2 ms: the CPU (record plus submit) sets the pace, so a GPU saving
barely moves `frame_us`. At 1920x1080 (gamescope's pixel budget on the Deck) the GPU
passes are 3.3 ms against 2.6 ms of CPU, which is the shape closest to the Deck that
this host offers. **Read GPU spans, not `frame_us`, for any change here**, and the
Deck numbers (below) for the verdict.

## Where a frame goes

`OAG_RENDER_GPU_BENCH=120`, `oag-game --features perf-probe`, HD Talon's Junction
`--autopilot` at tick 2,500, **1920x1080**, Intel iGPU, defaults (MSAA off, blur off,
shadows `original`, anisotropy 16x), median microseconds, one machine:

| Piece | us | Of it |
| --- | ---: | --- |
| race pass | 3,258 | track solid 2,024 (depth prepass about 300, shading about 1,200 in the tick-600 split), track blended 403, craft 592, effects 86 |
| HD bloom chain | 661 | fixed floor about 210 at 320x200: the downsample ladder, adaptation, gate, two blurs and the encode are separate passes |
| shadow passes (`--shadows off` removes them) | about 1,370 | `Scene::render` 8,024 against 6,654 |
| the rest of `Scene::render` | about 1,000 | not attributed |

How the cost scales (race pass, tick 600): 1,015 us at 320x200, 2,520 at 1280x800,
3,550 at 1920x1080, 5,105 at 2560x1440 - about 1 ms per megapixel on top of a
**1 ms floor** that is geometry (track solid is 520 us at 320x200 for 248 k
fragments; craft 346 us for 44 k). So HD is two costs: a floor that does not fall
with the render extent, and a fill term that does.

The same frame, one setting at a time (race pass unless said):

| Setting | us | Delta |
| --- | ---: | --- |
| defaults | 3,258 | - |
| anisotropy off | 3,143 | -115 (3.5 %) |
| `--msaa 4x` | 4,513 | +1,255 (+39 %) |
| `--msaa 4x --motion-blur high`, chain | 2,194 | +2.2 ms beside the pass |
| the same with `--motion-blur-resolution half` | 1,533 | -660 against full |

## What was cleared

- **The depth prepass pays for itself at 1080p.** Pipelined race pass 3.48-3.73 ms
  with it against 3.95-4.28 ms without (`track solid` 2.07-2.24 ms alone): it
  stays on at Deck sizes. The earlier proof was at 3200x1800 only.
- **`StoreOp::Discard` on the velocity, depth and MSAA colour attachments** when
  nothing reads them: no change (5.5 against 5.7 ms pipelined, inside the noise).
  Not kept. The side passes (`behind the glass`, adverts) already discard theirs.
- **The bloom chain** follows the original's pass list at the original's buffer
  sizes; its floor is the ladder of small passes. Nothing to take without changing
  the picture.

## What landed: craft outside the frustum are not drawn

`Scene::render` drew every hull with no frustum, so a field of twelve cost twelve
hulls' vertices and draw calls however many were on screen. `scene/craft_cull.rs`
tests each hull's bounding sphere (`Model::radius`, widened 1.5x for the airbrake
flaps, node animation and camera jitter: **chosen, not measured**) against the
view's frustum, the same sphere test the circuit already uses; `frustum_culling`
off draws them all.

- **Byte-identical**: 24 frames (Talon's Junction and Anulpha Pass, ticks 300 to
  6,900 every 600, 1280x720) and a two-player horizontal split frame, cull on
  against cull off, `cmp` equal.
- **Craft span** (1080p, tick 2,500 and 5,000): 620 to 552 us and 649 to 519 us
  (-11 % and -20 %); at tick 600, with the whole grid on screen, no change. Draw
  calls fall about 10 % where it culls.
- **Whole race** (`hd-gpu-before2-*` against `hd-gpu-after-*`, same code but this
  commit): iGPU 1280x800 gpu p50 2,244 to 2,182 us, 2h 2,865 to 2,735; iGPU 1920x1080
  3,304 to 3,257 us, 2h 3,793 to 3,678; NVIDIA 1280x800 1,037 to 1,054 (CPU-bound,
  no change). **1 to 4 % of the GPU on the iGPU, inside run-to-run noise for the
  whole race**; the per-tick spans are the firmer evidence.
- **Pinned by** `tests::craft_cull::a_hull_out_of_view_is_not_drawn_and_one_in_view_is`
  (a real `Scene::render` over a grid: the craft behind the camera adds no triangle,
  the one in view adds one, frustum culling off draws both) and the unit tests in
  `scene::craft_cull`.

## What is open: the velocity attachment

Every opaque pipeline writes the velocity attachment (`Rg16Float`, the motion blur's
and FSR 3's input) whether or not anything reads it; `Scene::velocity`'s own doc says
that is by design. Masking its write for the opaque pipelines (an experiment, not
kept): race pass 3,484 to 2,973 us with MSAA off and 4,529 to 4,221 us with 4x,
1080p, tick 2,500, **0.3 to 0.5 ms, about 10 %**, and on a shared-memory APU the
bandwidth should weigh at least as much. Doing it for real means a second pipeline
variant for the opaque, prepass-shade and cutout pipelines, chosen per frame by
"blur is on or camera jitter is on". That reverses a stated design, so it is the
maintainer's call.

## Measure on the Deck (the maintainer runs this)

1. `just bench-render hd --size 1920x1080 --views 1,2h --tag deck-hd` and again
   with `--size 1280x800` (no `MESA_VK_DEVICE_SELECT`; the adapter must read
   `RADV VANGOGH`, not llvmpipe). **The offscreen bench leaves out the presented
   upscale and composite to 4K**, which cost 4 to 5 ms on the Deck
   ([dynamic resolution](dynamic-resolution.md)): add that to its frame before
   comparing with 28 fps.
2. The spans: `cargo build --release -p oag-game --features perf-probe`, then
   `OAG_RENDER_GPU_BENCH=150 OAG_RENDER_PERF=1 target/release/oag-game
   data/images/hdfury-ps3-eu-dec.iso --race --mode single_race --seed 1 --autopilot
   --no-audio --size 1920x1080 --ticks 2500 --screenshot out.png` and, for the
   likely profile, the same with `--msaa 4x --motion-blur high`. Read the
   `bench race pass`, `HD bloom`, `motion blur` and `span / ...` lines.
3. `--shadows off` on the same line is the shadow passes' cost on the Deck.
