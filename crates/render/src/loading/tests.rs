//! What the loading screen's procedural wave in [`super`] is asserted to do:
//! the envelope table, the ramp and the slew, the spatial walk, the quad it
//! builds, and where the band lands when it is drawn on a real device.
//!
//! Its own file rather than a `#[cfg(test)]` block at the end of
//! `loading.rs`: the tests are 650 lines, past the 200 an inline test module
//! may hold. See `scripts/check-file-size.py`, which is the rule as a gate.

use super::*;

fn rng() -> Rng {
    Rng::new(0x10ad)
}

#[test]
fn the_envelope_is_a_heartbeat() {
    assert_eq!(ENVELOPE.len(), 24);
    let peaks = ENVELOPE.iter().filter(|v| **v == ENVELOPE_PEAK).count();
    assert_eq!(peaks, 2, "two beats, with a trough between them");
    assert_eq!(
        ENVELOPE.iter().cloned().fold(0.0f32, f32::max),
        ENVELOPE_PEAK,
        "the divisor must be the table's own maximum"
    );
    assert!(
        ENVELOPE[18..].iter().all(|v| *v == 0.0),
        "the tail is six frames of silence"
    );
}

#[test]
fn the_ramp_clamps_at_both_ends() {
    assert_eq!(ramp255(30, 286, 0), 0);
    assert_eq!(ramp255(30, 286, 30), 0);
    assert_eq!(ramp255(30, 286, 286), 255);
    assert_eq!(ramp255(30, 286, 480), 255);
    assert_eq!(ramp255(30, 286, 158), 127, "half way is half way");
}

#[test]
fn the_slew_does_not_move_inside_its_deadband() {
    assert_eq!(slew_toward(0.0, 3.9, SLEW_STEP, SLEW_DEADBAND), 0.0);
    assert_eq!(slew_toward(0.0, 4.0, SLEW_STEP, SLEW_DEADBAND), 0.0);
    assert_eq!(slew_toward(0.0, 4.1, SLEW_STEP, SLEW_DEADBAND), 0.5);
    assert_eq!(slew_toward(0.0, -4.1, SLEW_STEP, SLEW_DEADBAND), -0.5);
    assert_eq!(
        slew_toward(0.0, 400.0, SLEW_STEP, SLEW_DEADBAND),
        0.5,
        "a huge gap still moves by at most one step"
    );
}

#[test]
fn the_texture_mirrors_with_period_64() {
    assert_eq!(texture_column(0), 0);
    assert_eq!(texture_column(30), 30);
    assert_eq!(texture_column(32), 31);
    assert_eq!(texture_column(62), 1);
    assert_eq!(texture_column(64), 0, "back to the start after 64");
    for x in 0..512 {
        let u = texture_column(x);
        assert!((0..32).contains(&u), "u={u} is outside the 32-wide strip");
    }
}

#[test]
fn the_wave_is_pinned_flat_at_the_left_edge() {
    let wave = Wave::new();
    let columns = wave.columns(&mut rng());
    assert_eq!(
        columns[0], [0.0; BANDS],
        "the first column reads energy before it is ever kicked"
    );
}

#[test]
fn the_wave_develops_towards_the_right() {
    // The amplitude ramp is zero until x=30 and the walk starts from
    // nothing, so the left third must be calmer than the right third.
    let wave = Wave {
        phase: 5,
        finished: false,
    };
    let columns = wave.columns(&mut rng());
    let spread = |slice: &[Column]| slice.iter().map(|c| c[0].abs()).fold(0.0f32, f32::max);
    let left = spread(&columns[..COLUMNS / 3]);
    let right = spread(&columns[COLUMNS * 2 / 3..]);
    assert!(
        right > left,
        "left {left} should be calmer than right {right}"
    );
}

#[test]
fn the_walk_is_spatial_so_a_frame_is_not_an_integration_of_the_last() {
    // The tell: with the same draws, the wave is identical. If any state
    // were carried across frames this would differ.
    let wave = Wave::new();
    let a = wave.columns(&mut rng());
    let b = wave.columns(&mut rng());
    assert_eq!(
        a, b,
        "nothing but the rng and the phase may survive a frame"
    );
}

