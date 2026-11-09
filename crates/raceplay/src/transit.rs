//! A craft's jump along the circuit (a simulation mod's `transit`): out of the world for
//! a number of ticks, then put further round at a given speed.
//!
//! - **The primitive** ([`Race::move_to`]): a craft put at a course point forward of it,
//!   under a lap on, with the ground between credited as driven (a crossing of the line in
//!   it is a lap or a finish on that tick: [`oag_race::Standing::jumped`]). A transit is
//!   this behind a stay out of the world; a reset is not (it goes back, to a crumb, and
//!   earns nothing), and shares only the cursor reset, [`Race::reseat`].
//! - **Entry** ([`Race::begin_transit`]): the exit point is resolved at once
//!   ([`Race::point_ahead`]: units on, seconds at the speed it has now, a share of the lap,
//!   a place on the lap, or a track trigger); the craft keeps its pose and is frozen there.
//! - **Out of the world** ([`oag_gameplay::Ship::in_world`]): not stepped (no physics, no
//!   pads, no triggers, its controls and pickup presses ignored), not drawn, no
//!   craft-to-craft contact, and `oag_weapons::Craft::active` is false, so no weapon hits,
//!   drains or locks it. Its reset triggers are held ([`Race::reset_held`]). Its standing
//!   is still updated on the frozen pose, so nothing wraps and the Zone clock runs.
//! - **Exit** ([`Race::advance_transits`], before the tick reads any craft's controls): on the tick the
//!   count runs out, the craft is moved to [`oag_gameplay::Transit::to`]
//!   ([`Race::move_to`]) and stepped from there the same tick, so a crossing of the line in
//!   the jump, a finish included, is timed at the exit tick.

use oag_core::math::Vec3;
use oag_gameplay::Transit;

use crate::Race;

pub use oag_ext_abi::Distance;
use oag_ext_abi::{MAX_SPEED_FACTOR, MAX_TRANSIT_TICKS};

/// A course point a transit comes out at: a ring point, and the fork route it is on when it
/// is on one (`oag_race::Course::route_point` maps the ring point onto the route).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Exit {
    /// The ring point, the stand-in for how far round the lap it is.
    pub to: usize,
    /// The route's index in [`oag_race::Course::routes`] plus one; `0` for the ring.
    pub route: u16,
}

impl Exit {
    /// An exit on the ring itself.
    #[must_use]
    pub fn ring(to: usize) -> Self {
        Self { to, route: 0 }
    }

    /// The exit a point `centre` (a track trigger's) names: the route when a route's sample is
    /// nearer to it than the ring's ([`oag_race::Course::locate_exit`]). `None` off the course.
    #[must_use]
    pub fn at(course: &oag_race::Course, centre: Vec3) -> Option<Self> {
        let (to, route) = course.locate_exit(centre)?;
        let route = route.map_or(0, |route| u16::try_from(route + 1).unwrap_or(0));
        Some(Self { to, route })
    }
}

/// How many rays a clearance probe casts round its half disc, end to end. Chosen.
const CLEARANCE_RAYS: usize = 9;
/// How far above the track surface a clearance probe's rays start, so the floor under
/// the disc's straight edge is not read as an obstacle. Chosen.
const CLEARANCE_LIFT: f32 = 0.5;

impl Race {
    /// Starts slot `slot`'s transit: `ticks` out of the world, then out at `distance`
    /// ([`Self::point_ahead`]) at `exit_factor` times the speed it has now. A refusal names
    /// why and changes nothing: a craft not racing or already in a transit, a tick count
    /// of 0 or past [`MAX_TRANSIT_TICKS`], a factor out of `[0, MAX_SPEED_FACTOR]`, a
    /// point that does not resolve.
    ///
    /// # Errors
    ///
    /// The refusal, for the caller to fault the module with.
    pub fn begin_transit(
        &mut self,
        slot: usize,
        ticks: u16,
        distance: Distance,
        exit_factor: f32,
    ) -> Result<(), String> {
        let ship = &self.sim.world.ships[slot];
        if !ship.active || ship.physics.craft_state != oag_physics::CraftState::Racing {
            return Err(format!("slot {slot} holds no racing craft"));
        }
        if ship.transit.is_some() {
            return Err(format!("slot {slot} is already in a transit"));
        }
        if ticks == 0 || ticks > MAX_TRANSIT_TICKS {
            return Err(format!(
                "a transit of {ticks} ticks is not within 1..={MAX_TRANSIT_TICKS}"
            ));
        }
        if !(exit_factor.is_finite() && (0.0..=MAX_SPEED_FACTOR).contains(&exit_factor)) {
            return Err(format!(
                "exit speed factor {exit_factor} is not within [0, {MAX_SPEED_FACTOR}]"
            ));
        }
        let (exit, _) = self.exit_ahead(slot, distance)?;
        let speed = self.speed_of(slot);
        self.sim.world.ships[slot].transit = Some(Transit {
            ticks_left: ticks,
            to: u32::try_from(exit.to).unwrap_or(u32::MAX),
            route: exit.route,
            exit_speed: speed * exit_factor,
        });
        Ok(())
    }

