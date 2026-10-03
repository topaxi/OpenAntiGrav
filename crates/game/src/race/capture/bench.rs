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
//! Neither touches the frame the capture writes out: both record into
//! encoders of their own.

/// Runs whichever benches the environment asks for, `record` encoding the
/// frame being measured.
pub(super) fn run(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    mut record: impl FnMut(&mut wgpu::CommandEncoder),
) {
    if let Some(runs) = runs("OAG_RENDER_BENCH") {
        let mut samples = Vec::with_capacity(runs);
        for _ in 0..runs {
            let mut encoder = encoder(device);
            let start = std::time::Instant::now();
            record(&mut encoder);
            samples.push(start.elapsed().as_secs_f64() * 1e6);
        }
        report("Scene::render CPU", &mut samples);
    }
    if let Some(runs) = runs("OAG_RENDER_GPU_BENCH") {
        let mut samples = Vec::with_capacity(runs);
        // The first tenth warms caches and clocks and is not counted.
        let warmup = runs / 10;
        for run in 0..warmup + runs {
            let mut encoder = encoder(device);
            record(&mut encoder);
            let commands = encoder.finish();
            let _ = device.poll(wgpu::PollType::wait_indefinitely());
            let start = std::time::Instant::now();
            queue.submit(Some(commands));
            let _ = device.poll(wgpu::PollType::wait_indefinitely());
            if run >= warmup {
                samples.push(start.elapsed().as_secs_f64() * 1e6);
            }
        }
        report("Scene::render GPU", &mut samples);
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
