use super::*;
use oag_formats::ByteOrder;

/// Auricom's camera as it ships on Pulse PSP: one key of value `0x5080`,
/// aspect 2.0, and `camera1` at `(0.4854, 0.4779, 31.832)` looking down -z.
fn auricom_camera() -> oag_vex::camera::Camera {
    let mut payload = vec![0u8; oag_vex::camera::PAYLOAD_LEN];
    payload[0..4].copy_from_slice(&1u32.to_le_bytes());
    payload[4..8].copy_from_slice(&0x20u32.to_le_bytes());
    payload[8..12].copy_from_slice(&0x22u32.to_le_bytes());
    payload[0x1c..0x20].copy_from_slice(&2.0f32.to_le_bytes());
    payload[0x22..0x24].copy_from_slice(&0x5080u16.to_le_bytes());
    let mut to_world = oag_vex::vex::matrix::IDENTITY;
    to_world[12] = 0.485_420_2;
    to_world[13] = 0.477_935_8;
    to_world[14] = 31.832_031;
    oag_vex::camera::Camera::parse(None, &payload, to_world, ByteOrder::Little)
        .expect("a full payload")
}

/// The captured matrices of the running original: view = translation by
/// `(-0.4854, -0.4779, -31.832)`, projection x scale 1.857 and y scale 3.714.
/// The model's origin therefore lands at
/// `(1.857 * -0.4854 / 31.832, 3.714 * -0.4779 / 31.832)` in clip space after
/// the divide - which fails if the field of view is read as vertical, if the
/// aspect scales x instead of y, or if the view is not the camera's inverse.
#[test]
fn the_origin_lands_where_the_captured_matrices_put_it() {
    let matrix = view_projection(&auricom_camera()).expect("a one-key perspective camera");
    let clip = matrix * Vec3::ZERO.extend(1.0);
    let (x, y) = (clip.x / clip.w, clip.y / clip.w);
    assert!((x - (-0.028_32)).abs() < 2e-4, "x {x}");
    assert!((y - (-0.055_76)).abs() < 2e-4, "y {y}");
    assert!((clip.w - 31.832).abs() < 1e-3, "w {}", clip.w);
}

/// A panel 34.3 wide and 17.1 tall at the origin fills the card exactly: the
/// camera was framed on it (`visible half-height = distance / (aspect * x
/// scale)`).
#[test]
fn the_frustum_at_the_model_plane_is_two_to_one() {
    let matrix = view_projection(&auricom_camera()).expect("camera");
    let at = |x: f32, y: f32| {
        let clip = matrix * Vec3::new(x, y, 0.0).extend(1.0);
        (clip.x / clip.w, clip.y / clip.w)
    };
    let (right, _) = at(0.4854 + 17.14, 0.0);
    let (_, top) = at(0.0, 0.4779 + 8.57);
    assert!((right - 1.0).abs() < 0.01, "{right}");
    assert!((top - 1.0).abs() < 0.01, "{top}");
}

#[test]
fn an_orthographic_camera_gets_no_card() {
    let mut camera = auricom_camera();
    camera.fov_flags = 1;
    assert!(view_projection(&camera).is_none());
}

impl Cards {
    /// Draws every card at `seconds` and reads each target back as RGBA8, in
    /// card order, with its slot.
    fn read_back(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        seconds: f32,
    ) -> Vec<(u32, Vec<u8>)> {
        let row = SIDE * 4;
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        self.render(queue, &mut encoder, seconds);
        let buffers: Vec<wgpu::Buffer> = self
            .cards
            .iter()
            .map(|card| {
                let buffer = device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("advert readback"),
                    size: u64::from(row * SIDE),
                    usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                    mapped_at_creation: false,
                });
                encoder.copy_texture_to_buffer(
                    card.target.as_image_copy(),
                    wgpu::TexelCopyBufferInfo {
                        buffer: &buffer,
                        layout: wgpu::TexelCopyBufferLayout {
                            offset: 0,
                            bytes_per_row: Some(row),
                            rows_per_image: Some(SIDE),
                        },
                    },
                    wgpu::Extent3d {
                        width: SIDE,
                        height: SIDE,
                        depth_or_array_layers: 1,
                    },
                );
                buffer
            })
            .collect();
        queue.submit([encoder.finish()]);
        buffers
            .iter()
            .zip(&self.cards)
            .map(|(buffer, card)| {
                buffer
                    .slice(..)
                    .map_async(wgpu::MapMode::Read, |r| r.unwrap());
                device
                    .poll(wgpu::PollType::wait_indefinitely())
                    .expect("poll");
                let bytes = buffer
                    .slice(..)
                    .get_mapped_range()
                    .expect("mapped")
                    .to_vec();
                (card.slot, bytes)
            })
            .collect()
    }
}

