//! [`load`]: everything a race needs, read out of one disc image, and the report
//! of what came back.
//!
//! Split out of `race.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change. Its tests are
//! `race/tests/load.rs`.

use super::*;

mod cameras;
mod environment;
use environment::{envsettings_fog, envsettings_light, hd_sky_model};

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
    // `source` names - see `docs/formats/dlc-pack.md`. **A Pure source mounts no
    // packs at all**, which `crate::title::open_source` decides rather than this
    // call site; see its own docs for why silently mounting Pulse's DLC behind a
    // different title would be worse than mounting none.
    let (packs, problems) = crate::dlc::packs(&options.dlc, &crate::boot::default_dlc_cache_dir());
    // **Opened as whichever title the source turned out to be**, the same way the
    // boot path already does it. This used to be `oag_pulse::open_with_packs`
    // outright, which refused a Pure disc by name (`WrongTitle`) - so `--race`
    // and the menus' own `Launch Game` could not reach a second title at all,
    // however much of the rest of the path was ready for one.
    let opened = crate::title::open_source(&options.source, packs)?;
    let title = opened.title;
    let mut archives = opened.archives;
    report.push(format!("racing on {}", title.name));
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
        Some(slot) => report.push(format!(
            "Start Position: {:?} facing {:?}",
            slot.position, slot.forward
        )),
        None => report.push("no Start Position node: spawning on the spline instead".to_string()),
    }
    let nodes = collision::from_vex(&track_blob).map_err(|e| anyhow::anyhow!("{}: {e}", track))?;
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
    // See [`Options::team`].
    let team = match &options.team {
        Some(asked) => asked.clone(),
        None => {
            report.push(format!(
                "no team named: {}'s own default, {}",
                title.name, title.race.team
            ));
            title.race.team.to_string()
        }
    };

    let stats_name = handling::entry_name(&team);
    let stats_blob = read(&mut archives, &stats_name)?;
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
    // Three distinct failures, reported as three distinct lines. The decoder was
    // deliberately tightened so an incomplete `<Global>` names the element,
    // attribute or class that is missing rather than returning a zero, and
    // collapsing that into one "unreadable" would throw the whole point away -
    // especially now that `<Zone>` and `<SpeedupPads>` share one result, so a
    // malformed pad block would otherwise be reported as a missing Zone law.
    let global = match read(&mut archives, handling::GLOBAL_ENTRY) {
        Err(e) => {
            report.push(format!("{}: {e}", handling::GLOBAL_ENTRY));
            None
        }
        Ok(blob) => match handling::global_from_blob(&blob) {
            Err(e) => {
                report.push(format!("{}: {e}", handling::GLOBAL_ENTRY));
                None
            }
            Ok(None) => {
                report.push(format!(
                    "{} carries no <Global> block",
                    handling::GLOBAL_ENTRY
                ));
                None
            }
            Ok(some) => some,
        },
    };
    // An absent or unreadable file means no boost and no auto-speed rather than
    // invented numbers. Said out loud, because a speed pad that quietly does
    // nothing reads as a physics bug and gets looked for in the force law.
    let pad_tunables = match global {
        Some(global) => global.speedup_pads(to_format_class(options.class)),
        None => {
            report.push("speed pads apply no boost this run".to_string());
            handling::SpeedupPads::default()
        }
    };
    // `<Special speedpad_jump>`, out of the same file and with the same fallback
    // reasoning: a zero means the boost simply does not tilt, which is a missing
    // feature rather than a broken race.
    let special = global.map(|global| global.special).unwrap_or_default();
    // Reported for the same reason the pad tunables are: a tilt that silently
    // reads zero is indistinguishable from a tilt that is not implemented, and
    // this is the only place the number's journey off the disc is observable.
    report.push(format!(
        "<Special speedpad_jump>: {} - the boost tilts {:.2} degrees toward the \
         hull's up while the pitch axis is held up",
        special.speedpad_jump,
        special.speedpad_jump.atan().to_degrees()
    ));
    // Nothing is scaled here: the four pre-scaled fields are converted exactly once
    // and this is not the place it happens, and neither `<SpeedupPads>` nor
    // `<Special>` is one of them.
    let handling = handling_for(&stats, options.class, pad_tunables, special);
    // `<AirbrakeGraphics>` is the fifth pre-scaled field and it *is* converted
    // here, by its own function: it is per team rather than per class, and it
    // is graphics, so it deliberately never enters `Handling`.
    let airbrake_graphics = oag_gameplay::airbrake_graphics_for(&stats);
    // The per-class scale on grounded gravity, `g_class_gravity_scale`. **The
    // fallback is the identity, not zero**, unlike the pad tunables above: a
    // missing boost is a missing feature, but a zero here would leave a grounded
    // craft weightless, which is not a degraded race - it is a broken one.
    //
    // Named `airborne` in the XML and applied to the *grounded* term; see
    // `oag_formats::handling::GravityMul`, which reads the VFPU pair chain out.
    let class_gravity_scale = match global {
        Some(global) => {
            let scale = global.gravity_mul(to_format_class(options.class)).airborne;
            report.push(format!(
                "<GravityMul>: grounded gravity scaled by {scale} for the {:?} class",
                options.class
            ));
            scale
        }
        None => {
            report.push("gravity is unscaled this run".to_string());
            1.0
        }
    };
    let zone = if options.mode == Mode::Zone {
        match global {
            Some(global) => report.push(format!(
                "<Zone>: start {}, increment {} per zone, recharge {}",
                global.zone.start, global.zone.increment, global.zone.recharge
            )),
            None => report.push("this run has no auto-speed".to_string()),
        }
        global.map(|g| g.zone)
    } else {
        None
    };
    // The weapon-pad debounce, out of the same `<GlobalClass>` block as the two
    // above and with the same "absent means the feature is off" fallback. Zero
    // is a real degradation rather than a neutral value - it would let one
    // crossing grant a pickup on every tick the hull is inside the volume - so
    // the trigger treats a zero as "grant once and never again on this pad"
    // rather than trusting it; see `Race::test_weapon_pads`.
    let weapon_pad_refresh = match global {
        Some(global) => {
            global
                .weapon_pads(to_format_class(options.class))
                .refresh_time
        }
        None => 0.0,
    };
    // The weapon table. Read on every mode even though only a single race arms
    // the pads: which mode is racing is a gameplay question and this is the
    // asset half, and reading it unconditionally is what makes a broken file a
    // reported line on every run rather than one nobody sees until they pick
    // the one mode that needs it.
    //
    // Two failures, two lines, for the reason `<Global>` above gives at length.
    let weapons = match read(&mut archives, oag_formats::weapons::RACE_ENTRY) {
        Err(e) => {
            report.push(format!("{}: {e}", oag_formats::weapons::RACE_ENTRY));
            None
        }
        Ok(blob) => match oag_formats::weapons::from_blob(&blob) {
            Err(e) => {
                report.push(format!("{}: {e}", oag_formats::weapons::RACE_ENTRY));
                None
            }
            Ok(stats) => {
                report.push(format!(
                    "{}: {} weapon(s) with an absorb value, {} pickup table(s)",
                    oag_formats::weapons::RACE_ENTRY,
                    stats.absorb.len(),
                    stats.pickups.len()
                ));
                Some(stats)
            }
        },
    };
    if options.mode.weapons_enabled() && weapons.is_none() {
        // Only worth saying on a mode that would otherwise hand something out.
        report.push("weapon pads hand nothing out this run".to_string());
    }
    let (chase, chase_close, internal) =
        cameras::resolve(&stats, &stats_name, options, &handling, &team, &mut report);

    // One hull, plume and nozzle per grid slot, each off its own team's
    // directory - see `crate::livery`, which also records what is recovered
    // here (the paths) and what is this project's (which team flies which
    // slot). Slot 0 is the player's.
    // Which teams the field may fly. The caller's list wins, because only the
    // caller can know about mounted DLC packs - the front end passes the whole
    // catalogue. When it is empty the disc's own plugin definition is read
    // here instead, which is what makes `--race`, a capture and a test field a
    // varied grid rather than eight identical ships; an entry point that
    // forgot to pass the list is exactly how this was first found.
    //
    // **The definition is the title's own**, and reaching for Pulse's constant
    // regardless of title is one of the two ways this same grid went uniform on
    // an HD source: HD names its game plugin where the PSP titles number it, so
    // the read missed. See `oag_title::Title::plugin_definition`.
    //
    // **The other way was `fexml::expand` rather than `fexml::text`**, and it
    // is the trap that page already records for the PS2 in-race HUD: shortening
    // is per *file*, not per platform, and HD's definition is plain `<?xml`.
    // `expand` refuses a file with no `<code>` dictionary, so a perfectly good
    // document came back as `NoDictionary` and the roster was empty again -
    // with the path already fixed. `text` decides from the blob.
    //
    // Both failures used to read out as `0 team(s)`, which is why the reason is
    // reported now: an empty roster and an unreadable one are the same grid and
    // very different bugs.
    let available: Vec<String> = if options.opponent_teams.is_empty() {
        let definition = title.plugin_definition;
        let declared = archives
            .read_name(definition)
            .map_err(|error| error.to_string())
            .and_then(|blob| oag_formats::fexml::text(&blob).map_err(|error| error.to_string()))
            .map(|xml| crate::catalogue::teams(&xml));
        let declared = match declared {
            Ok(declared) => {
                report.push(format!(
                    "{definition}: {} team(s) for the grid, read here because the caller \
                     passed none",
                    declared.len()
                ));
                declared
            }
            Err(error) => {
                report.push(format!(
                    "{definition}: {error} - no team declared, so the whole grid wears the \
                     player's hull"
                ));
                Vec::new()
            }
        };
        declared.into_iter().map(|team| team.id).collect()
    } else {
        options.opponent_teams.clone()
    };
    let slot_teams = livery::teams_for_slots(&team, &available, oag_gameplay::MAX_SHIPS);
    // HD's red-trail flag, from each slot's ship directory: the runtime sets
    // it by the model-variant name (`concept1`/`nitro`/`detonator`/
    // `chrome_c1`), and on the disc those variants live in `*_c1`, `*_n1` and
    // `detonator` directories. See `Loaded::hd_trail_red`.
    let hd_trail_red = std::array::from_fn(|slot| {
        let fury = slot_teams.get(slot).is_some_and(|team| {
            team.ends_with("_c1") || team.ends_with("_n1") || team == "detonator"
        });
        if fury { 1.0 } else { 0.0 }
    });
    let liveries = livery::load(
        &mut archives,
        &slot_teams,
        options.mode,
        title.race.zone_craft,
        title.flare,
        options.lod,
        &mut report,
    )?;
    report.push(format!(
        "grid liveries: {} - which team flies which slot is this project's, not \
         the original's (livery.rs)",
        slot_teams.join(", ")
    ));
    // The Rocket's own model, on the same terms as the boost plume: absence is
    // reported, not fatal. It is not per-team and not per-track - one entry
    // serves every rocket in the game, which is why it loads here once and the
    // scene clones it per projectile slot.
    let rocket_model = match archives.read_name(ROCKET_MODEL_ENTRY) {
        Ok(blob) => match mesh::build_with_textures(ROCKET_MODEL_ENTRY, &blob, None, options.lod) {
            Ok(model) => {
                report.push(format!(
                    "{ROCKET_MODEL_ENTRY}: {} triangle(s), radius {:.2} - a rocket in \
                     flight is this model, not a billboard",
                    model.indices.len() / 3,
                    model.radius
                ));
                Some(model)
            }
            Err(error) => {
                report.push(format!("{ROCKET_MODEL_ENTRY}: did not decode ({error})"));
                None
            }
        },
        Err(_) => {
            report.push(format!(
                "{ROCKET_MODEL_ENTRY}: absent - rockets fall back to a billboard"
            ));
            None
        }
    };
    // The cockpit half of the shield, on the same terms and for the same
    // reason: not per team, not per track, one entry for every craft in the
    // game. The shell beside it *is* per team and loads with the livery.
    let shield_cockpit = crate::livery::cockpit_shield(&mut archives, options.lod, &mut report);

    // Shared with the sky and the pads below: all three are node classes inside
    // the same track file, and a material in any of them names a texture by its
    // ordinal among *all* of the file's `Texture` nodes, not a per-class one -
    // see `mesh::build_sky`'s doc comment. Resolved once here, against the track
    // model's own embedded textures, and reused rather than re-read from the
    // archive and re-decoded per model.
    let mut ps2_track_textures: Option<Vec<Option<mesh::ModelTexture>>> = None;

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
    let ribbon =
        options.ribbon || (mesh::geometry_is_external(&track_blob) && ps3_geometry.is_none());
    // **Everything else built out of the track `.vex` is PSP-shaped geometry.**
    // The sky, the two pad classes and the per-node visibility all read a
    // `Mesh`-layout payload, which a PS3 file does not have - its `Skycube` and
    // pad geometry are in the `.rcsmodel` too, keyed by a hash this project has
    // not tied back to those classes. So they are skipped on a PS3 source for
    // the same reason they are skipped on a ribbon build, and reported the same
    // way: absent, not invented.
    let vex_geometry = !ribbon && ps3_geometry.is_none();

    let track_model = if let Some(geometry) = &ps3_geometry {
        let (model, built) = mesh::rcs::build_scene(&track, &track_blob, geometry, &mut |path| {
            archives.read_name(path).ok()
        })?;
        report.push(format!("{track}: {}", built.describe()));
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
            ps2_track_textures = Some(external.clone());
            track_model =
                mesh::build_with_textures(&track, &track_blob, Some(external), options.lod)?;
        }
        track_model
    };
    // **The circuit's own light rig, where it authors one.** Wipeout HD does:
    // `track.envsettings` sits beside `track.vex` and states a sun direction, a
    // sun colour and a constant ambient. `mesh.wgsl`'s two-light rig is a
    // stand-in for exactly this and says so, and `CLAUDE.md`'s rule about not
    // inventing what the assets already author is why it is read here rather
    // than approximated.
    //
    // A sibling-path rewrite, the same shape as `mesh::rcs::sibling_name`, and
    // the same failure behaviour: no file, an unparsable one, or a degenerate
    // sun direction all fall back to the stand-in and say so. Nothing is
    // substituted for a value the file does not carry.
    let light = envsettings_light(&mut archives, &track, &mut report);
    // The circuit's authored distance fog, on the same file. **The curve is no
    // longer a guess**: every fogged fragment variant of an HD circuit
    // `.rcsmaterial` computes `exp(-(coefficient * view_depth)^2)` and lerps a
    // patched `fogColour` in by it - read out of the microcode itself, see
    // `mesh_render::Fog::curve`. Gated to the PS3 path: a Pulse circuit fogs
    // through its `fogCube` volumes and authors no `.envsettings` at all.
    let authored_fog = if ps3_geometry.is_some() {
        envsettings_fog(&mut archives, &track, &mut report)
    } else {
        None
    };
    // The circuit's `HDR and Bloom` block, which is what turns the HD race
    // onto the linear float scene target and the read FunkLayerBloom chain -
    // see `oag_render::post::hd_bloom` for what of that is the microcode's
    // and what is this project's. Gated to the PS3 path like the fog above.
    let hd_bloom = if ps3_geometry.is_some() {
        environment::envsettings_bloom(&mut archives, &track, &mut report)
    } else {
        None
    };
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
    let sky_model = if ps3_geometry.is_some() {
        // Wipeout HD authors no `Skycube` node at all - its sky is the
        // `sky.gtf` cubemap beside the track, drawn through the same
        // camera-centred sky path. See `mesh::sky_cube` for what of that is
        // the disc's and what is this project's.
        hd_sky_model(&mut archives, &track, &mut report)
    } else if !vex_geometry {
        None
    } else {
        let sky = mesh::build_sky(&track, &track_blob, ps2_track_textures.clone())?;
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
    // shows none of the disc's art meshes, pads included.
    let pad_model = if !vex_geometry {
        None
    } else {
        let pads = mesh::build_pads(&track, &track_blob, ps2_track_textures.clone())?;
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
    // The pickup pads, built the same way and kept separate for the reason
    // `mesh::build_weapon_pads` gives: they are a different gameplay object and
    // a merged buffer could not show one without the other. **Nothing hands
    // anything out yet** - see the roadmap's weapons item.
    //
    // **Decoded here regardless of `options.mode`.** Whether it actually reaches
    // the screen is a `Scene::new` decision, not this one - see its own comment,
    // and `Mode::weapons_enabled`. Keeping the decode unconditional is what lets
    // `the_weapon_pads_are_drawn_where_they_trigger` check the geometry against
    // the trigger volumes on every mode's own default `Options`, and it mirrors
    // the original's own order of operations: `World_CollectNodeLists` always
    // walks the tree before anything asks whether weapons are on.
    let weapon_pad_model = if !vex_geometry {
        None
    } else {
        let pads = mesh::build_weapon_pads(&track, &track_blob, ps2_track_textures.clone())?;
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
        match (vex_geometry, ps3_geometry.is_some()) {
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
    // a sound bank; every model it names is on the disc, and **nothing places
    // them** - where a slot's transform comes from is unrecovered, so they stay
    // unwired rather than put somewhere plausible. Reported so that is a line
    // rather than a silence. See `oag_formats::trackstartup`.
    if ps3_geometry.is_some()
        && let Some(name) = track
            .rfind('/')
            .map(|at| format!("{}/trackstartup.xml", &track[..at]))
    {
        match archives.read_name(&name) {
            Err(_) => {}
            Ok(blob) => {
                let manifest =
                    oag_formats::trackstartup::TrackStartup::parse(&String::from_utf8_lossy(&blob));
                let models = manifest
                    .billboards
                    .iter()
                    .filter(|b| b.location().is_some())
                    .count();
                report.push(format!(
                    "{name}: {} billboard slot(s), {models} naming a model and {} a colour - \
                     none placed, because what a slot attaches to is unrecovered{}",
                    manifest.billboards.len(),
                    manifest.billboards.len() - models,
                    match &manifest.sound_bank {
                        Some(bank) => format!("; sound bank {bank}"),
                        None => String::new(),
                    },
                ));
            }
        }
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

    // The same shape for sound, and for the same reason: adding a cue is adding
    // its name and its trigger, never a loader. Zone races read `ship_zone.bnk`
    // instead of `ship.bnk` - a different bank with the same cue names.
    let sounds = crate::audio::sfx::Banks::load(
        &mut archives,
        title.race.sounds,
        options.mode == Mode::Zone,
    );
    report.extend(sounds.report.iter().cloned());

    // The ribbon's texture is a title axis, not a constant: Pulse and Pure name
    // one, HD authors a template whose material names its own. See
    // `oag_title::exhaust::Exhaust` and `assets::trail_texture`.
    let (noise, trail_blend, trail_shape) =
        assets::trail_texture(&mut archives, title, &mut report);
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
    // the ribbon's: Pulse and Pure name a sprite texture, HD authors a per-team
    // model. See `oag_title::flare::Flare` and `assets::flare_texture`.
    let flare = assets::flare_texture(&mut archives, title, &mut report);

    // The plugin list is the title's, and an empty one is a real answer rather
    // than a missing case: a title whose front end is unrecovered has no
    // declared languages, so the HUD draws its captions as their own `idstring`
    // keys and says so, which is what it already did for a source whose plugins
    // would not parse.
    let language_plugins = title
        .front_end
        .map_or::<&[&str], _>(&[], |front_end| front_end.language_plugins);
    let hud = load_hud(
        &mut archives,
        title,
        options.mode,
        language_plugins,
        &mut report,
    );

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
        setup: Setup {
            mode: options.mode,
            difficulty: options.difficulty,
            class: options.class,
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
        collision_model,
        sky_model,
        pad_model,
        weapon_pad_model,
        fog_volumes,
        light,
        authored_fog,
        hd_bloom,
        liveries,
        rocket_model,
        shield_cockpit,
        visibility,
        flare,
        noise,
        trail_blend,
        trail_shape,
        report,
    })
}
