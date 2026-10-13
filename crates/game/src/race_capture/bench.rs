//! Re-running one captured frame to measure what it costs, behind
//! `perf-probe`.
//!
//! - `OAG_RENDER_BENCH=N` re-records the frame N times into throwaway
//!   encoders and reports the CPU cost of recording alone. No submission and
//!   no presentation.
//! - `OAG_RENDER_GPU_BENCH=N` records, *submits* and waits on it N times,
//!   timing submit-to-idle on the wall clock. That is device time plus one
//!   submission's fixed overhead, which an A/B on the same machine cancels -
//!   and it is the only GPU number a capture can take without a window. The
//!   adapter is whichever `graphics.renderer` in the settings file names, so
//!   point that at the GPU the question is about.
//!
//!   Where the device can write timestamps, the GPU bench also brackets the
//!   three spans `Scene::render` takes timers for - the race pass, the motion
//!   blur chain and HD's bloom chain - and reports each, so what is left of
//!   the total is the passes nothing times (the shadow maps above all).
//!
//! `OAG_RENDER_BENCH_FRAMES_PER_TICK` says how many benched frames share a tick
//! ([`TickPacer`]); read it before comparing a number with one from before the
//! shadow gather was cached per tick.
//!
//! Neither touches the frame the capture writes out: both record into
//! encoders of their own.
//!
//! **A split race is benched by [`run_split`] instead, on the same two
//! variables**, because `Scene::render_viewports` submits its own command
//! buffers and so cannot be recorded into a throwaway encoder. There
//! `OAG_RENDER_BENCH` is the CPU cost of the whole call (recording and
//! submitting, the GPU idled before each call, so it is the CPU-bound number),
//! and `OAG_RENDER_GPU_BENCH` adds the wait for the GPU, each view's scene pass
//! by timestamp, and the pipelined cost per frame (frames submitted back to back,
//! one wait at the end), which is the number a frame budget is read against.

use oag_gpu::timing::{Half, PassTimer};

/// `OAG_RENDER_BENCH_FRAMES_PER_TICK=K`: how many benched frames share one
/// simulation tick, `1` by default.
///
/// The bench re-draws one captured tick over and over, and the shadow gather is
/// cached per tick, so left alone it would hit the cache on every frame but the
/// first and report a cost no player sees. At `1` every frame is a fresh tick (a
/// 60 Hz window, and the cost before the cache); `4` is a 240 Hz window; a very
/// large number is the upper bound where the cache never misses.
pub(super) struct TickPacer {
    frames_per_tick: u64,
    frame: std::cell::Cell<u64>,
}

impl TickPacer {
    pub(super) fn from_env() -> Self {
        Self {
            frames_per_tick: std::env::var("OAG_RENDER_BENCH_FRAMES_PER_TICK")
                .ok()
                .and_then(|text| text.parse().ok())
                .filter(|&k| k > 0)
                .unwrap_or(1),
            frame: std::cell::Cell::new(0),
        }
    }

    /// Called before each benched frame: whether it starts a new tick, so the
    /// per-tick caches must forget the last one.
    pub(super) fn starts_tick(&self) -> bool {
        let frame = self.frame.get();
        self.frame.set(frame + 1);
        frame.is_multiple_of(self.frames_per_tick)
    }
}

/// The timestamp pairs one recording is handed - all `None` on the CPU bench
/// and on a device without timestamps.
#[derive(Default)]
pub(super) struct Timestamps<'a> {
    pub scene: Option<wgpu::RenderPassTimestampWrites<'a>>,
    pub blur: Option<oag_post::motion_blur::ChainTimestamps<'a>>,
    pub bloom: Option<oag_post::hd_bloom::ChainTimestamps<'a>>,
}