    fn speed_of(&self, slot: usize) -> f32 {
        self.sim.world.ships[slot]
            .physics
            .body
            .linear_velocity
            .length()
    }

    /// The course ring point `distance` names, forward of slot `slot`'s craft and less
    /// than a lap on, with how far on it is in track units.
    ///
    /// # Errors
    ///
    /// Why it does not resolve: no course, an amount that is a lap or more on (or not
    /// above zero), a progress off the lap, a trigger the track does not have.
    pub fn point_ahead(&self, slot: usize, distance: Distance) -> Result<(usize, f32), String> {
        self.exit_ahead(slot, distance)
            .map(|(exit, ahead)| (exit.to, ahead))
    }

    /// [`Self::point_ahead`] with the route the point is on. Only a track trigger can name a
    /// route: it is on the route when a route's sample is nearer to its centre than the
    /// ring's, and the other distances are shares of the ring.
    ///
    /// # Errors
    ///
    /// As [`Self::point_ahead`].
    pub fn exit_ahead(&self, slot: usize, distance: Distance) -> Result<(Exit, f32), String> {
        let Some(course) = &self.sim.course else {
            return Err("the track has no course to move along".into());
        };
        let ship = &self.sim.world.ships[slot];
        let hint = ship.standing.course_index.map(|index| index as usize);
        let Some(from) = hint.or_else(|| {
            course
                .locate(ship.physics.body.position, None)
                .map(|located| located.index)
        }) else {
            return Err(format!("slot {slot}'s craft is not on the course"));
        };
        let length = course.length();
        let on = |to: usize| {
            let from = course.progress_at(from).unwrap_or(0.0);
            let to = course.progress_at(to).unwrap_or(0.0);
            (to - from).rem_euclid(length)
        };
        let units = match distance {
            Distance::Units(units) => units,
            Distance::Seconds(seconds) => seconds * self.speed_of(slot),
            Distance::Lap(share) => share * length,
            Distance::Progress(progress) => {
                let to = course.index_at_progress(progress).ok_or_else(|| {
                    format!("progress {progress} is not within [0, {length}), the lap")
                })?;
                return Ok((Exit::ring(to), on(to)));
            }
            Distance::Trigger(index) => {
                let centre = self
                    .sim
                    .extensions
                    .trigger_centre(usize::from(index))
                    .ok_or_else(|| format!("the track has no trigger {index}"))?;
                let exit = Exit::at(course, centre)
                    .ok_or_else(|| format!("trigger {index} is off the course"))?;
                return Ok((exit, on(exit.to)));
            }
        };
        let to = course
            .jump(from, units)
            .ok_or_else(|| format!("a move of {units} units is not within (0, {length}), a lap"))?;
        Ok((Exit::ring(to), units))
    }

    /// Whether a half disc of `radius`, standing on the track across its width at the
    /// width midpoint `distance` ahead of slot `slot`'s craft, fits there: the track is at
    /// least `2 * radius` wide, and no ray from the disc's centre out to its rim
    /// ([`CLEARANCE_RAYS`] of them, from one side across the top to the other) meets
    /// track geometry. With the distance probed, in track units; `(false, 0.0)` where the
    /// distance does not resolve or there is no course or spline to probe.
    #[must_use]
    pub fn clearance(&self, slot: usize, distance: Distance, radius: f32) -> (bool, f32) {
        let Ok((exit, ahead)) = self.exit_ahead(slot, distance) else {
            return (false, 0.0);
        };
        let Some((lateral, up, centre, width)) = self.road_frame(exit) else {
            return (false, ahead);
        };
        if width < radius * 2.0 {
            return (false, ahead);
        }
        let blocked = (0..CLEARANCE_RAYS).any(|k| {
            let angle = core::f32::consts::PI * k as f32 / (CLEARANCE_RAYS - 1) as f32;
            let (sin, cos) = oag_core::math::sin_cos(angle);
            let direction = lateral * cos + up * sin;
            let ray = oag_physics::collide::Ray::new(centre, direction, radius);
            oag_physics::collide::Raycaster::raycast(&*self.sim.collision, ray, None, false)
                .is_some()
        });
        (!blocked, ahead)
    }

    /// Where `exit` is: the ring's point, or the route's point that stands for it.
    fn exit_position(&self, exit: Exit) -> Option<Vec3> {
        let course = self.sim.course.as_ref()?;
        match usize::from(exit.route).checked_sub(1) {
            Some(route) => course.route_point(route, exit.to),
            None => course.position(exit.to),
        }
    }

