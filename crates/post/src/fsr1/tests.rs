//! What the FSR1 passes in [`super`] are asserted to do: the EASU and RCAS
//! constants against upstream's, and both passes drawn on a real device.
//!
//! Its own file rather than a `#[cfg(test)]` block at the end of
//! `fsr1.rs`: the tests are 410 lines, past the 200 an inline test module
//! may hold. See `scripts/check-file-size.py`, which is the rule as a gate.

use super::*;

#[test]
fn the_easu_constants_are_upstream_s_for_a_two_times_upscale() {
    let c = Constants::new((960, 540), (960, 540), (1920, 1080), Sharpness::DEFAULT);
    // Output pixel to input pixel is exactly one half in both axes, and the
    // half-texel offset upstream subtracts is -0.25 at this ratio.
    assert_eq!(c.con0, [0.5, 0.5, -0.25, -0.25]);
    // Reciprocal input size, and the first gather centre one texel right
    // and one texel *up* - the negative is upstream's, not a slip.
    let (rx, ry) = (1.0 / 960.0, 1.0 / 540.0);
    assert_eq!(c.con1, [rx, ry, rx, -ry]);
    assert_eq!(c.con2, [-rx, 2.0 * ry, rx, 2.0 * ry]);
    assert_eq!(c.con3, [0.0, 4.0 * ry, 0.0, 0.0]);
}

#[test]
fn a_one_to_one_scale_maps_output_pixels_onto_input_pixels() {
    let c = Constants::new((1280, 720), (1280, 720), (1280, 720), Sharpness::DEFAULT);
    assert_eq!(c.con0[0], 1.0);
    assert_eq!(c.con0[1], 1.0);
    // Still the half-texel shift: EASU resolves at pixel centres.
    assert_eq!(c.con0[2], 0.0);
    assert_eq!(c.con0[3], 0.0);
}

#[test]
fn sharpness_counts_stops_downward_and_off_is_a_zero_lobe() {
    // Zero stops is maximum sharpening, and each whole stop halves it.
    assert_eq!(Sharpness::stops(0.0).factor(), 1.0);
    assert_eq!(Sharpness::stops(1.0).factor(), 0.5);
    assert_eq!(Sharpness::stops(2.0).factor(), 0.25);
    // Out of range in either direction lands on the documented limits
    // rather than on an lobe that produces unnatural results.
    assert_eq!(Sharpness::stops(-3.0).factor(), 1.0);
    assert_eq!(Sharpness::stops(9.0).factor(), 0.25);
    // `OFF` multiplies the lobe by exactly zero. That is *not* the same as
    // RCAS becoming a pass-through - see the constant's docs - but the
    // factor itself is exact.
    assert_eq!(Sharpness::OFF.factor(), 0.0);
}

/// Compiles both shaders and runs both passes on a real device.
///
/// The arithmetic above is CPU-side and says nothing about whether the WGSL
/// parses, whether `textureGather`'s component argument is accepted, or
/// whether the two pipelines agree with their bind group layout - all of
/// which are runtime failures in wgpu, not build ones.
///
/// **Skips when there is no adapter**, so a green CI run is not evidence
/// that it ran. Run it locally on real hardware.
#[test]
fn both_passes_build_and_draw_on_a_real_device() {
    let instance = wgpu::Instance::default();
    let Ok(adapter) = pollster::block_on(instance.request_adapter(&Default::default())) else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .expect("requesting the device");

    let format = wgpu::TextureFormat::Rgba8UnormSrgb;
    let scene = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("scene"),
        size: wgpu::Extent3d {
            width: 160,
            height: 90,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        // The non-sRGB twin FSR 1 reads through - see the module docs.
        view_formats: &[format.remove_srgb_suffix()],
    });
    let source = scene.create_view(&wgpu::TextureViewDescriptor {
        format: Some(format.remove_srgb_suffix()),
        ..Default::default()
    });

    let mut fsr = Fsr1::new(&device, format).expect("building the pipelines");
    assert!(fsr.output().is_none(), "no output before the first render");

    let mut encoder = device.create_command_encoder(&Default::default());
    fsr.render(
        &device,
        &queue,
        &mut encoder,
        Frame {
            source: &source,
            viewport: (160, 90),
            input: (160, 90),
            output: (320, 180),
            sharpness: Sharpness::DEFAULT,
        },
    );
    queue.submit(Some(encoder.finish()));
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("draining the queue");

    assert!(fsr.output().is_some(), "the sharpened target must exist");
}

