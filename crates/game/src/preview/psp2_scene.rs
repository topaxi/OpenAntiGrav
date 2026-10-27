//! A `.vex` whose geometry is a PS4/Vita `.rcsmodel`: Omega's scenes.
//!
//! The one place a scene's motion (`.vex`) and its geometry (the `.rcsmodel`
//! beside it, 2048's container) are joined, for the menu backdrop
//! ([`crate::boot`]) and a selection screen's model ([`model`]): a circuit's on
//! `Track Select`, a craft's hull on `Ship Select`.
//!
//! **Scoped to the selection screens on purpose**: [`super::model`] stays as it
//! was, so no other caller (the flyer cards, the end-of-race placements) starts
//! drawing a PS4 scene it did not draw before.

use anyhow::{Context, Result};
use oag_mesh::mesh::{self, Model};

/// Builds `entry`'s model from `geometry` when it is a PS4/Vita `.rcsmodel`,
/// with the build's own report line, or `None` when it is HD's container.
///
/// # Errors
///
/// The container is recognised but will not decode.
pub fn build(
    archives: &mut oag_assets::Archives,
    entry: &str,
    vex: &[u8],
    geometry: &[u8],
) -> Result<Option<(Model, String)>> {
    Ok(build_named(archives, entry, vex, geometry)?.map(|(model, report, _)| (model, report)))
}

/// [`build`], and the path of each texture the build asked for, in the order it
/// asked: what a card hides its unread effects by (`crate::flyer`).
fn build_named(
    archives: &mut oag_assets::Archives,
    entry: &str,
    vex: &[u8],
    geometry: &[u8],
) -> Result<Option<(Model, String, Vec<String>)>> {
    if !mesh::rcs::psp2::is_psp2(geometry) {
        return Ok(None);
    }
    let mut textures = Vec::new();
    let (model, built) = mesh::rcs::psp2::build_with_vex(entry, geometry, vex, &mut |path| {
        // Materials are asked for too; a draw's texture slot counts the pictures only.
        if path.ends_with(".gnf") {
            textures.push(path.to_string());
        }
        archives.read_name(path).ok()
    })?;
    Ok(Some((model, built.describe(), textures)))
}

/// [`super::model_named`] for a flyer card: the PS4 scene when the `.rcsmodel`
/// beside `entry` is one, else the PS3 path. Scoped to the flyer cards, like
/// [`model`] to the selection screens: [`super::model_named`] stays as it was.
///
/// # Errors
///
/// The entry is missing or will not decode.
pub fn model_named(
    archives: &mut oag_assets::Archives,
    entry: &str,
) -> Result<(Model, Vec<String>)> {
    if let Some(sibling) = mesh::rcs::sibling_name(entry)
        && let Ok(geometry) = archives.read_name(&sibling)
    {
        let vex = archives.read_name(entry)?;
        if let Some((mut model, report, textures)) =
            build_named(archives, entry, &vex, &geometry)
                .with_context(|| format!("decoding the card model {entry}"))?
        {
            log::debug!("card model {entry}: {report}");
            model.keep_nearest();
            alpha_from_the_picture(&mut model);
            coverage_from_the_atlas(archives, &geometry, &mut model);
            return Ok((model, textures));
        }
    }
    super::model_named(archives, entry)
}

/// Reads a card's coverage from its picture's **alpha** channel.
///
/// The PS4 build leaves a vertex's coverage channel at `0` (the red channel), which
/// is not what `simpletextureandtexturealpha` samples: its one sampler is `Texture1` and
/// the alpha is that texture's own (`blitzed_grid.gnf` is white with the hexagons in its
/// alpha: read from red it draws a solid white box). Scoped to the flyer cards on
/// purpose, like [`model_named`]: the circuits' and hulls' blended draws keep the channel they
/// had until measured (a census lead, not measured).
fn alpha_from_the_picture(model: &mut Model) {
    use mesh::slots;
    for vertex in &mut model.vertices {
        if (vertex.slots >> 3) & 3 == 0 {
            vertex.slots |= slots::DEFAULT;
        }
    }
}

/// The `.gnf` sampler hash of the elements atlas on `basicalpha` and `scrollingalpha`:
/// the coverage, where the other sampler is the picture.
const ELEMENTS_SAMPLER: u32 = 2_821_073_761;

