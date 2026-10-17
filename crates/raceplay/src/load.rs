//! [`load`]: everything a race needs, read out of one disc image, and the report
//! of what came back.
//!
//! Split out of `race.rs` under the 1,000-line rule; its tests are `race/tests/load.rs`.

use super::*;
#[cfg(feature = "view")]
mod audio;
#[cfg(feature = "view")]
mod cameras;
#[cfg(feature = "view")]
pub mod campaign;
mod closing_report;
#[cfg(feature = "view")]
mod countdown;
#[cfg(feature = "view")]
mod engine_light;
#[cfg(feature = "view")]
mod environment;
mod family;
#[cfg(feature = "view")]
mod gantry_visibility;
#[cfg(feature = "view")]
mod geometry;
mod global;
#[cfg(feature = "view")]
mod glow_mask;
#[cfg(feature = "view")]
mod intro;
#[cfg(feature = "view")]
pub(crate) mod magstrip_wake;
mod original_laws;
#[cfg(feature = "view")]
mod pads;
mod pose;
#[cfg(feature = "view")]
mod pulse_ps2;
mod pulse_psp;
#[cfg(feature = "view")]
pub(super) mod ripple;
mod roster;
#[cfg(feature = "view")]
mod skin;
mod surfaces;
mod track_inputs;
mod track_stats;
#[cfg(feature = "view")]
mod variant;
#[cfg(feature = "view")]
mod weapon_models;
#[cfg(feature = "view")]
mod zone_livery;
#[cfg(feature = "view")]
pub use campaign::load_event;
#[cfg(feature = "view")]
use environment::{cloud_layer, hd_sky_model, psp2_sky_model};
use oag_source::remix::craft_of;

/// A disc circuit as a track in this project's own format, opened the way
/// [`load`] opens its source. **Local use only**: the result is disc content.
///
/// # Errors
///
/// Whatever [`load`] would refuse the circuit for, and a circuit with no
/// `Start Position` node.
pub fn export_track(options: &Options) -> Result<oag_trackfile::Track> {
    let (packs, pure_packs, _) = oag_source::dlc::packs_from_defaults(
        &options.dlc,
        &oag_source::cache::default_dlc_cache_dir(),
    );
    let remix = oag_source::remix::Remix::open(&options.source, None, packs, pure_packs)?;
    let (opened, _, title, _) = remix.into_parts();
    let mut archives = opened.archives.memoising_reads();
    track_inputs::export(options, &mut archives, title)
}