#[test]
fn fresh_draws_make_it_wriggle() {
    let wave = Wave::new();
    let mut shared = rng();
    let a = wave.columns(&mut shared);
    let b = wave.columns(&mut shared);
    assert_ne!(a, b, "consecutive frames must not be identical");
}

#[test]
fn the_heartbeat_wraps_at_twenty_four() {
    let mut wave = Wave::new();
    for _ in 0..ENVELOPE.len() {
        wave.advance();
    }
    assert_eq!(wave.phase(), 0, "one beat is exactly the table's length");
}

#[test]
fn a_finished_wave_freezes_rather_than_fading_on() {
    let mut wave = Wave::new();
    wave.advance();
    let frozen = wave.phase();
    wave.finish();
    for _ in 0..50 {
        wave.advance();
    }
    assert_eq!(wave.phase(), frozen);
    assert!(wave.is_finished());
}

#[test]
fn the_amplitude_never_falls_to_nothing_between_beats() {
    // ENVELOPE[0] is zero, but the floor keeps the band alive.
    let wave = Wave::new();
    assert_eq!(wave.pulse(), 0.0);
    let idle = wave.pulse() / ENVELOPE_PEAK + ENVELOPE_FLOOR;
    assert_eq!(idle, ENVELOPE_FLOOR, "it idles at a tenth, not at zero");
}

#[test]
fn the_band_spans_the_full_width_whatever_it_is_drawn_into() {
    // The PS2 port's bug, stated as a test: 240 columns of 2 px is 480 px,
    // which covers 640 only three quarters of the way across. Normalised
    // output cannot express that failure.
    let wave = Wave::new();
    let columns = wave.columns(&mut rng());
    let quads = wave.quads(&columns);
    assert_eq!(quads.len(), COLUMNS * BANDS);
    assert_eq!(
        quads.first().expect("a quad").x,
        0.0,
        "the band must start at the left edge"
    );
    let right = quads.last().expect("a quad");
    assert!(
        (right.x + right.w - 1.0).abs() < 1e-5,
        "the band must reach the right edge, got {}",
        right.x + right.w
    );
    for quad in &quads {
        assert!(
            (0.0..=1.0).contains(&quad.u0) && (0.0..=1.0).contains(&quad.u1),
            "texture coordinates must stay inside the strip"
        );
    }
}

#[test]
fn the_baseline_sits_at_the_same_fraction_of_the_height() {
    let wave = Wave::new();
    let columns = wave.columns(&mut rng());
    let quads = wave.quads(&columns);
    // Column 0 is pinned flat, so its band is exactly on the baseline.
    let expected = BASELINE_Y / REF_HEIGHT - (STRIP_SIZE / REF_HEIGHT) * 0.5;
    assert!((quads[0].y - expected).abs() < 1e-6);
}

#[test]
fn a_quad_becomes_two_triangles_with_the_strip_the_right_way_up() {
    let quad = Quad {
        x: 0.25,
        y: 0.5,
        w: 0.1,
        h: 0.2,
        u0: 0.0,
        u1: 1.0 / STRIP_SIZE,
        alpha: 0.75,
    };
    let v = quad_vertices(&quad);
    assert_eq!(v.len(), 6);
    // The capture below cannot catch a flipped `v`: the strip's own
    // intensity is very nearly symmetric about its middle row, so
    // mirroring it moves the drawn band by far less than the tolerance
    // there. Pinned exactly here instead.
    assert_eq!(v[0].position, [quad.x, quad.y], "vertex 0 is the top left");
    assert_eq!(v[0].texcoord, [quad.u0, 0.0], "the top edge samples v = 0");
    assert_eq!(
        v[5].texcoord[1], 0.0,
        "the second triangle's top vertex too"
    );
    assert!(
        v.iter().all(|vertex| vertex.alpha == quad.alpha),
        "the ramp is per column, so every vertex of a quad carries it"
    );
    let bottom = v
        .iter()
        .filter(|vertex| vertex.texcoord[1] == 1.0)
        .collect::<Vec<_>>();
    assert_eq!(bottom.len(), 3, "three of the six are on the bottom edge");
    assert!(
        bottom
            .iter()
            .all(|vertex| vertex.position[1] == quad.y + quad.h),
        "v = 1 must be the lower edge on screen, not the upper one"
    );
}

