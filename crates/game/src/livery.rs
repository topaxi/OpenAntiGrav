//! One hull, plume and nozzle per grid slot, off the player's own disc.
//!
//! Until this module existed every craft on the grid wore the player's hull:
//! `race::load` read one `Ship.vex` and `Scene::new` cloned it eight times, so
//! a race was eight identical ships in one livery and one nozzle position for
//! the whole field.
//!
//! **The hulls are genuinely different models, not one model repainted.**
//! Measured across the eight teams the PSP disc's own plugin definition
//! declares: 845 to 1,497 triangles, 1,273 to 1,650 vertices, radius 6.45 to
//! 7.09. That is worth stating because the *Zone* hull is the other way round -
//! every team's `Zone.vex` decodes to the same 1213 vertices / 1149 triangles /
//! 8 meshes, and only its paint varies (see `race::ship_entry_name`). So a
//! livery here is a model load, and a Zone livery would be a texture swap.
//!
//! # What is recovered and what is ours
//!
//! **The hull paths are the disc's**: `ship_entry_name` builds
//! `Data\Ships\<Team>\Ship.vex` from the team id the definition declares, and
//! `PI_TeamModel`'s `location` is a *file stem* (the literal `"ship"`), not a
//! team name - which is the naming trap `HANDOVER.md` records. Nothing here
//! invents a path.
//!
//! **Which team flies which slot is this project's**, and is labelled so in the
//! load report. `Race_SpawnGrid` passes each racer an `id` that comes from a
//! racer list built upstream, and that list has not been read - so the original
//! may draw teams by championship entry, by player choice, or at random, and
//! nothing here should be taken as reproducing it. [`teams_for_slots`] fills
//! the grid from the catalogue in file order, which is deterministic, needs no
//! generator, and is honest about being a stand-in. Same footing as the pickup
//! grant (`docs/gameplay/pickups.md`).
//!
//! # Short lists and long ones
//!
//! Both happen, and neither is an error:
//!
//! - **Longer than the grid.** The four Pulse DLC packs each add a team, so a
//!   player with all of them has twelve for eight slots. The extra teams simply
//!   do not race, and *which* seven of eleven do is the unrecovered selection
//!   above rather than a rule this module knows.
//! - **Shorter than the grid.** Wipeout Pure declares its own, a PS2 set may
//!   not carry every hull under the PSP's name, and one team's `Ship.vex` does
//!   not resolve at all on either disc. The list is then cycled, so the field
//!   is as varied as the data allows, and the report says a livery repeated.
//!
//! Every failure is reported and none is silent: a hull that will not load
//! falls back to the player's, and the report names the slot and the reason.
//! Eight identical ships is exactly what a working feature looks like from the
//! outside, which is why the absence has to be legible in the log.

use anyhow::{Context, Result};
use oag_core::math::Vec3;
use oag_formats::vex;
use oag_race::Mode;
use oag_render::mesh::{self, Model};

use crate::race::{boost_entry_name, ps2_texture_set, ship_entry_name};

/// Everything a single grid slot draws that belongs to its team.
#[derive(Debug)]
pub struct Livery {
    /// The team id this slot flies, e.g. `Feisar`. What the report names, and
    /// **not** a display name - see `catalogue`'s "an id is not a name".
    pub team: String,
    /// The hull: `Data\Ships\<Team>\Ship.vex`, or `Zone.vex` in Zone mode.
    pub hull: Model,
    /// The `Engine Flare` locator in this hull's own model space, which is
    /// where this craft's exhaust ribbon and flare are anchored.
    ///
    /// Per team because the hulls differ: mounting eight ribbons on one team's
    /// nozzle put the flare inside the fuselage on some of them.
    pub nozzle: Option<Vec3>,
    /// The additive plume a speed pad reveals, `shipboost.vex` beside the hull.
    pub boost: Option<Model>,
    /// The `Ship Collision Fx` locators in this hull's own model space, the
    /// spark anchors the original picks the nearest of.
    ///
    /// Per team for the same reason [`Self::nozzle`] is: the hulls are
    /// different models, so a locator set belongs to the one it was authored
    /// on.
    pub collision_fx: Vec<Vec3>,
    /// The plume's own authored texture-transform animation.
    ///
    /// **Per team, and this is not a formality.** Eight per-team plumes sampled
    /// through one team's keyframes is the kind of wrong that renders
    /// plausibly. Every team read so far carries the identical track, which
    /// makes it *currently* invisible and is not a reason to share one.
    pub boost_uv: Option<vex::TexTransform>,
}

