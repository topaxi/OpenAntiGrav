# The frame path allocated 1.6 MB a frame, and five of the reasons are fixed

A render-performance review of the per-frame path, 2026-09-02. Everything here
is measured on a real disc through the headless capture path, not reasoned
about: `oag-game --race --screenshot`, Talon's Junction (`16_Track`, Pulse's
`DEFAULT_TRACK`), at tick 600, in a time trial (one craft) and a full grid
(eight).

## What was measured, and how

Two instruments, both debug-only and both still in the tree - see "The
instrumentation is still here" below.

- `oag_gpu::perfprobe`, a counting global allocator plus hand-placed
  counters at every `create_bind_group`, `create_view` and `set_bind_group`
  in the frame path. `OAG_RENDER_PERF=1` prints one line a frame and a
  per-stage allocation breakdown.
- `OAG_RENDER_BENCH=N` in the capture path, which re-records the *same* frame
  N times into throwaway encoders and reports the CPU cost of `Scene::render`
  alone. No submission and no presentation: this is the command-recording half
  of a frame, which is the half every finding below is about.

**A wall-clock frame time was not available and is not claimed.** This machine
presents no window (screenshots off a presented surface come back black), so
there is no vsync, no surface acquire and no GPU-side number here. Anything
that would need one - overdraw, bandwidth, whether a depth prepass would pay -
is listed under "Not measurable here" rather than guessed at.

## Before and after

Full grid, tick 600, one frame:

| | before | after |
| --- | --- | --- |
| heap allocations | 603 | 392 |
| heap bytes | 1,614 KiB | 360 KiB |
| `create_bind_group` | 7 | 0 |
| `create_view` | 3 | 0 |
| `set_bind_group(1, ..)` issued | 821 | 678 |
| `Scene::render` CPU, median | 302.0 us | 290.6 us |

**The probe was in both columns and cost about 17 allocations a frame itself**
until `perfprobe::on()` was given a `OnceLock`: `env::var_os` allocates, and
it was asked once per `mark`. The `before` figure carries that overhead too, so
the delta above is if anything understated. Every other counter is unaffected -
they are relaxed atomics.

One craft, same track and tick: 426 -> 262 allocations, 397 KiB -> 277 KiB,
130.0 us -> 123.3 us median.

**The CPU figures are an A/B, and they are the smallest claim here.** They come
from six interleaved before/after pairs of 600 recordings each, run back to
back on an otherwise-quiet machine, against a *second* worktree at `157a4666`
carrying the same `OAG_RENDER_BENCH` block and nothing else (temporary, and
removed - recreate it with `git worktree add <dir> 157a4666`, copy
`crates/gpu/src/perfprobe.rs` and the `OAG_RENDER_BENCH` block across, and
`just link-data`) - a single-binary
before/after here drifts by more than the effect does, and the first attempt at
one produced numbers twice the real size. Mean of the six medians: **130.0 ->
123.3 us with one craft (-5.2 %) and 302.0 -> 290.6 us with a full grid
(-3.8 %)**, one pair in twelve showing no change and none showing a
regression.

Real, repeatable, and much smaller than the allocation numbers above. **The
allocation traffic is the structural result** - 1.6 MB a frame is 396 MB/s of
malloc at the default 240 fps frame limit, and it is now 86 MB/s. A CPU number
recorded on one machine with one allocator is the least portable thing in this
file; the counts are the same everywhere.

**Eleven screenshots are byte-identical against a binary built from
`157a4666`**: three tick counts x two race modes on `pulse-psp-usa`, one
Wipeout HD frame off `hdfury-ps3-eu-dec.iso`, and four ticks of an
`--autopilot --give rocket --press square` run - a craft actually driving a
lap and firing, which is what puts a moving camera, a full ribbon, live plumes
and rockets in the frame that a stationary `--race` capture never has. That is
the same bar `[graphics] frustum_culling` and `pvs_culling` had to clear before
they defaulted on, and nothing here is a picture change.

