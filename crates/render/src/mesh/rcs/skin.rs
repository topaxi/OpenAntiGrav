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
        let program = variants.get(slot).copied().flatten().and_then(|variant| {
            let blob = cache
                .entry(material.name.clone())
                .or_insert_with(|| textures(&format!("/{}", material.name)))
                .clone()?;
            Program::parse(&blob, variant.fragment.offset)
        });
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
            if colour.unit() == Some(1) {
                packed |= slots::ALBEDO_FROM_SECOND;
            }
            if let Texel::Unit {
                unit: unit @ (0 | 1),
                channel: Some(channel),
            } = texels[3]
            {
                packed &= !slots::alpha_channel(3);
                packed |= slots::alpha_channel(u32::from(channel));
                if unit == 1 {
                    packed |= slots::ALPHA_FROM_SECOND;
                }
            }
        }
        out.push(packed);
    }
    out
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
