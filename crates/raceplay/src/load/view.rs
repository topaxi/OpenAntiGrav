//! The view phase of a load: everything only the picture and the sound of a race are
//! built from, read after the sim phase ([`super::load_sim`]) and off what it read.
//!
//! Its steps run in the order a whole load runs them, and each group's report lines go
//! where the sim phase marked its [`Seam`], so the report reads as one pass.

use super::seams::{Seam, Seams};
use super::*;
use environment::cloud_layer;

/// Reads the picture's and the sound's half of a race off what [`super::load_sim`] read:
/// the [`ViewSetup`] and the drawn assets, and the [`Loaded`] that carries both halves.
///
/// # Errors
///
/// Propagates a track model, sky or pad set that will not build, and the player's own hull
/// failing to read or build.
pub fn load_view(sim: SimLoad, options: &Options) -> Result<Loaded> {
    let SimLoad {
        setup,
        modules,
        title,
        craft_title,
        circuit: track,
        report: sim_report,
        seams: sim_seams,
        mut archives,
        mut craft,
        texture_pack,
        track_blob,
        label,
        track_classes,
        nodes,
        foreign_art: track_art,
        stats,
        stats_name,
        team,
        hull_variant,
        circuit_hull,
        zone_file_read,
        extension_sources,
        track_stats,
        vex_geometry: sim_vex_geometry,
        extents,
    } = sim;
    let _pack = texture_pack;
    let mut report = Vec::new();
    let mut seams = Seams::default();
    let (ai, start_position, handling, slot_teams, weapons) = (
        &setup.ai,
        &setup.start_position,
        &setup.handling,
        &setup.slot_teams,
        &setup.weapons,
    );

    seams.mark(Seam::TrackArt, &report);
    let (mut drawn, mut foreign_art) = match &options.foreign_track {
        Some(path) => {
            let (model, extras) = track_art::foreign_drawn(
                path,
                &label,
                &nodes,
                track_art,
                &setup.speedup_pads,
                &setup.weapon_pads,
                title,
                &mut archives,
                &mut report,
            );
            (Some(model), extras)
        }
        None => (None, crate::foreign::art::Extras::default()),
    };
    // `<AirbrakeGraphics>` is the fifth pre-scaled field and it *is* converted
    // here, by its own function: it is per team rather than per class, and it
    // is graphics, so it deliberately never enters `Handling`.
    let airbrake_graphics = oag_gameplay::airbrake_graphics_for(&stats);

    seams.mark(Seam::Cameras, &report);
    let (chase, chase_close, internal) = cameras::resolve(
        title,
        &stats,
        &stats_name,
        options,
        handling,
        &team,
        zone_file_read,
        &mut report,
    );
    // The title's view matrix: a shear where the original's camera node is one. The lock-on's
    // rig below never draws, so it keeps the flag off.
    let (chase, chase_close) = {
        let sheared = craft_title
            .race
            .craft_laws
            .is_some_and(|laws| laws.chase_view_sheared.value);
        (
            ChaseParams {
                view_sheared: sheared,
                ..chase
            },
            ChaseParams {
                view_sheared: sheared,
                ..chase_close
            },
        )
    };
    debug_assert_eq!(
        chase, setup.chase,
        "the drawn far view and the simulation's lock-on rig are the same rig"
    );

    // What the scene target stores, which baked colours are encoded for; told before the staging.
    let store = environment::scene_store(title, &track_blob, options.ribbon);

    seams.mark(Seam::Looks, &report);
    let looks = roster::looks(
        craft_of(&mut craft, &mut archives),
        craft_title,
        options,
        &team,
        &mut report,
    );
    seams.mark(Seam::Liveries, &report);
    let roster::Grid {
        hd_trail_red,
        liveries,
    } = roster::grid(
        craft_of(&mut craft, &mut archives),
        craft_title,
        options,
        looks,
        slot_teams,
        (hull_variant, circuit_hull, store),
        &mut report,
    )?;

    seams.mark(Seam::Scene, &report);
    let shadows = super::shadow::assets(
        craft_of(&mut craft, &mut archives),
        slot_teams,
        craft_title.race,
        options.mode,
        options.shadow_resolution,
        &mut report,
    );
    // Every weapon's own body, per title; a Detonator race's mine and bomb are its own
    // (`oag_title::DetonatorObjects`).
    let detonator_objects = craft_title
        .race
        .detonator
        .and_then(|detonator| detonator.objects)
        .filter(|_| options.mode == Mode::Detonator);
    let detonator_weapons;
    let replaces_circuit_pads = craft_title
        .race
        .detonator
        .is_some_and(|detonator| detonator.pads.replaces_circuit_pads.value);
    let wm = match &detonator_objects {
        Some(objects) => {
            detonator_weapons =
                weapon_models::detonator_weapon_models(craft_title.weapon_models, objects);
            &detonator_weapons
        }
        None => craft_title.weapon_models,
    };
    let (
        rocket_model,
        mine_model,
        bomb_model,
        cannon_model,
        plasma_blast_models,
        mut bomb_blast_models,
        leach_ball_model,
    ) = weapon_models::load_bodies(&mut archives, wm, store, &mut report);
    if let Some(entry) = title.destroyed.value.gaseous_flash {
        bomb_blast_models.gaseous = weapon_models::load_optional(
            &mut archives,
            Some(entry),
            "the destroyed craft's gaseous flash (nothing else is drawn in its place)",
            false,
            store,
            &mut report,
        );
    }
    if let Some(objects) = &detonator_objects {
        let pads = setup
            .speedup_pads
            .iter()
            .chain(&setup.weapon_pads)
            .map(|pad| Mat4::from_cols_array(&pad.to_world))
            .collect();
        bomb_blast_models.detonator =
            weapon_models::load_detonator_objects(&mut archives, objects, pads, store, &mut report);
    }
    let hides_circuit_pads = replaces_circuit_pads && bomb_blast_models.detonator.pad.is_some();
    // The cockpit half of the shield, on the same terms and for the same
    // reason: not per team, not per track, one entry for every craft in the
    // game. The shell beside it *is* per team and loads with the livery above.
    let (shield_cockpit, shield_cockpit_alpha) =
        oag_livery::cockpit_shield(craft_of(&mut craft, &mut archives), store, &mut report);

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
    // `oag_mesh::mesh::rcs::build_scene`. A source with no sibling falls back
    // to the derived ribbon, which draws the circuit this project can derive
    // rather than nothing at all, and says so, so nobody mistakes an invented
    // surface for the disc's art.
    //
    // **Two spellings are tried, in this order: `track.rcsmodel`, then the
    // Omega Collection's `track.final.rcsmodel`.** No circuit ships both (see
    // `mesh::rcs::siblings`), so the order only decides which title's
    // lookup pays a second, failing read - every other title's pays none.
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
    stages::checkpoint()?;
    // **Built here rather than below, because whether it built decides the
    // fallback.** A `.rcsmodel` that will not decode is the same outcome for a
    // race as none at all - the derived ribbon - and Wipeout 2048 made the
    // distinction matter: it ships one beside every circuit, in a container
    // that is not HD's, and that used to stop the race. The report says which.
    let (rcs_model, mut behind_glass) = geometry::track_model(
        &mut archives,
        &track,
        &track_blob,
        &ps3_geometry,
        geometry_name.as_deref(),
        title.looks,
        options.mode == Mode::Zone,
        store,
        options.water,
        geometry::hidden_pads(hides_circuit_pads, options.weapons_on()),
        &mut report,
    );
    // The target's size is the original's, or a fraction of it
    // (`graphics.reflection_resolution`); a circuit with no target has nothing to resize.
    if !behind_glass.groups.is_empty() || behind_glass.mirror.is_some() {
        behind_glass.resolution = options.reflection_resolution;
        let (original, size) = (behind_glass.size, behind_glass.drawn_size());
        if options.reflection_resolution.is_auto() {
            report.push(match options.reflection_timed {
                Some(true) => format!(
                    "{track}: the glass and sea target starts at {}x{} under reflection resolution \
                     auto, and steps to half and quarter by its own timed cost",
                    original.0, original.1
                ),
                Some(false) => format!(
                    "{track}: the glass and sea target stays at {}x{} under reflection resolution \
                     auto: nothing times its pass here, so there is no cost to step it by",
                    original.0, original.1
                ),
                None => format!(
                    "{track}: the glass and sea target starts at {}x{} under reflection resolution \
                     auto, and steps to half and quarter by its own timed cost where the frame loop \
                     can time it",
                    original.0, original.1
                ),
            });
        } else if size != original {
            report.push(format!(
                "{track}: the glass and sea target is {}x{} at reflection resolution {} (the \
                 original's is {}x{})",
                size.0, size.1, options.reflection_resolution, original.0, original.1
            ));
        }
    }
    stages::checkpoint()?;
    let ribbon = options.ribbon
        || drawn.is_some()
        || (oag_vex::vex::geometry_is_external(&track_blob) && rcs_model.is_none());
    // **Everything else built out of the track `.vex` is PSP-shaped geometry.**
    // The sky and the per-node visibility read a `Mesh`-layout payload, which a
    // PS3 file does not have - its `Skycube` is in the `.rcsmodel` too, keyed
    // by a hash this project has not tied back to that class. So they are
    // skipped on a PS3 source for the same reason they are skipped on a
    // ribbon build, and reported the same way: absent, not invented.
    let vex_geometry = !ribbon && rcs_model.is_none();
    // Whether the `.rcsmodel` *built*, not merely whether one was found: a sibling
    // that will not decode falls back to the ribbon.
    let rcs_drawn = rcs_model.is_some();
    // Both pad classes, decoded once and shared between the two blocks below. Only
    // HD's node tree has a recovered `Speedup Pad`/`Weapon Pad` class id; a 2048
    // source stays `(None, None)`, as `sky_model`'s own HD/2048 split just past it.
    let (ps3_pad_model, ps3_weapon_pad_model) = match &ps3_geometry {
        Some(geometry) if rcs_drawn && !mesh::rcs::psp2::is_psp2(geometry) => pads::ps3_pad_models(
            &mut archives,
            &track,
            &track_blob,
            (geometry, store),
            &mut report,
        )?,
        _ => (None, None),
    };

    let mut track_model = if let Some(model) = rcs_model {
        model
    } else if ribbon {
        drawn
            .take()
            .unwrap_or_else(|| track_render::build_model(&label, ai))
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
        water::enhanced(&mut track_model, options.water, title, &track, &mut report);
        geometry::with_batch_cull(track_model, title.looks, archives.layout.platform)
    };
    // HD's zoom-streak ring on a title that draws none of its own: the player's
    // `graphics.boost_zoom`, honoured only where the title offers it. Chosen, not
    // measured - see `crate::zoom`.
    let borrowed_zoom = options.boost_zoom.filter(|_| title.race.borrows_zoom_ring);
    // The circuit's staging, all four readers at once: its light rig, its
    // distance fog, its bloom block and the Zone stage grade laid over the
    // first two. Every one of them reports what it found and substitutes
    // nothing for what it did not - see `environment::staging`.
    let environment::Staging {
        light,
        authored_fog,
        hd_bloom,
        omega_tonemap,
        wo_composite,
        zone_grade,
    } = environment::staging(
        &mut archives,
        &track,
        environment::GeometryKind::of(ps3_geometry.as_deref()),
        title,
        options.mode,
        options.zone_stage,
        &mut report,
    );
    // A comparison's held exposure over the circuit's adapting one.
    let wo_composite = wo_composite.map(|params| oag_post::wo_composite::Params {
        exposure: options.exposure_hold.map_or(
            oag_post::wo_composite::Exposure::Adapting,
            oag_post::wo_composite::Exposure::Held,
        ),
        ..params
    });
    let probes =
        environment::read_probes(&mut archives, &track, ps3_geometry.as_deref(), &mut report);
    // `fc01_dummy`'s pixel is the circuit's fog colour, which only the staging
    // knows: write it into the vertices that carry the placeholder.
    environment::fill_flat_fog(
        &mut track_model,
        authored_fog,
        light.output_scale,
        &mut report,
    );
    let spu_vertex_lights = engine_light::spu_vertex_lights_enabled(
        &mut archives,
        &track,
        ps3_geometry.is_some(),
        &mut report,
    );
    // The track's authored fog volumes. Empty for a ribbon build, and empty for
    // the four circuits that author no `fogCube` at all - both ordinary, and
    // both meaning the race renders unfogged.
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

    stages::checkpoint()?;
    // The ribbon build draws an invented surface rather than the disc's art, so
    // it gets no sky either: the two belong to the same "show what shipped" mode.
    let sky_scene = (title, options.mode, store);
    let sky_model = if let Some(geometry) = &ps3_geometry {
        environment::ps3_sky_model(&mut archives, &track, geometry, sky_scene, &mut report)
    } else if let Some(sky) = foreign_art.own_sky.take() {
        Some(sky)
    } else if let Some(entry) = &foreign_art.sky {
        track_art::foreign_sky(&mut archives, entry, &mut report)
    } else if let Some(sky) =
        track_art::host_zone_sky(&mut archives, &track, sky_scene, &mut report)
    {
        Some(sky)
    } else if !vex_geometry {
        None
    } else {
        let mut sky = mesh::build_sky(&track, &track_blob, ps2_track_textures.as_ref())?;
        environment::unlit_sky(&mut sky, &mut report);
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
        foreign_art.pads.take()
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
    stages::checkpoint()?;
    let weapon_pad_model = if rcs_drawn {
        geometry::weapon_pad_model(ps3_weapon_pad_model, options.weapons_on(), &mut report)
    } else if !vex_geometry {
        crate::foreign::art::weapon_pads(&mut foreign_art, options.weapons_on(), &mut report)
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
    report.push(format!(
        "drawing the track's {}: {} triangle(s), radius {:.0}",
        match (vex_geometry, rcs_drawn) {
            (true, _) => "art meshes",
            (false, true) => "art meshes out of the .rcsmodel beside it",
            (false, false) if track.starts_with("foreign:") => "foreign track model",
            (false, false) => "driveable ribbon",
        },
        track_model.indices.len() / 3,
        track_model.radius
    ));

    // A Detonator race draws its own `Detonator_Pad` on every speed and weapon pad in place of the
    // circuit's own (`docs/ghidra/functions/ps3-hdfury-eu/detonator-emp.md`, "Do the circuit's
    // own pads still draw?"): where the title's pad model loaded, the circuit's two pad models
    // are dropped.
    let circuit_pad_key = craft_title
        .race
        .detonator
        .and_then(|d| d.pads.circuit_weapon_pad_key)
        .filter(|_| options.mode == Mode::Detonator && detonator_objects.is_none());
    let (pad_model, weapon_pad_model) = if let Some(key) = circuit_pad_key {
        // A title with no Detonator pad model: every pad carries the race circuit's own weapon
        // pad, in place of the circuit's speed and weapon pads (`docs/gameplay/detonator-pulse.md`).
        let places: Vec<[f32; 16]> = setup
            .speedup_pads
            .iter()
            .chain(&setup.weapon_pads)
            .map(|pad| pad.to_world)
            .collect();
        match pads::circuit_weapon_pads(&mut archives, title, &track, &places, key, &mut report) {
            Some(model) => (Some(model), None),
            None => (pad_model, weapon_pad_model),
        }
    } else if hides_circuit_pads {
        report.push(
            "detonator: the circuit's own speed and weapon pads are not drawn under the \
             Detonator pad"
                .to_string(),
        );
        (None, None)
    } else {
        (pad_model, weapon_pad_model)
    };
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

    let (gantry, visibility, adverts, zone_equaliser) = gantry_visibility::build(
        &mut archives,
        &track,
        &mut track_model,
        &track_blob,
        ai,
        ps3_geometry.as_deref(),
        geometry_name.as_deref(),
        vex_geometry,
        gantry_visibility::adverts_for(title, options.mode),
        gantry_visibility::gantry_card(title),
        gantry_visibility::zone_shares_texture(title, options.mode),
        options.mode,
        start_position.as_ref(),
        &mut report,
    );
    // What `vex_geometry` comes to above with nothing built: an `.rcsmodel` that
    // builds and one that does not both leave it false, so only the request,
    // a foreign track and the file's own export decide it. `pulse_psp` below
    // reads it, and through `track_stats` the finished thrust does.
    debug_assert_eq!(
        vex_geometry, sim_vex_geometry,
        "the sim phase reads the track's own geometry where the view draws it"
    );
    stages::reach(stages::Stage::TrackBuilt)?;

    seams.mark(Seam::CollisionModel, &report);
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
    stages::reach(stages::Stage::WorldBuilt)?;
    // The player's own, and reported by `livery::load` beside the hull it was
    // read off. Sparks are slot 0's today - `oag_fx::sparks` triggers off
    // the player's contacts alone - so this takes slot 0's locators rather than
    // carrying eight sets nothing reads.
    let collision_fx = liveries[0].collision_fx.clone();
    let absorb_burst = super::absorb::absorb_burst_for(craft_title);
    let absorb_anchors = super::absorb::anchors(absorb_burst, &liveries);
    let mut effects = psys::Library::new();
    let particle_laws = title.looks.particle_laws.applies(archives.layout.platform);
    let psp_clip = title
        .looks
        .psp_particle_clip
        .applies(archives.layout.platform);
    let circuit_weather = vex_geometry
        && title
            .looks
            .circuit_weather
            .applies(archives.layout.platform);

    seams.mark(Seam::Effects, &report);
    for name in craft_title.effects.names_on(archives.layout.platform) {
        let dir = title.race.effect_dir_for(&track);
        let walk = title.effects.colour_walk_for(dir);
        // The law of the title whose archive the file is in, like the walk: its cap word is
        // only that title's record's, and so are the flipbook's frame bytes and size bounds.
        let near = title.effects.near_law;
        match listed_particle_effect(
            &mut archives,
            dir,
            craft_title.effects,
            name,
            particle_laws
                || (circuit_weather && oag_title::engine_effects::WEATHER_EFFECTS.contains(&name)),
            walk,
            title.effects,
        ) {
            Ok((mut effect, note)) => {
                if let Some(law) = near {
                    effect.with_near_law(psys::NearLaw {
                        cull_depth: law.cull_depth,
                        fade_depth: law.fade_depth,
                        template_cap: law.template_cap.map(|(cap, _)| cap),
                    });
                }
                if psp_clip {
                    effect.with_psp_clip();
                }
                report.push(note);
                effects.insert(name, effect);
                report.extend(super::assets::sheet_note(name, &effects));
            }
            Err(why) => report.push(format!("{why} - {name} will not be drawn")),
        }
    }
    // The ribbon's texture is a title axis, not a constant: Pulse and Pure name
    // one, HD authors a template whose material names its own - craft content,
    // hence `craft_title`. See `oag_title::exhaust::Exhaust`.
    let (noise, trail_blend, trail_shape) = assets::trail_texture(
        craft_of(&mut craft, &mut archives),
        craft_title,
        &mut report,
    );
    // Lent before the mods' effects, so no mod sprite moves; a no-op unless `graphics.trail_hits`.
    // Redrawn in the colour of this race's own ribbon, texture and layer law.
    let ps2_trail = title.looks.ps2_trail.applies(archives.layout.platform);
    let ribbon_hue = oag_fx::exhaust::TrailLaw::for_ps2(ps2_trail).ribbon_colour(
        noise
            .as_ref()
            .map_or([1.0; 3], oag_fx::exhaust::FlareTexture::mean_rgb),
    );
    let lent_trail_hit =
        trail_hits::lend(options, craft_title, ribbon_hue, &mut effects, &mut report);
    let mods = mod_assets::ModAssets::new(&extension_sources, title, options);
    let (mod_shaders, mod_effects) =
        mods.shaders_and_effects(&mut archives, &mut effects, &mut report);
    let handles = trail_hits::gate(
        options,
        craft_title,
        super::EffectHandles::resolve(craft_title, &effects)
            .with_own(mod_effects)
            .with_shaders(mod_shaders)
            .lending(Trigger::TrailHitship, lent_trail_hit),
    );
    let handles = own_rockets::load(options, craft_title, &mut archives, &mut effects)
        .lend(handles, &mut report);

    stages::checkpoint()?;
    let vr_fx = oag_sound::sfx::circuit_manifest(&mut archives, &track)
        .is_some_and(|manifest| manifest.vr_fx_on());
    let (mut sounds, announcer, class_announcer) = audio::banks_and_announcers(
        &mut archives,
        title,
        options.mode,
        vr_fx,
        slot_teams,
        options.no_effects,
        &mut report,
    );
    mods.sounds(&mut sounds, &mut report);
    stages::checkpoint()?;
    let rumble = crate::rumble_library::load(&mut archives, &mut report);
    let track_emitters = audio::track_emitters(
        &mut archives,
        title.race.sounds,
        &track,
        &track_blob,
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
             flare's chromatic fringe and occlusion query (its 4:1 quad, distance \
             fade, cos^32 highlight and alpha walk draw by the traced law, on every \
             craft but the viewing player's - exhaust::hd::Sprite). The flame \
             surface's Speed*time scroll draws now: time is engine shader parameter \
             slot 0, a global seconds clock (renderer.md)"
                .into(),
        );
        report.push(
            "a craft flying into a trail sparks: WO_TRAIL_HITSHIP, red variant when the \
             ribbon's owner is Fury-skinned, spawned every tick on the intruder's hull \
             anchor nearest the contact and carrying half its velocity. The consumer, \
             spawner, colour select, rate and velocity are read; the geometric test is \
             this engine's - RaceView::advance_trail_hits names it"
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

    // The LeachBeam ribbon's own texture - see `assets::leach_beam_texture`.
    // A table that failed to load keeps asking, as it always did.
    let leach_authored = weapons.as_ref().is_none_or(|w| w.leach_beam().is_some());
    let leach_beam_texture = assets::leach_beam_texture(
        craft_of(&mut craft, &mut archives),
        wm.leachbeam_ball.is_some(),
        leach_authored,
        &mut report,
    );

    let magstrip_wake = magstrip_wake::load(
        craft_of(&mut craft, &mut archives),
        (wm, craft_title, options),
        &liveries,
        &mut report,
    );

    // The plugin list is the craft's title's, and an empty one is a real
    // answer: a title whose front end is unrecovered declares no languages, so
    // the HUD draws its captions as their own `idstring` keys and says so, as
    // it already did for a source whose plugins would not parse.
    let language_plugins = craft_title
        .front_end
        .map_or::<&[&str], _>(&[], |front_end| front_end.language_plugins);
    // **The HUD follows the craft**, art included - `load_hud` reads its
    // atlas, font and sight art out of whichever `Archives` it is given.
    // **`options.language`, not `None`** - see that field's own doc for the
    // bug this closes: every launch site now sets it from the live
    // `settings::Settings::language` before `load` ever runs.
    let mod_icons = crate::extensions::mod_icons(
        &extension_sources,
        &title.slug(),
        options.foreign_track.as_deref(),
        &mut report,
    );
    let hud = load_hud(
        craft_of(&mut craft, &mut archives),
        craft_title,
        options.mode,
        language_plugins,
        options.language.as_deref(),
        options,
        &track,
        crate::hud::SplitViews::of(options),
        &mod_icons,
        &mut report,
    );

    // The countdown's own `<Mode3D><Model>` mesh - see `load::countdown`.
    let countdown_model = countdown::model(&hud, &mut archives, &mut report);
    let medal_models = countdown::medal_models(&hud, &mut archives, &mut report);
    stages::reach(stages::Stage::CraftsBuilt)?;
    let circuit_fx = vex_geometry
        && title
            .looks
            .circuit_effects
            .applies(archives.layout.platform);
    let road_ripple = vex_geometry && title.looks.road_ripple.applies(archives.layout.platform);
    let pre_race = intro::rule(title, &archives).filter(|_| !ribbon);
    let hull_lights = vex_geometry && title.looks.hull_lights.applies(archives.layout.platform);
    let psp_glow_mask = vex_geometry && title.looks.psp_glow_mask.applies(archives.layout.platform);
    let track_module = modules.has_track_module();
    let sight = super::Sighting::of(&hud);
    debug_assert_eq!(
        (
            sight.lock_screen,
            sight.leach_law,
            sight.chase_gain,
            sight.span
        ),
        (
            setup.sight.lock_screen,
            setup.sight.leach_law,
            setup.sight.chase_gain,
            setup.sight.span
        ),
        "the HUD's lock grid and law are the ones the simulation was given"
    );

    seams.mark(Seam::Flyby, &report);
    let cockpit_hit_noise = weapon_models::load_cockpit_hit(&mut archives, wm, &mut report);
    let destroy_stations = super::destroy_camera::stations(
        circuit_fx,
        &title.destroyed.value.camera,
        &track_blob,
        &mut report,
    );
    let intro_camera = intro::read(&mut archives, &track, options, pre_race, &mut report);
    let intro_voice = intro::read_voice(&mut archives, title, &track, pre_race, &mut report);

    seams.mark(Seam::Finish, &report);
    let mut now_playing = crate::now_playing::View::read(&mut archives, title, &hud, &mut report);
    if let Some(song) = options.hud.as_ref().and_then(|hud| hud.song.as_ref()) {
        match now_playing.as_mut() {
            Some(view) => view.pin(song),
            None => {
                report.push("now playing: --hud-song given, but the title has no display".into())
            }
        }
    }
    if options
        .hud
        .as_ref()
        .is_some_and(|hud| hud.head_to_head.is_some())
        && let Some(gap) = crate::hud::head_to_head_pin_gap(hud.layout.as_ref())
    {
        log::warn!("--hud-head-to-head: {gap}");
        report.push(format!("--hud-head-to-head: {gap}"));
    }
    let demo_overlay = intro::read_demo_screen(&mut archives, title, options, &hud, &mut report);
    let track_panel = intro::read_panel(
        &mut archives,
        title,
        &track,
        pre_race,
        options.language.as_deref(),
        &mut report,
    );
    let clouds = cloud_layer(&mut archives, &track_blob, vex_geometry, &mut report);
    let cannon_quad_textures = weapon_models::cannon_quads(&mut archives, wm, &mut report);
    let ghost_static = weapon_models::load_ghost_static(&mut archives, wm, &mut report);
    let view = ViewSetup {
        trail_sparks: options.trail_sparks,
        request: Some(options.replay_setup_on(&track)),
        campaign_event: false,
        score_pass: None,
        hd_trail: trail_shape.is_some().then_some(hd_trail_red),
        ps2_trail,
        shield_targets: roster::shield_targets(
            options,
            craft_title,
            craft_of(&mut craft, &mut archives).layout.platform,
            (&liveries, shield_cockpit_alpha),
        ),
        shield_palette: roster::shield_palette(
            options,
            craft_title,
            craft_of(&mut craft, &mut archives).layout.platform,
        ),
        hd_plasma_blast: wm.plasma_blast_hd.is_some(),
        plasma_blast_law: wm.plasma_blast_pulse.map(|blast| blast.law),
        disruptor_halo_law: wm.disruptor_halo,
        disruptor_craft_law: wm.disruptor_craft,
        hd_bomb_blast: wm.bomb_blast_hd.is_some(),
        bomb_shockwave_fades: wm.bomb_blast_pulse.is_some_and(|b| b.shockwave_fades),
        hd_missile_blast: wm.missile_blast_hd.is_some(),
        detonator_cannon_lights: options.mode == Mode::Detonator
            && craft_title
                .race
                .detonator
                .is_some_and(|detonator| detonator.cannon_lights),
        detonator_rounds_as_cannon: options.mode == Mode::Detonator
            && craft_title
                .race
                .detonator
                .is_some_and(|detonator| detonator.objects.is_none()),
        detonator_emp_repulsor: options.mode == Mode::Detonator
            && craft_title
                .race
                .detonator
                .is_some_and(|detonator| detonator.emp_borrows_repulsor),
        detonator_score_popups: options.mode == Mode::Detonator
            && craft_title
                .race
                .detonator
                .is_some_and(|detonator| detonator.floats_score.value),
        detonator_emp_light: options.mode == Mode::Detonator
            && craft_title
                .race
                .detonator
                .is_some_and(|detonator| detonator.emp_light),
        detonator_mine_reach: mine_model
            .as_ref()
            .filter(|_| options.mode == Mode::Detonator && detonator_objects.is_none())
            .map(crate::detonator_look::model_reach),
        detonator_mine_spin: craft_title
            .race
            .detonator
            .filter(|_| options.mode == Mode::Detonator)
            .and_then(|detonator| detonator.mine_spin),
        detonator_ammo_gauge: craft_title
            .race
            .detonator
            .and_then(|detonator| detonator.ammo_gauge)
            .filter(|_| options.mode == Mode::Detonator),
        detonator_hull_charge: craft_title
            .race
            .detonator
            .and_then(|detonator| detonator.hull_charge)
            .filter(|_| options.mode == Mode::Detonator),
        pulse_laid_pose: craft_title.looks.laid_pose.applies_everywhere(),
        laid_pose_scaled: craft_title.looks.laid_pose_scaled.applies_everywhere(),
        screen_flash: extents,
        quake_flash: craft_title.effects.quake_look.flash,
        craft_blast_carry: craft_title.effects.craft_blast_carry.is_some(),
        absorb_burst,
        absorb_anchors,
        hit_spark_anchors: super::hit_sparks::anchors(craft_title, &liveries),
        weapon_spark_anchors: super::hit_sparks::weapon_anchors(craft_title, &liveries),
        cockpit_hit_noise,
        wreck_anchors: super::wreck_fx::anchors(craft_title, &liveries),
        zone_end: liveries.first().and_then(|livery| livery.zone_end.clone()),
        ship_debris: liveries
            .first()
            .and_then(|livery| livery.ship_debris.clone()),
        ship_debris_spec: craft_title.destroyed.value.ship_debris,
        // Pulse's ship shockwave goes with its blast; HD's is thrown on a different edge and its
        // shockwave is unread.
        wreck_shockwave: craft_title
            .effect_on(oag_title::Trigger::WreckExplosion)
            .is_some()
            && craft_title.destroyed.value.blast_at == oag_title::destroyed::BlastAt::DwellEnd,
        destroy_stations,
        intro_camera,
        intro_voice,
        sight,
        magstrip_wake: magstrip_wake.anchors,
        magstrip_pob: wm.magstrip_pob,
        leach_strip: magstrip_wake.leach_anchors,
        missile_trail: magstrip_wake.missile_trail,
        wingtip: magstrip_wake.wingtip,
        airbrake_graphics,
        chase_close,
        internal,
        nozzles: liveries.iter().map(|livery| livery.nozzle).collect(),
        nozzle_axes: liveries.iter().map(|livery| livery.nozzle_axis).collect(),
        engine_lights: engine_light::per_slot(&liveries, title, &track),
        spu_vertex_lights,
        hull_points: oag_mesh::mesh_render::HullPointLights::none(),
        spark_anchors: liveries
            .iter()
            .map(|livery| livery.collision_fx.iter().map(|a| a.position).collect())
            .collect(),
        collision_fx,
        scenery_fx: Default::default(),
        effects,
        handles,
        sounds,
        track_emitters,
        announcer,
        class_announcer,
        rumble,
        zoom_ring: title.race.zoom_ring.or(borrowed_zoom),
        zoom_borrowed: borrowed_zoom.is_some(),
        mine_blast_colours: title
            .effects
            .mine_blast_recolour
            .and(zone_grade.as_ref())
            .map(crate::zone_grade::ZoneGrade::detonator_mine_rgba8)
            .unwrap_or_default(),
        capture_hud: options.hud.clone(),
        hud_waits_for_go: title
            .race
            .hud_waits_for_go
            .value
            .contains(&options.mode.name()),
        hull_shake: title.race.hull_shake.map(|shake| shake.value),
        hull_draw_factor: crate::camera::hull_draw_factor(title, options.mode),
        now_playing,
        world_clock_offset: options.world_clock_offset,
        gantry_seconds: options.gantry_seconds,
        flame_seconds: options.flame_seconds,
        camera_override: options.camera,
        fov_override: options.fov_override,
        track_sections: visibility.as_ref().and_then(|v| v.pvs().cloned()),
    };
    let mut loaded = Loaded {
        started: None,
        title,
        circuit: track.clone(),
        platform: archives.layout.platform,
        setup,
        view,
        modules,
        demo_overlay,
        hud,
        track_panel,
        track_stats,
        track_model,
        behind_glass,
        billboards: crate::adverts::Billboards {
            gantry,
            adverts,
            zone_equaliser,
            zone_equaliser_bars: title.adverts.is_some_and(|spec| spec.zone_equaliser_bars),
        },
        collision_model,
        sky_model,
        pad_model,
        weapon_pad_model,
        fog_volumes,
        clouds,
        light,
        circuit_lighting: options.circuit_lighting,
        probes,
        authored_fog,
        hd_bloom: hd_bloom.map(|p| p.with_eight_bit(options.scene_precision.is_eight_bit())),
        omega_tonemap,
        wo_composite,
        zone_grade,
        liveries,
        rocket_model,
        cannon_model,
        mine_model,
        bomb_model,
        plasma_blast_models,
        bomb_blast_models,
        leach_ball_model,
        shield_cockpit: roster::own_cockpit(shield_cockpit, options, craft_title),
        countdown_model,
        medal_models,
        visibility,
        flare,
        leach_beam_texture,
        ribbon_textures: magstrip_wake.textures,
        noise,
        trail_blend,
        trail_shape,
        cannon_quad_textures,
        ghost_static,
        shadows,
        // Overwritten by `load_event` on the one path that has an event
        // name to carry - see `Loaded::campaign_2048_event`'s own doc.
        campaign_2048_event: None,
        ripples: ripple::Ripples::default(),
        report,
    };
    let track_shine = {
        let platform = archives.layout.platform;
        let track_shine =
            vex_geometry && options.track_shine && title.looks.track_shine.applies(platform);
        if !track_shine {
            loaded.track_model.shine_draws.clear();
        } else if !title.looks.track_shine_weighted.applies(platform) {
            oag_mesh::mesh::shine_pass::unweight(&mut loaded.track_model);
        }
        track_shine
    };
    caustic::attach(
        &mut loaded,
        &mut archives,
        title,
        vex_geometry && options.water != oag_display::display::Water::Off,
    );
    pulse_psp::finish(
        &mut loaded,
        (circuit_fx, circuit_weather, road_ripple, hull_lights),
        psp_glow_mask,
        track_shine,
        &track_blob,
        (&mut archives, &track, title),
    );
    {
        loaded.view.scenery_fx.bloom_under_hud = psp_glow_mask
            && title
                .looks
                .bloom_under_hud
                .applies(archives.layout.platform);
    }
    if vex_geometry && title.looks.lens_flare.applies(archives.layout.platform) {
        loaded.view.scenery_fx.lens_flare =
            lens_flare::read(&track_blob, &mut loaded.report).map(Box::new);
    }
    if track_module && loaded.view.scenery_fx.weather.is_none() {
        pulse_psp::place_driven_weather(&mut loaded, title, &mut archives);
    }
    if foreign_art.lens_flare.is_some() {
        loaded.view.scenery_fx.lens_flare = foreign_art.lens_flare.map(Box::new);
    }
    {
        loaded.view.scenery_fx.sun = foreign_art.sun_shadow;
        (
            loaded.view.scenery_fx.sun_turn,
            loaded.view.scenery_fx.flare_turns,
        ) = (foreign_art.sun_turn, foreign_art.flare_turns);
        if let Some(hull) = foreign_art.hull
            && title.looks.hull_lights.applies(archives.layout.platform)
        {
            // A track file's own light through the host title's hull law
            // (ADR-0076 decision 12); HD-family hosts are not wired.
            loaded.light.hull = hull;
            loaded
                .report
                .push("hull lights: the track file's ambient and sun, the host's GE rig".into());
        }
    }
    lod_bias::finish(&mut loaded, title.looks.material_lod_levels());
    pulse_ps2::finish(
        &mut loaded,
        vex_geometry && pulse_ps2::is_pulse_ps2(title, &archives),
    );
    loaded.report = super::seams::weave(sim_report, &sim_seams, loaded.report, &seams);
    closing_report::finish(&mut loaded.report, &mut archives, craft.as_mut());
    Ok(loaded)
}