**One branch a screenshot cannot reach was checked separately.** Every capture
above has all nine of Talon's Junction's weapon pads ready, so finding 5's
cooldown branch never ran in one - the autopilot does not consume a pickup.
Forcing alternate pads to cooldown (a temporary override, since removed) with
`assert_eq!(tinted.len(), base.len())` armed at the upload produced a
*different* picture and no assertion failure: the branch reaches the reused
buffer, and the reused buffer still uploads exactly its pad's span.

`just` passes. **`just test-data` reports six pre-existing failures, and an
earlier version of this line said two** - a count taken from a run that stopped
early. Corrected 2026-09-02, and the way it was wrong is worth keeping, because
four separate wrong numbers came out of this one afternoon and **every one was
a green-looking figure produced by a run that had not exercised the thing**:

- `just test-data` is `cargo nextest run --workspace --run-ignored all` with no
  `--no-fail-fast`, so nextest stops on failure. The run behind "two" executed
  **138 of 3,298 tests** and never reached the other four. The identity of the
  two was right and their trace back to `157a4666` holds; the implied
  completeness did not. **A `test-data` run cannot enumerate what is red** -
  add `--no-fail-fast` when that is the question.
- With `--no-fail-fast` on `5e212d1b`: 3,304 run, **6 failed, 0 skipped**. None
  is in a crate this work touches, and the FSR 1 thread reproduced the same six
  independently on a clean worktree.
- **Setting `OAG_REQUIRE_GAME_DATA=1` gives 17, and the extra 11 are not code.**
  That variable turns "silently pass on missing data" into a failure, and
  `data/traces/` is *derived* evidence this checkout is missing three files of -
  `talons-junction-standing-start.csv`, `pad0-boost.csv` and
  `talons-junction-time-trial-lap-omega.csv`. The 11 are `chase_camera`,
  `maglock`, `pure_dlc`, `wall_contact` and `yaw_authority` ground truth, all
  trace-backed. Quote a red count with the variable's state attached or it means
  nothing.
- **`0 skipped` does not mean the data was there.** nextest counts a test that
  early-returns on a missing file as *passed*; the 17-failure run reported
  `0 skipped` too. It says nothing was filtered out, not that anything was
  exercised.
- **They do not pass when re-run filtered, and a report that they do is a
  fourth version of the same mistake.** With `--run-ignored all` and an
  exact-name filter, the four fail **4 of 4**, the same as in the sweep. There
  is no filtered-versus-sweep sensitivity and no shared-state hypothesis to
  chase: they fail deterministically whenever they run.

  **Two independent ways to get a false green out of a filtered re-run here,
  and the second is the one that actually happened.**

  *`nextest`'s `test()` predicate matches a test's own name and module path,
  never its binary.* Every one of these lives in a `*_ground_truth` binary
  whose name appears in no test name, so a filter written off the binary
  quietly selects a different set and reports confidently on it -
  `test(rcsmodel)` picks up `rcsmodel::tests::*` and
  `rcsmodel::vertex_decl::tests::*` unit tests, and `test(race_finish)` picks
  up exactly one unrelated unit test,
  `race::results::tests::the_board_is_taken_on_the_tick_the_race_finishes`.
  Thirty-nine tests genuinely passed; not one was a failure being re-run.
  **`binary(race_finish_ground_truth)` is the predicate that selects what the
  name suggests** - verified, it lists all three of that binary's tests.

  *And they are all `#[ignore]`d*, so even a correct exact-name filter runs
  none of them without `--run-ignored all`, in a shape that reads as success:

      Starting 0 tests across 166 binaries (3304 tests skipped)
      Summary 0 tests run: 0 passed, 3304 skipped

## Fixed

1. **The motion blur rebuilt seven bind groups every frame** - 194 allocations
   and 21.6 KiB, about half of everything the frame allocated, for seven driver
   objects that were identical to the previous frame's. `post::motion_blur`
   now keeps a `Groups` cache keyed on the three caller views plus its own
   target geometry. The key **holds the views by value**, which is what makes
   it sound rather than merely cheap: `wgpu::TextureView` compares by handle
   identity, and a clone kept in the cache keeps the old handle alive, so an
   address the cache still holds can never be recycled into a different view.
   No caller has to promise anything about when it rebuilds its attachments.