/// `GlowStrip::decode` on a blob shaped like the real one.
///
/// Synthetic bytes, not the disc's: a 32x32 8bpp texture is a 16-byte
/// header, a 256-entry palette and one index per pixel, which is 2,064
/// bytes - the size `LoadingPulseOverlay.mip` is documented at. Building it
/// here rather than reading entry 68 keeps the test runnable without a disc
/// image and keeps no game content anywhere near the repository.
#[test]
fn the_glow_strip_decodes_from_a_mip_blob() {
    let size = STRIP_SIZE as usize;
    let mut blob = vec![0u8; 16];
    blob[0..2].copy_from_slice(&(size as u16).to_le_bytes());
    blob[2..4].copy_from_slice(&(size as u16).to_le_bytes());
    blob[4] = 8;
    blob[6] = 1;
    // A palette ramping black to the documented `(222, 255, 255)`, opaque
    // throughout, which is what the real one does.
    for i in 0..256u32 {
        let level = |peak: u32| (peak * i / 255) as u8;
        blob.extend_from_slice(&[level(222), level(255), level(255), 255]);
    }
    blob.extend((0..size * size).map(|i| (i % 256) as u8));
    assert_eq!(blob.len(), 2064, "the documented size of entry 68");

    let strip = GlowStrip::decode(&blob).expect("a well-formed blob must decode");
    assert_eq!((strip.width, strip.height), (32, 32));
    assert_eq!(strip.rgba.len(), 32 * 32 * 4);
    assert_eq!(
        &strip.rgba[..4],
        &[0, 0, 0, 255],
        "index 0 is the black end"
    );

    assert!(
        GlowStrip::decode(&blob[..blob.len() - 1]).is_err(),
        "the size arithmetic is exact, so a truncated blob is not a texture"
    );
}

/// Renders the overlay offscreen and hands the frame back as RGBA8.
///
/// `Rgba8Unorm`, not sRGB: this is checking what the additive blend put on
/// screen against a known input, and an encode would put a transfer
/// function between the two. Sample count 1 and **no depth attachment** -
/// [`Pipeline`] declares `depth_stencil: None`, so attaching one is a
/// validation error rather than a harmless extra.
fn capture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    strip: &GlowStrip,
    vertices: &[GpuVertex],
    width: u32,
    height: u32,
) -> Vec<u8> {
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let size = wgpu::Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
    };
    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("loading capture"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = target.create_view(&wgpu::TextureViewDescriptor::default());

    let mut pipeline = Pipeline::new(device, queue, format, strip, 1);
    pipeline.upload(queue, [1.0; 4], vertices);

    let unpadded = width as usize * 4;
    let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as usize;
    let padded = unpadded.div_ceil(align) * align;
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("readback"),
        size: (padded * height as usize) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let mut encoder = device.create_command_encoder(&Default::default());
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("loading wave"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    // Black, which is what the wave is drawn over: the
                    // backdrop is either cleared or a tip image, and an
                    // additive blend over black is the strip itself.
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pipeline.draw(&mut pass);
    }
    encoder.copy_texture_to_buffer(
        target.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded as u32),
                rows_per_image: Some(height),
            },
        },
        size,
    );
    queue.submit(Some(encoder.finish()));
    readback.slice(..).map_async(wgpu::MapMode::Read, |_| {});
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("draining the queue");
    let mapped = readback.slice(..).get_mapped_range().expect("mapping");
    let mut pixels = Vec::with_capacity(unpadded * height as usize);
    for row in mapped.chunks(padded).take(height as usize) {
        pixels.extend_from_slice(&row[..unpadded]);
    }
    drop(mapped);
    readback.unmap();
    pixels
}

