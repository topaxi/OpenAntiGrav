//! What a running race says about itself: one tick's telemetry, the wrong-way
//! test, and the HUD's readout.
//!
//! Split out of `race.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change. The
//! [`Telemetry`] struct itself followed it out of the root on 2026-08-23, for
//! the same reason and to the module it was always named for.

use super::*;

/// One tick's worth of what the simulation did, for a log or an overlay.
///
/// Everything here is read out of the state *after* a step; nothing in it is an
/// input to the next one.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Telemetry {
    /// Ticks elapsed since the race began.
    pub tick: u64,
    /// Where the ship is.
    pub position: Vec3,
    /// How fast it is going, in world units per second.
    pub speed: f32,
    /// Probes in contact over two: `0.0`, `0.5` or `1.0`.
    pub grounded: f32,
    /// Distance to the nearest spline sample.
    pub spline_distance: f32,
    /// Height above that sample along its own up axis. Negative is below the
    /// surface line.
    pub height_above_spline: f32,
    /// What the craft is carrying, if anything.
    ///
    /// Here so `--race --screenshot`'s own log answers "did the pad grant
    /// anything" without a debugger. It is the question every weapon capture
    /// starts with, and a HUD icon in a screenshot is a poor way to ask it.
    pub pickup: Option<oag_tables::weapons::Weapon>,
    /// How many projectiles are in the air.
    pub projectiles: usize,
}

impl Race {
    /// What the ship did, as of the last tick.
    #[must_use]
    pub fn telemetry(&self) -> Telemetry {
        let ship = self.ship();
        let position = ship.physics.body.position;
        let (spline_distance, height_above_spline) = self.sim.spline.nearest(position).map_or(
            (f32::NAN, f32::NAN),
            |(_, sample, distance)| {
                let up = (-Vec3::from_array(sample.down)).normalize_or_zero();
                (distance, (position - Vec3::from_array(sample.pos)).dot(up))
            },
        );

        Telemetry {
            tick: self.sim.world.tick,
            position,
            speed: ship.physics.body.linear_velocity.length(),
            pickup: ship.pickup.weapon,
            projectiles: self.sim.world.projectiles.live(),
            grounded: ship.physics.grounded,
            spline_distance,
            height_above_spline,
        }
    }

    /// Whether the ship is pointing back down the track.
    ///
    /// `dot(craft_forward, sample.tangent) < 0`. **The tangent, not the sample
    /// index** - `HANDOVER.md` records index order as unusable for this, because
    /// `Spline::from_track` concatenates paths in file order rather than travel
    /// order, and on a slow capture most windows step by zero. The dot product has
    /// neither failure mode and reads `+0.9999` against `-0.9999`.
    ///
    /// [`ai_order`] did not change that. It gives the *drivers* a travel-ordered
    /// view; this reads `Spline::nearest`, which is still the whole table in
    /// file order, and has to be - the player can be anywhere the track is,
    /// including on a branch the lap never drives.
    ///
    /// `false` when the ship is not near the spline at all, which is the safe way
    /// round: a spurious warning is worse than a missing one.
    #[must_use]
    pub fn wrong_way(&self) -> bool {
        self.wrong_way_for(self.player_slot())
    }

    /// [`Self::wrong_way`] for the craft in `slot`.
    #[must_use]
    pub fn wrong_way_for(&self, slot: usize) -> bool {
        let ship = &self.sim.world.ships[slot];
        let Some((_, sample, _)) = self.sim.spline.nearest(ship.physics.body.position) else {
            return false;
        };
        let tangent = Vec3::from_array(sample.tangent).normalize_or_zero();
        ship.physics.body.forward().dot(tangent) < 0.0
    }

    #[cfg(feature = "view")]
    /// What the HUD shows, as of the last tick.
    ///
    /// Separate from [`Telemetry`], which is a log line and carries diagnostics
    /// no player sees. The fields with no source yet are left at their "unknown"
    /// value rather than filled with a plausible number - `lap` and `place` read
    /// zero, and [`oag_hud`] omits a widget rather than claiming a value it does
    /// not have. See `docs/ui/hud.md`.
    #[must_use]
    pub fn readout(&self) -> oag_hud::Readout {
        self.readout_for(self.player_slot())
    }