    /// The spline sample under `exit`: the route's own deck for a route exit.
    pub(crate) fn road_sample(&self, exit: Exit) -> Option<&oag_vex::track::Sample> {
        let target = self.exit_position(exit)?;
        self.sim.spline.nearest(target).map(|(_, at, _)| at)
    }

    /// The road at `exit`: its lateral and up directions, the middle of its width lifted
    /// [`CLEARANCE_LIFT`] off the surface, and its width. `None` with no course position
    /// or spline near it. What a portal's probe and its drawn disc stand on.
    pub(crate) fn road_frame(&self, exit: Exit) -> Option<(Vec3, Vec3, Vec3, f32)> {
        let at = self.road_sample(exit)?;
        let lateral = Vec3::from_array(at.lateral);
        let up = -Vec3::from_array(at.down);
        let centre = Vec3::from_array(at.pos)
            + lateral * ((at.half_width_right - at.half_width_left) * 0.5)
            + up * CLEARANCE_LIFT;
        Some((
            lateral,
            up,
            centre,
            at.half_width_left + at.half_width_right,
        ))
    }

    /// Counts every transit down by a tick and brings out each craft whose count ran out
    /// ([`Self::relocate`]). Returns the slots that came out, one bit each. Nothing in a
    /// race without a transit.
    pub(super) fn advance_transits(&mut self) -> u32 {
        let mut out = 0;
        for slot in 0..self.sim.world.ship_count as usize {
            let Some(transit) = &mut self.sim.world.ships[slot].transit else {
                continue;
            };
            transit.ticks_left = transit.ticks_left.saturating_sub(1);
            if transit.ticks_left > 0 {
                continue;
            }
            let Transit {
                to,
                route,
                exit_speed,
                ..
            } = *transit;
            self.sim.world.ships[slot].transit = None;
            let exit = Exit {
                to: to as usize,
                route,
            };
            self.move_to(slot, exit, exit_speed);
            out |= 1 << slot;
        }
        out
    }

    /// Puts slot `slot`'s craft at `exit`, a course point forward of it and less than a lap
    /// on ([`Self::exit_ahead`] gives one), at the width midpoint facing
    /// down the track (the reset's own placement, [`crate::breadcrumbs::relocate`]),
    /// moving at `speed`. The ground between counts as driven: the lap counters credit a
    /// crossing of the line in it now ([`oag_race::Standing::jumped`],
    /// [`oag_race::RaceState::jumped`]), a finish included. Everything a race keys on where
    /// the craft is goes with it: the driver's index, the dwells, the pad sweep and the
    /// crumb ring ([`Self::reseat`]). Not a respawn: nothing is counted and nothing is
    /// charged. `false` when the course or the spline has nothing to place on, and the
    /// craft stays where it is.
    pub fn move_to(&mut self, slot: usize, exit: Exit, speed: f32) -> bool {
        let Some(course) = &self.sim.course else {
            return false;
        };
        let to = exit.to;
        let Some(target) = self.exit_position(exit) else {
            return false;
        };
        let Some(relocation) = crate::breadcrumbs::relocate(&self.sim.spline, target, speed) else {
            return false;
        };
        // The stamp a crossing driven in this tick's step gets: the lap counters run
        // after the world's count has moved on (`Race::tick_body`).
        let tick = self.sim.world.tick + 1;
        let laps_target = self.sim.world.laps_target();
        let ship = &mut self.sim.world.ships[slot];
        let body = &mut ship.physics.body;
        body.position = relocation.pose.position;
        body.orientation = relocation.pose.orientation;
        body.linear_velocity = relocation.velocity;
        body.angular_velocity = Vec3::ZERO;
        ship.standing.jumped(course, to, tick, laps_target);
        // Only a person's clock runs (`advance_player_rules`); an AI slot's is never read.
        if self.sim.world.person_flown(slot) {
            self.sim.world.race[slot].jumped(course, to);
        }
        self.sim.jumps[slot] = self.sim.jumps[slot].wrapping_add(1);
        self.sim.pad_previous_position[slot] = None;
        // A lane snap computed from where it was would put it back.
        if let Some(rail) = self.sim.autopilot_rail.get_mut(slot) {
            *rail = None;
        }
        self.sim.breadcrumbs[slot] =
            crate::breadcrumbs::Breadcrumbs::seeded(relocation.pose.position);
        self.reseat(slot, relocation.pose.position);
        // Told which line it is on, as a route's own samples are the nearest to a craft there.
        if let Some(route) = usize::from(exit.route).checked_sub(1) {
            self.seat_on_route(slot, route);
        }
        true
    }

    /// Whether slot `slot`'s reset triggers are held this tick, whatever their gates: a
    /// craft in a transit is out of the world and nothing may put it back.
    pub(super) fn reset_held(&self, slot: usize) -> bool {
        self.sim.world.ships[slot].transit.is_some()
    }
}
