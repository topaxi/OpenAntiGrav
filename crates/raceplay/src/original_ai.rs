//! The original's own opponent (`oag_ai::original`), rubber banding included,
//! flown on each craft's own team's physics (`Setup::opponent_handling`; the
//! controller reads no handling, the step does): the AI every race with
//! opponents flies by default (maintainer, 2026-10-13), until this project's
//! own driver (`oag_ai::Driver`) is solid. It began as the campaign's third AI
//! mode (2026-10-11), when only a campaign launch switched it on.
//!
//! **Still never a player setting** (the 2026-10-11 rule). What changed is who
//! asks: the composition root (`oag_game::ai_choice`) calls
//! [`Race::use_original_ai`] for every race that fields opponents, and the only
//! way back to this project's driver is the developer flag `oag-game --ai
//! ours`, a run flag and not a menu row or a settings key. Evidence:
//! `docs/gameplay/ai.md`, "The original driver, recovered" and "Which AI flies".
//!
//! **Mapped, not measured**, and said so where each happens below: the
//! original's own projected sample is this engine's nearest spline sample
//! interpolated onto the craft; its lap progress is the course distance over
//! the course length; its lookahead walks the sample table; its contact list
//! is the last step's wall response; its race time starts at the green light.

use super::*;
use oag_ai::original::{Field, Output, Pose, Standings, Tick, TrackPoint};

/// The original field and what the last physics step said about contacts.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct OriginalAi {
    pub(super) field: Field,
    /// Per slot, the last step's wall contacts: signed by side (`+` on the
    /// craft's left) and whether the body touched anything.
    contacts: [(i32, bool); MAX_SHIPS],
}

/// The contact summary `Ai_PlanLane` and `Ai_SnapToLane` read, off a step's
/// wall response. The original walks the body's contact list (`body+0x170`,
/// kind `1` for the push, any for the rail); this engine reports a count and
/// one resolved normal, so every non-floor contact is put on the resolved
/// one's side. A craft-to-craft contact is not in it.
fn contact_summary(wall: &oag_physics::WallResponse, right: Vec3) -> (i32, bool) {
    let walls = wall.contacts.saturating_sub(wall.floor_contacts) as i32;
    let side = match wall.resolved {
        // The normal points off the wall toward the craft: a wall on the
        // craft's left pushes it right.
        Some(contact) if contact.normal.dot(right) > 0.0 => 1,
        Some(_) => -1,
        None => 0,
    };
    (walls * side, wall.contacts > 0)
}

impl Race {
    /// Flies every opponent with the original's own driver for the rest of
    /// this race. `skill` is `AI_ResolveSkillScale`'s campaign value
    /// (`Session::resolve_campaign_ai_skill_scale`), `None` for a race with
    /// none, which reads the race's ambient value the load resolved; `level`
    /// the integer rung, `0` Easy to `2` Hard, which the pack shuffle reads: a
    /// campaign cell's own, else the one `oag_game::ai_choice::free_level`
    /// chooses from `[ai] difficulty` (chosen, not measured).
    ///
    /// `false`, with the reason logged, when the class's AI tables did not
    /// read or the track has no closed lap: the race keeps this project's
    /// driver, which is an honest fallback rather than a guessed table.
    pub fn use_original_ai(&mut self, skill: Option<f32>, level: usize) -> bool {
        let Some(tables) = self.sim.original_tables else {
            log::warn!(
                "AI: this project's driver flies (no original tables for this title and class; \
                 see the loader report)"
            );
            return false;
        };
        // Pure's cruise throttle reads no skill; a Pulse-shaped table always
        // comes with its finished-thrust read, the same file.
        let skill = skill
            .or(self.sim.finished_thrust.map(|thrust| thrust.skill))
            .unwrap_or(0.0);
        if self.sim.course.is_none() {
            log::warn!("AI: this project's driver flies (the track has no closed lap)");
            return false;
        }
        let count = self.sim.world.ship_count as usize;
        // The grid position (1 front, 8 back) each craft starts on, which is the row
        // `rank_from_back` gave it: the player last with the opponents seated from the
        // back, or - with several people - the people on the last rows.
        let people = usize::from(self.sim.people);
        let grid: [u8; MAX_SHIPS] = std::array::from_fn(|slot| {
            let rank = oag_gameplay::rank_from_back(slot, count, people);
            MAX_SHIPS.saturating_sub(rank).clamp(1, MAX_SHIPS) as u8
        });
        self.sim.original_ai = Some(Box::new(OriginalAi {
            field: tables.field(skill, level.min(2), grid, self.sim.race_seed),
            contacts: [(0, false); MAX_SHIPS],
        }));
        log::info!("AI: the original driver flies the opponents, skill {skill:.2}, level {level}");
        true
    }

