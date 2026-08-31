use super::*;

/// When the caller already hands over one value per texel, interpolation
/// must be the identity - no blur introduced where none is asked for.
#[test]
fn as_many_bands_as_texels_is_the_identity() {
    let bands: Vec<f32> = (0..VIS_WIDTH).map(|i| i as f32 / 255.0).collect();
    let got = levels(&bands);
    for (texel, (&want, &got)) in bands.iter().zip(got.iter()).enumerate() {
        assert!(
            (want - got).abs() < 1e-6,
            "texel {texel}: got {got}, want {want}"
        );
    }
}

/// **The regression this file exists to catch.** A shipped stage texture
/// samples a narrow, arbitrary window of indices - `zone-shader.md` measures
/// `zonemodetrack9`/`10` at exactly `31..=40` - and a caller with far fewer
/// bands than 256 must still show *some* variation across that window, not
/// one repeated value. A block-repeat mapping (`texel * bands.len() /
/// VIS_WIDTH`) fails this: with `oag_audio::spectrum::BANDS` bands, texels
/// 31..=40 all land in the same one or two blocks.
#[test]
fn a_narrow_disc_chosen_window_is_not_one_repeated_value() {
    const BANDS: usize = 32;
    // A monotonic ramp, so any real variation across the window is visible
    // and not a coincidence of a flat input. With a full 0..1 ramp over
    // `BANDS` bands, linear interpolation puts texels 31..=40 at band-space
    // positions ~3.77..~4.87 (`texel / (VIS_WIDTH - 1) * (BANDS - 1)`), a
    // true span of ~0.035 - the threshold below is set just under that, so
    // this still fails loudly if the mapping regresses to block-repeat
    // (which would give exactly 0.0 here) rather than passing on a margin
    // wide enough to hide a real bug.
    let bands: Vec<f32> = (0..BANDS).map(|i| i as f32 / (BANDS - 1) as f32).collect();
    let got = levels(&bands);
    let window = &got[31..=40];
    let (min, max) = window
        .iter()
        .fold((f32::MAX, f32::MIN), |(lo, hi), &v| (lo.min(v), hi.max(v)));
    assert!(
        max - min > 0.03,
        "texels 31..=40 span only {:?} (min {min}, max {max}) - the shipped \
         zonemodetrack9/10 window would read as one flat colour",
        max - min
    );
}

/// The strip's own two ends land exactly on the spectrum's own two ends,
/// whatever the band count - the boundary the interpolation must not miss.
#[test]
fn the_first_and_last_texel_match_the_first_and_last_band() {
    for band_count in [1, 2, 8, 32, 256] {
        // Kept inside `0.0..=1.0`, the range `levels` documents and clamps
        // to - unclamped test data would silently exercise the clamp
        // instead of the interpolation this test means to check.
        let bands: Vec<f32> = (0..band_count)
            .map(|i| i as f32 / (band_count - 1).max(1) as f32)
            .collect();
        let got = levels(&bands);
        assert_eq!(got[0], bands[0], "band count {band_count}: first texel");
        assert_eq!(
            got[VIS_WIDTH as usize - 1],
            bands[band_count - 1],
            "band count {band_count}: last texel"
        );
    }
}

/// A single band has nothing to interpolate between and must not panic -
/// the whole strip reads that one value.
#[test]
fn a_single_band_fills_the_whole_strip() {
    let got = levels(&[0.75]);
    assert!(
        got.iter().all(|&v| v == 0.75),
        "every texel should read the one band's own value"
    );
}

/// [`write_vis`] blanks the texture - not a stale or invented colour - when
/// there is no tint or no spectrum, checked at the pixel level.
#[test]
fn write_vis_blanks_on_no_tint_or_no_bands() {
    let instance = wgpu::Instance::default();
    let Ok(adapter) = pollster::block_on(instance.request_adapter(&Default::default())) else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .expect("requesting the device");

    for (bands, tint) in [(&[0.5f32; 8][..], None), (&[][..], Some([255, 255, 255]))] {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("vis blank test"),
            size: wgpu::Extent3d {
                width: VIS_WIDTH,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: texture::FORMAT,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        // Seed it with something not-black first, so a no-op that merely
        // left the old contents alone would still be caught.
        write_vis(&queue, &texture, &[1.0; 4], Some([255, 255, 255]));
        write_vis(&queue, &texture, bands, tint);

        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("vis readback"),
            size: u64::from(VIS_WIDTH * 4),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = device.create_command_encoder(&Default::default());
        encoder.copy_texture_to_buffer(
            texture.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(VIS_WIDTH * 4),
                    rows_per_image: None,
                },
            },
            wgpu::Extent3d {
                width: VIS_WIDTH,
                height: 1,
                depth_or_array_layers: 1,
            },
        );
        queue.submit(Some(encoder.finish()));
        readback.slice(..).map_async(wgpu::MapMode::Read, |_| {});
        device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("draining the queue");
        let mapped = readback.slice(..).get_mapped_range().expect("mapping");
        assert!(
            mapped.iter().all(|&b| b == 0),
            "blanking must write every byte, including alpha, to zero"
        );
    }
}
