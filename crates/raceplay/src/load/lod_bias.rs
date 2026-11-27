//! The title's LOD bias on the material textures of the models a circuit's own
//! geometry does not build: the craft hulls and the mine and bomb bodies. `geometry::track_model` sets it on the track and what the
//! track draws behind its glass; HD's original reads the same filter word on these
//! (`docs/rendering/hd-track-tone.md`, "The LOD bias on hulls, pads and weapons").
//!
//! Left at 0 on HD: the speedup and weapon pads (no stored frame shows one), the sky, the
//! shadow hulls, every shield, absorb, leach, flare and wreck model, and the other
//! weapon models, none of which that census read. A title whose loader biases every
//! texture (2048) reaches all of those at once: `Scene::new` puts the bias on every
//! pipeline it builds, and [`finish_effects`] on the effect textures.

use super::Loaded;
use oag_mesh::mesh::Model;

/// Sets `bias` (in levels, [`oag_title::Looks::material_lod_levels`]) on the
/// models the filter word was read on. A title that leaves the rule unread passes
/// 0 and nothing changes.
pub(super) fn finish(loaded: &mut Loaded, bias: f32) {
    if bias == 0.0 {
        return;
    }
    let hulls = loaded.liveries.iter_mut().map(|livery| &mut livery.hull);
    let others = [&mut loaded.mine_model, &mut loaded.bomb_model]
        .into_iter()
        .flatten();
    apply(hulls.chain(others), bias);
}

/// Sets `bias` (in levels, [`oag_title::Looks::every_texture_lod_levels`]) on the effect
/// textures a race samples through `oag_fx`'s pipelines: the flare sprite, the ribbon's two,
/// the leach beam's, the Cannon's two and every `beam` ribbon's. Each then binds its file's own
/// mip chain under it
/// ([`oag_fx::exhaust::FlareTexture::lod_bias`]). A title that leaves the rule unread passes
/// 0 and each binds its base alone. The particle sprites take the same bias at
/// `psys::Pipeline::with_lod_bias`.
pub(super) fn finish_effects(loaded: &mut Loaded, bias: f32) {
    let (bolt, flash, _) = &mut loaded.cannon_quad_textures;
    let ribbons = &mut loaded.ribbon_textures;
    let magstrip = ribbons.magstrip.iter_mut().flatten();
    let rocket = ribbons.rocket_smoke.iter_mut().map(|a| &mut a.texture);
    let missile = ribbons
        .missile_smoke
        .iter_mut()
        .flat_map(|a| std::iter::once(&mut a.texture).chain(a.streak_texture.iter_mut()));
    let leach = ribbons
        .leach_strip
        .iter_mut()
        .flat_map(|a| [&mut a.glow, &mut a.noise]);
    let wingtip = ribbons.wingtip.iter_mut().map(|a| &mut a.texture);
    magstrip
        .chain(rocket)
        .chain(missile)
        .chain(leach)
        .chain(wingtip)
        .chain(
            [
                &mut loaded.flare,
                &mut loaded.noise,
                &mut loaded.trail_shape,
                &mut loaded.leach_beam_texture,
                bolt,
                flash,
            ]
            .into_iter()
            .flatten(),
        )
        .for_each(|texture| texture.lod_bias = bias);
}

fn apply<'a>(models: impl IntoIterator<Item = &'a mut Model>, bias: f32) {
    for model in models {
        model.material_lod_bias = bias;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bias_lands_on_every_model_it_is_given_and_on_no_other() {
        let mut hull = Model::none("hull");
        let mut pad = Model::none("pad");
        let untouched = Model::none("sky");
        apply([&mut hull, &mut pad], -1.5);
        assert_eq!(hull.material_lod_bias, -1.5);
        assert_eq!(pad.material_lod_bias, -1.5);
        assert_eq!(untouched.material_lod_bias, 0.0);
    }
}