    /// A 2048 event's two AI skills, after [`Self::use_original_ai`]: the
    /// field's ring rate follows them where the title's laws say so
    /// (`oag_ai::original::Field::set_event_ai_skills`). Nothing without an
    /// original field.
    pub fn set_original_event_ai_skills(&mut self, pad_tracking: f32, weapon_avoidance: f32) {
        if let Some(ai) = self.sim.original_ai.as_mut() {
            ai.field.set_event_ai_skills(pad_tracking, weapon_avoidance);
        }
    }

    /// Whether this race's opponents fly the original's driver.
    #[must_use]
    pub fn original_ai_active(&self) -> bool {
        self.sim.original_ai.is_some()
    }

    /// The original field, for a measurement that reads its state.
    #[must_use]
    pub fn original_ai_field(&self) -> Option<&Field> {
        self.sim.original_ai.as_deref().map(|ai| &ai.field)
    }

    /// `AiManager_Update`'s head, once a tick before the opponents: which
    /// slot runs its thrust law and lane choice this tick.
    pub(super) fn original_ai_begin_tick(&mut self) -> Option<usize> {
        let (list, n) = self.ai_flown_slots();
        let ai = self.sim.original_ai.as_mut()?;
        Some(ai.field.begin_tick(&list[..n]))
    }

    /// Every active craft no person flies, in slot order, and how many.
    ///
    /// With one person in slot 0 this is slots `1..count`, the list the round
    /// robin always took; a person in another slot is left out as slot 0 is.
    pub(super) fn ai_flown_slots(&self) -> ([usize; MAX_SHIPS], usize) {
        let count = self.sim.world.ship_count as usize;
        let mut list = [0usize; MAX_SHIPS];
        let mut n = 0;
        for slot in (0..count)
            .filter(|&s| self.sim.world.ships[s].active && !self.sim.world.person_flown(s))
        {
            list[n] = slot;
            n += 1;
        }
        (list, n)
    }

    /// The craft the rubber band and the position-keyed tables read as "the
    /// player": the best-placed person-flown slot, the lowest slot on a tie.
    ///
    /// The original has one player, and `AI_ComputeOpponentThrust` reads that
    /// player's place, the gap to it and its ship state (ai-controls.md); how
    /// it would treat two is not recovered. **Chosen, not measured, no
    /// confidence score:** with several people in a race the field chases and
    /// waits for the one in front, so the human who is leading is never
    /// handed a field that has eased off because a trailing human is far
    /// behind. One person flown - every race but a split-screen one - is that
    /// person, and a race with nobody flown (the attract demo) is
    /// [`oag_gameplay::World::primary_slot`], as before.
    pub(super) fn rubber_band_reference(&self, places: &[u8; MAX_SHIPS]) -> usize {
        let world = &self.sim.world;
        (0..world.ship_count as usize)
            .filter(|&slot| world.person_flown(slot))
            .min_by_key(|&slot| (places[slot], slot))
            .unwrap_or_else(|| world.primary_slot())
    }

