//! What a model's pipelines are told about its vertices, so the driver can drop the branches of
//! `shade.wesl` none of them takes. Everything here keeps the picture exact: the values are
//! supersets of what the vertices carry, and a read they rule out is a read of zero.

use crate::mesh::{Model, PROGRAM_BILLBOARD, PROGRAM_WATER, slots};

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

/// The `slots` bits whose role reads the second texture at the diffuse coordinate: the albedo and
/// the alpha taken from it, and the facing-ramp sheen's grid.
const SECOND_ROLES: u32 =
    slots::ALBEDO_FROM_SECOND | slots::ALPHA_FROM_SECOND | slots::FACING_RAMP_SHEEN;

/// Whether a vertex `glow` value names the second texture as its mask's source: `shade.wesl`'s
/// `program_glow` decode of `mesh::rcs::glow::texel_glow`, source 1.
fn glow_reads_second(glow: f32) -> bool {
    glow < 0.0 && ((-glow / 8.0).floor() as u32) >> 2 == 1
}

/// The value of `shade.wesl`'s `second_read` override for the `plain` draws of `draws`, whose
/// `slots_present` is `present`: `false` where nothing they draw reads the second texture at the
/// diffuse coordinate. Its readers are [`SECOND_ROLES`], a glow mask sourced from it, and the
/// pane and twin programs, which need an emissive row (`emissive_rows`).
pub(super) fn reads_second(
    model: &Model,
    draws: &[crate::mesh::DrawCall],
    plain: &[bool],
    present: u32,
) -> bool {
    if present & SECOND_ROLES != 0 || emissive_rows(present) != 0 {
        return true;
    }
    draws
        .iter()
        .zip(plain)
        .filter(|&(_, &plain)| plain)
        .any(|(draw, _)| {
            model.indices[draw.range.start as usize..draw.range.end as usize]
                .iter()
                .any(|&index| glow_reads_second(model.vertices[index as usize].glow))
        })
}

/// The constants a `build` takes from the open scope for `model`: the scope's own, and
/// `zone_stage_light` for a circuit (`prepass`) of a race whose frame lights it by the stage,
/// where [`zone_light_exact`].
pub(super) fn scope_constants_for(model: &Model, prepass: bool) -> Vec<(&'static str, f64)> {
    let mut constants = super::pipeline_cache::scope_constants();
    if prepass && super::pipeline_cache::scope_zone_stage_light() && zone_light_exact(model) {
        constants.push(("zone_stage_light", 1.0));
    }
    constants
}

/// Whether `shade.wesl`'s `zone_stage_light` draws every vertex of `model` as the Zone branches
/// it replaces do. It gives each chunk the stage's light, and an emissive plain chunk
/// (`ZONE_PLAIN` with `EMISSIVE`) the `1.0` and no sun its own program leaves: so no such vertex
/// may be one an earlier branch relights (the billboard and water programs, and `water_test_2`'s
/// bit).
pub(super) fn zone_light_exact(model: &Model) -> bool {
    let rows = super::Emissives::of(model);
    model
        .vertices
        .iter()
        .all(|vertex| vertex_keeps_own_light(vertex.slots, &rows))
}

/// [`zone_light_exact`] for one vertex's `slots`, `rows` the model's emissive table.
fn vertex_keeps_own_light(carried: u32, rows: &super::Emissives) -> bool {
    let plain_emissive = slots::ZONE_PLAIN | slots::EMISSIVE;
    if carried & plain_emissive != plain_emissive {
        return true;
    }
    let row = (carried >> slots::MATERIAL_SHIFT) & slots::MATERIAL_MASK;
    let program = rows.scale[row as usize][2];
    let relit = |code: f32| (program - code).abs() < 0.5;
    carried & slots::ICE == 0 && !relit(PROGRAM_BILLBOARD) && !relit(PROGRAM_WATER)
}

/// Whether any vertex of `model` carries a tangent (`GpuVertex::tangent`'s `xyz` bytes not all
/// zero, the test `vs_main` makes): `false` lets its pipelines fold the normal-map branches.
pub(super) fn tangents_present(model: &Model) -> bool {
    model.vertices.iter().any(|v| v.tangent & 0x00ff_ffff != 0)
}

/// Whether any vertex of `model` is a 2048 vertex-lit one (`sun_mask` 2 or above, read by
/// `shade.wesl` as `>= 1.5`): `false` lets its pipelines fold `nova_vertex` to false. An
/// interpolated `sun_mask` never exceeds its vertices', so a model none of whose vertices
/// reaches 1.5 never draws a pixel that does.
pub(super) fn nova_vertices(model: &Model) -> bool {
    model.vertices.iter().any(|v| v.sun_mask >= 1.5)
}

/// Whether any row of `model`'s emissive table, as uploaded (`Emissives::of`), carries a program
/// (`Emissive::program`, the `zw` of its `scale` row): `false` lets its pipelines read every
/// row's `zw` as the zero they all are, so every program test on the table folds. Nothing
/// rewrites those lanes after the upload (the pad cycle writes tints alone).
pub(super) fn programs_present(model: &Model) -> bool {
    super::Emissives::of(model)
        .scale
        .iter()
        .any(|row| row[2] != 0.0 || row[3] != 0.0)
}