    #[cfg(feature = "view")]
    /// What the HUD of the view that follows `slot` shows.
    ///
    /// [`Self::readout`] for the primary slot, which is the full-screen HUD. A
    /// split-screen race draws one per viewed slot. Every number comes off that
    /// slot's craft and race state; the bars animate from that slot's
    /// [`SlotHud`](super::slot_hud::SlotHud). **Primary only**, and left empty for
    /// any other slot: the split gap, which compares laps against the primary's
    /// stored best. The message lines are per slot (`SlotHud::messages`), each
    /// person's own. The Zone
    /// colour grade the caller fills in is one per scene.
    #[must_use]
    pub fn readout_for(&self, slot: usize) -> oag_hud::Readout {
        let primary = slot == self.player_slot();
        let ship = &self.sim.world.ships[slot];
        let race = &self.sim.world.race[slot];
        let hud = &self.view.huds[slot];
        // Zero still means "unknown", and a track with no closed ring still has
        // no lap counter - the widget is omitted rather than reading 1 of 3 on a
        // course that cannot tell.
        let counted = self.sim.course.is_some();
        // A person's view after their own crossing, while the others race on: the
        // lap counter holds at the target, the clocks at the moment they finished,
        // and a standing line says so. See `Self::finished_view`.
        let finished = self.finished_view(slot);
        let held = self.held_for_hud(slot);
        let mut readout = oag_hud::Readout {
            speed_kmh: ship.physics.body.linear_velocity.length() * oag_fx::exhaust::SPEED_TO_KMH,
            // The ship's own pool, which wall contact spends and absorbing a
            // pickup pays back into - `oag_physics::damage`. (This comment said
            // "nothing depletes this yet" until 2026-08-11, three subsystems
            // after it stopped being true.)
            shield: ship.physics.shield,
            shield_max: ship.handling.dimensions.shield,
            lap: if counted {
                finished.map_or(race.lap, |held| held.lap)
            } else {
                0
            },
            // A speed lap and a Zone run have no lap target. Zero is what the HUD
            // already reads as "unknown" and it omits the "of N" half.
            laps: race.laps_target.filter(|_| counted).unwrap_or(0),
            // The player's place in the field, from the same table the results
            // will read - [`Self::player_place`], slot 0 of [`Self::places`].
            //
            // Two gates, both meaning "there is no place to report" rather than
            // "the place is one". `counted`, because with no closed ring there is
            // no distance round the circuit to order the field by and
            // [`Self::places`] answers "everyone is first" - honest as a return
            // value and a lie on screen. And a field of **more than one craft**,
            // because a place is a position among opponents: a time trial, a speed
            // lap and a Zone run all grid the player alone, and so does a single
            // race on a track with no authored `Start Position` node. `1 / 1` is
            // arithmetic rather than a standing, and the widget group is omitted
            // the same way the lap group is on a track that cannot count.
            place: if counted && self.sim.world.ship_count > 1 {
                self.field_place(slot)
            } else {
                0
            },
            ships: u32::from(self.sim.world.ship_count),
            // The racing clock, not the raw tick: the original's clocks read
            // zero through the whole start-line countdown and start on the
            // release. `oag_race::race_clock_ticks` has the measurement.
            // The gates and animations that need the raw tick read
            // `world.tick` themselves.
            race_ticks: finished.map_or_else(
                || oag_race::race_clock_ticks(self.sim.world.tick),
                |held| held.race_ticks,
            ),
            lap_ticks: if let Some(held) = finished {
                held.lap_ticks
            } else if counted {
                race.lap_ticks(self.sim.world.tick)
            } else {
                oag_race::race_clock_ticks(self.sim.world.tick)
            },
            best_lap_ticks: race.best_lap_ticks,
            // Gated the same as `lap`/`laps` above: a track with no closed ring
            // has no lap history either.
            lap_splits: if counted {
                race.lap_splits
            } else {
                [None; oag_race::MAX_RECORDED_LAPS]
            },
            wrong_way: self.wrong_way_for(slot),
            zone: race.zone.into(),
            score: race.score,
            // The colour grade lives on `crate::Scene`, not on the
            // simulation, so the two draw sites that have both fill this in -
            // see `Scene::zone_stage`. Zero here means "no grade", which is what
            // rung zero already reads as: unnamed.
            zone_stage: 0,
            zone_next_in: None,
            // A running Autopilot keeps its icon up when nothing else is held:
            // `PlayerStatus_Update` forces the icon id to the Autopilot's while the
            // running bit is set and the held id maps to none. The timer under it is
            // `Self::sub_weapon`.
            pickup: held,
            ext: oag_hud::ext_draws::Outputs {
                pickup_look: hud.pickup_look.clone(),
                ..self.sim.extensions.hud(slot, self.sim.world.tick)
            },
            sub_weapon: self.sub_weapon(slot),
            campaign_event: self.view.campaign_event,
            // The reticle, whatever it is doing. `Sight::visible` is what
            // decides whether anything reaches the screen, so this is passed
            // unconditionally rather than gated here - the brackets are watched
            // opening again after a target is lost, which a gate on "has a
            // target" would cut off.
            sight: Some(self.drawn_sight(slot)),
            shield_flashing: hud.shield_flash_timer > 0.0,
            shield_absorbing: self.absorb_window_active(slot),
            shield_blink_phase: hud.shield_blink_timer,
            shield_flashing_whole: hud.shield_flash_timer_whole > 0.0,
            shield_blink_phase_whole: hud.shield_blink_timer_whole,
            energy_bar_delay_fraction: hud.energy_bar_delay_fraction,
            pickup_announcing: hud.pickup_announce_left > 0.0,
            pickup_fade: hud.pickup_fade,
            thrust_chase_percent: hud.thrust_chase_percent,
            thrust_percent: ship.physics.thrust.max(0.0),
            pilot_assist: self.pilot_assist_shown(slot),
            assist: hud.assist_indicator.readout(),
            mode: self.sim.world.mode(),
            // Not this struct's to know: it needs the campaign cell and the
            // stored best, which live on `RaceStage`, one level up -
            // `RaceStage::draw_hud` fills it in after this method returns,
            // the same seam
            // `zone_stage`/`zone_next_in` already use above.
            time_trial_pace: None,
            head_to_head: self.head_to_head(slot),
            kill_tags: self.kill_tags(slot),
            kill_target: if self.sim.world.mode() == oag_race::Mode::Eliminator {
                self.sim.eliminator_kill_target
            } else {
                0
            },
            messages: hud.messages.lines(),
            time_gap: self
                .view
                .split_gap
                .as_ref()
                .filter(|_| primary)
                .and_then(|split| split.display.line().cloned()),
            // The drawn lean's follower, which a title with a measured sway slides its whole HUD
            // by (`oag_hud::Readout::steer_lean_follower`). Display state, never hashed.
            steer_lean_follower: ship.physics.steer_lean_follower,
            // The flare's boost timer over its length: what the HUD's multi-copy draw and the speed
            // bar's glow stamp run on (`oag_hud::boost_smear`). Display state, never hashed.
            best_lap_centiseconds: None,
            boost_phase: (self.exhaust_of(slot).boost_timer() / oag_fx::exhaust::BOOST_SECONDS)
                .clamp(0.0, 1.0),
            now_playing: if primary { self.now_playing() } else { None },
        };
        if let Some(hud) = self.view.capture_hud.filter(|_| primary) {
            readout.place = hud.place;
            readout.lap = hud.lap;
            readout.lap_ticks = hud.lap_ticks;
            readout.race_ticks = hud.race_ticks;
            readout.speed_kmh = hud.speed_kmh.unwrap_or(readout.speed_kmh);
            if let Some(percent) = hud.shield_percent {
                readout.shield = readout.shield_max * percent * 0.01;
            }
            readout.best_lap_ticks = hud.best_lap_ticks.or(readout.best_lap_ticks);
            readout.best_lap_centiseconds = hud.best_lap_centiseconds;
            if let Some(phase) = hud.boost_phase {
                readout.boost_phase = phase.clamp(0.0, 1.0);
            }
            if let Some(seconds) = hud.autopilot_seconds.filter(|&seconds| seconds > 0.0) {
                // The same icon fallback and countdown a running Autopilot gives above.
                readout.pickup = readout
                    .pickup
                    .or(Some(oag_tables::weapons::Weapon::Autopilot));
                readout.sub_weapon = Some(oag_hud::SubWeapon::Autopilot(seconds));
            }
        }
        if finished.is_some() {
            readout.messages[oag_hud::messages::SLOTS - 1] = Some(oag_hud::messages::MessageLine {
                text: FINISHED_LINE.to_string(),
                color: [1.0; 4],
                authored: true,
                fades_border: false,
            });
        }
        readout
    }

