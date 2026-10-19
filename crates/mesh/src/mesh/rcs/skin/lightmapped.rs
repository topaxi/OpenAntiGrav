//! The picture a lightmapped HD material binds.

use oag_rcs::{rcsmaterial, rcsmodel};

/// What a lightmapped material's picture binding resolves to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Albedo {
    /// This entry is the picture.
    Entry(usize),
    /// The resolved program samples no colour texture: every texture the
    /// material names is a lookup, a normal map or the lightmap, and its colour
    /// is vertex colour, constants and light. Nothing is bound.
    Unsampled,
    /// No variant resolved, so nothing is known: the caller's default stands.
    Unresolved,
}

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
///
/// **A declared entry the program only reads one lane wide is not a picture
/// either** ([`rcsmaterial::fragment::Program::samples_colour`]): Sebenco's
/// `sebenco_ice` declares `and_ice1` at unit 0 and reads `TEX H1.x` from it, a
/// specular-power scalar, so the first-declared rule bound a purple scalar map
/// as the pool's colour. The first entry the program reads as a colour is
/// preferred, the first declared one next.
///
/// **Nor is one it unpacks as a tangent-space normal**
/// ([`rcsmaterial::fragment::Program::samples_as_normal`]), and a resolved
/// program with no picture left is [`Albedo::Unsampled`] rather than entry 0.
/// `constantdiffuse_specular_normal` (its `ds_sfblend_ns` is the start line on
/// twelve circuits) names only a normal-and-specular map at `0x3bdc0403` and the
/// lightmap; entry 0 drew that map as the colour, a purple ridged strip across
/// the road.
pub(super) fn albedo(
    material: &rcsmodel::Material,
    variant: Option<rcsmaterial::Variant>,
    mut read: impl FnMut(&str) -> Option<Vec<u8>>,
) -> Albedo {
    let picture =
        |hash: &u32, path: &Option<String>| path.is_some() && super::is_picture(material, *hash);
    let parsed = variant.and_then(|variant| {
        let blob = read(&material.name)?;
        let declared = rcsmaterial::Declared::parse(&blob, variant.fragment.offset)?;
        let program = rcsmaterial::fragment::Program::parse(&blob, variant.fragment.offset)?;
        Some((declared, program))
    });
    let Some((declared, program)) = parsed else {
        return material
            .samplers
            .iter()
            .position(|(hash, path)| picture(hash, path))
            .map_or(Albedo::Unresolved, Albedo::Entry);
    };
    if let Some(entry) = declared_picture(material, &declared, &program, false)
        .or_else(|| declared_picture(material, &declared, &program, true))
    {
        return Albedo::Entry(entry);
    }
    material
        .samplers
        .iter()
        .position(|(hash, path)| {
            picture(hash, path)
                && declared
                    .samplers
                    .iter()
                    .find_map(|&(h, unit)| (h == *hash).then_some(unit))
                    .is_none_or(|unit| !program.samples_as_normal(u8::try_from(unit).unwrap_or(0)))
        })
        .map_or(Albedo::Unsampled, Albedo::Entry)
}

/// The first entry that is a picture, is declared by the resolved variant, and
/// - unless `any_read` - is sampled by `program` as a colour.
pub(super) fn declared_picture(
    material: &rcsmodel::Material,
    declared: &rcsmaterial::Declared,
    program: &rcsmaterial::fragment::Program,
    any_read: bool,
) -> Option<usize> {
    material.samplers.iter().position(|(hash, path)| {
        path.is_some()
            && super::is_picture(material, *hash)
            && declared
                .samplers
                .iter()
                .find_map(|&(h, unit)| (h == *hash).then_some(unit))
                .is_some_and(|unit| {
                    let unit = u8::try_from(unit).unwrap_or(0);
                    !program.samples_as_normal(unit) && (any_read || program.samples_colour(unit))
                })
    })
}
