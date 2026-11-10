use super::*;
use oag_vex::probes::{self, IRRADIANCE_SIZE as IRRADIANCE, REFLECTION_SIZE as REFLECTION, Taps};

fn device() -> Option<(wgpu::Device, wgpu::Queue)> {
    let instance = wgpu::Instance::default();
    let adapter = oag_gpu::test_adapter::request(&instance)?;
    pollster::block_on(adapter.request_device(&Default::default())).ok()
}

/// A fixed stream of words: the test's images and weights are the same on every run.
struct Stream(u32);

impl Stream {
    fn word(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        self.0
    }

    fn unit(&mut self) -> f32 {
        (self.word() >> 8) as f32 / 16_777_216.0
    }

    /// A position between two sections or across the track: mostly inside, sometimes on an
    /// edge or a hair off one, where a weight is zero or below every texel's reach.
    fn fraction(&mut self) -> f32 {
        match self.word() % 16 {
            0 => 0.0,
            1 => 1.0,
            2 => 1.0e-30,
            3 => 1.0e-42,
            4 => 0.5,
            _ => self.unit(),
        }
    }

    fn bytes(&mut self, len: usize) -> Vec<u8> {
        (0..len).map(|_| (self.word() >> 11) as u8).collect()
    }
}

struct Image {
    irradiance: Vec<u8>,
    reflection: Vec<u8>,
}

fn images(stream: &mut Stream) -> Vec<Image> {
    let sizes = (
        IRRADIANCE.0 * IRRADIANCE.1 * 4,
        REFLECTION.0 * REFLECTION.1 * 4,
    );
    (0..SLOTS)
        .map(|slot| match slot {
            0 => Image {
                irradiance: vec![0; sizes.0],
                reflection: vec![0; sizes.1],
            },
            1 => Image {
                irradiance: vec![255; sizes.0],
                reflection: vec![255; sizes.1],
            },
            _ => Image {
                irradiance: stream.bytes(sizes.0),
                reflection: stream.bytes(sizes.1),
            },
        })
        .collect()
}

/// The weights `Probes::locate` gives a craft `u` along and `v` across.
fn bilinear(u: f32, v: f32) -> [f32; 4] {
    [(1.0 - u) * (1.0 - v), u * (1.0 - v), (1.0 - u) * v, u * v]
}

/// What `mix` makes of `taps`, as the CPU blend calls it: the taps of weight zero left out.
fn mixed(images: &[Image], taps: &Taps, of: impl Fn(&Image) -> &[u8]) -> Vec<u8> {
    let used: Vec<(&[u8], f32)> = taps
        .probes
        .iter()
        .zip(&taps.weights)
        .filter(|(_, weight)| **weight > 0.0)
        .map(|(&probe, &weight)| (of(&images[probe]), weight))
        .collect();
    probes::mix(&used)
}

/// The top-left `size` texels of a layer read back at the array's size.
fn corner(layer: &[u8], size: (usize, usize)) -> Vec<u8> {
    layer
        .chunks(ENV_PROBE_SIZE.0 as usize * 4)
        .take(size.1)
        .flat_map(|row| &row[..size.0 * 4])
        .copied()
        .collect()
}

/// The first texel channel where two images differ, and how many do.
fn difference(ours: &[u8], theirs: &[u8]) -> Option<(usize, u8, u8, usize)> {
    assert_eq!(ours.len(), theirs.len());
    let mut differing = ours
        .iter()
        .zip(theirs)
        .enumerate()
        .filter(|(_, (a, b))| a != b);
    let (at, (&a, &b)) = differing.next()?;
    Some((at, a, b, 1 + differing.count()))
}

/// The pass writes the bytes `oag_vex::probes::mix` computes, on whatever adapter the test
/// runs on: every job of 40 passes over random probes, weights on and off the edges of a
/// cell, the all-black and all-white probes among them.
#[test]
fn the_pass_writes_the_bytes_the_cpu_blend_computes() {
    let Some((device, queue)) = device() else {
        eprintln!("no adapter: skipped");
        return;
    };
    let mut stream = Stream(0x9e37_79b9);
    let images = images(&mut stream);
    let blend = ProbeBlend::new(&device);
    for (slot, image) in images.iter().enumerate() {
        blend.write_source(&queue, slot as u32, &image.irradiance, &image.reflection);
    }
    let fixed = [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
        [0.5, 0.5, 0.0, 0.0],
        [0.25, 0.25, 0.25, 0.25],
        [0.0, 0.3, 0.0, 0.7],
        [0.652, 0.340, 0.0, 0.0],
        [0.1, 0.2, 0.3, 0.4],
        [1.0e-30, 1.0, 0.0, 1.0e-42],
    ];
    let mut compared = 0usize;
    for round in 0..40 {
        let taps: Vec<Taps> = (0..JOBS)
            .map(|craft| Taps {
                probes: std::array::from_fn(|_| stream.word() as usize % SLOTS as usize),
                weights: if round == 0 {
                    fixed[craft]
                } else {
                    bilinear(stream.fraction(), stream.fraction())
                },
            })
            .collect();
        let jobs: Vec<Job> = taps
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
            let layers = [
                (
                    corner(
                        &blend.read_back(&device, &queue, craft as u32 * 2),
                        IRRADIANCE,
                    ),
                    mixed(&images, taps, |image| &image.irradiance),
                ),
                (
                    blend.read_back(&device, &queue, craft as u32 * 2 + 1),
                    mixed(&images, taps, |image| &image.reflection),
                ),
            ];
            for (which, (ours, theirs)) in layers.iter().enumerate() {
                assert_eq!(
                    difference(ours, theirs),
                    None,
                    "round {round}, craft {craft}, layer {which}, {taps:?}: (first byte, gpu, cpu, count)"
                );
                compared += ours.len();
            }
        }
    }
    eprintln!("{compared} bytes compared, none differ");
}

/// A pass of fewer jobs leaves the other crafts' layers as they were.
#[test]
fn a_job_writes_its_own_crafts_layers_only() {
    let Some((device, queue)) = device() else {
        eprintln!("no adapter: skipped");
        return;
    };
    let mut stream = Stream(0x1234_5678);
    let images = images(&mut stream);
    let blend = ProbeBlend::new(&device);
    for (slot, image) in images.iter().enumerate() {
        blend.write_source(&queue, slot as u32, &image.irradiance, &image.reflection);
    }
    let job = |craft, slot| Job {
        craft,
        slots: [slot; 4],
        scales: [1.0, 0.0, 0.0, 0.0],
    };
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    blend.blend(&queue, &mut encoder, &[job(2, 5), job(6, 9)]);
    queue.submit([encoder.finish()]);
    let before = blend.read_back(&device, &queue, 2 * 2 + 1);
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    blend.blend(&queue, &mut encoder, &[job(6, 12)]);
    queue.submit([encoder.finish()]);
    assert_eq!(blend.read_back(&device, &queue, 2 * 2 + 1), before);
    let opaque = |image: &[u8]| -> Vec<u8> {
        image
            .chunks(4)
            .flat_map(|texel| [texel[0], texel[1], texel[2], 255])
            .collect()
    };
    assert_eq!(before, opaque(&images[5].reflection));
    assert_eq!(
        blend.read_back(&device, &queue, 6 * 2 + 1),
        opaque(&images[12].reflection)
    );
}
