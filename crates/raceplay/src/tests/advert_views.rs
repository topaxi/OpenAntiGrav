//! A one-view frame encodes only the advert cards its view shows (`adverts::render`, `Seen`);
//! a frame of several views encodes every card, since its first view does the frame's work
//! for views it has not culled yet.

use super::scene_build::{livery, scene_with_billboards, test_gpu, triangle, vertex};
use super::split_blur::{AREA, target};
use super::*;
use crate::adverts::{Billboards, Card};
use oag_display::display::{Fov, MotionBlur, Msaa, Shadows};
use oag_mesh::mesh::{Bounds, DrawCall};

/// A track of two placeholder quads, `billboard1` ahead of the grid along `+x` and
/// `billboard2` far behind it, so the chase camera's frustum holds the first only.
fn track() -> Model {
    let mut track = triangle();
    track.radius = 1000.0;
    let template = track.draws[0].clone();
    let at = [[200.0f32, 2.0, 0.0], [-600.0, 2.0, 0.0]];
    track.vertices = at
        .iter()
        .flat_map(|&[x, y, z]| {
            [[0.0, -1.0, -1.0], [0.0, -1.0, 1.0], [0.0, 1.0, 0.0]]
                .map(|[dx, dy, dz]| vertex([x + dx, y + dy, z + dz]))
        })
        .collect();
    track.indices = (0..6).collect();
    track.draws = at
        .iter()
        .enumerate()
        .map(|(slot, &centre)| DrawCall {
            range: (slot as u32 * 3)..(slot as u32 * 3 + 3),
            texture: Some(slot),
            bounds: Bounds {
                centre,
                radius: 1.5,
            },
            ..template.clone()
        })
        .collect();
    track.textures = (1..=2)
        .map(|number| {
            Some(std::sync::Arc::new(mesh::ModelTexture::rgba8(
                format!("billboard{number}.tga"),
                1,
                1,
                vec![255; 4],
                None,
            )))
        })
        .collect();
    track
}

fn card(slot: u32) -> Card {
    Card {
        slot,
        model: triangle(),
        view_projection: Mat4::IDENTITY,
        size: (4, 4),
        wrap: false,
        card_store: oag_title::adverts::CardStore::Rgba8,
        timeline: None,
    }
}

fn scene(device: &wgpu::Device, queue: &wgpu::Queue, race: &Race) -> Scene {
    let liveries: Vec<_> = (0..race.core.sim.ship_count()).map(|_| livery()).collect();
    let scene = scene_with_billboards(
        device,
        queue,
        track(),
        &liveries,
        Msaa::Off,
        (None, None, None),
        Billboards {
            adverts: vec![card(1), card(2)],
            ..Billboards::default()
        },
    );
    assert_eq!(scene.adverts_rebound(), 2, "both quads show a card");
    scene
}

/// The card passes one frame of one view encodes, with the view frustum culling or not.
fn one_view(gpu: &(wgpu::Device, wgpu::Queue), scene: &Scene, race: &Race, cull: bool) -> usize {
    let view = target(&gpu.0).create_view(&Default::default());
    let before = scene.adverts_encoded();
    let mut encoder = gpu.0.create_command_encoder(&Default::default());
    scene.render(
        &gpu.0,
        &gpu.1,
        &mut encoder,
        &view,
        race,
        AREA,
        Fov::AUTHORED,
        cull,
        true,
        None,
        MotionBlur::Off,
        Shadows::Off,
        None,
        &[],
        None,
        None,
        None,
    );
    gpu.1.submit(Some(encoder.finish()));
    scene.adverts_encoded() - before
}

/// The card behind the camera is not encoded; with frustum culling off nothing culls it,
/// and it is encoded again. Drop the `Seen` test in `Cards::render` and the first frame
/// encodes two; pass `Seen` a frustum other than the scene pass's and the third reads one.
#[test]
fn one_view_encodes_only_the_cards_it_shows() {
    let Some(gpu) = test_gpu() else {
        return;
    };
    let race = race_with_a_grid();
    let scene = scene(&gpu.0, &gpu.1, &race);
    assert_eq!(
        one_view(&gpu, &scene, &race, true),
        1,
        "the card ahead only"
    );
    assert_eq!(
        one_view(&gpu, &scene, &race, true),
        1,
        "and on the next frame"
    );
    assert_eq!(one_view(&gpu, &scene, &race, false), 2, "unculled, both");
}

/// A split's first view draws the cards for every view of the frame, so it draws them all.
/// Fails if a split frame culls the cards by its first view.
#[test]
fn a_split_frame_encodes_every_card() {
    let Some((device, queue)) = test_gpu() else {
        return;
    };
    let mut race = race_with_a_grid();
    assert_eq!(race.set_viewport_count(2), 2);
    let scene = scene(&device, &queue, &race);
    let view = target(&device).create_view(&Default::default());
    scene.render_viewports(
        &device,
        &queue,
        &view,
        &race,
        AREA,
        Fov::AUTHORED,
        true,
        true,
        None,
        Shadows::Off,
        &[],
        WindowEffects::default(),
        &mut [],
    );
    assert_eq!(scene.adverts_encoded(), 2, "once per frame, both cards");
}
