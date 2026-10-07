# The load-to-race transition

What the frame thread does between a player pressing `LAUNCH RACE` and the
first race frame, measured frame by frame, and why none of it may take longer
than a frame. This is engineering rather than reverse engineering: every
number here is a wall-clock measurement of this build, and every budget is
**chosen, not measured**.

## The rule

The loading screen draws every frame until the race is ready. Anything that
takes longer than a frame runs on a worker thread and is polled, never joined
while it is still running:

| Work | Thread | Handed back by |
| --- | --- | --- |
| Reading the circuit off the disc (`race::load`) | `race-load` (`race::LoadWorker`) | `LoadWorker::is_finished`, then `join` |
| Locating and decoding the race music | `race-music` (`audio::MusicFetchWorker`) | `is_finished`, then `join` at the hand-off |
| Building the race scene and warming its pipelines | `race-build` (`crate::race_build::BuildWorker`) | `BuildWorker::take`, a poll |
| Releasing a parked race's GPU resources on `LAUNCH RACE` | `race-drop` (`race_build::drop_off_thread`) | nothing - fire and forget |

**`race-load` uploads textures as it decodes them.** `LAUNCH RACE` hands the
worker the frame loop's own device and queue as a `race::TextureSink`; the
thread opens an `oag_mesh::mesh_render::TextureSinkScope` around `race::load`,
and every texture an `.rcsmodel` build decodes (Omega, HD, 2048) goes up through
`Queue::write_texture` at once and is replaced in its `Model` by a
`Texels::Uploaded` view. The scene on `race-build` then binds those views
instead of uploading, so the uploads sit on `race-load` and not `race-build`:
both are off the frame thread and the loading screen keeps drawing through
them. No wall-clock figure is recorded here for the change - the one debug,
software-rasteriser run taken was contended and this page's protocol is a
release build with `--measure-race-load 2`. Memory numbers are in [`omega-status.md`](../formats/omega-status.md#what-one-race-costs-in-memory).

`Session::advance_race_build` is the only place the frame loop touches the
build: on the frame the circuit's load lands it copies what the build reads
into a `race_build::Request` and spawns the worker, and on every later frame
it asks `take()`, which returns `None` without waiting until the build is in.

The build can run off the frame thread because nothing in a race scene is
tied to the thread that made it: `wgpu::Device` and `wgpu::Queue` are
reference-counted and `Send + Sync` in wgpu 30, `RaceStage` is `Send`, and
`build_race_stage`, `RaceStage::render`, `draw_hud` and `warm_up` take a
`GpuContext` - the device, the queue and the surface format - rather than the
window's whole `Gpu`, which also owns the window and the surface.

## Measurements

Where `race::load` itself spends its time, title by title, is
[Load time](load-time.md); this section is about the frame thread.

Release build, this project's development desktop (Linux, Vulkan; frame
intervals of about 10 ms suggest a 100 Hz panel, not checked), `--no-audio`, the default circuit and settings. "Worst work" is the
longest single frame of main-thread work: the stage update, the ticks and the
draw, including the swapchain acquire. Taken with `--measure-race-load 2`; the
second run of each pair pays no cold disc cache.

### Before (2026-09-25, `e8ef0b23`'s parent)

| Source | Worst main-thread frame | Where |
| --- | ---: | --- |
| Pulse PSP (`pulse-psp-eu.chd`) | 8,049 ms | the loading frame that built the scene |
| Pulse PS2 (`pulse-ps2-eu.chd`) | 7,914 ms | the same |
| Wipeout HD Fury (`hdfury-ps3-eu-dec.iso`) | 7,435 ms | the same |

The PS2 and HD rows were taken with the build forced back onto the frame
thread (a blocking `take`) on top of the fix, which reproduces the old path
exactly; the PSP row is the unmodified old code.

Every row is one frame: the loading screen's wave stopped dead for eight
seconds and then the fade began. Inside it, `build_race_stage` was 7.2 to 8.0 s
and `warm_up` 30 to 50 ms. A perf profile of the PSP build is almost entirely
naga - WGSL parsing, validation and SPIR-V writing - because
`mesh_render::build` creates a shader module and its pipelines per model.

Three smaller spans surfaced once that one was gone:

| Span | Main-thread time | Fix |
| --- | ---: | --- |
| First race tick: `Audio::tick` re-reading the booted disc's soundtrack table to find the next track's index | 36 to 38 ms | the race load's `MusicFetchWorker` reads it and `finish_race_music` keeps it |
| `LAUNCH RACE` after escaping a race: dropping the parked `RaceStage` | 86 ms | dropped on a `race-drop` thread |
| `launch_race` building the loading screen's own renderer | 1.4 to 5.4 ms | none needed |

### After