2. **`Scene::render` made its three attachment views every frame** - depth,
   velocity and the MSAA colour target, all owned by `Scene` and all moved only
   in `new`/`resize`. Now built with the textures, in `Scene::attachment_views`.
   This is also what makes finding 1's cache ever hit: a view made fresh each
   frame is never the same view twice.

3. **The engine ribbon allocated 1.18 MB a frame on a full grid.**
   `Exhaust::trail_vertices` returns a fixed 648 vertices per craft by value -
   36 KiB an allocation, eight of them - into a `Vec::new()` that then doubled
   its way from empty to 5,184 vertices, copying at every step. `Scene` now
   owns the gather buffers (`Scene::scratch`, cleared not rebuilt) and both
   ribbon builders grew an `extend_*` form that appends into the caller's list.
   The returning forms remain for the tests that use them. 1.18 MB -> 5.7 KiB.

4. **143 of 821 texture bind-group sets a frame rebound the group already
   bound.** `Drawable::draw` sets group 1 per draw call unconditionally, while
   the `set_pipeline` five lines below it already elides repeats. The same
   elision now applies, carried across all three lists - a `set_pipeline`
   between them unbinds nothing, which is the same fact that already lets
   groups 0, 2 and 3 be bound once at the top.

5. **The weapon-pad tint allocated one vertex list per pad per frame**, 85 KiB
   a frame, for a recolour that runs on every circuit on every frame.

6. **`Drawable::tint` and `Drawable::deflect_airbrakes` did the same thing** -
   a whole shield shell's vertices per visible shell, and the two airbrake
   flaps' on every frame of every race. With a shield up, the stage holding all
   three fell from 51 allocations and 20,536 bytes a frame to 48 and 2,376.

   All four now refill one `Scene::scratch` buffer rather than four: they run
   one after another inside `Scene::render` and none reads what the last wrote,
   so four high-water marks would be kept alive to save nothing. Each clears it
   itself rather than trusting what it is handed.

7. **The particle systems allocated 1.99 MB in a frame with rockets in the
   air**, and this page had them ranked as nothing - because the quiet capture
   every earlier number came from measured them at **199 bytes**, no effect
   playing. `psys::System::vertices` returned a growing pair of `Vec`s per
   playing instance and `psys::Stage::vertices` appended each pair into a third
   growing pair. Both gained an `extend_vertices` form writing into the
   caller's lists, which are `Scene::scratch`'s. 1,990,674 -> 594 bytes,
   128 -> 12 allocations.

   **The lesson is about the capture, not the code**: a stationary `--race`
   frame plays no effects, fires no weapons and raises no shield, so it reads
   as quiet on exactly the paths that cost the most when they run. Every figure
   on this page from here on is taken from `--autopilot --give <weapon>
   --press square`.

   **It is not only a measurement trap.** The FSR 1 thread found the same blind
   spot settling a *default*: the one FSR 1 versus bilinear comparison this
   project has made was taken from a stationary `--race` frame, so it contained
   no spark burst, no rocket and no shield shell - none of the high-frequency
   additive content a sharpener rings hardest on, which was the doubt the
   comparison existed to answer. Any capture-backed claim about a **picture**
   is exposed to this the same way a claim about a cost is.

## The CPU frame on an HD race, measured per phase (2026-10-07)

`OAG_RENDER_PERF=1` marks now print the microseconds since the previous mark
beside the allocation delta (`perf-probe` build). Wipeout HD, Vineta K
(`/data/environments/01_vineta_k/track.vex`), `--autopilot --ticks 600`,
960x540, release, `OAG_RENDER_BENCH=300`, load average 9-18 (contended, so
read the ratio, not the microseconds).

**The largest CPU cost was not `write_buffer`'s call count.** One mark held
1.6 ms of a 2.0 ms `Scene::render`: `Scene::write_weapon_scenes` wrote the circuit's
scene block with a direct `queue.write_buffer` (uncounted by
`perfprobe::write_buffer`, which is why the 44 counted writes a frame never
showed it) into every PS3-shaded drawable of every weapon pool, live or not -
about 600 writes and 4,200 allocations a frame on HD, with no rocket in the
air. Each weapon's own writer now calls `weapon_models::write_fog` for the
drawables it writes (the live prefix of a dense pool, the live plasma blast
slots, a live LeachBall), which are exactly the drawables the frame draws.
`write_weapon_scenes` is gone.

