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
use oag_race::Mode;
use oag_render::mesh::{self, Model};
use oag_vex::vex;

use crate::race::{boost_entry_name, ps2_texture_set, ship_entry_name};

mod absorb;
pub(crate) mod engine_light;
mod flare;
pub(crate) mod ship_skin;

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
    ///
    /// **On Wipeout HD this is the `EF_Boost` group of the flare model**, not a
    /// file of its own - HD ships no `shipboost.vex` and the plume is a subtree
    /// of `engineflare.vex`. See [`flare::per_team`].
    pub boost: Option<Model>,
    /// The always-on engine flame, for a title that authors one as a model.
    ///
    /// `None` on Pulse, Pure and the PS2 port, whose flare is a sprite the
    /// exhaust pipeline draws rather than geometry - see
    /// [`oag_title::flare::Flare`].
    pub flare: Option<Model>,
    /// The additive shell a fired Shield pickup raises,
    /// `shipshield.vex` beside the hull - or `Data\Weapons\shield.vex` on a
    /// source that carries no per-team one, which is every Pure source.
    ///
    /// Per team for the reason [`Self::boost`] is, and more so: the shell is
    /// modelled to the hull it wraps, so one team's around another's would sit
    /// inside the fuselage in places and outside it in others. See
    /// [`crate::race::shield_entry_names`].
    pub shield: Option<Model>,
    /// The `Ship Collision Fx` locators in this hull's own model space, the
    /// spark anchors the original picks the nearest of.
    ///
    /// Per team for the same reason [`Self::nozzle`] is: the hulls are
    /// different models, so a locator set belongs to the one it was authored
    /// on.
    pub collision_fx: Vec<SparkAnchor>,
    /// Wipeout HD's `absorb` locators, model space, where its absorb burst
    /// plays - empty on every source that has no such class. See [`absorb`].
    pub absorb: Vec<Vec3>,
    /// The hull redrawn under `absorb_surface.mip` for the second after an
    /// absorb - see [`absorb::overlay`]. `None` wherever it is not built.
    pub absorb_overlay: Option<Model>,
    /// The same under [`absorb::LEACH`], while this craft's LeachBeam pulses.
    pub leach_overlay: Option<Model>,
    /// HD's `AbsorbEffect` shell - see [`absorb::shell`]. `None` elsewhere.
    pub absorb_shell: Option<Model>,
    /// The plume's own authored texture-transform animation.
    ///
    /// **Per team, and this is not a formality.** Eight per-team plumes sampled
    /// through one team's keyframes is the kind of wrong that renders
    /// plausibly. Every team read so far carries the identical track, which
    /// makes it *currently* invisible and is not a reason to share one.
    pub boost_uv: Option<vex::TexTransform>,
    /// Wipeout HD's engine light: `EngineLightData.xml` and the flare
    /// locator's axis, or `None` on every title that authors neither. See
    /// [`engine_light::load`].
    pub engine_light: Option<engine_light::EngineLight>,
}

/// One `Ship Collision Fx` locator, in its hull's own model space: where a
/// spark burst spawns, and which way its authored `+Y` points.
///
/// Both fields come off the same 4x4 payload
/// [`oag_vex::vex::class_world_transforms`] decodes - `position` is the
/// matrix's row 3, `up` its row 1 (see `oag_vex::vex::transform`'s own doc
/// comment for the row convention). Composing `up` with the hull's own live
/// matrix, the way `position` already is every tick, is what lets a burst's
/// spray direction follow a banking craft instead of assuming world up -
/// see `oag_render::psys::System::advance`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SparkAnchor {
    /// The locator's own position, model space.
    pub position: Vec3,
    /// The locator's authored `+Y` axis, model space. `(0, 1, 0)` on every
    /// locator measured so far - see `docs/formats/pob.md`'s "the authored
    /// emitter-node frame" section - but read from the file rather than
    /// assumed, so a hull whose locator is not a pure translation is still
    /// played correctly rather than silently flattened to world up.
    pub up: Vec3,
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

