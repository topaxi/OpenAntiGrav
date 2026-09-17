//! Loading a weapon's own body model - the Rocket's, the Plasma bolt's, the
//! Mine's, the Bomb's or the Cannon's - off a real disc, on the terms every
//! optional asset here follows: absence is reported, not fatal.
//!
//! Split out of `load.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; `load.rs` was already at the ceiling with
//! only the Rocket's own inline block, and a second and third weapon needed
//! the identical shape. A move, with no behaviour change to the Rocket's own
//! reporting.
//!
//! **Per-title since 2026-09-17.** Every entry name comes from the caller's
//! own `oag_title::weapons::WeaponModels`, not from a Pulse-spelled constant
//! in this crate - `oag_game::race::ROCKET_MODEL_ENTRY` and friends were
//! reached for on every source, so a Wipeout HD race asked for
//! `Data\Weapons\Rocket.vex`, which resolves on no PS3 archive, and every
//! projectile fell back to a billboard even though HD authors its own models
//! under `Data\Weapons\hd_*`. See
//! `handover/rendering/hd-furys-weapons-draw-pulses-fallbacks-not-their-own-models.md`.
//!
//! **Both geometry layouts, on one path.** A PSP `.vex` carries its own
//! geometry; a PS3 `.vex` is a header whose vertices live in the
//! `.rcsmodel` beside it, the same branch `crate::livery::shield_model`
//! already takes for the shield - [`load`] takes it too, rather than each
//! caller re-deciding which loader a weapon needs.

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
    let blob = match archives.read_name(entry) {
        Ok(blob) => blob,
        Err(_) => {
            report.push(format!(
                "{entry}: absent - {fallback} falls back to a billboard"
            ));
            return None;
        }
    };
    // **A PS3 entry's geometry is in the `.rcsmodel` beside it** - see
    // `oag_render::mesh::rcs` for what is decoded and what is not, and this
    // module's own doc comment for why the branch lives here rather than
    // being duplicated per caller.
    if mesh::geometry_is_external(&blob) {
        let sibling = mesh::rcs::sibling_name(entry);
        let Some(geometry) = sibling.as_deref().and_then(|s| archives.read_name(s).ok()) else {
            report.push(format!(
                "{entry}: a PS3 .vex with no .rcsmodel beside it - {fallback} \
                 falls back to a billboard"
            ));
            return None;
        };
        return match mesh::rcs::build(
            entry,
            &blob,
            &geometry,
            &mut |path| archives.read_name(path).ok(),
            |c| c.mesh,
        ) {
            Ok((model, built)) => {
                report.push(format!(
                    "{entry}: {} - {fallback} is this model, not a billboard",
                    built.describe()
                ));
                Some(model)
            }
            Err(error) => {
                report.push(format!("{entry}: its .rcsmodel does not build ({error})"));
                None
            }
        };
    }
    match mesh::build_with_textures(entry, &blob, None, lod) {
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
    }
}

/// [`load`], but the entry itself is `None` on this title's own table -
/// a title fact, not a load failure, so this says nothing and returns
/// `None` rather than a report line for a field that was never going to
/// resolve.
fn load_optional(
    archives: &mut oag_assets::Archives,
    entry: Option<&str>,
    fallback: &str,
    lod: mesh::Lod,
    report: &mut Vec<String>,
) -> Option<Model> {
    load(archives, entry?, fallback, lod, report)
}

/// Every weapon's own body model, on [`load`]'s own terms - the Rocket's,
/// the Plasma bolt's, the Mine's, the Bomb's, the Cannon round's and the
/// Plasma blast's three.
pub(super) struct WeaponBodies {
    pub(super) rocket: Option<Model>,
    /// The Plasma bolt's own head - `HD_plasma_ball`, HD only. See
    /// `oag_title::weapons::WeaponModels::plasma_ball`.
    pub(super) plasma_ball: Option<Model>,
    pub(super) mine: Option<Model>,
    pub(super) bomb: Option<Model>,
    pub(super) cannon: Option<Model>,
    pub(super) plasma_blast: blast_models::PlasmaBlastModels,
}

/// Every weapon's own body model in one call, each on [`load`]'s own terms,
/// sourced from `models` rather than from a constant fixed to one title.
///
/// One function rather than one `let` per kind in `load.rs`, which is what
/// buys back the line budget that file has had none of to spare since the
/// Cannon's own row landed - see this module's doc comment for the seam
/// this continues.
pub(super) fn load_bodies(
    archives: &mut oag_assets::Archives,
    models: &oag_title::weapons::WeaponModels,
    lod: mesh::Lod,
    report: &mut Vec<String>,
) -> WeaponBodies {
    let mut one = |entry, fallback| load_optional(archives, entry, fallback, lod, report);
    // **Pulse's halo/hemisphere2/hemisphere1 trio and HD's ring/sphere/halo
    // trio share one container by load-order position, not by name.**
    // `blast_models::PlasmaBlastModels`'s own field names stay Pulse's -
    // `halo`, `hemisphere2`, `hemisphere1` - rather than being renamed for
    // this second title, both because the ease each field drives is decided
    // at draw time from `Options::hd_plasma_blast`, never from which model
    // is in which field, and because `load/weapon_models.rs` is shared with
    // a concurrent pass on the Cannon's own textures - renaming this struct
    // literal's fields would turn an append-append merge into a semantic
    // one for no reading gained. See `oag_title::weapons::HdPlasmaBlast`'s
    // own doc comment for the ordinal mapping this reuses.
    let plasma_blast = if let Some(hd) = models.plasma_blast_hd {
        blast_models::PlasmaBlastModels {
            halo: one(Some(hd.ring), "a plasma blast ring"),
            hemisphere2: one(Some(hd.sphere), "a plasma blast sphere"),
            hemisphere1: one(Some(hd.halo), "a plasma blast halo"),
        }
    } else if let Some(pulse) = models.plasma_blast_pulse {
        blast_models::PlasmaBlastModels {
            halo: one(Some(pulse.halo), "a plasma blast halo"),
            hemisphere2: one(Some(pulse.hemisphere2), "a plasma blast hemisphere"),
            hemisphere1: one(Some(pulse.hemisphere1), "a plasma blast hemisphere"),
        }
    } else {
        blast_models::PlasmaBlastModels::default()
    };
    WeaponBodies {
        rocket: one(models.rocket, "a rocket"),
        plasma_ball: one(models.plasma_ball, "a plasma bolt"),
        mine: one(models.mine, "a laid mine"),
        bomb: one(models.bomb, "a laid bomb"),
        cannon: one(models.cannon, "a cannon round"),
        plasma_blast,
    }
}
