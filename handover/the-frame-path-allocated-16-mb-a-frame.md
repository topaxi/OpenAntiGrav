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
  fourth version of the same mistake.** Every one of these is `#[ignore]`d, so
  a filtered re-run *without* `--run-ignored all` runs **none of them** and
  says so in a way that reads as success:

      $ cargo nextest run --workspace -E 'test(a_single_race_ends_when_the_player_completes_its_laps) or ...'
      Starting 0 tests across 166 binaries (3304 tests skipped)
      Summary 0 tests run: 0 passed, 3304 skipped

  Filter by file or by substring instead of by exact name and the non-ignored
  tests in those binaries report green on their own. With `--run-ignored all`
  the same four filtered fail **4 of 4**, the same as in the sweep. So there is
  no filtered-versus-sweep sensitivity to explain, and no shared-state
  hypothesis needed: they fail deterministically whenever they actually run.

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

## Not measurable here

No window presents on this machine, so none of these were tested and none are
claimed either way: overdraw and fill cost, whether the sky's `Always`-compare
full-viewport paint is worth a depth prepass, GPU-side cost of the six motion
blur passes, and whether finding 4's saving is visible on an integrated GPU (it
is a CPU-side saving; the GPU sees fewer descriptor binds).

## The instrumentation is still here, behind an off-by-default feature

`crates/render/src/perfprobe.rs` and its call sites, the `perfprobe::mark`
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
```

**`just` does not build with the feature on**, deliberately - a second clippy
pass on every commit to cover one `#[cfg]`'d static is not worth the minutes.
Run `cargo clippy -p oag-game --all-targets --features perf-probe` by hand
after touching anything in this list.

## Next Steps

1. A harness that presents N frames, so the AA chain's and `fn bind`'s
   per-frame bind groups get a number rather than a reading. The counters are
   all placed now; see the Open item for why `--presented` alone is not that
   harness.
2. If the `write_buffer` call count is worth attacking, it wants its own
   thread: one uniform buffer with dynamic offsets rather than one per
   drawable. Nothing smaller will move it.