/// Upscales a known picture and reads the result back.
///
/// A pipeline that builds and draws proves only that nothing was rejected;
/// it would pass just as happily on a shader that wrote black everywhere,
/// which is the failure a botched gather ordering or a wrong constant
/// actually produces. This asserts on the picture.
///
/// The input is a hard vertical edge - black left half, white right half -
/// which is the case an edge-adaptive resampler exists for, and the one
/// where it must visibly differ from a bilinear stretch.
///
/// **What this does not prove.** A vertical edge is symmetric under a
/// swapped `textureGather` component ordering, so a gather mix-up can
/// survive it. This asserts that the port produces a correct picture rather
/// than garbage, not that it is EASU specifically - a bilinear stretch of
/// this input would also pass. A diagonal edge is the case that
/// discriminates gather ordering, and is the test to add next.
///
/// **Skips when there is no adapter.** Run it locally on real hardware.
#[test]
fn upscaling_a_hard_edge_keeps_the_edge_and_the_two_flat_sides() {
    const IN: (u32, u32) = (64, 64);
    const OUT: (u32, u32) = (128, 128);

    let instance = wgpu::Instance::default();
    let Ok(adapter) = pollster::block_on(instance.request_adapter(&Default::default())) else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .expect("requesting the device");

    // `Rgba8Unorm` throughout: this is a test about resampling arithmetic,
    // and an sRGB round trip would put a transfer function between what is
    // written and what is asserted for no gain.
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let scene = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("edge"),
        size: wgpu::Extent3d {
            width: IN.0,
            height: IN.1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    let mut pixels = vec![0u8; (IN.0 * IN.1 * 4) as usize];
    for y in 0..IN.1 {
        for x in 0..IN.0 {
            let value = if x < IN.0 / 2 { 0 } else { 255 };
            let at = ((y * IN.0 + x) * 4) as usize;
            pixels[at..at + 4].copy_from_slice(&[value, value, value, 255]);
        }
    }
    queue.write_texture(
        scene.as_image_copy(),
        &pixels,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(IN.0 * 4),
            rows_per_image: Some(IN.1),
        },
        scene.size(),
    );
    let source = scene.create_view(&wgpu::TextureViewDescriptor::default());

    let mut fsr = Fsr1::new(&device, format).expect("building the pipelines");
    let mut encoder = device.create_command_encoder(&Default::default());
    fsr.render(
        &device,
        &queue,
        &mut encoder,
        Frame {
            source: &source,
            viewport: IN,
            input: IN,
            output: OUT,
            sharpness: Sharpness::DEFAULT,
        },
    );

    let unpadded = (OUT.0 * 4) as usize;
    let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as usize;
    let padded = unpadded.div_ceil(align) * align;
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("readback"),
        size: (padded * OUT.1 as usize) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    encoder.copy_texture_to_buffer(
        fsr.output_texture().expect("an output").as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded as u32),
                rows_per_image: Some(OUT.1),
            },
        },
        wgpu::Extent3d {
            width: OUT.0,
            height: OUT.1,
            depth_or_array_layers: 1,
        },
    );
    queue.submit(Some(encoder.finish()));
    readback.slice(..).map_async(wgpu::MapMode::Read, |_| {});
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("draining the queue");
    let mapped = readback.slice(..).get_mapped_range().expect("mapping");

    let row = OUT.1 / 2;
    let at = |x: u32| mapped[padded * row as usize + (x * 4) as usize];

    // The flat sides survive. An upscaler that got its gather ordering or
    // its constants wrong smears or shifts the edge, and either shows up
    // here as a mid-grey well away from the seam.
    assert_eq!(at(4), 0, "the black side is not black");
    assert_eq!(at(OUT.0 - 5), 255, "the white side is not white");

    // The edge lands where the geometry says it should - halfway - and is
    // genuinely sharp: at twice the scale a bilinear stretch spreads a hard
    // edge over about two output texels, and EASU plus RCAS must not do
    // worse than that. Counting the texels that are neither side tests the
    // thing the algorithm is for.
    let middle = (0..OUT.0).filter(|&x| at(x) > 8 && at(x) < 247).count();
    assert!(
        middle <= 2,
        "the edge spread over {middle} texels, which is not an upscale of a hard edge"
    );
    let first_white = (0..OUT.0).find(|&x| at(x) > 247).expect("a white side");
    assert!(
        first_white.abs_diff(OUT.0 / 2) <= 2,
        "the edge landed at {first_white}, not near {}",
        OUT.0 / 2
    );

    drop(mapped);
    readback.unmap();
}

