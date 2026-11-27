//! What each built-in screen filter does to colour, measured on the device: a
//! flat patch in, many frames so the persistence has settled, and the mean of
//! what comes out. A display averages in linear light, so the mean is taken
//! there and re-encoded (`docs/rendering/screen-filters.md`, "Colour audit").
//!
//! Skipped where there is no adapter.

use oag_post::screen::{Frame, Preset, Screen};

/// Frames a still picture is composited for before it is read: persistence at
/// 0.8, the largest any preset allows, is within 1e-3 of steady state here.
const SETTLE: u32 = 32;

pub(super) struct Gpu {
    device: wgpu::Device,
    queue: wgpu::Queue,
}

pub(super) fn gpu() -> Option<Gpu> {
    let instance = wgpu::Instance::default();
    let adapter = oag_gpu::test_adapter::request(&instance)?;
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("screen filter colour test"),
        ..Default::default()
    }))
    .ok()?;
    Some(Gpu { device, queue })
}

/// An RGBA8 picture.
#[derive(Clone)]
pub(super) struct Picture {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

impl Picture {
    pub fn flat(width: u32, height: u32, colour: [u8; 3]) -> Self {
        let pixels = std::iter::repeat_n(
            [colour[0], colour[1], colour[2], 255],
            (width * height) as usize,
        )
        .flatten()
        .collect();
        Self {
            width,
            height,
            pixels,
        }
    }
}

pub(super) fn preset(id: &str) -> Preset {
    let (_, source) = super::BUILT_IN
        .iter()
        .find(|(name, _)| *name == id)
        .unwrap_or_else(|| panic!("no built-in {id}"));
    Preset::parse(id, source).expect("a preset")
}

/// Composites `picture` through `preset` for [`SETTLE`] frames, and returns the
/// last two outputs (a field-alternating filter lights the other half on each).
pub(super) fn settled(
    gpu: &Gpu,
    preset: &Preset,
    picture: &Picture,
    native: (f32, f32),
) -> [Picture; 2] {
    let size = wgpu::Extent3d {
        width: picture.width,
        height: picture.height,
        depth_or_array_layers: 1,
    };
    let source = gpu.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("patch"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
        texture_binding_view_dimension: None,
    });
    gpu.queue.write_texture(
        source.as_image_copy(),
        &picture.pixels,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(picture.width * 4),
            rows_per_image: Some(picture.height),
        },
        size,
    );
    let view = source.create_view(&Default::default());
    let mut pass =
        Screen::new(&gpu.device, wgpu::TextureFormat::Rgba8Unorm, preset).expect("the pass");
    let mut outputs = Vec::new();
    for index in 0..SETTLE {
        let mut encoder = gpu.device.create_command_encoder(&Default::default());
        pass.render(
            &gpu.device,
            &gpu.queue,
            &mut encoder,
            Frame {
                source: &view,
                size: (picture.width, picture.height),
                native,
                time: 0.0,
                strength: 1.0,
                params: preset.defaults(),
            },
        );
        gpu.queue.submit(Some(encoder.finish()));
        if index + 2 >= SETTLE {
            outputs.push(read(
                gpu,
                pass.output_texture().expect("an output"),
                picture.width,
                picture.height,
            ));
        }
    }
    outputs.try_into().ok().expect("two frames")
}

fn read(gpu: &Gpu, texture: &wgpu::Texture, width: u32, height: u32) -> Picture {
    let padded = (width * 4).next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
    let buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("patch readback"),
        size: u64::from(padded) * u64::from(height),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = gpu.device.create_command_encoder(&Default::default());
    encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded),
                rows_per_image: Some(height),
            },
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
    gpu.queue.submit(Some(encoder.finish()));
    let slice = buffer.slice(..);
    slice.map_async(wgpu::MapMode::Read, |_| {});
    gpu.device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("the GPU");
    let mapped = slice.get_mapped_range().expect("the readback");
    let pixels = mapped
        .chunks(padded as usize)
        .take(height as usize)
        .flat_map(|row| row[..width as usize * 4].to_vec())
        .collect();
    drop(mapped);
    Picture {
        width,
        height,
        pixels,
    }
}

fn linear(value: u8) -> f64 {
    (f64::from(value) / 255.0).powf(2.2)
}