/// Which team flies each grid slot, the player first.
///
/// **This ordering is this project's, not the original's** - see the module
/// docs. Deterministic on purpose: no generator, no map iteration, and the same
/// grid on every replay of the same race.
///
/// `available` is the catalogue's own order, which is the definition file's
/// order, and the player's team is skipped in it rather than moved - a player
/// racing for the last team does not reshuffle everyone else. An empty list
/// gives every slot the player's team, which is what this engine did before
/// liveries and is still what a source with no readable definition gets.
#[must_use]
pub fn teams_for_slots(player: &str, available: &[String], slots: usize) -> Vec<String> {
    let mut out = Vec::with_capacity(slots);
    out.push(player.to_string());
    let others: Vec<&String> = available.iter().filter(|team| *team != player).collect();
    if others.is_empty() {
        out.resize(slots, player.to_string());
        return out;
    }
    // Cycles when the list is shorter than the grid, which is the Pure and PS2
    // case rather than the Pulse one - see the module docs.
    for slot in 1..slots {
        out.push(others[(slot - 1) % others.len()].clone());
    }
    out
}

/// Loads one [`Livery`] per entry of `teams`, in that order.
///
/// Each distinct team is read once however many slots fly it, because a
/// twelve-team field on an eight-slot grid is the *long* case and the short one
/// repeats - see the module docs.
///
/// # Errors
///
/// Only if the **first** team's hull cannot be read or built. That one is the
/// player's, and a race with no hull for the player is not a race; every other
/// failure falls back to slot 0's hull and says so in `report`.
pub fn load(
    archives: &mut oag_assets::Archives,
    teams: &[String],
    mode: Mode,
    lod: mesh::Lod,
    report: &mut Vec<String>,
) -> Result<Vec<Livery>> {
    let mut out: Vec<Livery> = Vec::with_capacity(teams.len());
    for (slot, team) in teams.iter().enumerate() {
        // Already loaded for an earlier slot: clone rather than re-read, which
        // is what makes a repeated livery cost nothing.
        if let Some(first) = out.iter().position(|other| &other.team == team) {
            let source = &out[first];
            let livery = Livery {
                team: team.clone(),
                hull: source.hull.clone(),
                nozzle: source.nozzle,
                collision_fx: source.collision_fx.clone(),
                boost: source.boost.clone(),
                boost_uv: source.boost_uv.clone(),
            };
            report.push(format!(
                "slot {slot}: {team}, the same livery as slot {first} - the source declares \
                 fewer teams than the grid has slots"
            ));
            out.push(livery);
            continue;
        }

        match one(archives, team, mode, lod, report) {
            Ok(livery) => out.push(livery),
            Err(error) if slot == 0 => return Err(error),
            Err(error) => {
                // Never silent: eight identical ships is what a working feature
                // looks like from the outside.
                report.push(format!(
                    "slot {slot}: {team} has no usable hull ({error}) - it wears slot 0's instead"
                ));
                let player = &out[0];
                out.push(Livery {
                    team: player.team.clone(),
                    hull: player.hull.clone(),
                    nozzle: player.nozzle,
                    collision_fx: player.collision_fx.clone(),
                    boost: player.boost.clone(),
                    boost_uv: player.boost_uv.clone(),
                });
            }
        }
    }
    Ok(out)
}