| `Scene::render` CPU, median, 3 interleaved pairs | before | after |
| --- | --- | --- |
| HD Vineta K | 2.02-2.04 ms | 0.355-0.360 ms |
| HD Talon's Junction (two pairs, load 23-27) | 2.07-3.46 ms | 0.31-0.41 ms |
| Pulse PSP Talon's Junction | 0.45 ms | 0.45-0.48 ms (no PS3-shaded weapon, untouched) |

Five captures are byte-identical before and after (`--autopilot`, tick 600):
Pulse still and rocket volley, HD Vineta still and rocket volley, HD Talon's
rocket volley; `hd_weapon_scene_ground_truth` passes 5 of 5.

**Still open, measured:** what is left of the 0.36 ms is spread thin (the
biggest mark is the scene pass at ~65 us, then ship anims ~50 us). The
per-drawable uniform buffer idea above is therefore worth at most the 44
counted writes a frame, tens of microseconds; not the lever it was assumed to
be. Separately, `Clip::sample_user_channel` counts presence bits one bit at a
time (`count_bits`), ~9 % of an HD run's CPU in a profile that includes the
load; it only matters where a track animates many UV curves per frame. Sim,
encode and submit were not timed per phase here: the capture path re-records
only `Scene::render`.

## Open

**Not fixed, with numbers, in the order they are worth fixing:**

- **230 KiB a frame is wgpu's own command-encoder growth** inside the race
  pass - 14 allocations, so it is amortised doubling rather than churn, and it
  is now the single largest line in the frame. Finding 4 above reduces what is
  recorded into it. Whether it can be reduced further is a question about
  reusing an encoder across frames, which this project has never tried.
- **What is left is `wgpu::Queue::write_buffer`, and it is not this code's to
  remove with another scratch buffer.** The two items that used to sit here -
  `Race::rocket_model_matrices`' per-frame `Vec` and the
  `write_anims`/`write_node_anims` pair - were both measured and both turned
  out to be the *call count*, not the arguments. `rocket_model_matrices` is
  exactly **one** allocation of the 515 its loop makes.

  Counted at three sites (`Drawable::write` and the two anim writers): **155
  `write_buffer` calls a frame**, and the allocations track them at roughly
  4 to 7 each - 126 rocket writes cost 515 allocations, 10 scenery-anim writes
  cost 69. They happen inside wgpu's staging path, not here.

  So the lever is *fewer calls*, not smaller ones: one uniform buffer with
  dynamic offsets in place of one buffer per drawable. That is an architecture
  change and wants its own thread, not a line in this one.

  **Do not read the rocket figure as a real-race cost.** It comes from
  `--give rocket --press square`, which puts **126** rockets in the air; a
  race has a handful. It is a magnifier for the per-call cost, not a
  measurement of a frame anybody plays.
- **The presentation path builds bind groups per frame, and none of it is
  measured.** `smaa::render` builds three a frame, `fxaa` and `fsr1` one each,
  and `upscale.rs`'s `fn bind` **one more** per frame whenever a post-process
  or an FSR 1 output is bound. Cite `fn bind` and not a line number: the
  callers moved when ADR-0036 split `resolve` into `resolve_scene` and
  `composite`, and `bind` is where the counter hangs off anyway. Three callers
  reach it, of which only `resolve_scene`'s two are per frame - `target` and
  `output` fire once per resize, so a resize shows as a spike rather than as
  churn. That sixth site is easy to miss: it is in
  `oag-game`, where the other five are in `oag-render`, so a sweep of the post
  chain alone does not find it. SMAA's own comment argues the rebuild is
  deliberate ("a bind group is cheap next to the draw it feeds"); the motion
  blur measurement above is the counter-evidence, at 194 allocations for seven
  groups.

  **All of it is read rather than measured**, and closing that wants two
  things, not one:

  - ~~*A counter on the `Framebuffer` pair.*~~ **Done**, on
    `worktree-handover-fsr-1s-default-is-open` rather than here, because that
    thread owned `upscale.rs` while ADR-0036 was landing: `bind` now goes
    through `perfprobe::bind_group`. One line, since all three callers funnel
    through it.
  - *A harness that presents N frames.* `--presented` on the race capture does
    drive the real path, but for **one** frame, and the question here is
    per-frame churn - a single-frame total cannot answer it. The front-end
    capture has no `Framebuffer` at all. So this wants the presented-path
    equivalent of `OAG_RENDER_BENCH`'s re-record loop, which is its own piece
    of work and belongs to neither thread yet.

    **The shape, so it is not re-derived** (the FSR 1 thread's reading, which
    owns that path): it is N presented frames from *one* process, and the
    cheapest route is a loop inside the existing capture rather than a new
    binary - `resolve_scene` and `composite` are already called from
    `race/capture.rs`, so the frame body exists and only the repetition does
    not. That is the same shape `OAG_RENDER_BENCH` already has around
    `Scene::render` in the same file.

