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

/// Every weapon's own body model in one call - the Rocket's, the Mine's,
/// the Bomb's, the Cannon round's and the Plasma blast's three - each on
/// [`load`]'s own terms.
///
/// One function rather than one `let` per kind in `load.rs`, which is what
/// buys back the line budget that file has had none of to spare since the
/// Cannon's own row landed - see this module's doc comment for the seam
/// this continues.
pub(super) fn load_bodies(
    archives: &mut oag_assets::Archives,
    lod: mesh::Lod,
    report: &mut Vec<String>,
) -> (
    Option<Model>,
    Option<Model>,
    Option<Model>,
    Option<Model>,
    blast_models::PlasmaBlastModels,
) {
    let mut one = |entry, fallback| load(archives, entry, fallback, lod, report);
    (
        one(ROCKET_MODEL_ENTRY, "a rocket"),
        one(MINE_MODEL_ENTRY, "a laid mine"),
        one(BOMB_MODEL_ENTRY, "a laid bomb"),
        one(CANNON_MODEL_ENTRY, "a cannon round"),
        blast_models::PlasmaBlastModels {
            halo: one(PLASMA_BLAST_HALO_MODEL_ENTRY, "a plasma blast halo"),
            hemisphere2: one(
                PLASMA_BLAST_HEMISPHERE2_MODEL_ENTRY,
                "a plasma blast hemisphere",
            ),
            hemisphere1: one(
                PLASMA_BLAST_HEMISPHERE1_MODEL_ENTRY,
                "a plasma blast hemisphere",
            ),
        },
    )
}
