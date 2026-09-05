//! Loading a weapon's own body model - the Rocket's, the Mine's or the
//! Bomb's - off a real disc, on the terms every optional asset here follows:
//! absence is reported, not fatal.
//!
//! Split out of `load.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; `load.rs` was already at the ceiling with
//! only the Rocket's own inline block, and a second and third weapon needed
//! the identical shape. A move, with no behaviour change to the Rocket's own
//! reporting.

use super::*;

/// Loads one weapon's own model by entry name.
///
/// Not per-team and not per-track - one entry serves every projectile of that
/// kind in the game, which is why this loads once and [`super::super::Scene`]
/// clones it per projectile slot. `fallback` names what a caller falls back
/// to when this returns `None`, for the report line alone - a billboard for
/// every kind wired so far.
pub(super) fn load(
    archives: &mut oag_assets::Archives,
    entry: &str,
    fallback: &str,
    lod: mesh::Lod,
    report: &mut Vec<String>,
) -> Option<Model> {
    match archives.read_name(entry) {
        Ok(blob) => match mesh::build_with_textures(entry, &blob, None, lod) {
            Ok(model) => {
                report.push(format!(
                    "{entry}: {} triangle(s), radius {:.2} - {fallback} is this model, \
                     not a billboard",
                    model.indices.len() / 3,
                    model.radius
                ));
                Some(model)
            }
            Err(error) => {
                report.push(format!("{entry}: did not decode ({error})"));
                None
            }
        },
        Err(_) => {
            report.push(format!(
                "{entry}: absent - {fallback} falls back to a billboard"
            ));
            None
        }
    }
}
