//! A `.vex` whose geometry is a PS4/Vita `.rcsmodel`: Omega's scenes.
//!
//! The one place a scene's motion (`.vex`) and its geometry (the `.rcsmodel`
//! beside it, 2048's container) are joined, for the menu backdrop
//! ([`crate::boot`]) and the preview path ([`super::model_named`]) alike.

use anyhow::Result;
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
    if !mesh::rcs::psp2::is_psp2(geometry) {
        return Ok(None);
    }
    let (model, built) = mesh::rcs::psp2::build_with_vex(entry, geometry, vex, &mut |path| {
        archives.read_name(path).ok()
    })?;
    Ok(Some((model, built.describe())))
}