/// Per-row brightness: the sum of every rgb channel on that row.
fn row_profile(pixels: &[u8], width: u32, height: u32) -> Vec<u64> {
    (0..height as usize)
        .map(|y| {
            pixels[y * width as usize * 4..(y + 1) * width as usize * 4]
                .chunks_exact(4)
                .map(|p| u64::from(p[0]) + u64::from(p[1]) + u64::from(p[2]))
                .sum()
        })
        .collect()
}

/// Per-column brightness, the same sum down the other axis.
fn column_profile(pixels: &[u8], width: u32, height: u32) -> Vec<u64> {
    let mut out = vec![0u64; width as usize];
    for row in pixels
        .chunks_exact(width as usize * 4)
        .take(height as usize)
    {
        for (x, pixel) in row.chunks_exact(4).enumerate() {
            out[x] += u64::from(pixel[0]) + u64::from(pixel[1]) + u64::from(pixel[2]);
        }
    }
    out
}

/// The band's intensity-weighted centre, as a fraction of the height.
fn band_centre(profile: &[u64]) -> f64 {
    let total: u64 = profile.iter().sum();
    assert!(total > 0, "nothing was drawn at all");
    let weighted: f64 = profile
        .iter()
        .enumerate()
        .map(|(y, v)| (y as f64 + 0.5) * *v as f64)
        .sum();
    weighted / total as f64 / profile.len() as f64
}

