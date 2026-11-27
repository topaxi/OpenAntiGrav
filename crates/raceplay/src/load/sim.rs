//! The sim phase of a load: everything the simulation of a race is built from, read off
//! the source, and what the view phase reads next.
//!
//! Its steps run in the order a whole load runs them, so every archive read, report line
//! and seeded draw the simulation sees is the one it saw when a single pass built both
//! halves; where a view step stood between two of them, a [`Seam`] marks the place for
//! its report lines ([`super::seams`]).

use super::seams::{Seam, Seams};
use super::*;

/// The handling file a race flies in place of the team's, or `None` for the team's own.
///
/// Detonator reads its own file, tested before the Zone mask (`Team_LoadHandlingStats`,
/// `docs/ghidra/functions/ps3-hdfury-eu/detonator-hull.md`, "The handling file"); every other
/// mode on the mask flies a title's Zone file where it reads one (`oag_title::zone_handling`).
#[must_use]
pub fn mode_handling(
    race: &oag_title::RaceDefaults,
    mode: Mode,
) -> Option<oag_title::zone_handling::ZoneHandling> {
    if mode == Mode::Detonator {
        return race.detonator.and_then(|detonator| detonator.handling);
    }
    race.zone_handling.copied().filter(|_| mode.on_zone_mask())
}

/// What [`load_sim`] read: the [`SimSetup`] and the simulation modules, what names the
/// race, and what the view phase builds the picture from.
#[derive(Debug)]
pub struct SimLoad {
    /// The simulation's half of the race.
    pub setup: SimSetup,
    /// The simulation modules the load compiled ([`crate::extensions::prepare`]).
    pub modules: crate::extensions::Prepared,
    /// Which title the track's source is. See [`Loaded::title`].
    pub title: &'static oag_title::Title,
    /// Which title the craft are from: `title`, or [`Options::craft_source`]'s.
    pub craft_title: &'static oag_title::Title,
    /// The archive entry of the circuit. See [`Loaded::circuit`].
    pub circuit: String,
    /// The sim phase's report lines.
    pub report: Vec<String>,
    /// Where the view phase's report lines go among [`Self::report`].
    pub seams: Seams,
    /// The track's source, open, with the reads memoised.
    pub archives: oag_assets::Archives,
    /// The craft's source, where it is another title's.
    pub craft: Option<oag_assets::Archives>,
    /// The player's texture packs, engaged until the load ends.
    pub texture_pack: crate::texture_pack::Engaged,
    /// The track's `.vex`, as read. Empty for a foreign track.
    pub track_blob: Vec<u8>,
    /// The track's name as its file gives it.
    pub label: String,
    /// The track file's own class numbering.
    pub track_classes: oag_vex::vex::classes::Classes,
    /// The collision nodes the collision world was built from.
    pub nodes: Vec<oag_vex::collision::CollisionNode>,
    /// A foreign track's art section, as the file has it.
    pub foreign_art: Option<oag_trackfile::art::Art>,
    /// The team's `stats.xml`, parsed, and its entry name.
    pub stats: oag_tables::handling::Stats,
    pub stats_name: String,
    /// The player's team, resolved.
    pub team: String,
    /// The player's alternate hull and the circuit's own hull, resolved.
    pub hull_variant: Option<&'static str>,
    pub circuit_hull: Option<&'static str>,
    /// Whether the race flies a Zone file's classless block.
    pub zone_file_read: bool,
    /// The simulation modules' sources, whose assets the view reads.
    pub extension_sources: Vec<crate::extensions::Source>,
    /// The circuit's own `stats.xml`, for the HUD's `RECORD` readout.
    pub track_stats: Option<oag_tables::track_stats::TrackStats>,
    /// Whether the track's art is the `.vex`'s own PSP-shaped geometry.
    pub vex_geometry: bool,
    /// Whether the source is Pulse's PSP disc, whose extents laws apply.
    pub extents: bool,
}