/// RCAS must not overshoot, which is the artifact that would hurt text.
///
/// A sharpener's characteristic failure is ringing: a bright halo just
/// outside a dark edge and a dark one just inside it. On 480x272-era
/// paletted sprite art and on glyphs lifted from a coverage atlas - which
/// is what the menus and the HUD are made of - that reads as fringing, and
/// it is the specific reason to doubt a global upscaler setting.
///
/// The hard black-and-white edge in the test above cannot show it, because
/// overshoot past 0 and 255 is clamped away by the format. A **mid-range**
/// edge can: anything darker than the dark side or brighter than the bright
/// side is ringing, with nowhere to hide.
///
/// **Some overshoot is what sharpening *is*, and RCAS does not promise
/// otherwise**: its limiters bound the result to `[0, 1]`, not to the local
/// neighbourhood. So this pins the magnitude rather than asserting zero.
/// Measured across a 128-level edge: 16/255 at maximum sharpening and
/// 10/255 at the shipped default - 12.5 % and 7.8 % of the edge contrast.
/// That is a well-behaved sharpener. This test exists to catch the day it
/// stops being one - a botched limiter or lobe would blow well past this.
///
/// **Skips when there is no adapter.**
#[test]
fn sharpening_a_mid_range_edge_rings_only_as_much_as_sharpening_must() {
    // A fifth of the edge contrast. Comfortably above what RCAS does and
    // far below what a broken limiter would.
    const BUDGET: u8 = (BRIGHT - DARK) / 5;
    let Some(max) = ringing(0.0) else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    let default = ringing(0.2).expect("an adapter, having just had one");
    for (what, (under, over)) in [("maximum", max), ("default", default)] {
        assert!(
            under <= BUDGET && over <= BUDGET,
            "at {what} sharpening: undershoot {under} below {DARK}, \
             overshoot {over} above {BRIGHT}, budget {BUDGET}"
        );
    }
    // And sharpening less must ring less, or the scale is upside down.
    assert!(default.0 <= max.0 && default.1 <= max.1);
}

const DARK: u8 = 64;
const BRIGHT: u8 = 192;