    #[cfg(feature = "view")]
    /// What a person's own view holds once their craft has crossed the line for the
    /// last time and the race is not over: `None` for one person (whose finish ends
    /// the race and hides the HUD), for a craft still racing, and for an AI craft.
    ///
    /// **Chosen, not measured.** The original has one player, and its HUD vanishes
    /// at the finish, so what a split view shows between its own line and the
    /// others' is ours: the lap counter held at the target, the clocks held at
    /// that crossing, the place the field gives them (frozen at the finish tick,
    /// see [`oag_race::Standing::finish_tick`]) and a standing line.
    pub(super) fn finished_view(&self, slot: usize) -> Option<FinishedView> {
        if self.viewport_count() <= 1 || !self.sim.world.person_flown(slot) {
            return None;
        }
        let finish = self.sim.world.ships[slot].standing.finish_tick?;
        let race = &self.sim.world.race[slot];
        let target = race.laps_target?;
        let laps = usize::try_from(target).unwrap_or(usize::MAX);
        Some(FinishedView {
            lap: target,
            race_ticks: oag_race::race_clock_ticks(finish),
            // The last lap's own time, from the splits the standing keeps; a race
            // longer than they hold shows the best lap instead of inventing one.
            lap_ticks: race
                .lap_splits
                .get(laps.saturating_sub(1))
                .copied()
                .flatten()
                .map(u64::from)
                .or_else(|| race.best_lap_ticks.map(u64::from))
                .unwrap_or(0),
        })
    }