## The GPU side, on an integrated GPU (2026-10-03)

`OAG_RENDER_GPU_BENCH=N` (beside `OAG_RENDER_BENCH`, in
`crates/game/src/race_capture/bench.rs`) records the captured frame, submits
it and waits for idle N times, timing submit-to-idle. Device time plus one
submission's overhead; an A/B on one machine cancels the overhead. The adapter
is `graphics.renderer` in the settings file - measured on the Ryzen 7900's
Raphael iGPU (RADV, 2 CUs), where fill cost is visible at all. The dGPU hides
it.

**The race pass was fragment-bound on `mesh.wgsl`'s `lit_texel`.** At
1600x900, MSAA off, swapping `lit_texel` for one albedo fetch took Pulse's pass
from 10.5 ms to 2.1 ms. The shader computed every title's every path on every
pixel and `mix`ed or `select`ed the unwanted ones away at weight zero: nine
Zone fetches, two flame, two absorb, two rim, the glow fetch, and HD's whole
authored rig (lightmap fetch, sun occlusion, half a dozen `pow`s), all for
Pulse. Each is now behind a branch on what already selected it: `zone.enabled`
and `light.enabled` (uniforms), `flame_shading` and `absorb_shading`
(overrides), slot bits 8, 11 and 12 (a flat varying - `lit_texel` carries
`@diagnostic(off, derivative_uniformity)`, and its comment says why the
derivatives stay sound).

Median `Scene::render` GPU time, start grid, `single_race`, drawn at
`--size 1600x900`, the maintainer's own render profiles (Pulse: MSAA 4x, no
motion blur; HD: MSAA 4x, motion blur `high`, which `Scene::render` encodes
inside the timed span), three interleaved pairs at load ~6. `render_scale`
does not reach this path: `--render-scale 100` and the profile's 200 measured
the same.

| step | Pulse PSP | Wipeout HD |
| --- | --- | --- |
| before | 13.7 ms | 37.2 ms |
| Zone gated | 11.5 | 32.6 |
| flame, absorb, rim gated | 9.7 | 26.4 |
| authored rig gated | 6.5 | 26.7 (rig is on; no change expected) |
| glow fetch gated | 6.6 | 24.2 |

**Ten captures were compared byte for byte at each step** (Pulse PSP still,
rocket volley, shield, Zone; Pulse PS2 still; HD still, Zone, plasma, shield,
leech beam). Every step after the first is byte-identical to the one before.
**The first step moved 2-8 pixels of 921,600 by one level on the four HD
captures that are not the still frame, and Pulse not at all.** `hd-zone` is
among them, a capture where the new branch is *taken* and the arithmetic is
the same, so the difference is the driver compiling the restructured shader
differently (contraction or scheduling on HD's float target, amplified by the
bloom), not a gated value leaking. Not chased further. Checked after the
commit against three more consumers of `lit_texel`: a 2048 race moves 3 pixels
by one level at the same step and none after it, and the Pulse and HD front
ends (backdrop, `Velocity::None` pipelines) are byte-identical.