/// Reads the simulation's half of a race: the [`SimSetup`], the simulation modules, and
/// what the view phase (`load_view`) needs next.
///
/// # Errors
///
/// Propagates a missing archive, a missing entry, a `.vex` that does not decode, a
/// `WO Track` payload the spline parser cannot account for, and a `handlingstats.xml`
/// with an attribute missing.
pub fn load_sim(options: &Options) -> Result<SimLoad> {
    let mut report = Vec::new();
    let mut seams = Seams::default();

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
    // **Opened as whichever title the source turned out to be**, the way the boot
    // path does, so `--race` and the menus' `Launch Game` reach every title.
    // **Two sources when `craft_source` names a second title** - see
    // `oag_source::pair::SourcePair`. `archives`/`title` stay the track's; a
    // craft-governed load reads `craft_title` and `craft_of(&mut craft, &mut archives)` instead.
    let pair = if options.server_source {
        oag_source::pair::SourcePair::open_server(&options.source, packs, pure_packs)?
    } else {
        oag_source::pair::SourcePair::open_in(
            &options.source,
            options.craft_source.as_deref(),
            packs,
            pure_packs,
            &options.layering(),
        )?
    };
    let (opened, craft_opened, title, craft_title) = pair.into_parts();
    let mut archives = opened.archives.memoising_reads();
    let mut craft = craft_opened.map(|opened| opened.archives.memoising_reads());
    report.push(format!("racing on {}", title.name));
    if craft.is_some() {
        report.push(format!("craft from {}", craft_title.name));
    }
    report.push(archives.layout.describe());
    refuse_a_mode_without_its_data(options.mode, title, &archives)?;
    let finder = options.find_texture_packs.as_ref();
    let texture_pack = crate::texture_pack::engage(finder, title, &archives, &mut report);
    for pack in &archives.packs {
        report.push(format!("dlc: {}", pack.label()));
    }
    report.extend(crate::loader_log::dlc_problems(problems));

    // `label`, `track_classes` and `nodes` build only what is drawn.
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
        props: track_props,
        art: foreign_art,
    } = track_inputs::resolve(options, &mut archives, title, craft_title, &mut report)?;
    seams.mark(Seam::TrackArt, &report);
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
    let hull_variant = variant::resolve(options.hull_variant.as_deref(), craft_title, &mut report);
    let circuit_hull =
        variant::circuit_hull(&mut archives, title, craft_title, &track, &mut report);

    let zone_handling = mode_handling(craft_title.race, options.mode);
    let stats_name = match zone_handling {
        Some(zone) => zone.entry.value.to_string(),
        None => handling::entry_name_in(craft_title.race.handling_dir_for(&team), &team),
    };
    let stats_blob = read(craft_of(&mut craft, &mut archives), &stats_name)?;
    let stats = match zone_handling {
        Some(zone) => {
            let unauthored = handling::Unauthored {
                landing_rebound: zone.landing_rebound.value,
                rebound_jump_time: zone.rebound_jump_time.value,
            };
            report.push(format!(
                "{}: flying {stats_name}'s classless block as {}, in place of {team}'s file",
                options.mode.name(),
                options.class
            ));
            handling::from_blob_classless(&stats_blob, &options.class, unauthored)
        }
        None => handling::from_blob(&stats_blob),
    }
    .map_err(|e| anyhow::anyhow!("{stats_name}: {e}"))?;
    let stats = zone_cameras::overlay(
        stats,
        craft_title,
        options.mode,
        &team,
        &mut archives,
        &mut report,
    );
    // **A borrowed pitch response is never silent.** Pure authors no `<pitch>` in any
    // `<Class>`, so `handling_for` substitutes `oag_gameplay::handling::PITCH_STAND_IN` -
    // another title's tuning on a ship whose own is unknown. That is exactly the kind of
    // substitution that looks like a physics bug three sessions later if nobody wrote it
    // down, so it is named here with the classes it applied to.
    let unauthored: Vec<&str> = stats
        .classes
        .iter()
        .filter(|class| class.pitch.is_none())
        .map(|class| class.raw_name.as_str())
        .collect();
    let code_pitch = craft_title
        .race
        .craft_laws
        .and_then(|laws| laws.code_pitch)
        .is_some();
    if !unauthored.is_empty() && code_pitch {
        report.push(format!(
            "{stats_name}: no <pitch> authored; flying on this title's own compiled-in pitch \
             (measured, docs/physics/pure-physics.md)"
        ));
    } else if !unauthored.is_empty() {
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
        wall_restitution,
    } = global::resolve(&mut archives, options, &stats, title, &mut report);
    // Nothing is scaled here: the four pre-scaled fields are converted exactly once and this
    // is not the place it happens, and neither `<SpeedupPads>` nor `<Special>` is one of them.
    // `None` where this team's file does not author the requested rung: a race
    // that cannot be set up, rather than one run on somebody else's tuning.
    let skill_level = family::skill_level(options, craft_title, &mut report);
    let mut handling = handling_for(&stats, &options.class, pad_tunables, special, skill_level)
        .with_context(|| {
            let (class, ladder) = (&options.class, stats.ladder());
            format!("{stats_name} authors no <Class name=\"{class}\"> - it carries {ladder}")
        })?;
    code_laws::apply(
        &mut handling,
        &stats,
        &options.class,
        craft_title.race.craft_laws,
    );
    // `<AirbrakeGraphics>` is the fifth pre-scaled field and it *is* converted
    // here, by its own function: it is per team rather than per class, and it
    // is graphics, so it deliberately never enters `Handling`.
    // The weapon table. Read on every mode even though only a single race arms
    // the pads: which mode is racing is a gameplay question and this is the
    // asset half, and reading it unconditionally is what makes a broken file a
    // reported line on every run rather than one nobody sees until they pick
    // the one mode that needs it.
    //
    // Two failures, two lines, for the reason `<Global>` above gives at length.
    let weapons = assets::load_weapons(&mut archives, title, options.mode, &mut report);
    let weapon_ai = assets::load_weapon_ai(&mut archives, title, &mut report);
    let detonator = assets::load_detonator_layout(&mut archives, title, options.mode, &mut report);
    let rule = crate::score_law::of(options, title, &weapon_ai, &mut archives, &mut report);
    if options.weapons_on() && weapons.is_none() {
        // Only worth saying on a mode that would otherwise hand something out.
        report.push("weapon pads hand nothing out this run".to_string());
    }
    seams.mark(Seam::Cameras, &report);
    // The lock-on's rig, which is all of the cameras the simulation reads, in the title's
    // view matrix: a shear where the original's camera node is one. The rig never draws, so
    // the shear reaches nothing it computes; it is set so the rig equals the drawn far view.
    let chase = {
        let sheared = craft_title
            .race
            .craft_laws
            .is_some_and(|laws| laws.chase_view_sheared.value);
        ChaseParams {
            view_sheared: sheared,
            ..crate::camera::chase_params_for(stats.external_camera_far, title, options.mode)
        }
    };

    stages::checkpoint()?;
    // The grid's roster - see `roster::available`.
    let available = roster::available(
        craft_of(&mut craft, &mut archives),
        craft_title,
        &options.opponent_teams,
        &mut report,
    );
    seams.mark(Seam::Looks, &report);
    let slot_teams = roster::teams(craft_title, options, &team, &available, &mut report);
    seams.mark(Seam::Liveries, &report);
    stages::checkpoint()?;
    // Each AI slot's own tuning where the title's variant family says it flies a
    // different team's file than the player's - see `family`.
    let (opponent_handling, opponent_weapon_counts) = family::opponent_handling(
        craft_of(&mut craft, &mut archives),
        craft_title.race,
        &slot_teams,
        &family::Shared {
            class: &options.class,
            pad_tunables,
            special,
            skill_level,
        },
        &mut report,
    );
    seams.mark(Seam::Scene, &report);

    // What `vex_geometry` comes to above with nothing built: an `.rcsmodel` that
    // builds and one that does not both leave it false, so only the request,
    // a foreign track and the file's own export decide it. `pulse_psp` below
    // reads it, and through `track_stats` the finished thrust does.
    let vex_geometry = !options.ribbon
        && !track.starts_with("foreign:")
        && !oag_vex::vex::geometry_is_external(&track_blob);

    let spline = Spline::from_track(&ai);
    report.push(format!(
        "{} spline sample(s), {} per segment, widest half-width {:.1}",
        spline.len(),
        Spline::STEPS_PER_SEGMENT,
        spline.max_half_width()
    ));
    // The collision cage, where the track's own header asks for it, its points carry
    // the bytes and the title runs a cage off that flag: every Pulse circuit; never
    // HD's Chenghou Project, whose header sets the flag but whose original runs no
    // cage (`CraftLaws::cage_from_track_header`); never a foreign or own-format
    // track, which has no header word to read (`crate::cage`).
    let header_flag = oag_vex::track::cage_flag_in_vex(&track_blob);
    let title_runs_it = title
        .race
        .craft_laws
        .is_none_or(|laws| laws.cage_from_track_header.value);
    let cage = oag_gameplay::cage::CageTrack::new(&ai, header_flag && title_runs_it);
    report.push(match &cage {
        Some(cage) => format!(
            "collision cage: on, {} control point(s)",
            cage.point_count()
        ),
        None if header_flag && !title_runs_it => format!(
            "collision cage: off, {} runs no cage off the track header's flag",
            title.name
        ),
        None => "collision cage: off (no header flag, or no cage bytes)".to_string(),
    });
    let spline = spline.with_cage(cage);
    seams.mark(Seam::CollisionModel, &report);
    // The mods' sources are found once, here, and read again by `prepare` below; their
    // own PNGs go into the HUD's sheet, so they are read and decoded first.
    let extension_sources =
        crate::extensions::sources_for_race(options, &title.slug(), &mut report);
    let extents = pulse_psp::is_pulse_psp(title, &archives);
    // The authored `<StartBoost>` is flown where the craft's title carries the rule, and the
    // report names whose law it is.
    let start_boost_origin = craft_title
        .looks
        .start_boost_origin(archives.layout.platform);
    let start_boost = start_boost.filter(|_| start_boost_origin.is_some());
    start_boost_report::say(start_boost_origin, start_boost.is_some(), &mut report);
    // A title that compiles its launch in (Pure) flies its own, measured on its own disc.
    let code_start_boost = craft_title
        .race
        .craft_laws
        .and_then(|laws| laws.code_start_boost)
        .map(|b| oag_physics::launch::StartBoost {
            window_start: b.value.window_start,
            window_end: b.value.window_end,
            stall_end: b.value.stall_end,
            overall_duration: b.value.overall_duration,
            stall_mul: b.value.stall_mul,
            normal_mul: b.value.normal_mul,
            boost_mul: b.value.boost_mul,
        });
    if code_start_boost.is_some() && start_boost.is_none() {
        report.push(
            "launch boost: this title's own, compiled in (measured, docs/physics/pure-physics.md)"
                .to_string(),
        );
    }
    let start_boost = start_boost.or(code_start_boost);
    seams.mark(Seam::Effects, &report);

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
    let pulse_ps2_data =
        vex_geometry && title.looks.ps2_glow_mask.applies(archives.layout.platform);
    let track_stats = track_stats::read(
        &options.source,
        &mut archives,
        &track,
        pulse_psp,
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
            // The AI's ambient skill reads the file on Pulse only: HD's and Omega's own AI
            // skill is measured without it, and the HUD's `RECORD` is the one other reader.
            track_stats: track_stats.as_ref().filter(|_| pulse_psp || pulse_ps2_data),
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
    let modules = crate::extensions::prepare(
        &extension_sources,
        &title.slug(),
        options.foreign_track.as_deref(),
        crate::extensions::Track {
            name: &track,
            props: &track_props,
            speedup_pads: &speedup_pads,
            weapon_pads: &weapon_pads,
            paths: &ai.paths,
            routes: course.as_ref().map_or(&[], |course| course.routes()),
        },
        options.language.as_deref(),
        &mut report,
    );
    seams.mark(Seam::Flyby, &report);
    let setup = SimSetup {
        mode: options.mode,
        eliminator_kill_target: rule.target,
        laps_override: crate::catalogue::launch_laps(title, &track, options),
        weapons_override: options.weapons_override,
        difficulty: options.difficulty,
        finished_thrust,
        original_tables,
        class: options.class.clone(),
        opponents: options.opponents,
        no_opponents: options.no_driver_for_class,
        people: options.people_flying(),
        seed: options.seed.unwrap_or(SEED),
        zone,
        foreign: track.strip_prefix("foreign:").map(str::to_string),
        ai,
        spline,
        course,
        start_position,
        laid_from_rear: craft_title.looks.laid_from_rear.applies_everywhere(),
        grid_frame_from_sample: extents,
        slot_teams: slot_teams.clone(),
        row_teams: slot_teams
            .iter()
            .map(|id| craft_title.team_key(id))
            .collect(),
        sight: super::Sighting::without_view(
            craft_of(&mut craft, &mut archives).layout.platform,
            craft_title,
            &track,
        ),
        collision,
        handling,
        opponent_handling,
        weapon_counts: oag_weapons::counts::WeaponCounts::from_stats(&stats),
        opponent_weapon_counts,
        chase,
        cannon_flash: muzzle::cannon_flash(
            craft_of(&mut craft, &mut archives),
            craft_title,
            slot_teams.first().map_or("", String::as_str),
            options,
            (hull_variant, circuit_hull),
            &mut report,
        ),
        muzzles: muzzle::per_slot(
            craft_of(&mut craft, &mut archives),
            craft_title,
            &slot_teams,
            options,
            (hull_variant, circuit_hull),
            &mut report,
        ),
        // Positions only: this list feeds the trail-hit sparks' nearest-of search
        // (`RaceView::spark_anchor_of`), which has no live hull orientation to aim along today
        // - see `collision_fx` below for the field that does.
        zone_stages: title.race.zone_stages,
        countdown_voice: title.race.countdown_voice.is_some(),
        countdown_beat: title
            .race
            .countdown_voice
            .is_some_and(|voice| voice.beat.is_some()),
        // A title whose grid hover skips Zone (HD's `Craft_Update`, modes 6, 13, 14) races Zone from the grid.
        launch_hover: title
            .race
            .launch_hover
            .filter(|hover| !options.mode.on_zone_mask() || hover.in_zone.value),
        // Zone flies the title's Zone hover where it has one (Pulse's `Ship_HoverFourCorner`).
        hover_rig: match title.race.zone_hover_rig {
            Some(zone) if options.mode.on_zone_mask() => Some(zone),
            _ => title.race.hover_rig,
        },
        craft_laws: title.race.craft_laws,
        autopilot_engine: title.race.autopilot_engine,
        opponent_autopilot: title
            .race
            .opponent_autopilot
            .flies(&options.class, options.difficulty.name()),
        destroyed: title.destroyed.value,
        weapon_pad_glow: title.weapon_pad_glow,
        leach_drain: title.weapons.leach_drain,
        blast_push: title.weapons.blast_push.value,
        leach_beams: title.weapons.leach_beams.value,
        quake_hit: title.weapons.quake_hit,
        autopilot_absorb_cancels: title.weapons.autopilot_absorb_cancels.value,
        autopilot_pad_grants: title.weapons.autopilot_pad_grants.value,
        launch_normal_is_track_up: title.weapons.launch_normal_is_track_up.value,
        floor_arms_steer_to_ride_point: title.weapons.floor_arms_steer_to_ride_point.value,
        floor_arms_ride_launch_normal: title.weapons.floor_arms_ride_launch_normal.value,
        plasma_release_at_display_scale: title.weapons.plasma_release_at_display_scale.value,
        rocket_launch_at_display_scale: title.weapons.rocket_launch_at_display_scale.value,
        own_craft_box_hides_track: title.weapons.own_craft_box_hides_track.value,
        surface_probe_lengths: title.weapons.surface_probe_lengths.value,
        rocket_open_air: title.weapons.rocket_open_air.value,
        reset_triggers: crate::respawn::reset_triggers_for(&title.reset_enables),
        contact_cues: title.contact_cues,
        opponent_fire: title.weapons.opponent_fire.value,
        score_law: rule.law,
        score_pace_terms: title.score_pace.value,
        flip_law: rule.flip,
        speedup_pads,
        weapon_pads,
        weapons,
        weapon_ai,
        detonator,
        detonator_pads: title.race.detonator.map(|d| d.pads),
        // A 2048 campaign event replaces this on the `Loaded` this returns. See
        // `SimSetup::allowed_weapons`'s own doc comment.
        allowed_weapons: title.pickup_roster.map_or_else(Vec::new, |roster| {
            roster
                .value
                .enabled(options.mode == oag_race::Mode::Eliminator)
                .to_vec()
        }),
        pickup_roster: title.pickup_roster.map(|roster| roster.value),
        weapon_pad_refresh,
        class_gravity_scale,
        start_boost,
        pilot_assist,
        wall_restitution,
        pose_override,
        field_poses: options.field.clone(),
        capture_zone: options.hud.as_ref().and_then(|hud| hud.zone),
    };
    seams.mark(Seam::Finish, &report);
    Ok(SimLoad {
        setup,
        modules,
        title,
        craft_title,
        circuit: track,
        report,
        seams,
        archives,
        craft,
        texture_pack,
        track_blob,
        label,
        track_classes,
        nodes,
        foreign_art,
        stats,
        stats_name,
        team,
        hull_variant,
        circuit_hull,
        zone_file_read: zone_handling.is_some(),
        extension_sources,
        track_stats,
        vex_geometry,
        extents,
    })
}

/// Refuses a race of `mode` on `title` when the data the mode loads of its own does not resolve in
/// `archives`: the rule the MODE row offers modes by (`oag_ui::menu::mode_offered`), applied to a
/// race asked for by flag, by a stale setting or by a campaign cell, so none races a half-built
/// mode. Detonator on Pulse is the case: its data is a mod's (ADR-0098).
fn refuse_a_mode_without_its_data(
    mode: oag_race::Mode,
    title: &oag_title::Title,
    archives: &oag_assets::Archives,
) -> Result<()> {
    if mode != oag_race::Mode::Detonator {
        return Ok(());
    }
    let named = title.detonator_data();
    let missing: Vec<&str> = named
        .iter()
        .copied()
        .filter(|entry| archives.locate(entry).is_none())
        .collect();
    if named.is_empty() || !missing.is_empty() {
        let why = if named.is_empty() {
            format!("{} has no Detonator", title.name)
        } else {
            format!(
                "Detonator on {} needs data no disc, DLC pack or enabled mod holds: {}",
                title.name,
                missing.join(", ")
            )
        };
        log::warn!("{why}");
        anyhow::bail!("{why}");
    }
    Ok(())
}