    #[cfg(feature = "view")]
    /// The `SubWeapon` line for `slot`'s craft: the Cannon's rounds left, else the
    /// Autopilot's seconds left, else nothing - see [`oag_hud::SubWeapon`].
    ///
    /// **The Cannon wins**, the order `PlayerStatus_Update` writes them in
    /// (`docs/ghidra/functions/psp-pulse-usa/sub-weapon.md`).
    ///
    /// **A Cannon nobody has fired reads the full magazine.** The original arms
    /// `craft+0x154` with `<Stats rounds>` the moment the pickup is granted
    /// (`FUN_0885776c`), so its HUD shows `30` on the pad; this engine arms
    /// [`oag_weapons::pickup::Held::cannon_rounds`] on the first round and holds it at
    /// `0` until then, so the unarmed counter reads as the authored count here, which
    /// is what the original has in it.
    fn sub_weapon(&self, slot: usize) -> Option<oag_hud::SubWeapon> {
        let ship = &self.sim.world.ships[slot];
        if ship.pickup.weapon == Some(oag_tables::weapons::Weapon::Cannon) {
            let cannon = self
                .sim
                .weapons
                .as_ref()
                .and_then(oag_tables::weapons::WeaponStats::cannon)?;
            let rounds = match ship.pickup.cannon_rounds {
                0 => cannon.rounds.max(0.0).round() as u32,
                armed => u32::from(armed),
            };
            return Some(oag_hud::SubWeapon::Rounds(rounds));
        }
        (ship.autopilot_timer > 0.0).then_some(oag_hud::SubWeapon::Autopilot(ship.autopilot_timer))
    }

    #[cfg(feature = "view")]
    /// The Eliminator's kill column, top row first, in the order the original
    /// draws it - see [`oag_hud::kill_tags`]. Empty in every other mode, and
    /// for a slot with no team on record (a `Setup` built by hand).
    fn kill_tags(&self, player: usize) -> Vec<oag_hud::KillTag> {
        let world = &self.sim.world;
        if world.mode() != oag_race::Mode::Eliminator {
            return Vec::new();
        }
        let slots: Vec<usize> = (0..world.ship_count as usize)
            .filter(|&slot| world.ships[slot].active)
            .collect();
        let kills: Vec<u32> = slots
            .iter()
            .map(|&slot| world.ships[slot].standing.kills)
            .collect();
        oag_hud::kill_tags::ranked(&kills)
            .into_iter()
            .filter_map(|row| {
                let slot = slots[row];
                Some(oag_hud::KillTag {
                    team: self.view.slot_teams.get(slot)?.clone(),
                    kills: kills[row],
                    player: slot == player,
                })
            })
            .collect()
    }