/// Draws the wave offscreen at two very different sizes and asserts on the
/// pixels.
///
/// **Skips when there is no adapter**, so a green CI run is not evidence
/// that it ran - the same caveat `post::fxaa`'s device tests carry.
///
/// Both captures are fed the *same* quads, built once from one seeded
/// [`Rng`]. Resolution independence is then exact rather than statistical:
/// the band's centre must land at the same fraction of the height on a
/// 480x272 frame and a 1920x1080 one, and any difference is rasterisation
/// quantisation rather than a difference in the geometry.
///
/// What it measured when written, on this machine:
///
/// | Size | Lit rows | As a fraction | Weighted centre |
/// | --- | --- | --- | ---: |
/// | 480x272 | 202..=238 | 0.743..0.879 | 0.8117 |
/// | 1920x1080 | 804..=949 | 0.744..0.880 | 0.8117 |
///
/// The centre sits 0.003 below `BASELINE_Y / REF_HEIGHT` (0.8088) because
/// the wave is not flat even on the envelope's silent frame - the floor
/// keeps a tenth of the amplitude alive - and the drawn spread is wider
/// than one 32-pixel strip for the same reason.
#[test]
fn the_band_draws_where_the_original_puts_it_at_any_resolution() {
    let instance = wgpu::Instance::default();
    let Ok(adapter) = pollster::block_on(instance.request_adapter(&Default::default())) else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .expect("requesting the device");

    let wave = Wave::new();
    let columns = wave.columns(&mut rng());
    let quads = wave.quads(&columns);
    let vertices = vertices(&quads);
    assert_eq!(vertices.len(), MAX_VERTICES);

    let strip = GlowStrip::placeholder(STRIP_SIZE as u32);
    let mut centres = Vec::new();
    for (width, height) in [(REF_WIDTH as u32, REF_HEIGHT as u32), (1920, 1080)] {
        let pixels = capture(&device, &queue, &strip, &vertices, width, height);
        let profile = row_profile(&pixels, width, height);
        let lit: Vec<usize> = profile
            .iter()
            .enumerate()
            .filter(|(_, v)| **v > 0)
            .map(|(y, _)| y)
            .collect();
        let first = *lit.first().expect("some row must be lit");
        let last = *lit.last().expect("some row must be lit");
        let centre = band_centre(&profile);
        eprintln!(
            "{width}x{height}: lit rows {first}..={last} ({:.4}..{:.4}), centre {centre:.4}",
            first as f64 / height as f64,
            (last + 1) as f64 / height as f64
        );

        // Well above and well below the band is untouched black. The band
        // is 32 reference pixels tall around y=220 of 272, so 0.70 and 0.92
        // are both clear of it even before the wave's own offsets - which
        // are small at phase 0, the envelope's silent frame.
        let above = (height as f64 * 0.70) as usize;
        let below = (height as f64 * 0.92) as usize;
        assert!(
            profile[..above].iter().all(|v| *v == 0),
            "the top 70% must be black, first lit row is {first}"
        );
        assert!(
            profile[below..].iter().all(|v| *v == 0),
            "the bottom 8% must be black, last lit row is {last}"
        );
        assert!(
            profile[first..=last].iter().any(|v| *v > 0),
            "the band itself must be lit"
        );

        // The across-screen ramp is a real thing to check: `ramp255(10,
        // 350, x)` is zero until x=10 of 480, so the far left is dark and
        // brightness rises to the right.
        let row = &pixels[centre_row_range(centre, width, height)];
        let brightness = |slice: &[u8]| -> u64 {
            slice
                .chunks_exact(4)
                .map(|p| u64::from(p[0]) + u64::from(p[1]) + u64::from(p[2]))
                .sum()
        };
        let third = row.len() / 3 / 4 * 4;
        let left = brightness(&row[..third]);
        let right = brightness(&row[row.len() - third..]);
        assert!(
            right > left,
            "the alpha ramp must brighten to the right: left {left}, right {right}"
        );

        // The band reaches the right edge of the framebuffer, whatever the
        // framebuffer is - the PS2 port's bug, checked in pixels this time
        // rather than in the quads' arithmetic. And the far left is
        // genuinely black rather than dim: `ramp255(10, 350, 0)` clamps to
        // zero, so the first columns draw nothing at all. That is the dark
        // left edge in a capture of this, and it is correct.
        let columns = column_profile(&pixels, width, height);
        assert_eq!(
            columns[0], 0,
            "the alpha ramp starts at x=10 of 480, so column 0 draws nothing"
        );
        assert!(
            columns[width as usize - 1] > 0,
            "the band must reach the right edge of a {width}-wide frame"
        );

        centres.push(centre);
    }

    // y = 220 on a 272-line display, the figure `BASELINE_Y` carries.
    let expected = f64::from(BASELINE_Y) / f64::from(REF_HEIGHT);
    for centre in &centres {
        assert!(
            (centre - expected).abs() < 0.01,
            "the band's centre is {centre:.4}, expected {expected:.4}"
        );
    }
    assert!(
        (centres[0] - centres[1]).abs() < 0.005,
        "the same quads must land at the same fraction of the height: {:.4} against {:.4}",
        centres[0],
        centres[1]
    );
}

/// The byte range of the row nearest `centre`.
fn centre_row_range(centre: f64, width: u32, height: u32) -> std::ops::Range<usize> {
    let y = ((centre * f64::from(height)) as usize).min(height as usize - 1);
    let stride = width as usize * 4;
    y * stride..(y + 1) * stride
}

/// Writes one frame to `data/cache/loading-wave.png`, for eyeballing.
///
/// `#[ignore]`d because it exists to produce a picture rather than to check
/// anything, the same split `tests/collision_capture.rs` uses - the test
/// above is the one that guards the behaviour. Run it with
/// `cargo nextest run -p oag-render --run-ignored all a_capture_for_eyeballing`.
/// `data/` is gitignored, which is where a derived picture belongs.
#[test]
#[ignore = "writes a picture rather than asserting anything"]
fn a_capture_for_eyeballing() {
    let instance = wgpu::Instance::default();
    let Ok(adapter) = pollster::block_on(instance.request_adapter(&Default::default())) else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .expect("requesting the device");

    // Phase 5 is the envelope's first 99, so the band is at full amplitude
    // rather than idling at the floor.
    let mut wave = Wave::new();
    for _ in 0..5 {
        wave.advance();
    }
    let columns = wave.columns(&mut rng());
    let vertices = vertices(&wave.quads(&columns));
    let strip = GlowStrip::placeholder(STRIP_SIZE as u32);
    let (width, height) = (REF_WIDTH as u32, REF_HEIGHT as u32);
    let pixels = capture(&device, &queue, &strip, &vertices, width, height);

    // Relative to the workspace root, not the crate: a test's working
    // directory is its own crate.
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/cache");
    std::fs::create_dir_all(&dir).expect("creating data/cache");
    let path = dir.join("loading-wave.png");
    std::fs::write(&path, oag_formats::png::encode_rgba(width, height, &pixels))
        .expect("writing the capture");
    eprintln!("wrote {}", path.display());
}

