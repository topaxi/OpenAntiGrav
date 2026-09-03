# DRS settles at 80 % while the frame runs at 60, because the budget is one pass

Reported from play, 2026-09-03: *"when using FSR 3 and a frame target of 120,
on HD/Fury, some races we end up at ~60 FPS and the resolution renders at ~80 %
even though the floor is set to 50 %"*, with the follow-up *"with a single race
and render scale of 150 or 200"*. The maintainer's own
`~/.config/oag/settings.toml` at the time: `[render_profiles."Wipeout HD"]`
`render_scale = 150`, `target_fps = "120"`, `minimum_resolution = 50`,
`upscaler = "fsr3"`, `anti_aliasing = "msaa4x"`, `motion_blur = "high"`;
`[display]` `vsync = "off"`, `frame_limit = "120"`.

## The floor is not the constraint, and the two numbers are in different units

The `80 %` on the `dev` overlay is `perf::RenderSize` - **extent over
allocation**. `minimum_resolution = 50` is a `display::Scale` against the
**aspect rectangle**, which `Session::drs_limits` says in its own doc comment.
At `render_scale = 150` those are not the same denominator:

- `Limits::floor_fraction` = 50/150 = **0.333**
- `policy::floor_steps` = `ceil(0.333 x 20)` = **7** of `GRID` = 20
- the observed 80 % is **16** steps

So nine grid steps sat unused below it. The controller was never clamped by the
floor - it was **satisfied**. `Target::at_most` is also ruled out: `frame_limit
= "120"` with vsync off gives `limiter_hz() = Some(120)`, and `120 > 120` is
false, so the target stands at 120 and the budget is `SCENE_SHARE / 120` =
**3.75 ms**.

## What it is satisfied by

