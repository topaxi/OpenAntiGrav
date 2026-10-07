//! The picture a lightmapped HD material binds.

use oag_rcs::{rcsmaterial, rcsmodel};

/// The picture entry of a lightmapped material: the first entry that supplies
/// a `.gtf` and is not a lookup (see [`super::NOT_A_PICTURE`]), **among the entries
/// the resolved variant declares a sampler for**.
///
/// An entry whose sampler hash the variant never declares is not sampled by
/// that program at all - the engine binds that slot itself. Vineta K's
/// `d_s_n_customr` floor (`ds_dualparaboloid_c` at entry 0, `0x8365b1f3`,
/// declared by no variant) was bound as the albedo by the plain "first
/// non-lookup" rule, painting the wet floor black with a reflection map's
/// pattern while its real picture, `ds_floorplain_wet_cs`, sat at entry 1
/// under the declared `0x3bdc0403`. Where no variant resolves, or none of the
/// entries is declared, the plain rule answers as it always did.
pub(super) fn albedo(
    material: &rcsmodel::Material,
    variant: Option<rcsmaterial::Variant>,
    mut read: impl FnMut(&str) -> Option<Vec<u8>>,
) -> Option<usize> {
    let picture =
        |hash: &u32, path: &Option<String>| path.is_some() && !super::NOT_A_PICTURE.contains(hash);
    let declared = variant.and_then(|variant| {
        let blob = read(&material.name)?;
        rcsmaterial::Declared::parse(&blob, variant.fragment.offset)
    });
    declared
        .and_then(|declared| {
            material.samplers.iter().position(|(hash, path)| {
                picture(hash, path) && declared.samplers.iter().any(|&(h, _)| h == *hash)
            })
        })
        .or_else(|| {
            material
                .samplers
                .iter()
                .position(|(hash, path)| picture(hash, path))
        })
}
