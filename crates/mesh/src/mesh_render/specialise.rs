//! What a model's pipelines are told about its vertices, so the driver can drop the branches of
//! `shade.wesl` none of them takes. Everything here keeps the picture exact: the values are
//! supersets of what the vertices carry, and a read they rule out is a read of zero.

use crate::mesh::{Model, slots};

/// The `slots` bits of a vertex's index into [`Model::emissive`].
const MATERIAL_BITS: u32 = 0x7f << slots::MATERIAL_SHIFT;

/// The bits every pipeline keeps whatever its vertices carry: the alpha channel,
/// `NO_AMBIENT`/`NO_SUN` and `LIT_ALPHA`. They tell apart HD's sky, hulls and engine flares, which
/// branch on little else.
const ALWAYS: u32 = slots::alpha_channel(3) | slots::EMISSIVE | slots::LIT_ALPHA;

/// The five weapon-only role bits: each has a role set of its own and covers a few pixels.
const WEAPON_ONLY: u32 = slots::RIM_GLOW
    | slots::RIM_EDGE
    | slots::CLOCK_SCROLL_RING
    | slots::CLOCK_SCROLL_HALO
    | slots::BOMB_HALO;

/// The value of `shade.wesl`'s `slots_present` override for `model`: every `slots` bit one of
/// its vertices carries, so the driver compiles out the branches none of them takes. Any superset
/// draws the same picture (`x & present == x` for every vertex), so two choices trade a little of
/// that folding for pipelines shared between models, each specialised pipeline being a fresh
/// compile when the driver's shader cache is cold:
///
/// - **[`ALWAYS`] set**, and the whole material index on a model where one vertex names a row.
/// - **All bits for a weapon effect** (a model carrying one of [`WEAPON_ONLY`]). Specialising them
///   took HD's race from 90 to 168 distinct pipelines.
pub(super) fn slots_present(model: &Model) -> u32 {
    present_for(model.vertices.iter().fold(0, |all, v| all | v.slots))
}

fn present_for(carried: u32) -> u32 {
    if carried & WEAPON_ONLY != 0 {
        return u32::MAX;
    }
    let material = if carried & MATERIAL_BITS != 0 {
        MATERIAL_BITS
    } else {
        0
    };
    carried | ALWAYS | material
}

/// The value of `bindings.wesl`'s `emissive_rows` override for a pipeline whose `slots_present`
/// is `present`: `0` where no vertex can name a row, so every read of the table is its all-zero
/// row 0 (`Emissives::of`) and the driver can drop the tests on it.
pub(super) fn emissive_rows(present: u32) -> u32 {
    u32::from(present & MATERIAL_BITS != 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_model_naming_no_row_folds_the_table_and_keeps_the_rest_of_its_bits() {
        let present = present_for(slots::SECOND_IS_LIGHTMAP);
        assert_eq!(present & MATERIAL_BITS, 0);
        assert_eq!(emissive_rows(present), 0);
        assert_eq!(
            present & slots::SECOND_IS_LIGHTMAP,
            slots::SECOND_IS_LIGHTMAP
        );
        assert_eq!(present & ALWAYS, ALWAYS);
    }

    #[test]
    fn one_row_named_keeps_the_whole_index_so_models_share_pipelines() {
        let present = present_for(1 << slots::MATERIAL_SHIFT);
        assert_eq!(present & MATERIAL_BITS, MATERIAL_BITS);
        assert_eq!(emissive_rows(present), 1);
    }

    #[test]
    fn a_weapon_effect_keeps_every_bit_and_its_table() {
        let present = present_for(slots::RIM_GLOW);
        assert_eq!(present, u32::MAX);
        assert_eq!(emissive_rows(present), 1);
    }
}