    #[cfg(feature = "view")]
    /// The place the HUD shows: [`Self::places`] ranked among the craft that are in
    /// the race. A slot with no craft in it sits at distance zero, so on a grid still
    /// short of the line (the back rows are) a field smaller than eight read `8 / 2`
    /// for the craft at the back. Only what is drawn: the simulation, the AI and the
    /// results keep reading [`Self::places`] as they always did.
    fn field_place(&self, slot: usize) -> u32 {
        let places = self.places();
        let world = &self.sim.world;
        let empty_ahead = (0..oag_gameplay::MAX_SHIPS)
            .filter(|&other| !world.ships[other].active && places[other] < places[slot])
            .count();
        u32::from(places[slot]) - u32::try_from(empty_ahead).unwrap_or(0)
    }

    #[cfg(feature = "view")]
    /// Head2Head's gap readout: the unwrapped-progress difference between the
    /// player and the one opponent, and who leads. `None` outside Head2Head.
    /// With two people the opponent is the other person, so each view reads its
    /// own gap to the other (**chosen, not measured**).
    ///
    /// The original takes `|craft+0xad0 - craft+0xad0|` over the pair
    /// (`FUN_0881d458`), and [`oag_race::Standing::distance`] is this
    /// engine's `craft+0xad0` with the same lap-1 wrap fix, in the same
    /// arc-length units. See `docs/ghidra/functions/psp-pulse-usa/head2head.md`.
    fn head_to_head(&self, player: usize) -> Option<oag_hud::HeadToHead> {
        if self.sim.world.mode() != oag_race::Mode::Head2Head {
            return None;
        }
        let course = self.sim.course.as_ref()?;
        let world = &self.sim.world;
        let opponent = (0..world.ship_count as usize)
            .find(|&slot| slot != player && world.ships[slot].active)?;
        // **Measured between the two craft, not read off [`Self::places`].** A place is
        // a rank among all eight slots, and a slot with no craft in it sits at
        // distance zero, so a person still short of the line (the back of the grid
        // is) ranks behind empty slots and neither of two people ever read first.
        let ahead = world.ships[player].standing.distance(course)
            - world.ships[opponent].standing.distance(course);
        // Two people: both rows carry a team name, each view's own first. One person: the
        // player's row is the profile tag in the original, which this build has none of.
        let named = self.sim.people > 1;
        let team = |slot: usize| self.view.slot_teams.get(slot).cloned();
        Some(oag_hud::HeadToHead {
            gap: ahead.abs(),
            player_leads: ahead > 0.0,
            own_team: if named { team(player) } else { None },
            other_team: team(opponent),
            stacked: false,
        })
    }
}

#[cfg(feature = "view")]
/// The id of the line a finished person's view carries: our own text, in
/// `assets/ui/strings/english.toml`.
const FINISHED_LINE: &str = "OAG_SPLIT_FINISHED";

#[cfg(feature = "view")]
/// A finished person's held readout, see [`Race::finished_view`].
#[derive(Debug, Clone, Copy)]
pub(super) struct FinishedView {
    lap: u32,
    race_ticks: u64,
    lap_ticks: u64,
}

/// One telemetry line, for a log or a report.
#[must_use]
pub fn describe(telemetry: &Telemetry) -> String {
    format!(
        "tick {:>5}  speed {:>8.2}  grounded {:>3.1}  spline {:>8.2}  height {:>8.2}  at {:.1}\
         {}",
        telemetry.tick,
        telemetry.speed,
        telemetry.grounded,
        telemetry.spline_distance,
        telemetry.height_above_spline,
        telemetry.position,
        match (telemetry.pickup, telemetry.projectiles) {
            (None, 0) => String::new(),
            (held, count) => {
                let held = held.map_or("-", oag_tables::weapons::Weapon::as_type);
                format!("  holding {held}  in the air {count}")
            }
        },
    )
}