`drs::Controller::record` is fed `Session::scene_cost`'s reading and nothing
else - the `race` pass alone, per the budget table on
[dynamic-resolution.md](../docs/rendering/dynamic-resolution.md#what-is-in-the-budget-and-what-is-not).
It stops moving the moment that one pass lands in `DEADBAND` (0.80..=0.95), i.e.
between **3.0 and 3.56 ms**. Every other pass in the frame is invisible to it:

- **untimed and scene-resolution**: the MSAA 4x resolve, `hd_bloom`,
  `motion_blur` at `high`. The docs already record these as "not yet, and still
  not";
- **timed and thrown away**: the FSR 3.1 chain. `Session::upscale_cost` exists,
  is fed every frame and reaches the overlay as `FSR3 x.xx MS` - and no path
  leads from it to `drs`;
- **fixed whatever the extent does**: FSR 3.1's `accumulate` and `rcas` are
  presentation-resolution (`post::fsr3::Pass`), as are the HUD, the composite
  and the blit.

The report's own two numbers force the split, with no reproduction needed:

- idle at 16/20 steps means the scene pass sat inside `DEADBAND`, so it was at
  most `0.95 x 3.75` = **3.5625 ms**;
- ~60 FPS means the frame was about **16.67 ms**;
- so **at least 13.1 ms of the frame is outside the budget** - the timed pass is
  at most **21 %** of the frame where `SCENE_SHARE` assumes 45 %.

At `render_scale = 150` with MSAA 4x the untimed remainder is the majority of
the frame, so the loop can be comfortable at 80 % of a 150 % allocation - which
is still **120 % of the aspect rectangle**, i.e. supersampling - while the frame
takes 16 ms. `SCENE_SHARE = 0.45` assumes an untimed:timed ratio of about
1.22:1; this profile is nowhere near that.

**Lowering `SCENE_SHARE` is the wrong lever.** One global constant cannot serve
both a Pulse profile and this one, and a large part of the remainder does not
shrink with the extent at all - so a more aggressive controller would walk to
the floor and still miss 120, which is exactly the "every pixel it gave up
would buy nothing" failure `Target::at_most` was written to prevent.

**FSR 3.1 is minifying here and is not gated on it.** `blit::magnifies` guards
FSR 1 only; `resolve_scene`'s ladder never asks it about FSR 3.1, so above 100 %
render scale the chain runs with `render > upscale`. Whether that is right is a
separate question from this one, but it is where a chunk of the untimed cost is.

## Trap: the windowed `--race` path discards the whole render profile

**Measured, not read.** Two 60-second runs of `hdfury-ps3-eu-dec.iso --race
--autopilot --upscaler fsr3 --anti-aliasing msaa4x`, NVIDIA RTX PRO 2000:

| `--render-scale` | scene pass p50 | FSR 3.1 chain readings |
| --- | --- | --- |
| 100 | 0.813 ms | **0** |
| 200 | 0.794 ms | **0** |

`Session::render_profile` returns `RenderProfile::default()` whenever
`self.shell` is `None`, and `crates/game/src/main/app.rs` line ~200 says
`--race` has no shell at all. `main.rs` applies every render-profile CLI flag by
walking `settings.render_profiles`, which that path then never reads - so
`--render-scale`, `--upscaler`, `--anti-aliasing` and `--motion-blur` are all
silently inert, and so is the settings file's own `[render_profiles."Wipeout
HD"]`. This is **the same bug the FSR 3.1 thread records as fixed for the
headless capture path**, still live one path over. It is why no `--race` run can
reproduce the report above, and it makes `just play <image> --race
--render-scale N` a measurement of nothing.

## Measured on the real profile, 2026-09-03

Read off the `dev` overlay by the maintainer during a heavy HD/Fury race, at
`render_scale = 100`, 2560x1440, MSAA 4x, motion blur high, FSR 3.1,
`target_fps = 120`:

| | |
| --- | --- |
| `GPU SCENE` | **2.4 ms** |
| `FSR3` | **2.4 ms** |
| Frame rate | 80-90 FPS (**11.1 to 12.5 ms**) |
| Render scale in force | 100 %, dipping to 95/90 |

**The controller is behaving correctly and its budget is wrong by about 2.25x.**
`2.4 / 3.75` = **0.64**, well under `*DEADBAND.start()` = 0.80, so `record`
counts the frame toward `RISE_PATIENCE` and asks to climb - it is already at
`GRID`, so nothing moves. Meanwhile the frame misses its 8.33 ms target by 2.8
to 4.2 ms. `SCENE_SHARE = 0.45` asserts the timed pass is 45 % of the frame; it
is **19 to 22 %**. The dips to 95/90 are the camera-dependent spread the
`RISE_PATIENCE` note already records, not the controller reacting to the deficit.

The split, at 85 FPS (11.76 ms):

| | ms | share |
| --- | --- | --- |
| `race` pass (timed, in the budget) | 2.4 | 20 % |
| FSR 3.1 chain (timed, **discarded**) | 2.4 | 20 % |
| MSAA 4x resolve, `hd_bloom`, `motion_blur`, HUD, composite, blit (untimed) | ~7.0 | 60 % |

Two consequences worth stating separately from the budget question:

- **At `render_scale = 100` the FSR 3.1 chain reconstructs no extra
  *resolution*, but it is not idle.** `extent == rect`, so `render == upscale`
  and `jitter::phases` returns the base 8; the 2.4 ms buys upstream's
  **native-AA mode**, which `assets/ui/menu.toml` already states is why the
  UPSCALER row carries no `fsr3` warning - it accumulates more samples per pixel
  than one frame carries. It is still **70 % of the 3.4 ms deficit**, and the
  one cost measured rather than estimated.
- **MSAA 4x on top of it is largely, not wholly, redundant.** FSR 3.1 reads the
  MSAA-*resolved* colour through `Framebuffer::perceptual`, so the samples do
  improve its input; but `INPUTS_MULTISAMPLED` takes **sample 0 only** of depth
  and velocity, so three of four samples on those two attachments are
  rasterized, stored and discarded. `resolve_scene` already skips FXAA/SMAA
  outright under FSR 3.1 - MSAA escapes the same treatment only because its
  sample count is baked into the scene pipelines at `Scene::new` and so cannot
  be a resolve-time choice.

## Open

- The maintainer's own scene/upscale split is unread. The `dev` overlay already
  prints `GPU SCENE x.xx MS  FSR3 x.xx MS`; one glance during a ~60 FPS race
  settles which of the three branches this is (scene 3.0-3.5 with a large FSR3
  figure = as diagnosed; scene well over 3.75 with no fall = a plumbing bug in
  `drs` instead; scene small with FSR3 dominating = a presentation-resolution
  floor DRS cannot touch at any scale).
- Nothing here is measured on the real profile, because of the trap above.
- Whether FSR 3.1 should run at all above 100 % render scale.
- **Decided and written up, not built**: the MSAA-under-FSR-3.1 question that
  came out of this thread is now
  [ADR-0041](../docs/architecture/adr/0041-one-row-for-what-resolves-the-frame.md) -
  one RECONSTRUCTION row (`off`/`fxaa`/`smaa`/`fsr1`/`fsr3`), MSAA on its own
  row greyed under `fsr3`. It needs a settings migration, a `menu.toml`
  rewrite and the warning tests regenerated. **It makes no frame faster** and
  is independent of the budget question below.

## Two traps in measuring any of this

**`DISPLAY=:99` does not reach Xvfb from a Wayland session.** winit prefers
Wayland whenever `WAYLAND_DISPLAY` is set, so every run above that believed it
was headless actually opened a window on the maintainer's Niri session at their
own resolution - which is also why an early note here claimed 2560x1440 when
the window was whatever Niri gave it. Use `env -u WAYLAND_DISPLAY -u
XDG_SESSION_TYPE DISPLAY=:99`, and set `XDG_CONFIG_HOME` to a scratch directory
while you are at it, or the run rewrites the maintainer's own
`settings.toml`.

**And once it is genuinely on Xvfb, the numbers are not usable for
performance.** Measured 2026-09-03 at 2560x1440: the scene pass reads 7.0 ms
against the 2.4 ms the same profile shows on the real display, the loop
manages about 6 FPS, and `--msaa off` against `--msaa 4x` and `--motion-blur
off` against `--motion-blur high` move **nothing at all** - four legs within
noise of each other. Xvfb presents in software, and a 2560x1440 copy per frame
dominates the frame and distorts the timestamp deltas around it. Xvfb is fine
for driving the game headlessly and for anything about *pixels*; it cannot size
a pass. The `dev` overlay on the real display is the instrument.

## Next Steps

1. ~~Fix the windowed `--race` profile fallback.~~ **Done**: `race::Loaded`
   carries the title `race::load` already resolved, `Session::race_title`
   carries it to `render_profile()`, and the capture path reads the file's own
   profile instead of a default. Verified by the measurement that exposed the
   bug: `--render-scale 100` now gives a 0.63 ms scene pass against 2.93 ms at
   200, where both used to give 0.80, and the FSR 3.1 chain reports 8,044
   readings where it used to report none.
2. With that, reproduce at `render_scale = 150`, MSAA 4x, motion blur high,
   FSR 3.1, target 120, and record the scene/untimed split.
3. Then decide the budget shape. The candidate is
   `(frame_period - measured_fixed_cost) * share`, with `measured_fixed_cost`
   coming from `upscale_cost` - which is already timed and currently discarded -
   and an explicit "this target is unreachable, say so rather than grinding to
   the floor" branch when it goes non-positive. That wants an ADR;
   [ADR-0040](../docs/architecture/adr/0040-the-dynamic-resolution-budget-is-a-share-of-a-frame.md)
   is the one it would supersede.
