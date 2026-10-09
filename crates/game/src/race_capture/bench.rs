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
//! Neither touches the frame the capture writes out: both record into
//! encoders of their own.

use oag_gpu::timing::{Half, PassTimer};

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