| Source | Run | Worst loading frame | Worst race frame | Frames over 16.7 ms |
| --- | ---: | ---: | ---: | ---: |
| Pulse PSP | 1 | 2.6 ms | 3.1 ms | 0 |
| Pulse PSP | 2 | 4.7 ms | 2.6 ms | 0 |
| Pulse PS2 | 1 | 2.3 ms | 2.9 ms | 0 |
| Pulse PS2 | 2 | 4.5 ms | 3.2 ms | 0 |
| Wipeout HD Fury | 1 | 5.5 ms | 4.6 ms | 0 |
| Wipeout HD Fury | 2 | 7.5 ms | 3.4 ms | 0 |

The build itself is unchanged - 7.2 to 8.1 s on the `race-build` thread - and
the loading screen animates through all of it. Under a heavily loaded machine
(load average 19, other builds running) a handful of loading frames reached 20
to 33 ms; they did not recur at a load average of 8, so they read as
scheduling rather than as contention with the build thread.

The second run's first loading frame shows a 90 to 100 ms *interval*: that is
the previous frame, which escaped the race back to the menus (`escape` and
`open_menus`), not the transition itself.

## Measuring it

```sh
cargo run --release -p oag-game -- data/images/pulse-psp-eu.chd \
    --no-audio --measure-race-load 2
```

`--measure-race-load [RUNS]` skips the boot sequence, presses `LAUNCH RACE`
on the first menu frame, follows each race for 180 frames, escapes and
launches the next, and prints a table per run: frames per stage, the worst
work and interval, every frame over 16.7 ms, and the named spans, including
the build's own two halves as the `race-build` thread timed them. It is
`perf::transition::Recorder` fed by `session::load_probe`; unlike the
performance overlay it keeps the frames a load stalls, because those are the
ones a player sees freeze.

