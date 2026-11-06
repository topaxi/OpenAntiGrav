---
status: ready-to-wire
titles: [all]
updated: 2026-11-06
summary: "CPU-bound devices: GPU-driven draws, bundles, GPU particles and trails; governor is on by default, undoes a useless step, skips per track"
categories: [rendering, tooling]
size: XL (35)
difficulty: medium (30)
---

# CPU-bound devices: move work to the GPU, and govern the CPU

Queued by the maintainer on 2026-10-27, after the Deck lane (`docs/rendering/frame-thread-cpu.md`, "On the
Steam Deck") brought HD Anulpha Pass to 90 fps by cutting wgpu's per-pass, per-binding and per-draw
bookkeeping in `encoder.finish()`. That bookkeeping, not arithmetic, is what a CPU-bound device pays for: about
54 passes a frame, each tracked per live buffer and texture, and every draw validated. The plan below
removes CPU work by letting the GPU do it, and adds a CPU-side quality governor beside the GPU-side dynamic
resolution controller (`crates/present/src/drs`), which does nothing while the GPU is idle.

## Rules for every lane here

- **One path where the device supports it, a CPU fallback where it does not.** Choose the path at startup
  from adapter features and limits (indirect draws, `multi_draw_indirect`, compute, storage buffers), not
  per frame. WebGL and older Android GPUs keep today's path. The fallback is the current code, kept working
  and tested, not a second implementation.
- **Same picture.** Captures stay byte-identical or the lane says exactly what moved and why; HD and Pulse
  compare pairs re-rendered (`just compare-rerender-ours`) not worse.
- **No simulation change.** Render side only; determinism hashes untouched.
- **Acceptance on the Deck.** Before/after `frame-phases` totals and `submit` on HD Anulpha Pass in a
  moving race (recipe in `the-decks-60-fps-is-anulpha-pass-submit-cost.md` in this directory), plus HD and
  Pulse default circuits; workstation numbers as corroboration. A lane that does not move the Deck number
  reports that honestly.
- **At most one lane in `crates/mesh` at a time**; these run in order.

## Lanes, in order

**Order amended 2026-10-27 (`prepare-view-breakdown`).** On the Deck, `prepare_view` (3.45 ms of 8.2) is
uniform builds and CPU image blends, not culling: culling, LOD and sorting are under 0.05 ms on Anulpha,
HD default and Pulse (`docs/rendering/frame-thread-cpu.md`, "What prepare_view spends on the Deck").
Before lane 1 step 2 (compute culling), do two CPU lanes: the hull environment-probe blend (about
0.9 ms on Anulpha) and scenery animation sampling (0.3 to 0.4 ms, every title). Measure with the
frame-thread profile recipe in that section.

1. **GPU-driven drawing** (opus). Indirect multi-draw per pass with a compute pass culling (PVS, frustum,
   LOD) into the draw list; per-draw state in storage buffers so one bind per pass replaces per-draw
   rebinding. The biggest lever on the measured cost. Fallback: today's per-draw path.
   **Step 1 measured and dropped (2026-10-27, `gpu-driven-submit`)**, numbers in
   `docs/rendering/frame-thread-cpu.md` ("Indirect multi-draw on the Deck"):
   - HD's prepass depth half as one indirect multi-draw (505 draws on Anulpha) gave the same
     picture, byte for byte.
   - On the Deck with the split finish, submit got worse, 3.14 to 3.25 ms. With
     `OAG_SPLIT_FINISH=0` it got better, 4.51 to 4.41.
   - Why: the saving lands on the finish worker, and wgpu's indirect validation adds a staging
     buffer and a bind group per pass on the frame thread.
   - The shaded draws cannot go indirect without a non-uniform material index, which costs
     6.5 to 16% GPU on ANV and NVIDIA.
   - So per-draw `finish` cuts in the race and shadow passes do not move the Deck. The frame
     thread's own work does: encoder drops in `maintain` (0.8 ms), `prepare_view` (3.4 ms), the
     post chain and the `write_buffer` calls.
   - **Parked, not dropped** (maintainer): the working implementation (feature check, `OAG_INDIRECT`
     switch, tests, byte-identical proof) is on the pushed branch `park/indirect-depth-prepass`,
     rebased on main at the time. Later steps (compute culling, colour passes) start from it.
   - **Re-measured under an interleaved split, colour half prototyped (2026-11-04, lane
     `indirect`)**, numbers in `docs/rendering/frame-thread-cpu.md` ("Re-measured under an
     interleaved split"). The port to main (two depth runs per view, by cull mode) and a
     measure-only colour half (`OAG_INDIRECT=colour`, slot as first instance, non-uniform
     index) are byte-identical on HD, Pure and Pulse. Pure and Pulse circuits take the depth
     path too: they have a prepass. In the shipped split (about 0.3 ms resolvable), depth
     alone is inside the noise. The colour step is 0.08 to 0.30 ms under depth alone, at or
     under the floor. The combination is +0.17 ms worse than none at 800p. The colour arm
     costs 2 to 3% GPU on the Deck, so fps drops at 4K. Not worth shipping as built: on this
     path the saving and the non-uniform index come together. The commits are `aa54e2afa`
     (port) and `fc967bd1a` (colour), on the lane's branch.
     Measure: the doc section's commands, `OAG_INDIRECT` in `0 1 colour`.
   - **Re-measure on Omega** (maintainer's idea, unmeasured): Omega's PS4 scenes may carry many more
     draws per pass than HD's, where the per-draw saving could outweigh the indirect validation's
     fixed per-pass cost again. First count draws per pass on an Omega race (the perf overlay's
     DRAWS line), then A/B the parked branch there, on the workstation and the Deck where Omega runs.
2. **Render bundles for static scenery and the circuit** (sonnet). Record once, replay per frame
   (`wgpu-levers-for-per-view-cpu-none-used-yet.md` in this directory). Measure against lane 1: if lane 1
   already removes the cost, close this one with its number.
   **Expect lane 1's result**: a bundle also cuts the race pass's `finish`, which the split finish
   already moved to the worker. Before building it, check on the Deck whether the frame thread
   waits on that worker for the race pass (`perf --tid`, the recipe in the doc section above).
3. **Particles on the GPU** (sonnet). Particle state in a storage buffer, advanced or extrapolated in the
   vertex shader (or compute) with one alpha uniform, so `.pob` effects, mist and clouds also draw between
   ticks. Shares its design with item 2 of `drawing-between-ticks-what-is-still-stepped.md`; land them
   as one. Spawns stay per tick, as in the originals. Fallback: CPU per tick.
4. **Trail ribbons in the vertex shader** (sonnet). Build the ribbon from a sample ring in the shader at
   fractional age (item 1 of the between-ticks thread). Fallback: CPU vertices.
5. **The `original` shadow tier's floor rays on the GPU** (sonnet). Project hull vertices against a height
   or depth map in a shader instead of one ray per vertex per tick (`docs/rendering/between-ticks.md`,
   about 0.4 ms a tick on Pulse's grid). Fallback: the CPU gather.
6. **A CPU-side governor** (sonnet; independent of `crates/mesh`, may run in parallel). Read the frame
   thread's CPU time (`frame-phases`) beside the GPU time DRS already reads; when the CPU is the bound side
   and over budget, turn a small ordered set of CPU-heavy knobs, then restore them with hysteresis. Knobs
   may be **only our own extras and quality tiers** (e.g. split-finish on, particle budget of our
   additions, LOD distance tiers, shadow tier above `original`), never what the original authored. Logged
   per change; a setting to turn it off; tests on the controller's state machine. Ties into
   `graphics-settings-resolve-from-intent-and-context.md` (blocked on the maintainer) only by not
   pre-empting it: the governor overrides at runtime, never rewrites saved settings.

## Open from `prepare-view` (2026-11-05)

Numbers in `docs/rendering/frame-thread-cpu.md`, "What is left of the frame thread on the Deck".

- **The frame thread's system time**: 1.0 ms a frame at one view, 1.6 ms at two, on HD 800p, invisible
  to a user-space profile (`perf_event_paranoid` 2, no sudo). Unattributed: command submissions, futex
  waits on the finish channels, the scoped thread per later view per frame. Read it with a kernel
  profile. Measure: `/proc/<pid>/task/<tid>/stat` fields 14 and 15 over 8 s of race.
- **Why reclaiming on another core costs one view `scene` +0.25 and the acquire +0.15**: not the snatch
  lock, the schedule, the sibling thread, glibc's arenas or the power budget
  (`docs/rendering/frame-thread-cpu.md`, "What the reaper costs one view, and what it is not"). One view
  now leaves it to the next submit, a trade: HD gains, Pulse and Pure lose a little `total` and p99. If
  the cause turns up, the reaper can come back at one view. Measure: `OAG_REAP_THREAD=0/1`, HD 800p.
- **A split's one submission, what is left** (`docs/rendering/split-screen.md`, "What is left"): the
  frame's submit now drops most of the encoders, 10 % of the frame thread at two views against 2.1 %
  before, because the reaper catches fewer; polling every 250 us read Pulse -0.10, Pure -0.11, HD
  -0.06 at `present` +0.04, not kept. `Log::encode` is 3.3 to 3.6 %: a mapped write with no second
  copy, or the log handed to a finishing thread. A view's scene pass is one `finish` that nothing
  splits, which is what HD's frame thread still waits for. Measure: two views, 800p, Deck,
  `OAG_SPLIT_SUBMIT=views` as the base arm.
- **Pulse's `write_shine`**: still 10.0 % of the frame thread in a split. Left: the first view's craft
  maps (3.4 %), which upload a whole `GpuVertex` to change eight bytes; a streamed coordinate buffer as
  the circuit's pass has would cut the copy. Measure: frame pointer build, Pulse two views, 800p.
- **Why the Deck ran 4 % faster for one session** on the same binary, 2026-11-05 22:26 to 23:10. Until
  known, compare only inside a session. Candidates: battery against docked and charging, whose change just after 23:07 matches the fast session's end (one same-binary run on each would settle it); in the slow session `steamwebhelper` read 19 % of a
  core and gamescope and Xwayland 10 % each. Measure: per-process CPU from `top` during one run.

## Also open from the Deck lane (cheap, any time)

- Fewer post-chain passes: 10 bloom downsample and 7 motion-blur passes a frame.
- Shared 1x1 fallback textures: 1,276 of the 1,863 live textures on Anulpha.
- Three upstreamable wgpu-core changes (skip init-action registration for fully initialised textures; a
  sparse usage scope; skip re-merging a bind group already merged in the pass). Upstream, not a fork.

## Low priority: measure, never ship (maintainer, 2026-10-27)

- What `InstanceFlags::VALIDATION_INDIRECT_CALL` costs once lane 1 issues indirect draws. It is the only wgpu
  validation flag on in release (`InstanceFlags::from_build_config`) and adds a GPU pre-pass sanitising indirect
  arguments. A measurement only: the maintainer's rule is no unsafe wgpu paths, so it stays on. Our instance
  (`crates/game/src/adapter.rs`, `InstanceDescriptor::new_without_display_handle`) does not read
  `WGPU_VALIDATION_INDIRECT_CALL`, so the A/B needs a local build that clears the flag, not an env var.

## Material data in one buffer, without indirect (maintainer question, 2026-10-27)

Worth deciding on numbers at the next drive: count `set_bind_group` calls per frame on the Deck (Anulpha)
and their share of the frame thread after the 2026-10-27 cuts. If still large, one material buffer
bound once per pass cuts wgpu's per-bind merge and init checks even with ordinary draws, eases render
bundles (lane 2) and unifies HD's and the psp2 path's layouts. Pass the per-draw entry as a uniform
dynamic offset (uniform index, works everywhere) rather than `first_instance` (non-uniform index,
+6.5-16 % GPU on ANV and NVIDIA, patchy on WebGL). About 3-5 days; indirect becomes a small step on
top of `park/indirect-depth-prepass` afterwards.

**Checked in the wgpu 30.0.1 / naga 30.0.1 sources (2026-10-27):**
- `Features::SHADER_DRAW_INDEX` exists (`@builtin(draw_index)`, `enable draw_index;`): Vulkan and GLES only,
  not DX12 or Metal yet, native only. But naga's uniformity analysis treats every builtin argument except
  the workgroup ones as non-uniform (`valid/analyzer.rs`, `Expression::FunctionArgument`), so indexing a
  binding array with it still gets the SPIR-V `NonUniform` decoration. It is no fix for the GPU penalty, and
  a fragment shader receives any per-draw index as a flat varying, which is non-uniform anyway.
- `Features::IMMEDIATES` (push constants, `var<immediate>`, `RenderPass::set_immediates`): DX12, Vulkan,
  Metal, OpenGL (emulated with uniforms); WebGPU is a proposal, not in browsers yet. A material index read
  from an immediate is a uniform load, so no `NonUniform` and no penalty, and `set_immediates` touches no
  tracked resource. **This is the per-draw index to use** with ordinary draws; the web build falls back to a
  uniform dynamic offset.
- Indirect stays non-uniform either way (no per-draw immediates inside one multi-draw), so the colour passes
  on indirect still need the per-material bucketing.
- `Features::MULTI_DRAW_INDIRECT_COUNT` (DX12, Vulkan 1.2+) also guarantees `multi_draw_indexed_indirect` is
  not emulated as a series of `draw_indexed_indirect`; without it a backend may emulate it. Check the Deck's
  adapter reports it before trusting a multi-draw number.

## Next Steps

1. Lane 6 (governor) landed, on by default (maintainer, 2026-10-27), and undoes a step that saved nothing and remembers skips per title, track and mode for the launch ([`cpu-governor.md`](../../docs/rendering/cpu-governor.md)).
2. Lanes 1 to 5 open the next drive in order, each with its Deck before/after.
