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
    let matrix =
        view_projection(&auricom_camera(), 1.2, 2017.7).expect("a one-key perspective camera");
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
    let matrix = view_projection(&auricom_camera(), 1.2, 2017.7).expect("camera");
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
    assert!(view_projection(&camera, 1.2, 2017.7).is_none());
}

/// A camera at the origin looking down -z with the given authored field of view
/// and aspect.
fn camera_at_origin(fov_degrees: f32, aspect: f32) -> oag_vex::camera::Camera {
    let mut payload = vec![0u8; oag_vex::camera::PAYLOAD_LEN];
    payload[0..4].copy_from_slice(&1u32.to_le_bytes());
    payload[4..8].copy_from_slice(&0x20u32.to_le_bytes());
    payload[8..12].copy_from_slice(&0x22u32.to_le_bytes());
    payload[0x1c..0x20].copy_from_slice(&aspect.to_le_bytes());
    let raw = (fov_degrees * 65535.0 / 180.0).round() as u16;
    payload[0x22..0x24].copy_from_slice(&raw.to_le_bytes());
    oag_vex::camera::Camera::parse(
        None,
        &payload,
        oag_vex::vex::matrix::IDENTITY,
        ByteOrder::Little,
    )
    .expect("a full payload")
}

/// The projection matrices of Wipeout HD's eight billboard slots, read live off
/// RPCS3 on Talon's Junction (2026-10-06, `g_BillboardSlots + 0x10 + k * 0x100 +
/// 0xb0`): `(authored fov, aspect, matrix x scale, matrix y scale)`. The fov and
/// aspect are the slot's own `.vex` camera, the scales the matrix the engine
/// built from it, so this fails if the law is not `x = 1 / tan(fov / 2)`,
/// `y = aspect * x` or the camera is read differently.
const HD_LIVE: [(f32, f32, f32, f32); 6] = [
    (14.474_709, 1.0, 7.8745, 7.8745),
    (68.877_09, 2.0, 1.4584, 2.9167),
    (58.321_81, 2.0, 1.7922, 3.5843),
    (9.676_356, 2.0, 11.8143, 23.6285),
    (18.039_825, 2.0, 6.2996, 12.5992),
    (61.727_627, 4.0, 1.6733, 6.6931),
];

#[test]
fn hd_projection_matches_the_matrices_read_live_off_rpcs3() {
    for (fov, aspect, want_x, want_y) in HD_LIVE {
        let camera = camera_at_origin(fov, aspect);
        let matrix = view_projection(&camera, 0.5, 5500.0).expect("a perspective camera");
        let (x, y) = (matrix.x_axis.x, matrix.y_axis.y);
        assert!(
            (x - want_x).abs() / want_x < 2e-3,
            "fov {fov}: x scale {x}, want {want_x}"
        );
        assert!(
            (y - want_y).abs() / want_y < 2e-3,
            "fov {fov}: y scale {y}, want {want_y}"
        );
    }
}

