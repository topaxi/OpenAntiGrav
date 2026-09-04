# FSR 3.1 is ported, wired and selectable; nobody has played it

The route was decided by
[ADR-0012](../docs/architecture/adr/0012-wgsl-upscalers-not-native-fidelityfx.md):
port the shaders to WGSL, drive no native SDK. Every renderer-side prerequisite
[modern-features.md](../docs/overview/modern-features.md) listed - readable
depth, per-object motion vectors, sub-pixel jitter, a UI-free scene - landed
before this thread opened, so what is left is the port itself.

The permanent record of *what* is being ported and *how it deviates* is
[docs/rendering/fsr3.md](../docs/rendering/fsr3.md). This file carries only
what is not done yet and the traps found on the way.

## Provenance, and why it is pinned

The reference is AMD's FidelityFX SDK at tag **`v1.1.4`**, commit
`c6efa6bf7f2027b3ec94f28578bb5965eabb9e55`. Nothing from it is vendored; it is
read from a scratch checkout outside the repository. The pin matters because
ADR-0012's whole mitigation for "a port is a fork" is that the WGSL stays
diffable against a named upstream, and `licences/AMD-FidelityFX-MIT.txt`
already sets the precedent by naming `v1.20210629` for FSR 1.

`v1.1.4` deliberately rather than SDK 2.x: 1.1.x is the last line that is MIT
source all the way down, with no FSR4 signed binaries anywhere in the tree.

To re-fetch it:

```sh
just fsr-reference   # into ~/.cache/oag-fsr/v1.1.4, never into the repo
```

## Traps found

- **`accumulation` and `luma_history` are render-sized, not
  presentation-sized**, despite their names, and getting that wrong produces no
  error at all - only the top-left corner of each is ever written, every read
  past it comes back zero, and the picture reads as "no history anywhere". Found
  by a readback test at pass 5, not by reading the descriptor table twice. The
  four that really are presentation-sized are `new_locks`,
  `internal_upscaled_color_1`/`_2` and the output.
- **Storable and filterable are different properties.** `R32Float` is
  baseline-storable and *not* baseline-filterable, so every intermediate some
  pass reads through the linear sampler needs `Rgba16Float` or `Rgba8Unorm`
  instead. Settle it by listing the `Sample*` callbacks the passes actually
  call - `SampleInputDepth` is declared and never called, which is the only
  reason the depth attachment's unfilterable binding is never a problem.
- **A dispatch cannot bind one texture as both a storage write and a sampled
  resource**, even when the shader ignores the read. The pyramid's level-0
  dispatch needed pointing at something other than the level it writes.
- **A dispatch is capped at 65535 workgroups per dimension**, which the
  buffer clear reached at 2880x1800 by being dispatched flat over the texel
  count. Everything walks a texture extent now. Note the shape of this and the
  MSAA one together: **both were found by playing, and both were invisible to a
  fixture small enough to be fast.** A test that runs the chain once at a real
  window size and a real anti-aliasing setting would have caught the pair.
- **MSAA changes the *type* of two inputs, and only playing it found that.**
  The scene's depth and velocity attachments are multisampled under
  `anti_aliasing = msaa4x`, which is a different WGSL binding type and a
  `create_bind_group` validation panic in the frame loop. Every test up to that
  point ran at sample count 1. `prepare_inputs` is now built twice and there is
  a test that runs both.
- **A small fixture catches size rules a realistic one never reaches.** 8x4
  render found the mip-count ceiling (a 4x2 target holds three levels, not six)
  and the resolution bug above.

- **`ffxFsr3UpscalerGetJitterOffset` is already what `oag_render::jitter`
  does.** `halton(index % phaseCount + 1, 2 or 3) - 0.5`, both axes, is
  upstream's function line for line - so `jitter::offset_pixels` turned out to
  be an accidental exact transliteration rather than a stand-in. Only the phase
  *count* was ours, and only that had to change.
- **The reconstructed-previous-depth pass scatters.** `ReconstructPrevDepth`
  writes up to four texels at reprojected positions with an `InterlockedMin`,
  so it cannot be a fragment pass and cannot avoid an atomic. See the doc page
  for why that became a storage *buffer* here rather than a storage texture.
- **Only two upstream passes use `groupshared` at all**, both of them the SPD
  pyramids. Everything else is per-pixel, which is what makes ADR-0012's
  predicted SPD substitution the *only* structural deviation rather than one of
  many.
- **Upstream ships no wave intrinsics in these headers.** `ffxWave*` appears
  nowhere under `gpu/fsr3upscaler/`, so there is no scalar-versus-wave fork to
  choose between - a worry that turned out not to exist.

