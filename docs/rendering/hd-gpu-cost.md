# Wipeout HD's race frame: where the GPU time goes (2026-10-15, `hd-gpu`)

The question was why HD is the title that runs out of GPU first (14.0 ms on the
Raphael iGPU at 1280x800; the "28 fps on the Steam Deck's worst stretch" turned out to be one
stalled frame at race start, see the last section) and what can
come off it without changing the picture. **One cut landed (craft outside the
frustum are not drawn, byte-identical, 1 to 4 % of the GPU on the iGPU); the velocity
attachment, first thought to be about 10 % of the race pass, measured as nothing
once repeated; and three more suspects were measured and cleared.** Everything is
`chosen` workload, not measured against the original. The per-piece costs below are
**shares of this frame on this machine, not a Deck forecast**: the Arc 140T is not
the Deck's 8-CU RDNA2 APU and favours different things.

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

On the NVIDIA card (same frame, 1080p, MSAA off) the shares are similar: race pass
about 1.2 ms of which track solid 0.72 ms (60 %), HD bloom 0.15 ms; with MSAA 4x
race pass 1.72 ms, track solid 0.99 ms (58 %). The track's shading is the biggest
single piece on both adapters, and what the original authors there is held to
byte-exactness.

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
tests each hull's bounding sphere against the view's frustum, the same sphere test
the circuit already uses; `frustum_culling` off draws them all. The sphere is
`Drawable::bound`, the vertices' own box centre and the farthest vertex, **not**
`Model::radius`: that is a sphere for an `rcs` model but only the largest
half-extent of the box when `merge` built it (the PSP and PS2 hulls), up to 1.7x too
small to cull by. It is widened 1.5x for the airbrake flaps, node animation and
camera jitter (**chosen, not measured**).

- **Byte-identical on six titles**: 36 frames, Pulse PSP, Pure PSP, Pulse PS2, 2048,
  Omega and HD, ticks 300 to 6,300 every 1,200, 1280x720, the binary before this
  change against the binary after (`cmp` equal); and 24 more HD frames on Talon's
  Junction and Anulpha Pass (ticks 300 to 6,900 every 600) plus a two-player
  horizontal split frame, cull on against cull off.
- **Craft span** (1080p, tick 2,500 and 5,000): 620 to 552 us and 649 to 519 us
  (-11 % and -20 %); at tick 600, with the whole grid on screen, no change. Draw
  calls fall about 10 % where it culls.
- **Whole race** (`hd-gpu-before2-*` against `hd-gpu-after-*`, same code but this
  commit): iGPU 1280x800 gpu p50 2,244 to 2,182 us, 2h 2,865 to 2,735; iGPU 1920x1080
  3,304 to 3,257 us, 2h 3,793 to 3,678; NVIDIA 1280x800 1,037 to 1,054 (CPU-bound,
  no change). **1 to 4 % of the GPU on the iGPU, inside run-to-run noise for the
  whole race**; the per-tick spans are the firmer evidence.