HD split after the last step (two pairs each, before -> after): motion blur
off 32.9 -> 20.6 ms, MSAA off as well 25.0 -> 16.9 ms. **That is not the blur
chain's cost**: the chain encodes only when the camera moved, and a stationary
`--race` capture re-recording one frame never moves it. With `--autopilot` it
runs, and it is **15.9 ms at 1600x900 and 58.9 ms at 3200x1800** on HD - the
maintainer's 14 ms. `--presented --render-scale 200` is the capture that
reaches the window's 200 % scale.

**The blur chain by pass** (HD, 1600x900, MSAA 4x, `high`): prepare 0.64 ms,
tile-max x 0.46, tile-max y and neighbour-max under 0.02, **reconstruct 14.6**,
copy 0.19. Reconstruct is 15 taps of two fetches each over every moving pixel,
spread up to the reach cap - `MAX_STRETCH` of the viewport height, so at 200 %
the same taps cover four times the texels and miss cache. Scaling the tap
count with the smear's length and an opt-in half-resolution gather (BLUR
RESOLUTION) have since shipped - figures and the two failed shapes in
`docs/rendering/motion-blur.md`. Full stays the default: the maintainer found
half too grainy for hardware that does not need it.

**Split by draw group, at the maintainer's 200% scale (3200x1800, MSAA 4x,
blur off).** `perfprobe::marks` writes timestamps and fragment-invocation
counts between the race pass's draw groups; the GPU bench prints both. HD,
before the sky moved: race pass 47 ms, of which the track's opaque list 31.8
ms over 12.1 M fragments (2.1x the 5.76 M pixels), its cutout and blended
lists 2.2 ms each, the craft 2.3 ms, effects 1.1 ms, and **the sky 5.1 ms over
5.76 M fragments - every pixel, for a picture the track then covered almost
entirely**. Pulse's sky was its largest single span, 5.8 ms. HD bloom is 3.2
ms beside the pass. Anisotropy 16x against 1x is about 10 % of HD's track.

**The sky now draws after the circuit's solid lists**, at the far plane,
landing only where nothing solid did: 256 k fragments, 0.45 ms. Race pass
19.1 -> 13.8 ms on Pulse and 48.2 -> 43.7 ms on HD, ten captures and a 2048
race byte-identical, render flags pinned. See `Scene::draw_track`.

**Pin the render flags in any capture comparison.** The game rewrites the
settings file, and the maintainer turning HD's motion blur off mid-session
made every HD capture differ for a reason that had nothing to do with the
change under test.

**HD's opaque circuit list draws through a depth prepass**
(`mesh_render::Prepass`): depth first, nearest batch first, with the fragment
stage masked, then the real shading against it under `LessEqual`, in the
model's own order **reversed by draw and by triangle** (a reversed index
buffer). Under the prepass every exactly coplanar surface passes, so the last
one shaded wins; reversed, that is the first in draw order - the one plain
`Less` lets win. Track-solid fragments 13.0 M -> 7.0 M, race pass 37.9 -> 32.0
ms at 3200x1800, and **ten captures and a 2048 race byte-identical to the
picture before any reordering**.

How it got there, because two of the steps are traps: sorting the *shading*
front to back was tried first and shipped briefly (44.1 -> 37.4 ms, 1 to 419
pixels drifting at coplanar ties, accepted by the maintainer). A prepass that
reversed draws but not triangles drifted the same pixels again - ties inside
one draw. `@invariant` on the clip position changed nothing: depth was already
bit-identical across the two pipelines, since `vs_main` carries no pipeline
constants. Pulse keeps the plain pipeline: its list shades 1.2 fragments a
pixel.

**The CPU side of the frame, 2026-10-03** (`OAG_RENDER_BENCH` under `perf`,
Pulse with a rocket volley). `Scene::render` 850 -> 587 us:
`psys::System::is_running` counted every particle to answer yes or no (now
`any`); the animated circuit shine copied its whole vertex list to place its
normals, then rebuilt and re-uploaded every vertex to change eight bytes of
each - environment mapping was 20 % of the profile and `memcpy` 18 %. The
circuit shine now streams its coordinates alone
(`mesh_render::Texcoords::Streamed`). What remains of it is the arithmetic,
about 24 % of the profile and roughly 140 us a frame here; moving it into the
vertex shader would remove it at the cost of byte-exactness (GPU `normalize`
rounds differently), not done. Two traps from that change, both silent:

