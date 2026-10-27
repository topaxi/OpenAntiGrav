---
status: ready-to-wire
titles: [all]
updated: 2026-10-27
summary: "CPU-bound devices: GPU-driven draws, bundles, GPU particles/trails/shadow rays; governor landed on, step self-check open"
categories: [rendering, tooling]
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

1. **GPU-driven drawing** (opus). Indirect multi-draw per pass with a compute pass culling (PVS, frustum,
   LOD) into the draw list; per-draw state in storage buffers so one bind per pass replaces per-draw
   rebinding. The biggest lever on the measured cost. Fallback: today's per-draw path.
2. **Render bundles for static scenery and the circuit** (sonnet). Record once, replay per frame
   (`wgpu-levers-for-per-view-cpu-none-used-yet.md` in this directory). Measure against lane 1: if lane 1
   already removes the cost, close this one with its number.
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

## Open: the governor does not undo a step that saved nothing (2026-10-27, `governor-unlocked`)

`oag_present::governor` records what each shed saved (`saving`, `governor.rs` around line 179) but uses it only
to decide when a restore fits (line 202). It never reverts a step whose saving was about zero, and it keeps
shedding the next knob. The maintainer wants a step that did not help on this device undone and skipped for
the rest of the session. The Deck logs show the need: on HD Anulpha (unlocked, limit 165) the sequence
`shadows original` (CPU 7.35 ms), `model detail high` (6.55), `model detail original` (6.87 to 6.90)
reads CPU 7.2 ms at level 2 against 6.9 at level 1, so the third step bought nothing, and the knob table in
`docs/rendering/cpu-governor.md` already measures `lod`/`texture_detail` as no saving. To build: when the
cooldown ends and the measured saving is under a floor (chosen, for example 0.05 ms or 1 % of the period,
above the 0.1 ms run-to-run noise), restore that level and mark the knob skipped until the session ends
(a `[bool; MAX_LEVEL]`), so the next shed jumps over it; tests on the state machine. Measure with
`ab.sh 3 60 anulpha` as in the doc: the Deck, `ENABLE_GAMESCOPE_WSI=0`, `vsync off`, `frame_limit 165`,
governor on against off; the target is the same fps with fewer knobs stepped down.

## Next Steps

1. Lane 6 (governor) landed and is on by default; its self-check is the open item above.
2. Lanes 1 to 5 open the next drive in order, each with its Deck before/after.