    /// The track sample `index` steps along slot `slot`'s line, as the
    /// original's driver reads one; `progress` is the lap fraction.
    fn original_point(
        &self,
        slot: usize,
        index: usize,
        next_fraction: f32,
        progress: f32,
    ) -> Option<TrackPoint> {
        let a = self.ai_sample_for(slot, index)?;
        let b = self.ai_sample_for(slot, index + 1).unwrap_or(a);
        let lerp3 = |x: [f32; 3], y: [f32; 3]| {
            Vec3::from_array(x) + (Vec3::from_array(y) - Vec3::from_array(x)) * next_fraction
        };
        let lerp = |x: f32, y: f32| x + (y - x) * next_fraction;
        let down = lerp3(a.down, b.down).normalize_or_zero();
        Some(TrackPoint {
            // Lifted, as the running game holds it (`Spline::track_sample`).
            position: lerp3(a.pos, b.pos) - down * oag_vex::track::HOVER_LIFT,
            tangent: lerp3(a.tangent, b.tangent).normalize_or_zero(),
            down,
            lateral: lerp3(a.lateral, b.lateral).normalize_or_zero(),
            bound_left: lerp(a.ai_bound_left, b.ai_bound_left),
            bound_right: lerp(a.ai_bound_right, b.ai_bound_right),
            racing_line: lerp(a.racing_line, b.racing_line),
            progress,
        })
    }

    /// Sample `index`'s position, unlifted.
    fn original_sample_position(&self, slot: usize, index: usize) -> Option<Vec3> {
        self.ai_sample_for(slot, index)
            .map(|s| Vec3::from_array(s.pos))
    }

    /// The craft's own sample: its located line index, projected onto the
    /// segment to the next sample.
    fn original_own(&self, slot: usize, progress: f32) -> Option<(TrackPoint, f32)> {
        let index = self.sim.world.ships[slot].driver.index as usize;
        let position = self.sim.world.ships[slot].physics.body.position;
        let p0 = self.original_sample_position(slot, index)?;
        let p1 = self.original_sample_position(slot, index + 1).unwrap_or(p0);
        let segment = p1 - p0;
        let length2 = segment.length_squared();
        let t = if length2 > 0.0 {
            ((position - p0).dot(segment) / length2).clamp(0.0, 1.0)
        } else {
            0.0
        };
        Some((self.original_point(slot, index, t, progress)?, t))
    }