- **`pipeline_cache` had no vertex layout in its key.** The craft's
  interleaved shine pipeline and the circuit's streamed one were one key, and
  whichever built second drew through the first's - reading a buffer it never
  bound. The key now carries the layouts.
- **Dropping an entry from `vertex_attr_array!` shifts every later offset**,
  because the macro lays offsets out by position. It draws, just wrongly.
  `vertex_layout::streamed_attributes` filters the full table instead, and a
  test pins it.

`just build-cpu` / `just appimage-deck` (`-C target-cpu=znver2`) take another
3-6 % off; see `docs/tools/packaging.md`.

**HD's opaque track by term, 2026-10-03.** `lit_texel`'s authored rig, each
term stubbed alone and timed against the same binary (`span / track solid`,
HD single race at the start grid, 3200x1800, MSAA 4x, blur off, shadows
`original`, anisotropy 16x, three interleaved pairs, load 2-4). Base 22.5 ms,
6.96 M fragments:

| term stubbed | ms | saved |
| --- | --- | --- |
| Omega's nova curve (second `pow`) | 21.9 | 0.55 |
| specular | 21.0 | 1.5 |
| the stand-in/plain path | 20.9 | 1.6 |
| the `sun_occlusion` call | 22.1 | 0.36 |

What landed, cumulative, **16 captures byte-identical at each step** (the ten
of `data/perf/matrix.sh`, plus a 2048 race, HD with `--msaa off`, HD with
`--shadows off`, an Omega race and the HD and Pulse `main` menu pages):

| step | track solid |
| --- | --- |
| before | 22.45 ms |
| `sun_occlusion` skipped where `receives_shadow` is 2 (track, gantry) | 22.20 |
| `nova_prelit` override, 0 under a non-nova rig (`BuildCacheScope::lit_by`) | 21.64 |

Three findings that cost time:

- **Specular stays.** `Lighting.Sun specular scale` is 1.0-3.5 on every HD
  circuit with a rig (Modesto Heights and Tech de Ra light with the stand-in);
  none is 0, so there is no per-circuit fold.
- **The nova curve cannot be a runtime `select` of the `pow`'s input.** That
  moved 1-4 pixels on four HD captures, with or without the bias: the driver
  folds HD's `pow(pow(x, 2.2), power)` into one `exp2` (`log2(exp2(m))`), and a
  select between the two `pow`s blocks the fold. An override folds the select
  at compile time and keeps HD's expression as it was. **The Omega capture
  does not exercise the curve** (its race reads no lightmaps yet), so its
  guard is the live default plus `crates/render/tests/omega_nova_prelit.rs`;
  re-capture Omega once its lightmaps draw.
- **The plain path cannot be branched past byte-exactly.** Skipping it where
  `light.enabled * lit == 1` (a dynamic branch, no override needed - `lit` is
  per vertex) is another 1.2 ms (21.64 -> 20.34 ms with both folds above), and
  every capture holds except one Omega pixel, one level. Proven, not assumed:
  a base shader whose only change is `select(mix(plain, authored, w),
  authored, w == 1.0)` moves the same pixel identically. The driver's
  `mix(p, a, 1.0)` is not exactly `a`, so dropping `p` changes the bits;
  the new value is the exact one. **Declined by the maintainer 2026-10-03:**
  byte-exactness stays the bar, so do not re-propose it as a saving.

Still open: the blur at full resolution is still 11 ms at 1600x900 and 50 ms
at 3200x1800 on the iGPU; the gather's long strides at high extents are the
cost.

## Not measurable here

No window presents on the machine the 2026-09-02 numbers came from, so none of
these were tested there: overdraw and fill cost, whether the sky's
`Always`-compare full-viewport paint is worth a depth prepass, and whether
finding 4's saving is visible on an integrated GPU (it is a CPU-side saving;
the GPU sees fewer descriptor binds). `OAG_RENDER_GPU_BENCH` above now reaches
the GPU side headlessly.

