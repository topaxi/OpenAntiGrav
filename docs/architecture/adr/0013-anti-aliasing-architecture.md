# ADR-0013: Separate anti-aliasing by class, and gate spatial passes against the upscaler by render scale

## Status

Accepted.

MSAA, FXAA and SMAA are built behind `[graphics] anti_aliasing`. Temporal
anti-aliasing (TAA) and FSR 3's own temporal reconstruction are not; this
records the shape they will take when they land and why there is no row for
them yet.

## Context

A player asks for "anti-aliasing" as if it were one dial, but three genuinely
different techniques answer to that name in this renderer, at three different
points in the frame:

- **Rasterization AA** - MSAA. The rasterizer itself supersamples triangle
  edges and resolves to a single sample before anything downstream sees the
  picture. Native wgpu support, no extra fullscreen pass, cannot smooth
  shader-computed aliasing (specular fireflies, alpha-tested cutouts past the
  edge test) the way a post-process pass can.
- **Spatial post-process AA** - FXAA and SMAA. Both read one *finished* frame
  and smooth it: FXAA with one lightweight edge-following blur, SMAA with
  edge detection, a precomputed-lookup blend-weight pass and a neighbourhood
  blend. Neither needs anything the renderer does not already have. FXAA is
  the cheaper and softer of the two; SMAA keeps more detail at the cost of
  three passes instead of one. Neither has been profiled on real hardware
  yet - see [Consequences](#consequences).
- **Temporal AA** - TAA proper, and FSR 3's own reconstruction. Both need
  motion vectors, camera jitter and a history buffer accumulated across
  frames - infrastructure this renderer does not have yet. [ADR-0012](0012-wgsl-upscalers-not-native-fidelityfx.md)
  already anticipated this gap for FSR 3.1's upscaling half; the anti-aliasing
  half needs the same jitter and history.

Treating these as one ordered quality slider - as if MSAA 4x-that-does-not-work
were the same kind of failure as FXAA-being-a-bit-soft - hides that they
compose differently. MSAA resolves *before* post-processing runs at all: it
changes what triangle edges look like, not what a later pass reads. FXAA and
SMAA read the same resolved pixels an upscaler does, and an upscaler that is
itself edge-adaptive - FSR 1's EASU - can end up reasoning about edges a
spatial pass has already blurred. Temporal AA and FSR 3's reconstruction are
coupled by construction: FSR 3 produces its anti-aliasing as a side effect of
upscaling, using the same history buffer, so "FSR 3 upscaling without its
AA" is not a configuration that exists upstream.

### What the render-scale tiers mean for FXAA/SMAA

FSR 1 only actually resamples when it is asked to magnify - `upscale::magnifies`
already declines to run it otherwise, and the UPSCALER row's own warning is
pinned to exactly the render scales where that is true (below 100%, i.e. 50
and 75 of `Scale::OFFERED`). Below that render scale, FXAA or SMAA running
before EASU blurs the low-resolution scene EASU is about to reconstruct edges
from - fighting the very kernel that reasons about them, for the same reason
the module docs at `oag_post` warn against feeding EASU a linear-light
image instead of the perceptual one it expects. At 100% and above, FSR 1 does
nothing regardless of whether it is selected, so there is nothing to fight.

At the render scale's *top* end - supersampling above 100%, up to `Scale::OFFERED`'s
200% ceiling - a spatial pass is not fighting anything, but it is doing
increasingly little: supersampling is itself a (much more expensive) way of
removing the aliasing FXAA and SMAA target. 200% specifically is enough
oversampling that a spatial pass on top reads as redundant work rather than a
correctness problem, unlike the sub-100% case above. That is a quality
judgement rather than a broken combination, so it does not get the same
`warn_when` treatment as the FSR 1 conflict - see
[Consequences](#consequences) for why this is recorded rather than enforced.

MSAA is not gated against the upscaler at all. It resolves before FSR 1 ever
runs, so by the time EASU reads the scene, MSAA has already finished being
MSAA - there is nothing left downstream for it to fight.

## Decision

**Keep the three classes as three separate mechanisms, one setting
(`[graphics] anti_aliasing`: `off`, `fxaa`, `smaa`, `msaa4x`), and gate only
what genuinely conflicts.**

- **MSAA ships at 4x only, not 2x** - `sample_count: 2` fails device
  validation on every adapter this game runs on, not a hardware-variance
  question a capability probe could route around; see
  [Consequences](#consequences). MSAA multisamples the race scene's colour
  and depth attachments and resolves into the caller's target at the end of
  the scene's one render pass
  (`race::Scene::render`, `oag_mesh::mesh_render::build`,
  `oag_render::exhaust`, `oag_render::sparks`). The sample count is baked into
  every scene pipeline when it is built, so - unlike `render_scale` and
  `upscaler`, which a frame reads fresh - a change here takes effect **the
  next time a race starts**, not the frame it was chosen. This reuses the
  `Restart`/`Menu::in_effect` mechanism `graphics.renderer` established in
  the menus, told from what the current race's `Scene` was actually built
  with rather than from the device - a race is rebuilt far more often than
  the process restarts, so the note is scoped to a race rather than to the
  whole game. Ungated against the upscaler, per the context above.
- **FXAA** is one WGSL fullscreen pass, ported from the published algorithm
  description rather than transliterated from a specific author's shader
  text, to keep its licensing unambiguous the way [ADR-0012](0012-wgsl-upscalers-not-native-fidelityfx.md)
  requires for a transliteration. See `oag_post::fxaa`.
- **SMAA** is ported from `iryoku/smaa` (MIT, no attribution required even in
  binary form) the same way FSR 1 was ported from `ffx_fsr1.h` - three WGSL
  passes (luma edge detection, blending weight calculation against the
  precomputed area/search lookup textures, neighbourhood blending) plus the
  upstream lookup textures themselves, embedded as binary data with the
  licence reproduced under `licences/` and a header naming the upstream
  repository on every ported file. See `oag_post::smaa`.
- **Both FXAA and SMAA run after MSAA's resolve (if any) and before the
  upscaler**, reading the same offscreen target FSR 1 reads - the "Post
  Processing" stage in the pipeline below. This is the reverse of a generic
  post-process-AA diagram, which usually draws it after upscaling; here it has
  to run first because the conflict this ADR gates against *is* running a
  spatial pass before the upscaler reads the scene - so if FXAA/SMAA ran
  after FSR 1 instead, the two would never be in a position to fight and the
  whole gating warning above would be describing a combination that cannot
  happen. Both are wired into `upscale::Framebuffer::resolve`, the same
  function FSR 1 already lives in, immediately before its own upscale step.
- **Whatever is already composited into the offscreen target when
  `Framebuffer::resolve` runs gets touched by FXAA/SMAA, HUD and performance
  overlay included.** The HUD and the performance overlay draw into the same
  offscreen target the scene does, before `resolve` is ever called - see
  `RaceStage::render` in `crates/game/src/main.rs` - so there is no
  scene-only surface left by the time a post-process pass could intercept it
  without holding a HUD draw back until after post-processing runs, which
  today's per-frame ordering does not do. This is not a new problem: FSR 1
  already resamples the whole composited frame, HUD included, and
  `Upscaler::Off`'s own doc comment already records the doubt this raises for
  glyph and paletted-sprite content. FXAA and SMAA inherit exactly that
  tradeoff rather than introducing a second one.
- **Mutually exclusive by construction**, not by a rule enforced elsewhere:
  `anti_aliasing` is one enum, so `off`/`fxaa`/`smaa`/`msaa4x` cannot be
  selected in combination. FXAA and SMAA cannot run together, and neither
  can run alongside TAA once TAA exists, without a deliberate change to this
  enum.
- **No `Taa` variant yet.** The infrastructure it needs - motion vectors,
  jitter, a history buffer - is not built, and [`docs/overview/roadmap.md`](../../overview/roadmap.md)
  should be checked before assuming otherwise. A row for a mode that does
  literally nothing is worse than no row at all - see `graphics.upscale_sharpness`'s
  `disabled_by` for the project's existing view on rows that could mislead,
  and the commit that added a warning for "a setting stored but doing
  nothing" (59b4c52).
- **FXAA and SMAA are warned, not disabled, against FSR 1 magnifying.** A
  player can still select the combination; the row says why it is pointless,
  the same as the UPSCALER row already does against RENDER SCALE. See
  `anti_aliasing_warns_against_the_upscaler_at_exactly_the_scales_it_fights_it_at`
  in `crates/game/src/menu.rs`, pinned against `upscale::magnifies` the same
  way the UPSCALER row's own warning is.

```text
                ┌──────────────┐
                │ Scene Render │
                └──────┬───────┘
                       │
              ┌────────▼────────┐
              │ Optional MSAA   │   (resolves here; nothing downstream sees
              └────────┬────────┘    more than one sample again)
                       │
                HUD / overlay        still render scale - drawn into the same
                 composite           offscreen target, before `resolve` runs
                       │             (see the bullet above)
              ┌────────▼────────┐
              │ Post Processing │   FXAA or SMAA, mutually exclusive, before
              └────────┬────────┘   the upscaler for the reason above
                       │
              ┌────────▼────────┐
              │    Upscaler     │   (FSR 1 today; off unless render_scale < 100)
              └────────┬────────┘
                       │
                  Present
```

Note this puts HUD compositing *before* FXAA/SMAA, not after - the reverse of
what a from-scratch design would choose. It falls out of the existing
render-scale architecture (see the bullet above), not a decision made by this
ADR.

### The future temporal shape

When FSR 3's temporal reconstruction lands, it will need to disable FXAA,
SMAA and MSAA's post-resolve role - not because they are incompatible in
principle, but because FSR 3 produces its own anti-aliasing as a side effect
of its own history buffer, and running a second spatial pass on top of a
temporally-reconstructed frame fights the same way FXAA/SMAA-before-FSR-1
does today.

**When that lands, do not expose `Taa` and `Fsr3` as separate user-facing
concepts unless a real reason turns up to.** From a player's seat they are
one family - temporal reconstruction, native or upscaling - and the cleaner
model is a `TemporalAntiAliasing` enum of its own:

```text
Temporal AA:
  Off
  Native Temporal AA
  FSR3 Native AA
  FSR3 Quality
  FSR3 Balanced
```

with `effective_temporal = true` and the resolved reconstruction mode read
off that one enum, rather than a `Taa` boolean crossed with a separate
`Upscaler::Fsr3`. This ADR does not build that enum - there is no FSR 3
infrastructure to hang it off yet - but records the shape so the eventual
change is an addition to this document's successor rather than a rediscovery.

## Alternatives considered

**One ordered quality enum (`off < fxaa < smaa < msaa4x < taa`), implying
each step strictly improves on the last.** Rejected: it is not true.
MSAA and a spatial pass fix different artefacts (geometric edges versus
shader-computed aliasing) and are not substitutes for each other; a player
who wants both is a real, coherent request this ADR's exclusivity refuses for
now, but "strictly ordered" would have refused it while also lying about why.

**Offer `Msaa2x` and fall back to `sample_count: 4` when the device rejects
2.** Rejected: the row would then say "2x" and quietly deliver "4x" - the
player picks the cheaper setting believing they got it, and gets the more
expensive one instead. That is worse than not offering the row: a wrong
answer, not a missing one. See the Consequences bullet on why `Msaa2x` is not
built at all.

**Disable FXAA/SMAA outright (`disabled_by`) rather than warn, when FSR 1 is
magnifying.** Rejected on the same grounds `graphics.upscale_sharpness` was:
this is a *combination* that does nothing useful, not a row that is
individually meaningless - the player can still want it (screenshot
comparison, a driver where the conflict matters less), and greying it says
"you cannot", which is false.

**Enforce the >150%/200% render-scale redundancy as a second live warning.**
The menu definition's `Entry::warning` field is a single `Option<Warning>`
today, and `Warning.all` is a conjunction - it can express "FXAA/SMAA select
*and* FSR 1 is active *and* render scale is low", but not a second,
independently-triggered warning ("*or* render scale is 200") without either
overloading one message to cover two different failure modes (imprecise, and
this codebase has already rejected an imprecise upscaler warning once) or
restructuring `warning` into a list. Recorded here as the documented product
policy and left for a follow-up that touches the menu definition type
generally, rather than shipped as a warning whose condition does not match
its message.

## Consequences

- **One row with two different menu behaviours.** `off`, `fxaa` and `smaa`
  are read fresh every frame by `upscale::Framebuffer::resolve`, the same as
  `upscaler` is - moving among the three takes effect the frame it changes.
  Moving to or from `msaa4x` does not, because MSAA's sample count is baked
  into every scene pipeline at build time - the same constraint FSR 1's own
  pipeline is under, except FSR 1's builds lazily on first use rather than
  needing every *other* pipeline in the scene to agree with it. A player
  moving the row to or from `msaa4x` mid-race sees no effect until the next
  `Launch Game`, and `Session::open_menus` computes which case applies from
  `race::Scene::anti_aliasing`'s own sample count rather than an exact value
  match, so switching among the three live modes does not trip a false
  restart note.
- **MSAA ships at 4x only.** `Msaa2x` was built, tested on a real adapter and
  removed within this same session -
  `crates/render/tests/msaa_resolve.rs`'s
  `msaa_2x_fails_validation_without_the_adapter_specific_format_features_device_feature`
  reproduces it: even on hardware the adapter itself reports as 2x-capable,
  `wgpu::Device::create_render_pipeline` at `sample_count: 2` fails
  validation, because `wgpu::DeviceDescriptor::default()` - what
  `Gpu::bring_up` and `capture.rs` both request - does not ask for
  `wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES`, and without it
  the WebGPU spec guarantees only `[1, 4]` samples for a format like this
  renderer's `Rgba8Unorm`/`Bgra8UnormSrgb` targets. This is not
  hardware-variance a capability probe could route around - it fails on every
  adapter this game runs on, because nothing requests the feature that would
  unlock it. Requesting that feature would fix this properly, but it is a
  first-of-its-kind device feature negotiation in this codebase, needed at
  two call sites, and it loosens format validation renderer-wide in ways
  nothing here has audited - its own decision, not a patch at the tail of
  this one.
- **SMAA is specifically upstream's MEDIUM preset.** Diagonal pattern search
  and corner detection are both off, matching `SMAA_PRESET_MEDIUM`'s own
  defines rather than a reduced port of HIGH/ULTRA - see `smaa.wgsl`'s module
  docs. This is a real quality ceiling next to the HIGH preset upstream also
  ships, not an invented shortcut: diagonal near-45-degree edges get the same
  horizontal/vertical treatment a corner does rather than their own search.
- **SMAA does not soften a perfectly straight axis-aligned edge, and that is
  correct, not a gap.** Its blending weight comes from `smaa_area`, which
  returns zero unless the search finds a crossing edge - a corner - within
  the search radius. A straight one-pixel transition has no corner anywhere,
  so SMAA (correctly) leaves it alone; FXAA does not make this distinction
  and blurs it regardless. This is a real quality difference between the two,
  not a bug in either - see `smaa.rs`'s test for the staircase pattern this
  needs to actually exercise the blend.
- **SMAA's relative cost against FXAA is not measured.** "Three passes
  instead of one" above is a structural fact, not a profiled number - no
  frame-time comparison between the two has been run on real hardware. A
  claim like "roughly Nx the cost" belongs here once one has, not before.
- **The >150% render-scale guidance is unenforced.** It is real product
  policy, recorded above, and it is not a `warn_when` in `menu.toml` today -
  see Alternatives. A player at 200% render scale with SMAA selected gets no
  in-menu nudge that it is doing little; only documentation says so.
- **No forward compatibility promised for the eventual `TemporalAntiAliasing`
  enum.** The sketch above is not built, has no serialised representation
  yet, and nothing here fixes its exact variants - only the shape (one family,
  not two crossed booleans).
