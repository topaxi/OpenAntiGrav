//! A split screen runs HD's post chain and Omega's tone map per view, each with an adaptation
//! of its own ([`Scene::run_split_post`]).

use super::scene_build::test_gpu;
use super::split_blur::{AREA, drawn, hull_scene, pixels, rows, target};
use super::*;
use oag_display::display::{Fov, MotionBlur, Msaa, Shadows};

/// A circuit's `HDR and Bloom` block with an adaptation the picture can move, and no glow
/// (the exposure is what is under test).
fn params() -> oag_post::hd_bloom::Params {
    oag_post::hd_bloom::Params {
        alpha_contribution: 0.0,
        frame_contribution: 0.0,
        frame_exponent: 1.0,
        horizontal_size: 1.0,
        vertical_size: 1.0,
        adaption_rate: 0.5,
        adaption_boost: 1.0,
        tone_adaption_boost: 1.0,
        tone_darkening_clamp: 1.0,
        tone_maximum_brightness: 1.0,
        zoom: None,
    }
}

fn omega_params() -> oag_post::omega_tonemap::Params {
    oag_post::omega_tonemap::Params {
        luminance_a: 0.18,
        luminance_b: 0.05,
        exposure_minimum: 0.25,
        exposure_maximum: 4.0,
        exposure_response: 3.0,
        exposure_time: 4.0,
        source_end_a: 2.0,
        source_end_b: 0.5,
        start_angle: 10.0,
        end_angle: -20.0,
    }
}

/// Which post chain the scene is built with.
#[derive(Clone, Copy, Debug)]
enum Chain {
    Hd,
    Omega,
}

fn scene_of(device: &wgpu::Device, queue: &wgpu::Queue, race: &Race, chain: Chain) -> Scene {
    let (hd, omega) = match chain {
        Chain::Hd => (Some(params()), None),
        Chain::Omega => (None, Some(omega_params())),
    };
    hull_scene(device, queue, race, Msaa::Off, hd, omega)
}

const CHAINS: [Chain; 2] = [Chain::Hd, Chain::Omega];

fn draw_split(
    (device, queue): (&wgpu::Device, &wgpu::Queue),
    scene: &Scene,
    race: &Race,
    target: &wgpu::Texture,
) {
    scene.render_viewports(
        device,
        queue,
        &target.create_view(&Default::default()),
        race,
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
}

/// Every view of a split is resolved into its own rectangle by its own run of the chain, and
/// each seat keeps an adaptation of its own: two lanes after a two-view frame. Fails when the
/// chain is not run per view (no lanes, or a black view: the second view's resolve would be
/// the first's), and when a later view clears the frame (the first view goes black).
#[test]
fn every_view_resolves_into_its_own_rectangle_with_a_lane_of_its_own() {
    let Some((device, queue)) = test_gpu() else {
        return;
    };
    for chain in CHAINS {
        let mut race = race_with_a_grid();
        assert_eq!(race.set_viewport_count(2), 2);
        let scene = scene_of(&device, &queue, &race, chain);
        let target = target(&device);
        for _ in 0..90 {
            race.tick(&oag_gameplay::PlayerInputs::none());
        }
        draw_split((&device, &queue), &scene, &race, &target);
        let picture = pixels(&device, &queue, &target);
        let rects = race.view_rects(AREA);
        assert!(
            drawn(rows(&picture, rects[0])),
            "{chain:?}: the first view survives"
        );
        assert!(
            drawn(rows(&picture, rects[1])),
            "{chain:?}: the second view is drawn"
        );
        assert_eq!(
            scene.split_exposure_seats(),
            2,
            "{chain:?}: a lane per seat"
        );
    }
}

/// The first single-view frame after a split forgets the seats' lanes, as it forgets their
/// camera histories: a seat that comes back settles from its own first frame again.
#[test]
fn a_single_view_frame_forgets_the_seats_adaptation() {
    let Some((device, queue)) = test_gpu() else {
        return;
    };
    for chain in CHAINS {
        let mut race = race_with_a_grid();
        assert_eq!(race.set_viewport_count(2), 2);
        let scene = scene_of(&device, &queue, &race, chain);
        let target = target(&device);
        draw_split((&device, &queue), &scene, &race, &target);
        assert_eq!(scene.split_exposure_seats(), 2);
        assert_eq!(race.set_viewport_count(1), 1);
        let mut encoder = device.create_command_encoder(&Default::default());
        scene.render(
            &device,
            &queue,
            &mut encoder,
            &target.create_view(&Default::default()),
            &race,
            AREA,
            Fov::AUTHORED,
            true,
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
        queue.submit(Some(encoder.finish()));
        assert_eq!(scene.split_exposure_seats(), 0, "{chain:?}");
    }
}

/// The gap between the views is written every frame, not left to whatever the frame held: with
/// the target filled white before the split is drawn, the rows between the views come out opaque
/// black. Fails when no view clears the frame (Omega's apply pass only loads; HD's resolve is
/// the one that clears) and when the clear is transparent (alpha 0 in the gap).
#[test]
fn the_gap_between_views_is_cleared_every_frame() {
    let Some((device, queue)) = test_gpu() else {
        return;
    };
    for chain in CHAINS {
        let mut race = race_with_a_grid();
        assert_eq!(race.set_viewport_count(2), 2);
        let scene = scene_of(&device, &queue, &race, chain);
        let target = target(&device);
        let mut encoder = device.create_command_encoder(&Default::default());
        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("split exposure test: stale frame"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &target.create_view(&Default::default()),
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        queue.submit(Some(encoder.finish()));
        draw_split((&device, &queue), &scene, &race, &target);
        let picture = pixels(&device, &queue, &target);
        let rects = race.view_rects(AREA);
        let (top, bottom) = ((rects[0].1 + rects[0].3) as usize, rects[1].1 as usize);
        assert!(bottom > top, "the views leave a gap to look at");
        let row = (super::split_blur::SIZE * 4) as usize;
        for texel in picture[top * row..bottom * row].as_chunks::<4>().0 {
            assert_eq!(*texel, [0, 0, 0, 255], "{chain:?}: the gap is opaque black");
        }
    }
}