/// Runs whichever benches the environment asks for, `record` encoding the
/// frame being measured.
pub(super) fn run(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    mut record: impl FnMut(&mut wgpu::CommandEncoder, Timestamps<'_>),
) {
    if let Some(runs) = runs("OAG_RENDER_BENCH") {
        let mut samples = Vec::with_capacity(runs);
        for _ in 0..runs {
            let mut encoder = encoder(device);
            let start = web_time::Instant::now();
            record(&mut encoder, Timestamps::default());
            samples.push(start.elapsed().as_secs_f64() * 1e6);
        }
        report("Scene::render CPU", &mut samples);
    }
    if let Some(runs) = runs("OAG_RENDER_GPU_BENCH") {
        let mut samples = Vec::with_capacity(runs);
        let mut timers: [Option<PassTimer>; 3] =
            std::array::from_fn(|_| PassTimer::new(device, queue));
        let mut spans: [Vec<f64>; 3] = Default::default();
        let marks = oag_gpu::perfprobe::marks::Marks::new(device, queue);
        let mut groups: Vec<(&'static str, Vec<f64>, Option<u64>)> = Vec::new();
        // The first tenth warms caches and clocks and is not counted.
        let warmup = runs / 10;
        for run in 0..warmup + runs {
            let mut encoder = encoder(device);
            for timer in timers.iter_mut().flatten() {
                timer.begin(run as u64);
            }
            if let Some(marks) = &marks {
                marks.begin();
            }
            let [scene, blur, bloom] = &timers;
            record(
                &mut encoder,
                Timestamps {
                    scene: scene.as_ref().and_then(PassTimer::writes),
                    blur: blur.as_ref().and_then(|t| {
                        Some(oag_post::motion_blur::ChainTimestamps {
                            begin: t.half_writes(Half::Begin)?,
                            end: t.half_writes(Half::End)?,
                        })
                    }),
                    bloom: bloom.as_ref().and_then(|t| {
                        Some(oag_post::hd_bloom::ChainTimestamps {
                            begin: t.half_writes(Half::Begin)?,
                            end: t.half_writes(Half::End)?,
                        })
                    }),
                },
            );
            for timer in timers.iter_mut().flatten() {
                timer.resolve(&mut encoder);
            }
            if let Some(marks) = &marks {
                marks.resolve(&mut encoder);
            }
            let commands = encoder.finish();
            let _ = device.poll(wgpu::PollType::wait_indefinitely());
            let start = web_time::Instant::now();
            queue.submit(Some(commands));
            let _ = device.poll(wgpu::PollType::wait_indefinitely());
            let elapsed = start.elapsed().as_secs_f64() * 1e6;
            for (timer, span) in timers.iter_mut().zip(&mut spans) {
                // A chain that did not run this frame writes no timestamps
                // and reads back as nothing, which is reported as such.
                let reading = timer.as_mut().and_then(|t| {
                    let _ = device.poll(wgpu::PollType::wait_indefinitely());
                    t.read(device)
                });
                if run >= warmup
                    && let Some(reading) = reading
                {
                    span.push(f64::from(reading.seconds) * 1e6);
                }
            }
            if run >= warmup {
                samples.push(elapsed);
                for span in marks.as_ref().map(|m| m.read(device)).unwrap_or_default() {
                    // The same frame each time, so the count is the same
                    // each time; the last one is kept.
                    match groups.iter_mut().find(|(l, ..)| *l == span.label) {
                        Some((_, list, fragments)) => {
                            list.push(span.micros);
                            *fragments = span.fragments;
                        }
                        None => groups.push((span.label, vec![span.micros], span.fragments)),
                    }
                }
            }
        }
        report("Scene::render GPU", &mut samples);
        for (name, span) in ["race pass", "motion blur", "HD bloom"]
            .iter()
            .zip(&mut spans)
        {
            // A chain that encoded nothing - the motion blur, when the same
            // camera is recorded twice - brackets no work and reads as zero.
            if span.iter().all(|&s| s <= 0.0) {
                println!("bench {name}: not timed (did not run, or no timestamps)");
            } else {
                report(name, span);
            }
        }
        for (label, span, fragments) in &mut groups {
            report(&format!("  span / {label}"), span);
            if let Some(fragments) = fragments {
                println!("bench   span / {label}: {fragments} fragment invocation(s)");
            }
        }
    }
}

fn runs(var: &str) -> Option<usize> {
    std::env::var(var).ok()?.parse().ok().filter(|&n| n > 0)
}

fn encoder(device: &wgpu::Device) -> wgpu::CommandEncoder {
    device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("race bench"),
    })
}

