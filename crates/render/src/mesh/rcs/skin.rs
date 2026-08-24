//! What each material slot contributes: its textures, and the shader variant
//! the lit-race key resolves to.
//!
//! Split out of `mesh/rcs.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change.

use oag_formats::{gtf, rcsmaterial, rcsmodel};

use super::super::ModelTexture;
use super::{Report, Textures};

/// Decodes one material's texture, or says which way it could not be.
///
/// **A `.gtf` that will not decode draws nothing rather than something.**
/// `Texture::to_rgba` refuses the RSX's Morton-swizzled layouts and cubemaps -
/// 53 of the disc's 7,333 files - and a refusal here leaves the slot `None`,
/// which `mesh_render::build` binds its white 1x1 for. That is the same white
/// sheet this module has been removing, so it is counted in
/// [`Report::untextured`] rather than left to be discovered in a screenshot.
pub(super) fn decode_texture(label: &str, blob: &[u8]) -> Option<ModelTexture> {
    let parsed = gtf::Gtf::parse(blob).ok()?;
    let texture = parsed.only()?;
    let rgba = texture.to_rgba(blob).ok()?;
    let (width, height) = texture.level_size(0);
    Some(ModelTexture {
        label: label.to_string(),
        width,
        height,
        rgba: rgba.into_iter().flatten().collect(),
    })
}

/// One texture slot per material, in material-table order.
///
/// Positional and never compacted, because a chunk names its material by
/// ordinal - dropping the ones that fail to decode would re-skin the model.
///
/// Returns the first texture and the **second**, whatever its role. Until
/// 2026-08-20 the second was loaded and dropped unless it was the circuit's
/// lightmap, on the grounds that its role was unread; [`roles`] reads it now,
/// so the texture has to be there for the role to mean anything.
pub(super) fn skin(
    model: &rcsmodel::Model,
    textures: Textures<'_>,
    report: &mut Report,
) -> (Vec<Option<ModelTexture>>, Vec<Option<ModelTexture>>) {
    let mut cache: std::collections::HashMap<String, Option<ModelTexture>> = Default::default();
    let mut load = |path: &str, textures: Textures<'_>| {
        // A circuit's 442 materials name far fewer distinct textures, and
        // decoding a 2048x2048 DXT5 twice is the cost this avoids. The two
        // slots share the cache because a lightmap atlas is named by dozens of
        // materials at once.
        cache
            .entry(path.to_string())
            .or_insert_with(|| {
                textures(path)
                    .as_deref()
                    .and_then(|blob| decode_texture(path, blob))
            })
            .clone()
    };
    let mut skins = Vec::with_capacity(model.materials.len());
    let mut seconds = Vec::with_capacity(model.materials.len());
    for material in &model.materials {
        if material.texture.is_empty() {
            report.untextured += 1;
            skins.push(None);
        } else {
            let decoded = load(&material.texture, textures);
            if decoded.is_none() {
                report.untextured += 1;
            }
            skins.push(decoded);
        }
        // The lightmap is still counted apart from every other use of the
        // slot, because the two are answered by different evidence: the
        // lightmap by `Material::lightmap`'s four-signal path reading, and
        // everything else by the material's own microcode. See
        // `oag_formats::rcsmodel::Material::lightmap` and [`roles`].
        let second = material
            .second_texture
            .as_deref()
            .map(|path| load(path, textures));
        match (material.lightmap().is_some(), &second) {
            (true, Some(Some(_))) => report.lightmapped += 1,
            // Named and did not decode: counted apart, because a lightmap that
            // silently fails to load leaves the surface at full brightness,
            // which is what an unlit surface looks like anyway.
            (true, _) => report.lightmap_undecoded += 1,
            (false, Some(Some(_))) => report.second_texture_loaded += 1,
            (false, Some(None)) => report.second_texture_unread += 1,
            (false, None) => {}
        }
        seconds.push(second.flatten());
    }
    (skins, seconds)
}