On a Steam Deck, after `just deploy-deck` (see
[packaging](../tools/packaging.md#running-it-on-a-steam-deck)), from a Desktop
Mode terminal:

```sh
~/Desktop/OpenAntiGrav-x86_64-portable.AppImage \
    ~/.local/share/oag/images/pulse-psp-eu.chd --no-audio --measure-race-load 2
~/Desktop/OpenAntiGrav-x86_64-portable.AppImage \
    ~/.local/share/oag/images/hdfury-ps3-eu-dec.iso --no-audio --measure-race-load 2
```

**The Steam Deck is unmeasured.** The report that started this was the Deck's
own, and it is the machine where the build takes longest and the budget
matters most.

`race_build::tests::race_build_polls_never_wait_on_the_build` is the
disc-backed check: it builds a Pulse PSP scene on a headless device while the
test thread submits a frame's work and polls `take()` every 16 ms, and asserts
that the first poll did not return the build and that no poll took longer
than 50 ms - a bound chosen loose enough to survive `just test-data`'s
contention, against the 8 s the old path cost.

## The shader module and pipeline cache

**The build was 7.2-8.1 s and is now 195-372 ms - a 20-38x cut, on the same
desktop, same release build, same `--measure-race-load 2`.** `mesh_render::build`
runs once per drawable - once per craft, per plume, per shield, per weapon
model, some seventy calls in one race scene - and every one of them called
`device.create_shader_module` on `mesh.wgsl` from scratch and then built its
own eight-to-thirteen render pipelines, each asking wgpu to translate the
already-identical shader a second, third, eighth time with the same override
constants a sibling drawable had already asked for. The cache below counts
every one of those asks whether or not it hits, so its own "calls" total is
exactly what the unfixed code ran every single time it built a Pulse PSP
scene: 1,197 calls asking for the shader module and 9,859 asking for a render
pipeline, against 53 *distinct* pipelines the scene actually needs - every one
of the rest was the same descriptor a call before it had already built.

`oag_mesh::mesh_render::pipeline_cache` (`crates/mesh/src/mesh_render/pipeline_cache.rs`)
fixes both: a thread-local cache, opened for one thread by a `BuildCacheScope`
and cleared on drop. Inside an open scope, `build()` asks for the shared
`mesh.wgsl` module instead of parsing its own, and asks for a pipeline by a
key covering everything that can make one descriptor differ from another -
entry points, targets (format, blend, write mask, the velocity target),
`PrimitiveState`, `DepthStencilState`, `MultisampleState`, and the override
constants (`f64::to_bits`, since `f64` has no `Hash`) - reusing the
`wgpu::RenderPipeline` handle (`Clone`, proxy-`Eq` on the same underlying
handle - `crate::cmp::impl_eq_ord_hash_proxy!` in wgpu itself) whenever a call
matches one already built. The four bind group layouts a `build()` call
creates fresh every time (`layout`, `texture_layout`, `fog_layout`,
`anim_layout`) are **not** part of the key and do not need to be: they come
from fixed descriptors with no model-dependent shape, and wgpu already
deduplicates a `BindGroupLayout` - and the `PipelineLayout` built from it - by
its descriptor's own content, the fact `material_bind_group_layout`'s own doc
comment already relied on for the sun occlusion pass. A pipeline cached from
one `build()` call's layouts is exactly as valid against a later call's bind
groups as one built fresh for it would have been.

`race::Scene::new` opens the scope itself and logs what it did. It used to be
`Stage::build_race_stage` that opened it around the call, which left the
`--screenshot` path (`race/capture.rs`) with no scope at all: every one of its
~1,200 drawables (1,152 of them the weapon pools, 128 per model) compiled its
own pipelines, about 7 MiB of resident memory each, and a headless Pulse or HD
race peaked at 8.4 GiB where the same scene with the scope open peaks at about
0.58 GiB. With the scope inside `Scene::new` no caller can forget it, and
`race::tests::scene_build` fails when a pool of identical drawables stops
sharing its pipelines:

```
race scene build cache: shader 1196/1197 reused, pipeline 9806/9859 reused (53 distinct built)
```

Outside an open scope - the asset viewer, every pre-existing render test -
`build()` keeps exactly its old behaviour: a fresh module and fresh pipelines
every call, checked by
`mesh_render::pipeline_cache::tests::with_no_scope_open_every_build_creates_its_own_shader_and_pipelines`.
Two more tests in that module build the same model twice inside one scope
against a real (adapter-gated) device and assert the second call's pipeline
and shader module compare equal to the first's - `wgpu::RenderPipeline` and
`wgpu::ShaderModule` both compare by handle, not content, so a stale-cache bug
that handed back the wrong pipeline would fail these, not just look plausible.

**A weapon pool is one drawable and 127 names for it.** `race/scene/weapon_models.rs`
builds `MAX_PROJECTILES` (128) drawables per weapon model, one per pool slot,
and the first is built whole with `Drawable::new` while the other 127 come from
`Drawable::instance` (`race/drawable/instance.rs`): the same vertex and index
buffers, textures, pipelines and fog buffer (reference-counted `wgpu` handles),
plus a uniform buffer of their own for the slot's matrix, plus their own
animation buffers when the model animates, because a Plasma blast scrubs one
clock per slot. Built whole, a slot cost about 200 kB with the cache open (and
about 7 MiB without it); shared, roughly 40 kB. Headless Pulse peaks at about
340 MiB, HD at about 720, from 545 and 885 with a whole drawable per slot, and
the frames are byte-identical (`race::tests::scene_build` pins the sharing).

**Pixels checked unchanged, not just assumed:** `--race --screenshot ...
--ticks 90 --hold cross --no-audio` on Pulse PSP, Pulse PS2 and Wipeout HD
Fury, once against this commit and once against its parent (the tree
immediately before the cache), produced byte-identical PNGs on all three -
`cmp` reported no difference, all six files exactly 4,701,399 bytes. Redo it
with the images this project does not ship:

```sh
cargo build --release -p oag-game
git stash push -u -m "baseline"
cargo build --release -p oag-game
./target/release/oag-game data/images/pulse-psp-eu.chd --race \
    --screenshot /tmp/before.png --ticks 90 --hold cross --no-audio
git stash pop
cargo build --release -p oag-game
./target/release/oag-game data/images/pulse-psp-eu.chd --race \
    --screenshot /tmp/after.png --ticks 90 --hold cross --no-audio
cmp /tmp/before.png /tmp/after.png && echo identical
```

### Measurements

Same machine, same release build, same `--measure-race-load 2` as the table
above; "build" and "warm up" are the `race-build` thread's own two spans.

| Source | Run | Build | Warm up | Shader reuse | Pipeline reuse |
| --- | ---: | ---: | ---: | ---: | ---: |
| Pulse PSP | 1/2 | 372 ms | 5.0 ms | 1196/1197 | 9806/9859 (53 distinct) |
| Pulse PS2 | 1 | 224 ms | 3.1 ms | 1180/1181 | 9554/9587 (33 distinct) |
| Pulse PS2 | 2 | 195 ms | 2.8 ms | 1180/1181 | 9554/9587 (33 distinct) |
| Wipeout HD Fury | 1 | 323 ms | 6.4 ms | 939/940 | 9098/9168 (70 distinct) |
| Wipeout HD Fury | 2 | 303 ms | 6.4 ms | 939/940 | 9098/9168 (70 distinct) |

HD Fury shares fewer calls but builds more distinct pipelines than Pulse - its
materials author more of the per-model knobs the key covers (flame shading,
the absorb shell, authored blend equations), so fewer of its seventy-odd
drawables end up wanting the exact same one. It is still a 23-24x cut on the
same shape of win: almost nothing is asked for that was not already built.

**What is not yet won:** the shader module and pipeline count are now a
one-time cost of the scene's own shape, not of drawable count - a track with
more circuits-worth of distinct materials would grow the "distinct built"
column, not the reuse ratio. Nothing here changes `warm_up`'s own cost or the
`Shadows::Blob`/`MotionBlur::Off`-only warmup below, and nothing here touches
`mesh.wgsl` itself or what it draws.

## Still open

- **`warm_up` warms `Shadows::Blob` and `MotionBlur::Off` only.** A player on
  shadow maps or motion blur still compiles those pipelines on the first frame
  that uses them. No first-race-frame spike showed at the default settings.
- **The HD bar's fill is an estimate** - see
  [hd-loading.md](../formats/hd-loading.md).