fn report(what: &str, samples: &mut [f64]) {
    samples.sort_by(f64::total_cmp);
    let mean = samples.iter().sum::<f64>() / samples.len() as f64;
    println!(
        "bench {what} over {} runs: mean {mean:.1} us, median {:.1} us, p10 {:.1} us, p90 {:.1} us",
        samples.len(),
        samples[samples.len() / 2],
        samples[samples.len() / 10],
        samples[samples.len() * 9 / 10],
    );
}

/// The split-screen counterpart of [`run`]: `frame` draws the whole split race
/// (it submits its own work) and is handed one timer per view to bracket the
/// view's scene pass with.
pub(super) fn run_split(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    adapter: &wgpu::AdapterInfo,
    views: usize,
    mut frame: impl FnMut(&mut [Option<PassTimer>]),
) {
    let (cpu, gpu) = (runs("OAG_RENDER_BENCH"), runs("OAG_RENDER_GPU_BENCH"));
    if cpu.is_none() && gpu.is_none() {
        return;
    }
    println!(
        "bench split: {views} view(s) on {} ({:?}, {:?})",
        adapter.name, adapter.device_type, adapter.backend
    );
    let idle = || {
        let _ = device.poll(wgpu::PollType::wait_indefinitely());
    };
    if let Some(runs) = cpu {
        let mut samples = Vec::with_capacity(runs);
        for run in 0..runs + runs / 10 {
            idle();
            // The last run's per-phase CPU spans, when `OAG_RENDER_PERF` is set.
            let last = run + 1 == runs + runs / 10;
            if last {
                oag_gpu::perfprobe::reset();
            }
            let start = web_time::Instant::now();
            frame(&mut []);
            let micros = start.elapsed().as_secs_f64() * 1e6;
            if last {
                oag_gpu::perfprobe::report_frame(0);
            }
            if run >= runs / 10 {
                samples.push(micros);
            }
        }
        idle();
        report("render_viewports CPU (record + submit)", &mut samples);
    }
    if let Some(runs) = gpu {
        let warmup = runs / 10;
        let mut timers: Vec<Option<PassTimer>> =
            (0..views).map(|_| PassTimer::new(device, queue)).collect();
        let mut total = Vec::with_capacity(runs);
        let mut passes = Vec::with_capacity(runs);
        let mut per_view: Vec<Vec<f64>> = vec![Vec::new(); views];
        for run in 0..warmup + runs {
            idle();
            for timer in timers.iter_mut().flatten() {
                timer.begin(run as u64);
            }
            let start = web_time::Instant::now();
            frame(&mut timers);
            idle();
            let elapsed = start.elapsed().as_secs_f64() * 1e6;
            let mut sum = 0.0;
            let mut timed = 0;
            for (timer, span) in timers.iter_mut().zip(&mut per_view) {
                idle();
                if let Some(reading) = timer.as_mut().and_then(|t| t.read(device)) {
                    let micros = f64::from(reading.seconds) * 1e6;
                    sum += micros;
                    timed += 1;
                    if run >= warmup {
                        span.push(micros);
                    }
                }
            }
            if run >= warmup {
                total.push(elapsed);
                if timed == views {
                    passes.push(sum);
                }
            }
        }
        report(
            "render_viewports submit-to-idle (serialised: CPU + GPU)",
            &mut total,
        );
        if passes.is_empty() {
            println!("bench split scene passes: not timed (no timestamps on this device)");
        } else {
            report(&format!("sum of {views} scene pass(es), GPU"), &mut passes);
            for (view, span) in per_view.iter_mut().enumerate() {
                if !span.is_empty() {
                    report(&format!("  view {view} scene pass, GPU"), span);
                }
            }
        }
        // Back to back with one wait at the end: CPU recording overlaps the
        // GPU, so this is the larger of the two costs plus what neither hides.
        let mut piped = Vec::new();
        for _ in 0..5 {
            idle();
            let start = web_time::Instant::now();
            for _ in 0..runs {
                frame(&mut []);
            }
            idle();
            piped.push(start.elapsed().as_secs_f64() * 1e6 / runs as f64);
        }
        report(
            &format!("pipelined per frame ({runs} frames back to back, 5 rounds)"),
            &mut piped,
        );
    }
}
