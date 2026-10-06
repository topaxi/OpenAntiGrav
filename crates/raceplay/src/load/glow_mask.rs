//! Which of a race's models stamp the bloom's glow mask, and by which rule.
//!
//! Pulse on the PSP stamps a constant per batch (`pulse_psp`), Pulse on the PS2
//! stamps the fragment's own alpha (`pulse_ps2`); the set of models is the same
//! on both, because both originals run one batch state list over every `.vex`
//! model the race draws.

use oag_mesh::mesh::Model;

use super::Loaded;

/// Marks the track, sky, pads, hulls, plumes, shields and weapon bodies as
/// stamping the mask. `by_texel` is the PS2's rule - see
/// [`oag_mesh::mesh_render::GlowMask::StampedByTexel`].
pub(super) fn stamp_models(loaded: &mut Loaded, by_texel: bool) {
    let stamp = |model: &mut Model| {
        model.stamps_glow = true;
        model.glow_by_texel = by_texel;
    };
    stamp(&mut loaded.track_model);
    if let Some(model) = &mut loaded.shield_cockpit {
        if by_texel {
            model.glow_by_texel = true;
        } else {
            stamp(model);
        }
    }
    for model in [
        &mut loaded.sky_model,
        &mut loaded.pad_model,
        &mut loaded.weapon_pad_model,
        &mut loaded.rocket_model,
        &mut loaded.mine_model,
        &mut loaded.bomb_model,
        &mut loaded.cannon_model,
        &mut loaded.plasma_blast_models.shuriken,
    ]
    .into_iter()
    .flatten()
    {
        stamp(model);
    }
    // The Bomb's blast dome: `hemisphere_disperse1_ADD_GLOW`, a transparent batch with the
    // glow bit, which the original's state list stamps (`0x50`) like the arch lights. The
    // shockwave beside it has no glow bits, so for it the flag is a no-op.
    for model in [
        &mut loaded.bomb_blast_models.hemisphere,
        &mut loaded.bomb_blast_models.shockwave,
        // `noise2_ADD_GLOW`, the same glow-named transparent batch as the dome.
        &mut loaded.bomb_blast_models.repulser_field,
    ]
    .into_iter()
    .chain(&mut loaded.bomb_blast_models.mag_floor)
    .flatten()
    {
        // The magstrip pair joins them: `MagFloorFx_Construct` loads both
        // through the same `Vex_LoadModel`.
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
            if by_texel {
                // The PS2's plume body and shell write no mask: its readout
                // shows only a small block at the nozzle, not the plume's
                // length (docs/rendering/ps2-bloom.md).
                model.stamps_glow = false;
                model.glow_by_texel = true;
            } else {
                stamp(model);
            }
        }
    }
}