## The instrumentation is still here, behind an off-by-default feature

`crates/gpu/src/perfprobe.rs` and its call sites, the `perfprobe::mark`
calls through `race/scene/frame.rs`, the `OAG_RENDER_BENCH` block in
`race/capture.rs` and the counting `#[global_allocator]` in
`crates/game/src/main.rs` are all behind **`perf-probe`, which is off**. A
default build has no allocator wrapper (`nm` finds no `perfprobe` symbol in
it), and every counter folds to nothing: they sit behind `cfg!` rather than
`#[cfg]`, so the bodies still compile with the flag off and cannot rot between
the runs that use them. The one true `#[cfg]` is the `#[global_allocator]`
static itself, which has to be absent rather than inert.

Verified both ways: with the feature the numbers above reproduce; without it
the same commands print nothing, and the eleven captures come out byte-identical
either way.

Reproduce:

```sh
OAG_RENDER_PERF=1 cargo run --release -p oag-game --features perf-probe -- \
  data/images/pulse-psp-usa.chd --race --mode single_race \
  --screenshot /tmp/race.png --ticks 600
OAG_RENDER_BENCH=300 cargo run --release -p oag-game --features perf-probe -- \
  data/images/pulse-psp-usa.chd --race --mode single_race \
  --screenshot /tmp/race.png --ticks 600
OAG_RENDER_GPU_BENCH=150 cargo run --release -p oag-game --features perf-probe -- \
  data/images/pulse-psp-eu.chd --no-audio --race --mode single_race \
  --screenshot /tmp/race.png --ticks 600 --size 1600x900
```

**`just` does not build with the feature on**, deliberately - a second clippy
pass on every commit to cover one `#[cfg]`'d static is not worth the minutes.
Run `cargo clippy -p oag-game --all-targets --features perf-probe` by hand
after touching anything in this list.

## Next Steps

Ranked by measured cost on the Raphael iGPU, HD at the maintainer's 200 %
scale (3200x1800, MSAA 4x) unless stated. Measure with `OAG_RENDER_GPU_BENCH`
plus `--autopilot` (a still camera never runs the blur), `--presented
--render-scale` for scaled extents, render flags pinned on the command line.

1. **Full-resolution motion blur, 50 ms.** The gather's taps stride up to
   `MAX_STRETCH` of the extent, so at 200 % they miss cache. Candidates: a
   compute gather that stages each tile's neighbourhood in workgroup memory;
   or gathering at presentation rather than render resolution when
   `render_scale` > 100. Exactness is not the bar here - the blur is this
   project's own effect - but the maintainer judges grain (half was too much).
2. **HD's opaque track, 21.6 ms after the folds above.** The per-title
   terms are folded (sun occlusion, nova); specular is live on every circuit.
   The plain-path branch (1.2 ms) was declined for moving one Omega pixel -
   see "HD's opaque track by term". What is left is the rig's remaining
   `pow`s (the atlas and albedo sRGB decodes, the specular exponent, the
   encode), which are the picture rather than dead
   terms, so any further saving costs byte-exactness.
3. **Defaults for weak GPUs.** 200 % scale plus MSAA 4x is 16 samples a
   displayed pixel. An integrated or Deck-class adapter could start from a
   cheaper profile; `target_fps` dynamic resolution already exists to lean on.
   A product decision first - ask.
4. **The velocity target is written when nothing reads it** (blur off and no
   temporal reconstruction): a 4x MSAA `Rg16Float` attachment cleared and
   stored every frame. Unmeasured; pure bandwidth on a shared-memory iGPU.
5. **On a real Steam Deck**: `just appimage-deck`, the perf overlay, the same
   circuits. Every number above is the 2-CU iGPU standing in for the Deck's
   8-CU one; the maintainer has the hardware.

Later, smaller: HD bloom (3.2 ms) and HD shadow passes (about 2.4 ms); the
shine's texture coordinates on the GPU (about 140 us of CPU, costs
byte-exactness); a harness that presents N frames so the AA chain's
per-frame bind groups get a number; one uniform buffer with dynamic offsets
if the `write_buffer` call count ever matters.
