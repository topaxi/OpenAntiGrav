//! Reassembling a PS3 circuit's pad plates into `--mesh`'s merged picture.
//!
//! Split out of `main.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change.

use oag_mesh::mesh;

/// Draws a PS3 circuit's `Speedup Pad`/`Weapon Pad` geometry back into
/// `model`, which `mesh::rcs::scene_from` built without them.
///
/// `build_scene`'s own world-space pass excludes both pad classes' chunks
/// (see its own doc comment) so a caller building a separate, tintable pad
/// `Drawable` - `oag_raceplay` - draws each chunk once rather than twice.
/// This viewer wants one merged picture instead, so it draws the pads back
/// in here rather than showing a circuit with no pad plates.
///
/// A no-op, returning `model` unchanged, for a source with no `.rcsmodel`
/// sibling - which [`mesh::rcs::scene_from`] having returned `Some` already
/// rules out in practice, but this stays honest rather than assuming it.
pub fn with_pads(archive: &str, name: &str, data: &[u8], model: mesh::Model) -> mesh::Model {
    let Some(geometry) = mesh::rcs::sibling_geometry(archive, name, data) else {
        return model;
    };
    let mut parts = vec![model];
    for build in [mesh::rcs::build_pads, mesh::rcs::build_weapon_pads] {
        if let Ok((pads, report)) = build(name, data, &geometry, &mut |path| {
            mesh::read_blob(archive, path).ok()
        }) {
            println!("{}", report.describe());
            parts.push(pads);
        }
    }
    mesh::merge(name, parts)
}
