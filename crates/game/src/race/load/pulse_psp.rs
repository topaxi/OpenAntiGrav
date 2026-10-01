//! What Pulse on the PSP draws differently from every other source, measured
//! live on PPSSPP: the hull's GE lights and the bloom's stencil-stamped glow
//! mask. Neither has been measured on the PS2 port, Pure or any later title,
//! so both stay behind [`is_pulse_psp`]. The Quake's road ripple joins them:
//! it is read off the PSP executable and moves PSP-shaped GE batches.
//!
//! See `docs/ghidra/functions/psp-pulse-usa/scene-light.md` and
//! `docs/rendering/glow-mask.md`.

use oag_render::mesh::Model;
use oag_render::mesh_render;

use super::Loaded;

/// Whether this race is Pulse off a PSP disc.
pub(super) fn is_pulse_psp(title: &oag_title::Title, archives: &oag_assets::Archives) -> bool {
    title.name == oag_pulse::TITLE.name && archives.layout.platform == oag_assets::Platform::Psp
}

/// Lights the hulls with the circuit's own `AmbientLight`/`DirectionalLight`
/// nodes and marks every `.vex` model the race draws as stamping the glow
/// mask. A no-op when `pulse_psp` is false.
pub(super) fn finish(loaded: &mut Loaded, pulse_psp: bool, track_blob: &[u8]) {
    if !pulse_psp {
        return;
    }
    let nodes = oag_vex::vex::nodes(track_blob).unwrap_or_default();
    match mesh_render::HullLights::from_track(track_blob, &nodes, 255) {
        Some(hull) => {
            loaded.report.push(format!(
                "hull lights: the circuit's own, ambient {:?}, diffuse {:?}",
                hull.ambient, hull.diffuse
            ));
            loaded.light.hull = hull;
        }
        None => loaded
            .report
            .push("hull lights: the circuit authors none; hulls keep the stand-in rig".into()),
    }

    loaded.ripples = super::ripple::build(
        track_blob,
        &loaded.setup.ai,
        loaded.setup.course.as_ref(),
        [
            Some(&loaded.track_model),
            loaded.pad_model.as_ref(),
            loaded.weapon_pad_model.as_ref(),
        ],
        &mut loaded.report,
    );

    let stamp = |model: &mut Model| model.stamps_glow = true;
    stamp(&mut loaded.track_model);
    for model in [
        &mut loaded.sky_model,
        &mut loaded.pad_model,
        &mut loaded.weapon_pad_model,
        &mut loaded.shield_cockpit,
    ]
    .into_iter()
    .flatten()
    {
        stamp(model);
    }
    for livery in &mut loaded.liveries {
        stamp(&mut livery.hull);
        if let Some(wreck) = &mut livery.wreck {
            stamp(&mut wreck.model);
        }
        for model in [&mut livery.boost, &mut livery.shield]
            .into_iter()
            .flatten()
        {
            stamp(model);
        }
    }
    loaded
        .report
        .push(match oag_render::shine::build_track(&loaded.track_model) {
            Some(pass) => format!(
                "track shine: {} chrome-map batches drawn, {} left undrawn (animated nodes or \
             outside the opaque list)",
                pass.model.draws.len(),
                pass.skipped
            ),
            None if loaded.track_model.shine_draws.is_empty() => {
                "track shine: off, or the circuit authors none".into()
            }
            None => format!(
                "track shine: none drawn, {} batches left undrawn (animated nodes or outside \
             the opaque list)",
                loaded.track_model.shine_draws.len()
            ),
        });
    loaded.report.push(
        "glow mask: the track, sky, pads, hulls, plumes and shields stamp it as the \
         original's stencil does, a blended batch with the glow bits through a second draw"
            .into(),
    );
}
