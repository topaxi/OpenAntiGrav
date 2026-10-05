use super::*;

/// The segment count is the original's `min(10, trunc(level * 11.0))`, and
/// the boundaries are what a `10.0` scale would get wrong.
#[test]
fn a_bars_segment_count_is_the_recovered_eleven_step_conversion() {
    assert_eq!(segments(0.0), 0);
    assert_eq!(segments(-1.0), 0, "a negative level lights nothing");
    assert_eq!(
        segments(f32::NAN),
        0,
        "NaN lights nothing rather than panics"
    );
    // `1/11` is the first step: just under it lights nothing, just over it
    // lights one. A `* 10.0` conversion would put this boundary at `0.1`.
    assert_eq!(segments(0.09), 0);
    assert_eq!(segments(0.10), 1);
    assert_eq!(segments(0.5), 5);
    // The tenth segment lights from `10/11` on, not only at `1.0` - the
    // whole reason the original scales by eleven and clamps.
    assert_eq!(segments(0.91), 10);
    assert_eq!(segments(1.0), SEGMENTS, "a full level lights every segment");
    assert_eq!(segments(4.0), SEGMENTS, "and an over-range one no more");
}

/// **The peak-hold's recovered ballistics**: instant attack, a linear
/// [`Hold::DECAY`] per frame, clamped at zero.
#[test]
fn the_hold_rises_at_once_and_falls_by_a_tenth_a_frame() {
    let mut hold = Hold::default();
    assert_eq!(hold.advance(&[1.0, 0.0]), &[1.0, 0.0]);
    // Falling: one linear decay step, not a multiplicative fade.
    let fallen = hold.advance(&[0.0, 0.0])[0];
    assert!((fallen - 0.9).abs() < 1e-6, "got {fallen}");
    // Rising again takes the new value whole rather than easing towards it.
    assert_eq!(hold.advance(&[1.0, 0.0])[0], 1.0);
    // And it never falls below zero however long it falls.
    for _ in 0..40 {
        hold.advance(&[0.0, 0.0]);
    }
    assert_eq!(hold.advance(&[0.0, 0.0]), &[0.0, 0.0]);
}

/// **The assertion that ties the recovered layout to the shipped art.**
/// `zone-shader.md` histogrammed `zonemodetrack9`/`10`'s alpha at exactly
/// `{31..40}` before this layout was read out of the executable, and
/// `31..=40` is band 3's ten segments under the recovered stride. A
/// regression to any other stride stops those ten texels being one band's
/// bar, which is what made the effect invisible before.
#[test]
fn band_three_is_the_ten_texels_the_shipped_stage_texture_tags() {
    let mut bands = vec![0.0f32; 16];
    bands[3] = 0.5;
    let pixels = vis_pixels(&bands, Some([10, 20, 30]));
    for (offset, texel) in pixels[31..=40].iter().enumerate() {
        let want = if offset < 5 {
            [10, 20, 30, 255]
        } else {
            [0; 4]
        };
        assert_eq!(*texel, want, "texel {}", 31 + offset);
    }
    // And band 3 lit nothing outside its own bar.
    assert_eq!(pixels[30], [0; 4], "band 2's last segment");
    assert_eq!(pixels[41], [0; 4], "band 4's first segment");
}

/// Texel 0 is never written, and each band's smooth slot sits one texel apart
/// from [`SMOOTH_BASE`] - the second half of the recovered layout.
#[test]
fn the_smooth_half_is_one_texel_a_band_after_the_bars() {
    let mut bands = vec![0.0f32; 16];
    bands[0] = 1.0;
    bands[15] = 1.0;
    let pixels = vis_pixels(&bands, Some([200, 100, 50]));
    assert_eq!(
        pixels[0], [0; 4],
        "texel 0 is the original's untouched slot"
    );
    assert_eq!(pixels[SMOOTH_BASE], [200, 100, 50, 255]);
    assert_eq!(pixels[SMOOTH_BASE + 15], [200, 100, 50, 255]);
    assert_eq!(pixels[SMOOTH_BASE + 7], [0, 0, 0, 255], "a silent band");
    // Sixteen bands of bars end at texel 160, immediately before the smooth
    // half - the arithmetic that makes 161 the right base.
    assert_eq!(1 + SEGMENTS * 16, SMOOTH_BASE);
}

/// A lit segment carries the **whole** tint, not a fraction of it - the
/// difference between bars and the faint wash the invented mapping this
/// replaced produced, and what makes the effect visible at all.
#[test]
fn a_lit_segment_is_the_full_tint_and_an_unlit_one_is_transparent_black() {
    let pixels = vis_pixels(&[0.2], Some([255, 128, 64]));
    assert_eq!(pixels[1], [255, 128, 64, 255], "segment 1 at level 2");
    assert_eq!(pixels[2], [255, 128, 64, 255], "segment 2 at level 2");
    assert_eq!(pixels[3], [0; 4], "segment 3 is dark, not dim");
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
