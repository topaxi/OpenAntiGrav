//! What a model's pipelines are told about its vertices, so the driver can drop the branches of
//! `shade.wesl` none of them takes. Everything here keeps the picture exact: the values are
//! supersets of what the vertices carry, and a read they rule out is a read of zero.

use crate::mesh::{Model, slots};

/// The `slots` bits of a vertex's index into [`Model::emissive`].
const MATERIAL_BITS: u32 = slots::MATERIAL_MASK << slots::MATERIAL_SHIFT;

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

/// Whether any vertex of `model` is a 2048 vertex-lit one (`sun_mask` 2 or above, read by
/// `shade.wesl` as `>= 1.5`): `false` lets its pipelines fold `nova_vertex` to false. An
/// interpolated `sun_mask` never exceeds its vertices', so a model none of whose vertices
/// reaches 1.5 never draws a pixel that does.
pub(super) fn nova_vertices(model: &Model) -> bool {
    model.vertices.iter().any(|v| v.sun_mask >= 1.5)
}

/// The value of `bindings.wesl`'s `emissive_rows` override for a pipeline whose `slots_present`
/// is `present`: `0` where no vertex can name a row, so every read of the table is its all-zero
/// row 0 (`Emissives::of`) and the driver can drop the tests on it.
pub(super) fn emissive_rows(present: u32) -> u32 {
    u32::from(present & MATERIAL_BITS != 0)
}

/// The half of a circuit's opaque list that is worth a pipeline of its own.
///
/// One `Model` is a whole circuit, so one rare material keeps every branch for every pixel: on
/// HD's Talon's Junction 17 emissive rows held the shading of the 93 % of its opaque indices that
/// name none, and Omega's Tech de Ra the same with 99.9 % (`docs/rendering/hires-gpu-levers.md`).
/// The draws naming no row and no weapon role shade through a pipeline told so.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PlainDraws {
    /// Per opaque draw, whether it is one of them.
    pub(super) draws: Vec<bool>,
    /// `slots_present` for those draws alone.
    pub(super) present: u32,
}

/// The plain half of `model`'s opaque list, or `None` where one pipeline already says everything:
/// none of the draws is plain, or all of them are.
pub(super) fn plain_draws(model: &Model) -> Option<PlainDraws> {
    plain_of(model, &model.draws)
}

/// [`plain_draws`] for `model`'s transparent list: on Omega's Tech de Ra 162 of its 166 blended
/// draws name no row, and the four that do held the road's decal layers on the rich shading.
pub(super) fn plain_transparent_draws(model: &Model) -> Option<PlainDraws> {
    plain_of(model, &model.transparent_draws)
}

fn plain_of(model: &Model, draws: &[crate::mesh::DrawCall]) -> Option<PlainDraws> {
    let carried = |draw: &crate::mesh::DrawCall| {
        model.indices[draw.range.start as usize..draw.range.end as usize]
            .iter()
            .fold(0, |all, &i| all | model.vertices[i as usize].slots)
    };
    split_plain(draws.iter().map(carried))
}

/// Whether every vertex the `plain` draws of `model` use is lit by the rig (`lit` exactly 1.0):
/// under a rig that is on, each of their pixels then weighs the rig's result at one.
pub(super) fn plain_is_lit(model: &Model, plain: &[bool]) -> bool {
    plain_list_is_lit(model, &model.draws, plain)
}

/// [`plain_is_lit`] over any of `model`'s lists.
pub(super) fn plain_list_is_lit(
    model: &Model,
    draws: &[crate::mesh::DrawCall],
    plain: &[bool],
) -> bool {
    draws
        .iter()
        .zip(plain)
        .filter(|&(_, &plain)| plain)
        .all(|(draw, _)| {
            every_one(
                model.indices[draw.range.start as usize..draw.range.end as usize]
                    .iter()
                    .map(|&index| model.vertices[index as usize].lit),
            )
        })
}

/// Whether each of `lit` is exactly 1.0.
fn every_one(mut lit: impl Iterator<Item = f32>) -> bool {
    lit.all(|lit| lit == 1.0)
}

/// [`plain_draws`] from each draw's OR of its vertices' `slots`.
fn split_plain(carried: impl Iterator<Item = u32>) -> Option<PlainDraws> {
    let mut present = 0;
    let mut rest = false;
    let draws: Vec<bool> = carried
        .map(|carried| {
            let plain = carried & (MATERIAL_BITS | WEAPON_ONLY) == 0;
            if plain {
                present |= carried;
            } else {
                rest = true;
            }
            plain
        })
        .collect();
    (rest && draws.contains(&true)).then(|| PlainDraws {
        draws,
        present: present_for(present),
    })
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
    fn the_plain_half_is_told_only_what_its_own_draws_carry() {
        let row = 3 << slots::MATERIAL_SHIFT;
        let glow = slots::ADD_SECOND | row;
        let plain = split_plain([slots::SECOND_IS_LIGHTMAP, glow, slots::NO_AMBIENT].into_iter())
            .expect("a list with both kinds splits");
        assert_eq!(plain.draws, [true, false, true]);
        assert_eq!(plain.present & slots::ADD_SECOND, 0);
        assert_eq!(emissive_rows(plain.present), 0);
        for carried in [slots::SECOND_IS_LIGHTMAP, slots::NO_AMBIENT] {
            assert_eq!(carried & plain.present, carried);
        }
    }

    #[test]
    fn a_list_of_one_kind_needs_no_second_pipeline() {
        assert_eq!(split_plain([slots::NO_AMBIENT, 0].into_iter()), None);
        assert_eq!(
            split_plain([1 << slots::MATERIAL_SHIFT, slots::RIM_GLOW].into_iter()),
            None
        );
    }

    #[test]
    fn one_prelit_or_half_lit_vertex_keeps_the_mix() {
        assert!(every_one([1.0, 1.0].into_iter()));
        assert!(!every_one([1.0, 0.0, 1.0].into_iter()));
        assert!(!every_one([1.0, 0.999].into_iter()));
    }

    #[test]
    fn a_weapon_effect_keeps_every_bit_and_its_table() {
        let present = present_for(slots::RIM_GLOW);
        assert_eq!(present, u32::MAX);
        assert_eq!(emissive_rows(present), 1);
    }
}
