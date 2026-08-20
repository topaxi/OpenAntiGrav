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
    let mut lightmaps = Vec::with_capacity(model.materials.len());
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
        // **Only the slot the material identifies as a lightmap**, which is a
        // reading rather than a preference for the second texture - see
        // `oag_formats::rcsmodel::Material::lightmap`. A second texture that is
        // an emissive map, a normal map or a coverage mask stays unsampled.
        let lit = material.lightmap().map(|path| load(path, textures));
        match &lit {
            Some(Some(_)) => report.lightmapped += 1,
            // Named and did not decode: counted apart, because a lightmap that
            // silently fails to load leaves the surface at full brightness,
            // which is what an unlit surface looks like anyway.
            Some(None) => report.lightmap_undecoded += 1,
            None => {
                // Not a lightmap by the four-signal reading above, but a
                // second texture may still be named - see
                // `docs/formats/rcsmaterial.md`, "The glass family's second
                // slot: traced, not solved". Decoded and counted so the load
                // report says a second slot exists and is unread rather than
                // that nothing does; not bound to a sampler, because which
                // texture unit the resolved shader actually reads it through,
                // and by what operation, is not confirmed - drawing it would
                // be a guess at the picture, not a reading of it.
                if let Some(path) = material.second_texture.as_deref() {
                    match load(path, textures) {
                        Some(_) => report.second_texture_loaded += 1,
                        None => report.second_texture_unread += 1,
                    }
                }
            }
        }
        lightmaps.push(lit.flatten());
    }
    (skins, lightmaps)
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