## Open

### A review on 2026-09-03, and what it changed

A read of the eight passes, `groups.rs`, `resources.rs`, the call site in
`upscale::Framebuffer::resolve_scene` and both doc pages. Nothing here reopened
ADR-0012 or ADR-0020; the gamma accumulation, the WGSL port and the
no-feature-probe widening are owned cost, not findings.

**All nine findings are implemented.** What each one was and what it now is:

1. **Nothing timed the passes, and `oag_render::timing::Timer` existed.** Both
   recordings passed `timestamp_writes: None` while six other subsystems used
   the timer. `PassTimer::compute_writes` is new (wgpu spells the compute and
   render descriptors as unrelated types), the chain's single compute pass now
   carries a pair, and `Session::upscale_timer` is a **ring of its own** - a
   slot shared with the scene pass would be a frame the resolution controller
   got no reading for. It lands on the `dev` overlay as
   `GPU SCENE x.xx MS  FSR3 x.xx MS`, which also gives `Session::scene_cost` the
   reader its own doc had been claiming for it.

   **The trap inside this one is worth reading before touching it again.** The
   first attempt claimed a slot on every race frame, which is wrong for a
   reason no test would have shown: a timestamp pair that no pass writes does
   **not** resolve to zero. Its value is unspecified, the query set is not
   cleared between frames, and the ring reuses a slot every four - so a race at
   `--upscaler off` would eventually read back the pair a *previous* frame
   wrote and report `FSR3 x.xx MS` for a frame the bilinear blit resolved. A
   plausible number attributed to the wrong frame is worse on an overlay than a
   blank one, and a `seconds > 0.0` filter does not catch it. The claim is
   therefore gated on the three things `resolve_scene` itself decides with -
   `temporal.is_some()`, the row saying `fsr3`, and
   `Framebuffer::temporal_upscaler_viable` (a pipeline build failure is kept
   rather than retried, so it would otherwise claim forever). `resolve` stays
   unconditional: `PassTimer::begin`'s own documentation is that a
   claimed-and-unresolved slot never comes back, and four of those end
   measurement for the run. **Claim less, always resolve.**
2. **FXAA/SMAA ran and was discarded whenever FSR 3.1 resolved.** The behaviour
   was right - a pre-blur destroys what a temporal resolve reconstructs from,
   which is what the anti-aliasing row warns about - and the cost was not: a
   full-screen pass for a frame nothing read. The FSR 3.1 resolve is now encoded
   **above** the anti-aliasing match rather than below it, so the skip keys on
   `temporally_resolved.is_some()` - the exact answer - rather than on the
   setting, which would have dropped anti-aliasing on the never-expected path
   where the pipelines fail to build. `resolve_scene`'s doc comment claimed the
   opposite contract and the menu warning gave a reason that was not what
   happens; both are rewritten.
3. **About fifteen bind groups were rebuilt per frame at the one site in
   `post/` that bypassed `perfprobe::bind_group`.** Nothing in them is
   per-frame: `groups::Cache` holds both ping-pong parities and rebuilds only
   when the scene views change identity, when MSAA changes which build of
   `prepare_inputs` runs, or when `resize` replaces the targets. All nine sites
   now go through the probe, so `OAG_RENDER_PERF` stops reporting the
   renderer's largest producer as zero.
4. **`history_sample` computed the same four Lanczos weights four times** - 40
   `sin` per presentation pixel where 16 suffice, ~50 million redundant
   evaluations a frame at 1920x1080. `lanczos2_weights` is split out and the row
   loop is handed one `vec4<f32>`.
5. **`history_sample` fetched four of its sixteen taps twice.** The deringing
   clamp's inner 2x2 is exactly rows 1 and 2's middle taps; the min and max are
   folded in as the loop passes them. `min`/`max` are associative and exact, so
   it is the same reduction.
6. **The widening's cost is now printed.** `Fsr3::sizes` had no caller outside
   `tests.rs`; `resolve_scene` logs it at `info` once per allocation. And
   `Target::bytes` counted mip 0 only, so `spd_mips`'s five further levels went
   missing - small (0.33 MiB of 93.8 at 1080p/50 %) and wrong in the one number
   the whole widening argument is checked with.
7. **Four doc statements the code had outgrown**: both module headers still
   said nothing selected FSR 3.1; `output()` was documented as returning linear
   light; `fsr3.md` said the depth buffer was cleared with `clear_buffer` when
   `clear_buffer` writes the one value `atomicMin` must not start from; and
   `resources.rs` justified omitting `COPY_DST` by citing that same absent
   mechanism.