/// What each material slot's own microcode says its two texture units are for,
/// packed as [`crate::mesh::slots`], in material-table order.
///
/// **The reading this project spent a year without.** A `.rcsmaterial` is a
/// table of shader variants; [`variants`] already resolves which one an
/// ordinary lit race draws through, and that variant's fragment program states
/// outright which unit it samples for the picture and which unit and channel it
/// writes to the output alpha.
/// `oag_formats::rcsmaterial::fragment::Program::output_texels` traces it, and
/// this turns the answer into the four bits `mesh.wgsl` decodes.
///
/// **Only a positive reading is acted on.** A lane the taint could not follow
/// answers [`rcsmaterial::fragment::Texel::Untraced`] - a constant, an
/// interpolator, or an opcode this decoder does not model - and `Untraced` is
/// not the same claim as "opaque". Those slots keep
/// [`crate::mesh::slots::DEFAULT`], which is exactly what this renderer did
/// before any of this existed, so the reading can only add correct surfaces
/// and never take a working one away. The same rule covers a unit above 1: a
/// `.rcsmodel` material names two textures and some programs sample four, so
/// an alpha traced to unit 2 is recorded by the census and not acted on here.
pub(super) fn roles(
    model: &rcsmodel::Model,
    variants: &[Option<rcsmaterial::Variant>],
    seconds: &[Option<ModelTexture>],
    textures: Textures<'_>,
) -> Vec<u32> {
    use crate::mesh::slots;
    use rcsmaterial::fragment::{Program, Texel};

    let mut cache: std::collections::HashMap<String, Option<Vec<u8>>> = Default::default();
    let mut out = Vec::with_capacity(model.materials.len());
    for (slot, material) in model.materials.iter().enumerate() {
        let mut packed = slots::DEFAULT;
        if material.lightmap().is_some() {
            packed |= slots::SECOND_IS_LIGHTMAP;
        }
        let variant = variants.get(slot).copied().flatten();
        let blob = variant.and_then(|_| {
            cache
                .entry(material.name.clone())
                .or_insert_with(|| textures(&format!("/{}", material.name)))
                .clone()
        });
        let program = variant
            .zip(blob.as_deref())
            .and_then(|(variant, blob)| Program::parse(blob, variant.fragment.offset));
        let declared = variant.zip(blob.as_deref()).and_then(|(variant, blob)| {
            rcsmaterial::Declared::parse(blob, variant.fragment.offset)
        });
        let (first_unit, second_unit) = units(declared.as_ref(), material);
        // **Only where there is a second texture to point at.** A program's
        // unit 1 and a `.rcsmodel` material's second slot are not the same
        // thing: 40 of the disc's materials sample two units while naming one
        // texture, and pointing the albedo at a unit nothing is bound to
        // samples `mesh_render::build`'s black placeholder - which is a
        // black surface, the loudest possible way to be wrong. Caught by the
        // picture on the first run of this reading.
        let bound = seconds.get(slot).is_some_and(Option::is_some);
        if let (Some(program), true) = (program, bound) {
            let texels = program.output_texels();
            let colour = texels[0].merge(texels[1]).merge(texels[2]);
            // **Never the lightmap.** When the second slot is the circuit's
            // baked atlas, its alpha is the sun-occlusion mask - `mesh.wgsl`
            // reads it as exactly that, `mask = baked.a * in.sun_mask` - and
            // its RGB is a light term, not a picture. Pointing coverage or
            // albedo at it paints a shadow map as a stencil. Measured on the
            // frame: without this guard nine of Talon's Junction's
            // lightmapped slots took `ALPHA_FROM_SECOND`, and the circuit's
            // perforated trackside barrier beside the blimp lost its holes
            // and washed out at tick 295.
            let lightmapped = packed & slots::SECOND_IS_LIGHTMAP != 0;
            let is_second = |unit: u32| !lightmapped && unit == second_unit;
            if colour.unit().map(u32::from).is_some_and(is_second) {
                packed |= slots::ALBEDO_FROM_SECOND;
            }
            if let Texel::Unit {
                unit,
                channel: Some(channel),
            } = texels[3]
                && (u32::from(unit) == first_unit || is_second(u32::from(unit)))
            {
                packed &= !slots::alpha_channel(3);
                packed |= slots::alpha_channel(u32::from(channel));
                if is_second(u32::from(unit)) {
                    packed |= slots::ALPHA_FROM_SECOND;
                }
            }
        }
        out.push(packed);
    }
    out
}