/// One team's hull, nozzle and plume.
fn one(
    archives: &mut oag_assets::Archives,
    team: &str,
    mode: Mode,
    lod: mesh::Lod,
    report: &mut Vec<String>,
) -> Result<Livery> {
    let hull_name = ship_entry_name(team, mode);
    let blob = archives
        .read_name(&hull_name)
        .with_context(|| format!("reading {hull_name}"))?;
    // **A PS3 hull's geometry is in the `.rcsmodel` beside it**, so it is read
    // from the pair. Untextured and lit off computed face normals; see
    // `oag_render::mesh::rcs` for what is decoded and what is not. A source
    // with no sibling draws nothing and says so rather than substituting
    // anything.
    if mesh::geometry_is_external(&blob) {
        let sibling = mesh::rcs::sibling_name(&hull_name);
        let geometry = sibling.as_deref().and_then(|s| archives.read_name(s).ok());
        let Some(geometry) = geometry else {
            report.push(format!(
                "{hull_name}: a PS3 .vex with no .rcsmodel beside it - nothing draws \
                 for this craft, and the race still runs"
            ));
            return Ok(Livery {
                team: team.to_string(),
                hull: mesh::Model::none(&hull_name),
                nozzle: None,
                collision_fx: Vec::new(),
                boost: None,
                boost_uv: None,
            });
        };
        let (hull, built) = mesh::rcs::build(&hull_name, &blob, &geometry, |c| c.mesh)?;
        report.push(format!("{hull_name}: {}", built.describe()));
        return Ok(Livery {
            team: team.to_string(),
            nozzle: engine_flare(&blob),
            collision_fx: collision_fx_locators(&blob),
            hull,
            boost: None,
            boost_uv: None,
        });
    }
    let mut hull = mesh::build_with_textures(&hull_name, &blob, None, lod)?;
    // The PS2 signature: `Texture` nodes exist (the model wants textures) but
    // every one is missing (its embedded block was empty). Only then is the
    // directory-position heuristic worth trying - see `ps2_texture_set`.
    if !hull.textures.is_empty()
        && hull.textures.iter().all(Option::is_none)
        && let Some(external) = ps2_texture_set(archives, &hull_name)
    {
        hull = mesh::build_with_textures(&hull_name, &blob, Some(external), lod)?;
    }
    report.push(format!(
        "{hull_name}: {} triangle(s), model centre {:?}, radius {:.2}",
        hull.indices.len() / 3,
        hull.centre,
        hull.radius
    ));

    let nozzle = engine_flare(&blob);
    match nozzle {
        Some(at) => report.push(format!(
            "{hull_name}: engine_flare locator at {at:?} in model space"
        )),
        None => report.push(format!(
            "{hull_name}: no Engine Flare node - this craft's exhaust will not be drawn"
        )),
    }

    let collision_fx = collision_fx_locators(&blob);
    report.push(if collision_fx.is_empty() {
        format!("{hull_name}: no Ship Collision Fx nodes - sparks anchor at the contact point")
    } else {
        format!(
            "{hull_name}: {} Ship Collision Fx locator(s) for the spark anchor",
            collision_fx.len()
        )
    });

    let (boost, boost_uv) = plume(archives, team, mode, lod, report);
    Ok(Livery {
        team: team.to_string(),
        hull,
        nozzle,
        collision_fx,
        boost,
        boost_uv,
    })
}

/// The `Engine Flare` locator's position in a hull's own model space.
///
/// Takes the **first** node if a model somehow had several. Every team checked
/// has exactly one, so this is a total order on a set of size one rather than a
/// policy.
fn engine_flare(ship_blob: &[u8]) -> Option<Vec3> {
    // The id from the file's own version word, not the version-6 constant: on a
    // version-4 ship `0x3bf` is some other class entirely, so a hit would be a
    // flare mounted on whatever that is. `None` means no flare, which is what an
    // unrecovered id honestly gives.
    let class = vex::classes_of(ship_blob).ok()?.engine_flare?;
    let nodes = vex::nodes(ship_blob).ok()?;
    let m = vex::class_world_transforms(ship_blob, &nodes, class)
        .into_iter()
        .next()?;
    Some(Vec3::new(m[12], m[13], m[14]))
}

/// Every `Ship Collision Fx` locator's position in a hull's own model space -
/// the same 4x4-payload decode as [`engine_flare`], kept all rather than first:
/// the original picks the nearest to each contact.
fn collision_fx_locators(ship_blob: &[u8]) -> Vec<Vec3> {
    // Version-keyed, on the same terms as [`engine_flare`].
    let Ok(Some(class)) = vex::classes_of(ship_blob).map(|classes| classes.ship_collision_fx)
    else {
        return Vec::new();
    };
    let Ok(nodes) = vex::nodes(ship_blob) else {
        return Vec::new();
    };
    vex::class_world_transforms(ship_blob, &nodes, class)
        .into_iter()
        .map(|m| Vec3::new(m[12], m[13], m[14]))
        .collect()
}