    /// The sample `distance` further along slot `slot`'s line from its own,
    /// walking the sample table.
    fn original_ahead(
        &self,
        slot: usize,
        start_fraction: f32,
        distance: f32,
        progress: f32,
    ) -> Option<TrackPoint> {
        let start = self.sim.world.ships[slot].driver.index as usize;
        let length = self.sim.racing_line.len().max(1);
        let mut here = self.original_sample_position(slot, start)?;
        let next = self.original_sample_position(slot, start + 1)?;
        here = here + (next - here) * start_fraction;
        let mut left = distance;
        for index in (start..).take(length) {
            let next = self.original_sample_position(slot, index + 1)?;
            let step = (next - here).length();
            if step >= left && step > 0.0 {
                let base = self.original_sample_position(slot, index)?;
                let span = (next - base).length();
                let reached = here + (next - here) * (left / step);
                let fraction = if span > 0.0 {
                    ((reached - base).length() / span).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                return self.original_point(slot, index, fraction, progress);
            }
            left -= step;
            here = next;
        }
        None
    }

    /// One opponent's tick on the original driver: its controls and the
    /// rail's body write, or `None` when it cannot place itself on the track.
    pub(super) fn drive_original(
        &mut self,
        slot: usize,
        lane_slot: usize,
        places: &[u8; MAX_SHIPS],
    ) -> Option<Output> {
        let course = self.sim.course.as_ref()?;
        let lap_length = course.length();
        let count = self.sim.world.ship_count as usize;
        // Lap-length units: `Standing::distance`, laps included.
        let racers: [(f32, u8); MAX_SHIPS] = std::array::from_fn(|k| {
            let ship = &self.sim.world.ships[k];
            (ship.standing.distance(course), places[k])
        });
        let ship = &self.sim.world.ships[slot];
        let progress = (ship.standing.progress.unwrap_or(0.0) / lap_length).rem_euclid(1.0);
        // HD's `racer+0x90`, measured: the distance into the current lap.
        let lap_distance = ship.standing.progress.unwrap_or(0.0).rem_euclid(lap_length);
        let race_fraction = oag_ai::original::race_fraction(
            ship.standing.distance(course),
            self.sim.world.laps_target().unwrap_or(0),
            lap_length,
        );
        let body = ship.physics.body;
        let finished = ship.standing.finished();
        let grounded = ship.physics.grounded;
        let player = self.rubber_band_reference(places);
        let player_rubber_band_off =
            self.sim.world.ships[player].physics.craft_state != oag_physics::CraftState::Racing;
        let race_time = self
            .sim
            .world
            .tick
            .saturating_sub(oag_race::COUNTDOWN_TICKS) as f32
            * self.sim.dt;
        let (own, fraction) = self.original_own(slot, progress)?;
        let sample_index = self.sample_index_of(self.sim.world.ships[slot].driver.index as usize);
        let point_ai = sample_index
            .map(|i| self.sim.spline.point_ai(i))
            .unwrap_or_default();
        let (contact_sides, touching) = self.sim.original_ai.as_ref()?.contacts[slot];
        let tick = Tick {
            pose: Pose {
                position: body.position,
                up: body.up(),
                forward: body.forward(),
                left: -body.right(),
                velocity: body.linear_velocity,
                angular_velocity: body.angular_velocity,
            },
            own,
            band: point_ai.band,
            speed_floor: point_ai.speed_floor,
            grounded,
            contact_sides,
            touching,
            finished,
            race_fraction,
            lap_distance,
        };
        let dt = self.sim.dt;
        let standings = Standings {
            racers: &racers[..count],
            player,
            player_rubber_band_off,
            race_time,
            eliminator: self.sim.world.mode() == oag_race::Mode::Eliminator,
            lap_length,
        };
        // Out of the sim for the call, so the lookahead can read the track
        // while the field is driven.
        let mut ai = self.sim.original_ai.take()?;
        let output = ai
            .field
            .drive(slot, slot == lane_slot, &tick, &standings, dt, |distance| {
                let ahead_progress = (progress + distance / lap_length).rem_euclid(1.0);
                self.original_ahead(slot, fraction, distance, ahead_progress)
            });
        self.sim.original_ai = Some(ai);
        Some(output)
    }

    /// `oag_ai::original::Field::end_tick` from the standings after this
    /// tick's step: Pure's zone multiplier and launch flag. Nothing without
    /// an original field or a closed lap.
    pub(super) fn original_ai_end_tick(&mut self, places: &[u8; MAX_SHIPS]) {
        let Some(course) = self.sim.course.as_ref() else {
            return;
        };
        let count = self.sim.world.ship_count as usize;
        let racers: [(f32, u8); MAX_SHIPS] =
            std::array::from_fn(|k| (self.sim.world.ships[k].standing.distance(course), places[k]));
        let player = self.rubber_band_reference(places);
        let standings = Standings {
            racers: &racers[..count],
            player,
            player_rubber_band_off: false,
            race_time: 0.0,
            eliminator: false,
            lap_length: course.length(),
        };
        if let Some(ai) = self.sim.original_ai.as_mut() {
            ai.field.end_tick(&standings);
        }
    }

    /// Notes what this tick's step said about slot `slot`'s contacts, for the
    /// next tick's planner and rail.
    pub(super) fn original_ai_contacts(&mut self, slot: usize, wall: &oag_physics::WallResponse) {
        let right = self.sim.world.ships[slot].physics.body.right();
        if let Some(ai) = self.sim.original_ai.as_mut() {
            ai.contacts[slot] = contact_summary(wall, right);
        }
    }
}