/// The per-race context [`load`]/[`one`] need beyond the team list, the
/// archives and the report - grouped so neither function's own parameter
/// list grows past `clippy::too_many_arguments` every time a title fact
/// joins it.
#[derive(Debug, Clone, Copy)]
pub struct LoadContext<'a> {
    /// Where a team's hull, plume, shield and handling stats resolve - see
    /// [`oag_title::RaceDefaults::ships_for`].
    pub race: &'a oag_title::RaceDefaults,
    /// Which mode's rules the race runs under - selects [`ship_entry_name`]'s
    /// Zone/non-Zone branch.
    pub mode: Mode,
    /// This title's own engine-flare mechanism - geometry, a sprite, or
    /// unread. See [`oag_title::flare::Flare`].
    pub flare: &'a oag_title::flare::Flare,
    /// Whether to build the absorb hull overlay - Pulse's alone, and only
    /// while `oag_render::hull_overlay::DRAWN` says so. See [`absorb::overlay`].
    pub hull_overlay: bool,
    /// Whether to load HD's absorb shell. See [`absorb::shell`].
    pub absorb_shell: bool,
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
///
/// `hull_variant` is [`crate::race::ship_entry_name`]'s own override, applied
/// to **slot 0 only**: it is the RACE page's own pick for the player's craft,
/// and nothing offers an opponent a choice of their own - see
/// `oag_title::race::HullVariant`'s doc comment for why there is no unlock
/// model behind it either.
///
/// `skin` is the *other* half of that axis and lands on slot 0 for the same
/// reason: a `PI_ModelSkin`'s own `.dat`, already resolved to an archive entry
/// by [`ship_skin::resolve`], repainting the hull this slot just built. **Which
/// skin a race flies is chosen, not measured** - see that module's own docs
/// for what the original appears to do instead and what would settle it.
pub fn load(
    archives: &mut oag_assets::Archives,
    teams: &[String],
    ctx: &LoadContext,
    hull_variant: Option<&str>,
    skin: Option<&str>,
    report: &mut Vec<String>,
) -> Result<Vec<Livery>> {
    let mut out: Vec<Livery> = Vec::with_capacity(teams.len());
    for (slot, team) in teams.iter().enumerate() {
        // Already loaded for an earlier slot: clone rather than re-read, which
        // is what makes a repeated livery cost nothing. Never true for slot 0
        // itself, so a variant pick can only ever land once, on the player.
        if let Some(first) = out.iter().position(|other| &other.team == team) {
            let source = &out[first];
            let livery = Livery {
                team: team.clone(),
                hull: source.hull.clone(),
                nozzle: source.nozzle,
                collision_fx: source.collision_fx.clone(),
                absorb: source.absorb.clone(),
                absorb_overlay: source.absorb_overlay.clone(),
                leach_overlay: source.leach_overlay.clone(),
                absorb_shell: source.absorb_shell.clone(),
                boost: source.boost.clone(),
                boost_uv: source.boost_uv.clone(),
                flare: source.flare.clone(),
                shield: source.shield.clone(),
                engine_light: source.engine_light,
            };
            report.push(format!(
                "slot {slot}: {team}, the same livery as slot {first} - the source declares \
                 fewer teams than the grid has slots"
            ));
            out.push(livery);
            continue;
        }

        match one(
            archives,
            team,
            ctx.race.ships_for(team),
            ctx,
            if slot == 0 { hull_variant } else { None },
            if slot == 0 { skin } else { None },
            report,
        ) {
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
                    absorb: player.absorb.clone(),
                    absorb_overlay: player.absorb_overlay.clone(),
                    leach_overlay: player.leach_overlay.clone(),
                    absorb_shell: player.absorb_shell.clone(),
                    boost: player.boost.clone(),
                    boost_uv: player.boost_uv.clone(),
                    flare: player.flare.clone(),
                    shield: player.shield.clone(),
                    engine_light: player.engine_light,
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
    ships: oag_title::race::ShipPaths,
    ctx: &LoadContext,
    hull_variant: Option<&str>,
    skin: Option<&str>,
    report: &mut Vec<String>,
) -> Result<Livery> {
    let hull_name = ship_entry_name(ships, team, ctx.mode, hull_variant);
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
        // **A sibling that will not decode is the same outcome as no sibling**,
        // and it is Wipeout 2048 that made the distinction matter: it ships a
        // `.rcsmodel` for every craft and the container is not HD's, so this
        // used to be a hard error that stopped the race rather than a craft
        // that does not draw. Which of the two happened is in the report,
        // because they need different work to fix.
        let built = match geometry {
            None => Err(format!(
                "{hull_name}: a .vex with external geometry and no .rcsmodel beside it"
            )),
            // 2048's container is a different file under the same extension
            // and needs no `.vex` at all - see `oag_render::mesh::rcs::psp2`.
            Some(geometry) if mesh::rcs::psp2::is_psp2(&geometry) => {
                mesh::rcs::psp2::build(&hull_name, &geometry, None, &mut |path| {
                    archives.read_name(path).ok()
                })
                .map(|(model, built)| (model, built.describe()))
                .map_err(|error| format!("{hull_name}: {error:#}"))
            }
            Some(geometry) => mesh::rcs::build(
                &hull_name,
                &blob,
                &geometry,
                &mut |path| archives.read_name(path).ok(),
                |c| c.mesh,
            )
            .map(|(model, built)| (model, built.describe()))
            .map_err(|error| format!("{hull_name}: {error:#}")),
        };
        let (hull, built) = match built {
            Ok(pair) => pair,
            Err(why) => {
                report.push(format!(
                    "{why} - nothing draws for this craft, and the race still runs"
                ));
                return Ok(Livery {
                    team: team.to_string(),
                    hull: mesh::Model::none(&hull_name),
                    nozzle: None,
                    collision_fx: Vec::new(),
                    absorb: Vec::new(),
                    absorb_overlay: None,
                    leach_overlay: None,
                    absorb_shell: None,
                    boost: None,
                    boost_uv: None,
                    flare: None,
                    shield: None,
                    engine_light: None,
                });
            }
        };
        report.push(format!("{hull_name}: {built}"));
        // **No skin on this branch, and that is not a gap.** A `.dat` is a
        // paletted 4bpp swap onto a `\TEXTUREn.TGA` slot, which is the PSP
        // pipeline; HD and 2048 carry no such file and name no
        // `PI_ModelSkin` at all. A caller that asked for one on a PS3 or Vita
        // source hears so rather than getting silence.
        if let Some(skin) = skin {
            report.push(format!(
                "{skin}: a ship skin is a PSP-pipeline texture swap and {hull_name} takes its                  geometry from a .rcsmodel - the craft keeps its own paint"
            ));
        }
        let (nozzle, nozzle_axis, collision_fx) = locators(archives, &hull_name, &blob, report);
        let absorb = absorb::locators(archives, &hull_name, &blob, report);
        let engine_light =
            engine_light::load(archives, team, ships.dir, ctx.flare, nozzle_axis, report);
        // **The plume comes from here too on this title.** HD ships no
        // `shipboost.vex`; `EF_Boost` inside the flare model is what a speed
        // pad reveals, so both halves come out of one load. See
        // `flare::per_team`.
        let lit = authored_flare(archives, team, ships.dir, ctx.flare, nozzle, report);
        return Ok(Livery {
            team: team.to_string(),
            nozzle,
            collision_fx,
            absorb,
            absorb_overlay: None,
            leach_overlay: None,
            absorb_shell: absorb::shell(archives, team, ships.dir, ctx.absorb_shell, report),
            hull,
            boost: lit.boost,
            flare: lit.always,
            boost_uv: None,
            // **The shell is loaded on PS3 too**, unlike the plume: HD carries a
            // per-team `shipshield.vex` with its geometry in the `.rcsmodel`
            // beside it, under the same stem Pulse uses, for all eight teams and
            // Zone. `shell` takes the same external-geometry branch this hull
            // just took. See `crate::race::shield_entry_names`.
            shield: shell(archives, team, ships.dir, report),
            engine_light,
        });
    }
    let mut hull = mesh::build_with_textures(&hull_name, &blob, None)?;
    // The PS2 signature: `Texture` nodes exist (the model wants textures) but
    // every one is missing (its embedded block was empty). Only then is the
    // directory-position heuristic worth trying - see `ps2_texture_set`.
    if !hull.textures.is_empty()
        && hull.textures.iter().all(Option::is_none)
        && let Some(external) = ps2_texture_set(archives, &hull_name)
    {
        hull = mesh::build_with_textures(&hull_name, &blob, Some(&external))?;
    }
    // **After the PS2 rebuild above, never before it.** That branch replaces
    // `hull` wholesale, so a skin applied earlier would be thrown away with
    // the model it was painted onto.
    if let Some(skin) = skin {
        ship_skin::apply(archives, skin, &mut hull, report);
    }
    report.push(format!(
        "{hull_name}: {} triangle(s), model centre {:?}, radius {:.2}",
        hull.indices.len() / 3,
        hull.centre,
        hull.radius
    ));

    let (nozzle, nozzle_axis, collision_fx) = locators(archives, &hull_name, &blob, report);
    let engine_light =
        engine_light::load(archives, team, ships.dir, ctx.flare, nozzle_axis, report);

    let (boost, boost_uv) = plume(archives, team, ships, ctx.mode, report);
    let shield = shell(archives, team, ships.dir, report);
    // Nothing on a title whose flare is a sprite, which is every source that
    // reaches this branch today - the match is here rather than at the PS3
    // branch alone so a fourth source is answered by its own axis and not by
    // which decoder its hull happened to take.
    let lit = authored_flare(archives, team, ships.dir, ctx.flare, nozzle, report);
    let [absorb_overlay, leach_overlay] = [absorb::ABSORB, absorb::LEACH]
        .map(|which| absorb::overlay(archives, &hull, &blob, which, ctx.hull_overlay, report));
    Ok(Livery {
        team: team.to_string(),
        hull,
        nozzle,
        collision_fx,
        absorb: Vec::new(),
        absorb_overlay,
        leach_overlay,
        absorb_shell: None,
        boost,
        boost_uv,
        flare: lit.always,
        shield,
        engine_light,
    })
}

/// The flare this title authors as geometry, or nothing at all.
///
/// **A `Sprite` title yields nothing here and that is not a gap**: its flare is
/// six camera-facing vertices the exhaust pipeline draws from a texture, which
/// is a different mechanism entirely rather than a missing model. See
/// [`oag_title::flare::Flare`].
fn authored_flare(
    archives: &mut oag_assets::Archives,
    team: &str,
    ship_dir: &str,
    flare: &oag_title::flare::Flare,
    nozzle: Option<Vec3>,
    report: &mut Vec<String>,
) -> flare::Flare {
    match flare {
        oag_title::flare::Flare::PerTeam(authored) => {
            flare::per_team(archives, team, ship_dir, authored, nozzle, report)
        }
        oag_title::flare::Flare::Sprite(_) | oag_title::flare::Flare::Unread => {
            flare::Flare::default()
        }
    }
}

/// The hull's `Engine Flare` and `Ship Collision Fx` locators, from whichever
/// file of the pair actually carries them.
///
/// **Wipeout HD keeps them in a separate `Locators.vex`.** Pulse and the PS2
/// port put every locator node in the hull's own `.vex`, so the hull blob is
/// the whole answer there. On the PS3 set `Ship.vex` carries the class ids and
/// **no nodes of either class**, while `Locators.vex` beside it carries one
/// `Engine Flare` and ten `Ship Collision Fx` - which is why HD's engine flare
/// never attached: the effect loaded and there was no nozzle to hang it on.
///
/// The sibling is only read when the hull yields nothing, so a Pulse or PS2
/// source never looks for a file it does not ship, and a set that carries both
/// keeps the hull's own answer.
fn locators(
    archives: &mut oag_assets::Archives,
    hull_name: &str,
    blob: &[u8],
    report: &mut Vec<String>,
) -> (Option<Vec3>, Option<Vec3>, Vec<SparkAnchor>) {
    let mut source = hull_name.to_string();
    let mut flare = engine_flare(blob);
    let mut collision_fx = collision_fx_locators(blob);

    if flare.is_none()
        && collision_fx.is_empty()
        && let Some(name) = sibling_entry(hull_name, LOCATORS_ENTRY)
        && let Ok(sibling) = archives.read_name(&name)
    {
        flare = engine_flare(&sibling);
        collision_fx = collision_fx_locators(&sibling);
        if flare.is_some() || !collision_fx.is_empty() {
            source = name;
        }
    }

    let nozzle = flare.map(|(position, _)| position);
    let nozzle_axis = flare.map(|(_, axis)| axis);
    match flare {
        Some((at, axis)) => report.push(format!(
            "{source}: engine_flare locator at {at:?} in model space, Z axis {axis:?}"
        )),
        None => report.push(format!(
            "{hull_name}: no Engine Flare node - this craft's exhaust will not be drawn"
        )),
    }
    report.push(if collision_fx.is_empty() {
        format!("{hull_name}: no Ship Collision Fx nodes - sparks anchor at the contact point")
    } else {
        format!(
            "{source}: {} Ship Collision Fx locator(s) for the spark anchor",
            collision_fx.len()
        )
    });

    (nozzle, nozzle_axis, collision_fx)
}

/// The file HD keeps its locator nodes in, beside the hull.
const LOCATORS_ENTRY: &str = "Locators.vex";

/// `name` with its last path component replaced by `file`.
///
/// Both separators, because a WAD entry is spelled with backslashes and a
/// PSARC path with forward ones, and this runs on whichever the caller was
/// handed.
fn sibling_entry(name: &str, file: &str) -> Option<String> {
    let at = name.rfind(['\\', '/'])?;
    Some(format!("{}{file}", &name[..=at]))
}

/// The `Engine Flare` locator's position and Z axis in a hull's own model
/// space - the matrix's row 3 and row 2, the same rows
/// `EngineFlare_SubmitSpuLight` reads off the node's world matrix
/// (`+0x30`/`+0x20`; see [`engine_light`]).
///
/// Takes the **first** node if a model somehow had several. Every team checked
/// has exactly one, so this is a total order on a set of size one rather than a
/// policy.
fn engine_flare(ship_blob: &[u8]) -> Option<(Vec3, Vec3)> {
    // The id from the file's own version word, not the version-6 constant: on a
    // version-4 ship `0x3bf` is some other class entirely, so a hit would be a
    // flare mounted on whatever that is. `None` means no flare, which is what an
    // unrecovered id honestly gives.
    let class = vex::classes_of(ship_blob).ok()?.engine_flare?;
    let nodes = vex::nodes(ship_blob).ok()?;
    let m = vex::class_world_transforms(ship_blob, &nodes, class)
        .into_iter()
        .next()?;
    Some((Vec3::new(m[12], m[13], m[14]), Vec3::new(m[8], m[9], m[10])))
}

/// Every `Ship Collision Fx` locator in a hull's own model space - the same
/// 4x4-payload decode as [`engine_flare`], kept all rather than first: the
/// original picks the nearest to each contact. [`SparkAnchor::up`] is read
/// off the same matrix `engine_flare` discards the rotation of, rather than
/// assumed - see [`SparkAnchor`]'s own doc comment.
fn collision_fx_locators(ship_blob: &[u8]) -> Vec<SparkAnchor> {
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
        .map(|m| SparkAnchor {
            position: Vec3::new(m[12], m[13], m[14]),
            up: Vec3::new(m[4], m[5], m[6]),
        })
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
/// The shell a fired Shield pickup raises, from the first of
/// [`crate::race::shield_entry_names`] the source actually carries.
///
/// **Absence is reported rather than fatal**, exactly as [`plume`]'s is: a
/// source without either model is a source without a shield visual, which is a
/// missing feature and not a broken load. The report names which of the two
/// names answered, because they mean different things - the per-team one is what
/// `ShipShield_Construct` (`0x0885db38`) assembles, the shared one is this
/// project's reading of what a Pure source must mean by the same effect.
///
/// **No texture-transform track and no vertex rework**, unlike the plume: the
/// model is one mesh with one `_ADD` texture, and the animation is entirely in
/// the transform and the vertex colour that
/// [`oag_render::shield::ShipShield`] computes. Anything done to the mesh here
/// would be a second, invisible place for the look to come from.
/// One `.vex` by name, from whichever of the two geometry layouts the source
/// uses, or a sentence saying why not.
///
/// **The two layouts are a per-title fact, not a per-model one.** A PSP `.vex`
/// carries its own geometry; a PS3 one is a header whose vertices live in the
/// `.rcsmodel` beside it, which is why [`one`] already branches this way for the
/// hull. Every shield model this file loads needs the same branch, so it is here
/// once rather than copied per caller.
///
/// `Ok` carries a description of what was built, for the loader report; `Err`
/// carries the line to report instead.
fn shield_model(
    archives: &mut oag_assets::Archives,
    name: &str,
) -> Result<(Model, String), String> {
    let blob = archives
        .read_name(name)
        .map_err(|_| format!("{name}: not in the archive set"))?;
    if !mesh::geometry_is_external(&blob) {
        let model = mesh::build_with_textures(name, &blob, None)
            .map_err(|error| format!("{name}: {} bytes, does not parse ({error})", blob.len()))?;
        let note = format!("{} triangle(s)", model.indices.len() / 3);
        return Ok((model, note));
    }
    let sibling = mesh::rcs::sibling_name(name);
    let geometry = sibling
        .as_deref()
        .and_then(|s| archives.read_name(s).ok())
        .ok_or_else(|| format!("{name}: a PS3 .vex with no .rcsmodel beside it"))?;
    let (model, built) = mesh::rcs::build(
        name,
        &blob,
        &geometry,
        &mut |path| archives.read_name(path).ok(),
        |c| c.mesh,
    )
    .map_err(|error| format!("{name}: its .rcsmodel does not build ({error})"))?;
    // **Said out loud because the picture does not say it.** A shield model
    // draws additively and `mesh::rcs` resolves no material - HD authors one per
    // team (`/data/materials/ships/<team>_shield.rcsmaterial`) and nothing here
    // reads it. So the geometry is the disc's and the *brightness* is not: an
    // untextured additive surface at full white washes the hull out, where the
    // PSP shell's own vertices carry a blue-violet at half alpha. Drawing it is
    // still right - it is real decoded geometry, and the substitute is for the
    // missing material alone - but a reviewer looking at an HD capture has to be
    // told which half is recovered.
    Ok((
        model,
        format!(
            "{}, and no material - HD's own .rcsmaterial is not read, so it is \
             brighter and flatter than the disc's",
            built.describe()
        ),
    ))
}

/// The shell a fired Shield pickup raises, from the first of
/// [`crate::race::shield_entry_names`] the source actually carries.
///
/// **Absence is reported rather than fatal**, exactly as [`plume`]'s is: a
/// source without either model is a source without a shield visual, which is a
/// missing feature and not a broken load. The report names which of the two
/// names answered, because they mean different things - the per-team one is what
/// `ShipShield_Construct` (`0x0885db38`) assembles, the shared one is this
/// project's reading of what a Pure source must mean by the same effect.
fn shell(
    archives: &mut oag_assets::Archives,
    team: &str,
    ship_dir: &str,
    report: &mut Vec<String>,
) -> Option<Model> {
    let names = crate::race::shield_entry_names(ship_dir, team);
    for (index, name) in names.iter().enumerate() {
        let provenance = if index == 0 {
            "the team's own"
        } else {
            "shared - this source carries no per-team shell"
        };
        match shield_model(archives, name) {
            Ok((model, note)) => {
                report.push(format!("{name}: {note} - the shield shell, {provenance}"));
                return Some(model);
            }
            Err(line) => report.push(line),
        }
    }
    report.push(format!(
        "{}: no shield shell for this team",
        names.join(" nor ")
    ));
    None
}

/// The sphere the original draws **instead of** the shell when the camera is
/// inside the craft, `Data\Weapons\vr_shield_cockpit.vex`.
///
/// Not per team and not per track - one entry serves every craft, which is why
/// it loads once beside the rocket's model rather than per [`Livery`]. On the
/// disc for all three titles: Pulse and Pure carry it whole, HD as the usual
/// `.vex`/`.rcsmodel` pair.
///
/// **`ShipShield_Update` chooses between the two on `craft+0x6d`**, the same
/// byte `oag_display::display::CameraView::draws_own_ship` reads - see that function,
/// whose confidence-70 note this is a second consumer for.
pub(crate) fn cockpit_shield(
    archives: &mut oag_assets::Archives,
    report: &mut Vec<String>,
) -> Option<Model> {
    let name = oag_pulse::race::COCKPIT_SHIELD;
    match shield_model(archives, name) {
        Ok((model, note)) => {
            report.push(format!(
                "{name}: {note} - the shield seen from inside the cockpit"
            ));
            Some(model)
        }
        Err(line) => {
            report.push(format!("{line} - no cockpit shield"));
            None
        }
    }
}

fn plume(
    archives: &mut oag_assets::Archives,
    team: &str,
    ships: oag_title::race::ShipPaths,
    mode: Mode,
    report: &mut Vec<String>,
) -> (Option<Model>, Option<vex::TexTransform>) {
    let Some(boost_name) = boost_entry_name(ships, team, mode) else {
        report.push(format!("{team}: names no standalone boost-plume model"));
        return (None, None);
    };
    let Ok(blob) = archives.read_name(&boost_name) else {
        report.push(format!(
            "{boost_name}: not in the archive set - no boost plume for this team"
        ));
        return (None, None);
    };
    let mut model = match mesh::build_with_textures(&boost_name, &blob, None) {
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
    // The same PS2 signature the hull takes at [`one`], on the same gate and
    // for the same reason: `Texture` nodes exist and every one of them is
    // missing, because a PS2 model's own texture block is empty by design and
    // its pixels are the archive entry directly before it. The gate is what
    // keeps a PSP plume - one embedded `pulse_boost2_ADD` - out of this branch
    // entirely, and it must stay narrow enough never to overwrite a model that
    // already decoded on its own. See `ps2_texture_set`.
    if !model.textures.is_empty() && model.textures.iter().all(Option::is_none) {
        ps2_skin(archives, &boost_name, &blob, &mut model, report);
    }
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
    // **The PS2 plume hangs off two moving anchors and the PSP one off
    // nothing**, which is the second half of what the two files disagree
    // about. Reported rather than assumed because the placement and the
    // motion both come from it: geometry under an `Anim Transform` is baked
    // in that node's own space, so a plume whose table is never written draws
    // at the craft's origin with its two fins collapsed together. See
    // `race::scene::frame`'s draw loop, which writes it from the plume's own
    // reveal timer.
    if !model.anim_nodes.is_empty() {
        report.push(format!(
            "{boost_name}: {} anchor(s) with authored motion - the plume is placed \
             and animated by them, not by the mesh alone",
            model.anim_nodes.len(),
        ));
    }
    (model.into(), transform)
}

/// Re-skins a PS2 model from the texture set in the archive entry before it.
///
/// Split out of [`plume`] rather than inlined the way [`one`]'s is, because a
/// missing plume is a reported absence and not an error: every branch here
/// leaves `model` drawable and says in the report which one it took. A
/// silently untextured plume and a correctly skinned one look the same in a
/// log that only counts triangles.
///
/// **The lookup is the same directory-position rule the hull and the track
/// already use** - see `ps2_texture_set` and `docs/formats/ps2-texture.md`.
/// This is a third model type answering to it, not a new heuristic: every one
/// of the PS2 disc's 24 plume files (twelve teams, `shipboost` and
/// `Zoneboost` each) declares exactly two `Texture` nodes and is preceded by a
/// set that decodes exactly two entries.
fn ps2_skin(
    archives: &mut oag_assets::Archives,
    name: &str,
    blob: &[u8],
    model: &mut mesh::Model,
    report: &mut Vec<String>,
) {
    let slots = model.textures.len();
    let Some(external) = ps2_texture_set(archives, name) else {
        report.push(format!(
            "{name}: {slots} empty texture slot(s) and the preceding archive entry is \
             not a texture set - the plume draws untextured"
        ));
        return;
    };
    let found = external.entry_count();
    let decoded = external.decoded_count();
    match mesh::build_with_textures(name, blob, Some(&external)) {
        Ok(rebuilt) => {
            *model = rebuilt;
            report.push(format!(
                "{name}: {decoded} of {found} texture(s) from the preceding archive entry, \
                 into {slots} slot(s)"
            ));
        }
        Err(error) => report.push(format!(
            "{name}: the preceding archive entry holds {found} texture(s) but re-skinning \
             failed ({error}) - the plume draws untextured"
        )),
    }
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

    /// Both separators, because a WAD entry is spelled with backslashes and a
    /// PSARC path with forward ones and this runs on either.
    #[test]
    fn a_sibling_replaces_only_the_last_path_component() {
        assert_eq!(
            sibling_entry(r"Data\Ships\Detonator\Ship.vex", LOCATORS_ENTRY).as_deref(),
            Some(r"Data\Ships\Detonator\Locators.vex")
        );
        assert_eq!(
            sibling_entry("/data/ships/detonator/ship.vex", LOCATORS_ENTRY).as_deref(),
            Some("/data/ships/detonator/Locators.vex")
        );
        assert_eq!(sibling_entry("Ship.vex", LOCATORS_ENTRY), None);
    }
}