/// The plume `ExhaustFlare_Init` reveals once `boost_timer` passes
/// `exhaust::BOOST_GATE`: `shipboost.vex`, or `Zoneboost.vex` in Zone mode,
/// matching whichever hull [`ship_entry_name`] picked.
///
/// Absence is reported rather than failing the race: the PS2 set may not carry
/// it under the same name the PSP one does, and a missing boost plume is a
/// missing feature, not a broken load.
///
/// **The authored vertex alpha is left alone here, and the blend at the draw
/// site weights by it per fragment.** This used to premultiply alpha into the
/// RGB, and that was a real bug rather than a compensation: premultiplying at a
/// vertex and then letting the rasteriser interpolate is not the same
/// arithmetic as interpolating and then multiplying, and for this model the
/// difference is the whole visual.
///
/// What the authored data holds, measured on every batch of every PSP team's
/// `shipboost.vex` and pinned by `boost_plume_ground_truth.rs`, is why:
///
/// - the vertex colours are exactly **two** values, `(255, 98, 5, 0)` on the rim
///   and `(255, 255, 255, 255)` in the core, 29 and 22 of a 51-vertex batch,
///   nothing between;
/// - `pulse_boost2_ADD` is 64x16 with a **constant alpha of 238**, so the
///   texture supplies no gradient either.
///
/// Premultiplied per vertex, the rim becomes `(0, 0, 0)` and what crosses the
/// fin is black-to-white: **the authored orange exists nowhere on it**, which is
/// exactly the "reads white and un-orange" symptom. Weighted per fragment,
/// colour interpolates orange-to-white while alpha interpolates `0`-to-`1` and
/// the two meet at the fragment, leaving a dimmed orange fringe that fades out -
/// the original's own feathered edge.
///
/// Measured against the first matched-pose pad capture, over the magenta
/// signature in a crop around the craft: the original reads `(224, 166, 227)`,
/// the premultiplied build `(151, 93, 212)` - blue-dominant, hue lost - and the
/// raw build `(222, 168, 230)`, within three units per channel on all three. See
/// the draw site in `race::Scene::new` for the full table and for what remains
/// unrecovered: the GE's own route for this alpha, since
/// `Mesh_SetBatchDrawState` programs `GU_FIX` white on both sides *after* the
/// material's own display list.
///
/// Two suspects were raised and **refuted** earlier, recorded so nobody spends
/// the same afternoon on them. The plume's authored `u` never leaves the first
/// texel (`[0.000, 0.008]` against a `v` of `[0.031, 0.953]`), so it samples one
/// column of `pulse_boost2_ADD`; rendering with `u` scaled by 128, and again
/// with `u`/`v` swapped, changed the picture not at all. And the texture is
/// neither dropped nor mis-bound: replacing it with hard horizontal stripes
/// banded the two trailing streaks while leaving the wedges flat. What that
/// second test did expose is still open - the two short batches (9 and 10
/// vertices) decode to a **single** UV point, `u [0.008, 0.008]`, `v [0.031,
/// 0.031]`, so no texture content can reach them whatever the scale, while the
/// 51-vertex batches carry a real `v` sweep and do band.
///
/// **The load-time texel re-encode that used to sit here is gone, and putting it
/// back would be a double-encode.** It existed only to cancel `Drawable`'s
/// `Rgba8UnormSrgb` upload, whose sampler linearised the disc's bytes before the
/// shader saw them. That upload is raw now
/// ([ADR-0020](../../../docs/architecture/adr/0020-gamma-authoritative-colour-space.md)),
/// so the sampler already hands the shader the disc's own values and there is
/// nothing left to compensate for. The effect it was compensating for is real -
/// `One`/`One` on linearised texels crushes the halo's mid-tones by ~30% of
/// encoded brightness - which is why the fix moved to the upload rather than
/// being dropped.
fn plume(
    archives: &mut oag_assets::Archives,
    team: &str,
    mode: Mode,
    lod: mesh::Lod,
    report: &mut Vec<String>,
) -> (Option<Model>, Option<vex::TexTransform>) {
    let boost_name = boost_entry_name(team, mode);
    let Ok(blob) = archives.read_name(&boost_name) else {
        report.push(format!(
            "{boost_name}: not in the archive set - no boost plume for this team"
        ));
        return (None, None);
    };
    let model = match mesh::build_with_textures(&boost_name, &blob, None, lod) {
        Ok(model) => model,
        Err(error) => {
            report.push(format!(
                "{boost_name}: {} bytes, does not parse ({error}) - no boost plume for this team",
                blob.len()
            ));
            return (None, None);
        }
    };
    report.push(format!(
        "{boost_name}: {} triangle(s) - drawn additively while the plume is up",
        model.indices.len() / 3
    ));
    // The authored u-scroll: the keyframe block after each mesh's material
    // array, which the engine lerps per frame and feeds the GE as
    // `TEXOFFSET`/`TEXSCALE`. Both plume meshes carry the identical track on
    // every team read, so the first mesh that has one speaks for the model.
    let mesh_class = vex::classes_of(&blob).ok().and_then(|classes| classes.mesh);
    let transform = vex::nodes(&blob).ok().and_then(|nodes| {
        nodes
            .iter()
            .filter(|node| Some(node.class_id) == mesh_class)
            .find_map(|node| vex::mesh_tex_transform(&blob[node.payload()]))
    });
    if let Some(transform) = &transform {
        report.push(format!(
            "{boost_name}: authored uv scroll, {} offset key(s) over {} frames",
            transform.offset.times.len(),
            transform.offset.period(),
        ));
    }
    (model.into(), transform)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(names: &[&str]) -> Vec<String> {
        names.iter().map(|name| (*name).to_string()).collect()
    }

    #[test]
    fn the_player_takes_the_first_slot_and_the_rest_follow_in_order() {
        let slots = teams_for_slots("b", &ids(&["a", "b", "c", "d"]), 4);
        assert_eq!(slots, ids(&["b", "a", "c", "d"]));
    }

    /// A player racing for the last team must not reshuffle everyone else: the
    /// player is *skipped* in the list, not moved to the front of it.
    #[test]
    fn the_players_own_team_is_skipped_rather_than_moved() {
        let with_first = teams_for_slots("a", &ids(&["a", "b", "c"]), 3);
        let with_last = teams_for_slots("c", &ids(&["a", "b", "c"]), 3);
        assert_eq!(with_first, ids(&["a", "b", "c"]));
        assert_eq!(with_last, ids(&["c", "a", "b"]));
    }

    /// The Pure and PS2 case. A repeated livery beats a whole grid in one.
    #[test]
    fn a_short_list_cycles() {
        let slots = teams_for_slots("a", &ids(&["a", "b", "c"]), 6);
        assert_eq!(slots, ids(&["a", "b", "c", "b", "c", "b"]));
    }

    /// The DLC case: four packs take Pulse to twelve teams for eight slots, so
    /// the extra teams simply do not race. Which seven of eleven do is
    /// unrecovered - see the module docs.
    #[test]
    fn a_long_list_is_truncated_to_the_grid() {
        let available = ids(&["a", "b", "c", "d", "e", "f"]);
        let slots = teams_for_slots("a", &available, 3);
        assert_eq!(slots, ids(&["a", "b", "c"]));
    }

    /// What a source with no readable definition gets, and what this engine did
    /// before liveries existed.
    #[test]
    fn no_other_teams_means_the_whole_grid_wears_the_players_hull() {
        let slots = teams_for_slots("a", &[], 4);
        assert_eq!(slots, ids(&["a", "a", "a", "a"]));
        let only_the_player = teams_for_slots("a", &ids(&["a"]), 4);
        assert_eq!(only_the_player, ids(&["a", "a", "a", "a"]));
    }

    #[test]
    fn one_slot_is_just_the_player() {
        assert_eq!(teams_for_slots("a", &ids(&["a", "b"]), 1), ids(&["a"]));
    }
}
