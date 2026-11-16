//! The hull environment-probe blend on this machine's adapter: whether the compute pass
//! (`oag_render::probe_blend`) writes the bytes `oag_vex::probes::mix` computes, and what
//! the pass costs the GPU.
//!
//! ```sh
//! cargo run --release -p oag-render --example probe_blend_check            # 40 rounds
//! OAG_TEST_ADAPTER=integrated cargo run --release -p oag-render --example probe_blend_check -- 200
//! ```
//!
//! Needs no disc: the probes are random bytes, the weights the bilinear ones
//! `Probes::locate` gives, on and off the edges of a cell. It is the crate's own test
//! (`probe_blend::tests`) as a binary, for an adapter a test run does not reach (a Steam
//! Deck, a phone), with the GPU time of a full pass of eight crafts beside it. Exits 1 on
//! the first byte that differs.

use oag_render::probe_blend::{JOBS, Job, ProbeBlend, SLOTS};
use oag_vex::probes::{self, IRRADIANCE_SIZE, REFLECTION_SIZE, Taps};

struct Stream(u32);

impl Stream {
    fn word(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        self.0
    }

    fn fraction(&mut self) -> f32 {
        match self.word() % 16 {
            0 => 0.0,
            1 => 1.0,
            2 => 1.0e-30,
            3 => 1.0e-42,
            4 => 0.5,
            _ => (self.word() >> 8) as f32 / 16_777_216.0,
        }
    }

    fn bytes(&mut self, len: usize) -> Vec<u8> {
        (0..len).map(|_| (self.word() >> 11) as u8).collect()
    }
}

fn mixed(images: &[[Vec<u8>; 2]], taps: &Taps, which: usize) -> Vec<u8> {
    let used: Vec<(&[u8], f32)> = taps
        .probes
        .iter()
        .zip(&taps.weights)
        .filter(|(_, weight)| **weight > 0.0)
        .map(|(&probe, &weight)| (images[probe][which].as_slice(), weight))
        .collect();
    probes::mix(&used)
}

fn main() {
    let rounds: usize = std::env::args()
        .nth(1)
        .and_then(|arg| arg.parse().ok())
        .unwrap_or(40);
    let instance = wgpu::Instance::default();
    let adapter = oag_gpu::test_adapter::request(&instance).expect("an adapter");
    let info = adapter.get_info();
    // A timestamp between passes is what times the blend; without the feature the check
    // still runs.
    let timed = wgpu::Features::TIMESTAMP_QUERY | wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS;
    let features = adapter.features() & timed;
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        required_features: features,
        ..Default::default()
    }))
    .expect("a device");
    println!(
        "adapter: {} ({:?}, {})",
        info.name, info.backend, info.driver_info
    );

    let mut stream = Stream(0x9e37_79b9);
    let sizes = [
        IRRADIANCE_SIZE.0 * IRRADIANCE_SIZE.1 * 4,
        REFLECTION_SIZE.0 * REFLECTION_SIZE.1 * 4,
    ];
    let images: Vec<[Vec<u8>; 2]> = (0..SLOTS)
        .map(|slot| match slot {
            0 => sizes.map(|len| vec![0; len]),
            1 => sizes.map(|len| vec![255; len]),
            _ => sizes.map(|len| stream.bytes(len)),
        })
        .collect();
    let blend = ProbeBlend::new(&device);
    for (slot, image) in images.iter().enumerate() {
        blend.write_source(&queue, slot as u32, &image[0], &image[1]);
    }

    let mut compared = 0usize;
    let mut jobs = Vec::new();
    for round in 0..rounds {
        let taps: Vec<Taps> = (0..JOBS)
            .map(|_| {
                let (u, v) = (stream.fraction(), stream.fraction());
                Taps {
                    probes: std::array::from_fn(|_| stream.word() as usize % SLOTS as usize),
                    weights: [(1.0 - u) * (1.0 - v), u * (1.0 - v), (1.0 - u) * v, u * v],
                }
            })
            .collect();
        jobs = taps
            .iter()
            .enumerate()
            .map(|(craft, taps)| Job {
                craft: craft as u32,
                slots: taps.probes.map(|probe| probe as u32),
                scales: taps.scales(),
            })
            .collect();
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        blend.blend(&queue, &mut encoder, &jobs);
        queue.submit([encoder.finish()]);
        for (craft, taps) in taps.iter().enumerate() {
            let row = REFLECTION_SIZE.0 * 4;
            let corner: Vec<u8> = blend
                .read_back(&device, &queue, craft as u32 * 2)
                .chunks(row)
                .take(IRRADIANCE_SIZE.1)
                .flat_map(|line| &line[..IRRADIANCE_SIZE.0 * 4])
                .copied()
                .collect();
            let layers = [
                corner,
                blend.read_back(&device, &queue, craft as u32 * 2 + 1),
            ];
            for (which, ours) in layers.iter().enumerate() {
                let theirs = mixed(&images, taps, which);
                let differing = ours.iter().zip(&theirs).filter(|(a, b)| a != b).count();
                if differing > 0 {
                    println!(
                        "DIFFERS: round {round}, craft {craft}, layer {which}: {differing} of {} bytes, {taps:?}",
                        ours.len()
                    );
                    std::process::exit(1);
                }
                compared += ours.len();
            }
        }
    }
    println!("{compared} bytes compared over {rounds} passes of {JOBS} crafts: none differ");

    if features != timed {
        println!("no timestamp queries between passes on this adapter: the pass is not timed");
        return;
    }
    let queries = device.create_query_set(&wgpu::QuerySetDescriptor {
        label: Some("probe blend timing"),
        ty: wgpu::QueryType::Timestamp,
        count: 2,
    });
    let resolved = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("probe blend timing"),
        size: 16,
        usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let read = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("probe blend timing readback"),
        size: 16,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut micros: Vec<f64> = (0..200)
        .map(|_| {
            let mut encoder =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
            encoder.write_timestamp(&queries, 0);
            blend.blend(&queue, &mut encoder, &jobs);
            encoder.write_timestamp(&queries, 1);
            encoder.resolve_query_set(&queries, 0..2, &resolved, 0);
            encoder.copy_buffer_to_buffer(&resolved, 0, &read, 0, 16);
            queue.submit([encoder.finish()]);
            read.slice(..).map_async(wgpu::MapMode::Read, |_| {});
            device
                .poll(wgpu::PollType::wait_indefinitely())
                .expect("draining the queue");
            let ticks = {
                let mapped = read
                    .slice(..)
                    .get_mapped_range()
                    .expect("mapping the readback");
                let word =
                    |at: usize| u64::from_ne_bytes(mapped[at..at + 8].try_into().expect("8 bytes"));
                word(8).saturating_sub(word(0))
            };
            read.unmap();
            ticks as f64 * f64::from(queue.get_timestamp_period()) / 1000.0
        })
        .collect();
    micros.sort_by(f64::total_cmp);
    println!(
        "GPU time of one pass of {JOBS} crafts, 200 passes: median {:.1} us, p10 {:.1}, p90 {:.1}",
        micros[100], micros[20], micros[180]
    );
}
