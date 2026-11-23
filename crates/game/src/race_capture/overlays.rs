//! What a capture draws over the HUD: Detonator's medal and the start-line countdown, split out
//! of `race_capture.rs` under the size ratchet.

use oag_raceplay::{Race, Scene};

/// The medal and the countdown, on the same terms `RaceStage::draw_window_hud` draws them on.
///
/// The countdown follows `oag_hud::countdown`, `oag_race::RaceState::thrust_gated` and
/// `Scene::draws_gantry`, which stands this overlay down where the circuit's own gantry is
/// already showing the count.
#[allow(clippy::too_many_arguments)]
pub(super) fn draw_over_hud(
    (device, queue, format): (&wgpu::Device, &wgpu::Queue, wgpu::TextureFormat),
    encoder: &mut wgpu::CommandEncoder,
    hud_view: &wgpu::TextureView,
    (race, scene, overlay): (&Race, &Scene, &crate::hud_overlay::Overlay),
    (medal_models, countdown_model): (
        &[(oag_hud::ImageModel, oag_mesh::mesh::Model)],
        Option<&(oag_mesh::mesh::Model, oag_hud::Model)>,
    ),
    hud_viewport: (f32, f32, f32, f32),
    size: (u32, u32),
) {
    if !medal_models.is_empty() {
        crate::hud_medal::Medals::new(device, queue, format, medal_models.to_vec()).draw(
            device,
            queue,
            encoder,
            hud_view,
            race.detonator_medal(),
            race.core.sim.world.tick as f32 / 60.0,
            overlay.space(),
            &|rect| overlay.place_rect(rect),
            hud_viewport,
            size,
        );
    }
    if let (Some((model, widget)), true) = (
        countdown_model,
        oag_race::RaceState::thrust_gated(race.core.sim.world.tick) && !scene.draws_gantry(),
    ) {
        match crate::hud_countdown::Countdown::new(device, queue, format, model.clone(), widget) {
            Ok(mut countdown) => countdown.draw(
                device,
                queue,
                encoder,
                hud_view,
                race.core.sim.world.tick as f32 / 60.0,
                None,
                hud_viewport,
                size,
            ),
            Err(why) => {
                log::warn!("the countdown overlay did not build ({why}); capturing without one");
            }
        }
    }
}
