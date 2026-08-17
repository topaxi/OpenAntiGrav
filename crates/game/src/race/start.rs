//! [`Race::start`]: a loaded race turned into a running one - the world, the
//! grid, the camera and the drivers on it.
//!
//! Split out of `race.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change.

use super::*;

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
            difficulty,
            opponents,
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
            collision_fx,
            effects,
            speedup_pads,
            weapon_pads,
            weapons,
            weapon_pad_refresh,
            class,
            class_gravity_scale,
            pose_override,
            camera_override,
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
        world.race = RaceState::new(mode);
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
        // The override wins outright rather than being an offset from the grid
        // slot: it exists to put the craft at a position read off somewhere
        // else, and anything added to that would make the two disagree.
        let base = spawn_pose(&spline, start_position.as_ref(), &collision, &handling);
        if let Some(pose) = pose_override.or(base) {
            ship.place_at(pose);
        }

        // The rest of the grid. **The player keeps slot 0 of the array and slot
        // 8 of the grid**, which is not a coincidence: the array index is what
        // every rule, camera and HUD in this file means by "the ship", and slot
        // 8 is where the original puts the local player and where the authored
        // `Start Position` node is. So the seven opponents are ahead of the
        // player, which is what a Pulse grid looks like.
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
                eprintln!("pilots: {e:#} - racing with the built-in four");
                crate::pilots::Roster::built_in()
            }
        };
        if pose_override.is_none()
            && start_position.is_some()
            && (mode.has_opponents() || opponents)
            && let Some(base) = base
        {
            let poses = grid_poses(base, &collision, spawn_height(&handling));
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
            eprintln!("pilots loaded: {}", roster.summary());
            eprintln!("ai difficulty: {}", difficulty.name());
            let grid: Vec<String> = (1..GRID_SLOTS as usize)
                .map(|slot| format!("{slot}:{}", pilot_names[slot]))
                .collect();
            eprintln!("pilots on the grid: {}", grid.join(" "));
        } else {
            world.ship_count = 1;
        }

        let camera = Chase::snapped(target_of(&world.ships[0]), &chase);

        Self {
            racing_line: line,
            ai_order: order,
            // **Degraded from the measured tuning**, never boosted toward it -
            // see `oag_ai::Difficulty`. At the top level this is the
            // measurement unchanged.
            ai_tuning: difficulty.tune(&oag_ai::Tuning::default()),
            ai_pilots,
            world,
            collision,
            spline,
            course,
            // Only Zone reads these, so the other two modes carry `None` and the
            // engine keeps its ordinary throttle path.
            zone: zone.filter(|_| mode == Mode::Zone),
            chase_params: chase,
            chase_far: chase,
            chase_close,
            internal_params: internal,
            // The default rather than the settings file's value, for the reason
            // `set_boost_fov_kick` gives: `Setup` is what a *headless* race
            // needs and a display preference must not be in front of a caller
            // with no screen. `set_camera_view` is what applies the player's.
            view: crate::display::CameraView::default(),
            camera,
            camera_override,
            // ADR-0007: 60 Hz, from the clock rather than from a literal, so there
            // is one place the rate is decided.
            dt: TickClock::new(TickRate::DEFAULT).rate().dt(),
            respawn_cooldown: [0; oag_gameplay::MAX_SHIPS],
            respawns_in_a_row: [0; oag_gameplay::MAX_SHIPS],
            respawn_disabled: [false; oag_gameplay::MAX_SHIPS],
            respawns: [0; oag_gameplay::MAX_SHIPS],
            lost_ticks: [0; oag_gameplay::MAX_SHIPS],
            stalled_ticks: [0; oag_gameplay::MAX_SHIPS],
            rescue_distance,
            player_rescue_distance,
            last_on_track,
            // Cold, then snapped on the first tick. A race starts from a standing
            // start with no thrust, so there is nothing to snap *to* here.
            //
            // A cold `Exhaust` has an empty trail ring and `Exhaust::trail_ready`
            // gates the ribbon on a *full* one, so no craft draws a ribbon across
            // the track from the origin to its grid slot on the opening ticks.
            exhaust: [Exhaust::new(); MAX_SHIPS],
            exhaust_rng: std::array::from_fn(|slot| Rng::new(exhaust_seed(slot))),
            nozzles,
            collision_fx,
            sparks: psys::System::new(),
            effects,
            stage: psys::Stage::new(),
            engine_flare: [None; MAX_SHIPS],
            projectile_flare: [None; oag_gameplay::projectile::MAX_PROJECTILES],
            stage_rng: Rng::new(STAGE_SEED),
            sparks_ignitions: 0,
            sparks_rng: Rng::new(SPARKS_SEED),
            // No sync frame to be mid-scrape on, so the first tick's contact -
            // if any - is always read as a fresh impact.
            sparks_cooldown: 0.0,
            sparks_anchor: None,
            autopilot: false,
            results: None,
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
            boost_kick: 0.0,
            boost_fov_kick: crate::display::BoostFovKick::DEFAULT,
            flaps: [0.0, 0.0],
            flap_graphics: setup.airbrake_graphics,
            scheme: ControlScheme::default(),
        }
    }
}