fn distinct_colours(rgba: &[u8]) -> usize {
    rgba.as_chunks::<4>()
        .0
        .iter()
        .map(|p| [p[0], p[1], p[2]])
        .collect::<std::collections::BTreeSet<_>>()
        .len()
}

/// Loads `source`'s default circuit, draws its cards at two clock readings and
/// returns each card's pixels; also writes PNGs to `$OAG_ADVERT_DUMP` when set.
fn drawn_cards(source: &str, tag: &str) -> Option<Vec<(u32, usize)>> {
    let image = oag_testdata::image(source)?;
    let instance = wgpu::Instance::default();
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).ok()?;
    let (device, queue) = pollster::block_on(
        adapter.request_device(&mesh_render::device_descriptor("advert test", &adapter)),
    )
    .ok()?;
    let loaded = crate::load(&crate::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        ..crate::Options::default()
    })
    .expect("loading the race");
    let cards = Cards::new(&device, &queue, loaded.billboards.adverts, Anisotropy::Off)
        .expect("building the cards");
    let mut counts = Vec::new();
    let times: Vec<f32> = std::env::var("OAG_ADVERT_TIMES")
        .map(|v| v.split(',').filter_map(|t| t.parse().ok()).collect())
        .unwrap_or_else(|_| vec![0.0, 2.5]);
    for &seconds in &times {
        for (slot, pixels) in cards.read_back(&device, &queue, seconds) {
            if let Ok(dir) = std::env::var("OAG_ADVERT_DUMP") {
                let mut shown = pixels.clone();
                shown
                    .as_chunks_mut::<4>()
                    .0
                    .iter_mut()
                    .for_each(|p| p[3] = 255);
                let png = oag_texture::png::encode_rgba(SIDE, SIDE, &shown);
                std::fs::write(format!("{dir}/{tag}-slot{slot}-{seconds}.png"), png)
                    .expect("writing the dump");
            }
            counts.push((slot, distinct_colours(&pixels)));
        }
    }
    Some(counts)
}

/// A card that drew nothing is one flat colour at every reading; an advert
/// has at least its board and its lettering. The first slots fade in, so the
/// best of the readings counts.
fn assert_every_slot_shows_a_picture(counts: &[(u32, usize)]) {
    let mut best = std::collections::BTreeMap::new();
    for &(slot, colours) in counts {
        let entry = best.entry(slot).or_insert(0);
        *entry = colours.max(*entry);
    }
    for (slot, colours) in best {
        assert!(
            colours >= 2,
            "slot {slot}: {colours} distinct colour(s) at best"
        );
    }
}

#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd and a GPU"]
fn pulse_psp_cards_draw_something_into_their_targets() {
    let Some(counts) = drawn_cards("data/images/pulse-psp-usa.chd", "psp") else {
        return;
    };
    println!("{counts:?}");
    assert!(!counts.is_empty(), "no card was built");
    assert_every_slot_shows_a_picture(&counts);
}

#[test]
#[ignore = "needs data/images/pulse-ps2-eu.chd and a GPU"]
fn pulse_ps2_cards_draw_something_into_their_targets() {
    let Some(counts) = drawn_cards("data/images/pulse-ps2-eu.chd", "ps2") else {
        return;
    };
    println!("{counts:?}");
    assert!(!counts.is_empty(), "no card was built");
    assert_every_slot_shows_a_picture(&counts);
}
