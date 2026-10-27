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
            return Ok((model, textures));
        }
    }
    super::model_named(archives, entry)
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
