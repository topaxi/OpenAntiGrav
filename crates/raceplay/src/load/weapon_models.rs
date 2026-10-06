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
//! in this crate - `oag_raceplay::ROCKET_MODEL_ENTRY` and friends were
//! reached for on every source, so a Wipeout HD race asked for
//! `Data\Weapons\Rocket.vex`, which resolves on no PS3 archive, and every
//! projectile fell back to a billboard even though HD authors its own models
//! under `Data\Weapons\hd_*`. See
//! `docs/ghidra/functions/ps3-hdfury-eu/plasma.md`.
//!
//! **Both geometry layouts, on one path.** A PSP `.vex` carries its own
//! geometry; a PS3 `.vex` is a header whose vertices live in the
//! `.rcsmodel` beside it, the same branch `oag_livery::shield::shield_model`
//! already takes for the shield - [`load`] takes it too, rather than each
//! caller re-deciding which loader a weapon needs.

use super::*;

/// Loads one weapon's own model by entry name.
///
/// Not per-team and not per-track - one entry serves every projectile of that
/// kind in the game, which is why this loads once and [`super::super::Scene`]
/// clones it per projectile slot. `fallback` names what a caller falls back
/// to when this returns `None`, for the report line alone - a billboard for
/// every kind wired so far. `cull` asks for [`cull_as_authored`] on a PS3
/// model, which only a caller whose own placement matrix is a rotation may
/// ask for - see that function.
pub(super) fn load(
    archives: &mut oag_assets::Archives,
    entry: &str,
    fallback: &str,
    cull: bool,
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
    // `oag_mesh::mesh::rcs` for what is decoded and what is not, and this
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
            Ok((mut model, built)) => {
                let culled = cull && cull_as_authored(&mut model, &geometry);
                report.push(format!(
                    "{entry}: {}{} - {fallback} is this model, not a billboard",
                    built.describe(),
                    if culled { ", back faces culled" } else { "" }
                ));
                Some(model)
            }
            Err(error) => {
                report.push(format!("{entry}: its .rcsmodel does not build ({error})"));
                None
            }
        };
    }
    match mesh::build_with_textures(entry, &blob, None) {
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

/// Marks every draw of a PS3 weapon model back-face culled when **every**
/// material its `.rcsmodel` carries sets state bit 4, and says whether it
/// did.
///
/// **Bit 4 is `NV4097_SET_CULL_FACE_ENABLE`, read, not guessed.**
/// `Material_ApplyRenderState` (`0x005d8f68`) writes `state >> 4 & 1` to RSX
/// method `0x183c`, which RPCS3's own `gcm_enums.h` names
/// `NV4097_SET_CULL_FACE_ENABLE`; the cull face itself is left at `GL_BACK`
/// outside the ship-shadow pass. See
/// `docs/ghidra/functions/ps3-hdfury-eu/material-state.md`. `mesh::rcs`
/// does not carry the bit per draw yet, so this applies it per model, and
/// only where the answer is the same for every material; a mixed model keeps
/// drawing both faces rather than being half-applied. Measured on HD's own
/// weapon models: the Plasma's ring, sphere, halo and ball, the Rocket and
/// the Bomb qualify; the Mine and the Cannon round do not.
///
/// **Every HD weapon model asks for it since 2026-09-24.** Culling is only
/// right under a placement matrix that is a rotation - a reflection reverses
/// every triangle's winding on screen, and the cull then hides exactly the
/// faces the original shows. Until that date
/// `Race::projectile_model_matrices`, which places the Rocket, the Cannon
/// round and the bolt's head, built a reflection, so only the Plasma
/// explosion (placed by `blast_models::facing_away`, a rotation) could ask.
/// Both are rotations now. The Mine and the Cannon round still draw both
/// faces: their materials disagree on the bit, and this function refuses a
/// mixed model rather than half-applying it.
///
/// This matters for the Plasma's own explosion in particular: its sphere
/// and ring are each authored as **two** shells, one wound outward and one
/// inward, and the halo as one disc facing `-Z` - which only reads as the
/// original's picture once the back faces go. See `plasma.md`'s 2026-09-23
/// section.
fn cull_as_authored(model: &mut Model, geometry: &[u8]) -> bool {
    const CULL_FACE_ENABLE: u32 = 1 << 4;
    let Ok(parsed) = oag_rcs::rcsmodel::Model::parse(geometry) else {
        return false;
    };
    let culled = !parsed.materials.is_empty()
        && parsed
            .materials
            .iter()
            .all(|m| m.state & CULL_FACE_ENABLE != 0);
    if culled {
        for draw in model
            .draws
            .iter_mut()
            .chain(model.alpha_tested_draws.iter_mut())
            .chain(model.transparent_draws.iter_mut())
        {
            draw.culled = true;
        }
    }
    culled
}

/// [`load`], but the entry itself is `None` on this title's own table -
/// a title fact, not a load failure, so this says nothing and returns
/// `None` rather than a report line for a field that was never going to
/// resolve.
fn load_optional(
    archives: &mut oag_assets::Archives,
    entry: Option<&str>,
    fallback: &str,
    cull: bool,
    report: &mut Vec<String>,
) -> Option<Model> {
    load(archives, entry?, fallback, cull, report)
}

/// Every weapon's own body model, on [`load`]'s own terms - the Rocket's,
/// the Mine's, the Bomb's, the Cannon round's, the Plasma blast's three - the
/// bolt's own head included, riding inside the last element rather than a
/// sixth of its own; see [`blast_models::PlasmaBlastModels`]'s own doc
/// comment for why - the Bomb blast's own two, and the LeachBeam ball's own,
/// last because it rides no pool at all - see
/// `oag_title::weapons::WeaponModels::leachbeam_ball`.
pub(super) type WeaponBodies = (
    Option<Model>,
    Option<Model>,
    Option<Model>,
    Option<Model>,
    blast_models::PlasmaBlastModels,
    bomb_blast::BombBlastModels,
    Option<Model>,
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
    if let Some(own) = super::super::assets::platform_sibling(archives, entry) {
        return match super::super::assets::decode_texture(archives, &own) {
            Ok(texture) => {
                report.push(format!(
                    "{own}: {}x{} - {what}'s own texture",
                    texture.width, texture.height
                ));
                Some(texture)
            }
            Err(why) => {
                report.push(format!("{why} - {what} draws nothing"));
                None
            }
        };
    }
    match super::super::assets::exhaust_texture(archives, entry) {
        Ok((texture, note)) => {
            report.push(format!("{note} - {what}'s own texture"));
            Some(texture)
        }
        Err(why) => {
            report.push(format!("{why} - {what} draws nothing"));
            None
        }
    }
}

/// Both of the Cannon round's hand-built quads' textures and the title's own
/// reading of how they are drawn, in one call and one tuple -
/// [`super::Loaded::cannon_quad_textures`] takes the three unnamed rather
/// than as fields for the reason this returns them the same way: another
/// field, or another `let` in `load.rs` for them alone, would cost exactly
/// the line the tuple exists to not spend, in a file already at its ceiling.
///
/// A title with its own [`oag_title::weapons::CannonLook`] names its own
/// entries - Wipeout HD's are `.gtf`, off `CannonBullet`'s constructor -
/// and every other title reads Pulse's `.mip` pair, as it did before the
/// look existed.
pub(super) fn cannon_quads(
    archives: &mut oag_assets::Archives,
    models: &oag_title::weapons::WeaponModels,
    report: &mut Vec<String>,
) -> crate::CannonAssets {
    let look = models.cannon_look;
    let Some(own) = look else {
        let bolt = super::super::CANNON_BOLT_TEXTURE_ENTRY;
        let flash = super::super::CANNON_MUZZLE_FLASH_TEXTURE_ENTRY;
        return (
            load_flare_texture(archives, bolt, "the cannon bolt streak", report),
            load_flare_texture(archives, flash, "the cannon muzzle flash", report),
            None,
        );
    };
    let mut one = |entry: &str, what: &str| match super::super::assets::decode_gtf(archives, entry)
    {
        Ok(texture) => {
            report.push(format!(
                "{entry}: {}x{} .gtf - {what}'s own texture",
                texture.width, texture.height
            ));
            Some(texture)
        }
        Err(why) => {
            report.push(format!("{why} - {what} draws a placeholder"));
            None
        }
    };
    (
        one(own.bolt_texture, "the cannon bolt streak"),
        one(own.flash_texture, "the cannon muzzle flash"),
        look,
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
    report: &mut Vec<String>,
) -> WeaponBodies {
    // `cull` is `true` for every model a rotation places - every HD body and
    // the HD explosion trio - and only takes effect on a PS3 model whose
    // materials all ask for it; see `cull_as_authored`.
    let mut one = |entry, fallback, cull| load_optional(archives, entry, fallback, cull, report);
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
            halo: one(Some(hd.ring), "a plasma blast ring", true),
            hemisphere2: one(Some(hd.sphere), "a plasma blast sphere", true),
            hemisphere1: one(Some(hd.halo), "a plasma blast halo", true),
            ball: None, // overwritten below
            shuriken: None,
        }
    } else if let Some(pulse) = models.plasma_blast_pulse {
        blast_models::PlasmaBlastModels {
            halo: one(Some(pulse.halo), "a plasma blast halo", false),
            hemisphere2: one(Some(pulse.hemisphere2), "a plasma blast hemisphere", false),
            hemisphere1: one(Some(pulse.hemisphere1), "a plasma blast hemisphere", false),
            ball: None, // overwritten below
            shuriken: None,
        }
    } else {
        blast_models::PlasmaBlastModels::default()
    };
    // The bolt's own head, HD only - see `PlasmaBlastModels::ball`'s own doc
    // comment for why it rides in this container.
    let ball = one(models.plasma_ball, "a plasma bolt", true);
    // Pulse-only, `None` on every other title's own table - see
    // `WeaponModels::bomb_blast_pulse`'s own doc comment.
    let mut bomb_blast =
        models
            .bomb_blast_pulse
            .map_or(bomb_blast::BombBlastModels::default(), |pulse| {
                bomb_blast::BombBlastModels {
                    hemisphere: one(Some(pulse.hemisphere), "a bomb blast hemisphere", false),
                    shockwave: one(Some(pulse.shockwave), "a bomb blast shockwave", false),
                    repulser_field: None,
                    mag_floor: [None, None],
                }
            });
    // Rides the Bomb blast's container - see `BombBlastModels::repulser_field`.
    bomb_blast.repulser_field = one(models.repulser_field, "a repulser field", false);
    // And the magstrip effect's pair - see `BombBlastModels::mag_floor`.
    if let Some([first, second]) = models.mag_floor {
        bomb_blast.mag_floor = [
            one(Some(first), "a magstrip effect", false),
            one(Some(second), "a magstrip effect", false),
        ];
    }
    // The LeachBall rides no pool of its own - `Scene::new` builds one
    // drawable, not `MAX_PROJECTILES` of them, since only one beam is ever
    // live - but its model loads on the same terms as every other weapon
    // body above. `cull` is asked for on the same grounds the Rocket's is,
    // and `cull_as_authored` declines it anyway: the ball's material culls
    // (state bit 4) and the bloomring's does not, and a mixed model draws
    // both faces. **Drawn since 2026-09-25**, when `mesh::rcs::rim_glow`
    // gave its `hd_leachbeam_ball_glow` program the shading it computes; the
    // bloomring's texture-only program already drew through
    // `slots::EMISSIVE`. See docs/rendering/hd-unlit-programs.md.
    let leach_ball = one(models.leachbeam_ball, "a leachbeam ball", true);
    (
        one(models.rocket, "a rocket", true),
        one(models.mine, "a laid mine", true),
        one(models.bomb, "a laid bomb", true),
        // HD's Cannon model flashes at the muzzle rather than riding the round.
        one(
            models.cannon,
            match models.cannon_look {
                Some(_) => "a cannon muzzle flash",
                None => "a cannon round",
            },
            true,
        ),
        blast_models::PlasmaBlastModels {
            ball,
            shuriken: one(models.shuriken, "a shuriken blade", true),
            ..plasma_blast
        },
        bomb_blast,
        leach_ball,
    )
}

/// The ghost ship's static texture, `oag_render::ghost::STATIC_TEXTURE_ENTRY`.
///
/// Read on every title: Pulse's `Data.wad` carries it (entry 945 on the USA
/// pressing, see `docs/ghidra/functions/psp-pulse-usa/ghost.md`), and a source
/// without it is reported and drawn without the ghost's third pass.
pub(super) fn load_ghost_static(
    archives: &mut oag_assets::Archives,
    report: &mut Vec<String>,
) -> Option<FlareTexture> {
    load_flare_texture(
        archives,
        oag_render::ghost::STATIC_TEXTURE_ENTRY,
        "the ghost ship's static",
        report,
    )
}