- **Side effects checked** (a hull's draw is the only thing skipped): the craft's blob
  shadow, the shadow-map casters, the engine flare, the boost plume, the trail ribbons,
  the exhaust and spark particles, the shield shell and the engine light are separate
  draws or uniforms and are not tested against the hull's sphere, so an off-screen craft
  still shadows and lights the track as before. The behind-the-glass target draws
  circuit chunks, not craft. In a split every view runs `Scene::render` with its own
  view-projection, so the test is per view. Byte-identical against the binary with no
  cull on the start grid (ticks 60 and 150, craft beside and behind the chase camera),
  mid-race (tick 2,700), with `--msaa 4x --motion-blur high` and with `--presented
  --reconstruction fsr3 --camera-jitter`, on HD and Pulse (18 frames), and on a
  two-player split cut both ways at ticks 150 and 1,800.
- **Pinned by** `tests::craft_cull::a_hull_out_of_view_is_not_drawn_and_one_in_view_is`
  (a real `Scene::render` over a grid: the craft behind the camera adds no triangle,
  the one in view adds one, frustum culling off draws both) and the unit tests in
  `scene::craft_cull`.

## Measured and retracted: the velocity attachment

Every opaque pipeline writes the velocity attachment (`Rg16Float`, the motion blur's
and FSR 3's input) whether or not anything reads it. A single pair of runs with the
write masked on every pipeline read 3,484 to 2,973 us (MSAA off) and 4,529 to 4,221 us
(4x): "about 10 %". **That did not survive repetition.** Three interleaved rounds at
1080p, tick 2,500, MSAA off, Intel iGPU, race pass median us: no masking 3,176 / 3,423 /
3,168; track and hulls masked 2,959 / 3,325 / 3,394; every pipeline masked 3,324 / 3,185 /
3,651 - all inside this host's 9 % drift between runs of one binary. The 4x pair on the
iGPU is no different (4,340 / 4,770 ungated against 4,452 / 4,819 gated). On the NVIDIA
card the gated track and hulls read 2 to 8 % lower in four of four pairs (MSAA off
1,210 / 1,182 to 1,116 / 1,154; 4x 1,716 / 1,738 to 1,600 / 1,648), about 0.03 to 0.12
ms.

The gating itself was built and tested (`Velocity::Gated` building the opaque, prepass
and cutout pipelines a second time with the velocity target's write masked; one list of
the buffer's readers, the motion blur and a temporal upscaler; masked pipelines proven
colour-identical to the written ones by a render test) and **left out of the branch**:
it buys nothing measurable on the Intel iGPU and a few percent on the NVIDIA card for a
second set of pipelines and a reversal of the documented "always written" rule. It is
kept as the branch `hd-gpu-velocity-gate` (commit `e58a38d0e`) for the maintainer to try
on the Deck, where a shared-memory APU is the most likely place for a bandwidth saving
to show.

## Measure on the Deck (the maintainer runs this)

1. `just bench-render hd --size 1920x1080 --views 1,2h --tag deck-hd` and again
   with `--size 1280x800` (no `MESA_VK_DEVICE_SELECT`; the adapter must read
   `RADV VANGOGH`, not llvmpipe). **The offscreen bench leaves out the presented
   upscale and composite to 4K**, which cost 4 to 5 ms on the Deck
   ([dynamic resolution](dynamic-resolution.md)): add that to its frame before
   comparing with a presented run (the old 28 fps figure was a stalled frame at race
   start, not a stretch; see the last section).
2. The spans: `cargo build --release -p oag-game --features perf-probe`, then
   `OAG_RENDER_GPU_BENCH=150 OAG_RENDER_PERF=1 target/release/oag-game
   data/images/hdfury-ps3-eu-dec.iso --race --mode single_race --seed 1 --autopilot
   --no-audio --size 1920x1080 --ticks 2500 --screenshot out.png` and, for the
   likely profile, the same with `--msaa 4x --motion-blur high`. Read the
   `bench race pass`, `HD bloom`, `motion blur` and `span / ...` lines.
3. `--shadows off` on the same line is the shadow passes' cost on the Deck.

## The Deck's "28 fps" was one frame at race start (2026-10-19, `deck-hd-28fps`)

**Verdict.** HD never had a 28 fps stretch of the circuit on the Steam Deck. The number was
the first line of `--frame-log` in the race, and it came from **one stalled frame at race
start**, not from any render pass. On the Deck panel that frame took 248 to 264 ms. The frame
log's `fps` is a rolling mean over the last 120 frames (`oag_present::perf::WINDOW`), so one
250 ms frame stays in the first one or two lines a second apart. Read per second, that looked
like a slow stretch.

**Evidence, 2026-10-14 (`deck-4k`, a TV at 3840x2160, gamescope budget 1906x1080, `--race
--autopilot`, one view).** The first lines of the race, a second apart:

```
fps=28.5 frame_ms=35.11 p99_ms=46.53 cpu_ms=17.76 gpu_ms=7.97 present_ms=8.32 render=1906x1080
fps=38.9 frame_ms=25.69 p99_ms=46.53 cpu_ms=17.18 gpu_ms=7.52 present_ms=8.01 render=1906x1080
fps=59.9 frame_ms=16.68 p99_ms=20.04 cpu_ms=16.64 gpu_ms=7.14 present_ms=7.62 render=1906x1080
fps=64.2 frame_ms=15.57 p99_ms=19.67 cpu_ms=15.59 gpu_ms=7.25 present_ms=6.58 render=1906x1080
fps=71.1 ...  then 67 to 93 for the rest of the 56 s run
```

The two-view run reads the same: 27.1 and 38.7 in its first two lines, then 56.6 at its worst.
After the start window, HD's worst second at the 4K budget is about 60 to 64 fps for one view
and 56.6 for two. The scene-only bench figure (8.5 / 11.7 ms p50 / p99 at 1920x1080,
[high-resolution cost](high-resolution-cost.md)) was right, and nothing was missing from it.
Pulse's "56 fps worst second" in the same table is a real stretch later in its race. Its
first lines read 86.5 and 109.9.

**Cause (perf on the Deck, `perf record -g --call-graph dwarf`, main thread).** On the first
audio tick of a race, `Audio::maybe_prefetch_next_race_track` needs the playlist's length.
`Audio::race_soundtrack_len` found it unknown and read the booted soundtrack's listing
synchronously on the frame thread. A PSARC entry cannot be peeked, so on HD that read
decompresses all fifteen tracks whole (about 75 MiB; the samples are `miniz_oxide` inflate,
`psarc::Directory::read_entry` and `oag_music::mp3::probe`, about 210 ms of the frame). The
length was unknown on two routes: `--race`, which every Deck measurement used, and a race
resumed from the menus (`pause_race_music` clears the context, and `start_race_music` restarts
it with no length). A race launched from the menus takes the length from its fetch worker
(`Audio::finish_race_music`), so by the code it never stalled. That route was not run on the
Deck. Every title paid this read on those two routes; on Pulse and Pure it is a WAD header
peek and cheap.

**Fix.** `oag_sound::MusicDiscs` keeps every listing it has read, shared by its clones and
by the fetch workers (`crates/sound/src/listings.rs`). An error or a missing soundtrack is not
kept, so a failed read is retried. Audio only: no simulation crate is touched.
`music_resume_ground_truth.rs` checks, on all six titles' discs, that the first tick of a
started and of a resumed race opens no source on the calling thread. With the memo switched
off, the check fails on every archived title. The PS2 read does not go through the library,
so the check cannot count it, but the memo keeps that listing too.

**Before and after on the Deck panel** (Steam Deck LCD, gamescope 1280x800 at 90 Hz, vsync,
windowed `:1` with Steam's environment over ssh, `--race --autopilot --frame-log --no-audio`,
default HD circuit, 20 s per run, base and fix interleaved, AC power, GPU 1,040 MHz while
racing, Deck load average 0.9 to 1.4; release `perf-probe` build, `-C target-cpu=znver2`, in
the AppImage container):

| Run | Frames over 40 ms | First line: fps / p99 ms | Steady fps (median / min) |
| --- | --- | --- | --- |
| base 1 | 1 (252.3 ms) | 70.3 / 31.8 | 90.0 / 90.0 |
| base 2 | 1 (251.1 ms) | 70.4 / 32.3 | 90.0 / 90.0 |
| base 3 | 1 (252.2 ms) | 70.7 / 28.8 | 90.0 / 90.0 |
| fix 1 | 0 | 85.1 / 31.8 | 90.0 / 89.4 |
| fix 2 | 0 | 85.6 / 30.7 | 90.0 / 90.0 |
| fix 3 | 0 | 85.6 / 28.5 | 90.0 / 90.0 |

The panel is vsync bound at 90 fps, so the change shows only in the first line. What remains
there (p99 about 30 ms) is the first race frame's own uploads and passes, about 30 to 50 ms
in the profile. That is a separate, smaller cost and was left alone.

**At 4K (estimated, not measured).** No TV was attached to the Deck on 2026-10-19 (`eDP-1`
only, Xwayland 1280x800), so the docked case was not re-run. The removed read is CPU work
that does not depend on the resolution, so the 4K start window should lose the same frame. In
the 2026-10-14 first line, about 490 ms of the first second sat outside `cpu_ms`, against
about 210 to 240 ms on the panel. That leaves about 250 ms unattributed. If it is a second
one-off, the 4K first line reads about 45 fps after the fix (about 45 frames of 16.7 ms in
the 750 ms left), and only if it also goes does it reach about 60 (the third line's level).
A first line in the 40s on a TV run is that remainder, not a failed fix.

**The scout's two small CPU items could not be judged from this profile.** Bind groups rebuilt
each frame in the post passes, and the empty shadow clears. The steady race's main thread was
about 43 % busy and waiting on vsync at 90 fps, so neither limits the panel. But only 64 of the
1,613 main-thread samples from 5.6 to 15 s unwound past 8 frames at `-F 400`, so the absence of
`create_bind_group` in those stacks proves nothing. Both are left open.