/// The values of `bindings.wesl`'s `program_codes` and `rate_codes` overrides for `model`: bit
/// `k` where a row of its table as uploaded (`Emissives::of`) has a program (`scale.z`) or a rate
/// (`scale.y`) nearest `k`, bit 31 for any from 30.5 up. Every kind test on a row asks the bit of
/// the code it tests and a row's own bit is always set, so the masks draw the same picture as all
/// bits; a kind no row carries compiles out. Nothing rewrites those lanes after the upload
/// ([`programs_present`]).
pub(super) fn row_codes(model: &Model) -> (u32, u32) {
    super::Emissives::of(model)
        .scale
        .iter()
        .fold((0, 0), |(programs, rates), row| {
            (programs | code_bit(row[2]), rates | code_bit(row[1]))
        })
}

/// The bit of the code nearest `value` (`program_in`'s and `rate_in`'s ranges), 31 for any from
/// 30.5 up; none for a value no test takes (below -0.5, or not a number).
fn code_bit(value: f32) -> u32 {
    if value.is_nan() || value <= -0.5 {
        return 0;
    }
    1 << ((value + 0.5).floor() as u32).min(31)
}

/// Whether every vertex of `model` has `lit` exactly 1.0: under a rig that is on, each pixel of
/// every one of its draws then weighs the rig's result at one ([`plain_is_lit`] for a whole model).
pub(super) fn every_vertex_lit(model: &Model) -> bool {
    every_one(model.vertices.iter().map(|v| v.lit))
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
    fn the_stage_light_refuses_a_plain_emissive_vertex_an_earlier_branch_relights() {
        let plain_emissive = slots::ZONE_PLAIN | slots::EMISSIVE;
        let row = 1 << slots::MATERIAL_SHIFT;
        let exact = |carried: u32, program: f32| {
            let mut rows = super::super::Emissives::default();
            rows.scale[1][2] = program;
            vertex_keeps_own_light(carried, &rows)
        };
        assert!(exact(plain_emissive, 0.0));
        assert!(exact(plain_emissive | row, 12.0));
        assert!(!exact(plain_emissive | row, PROGRAM_BILLBOARD));
        assert!(!exact(plain_emissive | row, PROGRAM_WATER));
        assert!(!exact(plain_emissive | slots::ICE, 0.0));
        // Only an emissive plain chunk takes neither Zone branch.
        assert!(exact(slots::ZONE_PLAIN | row, PROGRAM_BILLBOARD));
        assert!(exact(slots::EMISSIVE | row, PROGRAM_WATER));
    }

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
    fn only_a_glow_mask_from_the_second_texture_reads_it() {
        // `mesh::rcs::glow::texel_glow`'s packing, `-(scale + 8 * (4 * source + channel))`.
        let texel =
            |source: f32, channel: f32, scale: f32| -(scale + 8.0 * (4.0 * source + channel));
        assert!(glow_reads_second(texel(1.0, 3.0, 7.5)));
        assert!(glow_reads_second(texel(1.0, 0.0, 0.0)));
        for source in [0.0, 2.0, 3.0] {
            assert!(!glow_reads_second(texel(source, 2.0, 1.0)));
        }
        assert!(!glow_reads_second(0.5));
        assert!(!glow_reads_second(0.0));
    }

    /// Each row's program and rate set the bit of the code a test in `bindings.wesl` would see
    /// them as: the nearest whole number, every value from 30.5 up as 31, and row 0's zeros.
    #[test]
    fn a_table_sets_the_bit_of_each_code_its_rows_carry() {
        let mut model = Model::none("circuit");
        assert_eq!(row_codes(&model), (1, 1));
        let row = |rate: f32, program: f32| crate::mesh::Emissive {
            rate,
            ..glow([program, 0.0])
        };
        model.emissive = vec![row(9.0, 12.0), row(1.0, 17.2), row(40.0, 0.6)];
        let (programs, rates) = row_codes(&model);
        assert_eq!(programs, 1 | 1 << 12 | 1 << 17 | 1 << 1);
        assert_eq!(rates, 1 | 1 << 9 | 1 << 1 | 1 << 31);
        assert_eq!(code_bit(-0.6), 0);
        assert_eq!(code_bit(f32::NAN), 0);
        assert_eq!(code_bit(2.49), 1 << 2);
    }

    #[test]
    fn a_weapon_effect_keeps_every_bit_and_its_table() {
        let present = present_for(slots::RIM_GLOW);
        assert_eq!(present, u32::MAX);
        assert_eq!(emissive_rows(present), 1);
    }

    fn glow(program: [f32; 2]) -> crate::mesh::Emissive {
        crate::mesh::Emissive {
            tint: [1.0, 0.5, 0.25],
            offset: 0.5,
            scale: 2.0,
            rate: 1.0,
            program,
        }
    }

    /// A craft hull names rows for its glow alone (HD's twelve hulls: every program `[0, 0]`),
    /// so the table's program lanes are all zero and fold; one program anywhere keeps them.
    #[test]
    fn a_table_of_plain_glows_carries_no_program() {
        let mut model = Model::none("hull");
        assert!(!programs_present(&model));
        model.emissive = vec![glow([0.0, 0.0]), glow([0.0, 0.0])];
        assert!(!programs_present(&model));
        model.emissive.push(glow([crate::mesh::PROGRAM_WATER, 0.0]));
        assert!(programs_present(&model));
        model.emissive = vec![glow([0.0, 2.0])];
        assert!(programs_present(&model));
    }

    #[test]
    fn a_hull_is_lit_by_the_rig_only_where_every_vertex_is() {
        let vertex = |lit: f32| crate::mesh::GpuVertex {
            lit,
            ..bytemuck::Zeroable::zeroed()
        };
        let mut model = Model::none("hull");
        model.vertices = vec![vertex(1.0), vertex(1.0)];
        assert!(every_vertex_lit(&model));
        model.vertices.push(vertex(0.0));
        assert!(!every_vertex_lit(&model));
    }
}