/// Renders the wave with the **real** `LoadingPulseOverlay.mip`, not the
/// placeholder.
///
/// Everything else here is checked against a synthetic strip, which proves
/// the pipeline but not that the game's own texture survives
/// [`GlowStrip::decode`] and reaches the shader. `Data.wad` entry 68 is
/// swizzled and its palette is fully opaque, so a linear read or an
/// alpha-respecting shader both give a plausible-but-wrong band - exactly
/// the failure a synthetic strip cannot reproduce.
#[test]
#[ignore = "needs a PSP disc image under data/images"]
fn the_games_own_glow_strip_reaches_the_shader() {
    let image =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/images/pulse-psp-eu.chd");
    if !image.exists() {
        eprintln!("skipping: {} not present", image.display());
        return;
    }
    let spec = format!("{}:PSP_GAME/USRDIR/Data.wad", image.display());
    let mut archive = oag_assets::Archive::open(&spec).expect("opening Data.wad");
    let blob = archive.read(68).expect("reading entry 68");

    let strip = GlowStrip::decode(&blob).expect("decoding the glow strip");
    assert_eq!(
        (strip.width, strip.height),
        (STRIP_SIZE as u32, STRIP_SIZE as u32),
        "the recovered strip is 32x32"
    );

    // The documented palette ends at Pulse's cyan, so the brightest texel
    // has to be blue-dominant. A swizzle or palette mistake shows up here
    // as grey or as a colour cast rather than as a crash.
    let brightest = strip
        .rgba
        .chunks_exact(4)
        .max_by_key(|p| u32::from(p[0]) + u32::from(p[1]) + u32::from(p[2]))
        .expect("a texel");
    assert!(
        brightest[2] >= brightest[0] && brightest[1] >= brightest[0],
        "brightest texel {brightest:?} is not on the black-to-cyan ramp"
    );
    assert!(
        strip.rgba.chunks_exact(4).all(|p| p[3] == 255),
        "every palette entry is opaque, which is why the shader ignores texel alpha"
    );

    let instance = wgpu::Instance::default();
    let Ok(adapter) = pollster::block_on(instance.request_adapter(&Default::default())) else {
        eprintln!("no GPU adapter: skipping the draw");
        return;
    };
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .expect("requesting the device");

    let mut wave = Wave::new();
    for _ in 0..5 {
        wave.advance();
    }
    let columns = wave.columns(&mut rng());
    let vertices = vertices(&wave.quads(&columns));
    let (width, height) = (REF_WIDTH as u32, REF_HEIGHT as u32);
    let pixels = capture(&device, &queue, &strip, &vertices, width, height);

    let lit = pixels
        .chunks_exact(4)
        .filter(|p| u32::from(p[0]) + u32::from(p[1]) + u32::from(p[2]) > 0)
        .count();
    assert!(
        lit > 0,
        "the real strip drew nothing, so it never reached the shader"
    );
    eprintln!("{lit} lit pixels of {}", width * height);

    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/cache");
    std::fs::create_dir_all(&dir).expect("creating data/cache");
    let out = dir.join("loading-wave-real.png");
    std::fs::write(&out, oag_formats::png::encode_rgba(width, height, &pixels))
        .expect("writing the capture");
    eprintln!("wrote {}", out.display());
}
