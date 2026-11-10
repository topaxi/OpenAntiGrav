//! The title's LOD bias on the material textures of the models a circuit's own
//! geometry does not build: the craft hulls and the mine and bomb bodies. `geometry::track_model` sets it on the track and what the
//! track draws behind its glass; HD's original reads the same filter word on these
//! (`docs/rendering/hd-track-tone.md`, "The LOD bias on hulls, pads and weapons").
//!
//! Left at 0: the speedup and weapon pads (no stored frame shows one), the sky, the
//! shadow hulls, every shield, absorb, leach, flare and wreck model, and the other
//! weapon models, none of which that census read.

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