/// Which texture unit each of a material's two `.gtf` slots reaches, read from
/// the **sampler name hash** the slot carries beside its path
/// (`rcsmodel::Material::texture_sampler`) against the resolved variant's own
/// sampler declaration.
///
/// # Why this is not the slot's ordinal
///
/// **Confidence 80**, and the number that earns it: of the disc's 13,933
/// populated texture slots, 8,021 carry a hash the material's own lit-race
/// variant declares as a sampler - and **only 776 of those land on the slot's
/// own ordinal**. The first slot reaches unit 1 on 4,751 of them. Reading the
/// hash is therefore closer to the file than counting slots is, on nine slots
/// out of ten. `docs/formats/rcsmaterial.md`, "A texture slot names its
/// sampler, and the slot's ordinal is not its unit", carries the sweep and the
/// three preimages (`Texture1`, `diffuse`, `lightmap`) that identify the field.
///
/// # Why the caller accepts this *and* the ordinal
///
/// **The hash adds identifications; it never removes them**, and that is a
/// measurement rather than caution. Two things break if it replaces the
/// ordinal outright. Talon's Junction's cloud plate is the first: its second
/// slot's hash is `lightmap`, its resolved variant declares no `lightmap` at
/// all, and its microcode plainly samples `cloud mask.gtf` at unit 1 - the
/// reading `docs/formats/rcsmaterial.md` traced end to end and this renderer
/// already draws correctly. The second showed up in a frame: replacing the
/// ordinal moved 27 of Talon's Junction's 442 slots off
/// `ALPHA_FROM_SECOND`, and the circuit's perforated trackside barrier - the
/// one beside the blimp at tick 295 - lost its holes and washed out. So
/// [`roles`] treats a unit as the second slot if *either* reading says so.
///
/// Where the variant declares no such sampler at all, this answers the
/// ordinal, which is what the caller would have used anyway.
fn units(declared: Option<&rcsmaterial::Declared>, material: &rcsmodel::Material) -> (u32, u32) {
    let unit_of = |hash: Option<u32>| {
        let (declared, hash) = (declared?, hash?);
        declared
            .samplers
            .iter()
            .find(|(h, _)| *h == hash)
            .map(|&(_, unit)| unit)
    };
    let first = unit_of((!material.texture.is_empty()).then_some(material.texture_sampler));
    let second = unit_of(material.second_texture_sampler);
    // A material whose two slots resolve to the same unit is a reading that
    // cannot be right, and taking it would paint the first texture through the
    // second's role. The ordinal answers for the second slot there.
    match (first.unwrap_or(0), second) {
        (f, Some(s)) if s != f => (f, s),
        (f, _) => (f, u32::from(f != 1)),
    }
}