fn encode(value: f64) -> f64 {
    value.max(0.0).powf(1.0 / 2.2) * 255.0
}

/// The colour a display shows over `rect` (`x0, y0, x1, y1`) of the pictures:
/// the linear-light mean, encoded again, on the 0..255 scale.
pub(super) fn shown(pictures: &[Picture], rect: (u32, u32, u32, u32)) -> [f64; 3] {
    let mut sum = [0.0; 3];
    let mut count = 0.0;
    for picture in pictures {
        for y in rect.1..rect.3 {
            for x in rect.0..rect.2 {
                let at = ((y * picture.width + x) * 4) as usize;
                for channel in 0..3 {
                    sum[channel] += linear(picture.pixels[at + channel]);
                }
                count += 1.0;
            }
        }
    }
    sum.map(|total| encode(total / count))
}

/// The middle half of a picture, which a curved tube still lights fully.
pub(super) fn middle(width: u32, height: u32) -> (u32, u32, u32, u32) {
    (width / 4, height / 4, width * 3 / 4, height * 3 / 4)
}

/// Hue in degrees and HSV saturation of an encoded colour.
pub(super) fn hue_saturation(colour: [f64; 3]) -> (f64, f64) {
    let [r, g, b] = colour;
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let chroma = max - min;
    if chroma <= 1e-9 {
        return (0.0, 0.0);
    }
    let hue = if max == r {
        ((g - b) / chroma).rem_euclid(6.0)
    } else if max == g {
        (b - r) / chroma + 2.0
    } else {
        (r - g) / chroma + 4.0
    };
    (hue * 60.0, chroma / max)
}

/// Signed shortest turn from `from` to `to`, degrees.
pub(super) fn turn(from: f64, to: f64) -> f64 {
    (to - from + 180.0).rem_euclid(360.0) - 180.0
}

pub(super) const GREYS: [u8; 6] = [32, 64, 96, 128, 192, 255];

pub(super) const COLOURS: [(&str, [u8; 3]); 9] = [
    ("red", [255, 0, 0]),
    ("green", [0, 255, 0]),
    ("blue", [0, 0, 255]),
    ("yellow", [255, 255, 0]),
    ("cyan", [0, 255, 255]),
    ("magenta", [255, 0, 255]),
    ("orange", [255, 128, 0]),
    ("sky", [100, 180, 255]),
    ("skin", [224, 172, 140]),
];

/// Output widths as multiples of the native width, which is what decides how
/// many pixels a cell gets: 1280, 1440, 1920, 2560 and 3840 for the PSP's 480.
pub(super) const SCALES: [(&str, u32, u32); 5] = [
    ("1280", 8, 3),
    ("1440", 3, 1),
    ("1920", 4, 1),
    ("2560", 16, 3),
    ("3840", 8, 1),
];

/// A small native grid with a width divisible by 3, so every ratio in
/// [`SCALES`] gives a whole output size.
pub(super) const NATIVE: (f32, f32) = (48.0, 27.0);

pub(super) fn output_size(scale: (u32, u32)) -> (u32, u32) {
    (48 * scale.0 / scale.1, 27 * scale.0 / scale.1)
}

#[test]
#[ignore = "a sweep that prints the colour tables; run with OAG_SWEEP=1"]
fn print_the_colour_tables() {
    if std::env::var_os("OAG_SWEEP").is_none() {
        return;
    }
    let Some(gpu) = gpu() else {
        eprintln!("no GPU adapter; skipping");
        return;
    };
    for (id, _) in super::BUILT_IN {
        let preset = preset(id);
        for (label, num, den) in SCALES {
            let (width, height) = output_size((num, den));
            let mut line = format!("{id} @{label}\n  grey in->out(r,g,b):");
            for grey in GREYS {
                let picture = Picture::flat(width, height, [grey; 3]);
                let out = settled(&gpu, &preset, &picture, NATIVE);
                let c = shown(&out, middle(width, height));
                line += &format!(" {grey}->({:.0},{:.0},{:.0})", c[0], c[1], c[2]);
            }
            println!("{line}");
            for (name, colour) in COLOURS {
                let picture = Picture::flat(width, height, colour);
                let out = settled(&gpu, &preset, &picture, NATIVE);
                let c = shown(&out, middle(width, height));
                let input = colour.map(f64::from);
                let (h0, s0) = hue_saturation(input);
                let (h1, s1) = hue_saturation(c);
                println!(
                    "  {name:8} out=({:3.0},{:3.0},{:3.0}) hue {:+6.1} sat {:+.3}",
                    c[0],
                    c[1],
                    c[2],
                    turn(h0, h1),
                    s1 - s0
                );
            }
        }
    }
}

