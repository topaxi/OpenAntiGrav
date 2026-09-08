//! [`load`]: everything a race needs, read out of one disc image, and the report
//! of what came back.
//!
//! Split out of `race.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change. Its tests are
//! `race/tests/load.rs`.

use super::*;

mod audio;
mod cameras;
mod environment;
mod geometry;
mod global;
mod pads;
mod roster;
mod surfaces;
mod variant;
mod weapon_models;
use crate::remix::craft_of;
use environment::{hd_sky_model, psp2_sky_model};

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
    // under the names the PSP uses - so the layout is the whole of the
    // difference. See `docs/formats/handling-stats.md`.
    //
    // Downloadable content is mounted behind them. A pack is not tied to the
    // release it was sold for here, so this is the same call whichever image
    // `source` names - see `docs/formats/dlc-pack.md`. **Which of the two lists
    // actually gets mounted** is `crate::title::open_source`'s decision, not this
    // call site's - see [`crate::dlc::pure_packs`] for why a Pure source needs
    // its own list rather than reusing Pulse's.
    let (packs, pure_packs, problems) =
        crate::dlc::packs_from_defaults(&options.dlc, &crate::boot::default_dlc_cache_dir());
    // **Opened as whichever title the source turned out to be**, the same way the
    // boot path already does it. This used to be `oag_pulse::open_with_packs`
    // outright, which refused a Pure disc by name (`WrongTitle`) - so `--race`
    // and the menus' own `Launch Game` could not reach a second title at all,
    // however much of the rest of the path was ready for one.
    //
    // **Two sources when `craft_source` names a Race Remix** - see
    // `crate::remix::Remix`. `archives`/`title` stay the track's; a
    // craft-governed load reads `craft_title` and `craft_of(&mut
    // craft, &mut archives)` instead.
    let remix = crate::remix::Remix::open(
        &options.source,
        options.craft_source.as_deref(),
        packs,
        pure_packs,
    )?;
    let (opened, craft_opened, title, craft_title) = remix.into_parts();
    let mut archives = opened.archives;
    let mut craft = craft_opened.map(|opened| opened.archives);
    report.push(format!("racing on {}", title.name));
    if craft.is_some() {
        report.push(format!("craft from {}", craft_title.name));
    }
    report.push(archives.layout.describe());
    for pack in &archives.packs {
        report.push(format!("dlc: {}", pack.label()));
    }
    report.extend(problems.into_iter().map(|p| format!("dlc: {p}")));

    // **The circuit this source actually offers**, resolved here because this is
    // the first point the title is known - see [`Options::track`]. Reported
    // either way, so a run that took a default says which title's default it was
    // rather than leaving a reader to work it out from the path.
    let track = match &options.track {
        Some(asked) => asked.clone(),
        None => {
            report.push(format!(
                "no track named: {}'s own default, {}",
                title.name, title.race.track
            ));
            title.race.track.to_string()
        }
    };

    // **Zone flies a different environment, and the disc authors it.** A Zone
    // circuit ships its own meshes, lights, `fogCube` and one-material
    // `Skycube`, so the mode's whole look is a file to open rather than a filter
    // to apply - `CLAUDE.md`'s rule about not inventing what the assets already
    // carry, and the reason nothing here tints or desaturates anything.
    //
    // Which file, though, is a title fact: Pulse keeps it beside the race
    // circuit under a prefixed name, Pure and HD keep whole circuits of their
    // own. `ZoneCircuit` is that axis, measured on all three.
    let track = if options.mode == Mode::Zone {
        let zone = match &options.track {
            Some(_) => title.race.zone.variant_of(&track),
            None => title.race.zone.default_track(title.race.track),
        };
        if zone == track {
            report.push(format!("zone: racing {track} as named"));
        } else {
            report.push(format!("zone: {track} -> {zone}"));
        }
        zone
    } else {
        track
    };

    let spec = archives
        .locate(&track)
        .ok_or_else(|| assets::zone_circuit_miss(&track, options.mode, title, &archives))?
        .to_string();

    let (ai, label) = track_render::load(&spec, &track)?;
    report.push(format!(
        "{label}: {} path(s), {} junction(s), {} control point(s)",
        ai.paths.len(),
        ai.junctions.len(),
        ai.point_count()
    ));

    let read = |archives: &mut oag_assets::Archives, name: &str| -> Result<Vec<u8>> {
        archives
            .read_name(name)
            .with_context(|| format!("reading {name} out of {}", archives.layout.describe()))
    };

    // Read again for the collision nodes: `oag_render::track::load` takes an
    // archive rather than bytes, so this blob is decompressed twice. See the
    // wanted-change note in `docs/tools/oag-game.md`. On the PS2 that costs more
    // than it does on the PSP, where nothing is compressed at all: 5,861 of
    // `WADS2.WAD`'s 7,200 entries are LZSS.
    let track_blob = read(&mut archives, &track)?;
    // **This track file's own class numbering.** Read once, off its version word,
    // and consulted everywhere below that used to spell a `vex::CLASS_*`
    // constant - all of which are version 6's. A version-4 track (Pure's are)
    // uses different numbers entirely, so the constants would find nothing here
    // or, worse, find the wrong node type. An id this project has not recovered
    // for this version comes back `None`, and each site below says what it does
    // with that rather than substituting version 6's answer. See
    // `oag_formats::vex::classes`.
    let track_classes =
        oag_formats::vex::classes_of(&track_blob).map_err(|e| anyhow::anyhow!("{}: {e}", track))?;
    let speedup_pad_class = track_classes.speedup_pad;
    let weapon_pad_class = track_classes.weapon_pad;
    let start_position = start_position_of(&track_blob);
    match start_position {
        Some(slot) => {
            report.push(format!(
                "Start Position: {:?} facing {:?}",
                slot.position, slot.forward
            ));
            // Slot 8 (this node) is measured for a full grid; a solo mode
            // instead uses this project's own reading of slot 1 - see
            // `Race::start`.
            if !(options.mode.has_opponents() || options.opponents) {
                report.push(
                    "solo mode: player on grid slot 1 (front), this project's own reading, \
                     not a separate measurement"
                        .to_string(),
                );
            }
        }
        None => report.push("no Start Position node: spawning on the spline instead".to_string()),
    }
    let nodes = surfaces::of_track(&mut archives, &track, &track_blob, &mut report)?;
    let collision = collision_world(&nodes);
    report.push(format!(
        "{} collision node(s) -> {} collider(s), {} triangle(s)",
        nodes.len(),
        collision.colliders().len(),
        collision
            .colliders()
            .iter()
            .map(oag_physics::TriangleSoup::triangle_count)
            .sum::<usize>()
    ));

    // The pads' trigger volumes, from the same blob the geometry comes from.
    // Reported with the distance from the grid to the nearest one, because that
    // is the number anyone testing a pad needs and there is nowhere else to get
    // it: a pad is a plate on the track surface with nothing to distinguish it
    // in a screenshot.
    let speedup_pads = oag_formats::vex::nodes(&track_blob)
        .map(|nodes| {
            speedup_pad_class
                .map(|class| oag_formats::pads::volumes(&track_blob, &nodes, class))
                .unwrap_or_default()
        })
        .unwrap_or_default();
    if speedup_pads.is_empty() {
        report.push("the track authors no Speedup Pad trigger volumes".to_string());
    } else {
        let nearest = start_position.map(|slot| {
            speedup_pads
                .iter()
                .map(|pad| Vec3::from_array(slot.position).distance(Vec3::from_array(pad.centre())))
                .fold(f32::INFINITY, f32::min)
        });
        report.push(match nearest {
            Some(d) => format!(
                "{} speedup pad trigger volume(s), nearest {d:.0} units from the grid",
                speedup_pads.len()
            ),
            None => format!("{} speedup pad trigger volume(s)", speedup_pads.len()),
        });
    }
    // The pickup pads, decoded the same way and **unconditionally**, unlike the
    // mesh below: nothing triggers them yet, so this is asset data rather than
    // gameplay, and `the_weapon_pads_are_drawn_where_they_trigger` checks the
    // decode against the drawn geometry regardless of mode. The original
    // decodes them unconditionally too - `World_CollectNodeLists` always walks
    // the tree - it only suppresses what a *weapons-off* mode does with the
    // result afterward, which is what `weapon_pad_model`'s own report line
    // below says plainly rather than repeating here.
    let weapon_pads = oag_formats::vex::nodes(&track_blob)
        .map(|nodes| {
            weapon_pad_class
                .map(|class| oag_formats::pads::volumes(&track_blob, &nodes, class))
                .unwrap_or_default()
        })
        .unwrap_or_default();
    report.push(if weapon_pads.is_empty() {
        "the track authors no Weapon Pad trigger volumes".to_string()
    } else {
        format!("{} weapon pad trigger volume(s)", weapon_pads.len())
    });

    // **The team this source spells that way**, resolved here for the same
    // reason the circuit above is: this is the first point the title is known.
    // See `roster::resolve_team`.
    let team = roster::resolve_team(options.team.as_deref(), craft_title, &mut report);

    // The player's own alternate hull file, resolved the same place `team`
    // is and for the same reason. See `variant::resolve`.
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
        zone,
        weapon_pad_refresh,
    } = global::resolve(&mut archives, options, &mut report);
    // Nothing is scaled here: the four pre-scaled fields are converted exactly once
    // and this is not the place it happens, and neither `<SpeedupPads>` nor
    // `<Special>` is one of them.
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
    let airbrake_graphics = oag_gameplay::airbrake_graphics_for(&stats);
    // The weapon table. Read on every mode even though only a single race arms
    // the pads: which mode is racing is a gameplay question and this is the
    // asset half, and reading it unconditionally is what makes a broken file a
    // reported line on every run rather than one nobody sees until they pick
    // the one mode that needs it.
    //
    // Two failures, two lines, for the reason `<Global>` above gives at length.
    let weapons = assets::load_weapons(&mut archives, title, &mut report);
    if options.mode.weapons_enabled() && weapons.is_none() {
        // Only worth saying on a mode that would otherwise hand something out.
        report.push("weapon pads hand nothing out this run".to_string());
    }
    let (chase, chase_close, internal) =
        cameras::resolve(&stats, &stats_name, options, &handling, &team, &mut report);

    // The grid's roster - see `roster::available`.
    let available = roster::available(
        craft_of(&mut craft, &mut archives),
        craft_title,
        &options.opponent_teams,
        &mut report,
    );
    let slot_teams = livery::teams_for_slots(&team, &available, oag_gameplay::MAX_SHIPS);
    let hd_trail_red = roster::hd_trail_red(&slot_teams);
    let liveries = livery::load(
        craft_of(&mut craft, &mut archives),
        &slot_teams,
        &livery::LoadContext {
            race: craft_title.race,
            mode: options.mode,
            flare: craft_title.flare,
            lod: options.lod,
        },
        hull_variant,
        &mut report,
    )?;
    report.push(format!(
        "grid liveries: {} - which team flies which slot is this project's, not \
         the original's (livery.rs)",
        slot_teams.join(", ")
    ));
    let (shadows, shadow_hulls) = super::shadow::assets(
        craft_of(&mut craft, &mut archives),
        &slot_teams,
        craft_title.race,
        options.mode,
        &mut report,
    );
    // The Rocket's, the Mine's and the Bomb's own bodies - see
    // `weapon_models::load` for the shared shape, and
    // `MINE_MODEL_ENTRY`/`BOMB_MODEL_ENTRY` for why the latter two carry no
    // drop-time effect alongside them.
    let mut body_model = |entry, fallback| {
        weapon_models::load(&mut archives, entry, fallback, options.lod, &mut report)
    };
    let rocket_model = body_model(ROCKET_MODEL_ENTRY, "a rocket");
    let mine_model = body_model(MINE_MODEL_ENTRY, "a laid mine");
    let bomb_model = body_model(BOMB_MODEL_ENTRY, "a laid bomb");
    let cannon_model = body_model(CANNON_MODEL_ENTRY, "a cannon round");
    // The cockpit half of the shield, on the same terms and for the same
    // reason: not per team, not per track, one entry for every craft in the
    // game. The shell beside it *is* per team and loads with the livery above.
    let shield_cockpit = crate::livery::cockpit_shield(
        craft_of(&mut craft, &mut archives),
        options.lod,
        &mut report,
    );

    // Shared with the sky and the pads below: all three are node classes inside
    // the same track file, and a material in any of them names a texture by its
    // ordinal among *all* of the file's `Texture` nodes, not a per-class one -
    // see `mesh::build_sky`'s doc comment. Resolved once here, against the track
    // model's own embedded textures, and reused rather than re-read from the
    // archive and re-decoded per model.
    let mut ps2_track_textures: Option<mesh::Ps2TextureSet> = None;

    // **A PS3 circuit's geometry is in the `.rcsmodel` beside it.** Both the
    // meshes its `.vex` places - which on HD are all props - and the far larger
    // set of chunks no node references, which is the road itself; see
    // `oag_render::mesh::rcs::build_scene`. A source with no sibling falls back
    // to the derived ribbon, which draws the circuit this project can derive
    // rather than nothing at all, and says so, so nobody mistakes an invented
    // surface for the disc's art.
    let ps3_geometry = if mesh::geometry_is_external(&track_blob) && !options.ribbon {
        let sibling = mesh::rcs::sibling_name(&track);
        let found = sibling.as_deref().and_then(|s| archives.read_name(s).ok());
        if found.is_none() {
            report.push(format!(
                "{track}: a PS3 .vex with no .rcsmodel beside it - drawing the \
                 derived ribbon instead, as --ribbon does"
            ));
        }
        found
    } else {
        None
    };
    // **Built here rather than below, because whether it built decides the
    // fallback.** A `.rcsmodel` that will not decode is the same outcome for a
    // race as no `.rcsmodel` at all - the derived ribbon - and Wipeout 2048 is
    // what made the distinction matter: it ships one beside every circuit and
    // the container is not HD's, so this used to be a hard error that stopped
    // the race. The report names which of the two happened.
    let rcs_model = geometry::track_model(
        &mut archives,
        &track,
        &track_blob,
        &ps3_geometry,
        &mut report,
    );
    let ribbon = options.ribbon || (mesh::geometry_is_external(&track_blob) && rcs_model.is_none());
    // **Everything else built out of the track `.vex` is PSP-shaped geometry.**
    // The sky and the per-node visibility read a `Mesh`-layout payload, which a
    // PS3 file does not have - its `Skycube` is in the `.rcsmodel` too, keyed
    // by a hash this project has not tied back to that class. So they are
    // skipped on a PS3 source for the same reason they are skipped on a
    // ribbon build, and reported the same way: absent, not invented.
    let vex_geometry = !ribbon && rcs_model.is_none();
    // Whether the `.rcsmodel` *built*, not merely whether one was found - the
    // report below says which surface is on screen, and since a sibling that
    // will not decode now falls back to the ribbon, "found" is the wrong
    // question to ask.
    let rcs_drawn = rcs_model.is_some();
    // Both pad classes, decoded once and shared between the two blocks below -
    // mirrors `sky_model`'s own HD/Wipeout 2048 split just past it, since only
    // HD's node tree has a recovered `Speedup Pad`/`Weapon Pad` class id;
    // Wipeout 2048's `.rcsmodel` carries no comparable node tree yet, so a
    // 2048 source stays `(None, None)` here exactly as it did before this
    // pair existed.
    let (ps3_pad_model, ps3_weapon_pad_model) = match &ps3_geometry {
        Some(geometry) if rcs_drawn && !mesh::rcs::psp2::is_psp2(geometry) => {
            pads::ps3_pad_models(&mut archives, &track, &track_blob, geometry, &mut report)?
        }
        _ => (None, None),
    };

    let mut track_model = if let Some(model) = rcs_model {
        model
    } else if ribbon {
        track_render::build_model(&label, &ai)
    } else {
        let mut track_model = mesh::build_with_textures(&track, &track_blob, None, options.lod)?;
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
            track_model =
                mesh::build_with_textures(&track, &track_blob, Some(&external), options.lod)?;
            ps2_track_textures = Some(external);
        }
        track_model
    };
    // The circuit's staging, all four readers at once: its light rig, its
    // distance fog, its bloom block and the Zone stage grade laid over the
    // first two. Every one of them reports what it found and substitutes
    // nothing for what it did not - see `environment::staging`.
    let environment::Staging {
        light,
        authored_fog,
        hd_bloom,
        zone_grade,
    } = environment::staging(
        &mut archives,
        &track,
        environment::GeometryKind::of(ps3_geometry.as_deref()),
        title.race,
        options.mode,
        options.zone_stage,
        &mut report,
    );
    // The track's authored fog volumes. Empty for a ribbon build, and empty for
    // the four circuits that author no `fogCube` at all - both ordinary, and
    // both meaning the race renders unfogged.
    let fog_volumes = if !vex_geometry {
        Vec::new()
    } else {
        let nodes = oag_formats::vex::nodes(&track_blob).unwrap_or_default();
        let volumes = oag_formats::fog::volumes(&track_blob, &nodes);
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
    let weapon_pad_model = if rcs_drawn {
        geometry::weapon_pad_model(ps3_weapon_pad_model, options.mode, &mut report)
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
                if options.mode.weapons_enabled() {
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

    // The authored PVS. Skipped for a ribbon build, whose geometry is generated
    // from the spline rather than authored, so the section boxes have nothing
    // to say about it.
    // **What the circuit asks to be loaded with it, and what of that this
    // project does.** `trackstartup.xml` names up to eight billboard slots and
    // a sound bank; every model it names is on the disc. Seven of the eight are
    // still unplaced - where a hoarding attaches is unrecovered, so they stay
    // unwired rather than put somewhere plausible. **Slot 8 is not a hoarding
    // and is no longer among them**: it is the start gantry, and the circuit's
    // own track geometry authors the surface it stands on. See
    // `oag_formats::trackstartup` and `docs/rendering/start-gantry.md`.
    //
    // **Not HD-only.** Pulse ships the same file per circuit, `fexml`-shortened
    // rather than plain - `TrackStartup::parse` expands either form - so this
    // also runs on real geometry (`vex_geometry`), whose paths are `\`-joined
    // where HD's are `/`-joined; `rfind('/')` alone found nothing on Pulse.
    let mut gantry = None;
    // Whether anything named a slot 8 at all, which is what separates "the gantry
    // did not load" (`place` says so itself) from "nothing asked for one", which
    // otherwise reports nothing at all - see the block after this one.
    let mut named_slot_8 = false;
    if (ps3_geometry.is_some() || vex_geometry)
        && let Some(name) = track
            .rfind(['/', '\\'])
            .map(|at| format!("{}/trackstartup.xml", &track[..at]))
        && let Ok(blob) = archives.read_name(&name)
    {
        let manifest =
            oag_formats::trackstartup::TrackStartup::parse(&String::from_utf8_lossy(&blob));
        let models = manifest
            .billboards
            .iter()
            .filter(|b| b.location().is_some())
            .count();
        report.push(format!(
            "{name}: {} billboard slot(s), {models} naming a model and {} a colour - \
             slots 1-7 unplaced, because what a hoarding attaches to is unrecovered{}",
            manifest.billboards.len(),
            manifest.billboards.len() - models,
            match &manifest.sound_bank {
                Some(bank) => format!("; sound bank {bank}"),
                None => String::new(),
            },
        ));
        // The manifest's own spelling of the model, not a constant here:
        // every Pulse circuit names the same file, and a source that names
        // another gets that one drawn rather than Pulse's substituted for it.
        if let Some(model) = manifest.billboard(8).and_then(|b| b.location()) {
            named_slot_8 = true;
            gantry = super::gantry::place(
                &mut archives,
                model,
                &track_model,
                start_position.as_ref(),
                options.lod,
                &mut report,
            );
        }
    }
    // **Draw nothing and say so**, on the one route into `place` that reports
    // nothing itself: a circuit whose manifest is missing, unreadable, or
    // names no slot 8 never calls it. That circuit can still have a measurable
    // mount, so the silence would read exactly like a circuit that has none.
    if !named_slot_8 && let Some(mount) = oag_render::gantry::mount(&track_model) {
        report.push(format!(
            "no start gantry: nothing named slot 8 - no manifest, or one that names no \
             model for it - although this circuit does author a mount for one, a \
             {:.1} x {:.1} panel at {:?}",
            mount.width,
            mount.height,
            mount.centre.to_array().map(|v| (v * 10.0).round() / 10.0),
        ));
    }
    // The billboard slots are never artwork - see `strip_slot_placeholders`'s own doc.
    let stripped = oag_render::gantry::strip_slot_placeholders(&mut track_model);
    if stripped > 0 {
        report.push(format!("{stripped} billboard-slot placeholder draw(s) suppressed: drawn nothing rather than the stub"));
    }
    let visibility = if vex_geometry {
        TrackVisibility::build(&track_model, &track_blob, &ai)
    } else if ps3_geometry.is_some() {
        TrackVisibility::from_hd_pvs(&mut archives, &track, &mut report)
    } else {
        None
    };
    // A PS3 circuit reports its own line from `from_hd_pvs`, so only the
    // section partition and its absence are described here.
    match &visibility {
        Some(v) if v.chunks().is_none() => report.push(format!(
            "{} authored visibility section(s); {} of {} draw call(s) governed by one \
             ({:.1}%), the rest always drawn; {} LOD-swap pair(s)",
            v.pvs.len(),
            v.placement.placed,
            v.placement.total(),
            v.placement.placed_fraction() * 100.0,
            v.swap_pairs(),
        )),
        None if vex_geometry => report.push(
            "no authored visibility sections: PVS culling is unavailable on this track".to_string(),
        ),
        _ => {}
    }

    let spline = Spline::from_track(&ai);
    report.push(format!(
        "{} spline sample(s), {} per segment, widest half-width {:.1}",
        spline.len(),
        Spline::STEPS_PER_SEGMENT,
        spline.max_half_width()
    ));

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

    // The player's own, and reported by `livery::load` beside the hull it was
    // read off. Sparks are slot 0's today - `oag_render::sparks` triggers off
    // the player's contacts alone - so this takes slot 0's locators rather than
    // carrying eight sets nothing reads.
    let collision_fx = liveries[0].collision_fx.clone();

    // One loop for all of them, and one report line each: adding an effect
    // is adding its name to `RACE_EFFECTS` and a trigger, never a loader.
    let mut effects = psys::Library::new();
    for name in RACE_EFFECTS {
        match particle_effect(&mut archives, name) {
            Ok((effect, note)) => {
                report.push(note);
                effects.insert(name, effect);
            }
            Err(why) => report.push(format!("{why} - {name} will not be drawn")),
        }
    }

    let (sounds, announcer, class_announcer) =
        audio::banks_and_announcers(&mut archives, title.race, options.mode, &mut report);
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
    let (noise, trail_blend, trail_shape) = assets::trail_texture(
        craft_of(&mut craft, &mut archives),
        craft_title,
        &mut report,
    );
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
             flare's spin, chromatic fringe, distance fade and occlusion query (its \
             radius, alpha walk and texture do draw - exhaust::hd::Sprite). The flame \
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
    let flare = assets::flare_texture(
        craft_of(&mut craft, &mut archives),
        craft_title,
        &mut report,
    );

    // The plugin list is the craft's title's, and an empty one is a real
    // answer rather than a missing case: a title whose front end is
    // unrecovered has no declared languages, so the HUD draws its captions as
    // their own `idstring` keys and says so, which is what it already did for
    // a source whose plugins would not parse.
    let language_plugins = craft_title
        .front_end
        .map_or::<&[&str], _>(&[], |front_end| front_end.language_plugins);
    // **The HUD follows the craft**, art included - `load_hud` reads its
    // atlas, font and sight art out of whichever `Archives` it is given.
    let hud = load_hud(
        craft_of(&mut craft, &mut archives),
        craft_title,
        options.mode,
        language_plugins,
        &mut report,
    );

    // The countdown's own `<Mode3D><Model>` mesh - see `crate::hud::countdown`
    // for why this one is drawn through the mesh pipeline rather than
    // `crate::hud::draw::model_draw`'s baked-quad shortcut every other
    // `<Mode3D>` widget uses. Placed by this mode's own layout, found by name
    // rather than assumed present: Pure's HUD authors `Ready_GO.vex` and no
    // `Cockpit321Go` widget at all, so a title or mode with no such widget has
    // nothing to place this model by and gets none, the same "absence is
    // reported, not fatal" rule `weapon_models::load` follows.
    let countdown_model = hud
        .layout
        .as_ref()
        .and_then(|layout| {
            layout
                .models
                .iter()
                .find(|model| model.name == "Cockpit321Go")
        })
        .filter(|widget| !widget.src.is_empty())
        .and_then(|widget| match archives.read_name(&widget.src) {
            Ok(blob) => match mesh::build(&widget.src, &blob) {
                Ok(model) => {
                    report.push(format!(
                        "{}: {} triangle(s), radius {:.2} - the countdown's own Mode3D \
                         model, placed at its widget's authored {:?}",
                        widget.src,
                        model.indices.len() / 3,
                        model.radius,
                        widget.position
                    ));
                    Some((model, widget.clone()))
                }
                Err(error) => {
                    report.push(format!("{}: did not decode ({error})", widget.src));
                    None
                }
            },
            Err(_) => {
                report.push(format!(
                    "{}: absent - the countdown draws nothing this race",
                    widget.src
                ));
                None
            }
        });

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

    // Resolved here rather than in `Race::start` because the spline is what
    // supplies the attitude, and `load` is where the spline is.
    let pose_override = options.pose.and_then(|request| match request {
        PoseRequest::SplineAligned { position, yaw } => {
            let (_, sample, distance) = spline.nearest(position)?;
            report.push(format!(
                "pose override: {position:?} yaw {:.1} deg, attitude from the spline sample \
                 {distance:.1} units away",
                yaw.to_degrees()
            ));
            Some(Pose::from_position_on_sample(sample, position, yaw))
        }
        // Verbatim: the whole point of an exact pose is that nothing here
        // second-guesses the recorded basis against the spline.
        PoseRequest::Exact(pose) => {
            report.push(format!("pose override: exact, at {:?}", pose.position));
            Some(pose)
        }
    });

    Ok(Loaded {
        title,
        setup: Setup {
            mode: options.mode,
            difficulty: options.difficulty,
            class: options.class.clone(),
            opponents: options.opponents,
            trail_sparks: options.trail_sparks,
            seed: options.seed.unwrap_or(SEED),
            zone,
            ai,
            spline,
            course,
            start_position,
            hd_trail: trail_shape.is_some().then_some(hd_trail_red),
            collision,
            handling,
            airbrake_graphics,
            chase,
            chase_close,
            internal,
            nozzles: liveries.iter().map(|livery| livery.nozzle).collect(),
            spark_anchors: liveries
                .iter()
                .map(|livery| livery.collision_fx.clone())
                .collect(),
            collision_fx,
            effects,
            sounds,
            track_emitters,
            announcer,
            class_announcer,
            zone_stages: title.race.zone_stages,
            speedup_pads,
            weapon_pads,
            weapons,
            weapon_pad_refresh,
            class_gravity_scale,
            pose_override,
            camera_override: options.camera,
        },
        hud,
        track_model,
        gantry,
        collision_model,
        sky_model,
        pad_model,
        weapon_pad_model,
        fog_volumes,
        light,
        authored_fog,
        hd_bloom,
        zone_grade,
        liveries,
        rocket_model,
        cannon_model,
        mine_model,
        bomb_model,
        shield_cockpit,
        countdown_model,
        visibility,
        flare,
        noise,
        trail_blend,
        trail_shape,
        shadows,
        shadow_hulls,
        report,
    })
}