/// Which shader variant each material slot resolves to, in material-table
/// order.
///
/// **The first use of the variant key this project can read.** A material is a
/// table of up to 68 shader variants and the original picks one by a two-part
/// key - see `oag_formats::rcsmaterial`. Half of it a chunk decides, from its
/// own vertex declaration; half a render pass decides, and for an ordinary lit
/// race that half is [`rcsmaterial::LIT_RACE_PASS`].
///
/// Resolving it **per material slot** rather than per draw is sound because a
/// slot never has to be two things at once: measured over all 123 `.rcsmodel`
/// on the disc and the 3,566 slots they use, zero serve chunks whose
/// chunk-determined keys differ (`a_material_slot_never_needs_two_different_variants`).
/// The first chunk naming a slot therefore answers for all of them.
///
/// Nothing shades differently for this yet. It is carried so the shader step is
/// a shader step, and counted so the loader report says how much of a circuit
/// the reading actually reaches.
pub(super) fn variants(
    model: &rcsmodel::Model,
    textures: Textures<'_>,
    report: &mut Report,
) -> Vec<Option<rcsmaterial::Variant>> {
    // One representative chunk per slot, which the invariant above licenses,
    // and how many chunks each slot serves - because the slot is not the
    // meaningful unit. Talon's Junction declares 442 materials and only 302 are
    // named by any chunk, so counting over all of them dilutes the answer with
    // slots nothing draws.
    let mut decl_of: std::collections::HashMap<u32, Option<&rcsmodel::VertexDecl>> =
        Default::default();
    let mut chunks_of: std::collections::HashMap<u32, usize> = Default::default();
    for mesh in &model.meshes {
        decl_of.entry(mesh.material).or_insert(mesh.decl.as_ref());
        *chunks_of.entry(mesh.material).or_default() += 1;
    }

    let mut cache: std::collections::HashMap<String, Option<rcsmaterial::RcsMaterial>> =
        Default::default();
    let mut out = Vec::with_capacity(model.materials.len());
    for (slot, material) in model.materials.iter().enumerate() {
        let ordinal = u32::try_from(slot).unwrap_or(u32::MAX);
        let chunks = chunks_of.get(&ordinal).copied().unwrap_or(0);
        if chunks == 0 {
            // Declared and never drawn: not a gap in this reading, so it is not
            // counted as one.
            out.push(None);
            continue;
        }
        // The material's own name is its path in the archive.
        let parsed = cache
            .entry(material.name.clone())
            .or_insert_with(|| {
                textures(&format!("/{}", material.name))
                    .as_deref()
                    .and_then(|blob| rcsmaterial::RcsMaterial::parse(blob).ok())
            })
            .clone();
        let Some(parsed) = parsed else {
            report.materials_unread += 1;
            out.push(None);
            continue;
        };
        let decl = decl_of.get(&ordinal).copied().flatten();
        // Numerically, not by union: bits 1-2 are a four-way field and
        // `Features::with` would leave both `Ambient` and `IleLightmap` set,
        // naming a permutation nothing ships. See `Features::chunk_word`.
        let word = rcsmaterial::Features::chunk_word(rcsmaterial::LIT_RACE_PASS, decl);
        let key = rcsmaterial::Features::from_pass_word(word);
        // `Static` is what every world chunk uses; `StaticQuake` is the same
        // programs with the Quake weapon's displacement in front, and nothing
        // here fires that weapon.
        match parsed.variant(rcsmaterial::Class::Static, key) {
            Some(v) => {
                report.variants_resolved += 1;
                report.variant_chunks += chunks;
                out.push(Some(*v));
            }
            None => {
                report.variants_unshipped += 1;
                report.variant_chunks_missed += chunks;
                out.push(None);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn declared(samplers: &[(u32, u32)]) -> rcsmaterial::Declared {
        rcsmaterial::Declared {
            parameters: Vec::new(),
            samplers: samplers.to_vec(),
        }
    }

    fn material(first: u32, second: Option<u32>) -> rcsmodel::Material {
        rcsmodel::Material {
            name: "m.rcsmaterial".to_string(),
            state: 1,
            src_factor: 0,
            dst_factor: 0,
            texture: "a.gtf".to_string(),
            second_texture: second.map(|_| "b.gtf".to_string()),
            texture_sampler: first,
            second_texture_sampler: second,
            samplers: Vec::new(),
            parameters: Vec::new(),
        }
    }

    /// The declared binding wins over the ordinal, which is the whole point:
    /// Talon's Junction's `track_surface` puts its first texture at unit 1 and
    /// its second at unit 2.
    #[test]
    fn a_slots_sampler_hash_names_its_unit_and_the_ordinal_does_not() {
        let d = declared(&[(0x11cb_4f74, 0), (0x739a_786e, 1), (0x37b5_db58, 2)]);
        assert_eq!(
            units(Some(&d), &material(0x739a_786e, Some(0x37b5_db58))),
            (1, 2)
        );
    }

    /// No declaration to read - no variant resolved, or a block this reader
    /// cannot frame - answers exactly what the caller would have assumed.
    #[test]
    fn with_nothing_declared_the_ordinal_answers() {
        assert_eq!(
            units(None, &material(0x3bdc_0403, Some(0x37b5_db58))),
            (0, 1)
        );
    }

    /// The cloud plate's shape: the first slot's hash is declared, the second
    /// slot's is not, and the second falls back to the ordinal rather than
    /// going unknown - which is what keeps that surface drawing.
    #[test]
    fn an_undeclared_second_slot_keeps_the_ordinal() {
        let d = declared(&[(0x515e_298e, 0), (0xfd66_9142, 1)]);
        assert_eq!(
            units(Some(&d), &material(0x515e_298e, Some(0x37b5_db58))),
            (0, 1)
        );
    }

    /// Both slots resolving to one unit cannot be right, and taking it would
    /// paint the first texture through the second's role.
    #[test]
    fn two_slots_on_one_unit_is_refused() {
        let d = declared(&[(0xaaaa_aaaa, 1)]);
        assert_eq!(
            units(Some(&d), &material(0xaaaa_aaaa, Some(0xaaaa_aaaa))),
            (1, 0)
        );
    }
}