/// Binds the **base campaign's card materials** the way HD reads them: `basicalpha` and
/// `scrollingalpha` take their colour from one sampler and their coverage from the red
/// channel of the elements atlas (`ELEMENTS_SAMPLER`), unlit.
///
/// Inherited from HD, not measured on PS4: HD's own build gives both materials
/// `ALPHA_FROM_SECOND`, alpha channel 0, no ambient and no sun (`0xc4`), with the swatch
/// first and the atlas second; Omega ships the same two material names and the same
/// textures with the samplers in the other order, which is why the build's pick (the first
/// in address order, the atlas) draws a white strip and dark blocks where the wordmark and
/// the edge stripes go. Scoped to the flyer cards, like [`alpha_from_the_picture`].
fn coverage_from_the_atlas(
    archives: &mut oag_assets::Archives,
    geometry: &[u8],
    model: &mut Model,
) {
    use mesh::slots;
    let Ok(decoded) = oag_rcs::rcsmodel::psp2::parse(geometry) else {
        return;
    };
    // The build gives each distinct material its own texture slot, in the order the
    // submeshes first name it.
    let mut materials: Vec<usize> = Vec::new();
    for material in decoded.submeshes.iter().filter_map(|sub| sub.material) {
        if !materials.contains(&material) {
            materials.push(material);
        }
    }
    let mut read = |path: &str| -> Option<std::sync::Arc<mesh::ModelTexture>> {
        let blob = archives.read_name(path).ok()?;
        mesh::ModelTexture::from_gnf(path, &blob).map(std::sync::Arc::new)
    };
    model.lightmaps.resize(model.textures.len(), None);
    for (slot, &index) in materials.iter().enumerate() {
        let material = &decoded.materials[index];
        let name = material.name.as_str();
        if !(name.ends_with("/basicalpha.rcsmaterial")
            || name.ends_with("/scrollingalpha.rcsmaterial"))
        {
            continue;
        }
        let atlas = material.sampler(ELEMENTS_SAMPLER);
        let picture = material
            .samplers
            .iter()
            .find(|(hash, _)| *hash != ELEMENTS_SAMPLER)
            .map(|(_, path)| path.as_str());
        let (Some(atlas), Some(picture)) = (atlas, picture) else {
            continue;
        };
        let (Some(picture_texture), Some(atlas_texture)) = (read(picture), read(atlas)) else {
            continue;
        };
        if slot >= model.textures.len() {
            continue;
        }
        model.textures[slot] = Some(picture_texture);
        model.lightmaps[slot] = Some(atlas_texture);
        for draw in model
            .draws
            .iter()
            .chain(&model.alpha_tested_draws)
            .chain(&model.transparent_draws)
            .filter(|draw| draw.texture == Some(slot))
        {
            let range = draw.range.start as usize..draw.range.end as usize;
            for &i in &model.indices[range] {
                let vertex = &mut model.vertices[i as usize];
                vertex.slots = (vertex.slots & !slots::alpha_channel(3))
                    | slots::ALPHA_FROM_SECOND
                    | slots::NO_AMBIENT
                    | slots::NO_SUN;
            }
        }
    }
}

/// A selection screen's model, a circuit or a craft's hull: the PS4 scene when
/// the `.rcsmodel` beside `entry` is one, else [`super::model`].
///
/// The sibling is asked rather than the `.vex`, because not every Omega `.vex`
/// carries the header flag HD's do (Talons Junction, Amphiseum, Modesto
/// Heights, Tech De Ra and 2048's ten do not).
///
/// # Errors
///
/// The entry is missing or will not decode.
pub fn model(archives: &mut oag_assets::Archives, entry: &str) -> Result<Model> {
    if let Some(sibling) = mesh::rcs::sibling_name(entry)
        && let Ok(geometry) = archives.read_name(&sibling)
    {
        let vex = archives.read_name(entry)?;
        if let Some((mut model, report)) = build(archives, entry, &vex, &geometry)
            .with_context(|| format!("decoding the selection model {entry}"))?
        {
            log::debug!("selection model {entry}: {report}");
            model.keep_nearest();
            return Ok(model);
        }
    }
    super::model(archives, entry)
}