8. **`Fsr3::render`'s `written != wanted` guard could never skip**, because
   `frame_index` moves every frame. Deleted rather than commented: the write is
   unconditional and goes through `perfprobe::write_buffer` now.
9. **Both `accumulate.wgsl` changes are recorded as deviations** in
   `docs/rendering/fsr3.md`. This page's premise is that the WGSL stays
   diffable against upstream, and a reader diffing those two functions will find
   them rearranged - silence there would have been the same mistake as an
   undocumented arithmetic change.

**What pins each of these.** A cache hit and a rebuild produce identical pixels,
and so do a discarded anti-aliasing pass and a skipped one - so both needed an
observable built for them: `Fsr3::group_rebuilds` and
`Framebuffer::built_spatial_anti_aliasing`. The tests are
`the_bind_groups_are_built_once_and_not_once_a_frame` (beside `groups.rs`, whose
contract it is) and
`fsr3_skips_the_spatial_anti_aliasing_pass_rather_than_discarding_it`, which
carries its own control. The shader changes are covered by
`a_jittered_still_scene_converges_on_a_history_it_trusts`, which exercises
`accumulate.wgsl` over 32 real Halton phases.

**One number is measured now, and the rest are not.** A headless
`--race --presented --render-scale 50 --upscaler fsr3 --anti-aliasing fxaa` run
on `pulse-psp-usa.chd` produces a real frame with no validation error and logs

```
FSR 3.1 intermediates: 61.3 MiB (23.5 render, 1.9 half, 35.9 presentation)
```

at 1440x816 presentation - so the widening's cost is a reading rather than an
argument for the first time, and it is the *presentation* half that dominates
exactly as `Sizes` predicted. **That extent is not the one finding 6's table was
computed at**, so it confirms nothing about the 93.8-against-70.2 figure; the
two agree by area and only this one is a measurement.

The run also proves the chain survives real data end to end - 120 ticks, a
detailed frame, no validation error - but it does **not** prove finding 2's
skip, and cannot: a discarded anti-aliasing pass and a skipped one produce the
same picture, which is why that claim rests on
`fsr3_skips_the_spatial_anti_aliasing_pass_rather_than_discarding_it` calling
the real `resolve_scene` and reading `built_spatial_anti_aliasing`.

What is still unread is the **timer**: a headless capture has no overlay and no
frame loop pacing it, so `GPU SCENE / FSR3` needs somebody playing it. The
counts in findings 4 and 5 are work removed, not a profile.

### From the port itself

- **It has been captured once and never played**, and that capture now
  predates a behaviour change: a `--anti-aliasing fxaa` or `smaa` alongside
  `--upscaler fsr3` no longer runs the spatial pass at all, where before it ran
  it and discarded the result. The *picture* is the same either way - FSR 3.1
  never read that output - so the capture below still describes what FSR 3.1
  produces; what changed is that the frame no longer pays for a pass nothing
  reads. `just compare-upscalers`
  produces the three frames now and the FSR 3.1 one is a clean, artefact-free
  race frame that sits *between* bilinear and FSR 1 in apparent sharpness -
  cleaner edges than the blit, less crisp than FSR 1 and without FSR 1's
  ringing on the barrier slats. Whether that is right is a from-play judgement
  nobody has made. Three candidate explanations for the softness, in the order
  worth testing: the capture is nearly static (the craft is stationary for 240
  of its 300 ticks, which is the worst case for a temporal upscaler); RCAS runs
  at its default sharpness rather than the settings row's; and accumulation runs
  on gamma-encoded values, which biases a converging history bright in the
  shadows.
- **Accumulation runs in gamma, and upstream's runs in linear.** ADR-0020 makes
  gamma authoritative here and says nothing linearises, so there is no linear
  light to hand FSR 3.1 - see `docs/rendering/fsr3.md`. It is the *consistent*
  choice rather than the accurate one, and its visible cost is unmeasured. A
  ghost trail behind a dark object is where it would show.
- **A camera cut does not reset the history.** `Scene::record_frame` sets
  `reset` on the sequence's first frame only, so a view change or a
  respawn hands the resolve a history of a different scene. It has not been
  seen because nothing cuts the camera mid-race yet.
- **~~A reset did not clear the accumulation it reads.~~ Fixed 2026-09-04**,
  found by a static diff against upstream's `resetAccumulation` path rather
  than by playing: `Fsr3::render` forgot the previous constants and stopped
  there, where upstream also clears the accumulation SRV. Into targets that
  already ran a sequence - every race after the first, since `Fsr3` outlives
  a `Scene` - the reset frame read "fully accumulated" everywhere and blended
  a zeroed history colour in at that weight, so a restarted race opened on
  darkened frames. The port now clears the half it reads with an empty render
  pass on every frame-zero; `docs/rendering/fsr3.md` records it as a
  deviation-in-mechanism and `reset_tests.rs` pins it. **The camera-cut item
  above gets this for free** once something sets `reset` mid-race.