/// Loads a track, a ship and its handling out of a disc image.
///
/// # Errors
///
/// Propagates a missing archive, a missing entry, a `.vex` that does not decode,
/// a `WO Track` payload the spline parser cannot account for, and a
/// `handlingstats.xml` with an attribute missing.
pub fn load(options: &Options) -> Result<Loaded> {
    let mut report = Vec::new();

    // Which archives this source has, rather than which archives a PSP disc has.
    // Every entry name below is the same on both releases - the PS2 build ships
    // `Data\Ships\<Team>\handlingstats.xml` and `Data\Environments\<n>_Track\...`
    // under the names the PSP uses, so only the layout differs (`docs/formats/handling-stats.md`).
    // Downloadable content is mounted behind them. A pack is not tied to the
    // release it was sold for here, so this is the same call whichever image
    // `source` names - see `docs/formats/dlc-pack.md`. **Which of the two lists
    // actually gets mounted** is `oag_source::title::open_source`'s decision, not this
    // call site's - see [`oag_source::dlc::pure_packs`] for why a Pure source needs
    // its own list rather than reusing Pulse's.
    let (packs, pure_packs, problems) = oag_source::dlc::packs_from_defaults(
        &options.dlc,
        &oag_source::cache::default_dlc_cache_dir(),
    );
    // **Opened as whichever title the source turned out to be**, the same way the
    // boot path already does it. This used to be `oag_pulse::open_with_packs`
    // outright, which refused a Pure disc by name (`WrongTitle`) - so `--race`
    // and the menus' own `Launch Game` could not reach a second title at all,
    // however much of the rest of the path was ready for one.
    //
    // **Two sources when `craft_source` names a Race Remix** - see
    // `oag_source::remix::Remix`. `archives`/`title` stay the track's; a
    // craft-governed load reads `craft_title` and `craft_of(&mut craft, &mut archives)` instead.
    let remix = if options.server_source {
        oag_source::remix::Remix::open_server(&options.source, packs, pure_packs)?
    } else {
        oag_source::remix::Remix::open(
            &options.source,
            options.craft_source.as_deref(),
            packs,
            pure_packs,
        )?
    };
    let (opened, craft_opened, title, craft_title) = remix.into_parts();
    let mut archives = opened.archives.memoising_reads();
    let mut craft = craft_opened.map(|opened| opened.archives.memoising_reads());
    report.push(format!("racing on {}", title.name));
    if craft.is_some() {
        report.push(format!("craft from {}", craft_title.name));
    }
    report.push(archives.layout.describe());
    for pack in &archives.packs {
        report.push(format!("dlc: {}", pack.label()));
    }
    report.extend(problems.into_iter().map(|p| format!("dlc: {p}")));

    // `label`, `track_classes` and `nodes` build only what is drawn.
    #[cfg_attr(not(feature = "view"), allow(unused_variables))]
    let track_inputs::Inputs {
        track,
        ai,
        label,
        track_blob,
        track_classes,
        start_position,
        nodes,
        collision,
        speedup_pads,
        weapon_pads,
        #[cfg(feature = "view")]
        mut drawn,
    } = track_inputs::resolve(options, &mut archives, title, &mut report)?;
    let read = |archives: &mut oag_assets::Archives, name: &str| -> Result<Vec<u8>> {
        archives
            .read_name(name)
            .with_context(|| format!("reading {name} out of {}", archives.layout.describe()))
    };

    // **The team this source spells that way**, resolved here for the same
    // reason the circuit above is: this is the first point the title is known.
    // See `roster::resolve_team`.
    let team = roster::resolve_team(options.team.as_deref(), craft_title, &mut report);

    // The player's own alternate hull file, resolved where `team` is: `variant::resolve`.
    #[cfg(feature = "view")]
    let hull_variant = variant::resolve(options.hull_variant.as_deref(), craft_title, &mut report);

    let stats_name = handling::entry_name_in(craft_title.race.handling_dir_for(&team), &team);
    let stats_blob = read(craft_of(&mut craft, &mut archives), &stats_name)?;
    let stats =
        handling::from_blob(&stats_blob).map_err(|e| anyhow::anyhow!("{stats_name}: {e}"))?;
    // **A borrowed pitch response is never silent.** Pure authors no `<pitch>` in
    // any `<Class>`, so `handling_for` substitutes
    // `oag_gameplay::handling::PITCH_STAND_IN` - another title's tuning on a ship
    // whose own is unknown. That is exactly the kind of substitution that looks
    // like a physics bug three sessions later if nobody wrote it down, so it is
    // named here with the classes it applied to.
    let unauthored: Vec<&str> = stats
        .classes
        .iter()
        .filter(|class| class.pitch.is_none())
        .map(|class| class.raw_name.as_str())
        .collect();
    if !unauthored.is_empty() {
        report.push(format!(
            "{stats_name}: {} of {} class(es) author no <pitch> ({}); flying on \
             oag_gameplay::handling::PITCH_STAND_IN, which is Pulse's block and \
             not this title's",
            unauthored.len(),
            stats.classes.len(),
            unauthored.join(", ")
        ));
    }
    // The engine-wide block, out of `Data\XML\HandlingStats.xml` rather than this
    // team's file - see `handling::GLOBAL_ENTRY`. Read on **every** mode now, not
    // only Zone: the speed-pad tunables live here too and every mode has pads.
    // The engine-wide `<Global>` block and the four numbers this race's rung
    // takes out of it. Resolved by **name**, so a title whose ladder is not
    // Pulse's reaches its own block - see `global::resolve`.
    let global::GlobalTunables {
        pad_tunables,
        special,
        class_gravity_scale,
        start_boost,
        zone,
        weapon_pad_refresh,
        pilot_assist,
    } = global::resolve(&mut archives, options, &stats, &mut report);
    // Nothing is scaled here: the four pre-scaled fields are converted exactly once and this
    // is not the place it happens, and neither `<SpeedupPads>` nor `<Special>` is one of them.
    // `None` where this team's file does not author the requested rung: a race
    // that cannot be set up, rather than one run on somebody else's tuning.
    let handling =
        handling_for(&stats, &options.class, pad_tunables, special).with_context(|| {
            let (class, ladder) = (&options.class, stats.ladder());
            format!("{stats_name} authors no <Class name=\"{class}\"> - it carries {ladder}")
        })?;
    // `<AirbrakeGraphics>` is the fifth pre-scaled field and it *is* converted
    // here, by its own function: it is per team rather than per class, and it
    // is graphics, so it deliberately never enters `Handling`.
    #[cfg(feature = "view")]
    let airbrake_graphics = oag_gameplay::airbrake_graphics_for(&stats);
    // The weapon table. Read on every mode even though only a single race arms
    // the pads: which mode is racing is a gameplay question and this is the
    // asset half, and reading it unconditionally is what makes a broken file a
    // reported line on every run rather than one nobody sees until they pick
    // the one mode that needs it.
    //
    // Two failures, two lines, for the reason `<Global>` above gives at length.
    let weapons = assets::load_weapons(&mut archives, title, options.mode, &mut report);
    let weapon_ai = assets::load_weapon_ai(&mut archives, title, &mut report);
    if options.weapons_on() && weapons.is_none() {
        // Only worth saying on a mode that would otherwise hand something out.
        report.push("weapon pads hand nothing out this run".to_string());
    }
    #[cfg(feature = "view")]
    let (chase, chase_close, internal) =
        cameras::resolve(&stats, &stats_name, options, &handling, &team, &mut report);
    // The lock-on's rig, which is all of the cameras the simulation reads.
    #[cfg(not(feature = "view"))]
    let chase = chase_params(stats.external_camera_far);

    // The grid's roster - see `roster::available`.
    let available = roster::available(
        craft_of(&mut craft, &mut archives),
        craft_title,
        &options.opponent_teams,
        &mut report,
    );
    #[cfg(feature = "view")]
    let roster::Grid {
        slot_teams,
        hd_trail_red,
        liveries,
    } = roster::grid(
        craft_of(&mut craft, &mut archives),
        craft_title,
        options,
        &team,
        &available,
        hull_variant,
        &mut report,
    )?;
    #[cfg(not(feature = "view"))]
    let slot_teams = roster::teams(craft_title, options, &team, &available, &mut report);
    // Each AI slot's own tuning where the title's variant family says it flies a
    // different team's file than the player's - see `family`.
    let opponent_handling = family::opponent_handling(
        craft_of(&mut craft, &mut archives),
        craft_title.race,
        &slot_teams,
        &options.class,
        pad_tunables,
        special,
        &mut report,
    );
    #[cfg(feature = "view")]
    let (shadows, shadow_hulls) = super::shadow::assets(
        craft_of(&mut craft, &mut archives),
        &slot_teams,
        craft_title.race,
        options.mode,
        &mut report,
    );
    #[cfg(feature = "view")]
    let wm = craft_title.weapon_models; // every weapon's own body, per title
    #[cfg(feature = "view")]
    let (
        rocket_model,
        mine_model,
        bomb_model,
        cannon_model,
        plasma_blast_models,
        bomb_blast_models,
        leach_ball_model,
    ) = weapon_models::load_bodies(&mut archives, wm, &mut report);
    // The cockpit half of the shield, on the same terms and for the same
    // reason: not per team, not per track, one entry for every craft in the
    // game. The shell beside it *is* per team and loads with the livery above.
    #[cfg(feature = "view")]
    let shield_cockpit =
        oag_livery::cockpit_shield(craft_of(&mut craft, &mut archives), &mut report);

    // Shared with the sky and the pads below: all three are node classes inside
    // the same track file, and a material in any of them names a texture by its
    // ordinal among *all* of the file's `Texture` nodes, not a per-class one -
    // see `mesh::build_sky`'s doc comment. Resolved once here, against the track
    // model's own embedded textures, and reused rather than re-read from the
    // archive and re-decoded per model.
    #[cfg(feature = "view")]
    let mut ps2_track_textures: Option<mesh::Ps2TextureSet> = None;

    // **A PS3 circuit's geometry is in the `.rcsmodel` beside it.** Both the
    // meshes its `.vex` places - which on HD are all props - and the far larger
    // set of chunks no node references, which is the road itself; see
    // `oag_mesh::mesh::rcs::build_scene`. A source with no sibling falls back
    // to the derived ribbon, which draws the circuit this project can derive
    // rather than nothing at all, and says so, so nobody mistakes an invented
    // surface for the disc's art.
    //
    // **Two spellings are tried, in this order: `track.rcsmodel`, then the
    // Omega Collection's `track.final.rcsmodel`.** No circuit ships both (see
    // `mesh::rcs::sibling_name_cooked`), so the order only decides which title's
    // lookup pays a second, failing read - every other title's pays none.
    #[cfg(feature = "view")]
    let (ps3_geometry, geometry_name) =
        if oag_vex::vex::geometry_is_external(&track_blob) && !options.ribbon {
            geometry::sibling_model(
                &mut archives,
                &track,
                options.mode == Mode::Zone && options.zone_model,
                &mut report,
            )
        } else {
            (None, None)
        };
    // **Built here rather than below, because whether it built decides the
    // fallback.** A `.rcsmodel` that will not decode is the same outcome for a
    // race as none at all - the derived ribbon - and Wipeout 2048 made the
    // distinction matter: it ships one beside every circuit, in a container
    // that is not HD's, and that used to stop the race. The report says which.
    #[cfg(feature = "view")]
    let (rcs_model, behind_glass) = geometry::track_model(
        &mut archives,
        &track,
        &track_blob,
        &ps3_geometry,
        geometry_name.as_deref(),
        &mut report,
    );
    #[cfg(feature = "view")]
    let ribbon = options.ribbon
        || drawn.is_some()
        || (oag_vex::vex::geometry_is_external(&track_blob) && rcs_model.is_none());
    // **Everything else built out of the track `.vex` is PSP-shaped geometry.**
    // The sky and the per-node visibility read a `Mesh`-layout payload, which a
    // PS3 file does not have - its `Skycube` is in the `.rcsmodel` too, keyed
    // by a hash this project has not tied back to that class. So they are
    // skipped on a PS3 source for the same reason they are skipped on a
    // ribbon build, and reported the same way: absent, not invented.
    #[cfg(feature = "view")]
    let vex_geometry = !ribbon && rcs_model.is_none();
    // Whether the `.rcsmodel` *built*, not merely whether one was found - the
    // report below says which surface is on screen, and since a sibling that
    // will not decode now falls back to the ribbon, "found" is the wrong
    // question to ask.
    #[cfg(feature = "view")]
    let rcs_drawn = rcs_model.is_some();
    // Both pad classes, decoded once and shared between the two blocks below -
    // mirrors `sky_model`'s own HD/Wipeout 2048 split just past it, since only
    // HD's node tree has a recovered `Speedup Pad`/`Weapon Pad` class id;
    // Wipeout 2048's `.rcsmodel` carries no comparable node tree yet, so a
    // 2048 source stays `(None, None)` here exactly as it did before this
    // pair existed.
    #[cfg(feature = "view")]
    let (ps3_pad_model, ps3_weapon_pad_model) = match &ps3_geometry {
        Some(geometry) if rcs_drawn && !mesh::rcs::psp2::is_psp2(geometry) => {
            pads::ps3_pad_models(&mut archives, &track, &track_blob, geometry, &mut report)?
        }
        _ => (None, None),
    };

    #[cfg(feature = "view")]
    let mut track_model = if let Some(model) = rcs_model {
        model
    } else if ribbon {
        drawn
            .take()
            .unwrap_or_else(|| track_render::build_model(&label, &ai))
    } else {
        let mut track_model = mesh::build_with_textures(&track, &track_blob, None)?;
        // Same PS2 signature and the same directory-position heuristic as the
        // ship above. Checked separately for tracks specifically (not just
        // assumed from the ship result): the entry directly before a track's
        // own `.vex` decodes as a texture set with the same entry count as the
        // model's `Texture` nodes on 27 of the 32 `<n>_Track`/`track_reversed`
        // pairs on the PS2 disc, off by only 1-2 slots (never more) on the
        // rest - see `docs/formats/ps2-texture.md`.
        if !track_model.textures.is_empty()
            && track_model.textures.iter().all(Option::is_none)
            && let Some(external) = ps2_texture_set(&mut archives, &track)
        {
            track_model = mesh::build_with_textures(&track, &track_blob, Some(&external))?;
            ps2_track_textures = Some(external);
        }
        track_model
    };
    // HD's zoom-streak ring on a title that draws none of its own: the player's
    // `graphics.boost_zoom`, honoured only where the title offers it. Chosen, not
    // measured - see `crate::zoom`.
    #[cfg(feature = "view")]
    let borrowed_zoom = options.boost_zoom.filter(|_| title.race.borrows_zoom_ring);
    // The circuit's staging, all four readers at once: its light rig, its
    // distance fog, its bloom block and the Zone stage grade laid over the
    // first two. Every one of them reports what it found and substitutes
    // nothing for what it did not - see `environment::staging`.
    #[cfg(feature = "view")]
    let environment::Staging {
        light,
        authored_fog,
        hd_bloom,
        omega_tonemap,
        mut zone_grade,
    } = environment::staging(
        &mut archives,
        &track,
        environment::GeometryKind::of(ps3_geometry.as_deref()),
        title,
        options.mode,
        options.zone_stage,
        &mut report,
    );
    #[cfg(feature = "view")]
    if zone_grade.is_none() && options.borrows_zone_look(title) {
        zone_grade = environment::borrowed_zone_grade(options, &track, &mut report);
    }
    #[cfg(feature = "view")]
    let spu_vertex_lights = engine_light::spu_vertex_lights_enabled(
        &mut archives,
        &track,
        ps3_geometry.is_some(),
        &mut report,
    );
    // The track's authored fog volumes. Empty for a ribbon build, and empty for
    // the four circuits that author no `fogCube` at all - both ordinary, and
    // both meaning the race renders unfogged.
    #[cfg(feature = "view")]
    let fog_volumes = if !vex_geometry {
        Vec::new()
    } else {
        let nodes = oag_vex::vex::nodes(&track_blob).unwrap_or_default();
        let volumes = oag_vex::fog::volumes(&track_blob, &nodes);
        report.push(match volumes.first() {
            None => "the track authors no fogCube; the race is unfogged".to_string(),
            Some(v) => format!(
                "fog: {} volume(s), {:.0} units across, {:.0}..{:.0} at the near end",
                volumes.len(),
                v.edge,
                v.near_end.near,
                v.near_end.far,
            ),
        });
        volumes
    };

    // The ribbon build draws an invented surface rather than the disc's art, so
    // it gets no sky either: the two belong to the same "show what shipped"
    // mode.
    #[cfg(feature = "view")]
    let sky_model = if let Some(geometry) = &ps3_geometry {
        // Two containers wear this extension, and their skies are unrelated
        // formats - see `geometry::track_model`'s own dispatch on the same
        // check. Wipeout HD authors no `Skycube` node at all: its sky is the
        // `sky.gtf` cubemap beside the track, drawn through the same
        // camera-centred sky path, and a Zone race swaps that file out (see
        // `mesh::sky_cube` and `oag_hd::race::ZONE_SKY`). Wipeout 2048's is
        // `skycube.rcsmodel`, an authored dome in the same `.rcsmodel`
        // container the track and craft already read - see
        // `environment::psp2_sky_model`.
        if mesh::rcs::psp2::is_psp2(geometry) {
            psp2_sky_model(&mut archives, &track, &mut report)
        } else {
            hd_sky_model(&mut archives, &track, title.race, options.mode, &mut report)
        }
    } else if !vex_geometry {
        None
    } else {
        let sky = mesh::build_sky(&track, &track_blob, ps2_track_textures.as_ref())?;
        if sky.indices.is_empty() {
            report.push(unrecovered_or_absent(
                track_classes,
                track_classes.skycube,
                "Skycube",
                "the sky stays black",
            ));
            None
        } else {
            report.push(format!(
                "drawing the track's sky: {} triangle(s), {} material(s)",
                sky.indices.len() / 3,
                sky.draws.len() + sky.alpha_tested_draws.len() + sky.transparent_draws.len(),
            ));
            Some(sky)
        }
    };

    // Same reasoning as the sky: a ribbon build shows an invented surface, so it
    // shows none of the disc's art meshes, pads included. The PS3 path is
    // `ps3_pad_model`, decoded above beside `ps3_weapon_pad_model` - see
    // `pads::ps3_pad_models`' own doc comment for why HD needs a pass of its
    // own rather than reusing `mesh::build_pads`' PSP-shaped one.
    #[cfg(feature = "view")]
    let pad_model = if ps3_pad_model.is_some() {
        ps3_pad_model
    } else if !vex_geometry {
        None
    } else {
        let pads = mesh::build_pads(&track, &track_blob, ps2_track_textures.as_ref())?;
        if pads.indices.is_empty() {
            report.push(unrecovered_or_absent(
                track_classes,
                track_classes.speedup_pad,
                "Speedup Pad",
                "no pad plates are drawn",
            ));
            None
        } else {
            report.push(format!(
                "drawing the track's speedup pads: {} triangle(s), {} material(s)",
                pads.indices.len() / 3,
                pads.draws.len() + pads.alpha_tested_draws.len() + pads.transparent_draws.len(),
            ));
            Some(pads)
        }
    };
    // The pickup pads, kept separate for the reason `mesh::build_weapon_pads`
    // gives: a merged buffer could not show one gameplay object without the
    // other. **Nothing hands anything out yet** - see the roadmap.
    // **Decoded here regardless of `options.mode`.** Whether it actually reaches
    // the screen is a `Scene::new` decision, not this one - see its own comment
    // and `Mode::weapons_enabled`. Keeping the decode unconditional is what lets
    // `the_weapon_pads_are_drawn_where_they_trigger` check the geometry against
    // the trigger volumes on every mode's own default `Options`, and it mirrors
    // the original's own order: `World_CollectNodeLists` always walks the tree
    // before anything asks whether weapons are on. The PS3 path is
    // `geometry::weapon_pad_model` - see its own doc comment for why.
    #[cfg(feature = "view")]
    let weapon_pad_model = if rcs_drawn {
        geometry::weapon_pad_model(ps3_weapon_pad_model, options.weapons_on(), &mut report)
    } else if !vex_geometry {
        None
    } else {
        let pads = mesh::build_weapon_pads(&track, &track_blob, ps2_track_textures.as_ref())?;
        if pads.indices.is_empty() {
            report.push(unrecovered_or_absent(
                track_classes,
                track_classes.weapon_pad,
                "Weapon Pad",
                "no pickup plates are drawn",
            ));
            None
        } else {
            report.push(format!(
                "{} the track's weapon pads: {} triangle(s), {} material(s)",
                if options.weapons_on() {
                    "drawing"
                } else {
                    "decoded but not drawing (weapons off in this mode)"
                },
                pads.indices.len() / 3,
                pads.draws.len() + pads.alpha_tested_draws.len() + pads.transparent_draws.len(),
            ));
            Some(pads)
        }
    };
    #[cfg(feature = "view")]
    report.push(format!(
        "drawing the track's {}: {} triangle(s), radius {:.0}",
        match (vex_geometry, rcs_drawn) {
            (true, _) => "art meshes",
            (false, true) => "art meshes out of the .rcsmodel beside it",
            (false, false) => "driveable ribbon",
        },
        track_model.indices.len() / 3,
        track_model.radius
    ));

    #[cfg(feature = "view")]
    for model in [
        Some(&liveries[0].hull),
        Some(&track_model),
        sky_model.as_ref(),
        pad_model.as_ref(),
        weapon_pad_model.as_ref(),
    ]
    .into_iter()
    .flatten()
    {
        if let Some(line) = untextured_note(model) {
            report.push(line);
        }
    }

    #[cfg(feature = "view")]
    let (gantry, visibility, adverts) = gantry_visibility::build(
        &mut archives,
        &track,
        &mut track_model,
        &track_blob,
        &ai,
        ps3_geometry.as_deref(),
        geometry_name.as_deref(),
        vex_geometry,
        gantry_visibility::adverts_for(title, options.mode),
        gantry_visibility::gantry_card(title),
        options.mode,
        start_position.as_ref(),
        &mut report,
    );
    // What `vex_geometry` comes to above with nothing built: an `.rcsmodel` that
    // builds and one that does not both leave it false, so only the request,
    // a foreign track and the file's own export decide it. `pulse_psp` below
    // reads it, and through `track_stats` the finished thrust does.
    #[cfg(not(feature = "view"))]
    let vex_geometry = !options.ribbon
        && !track.starts_with("foreign:")
        && !oag_vex::vex::geometry_is_external(&track_blob);
    stages::reach(stages::Stage::TrackBuilt);

    let spline = Spline::from_track(&ai);
    report.push(format!(
        "{} spline sample(s), {} per segment, widest half-width {:.1}",
        spline.len(),
        Spline::STEPS_PER_SEGMENT,
        spline.max_half_width()
    ));

    #[cfg(feature = "view")]
    let collision_model = if options.collision {
        let soup = render_collision::build_model(
            &label,
            &nodes,
            render_collision::Style::Wireframe,
            false,
        );
        report.push(format!(
            "drawing the collision soup: {} triangle(s) as outlines",
            soup.indices.len() / 3
        ));
        Some(soup)
    } else {
        None
    };
    stages::reach(stages::Stage::WorldBuilt);
    // The player's own, and reported by `livery::load` beside the hull it was
    // read off. Sparks are slot 0's today - `oag_fx::sparks` triggers off
    // the player's contacts alone - so this takes slot 0's locators rather than
    // carrying eight sets nothing reads.
    #[cfg(feature = "view")]
    let collision_fx = liveries[0].collision_fx.clone();
    #[cfg(feature = "view")]
    let absorb_burst = super::absorb::absorb_burst_for(craft_title);
    #[cfg(feature = "view")]
    let absorb_anchors = super::absorb::anchors(absorb_burst, &liveries);
    // One loop, one report line each: an effect is a name in the craft title's
    // `Effects` table, resolved to its trigger once below. Pulse's PSP laws only - see `psys::Effect::without_extents`,
    // `without_pulse_psp_draw` and `oag_fx::flash` for what others keep.
    #[cfg(feature = "view")]
    let mut effects = psys::Library::new();
    let extents = pulse_psp::is_pulse_psp(title, &archives);
    #[cfg(feature = "view")]
    let particle_laws = title.looks.particle_laws.applies(archives.layout.platform);
    // The launch boost is measured on Pulse PSP only; Pulse PS2 and HD author the
    // same element and are left without it, not applied by extension.
    let start_boost = start_boost.filter(|_| extents);
    if !extents {
        report.push("<StartBoost>: not applied, only measured on Pulse PSP".to_string());
    }
    #[cfg(feature = "view")]
    for name in craft_title.effects.names_on(archives.layout.platform) {
        let gtf = craft_title.effects.sprites.contains(&name);
        let extent_law = craft_title.effects.extents.contains(&name);
        let dir = title.race.effect_dir_for(&track);
        match particle_effect(&mut archives, dir, name, particle_laws, extent_law, gtf) {
            Ok((effect, note)) => {
                report.push(note);
                effects.insert(name, effect);
                report.extend(super::assets::sheet_note(name, &effects));
            }
            Err(why) => report.push(format!("{why} - {name} will not be drawn")),
        }
    }
    #[cfg(feature = "view")]
    let handles = super::EffectHandles::resolve(craft_title, &effects);

    #[cfg(feature = "view")]
    let (sounds, announcer, class_announcer) = audio::banks_and_announcers(
        &mut archives,
        title.race,
        title.hud_art.lap_hud,
        options.mode,
        &slot_teams,
        &mut report,
    );
    #[cfg(feature = "view")]
    let track_emitters = audio::track_emitters(
        &mut archives,
        title.race.sounds,
        &track,
        &track_blob,
        &mut report,
    );

    // The ribbon's texture is a title axis, not a constant: Pulse and Pure name
    // one, HD authors a template whose material names its own - craft content,
    // hence `craft_title`. See `oag_title::exhaust::Exhaust`.
    #[cfg(feature = "view")]
    let (noise, trail_blend, trail_shape) = assets::trail_texture(
        craft_of(&mut craft, &mut archives),
        craft_title,
        &mut report,
    );
    #[cfg(feature = "view")]
    if trail_shape.is_some() {
        report.push(
            "the ribbon's geometry is HD's own, measured from the running game: a 54-sample \
             three-fin tube in the craft's frame, half-width 0.5, white-to-red vertex ramp, \
             alpha = intensity x speed ramp, u stretched by speed and scrolled by the flare's \
             wrapping phase (docs/ghidra/functions/ps3-hdfury-eu/engine-trail.md)"
                .into(),
        );
        report.push(
            "read and not drawn on HD: the flame spikes' per-shape random flicker \
             (0.65..0.85), the Fury afterburner's second boost blend, and the sprite \
             flare's chromatic fringe and occlusion query (its 4:1 quad, distance \
             fade, cos^32 highlight and alpha walk draw by the traced law, on every \
             craft but the viewing player's - exhaust::hd::Sprite). The flame \
             surface's Speed*time scroll draws now: time is engine shader parameter \
             slot 0, a global seconds clock (renderer.md)"
                .into(),
        );
        report.push(
            "a craft flying into a trail sparks: WO_TRAIL_HITSHIP, red variant on a \
             Fury skin, spawned on the intruder at the closest point of the ribbon. \
             The consumer, spawner and colour select are read; the geometric test and \
             the once-per-entry rate are this engine's - Race::advance_trail_hits \
             names both"
                .into(),
        );
    }

    // The flare is a title axis too, and a second one rather than a variant of
    // the ribbon's: Pulse and Pure name a sprite texture, HD authors a
    // per-team model - craft content too. See `oag_title::flare::Flare`.
    #[cfg(feature = "view")]
    let flare = assets::flare_texture(
        craft_of(&mut craft, &mut archives),
        craft_title,
        &mut report,
    );

    // The LeachBeam ribbon's own texture - see `assets::leach_beam_texture`.
    // A table that failed to load keeps asking, as it always did.
    #[cfg(feature = "view")]
    let leach_authored = weapons.as_ref().is_none_or(|w| w.leach_beam().is_some());
    #[cfg(feature = "view")]
    let leach_beam_texture = assets::leach_beam_texture(
        craft_of(&mut craft, &mut archives),
        wm.leachbeam_ball.is_some(),
        leach_authored,
        &mut report,
    );

    #[cfg(feature = "view")]
    let magstrip_wake = magstrip_wake::load(
        craft_of(&mut craft, &mut archives),
        wm,
        &liveries,
        &mut report,
    );

    // The plugin list is the craft's title's, and an empty one is a real
    // answer: a title whose front end is unrecovered declares no languages, so
    // the HUD draws its captions as their own `idstring` keys and says so, as
    // it already did for a source whose plugins would not parse.
    #[cfg(feature = "view")]
    let language_plugins = craft_title
        .front_end
        .map_or::<&[&str], _>(&[], |front_end| front_end.language_plugins);
    // **The HUD follows the craft**, art included - `load_hud` reads its
    // atlas, font and sight art out of whichever `Archives` it is given.
    // **`options.language`, not `None`** - see that field's own doc for the
    // bug this closes: every launch site now sets it from the live
    // `settings::Settings::language` before `load` ever runs.
    #[cfg(feature = "view")]
    let hud = load_hud(
        craft_of(&mut craft, &mut archives),
        craft_title,
        options.mode,
        language_plugins,
        options.language.as_deref(),
        options.hud_style.as_deref(),
        crate::hud::SplitViews {
            count: options.views,
            people: usize::from(options.people_flying()),
        },
        &mut report,
    );

    // The countdown's own `<Mode3D><Model>` mesh - see `load::countdown`.
    #[cfg(feature = "view")]
    let countdown_model = countdown::model(&hud, &mut archives, &mut report);

    // The ring the lap counter runs on. Reported either way: "this track has no
    // lap counting" is exactly the kind of thing that otherwise gets discovered
    // as a HUD that never counts past one.
    let course = Course::from_track(&ai, start_position.as_ref().map(|s| Vec3::from(s.position)));
    match &course {
        Some(course) => report.push(format!(
            "course: {} points over {:.0} units, start line at point {}, path \
             boundaries at {:?}",
            course.len(),
            course.length(),
            course.start_index(),
            course.path_boundaries()
        )),
        None => report.push(
            "course: the spline's primary chain does not close, so this track has no lap \
             counting"
                .to_string(),
        ),
    }
    let pose_override = pose::resolve(options.pose, &spline, &mut report);
    let pulse_psp = vex_geometry && pulse_psp::is_pulse_psp(title, &archives);
    #[cfg(feature = "view")]
    let circuit_fx = vex_geometry
        && title
            .looks
            .circuit_effects
            .applies(archives.layout.platform);
    #[cfg(feature = "view")]
    let road_ripple = vex_geometry && title.looks.road_ripple.applies(archives.layout.platform);
    #[cfg(feature = "view")]
    let pre_race = intro::rule(title, &archives).filter(|_| !ribbon);
    let pulse_ps2_data =
        vex_geometry && title.looks.ps2_glow_mask.applies(archives.layout.platform);
    let track_stats = track_stats::read(
        &options.source,
        &mut archives,
        &track,
        (pulse_psp, pulse_ps2_data),
        &mut report,
    );
    let original_laws =
        original_laws::resolve(&mut archives, title, &track, &options.class, &mut report);
    let finished_thrust = super::finished_thrust::read(
        craft_of(&mut craft, &mut archives),
        &super::finished_thrust::RaceTerms {
            class: &options.class,
            difficulty: options.difficulty,
            mode: options.mode,
            weapons_on: options.weapons_on(),
            full_grid: options.opponents || options.mode.has_opponents(),
            track_stats: track_stats.as_ref(),
        },
        &mut report,
    );
    let original_tables = super::original_tables::read(
        craft_of(&mut craft, &mut archives),
        title,
        &options.class,
        &track,
        original_laws,
        &mut report,
    );
    stages::reach(stages::Stage::CraftsBuilt);
    #[cfg(feature = "view")]
    let models = &craft_title.weapon_models;
    #[cfg_attr(not(feature = "view"), allow(unused_mut))]
    let mut loaded = Loaded {
        #[cfg(feature = "view")]
        started: None,
        title,
        platform: archives.layout.platform,
        setup: Setup {
            mode: options.mode,
            eliminator_kill_target: options.eliminator_kill_target,
            laps_override: options.laps_override,
            weapons_override: options.weapons_override,
            difficulty: options.difficulty,
            finished_thrust,
            original_tables,
            class: options.class.clone(),
            opponents: options.opponents,
            no_opponents: options.no_driver_for_class,
            people: options.people_flying(),
            #[cfg(feature = "view")]
            trail_sparks: options.trail_sparks,
            seed: options.seed.unwrap_or(SEED),
            zone,
            foreign: track.strip_prefix("foreign:").map(str::to_string),
            ai,
            spline,
            course,
            start_position,
            #[cfg(feature = "view")]
            hd_trail: trail_shape.is_some().then_some(hd_trail_red),
            #[cfg(feature = "view")]
            shield_palette: roster::shield_palette(
                craft_title,
                craft_of(&mut craft, &mut archives).layout.platform,
            ),
            #[cfg(feature = "view")]
            hd_plasma_blast: models.plasma_blast_hd.is_some(),
            #[cfg(feature = "view")]
            hd_bomb_blast: models.bomb_blast_hd.is_some(),
            #[cfg(feature = "view")]
            bomb_shockwave_fades: models.bomb_blast_pulse.is_some_and(|b| b.shockwave_fades),
            #[cfg(feature = "view")]
            hd_missile_blast: models.missile_blast_hd.is_some(),
            #[cfg(feature = "view")]
            pulse_laid_pose: craft_title.looks.laid_pose.applies_everywhere(),
            #[cfg(feature = "view")]
            laid_pose_scaled: craft_title.looks.laid_pose_scaled.applies_everywhere(),
            laid_from_rear: craft_title.looks.laid_from_rear.applies_everywhere(),
            grid_frame_from_sample: extents,
            #[cfg(feature = "view")]
            screen_flash: extents,
            #[cfg(feature = "view")]
            quake_flash: craft_title.effects.quake_look.flash,
            #[cfg(feature = "view")]
            absorb_burst,
            #[cfg(feature = "view")]
            absorb_anchors,
            #[cfg(feature = "view")]
            hit_spark_anchors: super::hit_sparks::anchors(craft_title, &liveries),
            #[cfg(feature = "view")]
            weapon_spark_anchors: super::hit_sparks::weapon_anchors(craft_title, &liveries),
            #[cfg(feature = "view")]
            wreck_anchors: super::wreck_fx::anchors(craft_title, &liveries),
            #[cfg(feature = "view")]
            destroy_stations: super::destroy_camera::stations(circuit_fx, &track_blob, &mut report),
            #[cfg(feature = "view")]
            intro_camera: intro::read(&mut archives, &track, pre_race, &mut report),
            slot_teams: slot_teams.clone(),
            #[cfg(feature = "view")]
            sight: super::Sighting::of(&hud),
            #[cfg(not(feature = "view"))]
            sight: super::Sighting::without_view(
                craft_of(&mut craft, &mut archives).layout.platform,
                craft_title.hud_art,
            ),
            #[cfg(feature = "view")]
            magstrip_wake: magstrip_wake.anchors,
            #[cfg(feature = "view")]
            magstrip_pob: wm.magstrip_pob,
            #[cfg(feature = "view")]
            leach_strip: magstrip_wake.leach_anchors,
            collision,
            handling,
            opponent_handling,
            #[cfg(feature = "view")]
            airbrake_graphics,
            chase,
            #[cfg(feature = "view")]
            chase_close,
            #[cfg(feature = "view")]
            internal,
            #[cfg(feature = "view")]
            nozzles: liveries.iter().map(|livery| livery.nozzle).collect(),
            #[cfg(feature = "view")]
            engine_lights: liveries.iter().map(|livery| livery.engine_light).collect(),
            #[cfg(feature = "view")]
            spu_vertex_lights,
            // Positions only: this list feeds the trail-hit sparks' nearest-of search
            // (`RaceView::spark_anchor_of`), which has no live hull orientation to aim along today
            // - see `collision_fx` below for the field that does.
            #[cfg(feature = "view")]
            spark_anchors: liveries
                .iter()
                .map(|livery| livery.collision_fx.iter().map(|a| a.position).collect())
                .collect(),
            #[cfg(feature = "view")]
            collision_fx,
            #[cfg(feature = "view")]
            scenery_fx: Default::default(),
            #[cfg(feature = "view")]
            effects,
            #[cfg(feature = "view")]
            handles,
            #[cfg(feature = "view")]
            sounds,
            #[cfg(feature = "view")]
            track_emitters,
            #[cfg(feature = "view")]
            announcer,
            #[cfg(feature = "view")]
            class_announcer,
            zone_stages: title.race.zone_stages,
            countdown_voice: title.race.countdown_voice.is_some(),
            launch_hover: title.race.launch_hover,
            hover_rig: title.race.hover_rig,
            craft_laws: title.race.craft_laws,
            #[cfg(feature = "view")]
            zoom_ring: title.race.zoom_ring.or(borrowed_zoom),
            #[cfg(feature = "view")]
            zoom_borrowed: borrowed_zoom.is_some(),
            weapon_pad_glow: title.weapon_pad_glow,
            speedup_pads,
            weapon_pads,
            weapons,
            weapon_ai,
            // No caller of the general `load` sets this - only `race::load::campaign::load_event`
            // does, on the `Loaded` this returns, after this whole function has already run.
            // See `Setup::allowed_weapons`'s own doc comment.
            allowed_weapons: Vec::new(),
            weapon_pad_refresh,
            class_gravity_scale,
            start_boost,
            pilot_assist,
            pose_override,
            #[cfg(feature = "view")]
            camera_override: options.camera,
        },
        #[cfg(feature = "view")]
        hud,
        #[cfg(feature = "view")]
        track_panel: intro::read_panel(
            &mut archives,
            title,
            &track,
            pre_race,
            options.language.as_deref(),
            &mut report,
        ),
        #[cfg(feature = "view")]
        track_stats,
        #[cfg(feature = "view")]
        track_model,
        #[cfg(feature = "view")]
        behind_glass,
        #[cfg(feature = "view")]
        billboards: crate::adverts::Billboards { gantry, adverts },
        #[cfg(feature = "view")]
        collision_model,
        #[cfg(feature = "view")]
        sky_model,
        #[cfg(feature = "view")]
        pad_model,
        #[cfg(feature = "view")]
        weapon_pad_model,
        #[cfg(feature = "view")]
        fog_volumes,
        #[cfg(feature = "view")]
        clouds: cloud_layer(&mut archives, &track_blob, vex_geometry, &mut report),
        #[cfg(feature = "view")]
        light,
        #[cfg(feature = "view")]
        authored_fog,
        #[cfg(feature = "view")]
        hd_bloom,
        #[cfg(feature = "view")]
        omega_tonemap,
        #[cfg(feature = "view")]
        zone_grade,
        #[cfg(feature = "view")]
        liveries,
        #[cfg(feature = "view")]
        rocket_model,
        #[cfg(feature = "view")]
        cannon_model,
        #[cfg(feature = "view")]
        mine_model,
        #[cfg(feature = "view")]
        bomb_model,
        #[cfg(feature = "view")]
        plasma_blast_models,
        #[cfg(feature = "view")]
        bomb_blast_models,
        #[cfg(feature = "view")]
        leach_ball_model,
        #[cfg(feature = "view")]
        shield_cockpit,
        #[cfg(feature = "view")]
        countdown_model,
        #[cfg(feature = "view")]
        visibility,
        #[cfg(feature = "view")]
        flare,
        #[cfg(feature = "view")]
        leach_beam_texture,
        #[cfg(feature = "view")]
        ribbon_textures: magstrip_wake.textures,
        #[cfg(feature = "view")]
        noise,
        #[cfg(feature = "view")]
        trail_blend,
        #[cfg(feature = "view")]
        trail_shape,
        #[cfg(feature = "view")]
        cannon_quad_textures: weapon_models::cannon_quads(&mut archives, wm, &mut report),
        #[cfg(feature = "view")]
        ghost_static: weapon_models::load_ghost_static(&mut archives, wm, &mut report),
        #[cfg(feature = "view")]
        shadows,
        #[cfg(feature = "view")]
        shadow_hulls,
        // Overwritten by `load_event` on the one path that has an event
        // name to carry - see `Loaded::campaign_2048_event`'s own doc.
        #[cfg(feature = "view")]
        campaign_2048_event: None,
        #[cfg(feature = "view")]
        ripples: ripple::Ripples::default(),
        report,
    };
    #[cfg(feature = "view")]
    if !(pulse_psp && options.track_shine) {
        loaded.track_model.shine_draws.clear();
    }
    #[cfg(feature = "view")]
    pulse_psp::finish(
        &mut loaded,
        (pulse_psp, circuit_fx, road_ripple),
        &track_blob,
        (&mut archives, &track),
    );
    #[cfg(feature = "view")]
    pulse_ps2::finish(
        &mut loaded,
        vex_geometry && pulse_ps2::is_pulse_ps2(title, &archives),
    );
    closing_report::finish(&mut loaded.report, &mut archives, craft.as_mut());
    Ok(loaded)
}
