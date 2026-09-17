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
//! `docs/ghidra/functions/ps3-hdfury-eu/plasma.md`.
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
/// the Mine's, the Bomb's, the Cannon round's and the Plasma blast's three -
/// the bolt's own head included, riding inside the last element rather than
/// a sixth of its own; see [`blast_models::PlasmaBlastModels`]'s own doc
/// comment for why.
pub(super) type WeaponBodies = (
    Option<Model>,
    Option<Model>,
    Option<Model>,
    Option<Model>,
    blast_models::PlasmaBlastModels,
);

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
            ball: None, // overwritten below
        }
    } else if let Some(pulse) = models.plasma_blast_pulse {
        blast_models::PlasmaBlastModels {
            halo: one(Some(pulse.halo), "a plasma blast halo"),
            hemisphere2: one(Some(pulse.hemisphere2), "a plasma blast hemisphere"),
            hemisphere1: one(Some(pulse.hemisphere1), "a plasma blast hemisphere"),
            ball: None, // overwritten below
        }
    } else {
        blast_models::PlasmaBlastModels::default()
    };
    // The bolt's own head, HD only - see `PlasmaBlastModels::ball`'s own doc
    // comment for why it rides in this container.
    let ball = one(models.plasma_ball, "a plasma bolt");
    let bodies = (
        one(models.rocket, "a rocket"),
        one(models.mine, "a laid mine"),
        one(models.bomb, "a laid bomb"),
        one(models.cannon, "a cannon round"),
        blast_models::PlasmaBlastModels {
            ball,
            ..plasma_blast
        },
    );
    // See `blast_models::HD_BLAST_MODELS_DRAWN`'s own doc comment: the three
    // models above load and their ease still runs, but the draw itself is
    // gated off until the scale composition is understood - a report line
    // rather than a silent no-op, so the load report says why an HD Plasma
    // detonation looks the way it does. After `one`'s last use, since that
    // closure holds its own mutable borrow of `report`.
    if models.plasma_blast_hd.is_some() && !blast_models::HD_BLAST_MODELS_DRAWN {
        report.push(
            "HD plasma explosion models loaded, not drawn: the recovered \
             scale ease produces a screen-filling sphere - see plasma.md's \
             2026-09-17 \"the picture is oversized\" section"
                .to_string(),
        );
    }
    if models.plasma_ball.is_some() && !blast_models::HD_PLASMA_BALL_DRAWN {
        report.push(
            "HD_plasma_ball loaded, not drawn: it resolves through this \
             engine's one shared lit shader path with no additive/glow \
             route, and renders as an opaque black shard - see \
             docs/formats/rcsmaterial.md"
                .to_string(),
        );
    }
    bodies
}