impl Cards {
    /// Draws every card at `seconds` and reads each target back as RGBA8, in
    /// card order, with its slot and size.
    fn read_back(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        seconds: f32,
    ) -> Vec<(u32, (u32, u32), Vec<u8>)> {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        self.render(queue, &mut encoder, seconds, &|_| seconds);
        let buffers: Vec<wgpu::Buffer> = self
            .cards
            .iter()
            .map(|card| {
                let (width, height) = card.size;
                let row = width * 4;
                let buffer = device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("advert readback"),
                    size: u64::from(row * height),
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
                            rows_per_image: Some(height),
                        },
                    },
                    wgpu::Extent3d {
                        width,
                        height,
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
                (card.slot, card.size, bytes)
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
        for (slot, (width, height), pixels) in cards.read_back(&device, &queue, seconds) {
            if let Ok(dir) = std::env::var("OAG_ADVERT_DUMP") {
                let mut shown = pixels.clone();
                shown
                    .as_chunks_mut::<4>()
                    .0
                    .iter_mut()
                    .for_each(|p| p[3] = 255);
                let png = oag_texture::png::encode_rgba(width, height, &shown);
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

#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso and a GPU"]
fn hd_cards_draw_something_into_their_targets() {
    let Some(counts) = drawn_cards("data/images/hdfury-ps3-eu-dec.iso", "hd") else {
        return;
    };
    println!("{counts:?}");
    assert!(!counts.is_empty(), "no card was built");
    assert_every_slot_shows_a_picture(&counts);
}

/// The served placeholder vertices of `source`'s default circuit, as `(slot, v)`.
fn served_v(source: &str) -> Option<Vec<(u32, f32)>> {
    let image = oag_testdata::image(source)?;
    let loaded = crate::load(&crate::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        ..crate::Options::default()
    })
    .expect("loading the race");
    let model = &loaded.track_model;
    let slots = oag_render::gantry::placeholder_texture_slots(model);
    let served: Vec<u32> = loaded.billboards.adverts.iter().map(|c| c.slot).collect();
    let mut out = Vec::new();
    for draw in [
        &model.draws,
        &model.alpha_tested_draws,
        &model.transparent_draws,
    ]
    .into_iter()
    .flatten()
    {
        let Some(texture) = draw.texture else {
            continue;
        };
        let Some(&(_, slot)) = slots.iter().find(|&&(s, _)| s == texture) else {
            continue;
        };
        if !served.contains(&slot) {
            continue;
        }
        for i in draw.range.clone() {
            out.push((
                slot,
                model.vertices[model.indices[i as usize] as usize].texcoord[1],
            ));
        }
    }
    Some(out)
}

/// Pulse's draws that sample a card carry V negated, as the original's do
/// (`TEXSCALE (1, -1)` in the PPSSPP dump). The authored V of Talon's Junction's
/// slot 2 and slot 7 quads is 0 to 0.5 and 1.5 to 2.0, never negative, so a
/// positive V here means the flip was dropped.
#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn pulse_psp_samples_its_cards_with_v_negated() {
    let Some(v) = served_v("data/images/pulse-psp-usa.chd") else {
        return;
    };
    let slot7: Vec<f32> = v.iter().filter(|(s, _)| *s == 7).map(|&(_, v)| v).collect();
    assert!(!slot7.is_empty(), "no served slot 7 vertices");
    assert!(slot7.iter().all(|&v| v <= 0.0), "slot 7 kept a positive V");
    assert!(
        slot7.iter().any(|&v| v <= -1.0),
        "slot 7's V is not the authored 1.5 to 2.0 negated"
    );
    assert!(
        v.iter().filter(|(s, _)| *s == 2).all(|&(_, v)| v <= 0.0),
        "slot 2 kept a positive V"
    );
}

/// HD's quads author V running down, so its served draws keep the authored V:
/// slot 2's run from about 0.1 to 0.9.
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn hd_keeps_the_authored_v_on_its_cards() {
    let Some(v) = served_v("data/images/hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let slot2: Vec<f32> = v.iter().filter(|(s, _)| *s == 2).map(|&(_, v)| v).collect();
    assert!(!slot2.is_empty(), "no served slot 2 vertices");
    assert!(
        slot2.iter().all(|&v| (0.0..=1.0).contains(&v)),
        "HD slot 2 V moved"
    );
}

/// HD's cards on Talon's Junction: slots 2, 3, 5 and 7 have a quad on this circuit and
/// each is drawn into the 512 x 256 target the engine builds, not Pulse's 128 x 128.
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn hd_builds_a_512_by_256_card_for_each_slot_with_a_quad() {
    let Some(image) = oag_testdata::image("data/images/hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let loaded = crate::load(&crate::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        ..crate::Options::default()
    })
    .expect("loading the race");
    let slots: Vec<u32> = loaded.billboards.adverts.iter().map(|c| c.slot).collect();
    assert_eq!(slots, [2, 3, 5, 7, 8], "the slots with a quad and a model");
    assert!(
        loaded
            .billboards
            .adverts
            .iter()
            .all(|c| c.size == (512, 256)),
        "an HD card is not 512 x 256"
    );
}

/// Pulse's cards stay 128 x 128 next to HD's.
#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn pulse_psp_cards_stay_128_square() {
    let Some(image) = oag_testdata::image("data/images/pulse-psp-usa.chd") else {
        return;
    };
    let loaded = crate::load(&crate::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        ..crate::Options::default()
    })
    .expect("loading the race");
    assert!(!loaded.billboards.adverts.is_empty());
    assert!(
        loaded
            .billboards
            .adverts
            .iter()
            .all(|c| c.size == (128, 128))
    );
}

#[test]
fn hd_draws_its_gantry_as_a_card_and_pulse_does_not() {
    let hd = oag_hd::TITLE.adverts.expect("HD measured its advert pass");
    let pulse = oag_pulse::TITLE
        .adverts
        .expect("Pulse measured its advert pass");
    assert!(
        hd.gantry_card,
        "RPCS3 2026-10-08: slot 8 is a 512x256 card on HD"
    );
    assert!(
        !pulse.gantry_card,
        "Pulse's gantry stands on the mount, byte-identical"
    );
}

#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn hd_loads_slot_8_as_a_clocked_card_and_not_as_a_placed_model() {
    let Some(image) = oag_testdata::image("data/images/hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let loaded = crate::load(&crate::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        ..crate::Options::default()
    })
    .expect("loading the race");
    assert!(loaded.billboards.gantry.is_none(), "no placed gantry model");
    let card = loaded
        .billboards
        .adverts
        .iter()
        .find(|card| card.slot == 8)
        .expect("slot 8 is a card");
    assert!(card.timeline.is_some(), "the gantry card carries its clock");
    assert_eq!(card.size, (512, 256));
}