/// Largest spread between the channels of a grey, on the 0..255 scale.
const GREY_SPREAD: f64 = 2.0;
/// Largest turn of a colour's hue, degrees. What is left after it is the clip
/// of a bright channel under a mask that brightens (the CRT presets).
const HUE_TURN: f64 = 5.0;

/// Every grey and every colour of `id`, at every output width, against the
/// tolerances; the failures, one line each.
fn colour_failures(gpu: &Gpu, id: &str, hues: bool) -> Vec<String> {
    let preset = preset(id);
    let mut failures = Vec::new();
    for (label, num, den) in SCALES {
        let (width, height) = output_size((num, den));
        let patch = |colour: [u8; 3]| {
            let picture = Picture::flat(width, height, colour);
            shown(
                &settled(gpu, &preset, &picture, NATIVE),
                middle(width, height),
            )
        };
        for grey in GREYS {
            let out = patch([grey; 3]);
            let spread = out.iter().fold(f64::MIN, |a, b| a.max(*b))
                - out.iter().fold(f64::MAX, |a, b| a.min(*b));
            if spread > GREY_SPREAD {
                failures.push(format!("{id} @{label}: grey {grey} came out {out:.1?}"));
            }
        }
        for (name, colour) in COLOURS.iter().filter(|_| hues) {
            let out = patch(*colour);
            let turned = turn(
                hue_saturation(colour.map(f64::from)).0,
                hue_saturation(out).0,
            );
            if turned.abs() > HUE_TURN {
                failures.push(format!("{id} @{label}: {name} turned {turned:+.1} degrees"));
            }
        }
    }
    failures
}

fn assert_keeps_colour(ids: &[&str], hues: bool) {
    let Some(gpu) = gpu() else {
        eprintln!("no GPU adapter; skipping");
        return;
    };
    let failures: Vec<String> = ids
        .iter()
        .flat_map(|id| colour_failures(&gpu, id, hues))
        .collect();
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn the_psp_filters_keep_greys_neutral_and_hues_in_place() {
    assert_keeps_colour(&["psp-lcd", "psp-3000"], true);
}

#[test]
fn the_crt_television_filters_keep_greys_neutral_and_hues_in_place() {
    assert_keeps_colour(&["crt-interlaced", "crt-shadow-mask"], true);
}

/// The grille's per-channel beam width turns mixed colours (orange by 8
/// degrees), so only its greys are held here.
#[test]
fn the_aperture_grille_keeps_greys_neutral() {
    assert_keeps_colour(&["crt-aperture"], false);
}

/// A slow panel's smear is a delay, not a gain: a still picture settles to the
/// grid's own dimming, the same share of the lifted grey at every level.
#[test]
fn a_still_picture_through_the_psp_lcd_is_dimmed_evenly() {
    let Some(gpu) = gpu() else {
        eprintln!("no GPU adapter; skipping");
        return;
    };
    let preset = preset("psp-lcd");
    let lift = preset
        .params
        .iter()
        .find(|p| p.name == "black_lift")
        .map_or(0.0, |p| f64::from(p.default));
    for (label, num, den) in SCALES {
        let (width, height) = output_size((num, den));
        let shares: Vec<f64> = GREYS
            .iter()
            .map(|grey| {
                let picture = Picture::flat(width, height, [*grey; 3]);
                let out = settled(&gpu, &preset, &picture, NATIVE);
                let input = f64::from(*grey) / 255.0;
                shown(&out, middle(width, height))[1] / 255.0 / (lift + input * (1.0 - lift))
            })
            .collect();
        let most = shares.iter().fold(f64::MIN, |a, b| a.max(*b));
        let least = shares.iter().fold(f64::MAX, |a, b| a.min(*b));
        assert!(
            most - least < 0.03,
            "@{label}: share of the lifted grey per level {shares:.3?}"
        );
    }
}
