# The frame path allocated 1.6 MB a frame, and five of the reasons are fixed

A render-performance review of the per-frame path, 2026-09-02. Everything here
is measured on a real disc through the headless capture path, not reasoned
about: `oag-game --race --screenshot`, Talon's Junction (`16_Track`, Pulse's
`DEFAULT_TRACK`), at tick 600, in a time trial (one craft) and a full grid
(eight).

## What was measured, and how

Two instruments, both debug-only and both still in the tree - see "The
instrumentation is still here" below.

- `oag_render::perfprobe`, a counting global allocator plus hand-placed
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
`crates/render/src/perfprobe.rs` and the `OAG_RENDER_BENCH` block across, and
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

`just` passes. `just test-data` fails two `oag-formats` ground-truth tests
(`a_third_of_a_circuits_chunks_are_see_through`,
`the_disc_declares_more_widths_than_the_search_looks_for`) - **both reproduce
on pristine `157a4666`** and neither is in a crate this work touches.

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
   a frame, for a recolour that runs on every circuit on every frame. Refilled
   from `Scene::scratch` now.

## Open

**Not fixed, with numbers, in the order they are worth fixing:**

- **230 KiB a frame is wgpu's own command-encoder growth** inside the race
  pass - 14 allocations, so it is amortised doubling rather than churn, and it
  is now the single largest line in the frame. Finding 4 above reduces what is
  recorded into it. Whether it can be reduced further is a question about
  reusing an encoder across frames, which this project has never tried.
- **`Drawable::tint` rebuilds a shield shell's whole vertex buffer per visible
  shell per frame** - the same shape as finding 5 and not fixed only because a
  shell up is the uncommon case. Fold it into `Scene::scratch` next.
- **`Race::rocket_model_matrices` returns a `Vec` per frame**, inside the
  ~50 KiB `rockets+plumes` stage.
- **The per-drawable `write_anims`/`write_node_anims` pair costs 70
  allocations and 29 KiB a frame** across five drawables. Unread further.
- **`smaa::render` builds three bind groups a frame and `fxaa`/`fsr1` one
  each.** SMAA's own comment argues the rebuild is deliberate ("a bind group
  is cheap next to the draw it feeds"); the motion blur measurement above is
  the counter-evidence, at 194 allocations for seven groups. The AA chain runs
  in the presentation path, which the capture harness above does not reach, so
  **these were not measured** - only read. Instrumenting the presented path is
  the next step there.

**A stale comment**: `race/scene/frame.rs`'s `render` doc says `cull` is
"off by default"; `settings::default_frustum_culling` returns `true` and has
since the measurement in `Graphics::frustum_culling`'s own doc comment landed.

## Not measurable here

No window presents on this machine, so none of these were tested and none are
claimed either way: overdraw and fill cost, whether the sky's `Always`-compare
full-viewport paint is worth a depth prepass, GPU-side cost of the six motion
blur passes, and whether finding 4's saving is visible on an integrated GPU (it
is a CPU-side saving; the GPU sees fewer descriptor binds).

## The instrumentation is still here

`crates/render/src/perfprobe.rs` and its call sites, the `perfprobe::mark`
calls through `race/scene/frame.rs`, the `OAG_RENDER_BENCH` block in
`race/capture.rs`, and the `#[global_allocator]` in `crates/game/src/main.rs`
are **debug code, not for merge as-is**. Every one is marked in its own doc
comment. They are worth keeping until the Open list above is empty, because
every number in this file came out of them and re-deriving them costs an
afternoon.

Reproduce:

```sh
OAG_RENDER_PERF=1 cargo run --release -p oag-game -- \
  data/images/pulse-psp-usa.chd --race --mode single_race \
  --screenshot /tmp/race.png --ticks 600
OAG_RENDER_BENCH=300 cargo run --release -p oag-game -- \
  data/images/pulse-psp-usa.chd --race --mode single_race \
  --screenshot /tmp/race.png --ticks 600
```

## Next Steps

1. Fold `Drawable::tint` into `Scene::scratch`, the way `tint_weapon_pads`
   already is.
2. Instrument the presented path so the AA chain's per-frame bind groups get a
   number rather than a reading.
3. Fix the stale "off by default" in `race/scene/frame.rs`.
