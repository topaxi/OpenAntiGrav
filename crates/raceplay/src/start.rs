//! [`Race::start`]: a loaded race turned into a running one - the world, the
//! grid, the camera and the drivers on it.
//!
//! Split out of `race.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change.

use super::*;
use log::{debug, info, warn};

impl Race {
    /// Starts a race: one ship, on the racing line, at the start of the spline.
    ///
    /// **It does not learn the speed plans.** They are this project's driver's
    /// (`oag_ai::plan`) and cost seconds a long circuit (HD's default: 3.0 to
    /// 3.7 s of a 5.2 to 5.9 s start, release; the per-hull adapted plans and
    /// the route plans on top). The head of [`Self::tick`] learns them
    /// ([`Self::ensure_speed_plans`]) only if something will read them, after
    /// the caller has had its say on whose driver flies
    /// ([`Self::use_original_ai`]); [`Self::speed_plan`] and [`Self::plan_of`]
    /// learn them first, so no reader sees a plan missing that was there.
    ///
    /// The mass is copied into the rigid body because the force law reads
    /// `handling.physical.mass` while the integrator reads `body.mass`. They are one
    /// quantity stored twice and keeping them equal is this layer's job -
    /// `oag_physics` deliberately does not do it, so that a mismatch stays visible.
    ///
    /// The inertia is the title's box tensor (`crate::launch_hover::craft_inertia`, Pulse's
    /// [`box_inertia`] for `None`); see that function for why a derived tensor is used at all.
    #[must_use]
    pub fn start(setup: Setup) -> Self {
        let Setup {
            mode,
            eliminator_kill_target,
            laps_override,
            weapons_override,
            difficulty,
            finished_thrust,
            original_tables,
            opponents,
            no_opponents,
            people,
            #[cfg(feature = "view")]
            trail_sparks,
            seed,
            zone,
            ai,
            spline,
            course,
            start_position,
            collision,
            handling,
            opponent_handling,
            chase,
            #[cfg(feature = "view")]
            chase_close,
            #[cfg(feature = "view")]
            internal,
            #[cfg(feature = "view")]
            nozzles,
            #[cfg(feature = "view")]
            engine_lights,
            #[cfg(feature = "view")]
            spu_vertex_lights,
            #[cfg(feature = "view")]
            hull_points,
            #[cfg(feature = "view")]
            spark_anchors,
            #[cfg(feature = "view")]
            collision_fx,
            #[cfg(feature = "view")]
            scenery_fx,
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
            zone_stages,
            countdown_voice,
            countdown_beat,
            launch_hover,
            hover_rig,
            craft_laws,
            autopilot_engine,
            opponent_autopilot,
            #[cfg(feature = "view")]
            zoom_ring,
            #[cfg(feature = "view")]
            zoom_borrowed,
            weapon_pad_glow,
            leach_drain,
            speedup_pads,
            weapon_pads,
            weapons,
            weapon_ai,
            allowed_weapons,
            weapon_pad_refresh,
            class,
            class_gravity_scale,
            start_boost,
            pilot_assist,
            wall_restitution,
            pose_override,
            field_poses,
            #[cfg(feature = "view")]
            capture_hud,
            #[cfg(feature = "view")]
            camera_override,
            #[cfg(feature = "view")]
            hd_trail,
            #[cfg(feature = "view")]
            ps2_trail,
            #[cfg(feature = "view")]
            shield_palette,
            #[cfg(feature = "view")]
            hd_plasma_blast,
            #[cfg(feature = "view")]
            hd_bomb_blast,
            #[cfg(feature = "view")]
            bomb_shockwave_fades,
            #[cfg(feature = "view")]
            hd_missile_blast,
            #[cfg(feature = "view")]
            pulse_laid_pose,
            #[cfg(feature = "view")]
            laid_pose_scaled,
            laid_from_rear,
            grid_frame_from_sample,
            #[cfg(feature = "view")]
            screen_flash,
            #[cfg(feature = "view")]
            quake_flash,
            #[cfg(feature = "view")]
            absorb_burst,
            #[cfg(feature = "view")]
            absorb_anchors,
            #[cfg(feature = "view")]
            hit_spark_anchors,
            #[cfg(feature = "view")]
            weapon_spark_anchors,
            #[cfg(feature = "view")]
            wreck_anchors,
            #[cfg(feature = "view")]
            magstrip_wake,
            #[cfg(feature = "view")]
            magstrip_pob,
            #[cfg(feature = "view")]
            leach_strip,
            #[cfg(feature = "view")]
            destroy_stations,
            #[cfg(feature = "view")]
            intro_camera,
            slot_teams,
            sight,
            ..
        } = setup;

        // The mode gate, applied once at construction rather than on every tick,
        // because that is where the original applies it: `World_CollectNodeLists`
        // zeroes the trigger list's own count at track load, so a weapons-off
        // race has no weapon pads to walk rather than a walk that decides
        // nothing. Dropping the volumes here also makes it impossible for a
        // later change to reach them by accident.
        let inertia = crate::launch_hover::craft_inertia(craft_laws);
        // Seated with the inertia rather than left to the first tick's `set_hover_caps`:
        // the speed plan is built below, before any tick, and simulates this very state.
        let rig = crate::launch_hover::physics_rig(hover_rig);
        let steer_ramp_clamped = craft_laws.is_some_and(|laws| laws.steer_ramp_clamped.value);
        let air_pitch_bias = craft_laws.is_none_or(|laws| laws.air_pitch_bias.value);
        let pitch_gate = craft_laws.is_none_or(|laws| laws.pitch_gate.value);
        let sideshift_grid_gate = craft_laws.is_none_or(|laws| laws.sideshift_grid_gate.value);
        let turbo_running_laws = craft_laws.is_none_or(|laws| laws.turbo_running_laws.value);
        let eliminator_wrong_way_kill =
            craft_laws.is_none_or(|laws| laws.eliminator_wrong_way_kill.value);
        let inertia_frame = crate::launch_hover::inertia_frame(craft_laws);
        let mut physics_laws = crate::launch_hover::physics_laws(craft_laws);
        // The disc's own craft-wall restitution where its global handling file authors one
        // (Wipeout HD, 2048, Omega): data, so no title is named here.
        if let Some(restitution) = wall_restitution {
            physics_laws.wall_restitution = restitution;
        }
        // A title whose engine reads the launch multiplier for the player's craft alone (Pure)
        // launches its field unboosted.
        let field_start_boost = if craft_laws
            .and_then(|laws| laws.code_start_boost)
            .is_some_and(|boost| boost.value.player_only)
        {
            None
        } else {
            start_boost
        };
        let class_gravity_term =
            if craft_laws.is_some_and(|laws| laws.class_gravity_on_flight.value) {
                oag_physics::passive::ClassGravityTerm::Airborne
            } else {
                oag_physics::passive::ClassGravityTerm::Grounded
            };
        let weapons_on = weapons_override.unwrap_or_else(|| mode.weapons_enabled());
        let weapon_pads = if weapons_on { weapon_pads } else { Vec::new() };

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
        #[cfg(feature = "view")]
        let destroy_focus_rate = super::destroy_camera::focus_rate(&class);
        let lap_class = oag_race::SpeedClass::census(&class).unwrap_or_else(|| {
            warn!(
                "speed class {class:?} is not one of the four the lap census covers - \
                 lap count falls back to {venom}'s, which is chosen, not measured",
                venom = oag_race::SpeedClass::VENOM,
            );
            oag_race::SpeedClass::VENOM
        });
        // **Every slot, not slot 0's alone.** A `RaceState` is a racer's clock
        // now, and the mode and speed class it is built from are the *event's*
        // - all eight craft run the same one. Seeding the array uniformly here
        // is what makes `World::mode` and `World::laps_target` safe to read off
        // slot 0; see `World::race`'s own doc comment for the replication this
        // is the writer of.
        world.race = [RaceState::new(mode, lap_class); oag_gameplay::MAX_PLAYERS];
        // The cell's own `laps` wins over the per-class table, for the two
        // modes where a lap count ends the race at all - see
        // `Options::laps_override`'s own doc for why `SpeedLap`/`Zone` must
        // never reach here despite authoring a `laps` attribute too.
        if let Some(laps) = laps_override {
            for race in &mut world.race {
                race.laps_target = Some(laps);
            }
        }
        let ship = &mut world.ships[0];
        ship.active = true;
        ship.handling = handling;
        ship.physics.body.mass = handling.physical.mass;
        ship.physics.body.inertia = inertia;
        ship.physics.hover_rig = rig;
        ship.physics.steer_ramp_clamped = steer_ramp_clamped;
        ship.physics.air_pitch_bias = air_pitch_bias;
        ship.physics.pitch_gate = pitch_gate;
        ship.physics.sideshift_grid_gate = sideshift_grid_gate;
        ship.physics.turbo_running_laws = turbo_running_laws;
        ship.physics.inertia_frame = inertia_frame;
        // The pool starts full, which is `Ship_ResetShield` (`0x0883dd24`) - the
        // only thing on the disc that sets it outright. Wall contact spends it
        // from here (`oag_physics::damage`), and Zone's perfect-zone recharge
        // adds back to it.
        oag_physics::damage::reset(&mut ship.physics, &handling.dimensions);
        // Whether this race fields a grid at all: the mode's own answer, or
        // the `--opponents` escape hatch that measurement's own ground-truth
        // tests use to force it. Read once, because both the player's own
        // pose below and the opponent loop further down have to agree on it.
        //
        // **A Multiplayer Time Trial grids its people whatever else is asked**
        // ([`Mode::is_multiplayer_time_trial`]): every person is a craft the mode needs, so
        // neither the `--opponents` hatch nor a class with no driver (`no_opponents`, which
        // empties the AI's seats) may add or remove one.
        let multiplayer_time_trial = mode.is_multiplayer_time_trial(usize::from(people));
        let full_grid = start_position.is_some()
            && (multiplayer_time_trial || (!no_opponents && (mode.has_opponents() || opponents)));
        // How many of the grid's other seven slots are actually driven.
        // Every opponent-fielding mode except Head2Head fields all seven;
        // Head2Head fields exactly one - see [`Mode::opponent_count`]'s own
        // doc for the measurement. The `--opponents` escape hatch always
        // asks for the full seven, matching what it did before this field
        // existed.
        let field_opponents = if multiplayer_time_trial {
            // The people and nobody else, each on a grid slot of their own: person `k`
            // takes seat `k`, the back row first. **Chosen, not measured**: the original has no
            // such mode on the grid, and `grid.md`'s "a short grid packs to the back" is
            // contradicted by the measured Head to Head field of two ([`grid_row`]).
            people.min(GRID_SLOTS) - 1
        } else if no_opponents {
            0
        } else if mode.field_is_fixed() {
            // Head2Head: two craft, whoever flies them. A second person takes the
            // one opponent's slot, so the people never meet an AI craft.
            mode.opponent_count()
        } else {
            mode.opponent_count()
                .max(if opponents { GRID_SLOTS - 1 } else { 0 })
        };
        // The override wins outright rather than being an offset from the grid
        // slot: it exists to put the craft at a position read off somewhere
        // else, and anything added to that would make the two disagree.
        let grid_cap = launch_hover.map(|hover| hover.grid_cap.value);
        let base = spawn_pose(
            &spline,
            start_position.as_ref(),
            &collision,
            &handling,
            grid_cap,
            &rig,
        );
        // Pulse PSP lays its grid out the way `Race_ComputeGridLayout` does: a walk of the
        // located curve, not of resampled samples. `None` where the track has no node, off
        // a PSP Pulse disc, or where the node is not on the track it locates (one circuit,
        // `25_Track` reversed), and every one of those keeps the node-anchored grid below.
        let walked = grid_frame_from_sample
            .then_some(start_position.as_ref())
            .flatten()
            .and_then(|slot| {
                oag_gameplay::grid_walk::walk(&ai, oag_core::math::Vec3::from_array(slot.position))
            });
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
            .map(|base| {
                grid_poses(
                    base,
                    &spline,
                    &collision,
                    rig_spawn_height(&handling, grid_cap, &rig),
                    grid_frame_from_sample,
                    walked.as_ref(),
                )[0]
            });
        // The player's grid row. A full grid puts the player on slot 8, the authored node
        // re-laid on the corridor midpoint's own `+/-10` (not the node's raw position:
        // `Race_ComputeGridLayout` derives every slot, the eighth included, from the midpoint
        // at its own sample, `grid.md`; measured on `16_Track`, the raw node sits 1.68 units
        // from the original's eighth craft, all of it lateral). A Head to Head field is
        // anchored at the front instead, see [`grid_row`].
        let front_anchored = mode.field_is_fixed();
        let player_row = grid_row(
            oag_gameplay::rank_from_back(0, field_opponents as usize + 1, usize::from(people)),
            field_opponents as usize + 1,
            front_anchored,
        );
        let full_grid_slot_eight = full_grid.then_some(base).flatten().map(|base| {
            grid_poses(
                base,
                &spline,
                &collision,
                rig_spawn_height(&handling, grid_cap, &rig),
                grid_frame_from_sample,
                walked.as_ref(),
            )[player_row - 1]
        });
        if let Some(pose) = pose_override
            .or(solo_slot_one)
            .or(full_grid_slot_eight)
            .or(base)
        {
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
        let line = racing_line(&spline, &order, &collision, handling.antigrav.ride_height);
        let rescue_distance = spline.max_half_width() * RESCUE_HALF_WIDTHS;
        let player_rescue_distance = spline.max_half_width() * PLAYER_RESCUE_HALF_WIDTHS;
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
        if (pose_override.is_none() || !field_poses.is_empty())
            && full_grid
            && let Some(base) = base
        {
            let poses = grid_poses(
                base,
                &spline,
                &collision,
                rig_spawn_height(&handling, grid_cap, &rig),
                grid_frame_from_sample,
                walked.as_ref(),
            );
            // **Where a short field sits.** Most modes pack a field smaller than the grid to
            // the back, mirroring `Race_SpawnGrid`'s compaction (`grid.md`, "a short grid packs
            // to the back", confidence 82, composed with "the local player is forced to the
            // back", confidence 75). Head to Head is the measured exception: a field of two
            // starts on the two FRONT places, read off PPSSPP (one person, AI on place 1) and
            // PCSX2 (two people, no compaction, the slot is the racer index), `head2head.md`
            // on both pages, confidence 92. [`grid_row`] holds the rule; the pose below, the
            // load report and the original AI's grid read all go through it.
            let pose_start = (GRID_SLOTS as usize - 1) - field_opponents as usize;
            for slot_index in pose_start..GRID_SLOTS as usize - 1 {
                let ship_index = slot_index - pose_start + 1;
                // **Which grid row this craft starts on.** One person: the row it
                // always had (`slot_index`). Two or more: the people take the back
                // rows in order and the opponents the rows ahead of them, in their
                // own order - `oag_gameplay::rank_from_back`, read off PS2 Pulse and
                // HD. The craft's driver seed and pilot below stay keyed to
                // `slot_index`, the craft's own, so nobody's character moves.
                let rank = oag_gameplay::rank_from_back(
                    ship_index,
                    field_opponents as usize + 1,
                    usize::from(people),
                );
                let pose = &poses[grid_row(rank, field_opponents as usize + 1, front_anchored) - 1];
                // Its own team's tuning where the title's variant family gave
                // it one (`load::family`), the player's otherwise.
                let handling = opponent_handling
                    .get(ship_index)
                    .copied()
                    .flatten()
                    .unwrap_or(handling);
                let opponent = &mut world.ships[ship_index];
                opponent.active = true;
                opponent.handling = handling;
                opponent.physics.body.mass = handling.physical.mass;
                opponent.physics.body.inertia = inertia;
                opponent.physics.hover_rig = rig;
                opponent.physics.steer_ramp_clamped = steer_ramp_clamped;
                opponent.physics.air_pitch_bias = air_pitch_bias;
                opponent.physics.pitch_gate = pitch_gate;
                opponent.physics.sideshift_grid_gate = sideshift_grid_gate;
                opponent.physics.turbo_running_laws = turbo_running_laws;
                opponent.physics.inertia_frame = inertia_frame;
                oag_physics::damage::reset(&mut opponent.physics, &handling.dimensions);
                // **Eight drivers, not one driver eight times.** The seed is
                // the race's own and the craft's slot, so the same race fields
                // the same eight characters on every replay and the next race
                // fields eight others. Nothing here reads a clock or the
                // world's generator - see `oag_ai::Driver::for_slot`.
                let slot = slot_index as u32 + 1;
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
                // A captured field puts the craft where the original had it, not on its row.
                // The driver's lane index above is the row's: a posed capture takes no tick.
                opponent.place_at(field_poses.get(ship_index - 1).copied().unwrap_or(*pose));
            }
            world.ship_count = field_opponents + 1;
            // **What was loaded, and who is flying what, once per race.** The
            // digests are the point of the first line: thirty-two bits cannot
            // be inverted, so when two machines disagree on a world hash this
            // is the only thing that says *which* pilot file differs. The
            // second line is for the player, who wants to know the grid they
            // are about to race. A `*` marks a pilot that came from a file.
            debug!("pilots loaded: {}", roster.summary());
            debug!("ai difficulty: {}", difficulty.name());
            // Each opponent under the row it starts on, counted from the front (1 to 8),
            // which with more than one person is no longer its own slot number.
            let mut rows: Vec<(usize, &str)> = (1..=field_opponents as usize)
                .map(|ship| {
                    let rank = oag_gameplay::rank_from_back(
                        ship,
                        field_opponents as usize + 1,
                        usize::from(people),
                    );
                    (
                        grid_row(rank, field_opponents as usize + 1, front_anchored),
                        pilot_names[pose_start + ship].as_str(),
                    )
                })
                .collect();
            rows.sort_unstable_by_key(|&(row, _)| row);
            let grid: Vec<String> = rows
                .iter()
                .map(|(row, name)| format!("{row}:{name}"))
                .collect();
            info!("pilots on the grid: {}", grid.join(" "));
            if people > 1 {
                // Where each seat starts, counted from the front (1 to 8): the back rows, in seat order.
                let seats: Vec<String> = (0..usize::from(people).min(field_opponents as usize + 1))
                    .map(|seat| {
                        let rank = oag_gameplay::rank_from_back(
                            seat,
                            field_opponents as usize + 1,
                            usize::from(people),
                        );
                        format!(
                            "{}:{}",
                            seat + 1,
                            grid_row(rank, field_opponents as usize + 1, front_anchored)
                        )
                    })
                    .collect();
                info!("people on the grid: {}", seats.join(" "));
            }
        } else {
            world.ship_count = 1;
        }

        #[cfg(feature = "view")]
        let camera_view = oag_display::display::CameraView::default();
        #[cfg(feature = "view")]
        let chase_params = super::access::chase_block_for(camera_view, chase, chase_close);
        #[cfg(feature = "view")]
        let camera = Chase::snapped(target_of(&world.ships[0]), &chase_params);

        let weapon_pad_line = super::field::pad_seek_positions(&line, &weapon_pads);
        let routes = super::routes::route_lines(
            &spline,
            course.as_ref(),
            &order,
            &collision,
            handling.antigrav.ride_height,
        );
        // Where each craft is placed - after the whole grid is, so a slot a
        // person takes over later starts from its own grid pose - so the first recovery before they have
        // driven anywhere is onto the grid rather than onto sample zero - which
        // on a circuit whose start line is sample 2,791 is a different piece of
        // track entirely. See `RaceSim::last_on_track`.
        let last_on_track = std::array::from_fn(|slot| {
            spline
                .nearest(world.ships[slot].physics.body.position)
                .and_then(|(index, _, _)| u32::try_from(index).ok())
                .unwrap_or(0)
        });
        // `Race_PlaceGrid` pushes each craft's grid pose onto its ring, so a
        // reset before the craft has laid a crumb of its own puts it back there.
        let breadcrumbs = std::array::from_fn(|slot| {
            crate::breadcrumbs::Breadcrumbs::seeded(world.ships[slot].physics.body.position)
        });
        let race = Self {
            sim: RaceSim {
                racing_line: line,
                ai_order: order,
                routes,
                // Built below, once the field is seated: see
                // `Race::field_speed_plan`.
                speed_plan: None,
                plans_built: false,
                plan_reference: None,
                hull_plans: Vec::new(),
                ship_plan: [None; oag_gameplay::MAX_SHIPS],
                // **Degraded from the measured tuning**, never boosted toward it -
                // see `oag_ai::Difficulty`. At the top level this is the
                // measurement unchanged.
                ai_tuning: difficulty.tune(&oag_ai::Tuning::default()),
                ai_pilots,
                finished_thrust,
                original_tables,
                original_ai: None,
                race_seed: seed,
                people,
                // `--autopilot-skill` sets this after `Self::start` returns - see
                // `Self::set_autopilot_tuning`. Every race starts with the two
                // tunings agreeing.
                autopilot_tuning: None,
                autopilots: std::array::from_fn(|_| None),
                autopilot_rail: [None; oag_gameplay::MAX_PLAYERS],
                autopilot_ours: false,
                opponent_autopilot,
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
                last_respawn_cause: [None; oag_gameplay::MAX_SHIPS],
                cage_reset: [false; oag_gameplay::MAX_SHIPS],
                breadcrumbs,
                weapons_on,
                eliminator_kill_target: eliminator_kill_target
                    .unwrap_or(Mode::ELIMINATOR_KILL_TARGET_DEFAULT),
                last_damager: [None; oag_gameplay::MAX_SHIPS],
                locks: [sight.lock_sight(); oag_gameplay::MAX_SHIPS],
                lock_rig: chase,
                laid_from_rear,
                last_weapon_hit: [0; oag_gameplay::MAX_SHIPS],
                respawn_delay: [0.0; oag_gameplay::MAX_SHIPS],
                wreck_voice: [0.0; oag_gameplay::MAX_SHIPS],
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
                countdown_voice,
                countdown_beat,
                launch_hover,
                hover_rig: rig,
                steer_ramp_clamped,
                air_pitch_bias,
                pitch_gate,
                sideshift_grid_gate,
                turbo_running_laws,
                eliminator_wrong_way_kill,
                inertia_frame,
                physics_laws,
                autopilot_engine,
                weapon_pad_glow,
                leach_drain,
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
                weapon_pad_line,
                pad_seeking: super::PadSeeking::for_mode(mode),
                fire_law: if weapon_ai.is_some() {
                    super::FireLaw::Original
                } else {
                    super::FireLaw::Ours
                },
                weapon_ai,
                weapons,
                allowed_weapons,
                class,
                class_gravity_scale,
                class_gravity_term,
                start_boost,
                field_start_boost,
                pilot_assist,
                pilot_assist_levels: Default::default(),
                barrel_rolls: true,
                pad_current: [None; MAX_SHIPS],
                pad_previous_position: [None; MAX_SHIPS],
                scheme: ControlScheme::default(),
            },
            #[cfg(not(feature = "view"))]
            view: RaceView {
                slot_teams,
                ..RaceView::default()
            },
            #[cfg(feature = "view")]
            view: RaceView {
                chase_params,
                chase_far: chase,
                chase_close,
                viewpoints: Default::default(),
                internal_params: internal,
                // The default rather than the settings file's value, for the reason
                // `set_boost_fov_kick` gives: `Setup` is what a *headless* race
                // needs and a display preference must not be in front of a caller
                // with no screen. `set_camera_view` is what applies the player's.
                camera_view,
                camera,
                poses: Default::default(),
                correction: [Vec3::ZERO; MAX_SHIPS],
                correction_smoothing: true,
                seat: None,
                person_names: Vec::new(),
                contact: super::contact_fx::ContactFx::new(),
                intro: intro_camera
                    .filter(|_| camera_override.is_none())
                    .map(|(grid, rule)| super::intro_camera::IntroCamera::new(grid, rule)),
                camera_override,
                capture_hud,
                wreck_ended_tick: None,
                finish_camera: (!destroy_stations.is_empty()).then(|| {
                    super::finish_camera::FinishCamera::new(
                        destroy_stations.clone(),
                        destroy_focus_rate,
                        super::finish_camera::SPECTATOR_SEED,
                    )
                }),
                // Cold, then snapped on the first tick. A race starts from a standing
                // start with no thrust, so there is nothing to snap *to* here.
                //
                // A cold `Exhaust` has an empty trail ring and `Exhaust::trail_ready`
                // gates the ribbon on a *full* one, so no craft draws a ribbon across
                // the track from the origin to its grid slot on the opening ticks.
                exhaust: [Exhaust::new(); MAX_SHIPS],
                exhaust_rng: std::array::from_fn(|slot| Rng::new(exhaust_seed(slot))),
                zoom: zoom_ring
                    .map(|ring| oag_post::hd_zoom::Pulse::new(crate::zoom::tuning(ring))),
                zoom_borrowed,
                // Cold and empty for the same standing-start reason as `exhaust`
                // above: `hd::Tube::ready` gates on a full ring too.
                hd_trail: [exhaust::hd::Tube::new(); MAX_SHIPS],
                hd_trail_active: hd_trail.is_some(),
                hd_trail_red: hd_trail.unwrap_or([0.0; MAX_SHIPS]),
                ps2_trail,
                hull_reach: [0.0; MAX_SHIPS],
                trail_sparks,
                trail_inside: [0; MAX_SHIPS],
                hd_flame: [exhaust::hd::Flame::new(); MAX_SHIPS],
                hd_sprite: [exhaust::hd::Sprite::new(); MAX_SHIPS],
                shield: [ShipShield::with_palette(shield_palette); MAX_SHIPS],
                nozzles,
                engine_lights,
                spu_vertex_lights,
                hull_points,
                hd_weapon_lights: true,
                engine_light_jitter: [super::engine_light::Jitter::default(); MAX_SHIPS],
                engine_light_rng: std::array::from_fn(|slot| {
                    Rng::new(super::engine_light::engine_light_seed(slot))
                }),
                spark_anchors,
                slot_teams,
                collision_fx,
                effects,
                handles,
                sounds,
                track_emitters,
                announcer,
                class_announcer,
                stage: psys::Stage::new(),
                scenery_fx: super::scenery_fx::SceneryFx::new(scenery_fx),
                engine_flare: [None; MAX_SHIPS],
                projectile_flare: [None; oag_weapons::projectile::MAX_PROJECTILES],
                projectile_flare_orbit: [None; oag_weapons::projectile::MAX_PROJECTILES],
                quake_effect: None,
                repulser_effects: [[None; 4]; oag_weapons::projectile::repulser::POOL_SIZE],
                repulser_fields: [None; oag_weapons::projectile::repulser::POOL_SIZE],
                mag_floor_fx: Default::default(),
                magstrip_wake: magstrip_wake.map(|anchors| {
                    if magstrip_pob {
                        magstrip_wake::Wakes::pob(anchors)
                    } else {
                        magstrip_wake::Wakes::new(anchors)
                    }
                }),
                leach_strip: leach_strip.map(crate::leach_strip::Strip::new),
                quake_point: None,
                quake_surface: None,
                leach_beam_effect: None,
                leach_charge_effect: [None; MAX_SHIPS],
                leach_beam_ribbon: None,
                leach_beam_rng: Rng::new(LEACH_BEAM_SEED),
                rocket_smoke: crate::rocket_smoke::Smoke::new(),
                leach_ball_elapsed: 0.0,
                plasma_blasts: [None; blast_models::PLASMA_BLAST_SLOTS],
                bomb_blasts: [None; bomb_blast::BOMB_BLAST_SLOTS],
                hd_bomb_blasts: [None; bomb_blast::hd::SLOTS],
                hd_bomb_blast,
                bomb_shockwave_fades,
                hd_missile_blasts: [None; missile_blast::SLOTS],
                hd_missile_blast,
                hd_plasma_blast,
                pulse_laid_pose,
                laid_pose_scaled,
                screen_flash: if screen_flash {
                    Some(oag_fx::flash::ScreenFlash::default())
                } else {
                    quake_flash.then(|| oag_fx::flash::ScreenFlash::only(oag_fx::flash::QUAKE))
                },
                craft_flashes: Default::default(),
                wreck_fx: super::wreck_fx::WreckFx::new(wreck_anchors),
                destroy_camera: super::destroy_camera::DestroyCamera::new(
                    destroy_stations,
                    destroy_focus_rate,
                ),
                absorb_burst,
                absorb_anchors,
                absorb_bursts: Vec::new(),
                absorb_started: 0,
                hit_sparks: super::hit_sparks::HitSparks::new(hit_spark_anchors)
                    .with_weapon_anchors(weapon_spark_anchors),
                damage_fx: Default::default(),
                absorb_overlay: [None; MAX_SHIPS],
                absorb_shell: Default::default(),
                stage_rng: Rng::new(STAGE_SEED),
                results: None,
                run_stats: results::RunStats::default(),
                shield_was_up: false,
                // Every bar starts at `0.0`, not `1.0`: the pool is `0.0` until
                // `<Misc>` loads (see `Readout::shield_fraction`'s own
                // zero-guard), and a full-pool reading would arm the flash on a
                // false "drop" from 100% to whatever the first real tick measures.
                huds: std::array::from_fn(|_| crate::slot_hud::SlotHud {
                    sight: sight.view_sight(),
                    ..Default::default()
                }),
                pair_spoil: [false; MAX_SHIPS],
                split_gap: None,
                sight_screens: (sight.view_screen, sight.columns_screen),
                sight: sight.view_sight(),
                sight_state: sight::State::Absent,
                sight_fov: oag_display::display::Fov::AUTHORED,
                model_detail: oag_mesh::mesh::ModelDetail::default(),
                texture_detail: oag_mesh::mesh_render::TextureDetail::default(),
                boost_kick: 0.0,
                boost_fov_kick: oag_display::display::BoostFovKick::DEFAULT,
                flaps: [[0.0; 2]; MAX_SHIPS],
                flap_graphics: setup.airbrake_graphics,
            },
            #[cfg(feature = "view")]
            replay: replay::ReplayState::default(),
            quiet: false,
            #[cfg(feature = "view")]
            journal: Default::default(),
            #[cfg(test)]
            last_controls: Default::default(),
            #[cfg(test)]
            pair_contacts: Vec::new(),
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
