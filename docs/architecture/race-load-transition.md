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

Release build, this project's development desktop (Linux, Vulkan, a 100 Hz
panel), `--no-audio`, the default circuit and settings. "Worst work" is the
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

## Still open

- **The build takes 7 to 8 s on a desktop.** It no longer freezes anything,
  but it is still the whole of the wait, and on a Deck it will be longer. Most
  of it is naga re-parsing `mesh.wgsl` and re-translating it per pipeline,
  once per model; sharing one shader module per source and caching pipelines
  by their key would shorten the load itself.
- **`warm_up` warms `Shadows::Blob` and `MotionBlur::Off` only.** A player on
  shadow maps or motion blur still compiles those pipelines on the first frame
  that uses them. No first-race-frame spike showed at the default settings.
- **The HD bar's fill is an estimate** - see
  [hd-loading.md](../formats/hd-loading.md).
