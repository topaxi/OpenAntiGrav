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

/// Loads one of the Cannon round's two hand-built quads' textures.
///
/// Same absence-is-reported terms as [`load`]: a real disc resolves both
/// (`Data.wad` entries 1057/1058 on both PSP pressings - see
/// [`super::super::CANNON_BOLT_TEXTURE_ENTRY`]), so `None` here means a
/// decode error or a missing archive set, not an unauthored asset. The
/// caller falls back to [`FlareTexture::placeholder`] on `None`, the same
/// substitute-for-the-missing-asset-alone terms the exhaust flare's own
/// `flare`/`noise` textures already use in `Scene::new` - never an invented
/// picture where the real one exists but failed to load.
fn load_flare_texture(
    archives: &mut oag_assets::Archives,
    entry: &str,
    what: &str,
    report: &mut Vec<String>,
) -> Option<FlareTexture> {
    let blob = match archives.read_name(entry) {
        Ok(blob) => blob,
        Err(_) => {
            report.push(format!("{entry}: absent - {what} draws nothing"));
            return None;
        }
    };
    match oag_texture::texture::Texture::parse(&blob) {
        Ok(texture) => {
            report.push(format!(
                "{entry}: {}x{} .mip - {what}'s own texture",
                texture.width, texture.height
            ));
            Some(FlareTexture {
                width: u32::from(texture.width),
                height: u32::from(texture.height),
                rgba: texture.to_rgba(),
            })
        }
        Err(error) => {
            report.push(format!(
                "{entry}: did not decode ({error}) - {what} draws nothing"
            ));
            None
        }
    }
}

/// Both of the Cannon round's hand-built quads' textures, in one call and
/// one tuple - [`super::Loaded::cannon_quad_textures`] takes the pair
/// unnamed rather than as two fields for the reason this returns them the
/// same way: a second field, or a second `let` in `load.rs` for them alone,
/// would cost exactly the line the tuple exists to not spend, in a file
/// already at its ceiling.
pub(super) fn load_cannon_quad_textures(
    archives: &mut oag_assets::Archives,
    report: &mut Vec<String>,
) -> (Option<FlareTexture>, Option<FlareTexture>) {
    (
        load_flare_texture(
            archives,
            super::super::CANNON_BOLT_TEXTURE_ENTRY,
            "the cannon bolt streak",
            report,
        ),
        load_flare_texture(
            archives,
            super::super::CANNON_MUZZLE_FLASH_TEXTURE_ENTRY,
            "the cannon muzzle flash",
            report,
        ),
    )
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