- **`compute_motion_divergence` divides by zero on a perfectly static camera.**
  `saturate(reprojected_velocity / velocity_4k)` with both zero is `0/0`.
  Upstream has the same expression and relies on `saturate(NaN)` returning zero;
  a race always has motion so it never bites in play, but a still fixture walks
  straight into it. Noted rather than 'fixed', because changing it would be a
  divergence nobody has measured a need for.
- **Nothing is compared against upstream's own output.** There is no reference
  implementation to diff against without building the SDK, which ADR-0012
  declined.
- **The chain has now been timed on a Steam Deck, and the widening has
  not been.** Reported from play, 2026-09-04, dynamic resolution on: the dev
  overlay's `FSR3` row read **4-5 ms**, with `BLUR` in the same ballpark
  beside it. That closes the timer half of Next Step 2 and leaves the other
  half exactly where it was - the `info` line naming the intermediates' MiB
  is written once per allocation at `info` level and nobody has read it on
  the device, which is still the number
  [goals.md](../docs/overview/goals.md)'s first tier is owed. The same
  question the dynamic-resolution thread left open about the ceiling
  allocation.

  **4-5 ms is coherent rather than anomalous, and this port's own two
  documented deviations are what predict it.** A shipped FSR2/3 lands in the
  low single-digit milliseconds on eight RDNA2 CUs at that panel; this one
  runs no FP16 path (`f32` throughout, where upstream leans on packed math
  that is double rate on that architecture) and widens every intermediate to
  the baseline-storable formats `resources.rs` picked - `R32Float` for a
  scalar upstream keeps at 16 bits, `Rgba16Float` for a two-channel pair - on
  a part whose memory is shared with the CPU. Roughly 1.5-2x a native chain
  is what those two together cost, and that is the gap.

  **The two levers, in the order worth trying**, neither of them started:
  narrow the storage formats behind a probe of
  `adapter.get_texture_format_features()` on the device's own adapter rather
  than on a feature name (the guarantees have moved since `resources.rs`
  chose the baseline, and it chose deliberately), and take the FP16 path
  behind `wgpu::Features::SHADER_F16`. Both need a per-pass breakdown first:
  the timer is one pair over the whole chain, so nothing yet says whether the
  render-resolution passes or the presentation-resolution ones dominate, and
  the two levers do not help the same passes. Read this file's finding 1
  before adding pairs - eight claims a frame is eight chances to reintroduce
  the unresolved-slot trap.
- **The fallback ladder's lowest rung cannot be exercised here.** Both adapters
  on this machine report `DownlevelFlags::COMPUTE_SHADERS`, so a green run is
  not evidence that a compute-less adapter degrades to FSR 1 rather than
  failing to boot. Same shape of hole as the missing-`TIMESTAMP_QUERY` case.

## Next Steps

1. **Play it, and read the two numbers while you do.**
   `just play --race --upscaler fsr3 --render-scale 50` with
   `[graphics] perf_overlay = dev`, and look at a *moving* frame: the capture is
   nearly static and is the wrong instrument for the one thing a temporal
   upscaler is for. Ghosting behind the craft and shimmer on the barrier slats
   are what to watch; `GPU SCENE x.xx MS  FSR3 x.xx MS` is what the chain costs,
   and the `info` line at the first race frame is what it holds. Both are wired
   and neither has been read.
2. ~~**Read the same two on a Steam Deck.**~~ Half done, 2026-09-04: the
   `FSR3` row reads **4-5 ms** there with dynamic resolution on - see Open
   above for what that number does and does not settle. **What is still
   unread is the `info` line**, which is the one the widening argument has
   been waiting on since the port opened: 93.8 MiB against upstream's 70.2 at
   1080p/50 % is the number to check against what the device actually has. It
   is logged at `info` on the first race frame of an allocation, so it wants a
   run with the log level up rather than another look at the overlay.
3. Decide whether `fsr3` should be a default anywhere. It is off by default and
   the row already offers it; `Scale::default` is `FULL`, so like `fsr1` it is
   inert until a player lowers the render scale - except that unlike `fsr1` it
   is *not* inert at 100 %, because a temporal resolve still has more samples
   than one frame carries.
4. Reset the history on a camera cut, which nothing does yet - see above.