/// The worst undershoot and overshoot across a mid-range edge, or `None`
/// with no adapter.
fn ringing(stops: f32) -> Option<(u8, u8)> {
    const IN: (u32, u32) = (64, 64);
    const OUT: (u32, u32) = (128, 128);

    let instance = wgpu::Instance::default();
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).ok()?;
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).ok()?;

    let format = wgpu::TextureFormat::Rgba8Unorm;
    let scene = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("mid edge"),
        size: wgpu::Extent3d {
            width: IN.0,
            height: IN.1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    let mut pixels = vec![0u8; (IN.0 * IN.1 * 4) as usize];
    for y in 0..IN.1 {
        for x in 0..IN.0 {
            let value = if x < IN.0 / 2 { DARK } else { BRIGHT };
            let at = ((y * IN.0 + x) * 4) as usize;
            pixels[at..at + 4].copy_from_slice(&[value, value, value, 255]);
        }
    }
    queue.write_texture(
        scene.as_image_copy(),
        &pixels,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(IN.0 * 4),
            rows_per_image: Some(IN.1),
        },
        scene.size(),
    );
    let source = scene.create_view(&wgpu::TextureViewDescriptor::default());

    let mut fsr = Fsr1::new(&device, format).expect("building the pipelines");
    let mut encoder = device.create_command_encoder(&Default::default());
    fsr.render(
        &device,
        &queue,
        &mut encoder,
        Frame {
            source: &source,
            viewport: IN,
            input: IN,
            output: OUT,
            // Maximum sharpening: if anything rings, it rings here.
            sharpness: Sharpness::stops(stops),
        },
    );

    let unpadded = (OUT.0 * 4) as usize;
    let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as usize;
    let padded = unpadded.div_ceil(align) * align;
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("readback"),
        size: (padded * OUT.1 as usize) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    encoder.copy_texture_to_buffer(
        fsr.output_texture().expect("an output").as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded as u32),
                rows_per_image: Some(OUT.1),
            },
        },
        wgpu::Extent3d {
            width: OUT.0,
            height: OUT.1,
            depth_or_array_layers: 1,
        },
    );
    queue.submit(Some(encoder.finish()));
    readback.slice(..).map_async(wgpu::MapMode::Read, |_| {});
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("draining the queue");
    let mapped = readback.slice(..).get_mapped_range().expect("mapping");

    let row = OUT.1 / 2;
    let mut worst = (0u8, 0u8);
    for x in 0..OUT.0 {
        let v = mapped[padded * row as usize + (x * 4) as usize];
        worst.0 = worst.0.max(DARK.saturating_sub(v));
        worst.1 = worst.1.max(v.saturating_sub(BRIGHT));
    }
    drop(mapped);
    readback.unmap();
    Some(worst)
}

#[test]
fn a_degenerate_size_cannot_divide_by_zero() {
    let c = Constants::new((0, 0), (0, 0), (0, 0), Sharpness::DEFAULT);
    assert!(c.con0.iter().all(|v| v.is_finite()), "{:?}", c.con0);
    assert!(c.con1.iter().all(|v| v.is_finite()), "{:?}", c.con1);
}

/// `con0` reads the drawn rectangle, `con1`..`con3` read the resource.
///
/// The two were one argument until dynamic resolution needed them apart, and
/// they are equal in every configuration the game ships today - so nothing but
/// this test distinguishes a correct split from a folded one. Upstream's
/// `FsrEasuCon` takes `inputViewportInPixels` and `inputSizeInPixels`
/// separately for exactly this case; the numbers below are that signature read
/// literally.
#[test]
fn the_easu_constants_split_the_viewport_from_the_resource() {
    let c = Constants::new((960, 540), (1440, 816), (1920, 1080), Sharpness::DEFAULT);
    // The viewport-to-output ratio, and upstream's half-texel offset on it.
    assert_eq!(c.con0, [0.5, 0.5, -0.25, -0.25]);
    // Gather offsets are texels of the *resource*, which is larger.
    let (rx, ry) = (1.0 / 1440.0, 1.0 / 816.0);
    assert_eq!(c.con1, [rx, ry, rx, -ry]);
    assert_eq!(c.con2, [-rx, 2.0 * ry, rx, 2.0 * ry]);
    assert_eq!(c.con3, [0.0, 4.0 * ry, 0.0, 0.0]);
}
