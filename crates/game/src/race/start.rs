//! [`Race::start`]: a loaded race turned into a running one - the world, the
//! grid, the camera and the drivers on it.
//!
//! Split out of `race.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change.

use super::*;
use log::{info, warn};

impl Race {
    /// Starts a race: one ship, on the racing line, at the start of the spline.
    ///
    /// The mass is copied into the rigid body because the force law reads
    /// `handling.physical.mass` while the integrator reads `body.mass`. They are one
    /// quantity stored twice and keeping them equal is this layer's job -
    /// `oag_physics` deliberately does not do it, so that a mismatch stays visible.
    ///
    /// The inertia comes from [`box_inertia`]; see the module documentation and that
    /// function for why a derived tensor replaced the `(1, 1, 1)` placeholder.
    #[must_use]
    pub fn start(setup: Setup) -> Self {
        let Setup {
            mode,
            eliminator_kill_target,
            laps_override,
            difficulty,
            opponents,
            trail_sparks,
            seed,
            zone,
            spline,
            course,
            start_position,
            collision,
            handling,
            chase,
            chase_close,
            internal,
            nozzles,
            spark_anchors,
            collision_fx,
            effects,
            sounds,
            track_emitters,
            announcer,
            class_announcer,
            zone_stages,
            speedup_pads,
            weapon_pads,
            weapons,
            weapon_pad_refresh,
            class,
            class_gravity_scale,
            pose_override,
            camera_override,
            hd_trail,
            ..
        } = setup;

        // The mode gate, applied once at construction rather than on every tick,
        // because that is where the original applies it: `World_CollectNodeLists`
        // zeroes the trigger list's own count at track load, so a weapons-off
        // race has no weapon pads to walk rather than a walk that decides
        // nothing. Dropping the volumes here also makes it impossible for a
        // later change to reach them by accident.
        let weapon_pads = if mode.weapons_enabled() {
            weapon_pads
        } else {
            Vec::new()
        };

        let mut world = World::new(seed);
        // The lap count is per speed class, not per mode - the campaign's 236
        // authored `PI_Cell` records carry `laps` as exactly 3 Venom, 4 Flash,
        // 4 Rapier, 5 Phantom, confidence 90 (`race-campaign.md`). `oag_race`
        // matches on the class; resolving the disc's own spelling of it onto
        // that enum is this layer's job, because this is the layer that already
        // carries the name.
        //
        // **An unresolved rung falls back to Venom's count and says so.** The
        // one case that reaches it is Wipeout Pure's `VECTOR`, the rung it
        // authors below Venom - `oag_title::SpeedClasses::VECTOR` explains at
        // length why nothing grows a fifth enum variant for it. No Pure
        // campaign census has been read, so there is no measured lap count for
        // that rung at all; taking the slowest measured one is **chosen, not
        // measured**, and carries no confidence score. The warning is the point
        // - a silent Venom is how three of four classes raced short in the
        // first place.
        let lap_class = oag_race::SpeedClass::from_name(&class).unwrap_or_else(|| {
            warn!(
                "speed class {class:?} is not one of the four the lap census covers - \
                 lap count falls back to {venom}'s, which is chosen, not measured",
                venom = oag_race::SpeedClass::Venom,
            );
            oag_race::SpeedClass::Venom
        });
        world.race = RaceState::new(mode, lap_class);
        // The cell's own `laps` wins over the per-class table, for the two
        // modes where a lap count ends the race at all - see
        // `Options::laps_override`'s own doc for why `SpeedLap`/`Zone` must
        // never reach here despite authoring a `laps` attribute too.
        if let Some(laps) = laps_override {
            world.race.laps_target = Some(laps);
        }
        let ship = &mut world.ships[0];
        ship.active = true;
        ship.handling = handling;
        ship.physics.body.mass = handling.physical.mass;
        ship.physics.body.inertia = box_inertia();
        // The pool starts full, which is `Ship_ResetShield` (`0x0883dd24`) - the
        // only thing on the disc that sets it outright. Wall contact spends it
        // from here (`oag_physics::damage`), and Zone's perfect-zone recharge
        // adds back to it.
        oag_physics::damage::reset(&mut ship.physics, &handling.dimensions);
        // Whether this race fields the full eight-slot grid at all: the mode's
        // own answer, or the `--opponents` escape hatch that measurement's own
        // ground-truth tests use to force it. Read once, because both the
        // player's own pose below and the opponent loop further down have to
        // agree on it.
        let full_grid = start_position.is_some() && (mode.has_opponents() || opponents);
        // The override wins outright rather than being an offset from the grid
        // slot: it exists to put the craft at a position read off somewhere
        // else, and anything added to that would make the two disagree.
        let base = spawn_pose(&spline, start_position.as_ref(), &collision, &handling);
        // A solo mode (time trial, speed lap, Zone) grids the player alone, and
        // alone they go on slot 1, not slot 8.
        //
        // `base` above is the authored `Start Position` node's own pose, and
        // `grid.md` measures that node as **slot 8** - the back of an eight-car
        // grid, confirmed against a live single-race capture. That is right for
        // a full grid, where the player really does occupy slot 8 in the
        // original. It is not evidence for where a *solo* run starts: nothing
        // in this project has read whether `Race_SpawnGrid` even runs for a
        // field of one, and its own "a short grid packs to the back" compaction
        // rule (`grid.md`) would say slot 8 there too if it does.
        //
        // What the reference capture does say: `docs/formats/track.md
        // #start-position` reads a live time trial start pose on `16_Track`
        // that lands 137.9 units *ahead* of the authored node and 22.4 to its
        // left. That is slot 1's own offset from the node - `7 *
        // GRID_ROW_PITCH` is 138.53, and slot 1 is the odd slot that takes
        // `GRID_COLUMN_OFFSET` to the left - to within the couple of units the
        // eight-slot grid's own per-slot residual already carries. So slot 1 is
        // measured for a time trial, not merely picked; speed lap and Zone are
        // extended to it by the same mode-sharing `race-modes.md` already
        // documents rather than a capture of their own, and the load report
        // says so.
        //
        // Gated on an authored slot existing at all, same as `full_grid` is:
        // with no `Start Position` node, `base` is [`spawn_pose`]'s spline
        // fallback, not an anchor a grid offset means anything relative to -
        // walking `GRID_ROW_PITCH` forward of *that* would invent a slot from
        // a pose nobody authored, which is exactly what a track with no node
        // already opts out of for the seven opponents below.
        let solo_slot_one = (!full_grid && start_position.is_some())
            .then_some(base)
            .flatten()
            .map(|base| grid_poses(base, &spline, &collision, spawn_height(&handling))[0]);
        if let Some(pose) = pose_override.or(solo_slot_one).or(base) {
            ship.place_at(pose);
        }

        // The rest of the grid. **The player keeps slot 0 of the array and slot
        // 8 of the grid**, which is not a coincidence: the array index is what
        // every rule, camera and HUD in this file means by "the ship", and slot
        // 8 is where the original puts the local player and where the authored
        // `Start Position` node is. So the seven opponents are ahead of the
        // player, which is what a Pulse grid looks like. That is the full-grid
        // case only - see `solo_slot_one` above for the other one.
        //
        // Each one gets a driver here and its own `Environment`, standing and
        // exhaust as the race runs - see [`Self::step_opponents`],
        // [`Self::update_standings`] and [`Self::advance_exhausts`].
        //
        // **Gated on the mode first of all.** `mode.has_opponents()` is `false`
        // for a time trial, a speed lap and a Zone race - measured live,
        // confirmed on the running original, AI DIFFICULTY greyed to N/A the
        // same way WEAPONS is - so those three spawn the player alone, the way
        // the original does, and a single race fields the grid. `opponents` is
        // the verification override [`Setup::opponents`] documents.
        //
        // Skipped entirely when the caller overrode the pose: an override exists
        // to put *one* craft somewhere specific, and surrounding it with a grid
        // built from a different anchor would put opponents through the scenery.
        //
        // Also skipped when the track authors no `Start Position`. The whole
        // layout is offsets from that node, which is slot 8; anchored on the
        // spline fallback instead, the offsets would be measured from a pose
        // nobody authored and the seven opponents would be a guess wearing the
        // shape of a measurement.
        let mut ai_pilots = [oag_ai::Pilot::BALANCED; oag_gameplay::MAX_SHIPS];
        let order = ai_order(&spline, course.as_ref());
        let line = racing_line(&spline, &order);
        let rescue_distance = spline.max_half_width() * RESCUE_HALF_WIDTHS;
        let player_rescue_distance = spline.max_half_width() * PLAYER_RESCUE_HALF_WIDTHS;
        // Where the player is placed, so the first recovery before they have
        // driven anywhere is onto the grid rather than onto sample zero - which
        // on a circuit whose start line is sample 2,791 is a different piece of
        // track entirely. See `Race::last_on_track`.
        let last_on_track = spline
            .nearest(world.ships[0].physics.body.position)
            .and_then(|(index, _, _)| u32::try_from(index).ok())
            .unwrap_or(0);
        let mut pilot_names: [String; oag_gameplay::MAX_SHIPS] = Default::default();
        // The built-ins, plus whatever the player has authored. A directory
        // that cannot be read is not a reason to refuse to race: the built-ins
        // are always there, and the error is reported rather than fatal.
        let roster = match crate::pilots::load() {
            Ok(roster) => roster,
            Err(e) => {
                warn!("pilots: {e:#} - racing with the built-in four");
                crate::pilots::Roster::built_in()
            }
        };
        if pose_override.is_none()
            && full_grid
            && let Some(base) = base
        {
            let poses = grid_poses(base, &spline, &collision, spawn_height(&handling));
            for (index, pose) in poses.iter().enumerate().take(GRID_SLOTS as usize - 1) {
                let opponent = &mut world.ships[index + 1];
                opponent.active = true;
                opponent.handling = handling;
                opponent.physics.body.mass = handling.physical.mass;
                opponent.physics.body.inertia = box_inertia();
                oag_physics::damage::reset(&mut opponent.physics, &handling.dimensions);
                // **Eight drivers, not one driver eight times.** The seed is
                // the race's own and the craft's slot, so the same race fields
                // the same eight characters on every replay and the next race
                // fields eight others. Nothing here reads a clock or the
                // world's generator - see `oag_ai::Driver::for_slot`.
                let slot = index as u32 + 1;
                opponent.driver = oag_ai::Driver::for_slot(seed, slot);
                // **Where on the line this craft actually is**, found once with
                // a search over the whole line rather than left at zero.
                //
                // `Driver::drive` searches a 48-sample *window* around the last
                // index, deliberately, so a track that passes near itself
                // cannot make a craft latch onto a stacked section. That window
                // is also why the starting value matters: a driver that begins
                // at sample zero when the grid is at sample 2,500 never finds
                // itself, and steers at the piece of circuit it thinks it is
                // on. Measured before this line existed - on eight of the
                // disc's twelve circuits the field sat between 280 and 10,862
                // units from its own racing line, and only the two whose start
                // line happens to sit near sample zero worked.
                opponent.driver.index = line.nearest(pose.position, 0, line.len()) as u32;
                // **And the character it is a variation on**, drawn from the
                // race seed through a stream of its own. Sharing the driver
                // seed's would tie a craft's pilot to its personality, so the
                // aggressive slot would always be the one that also drew a high
                // commitment - see `oag_ai::pilot_for_slot`.
                let choice = oag_ai::pilot_for_slot(seed, slot, roster.len());
                let entry = &roster.entries()[choice as usize];
                // The level scales how a pilot treats other craft and leaves
                // the line it drives alone, so a shy pilot at novice is still
                // shy rather than generically slow.
                ai_pilots[slot as usize] = difficulty.temper(&entry.pilot);
                pilot_names[slot as usize].clone_from(&entry.name);
                // **The digest, into the world snapshot.** A pilot can come out
                // of the player's own config directory, so two machines running
                // "the same race" with different files must not agree on the
                // hash - see `oag_ai::Driver::pilot`.
                opponent.driver.pilot = entry.digest;
                opponent.place_at(*pose);
            }
            world.ship_count = GRID_SLOTS;
            // **What was loaded, and who is flying what, once per race.** The
            // digests are the point of the first line: thirty-two bits cannot
            // be inverted, so when two machines disagree on a world hash this
            // is the only thing that says *which* pilot file differs. The
            // second line is for the player, who wants to know the grid they
            // are about to race. A `*` marks a pilot that came from a file.
            info!("pilots loaded: {}", roster.summary());
            info!("ai difficulty: {}", difficulty.name());
            let grid: Vec<String> = (1..GRID_SLOTS as usize)
                .map(|slot| format!("{slot}:{}", pilot_names[slot]))
                .collect();
            info!("pilots on the grid: {}", grid.join(" "));
        } else {
            world.ship_count = 1;
        }

        let camera = Chase::snapped(target_of(&world.ships[0]), &chase);

        let race = Self {
            sim: RaceSim {
                racing_line: line,
                ai_order: order,
                // **Degraded from the measured tuning**, never boosted toward it -
                // see `oag_ai::Difficulty`. At the top level this is the
                // measurement unchanged.
                ai_tuning: difficulty.tune(&oag_ai::Tuning::default()),
                ai_pilots,
                // `--autopilot-skill` sets this after `Self::start` returns - see
                // `Self::set_autopilot_tuning`. Every race starts with the two
                // tunings agreeing.
                autopilot_tuning: None,
                world,
                collision,
                spline,
                course,
                // Only Zone reads these, so the other two modes carry `None` and the
                // engine keeps its ordinary throttle path.
                zone: zone.filter(|_| mode == Mode::Zone),
                // ADR-0007: 60 Hz, from the clock rather than from a literal, so there
                // is one place the rate is decided.
                dt: TickClock::new(TickRate::DEFAULT).rate().dt(),
                respawn_cooldown: [0; oag_gameplay::MAX_SHIPS],
                respawns_in_a_row: [0; oag_gameplay::MAX_SHIPS],
                respawn_disabled: [false; oag_gameplay::MAX_SHIPS],
                respawns: [0; oag_gameplay::MAX_SHIPS],
                eliminator_kill_target: eliminator_kill_target
                    .unwrap_or(Mode::ELIMINATOR_KILL_TARGET_DEFAULT),
                last_damager: [None; oag_gameplay::MAX_SHIPS],
                eliminator_respawn_timer: [0.0; oag_gameplay::MAX_SHIPS],
                rolls_armed: [0; oag_gameplay::MAX_SHIPS],
                rolls_spent: [0.0; oag_gameplay::MAX_SHIPS],
                wall_contact_ticks: [0; oag_gameplay::MAX_SHIPS],
                wall_inbound_ticks: [0; oag_gameplay::MAX_SHIPS],
                wall_damage: [0.0; oag_gameplay::MAX_SHIPS],
                wall_racing_ticks: [0; oag_gameplay::MAX_SHIPS],
                lost_ticks: [0; oag_gameplay::MAX_SHIPS],
                stalled_ticks: [0; oag_gameplay::MAX_SHIPS],
                rescue_distance,
                player_rescue_distance,
                last_on_track,
                zone_stages,
                autopilot: false,
                cues: Vec::new(),
                announcements: Vec::new(),
                class_announcements: Vec::new(),
                contact_cue_cooldown: [0.0; oag_gameplay::MAX_SHIPS],
                // Every pad starts due for a real test. `Pad_Bind` zeroes the same
                // cache at load, so the first tick measures rather than trusting a
                // distance nothing has computed yet.
                pad_distance: std::array::from_fn(|_| vec![0.0; speedup_pads.len()]),
                speedup_pads,
                // The same broadphase, and the same "due for a real test" start.
                weapon_pad_distance: std::array::from_fn(|_| vec![0.0; weapon_pads.len()]),
                // Every pad starts collectable. The original's `+0x1a0` is zero
                // until something stamps it, and nothing stamps it at load.
                weapon_pad_refresh_left: vec![0.0; weapon_pads.len()],
                weapon_pads,
                weapon_pad_current: [None; MAX_SHIPS],
                weapon_pad_refresh,
                weapons,
                class,
                class_gravity_scale,
                pad_current: [None; MAX_SHIPS],
                pad_previous_position: [None; MAX_SHIPS],
                scheme: ControlScheme::default(),
            },
            view: RaceView {
                chase_params: chase,
                chase_far: chase,
                chase_close,
                internal_params: internal,
                // The default rather than the settings file's value, for the reason
                // `set_boost_fov_kick` gives: `Setup` is what a *headless* race
                // needs and a display preference must not be in front of a caller
                // with no screen. `set_camera_view` is what applies the player's.
                camera_view: oag_display::display::CameraView::default(),
                camera,
                shake: oag_render::camera::shake::Shake::new(),
                shake_rng: Rng::new(SHAKE_SEED),
                camera_override,
                // Cold, then snapped on the first tick. A race starts from a standing
                // start with no thrust, so there is nothing to snap *to* here.
                //
                // A cold `Exhaust` has an empty trail ring and `Exhaust::trail_ready`
                // gates the ribbon on a *full* one, so no craft draws a ribbon across
                // the track from the origin to its grid slot on the opening ticks.
                exhaust: [Exhaust::new(); MAX_SHIPS],
                exhaust_rng: std::array::from_fn(|slot| Rng::new(exhaust_seed(slot))),
                // Cold and empty for the same standing-start reason as `exhaust`
                // above: `hd::Tube::ready` gates on a full ring too.
                hd_trail: [exhaust::hd::Tube::new(); MAX_SHIPS],
                hd_trail_active: hd_trail.is_some(),
                hd_trail_red: hd_trail.unwrap_or([0.0; MAX_SHIPS]),
                hull_reach: [0.0; MAX_SHIPS],
                trail_sparks,
                trail_inside: [0; MAX_SHIPS],
                hd_flame: [exhaust::hd::Flame::new(); MAX_SHIPS],
                hd_sprite: [exhaust::hd::Sprite::new(); MAX_SHIPS],
                shield: [ShipShield::new(); MAX_SHIPS],
                nozzles,
                spark_anchors,
                collision_fx,
                sparks: psys::System::new(),
                effects,
                sounds,
                track_emitters,
                announcer,
                class_announcer,
                stage: psys::Stage::new(),
                engine_flare: [None; MAX_SHIPS],
                projectile_flare: [None; oag_gameplay::projectile::MAX_PROJECTILES],
                projectile_flare_orbit: [None; oag_gameplay::projectile::MAX_PROJECTILES],
                quake_effect: None,
                leach_beam_effect: None,
                leach_charge_effect: None,
                stage_rng: Rng::new(STAGE_SEED),
                sparks_ignitions: 0,
                sparks_rng: Rng::new(SPARKS_SEED),
                // No sync frame to be mid-scrape on, so the first tick's contact -
                // if any - is always read as a fresh impact.
                sparks_cooldown: 0.0,
                sparks_anchor: None,
                sparks_anchor_up: None,
                sparks_attached: false,
                results: None,
                shield_was_up: false,
                // `0.0`, not `1.0`: the pool is `0.0` until `<Misc>` loads (see
                // `Readout::shield_fraction`'s own zero-guard), and a full-pool
                // reading here would arm the flash on a false "drop" from 100%
                // to whatever the first real tick measures.
                shield_flash_prev: 0.0,
                shield_flash_timer: 0.0,
                sight: sight::Sight::default(),
                sight_state: sight::State::Absent,
                sight_fov: oag_display::display::Fov::AUTHORED,
                boost_kick: 0.0,
                boost_fov_kick: oag_display::display::BoostFovKick::DEFAULT,
                flaps: [0.0, 0.0],
                flap_graphics: setup.airbrake_graphics,
            },
        };
        // The free Turbo is granted at the release edge, not here - see
        // `Race::tick`'s own `COUNTDOWN_TICKS` check for the mode, full-slot
        // and missing-table gates it shares with every later grant. The
        // original never holds anything in the pickup slot through the
        // countdown; granting it at tick 0 put a HUD icon on screen during
        // those 272 ticks that the disc never draws.
        race
    }
}
