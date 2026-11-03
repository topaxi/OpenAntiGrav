//! A craft's jump along the circuit (a simulation mod's `transit`): out of the world for
//! a number of ticks, then put further round at a given speed.
//!
//! - **Entry** ([`Race::begin_transit`]): the distance is resolved at once (track units,
//!   seconds at the speed it has now, or a share of the lap) and checked against
//!   [`oag_race::Course::jump`]'s bound; the craft keeps its pose and is frozen there.
//! - **Out of the world** ([`oag_gameplay::Ship::in_world`]): not stepped (no physics, no
//!   pads, no triggers, its controls and pickup presses ignored), not drawn, no
//!   craft-to-craft contact, and `oag_weapons::Craft::active` is false, so no weapon hits,
//!   drains or locks it. Its reset triggers are held ([`Race::reset_held`]). Its standing
//!   is still updated on the frozen pose, so nothing wraps and the Zone clock runs.
//! - **Exit** ([`Race::advance_transits`], at the start of phase C): on the tick the
//!   count runs out, the craft is placed [`oag_gameplay::Transit::distance`] further
//!   along ([`Race::relocate`]) and stepped from there the same tick. The lap counters
//!   see the jump on that tick's update, so a crossing of the line in the jump, a finish
//!   included, is timed at the exit tick.

use oag_core::math::Vec3;
use oag_gameplay::Transit;

use crate::Race;

pub use oag_ext_abi::Distance;
use oag_ext_abi::{MAX_SPEED_FACTOR, MAX_TRANSIT_TICKS};

/// How many rays a clearance probe casts round its half disc, end to end. Chosen.
const CLEARANCE_RAYS: usize = 9;
/// How far above the track surface a clearance probe's rays start, so the floor under
/// the disc's straight edge is not read as an obstacle. Chosen.
const CLEARANCE_LIFT: f32 = 0.5;

impl Race {
    /// Starts slot `slot`'s transit: `ticks` out of the world, then `distance` further
    /// along at `exit_factor` times the speed it has now. A refusal names why and changes
    /// nothing: no course to jump along, a craft not racing or already in a transit, a
    /// tick count of 0 or past [`MAX_TRANSIT_TICKS`], a distance not inside
    /// `(0, half a lap)`.
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
        let units = self.resolve_distance(slot, distance)?;
        let speed = self.sim.world.ships[slot]
            .physics
            .body
            .linear_velocity
            .length();
        self.sim.world.ships[slot].transit = Some(Transit {
            ticks_left: ticks,
            distance: units,
            exit_speed: speed * exit_factor,
        });
        Ok(())
    }

    /// `distance` from slot `slot`'s craft as track units, checked against
    /// [`oag_race::Course::max_jump`].
    fn resolve_distance(&self, slot: usize, distance: Distance) -> Result<f32, String> {
        let Some(course) = &self.sim.course else {
            return Err("the track has no course to jump along".into());
        };
        let speed = self.sim.world.ships[slot]
            .physics
            .body
            .linear_velocity
            .length();
        let units = match distance {
            Distance::Units(units) => units,
            Distance::Seconds(seconds) => seconds * speed,
            Distance::Lap(share) => share * course.length(),
        };
        if units.is_finite() && units > 0.0 && units < course.max_jump() {
            Ok(units)
        } else {
            Err(format!(
                "a jump of {units} units is not within (0, {}), half the lap",
                course.max_jump()
            ))
        }
    }

    /// Whether a half disc of `radius`, standing on the track across its width at the
    /// width midpoint `distance` ahead of slot `slot`'s craft, fits there: the track is at
    /// least `2 * radius` wide, and no ray from the disc's centre out to its rim
    /// ([`CLEARANCE_RAYS`] of them, from one side across the top to the other) meets
    /// track geometry. With the distance probed, in track units; `(false, 0.0)` where the
    /// distance does not resolve or there is no course or spline to probe.
    #[must_use]
    pub fn clearance(&self, slot: usize, distance: Distance, radius: f32) -> (bool, f32) {
        let Ok(ahead) = self.resolve_distance(slot, distance) else {
            return (false, 0.0);
        };
        let Some(course) = &self.sim.course else {
            return (false, 0.0);
        };
        let ship = &self.sim.world.ships[slot];
        let from = ship
            .standing
            .course_index
            .map(|index| index as usize)
            .or_else(|| {
                course
                    .locate(ship.physics.body.position, None)
                    .map(|located| located.index)
            });
        let Some(target) = from
            .and_then(|from| course.jump(from, ahead))
            .and_then(|to| course.position(to))
        else {
            return (false, ahead);
        };
        let Some((_, at, _)) = self.sim.spline.nearest(target) else {
            return (false, ahead);
        };
        if at.half_width_left + at.half_width_right < radius * 2.0 {
            return (false, ahead);
        }
        let lateral = Vec3::from_array(at.lateral);
        let up = -Vec3::from_array(at.down);
        let centre = Vec3::from_array(at.pos)
            + lateral * ((at.half_width_right - at.half_width_left) * 0.5)
            + up * CLEARANCE_LIFT;
        let blocked = (0..CLEARANCE_RAYS).any(|k| {
            let angle = core::f32::consts::PI * k as f32 / (CLEARANCE_RAYS - 1) as f32;
            let (sin, cos) = oag_core::math::sin_cos(angle);
            let direction = lateral * cos + up * sin;
            let ray = oag_physics::collide::Ray::new(centre, direction, radius);
            oag_physics::collide::Raycaster::raycast(&self.sim.collision, ray, None, false)
                .is_some()
        });
        (!blocked, ahead)
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
                distance,
                exit_speed,
                ..
            } = *transit;
            self.sim.world.ships[slot].transit = None;
            self.relocate(slot, distance, exit_speed);
            out |= 1 << slot;
        }
        out
    }

    /// Puts slot `slot`'s craft `distance` units further round the circuit, at the
    /// width midpoint facing down the track (the reset's own placement,
    /// [`crate::breadcrumbs::relocate`]), moving at `speed`. Everything a race keys on
    /// where the craft is goes with it: the lap counters' search ([`oag_race::Standing::jumped`]),
    /// the driver's index, the dwells, the pad sweep and the crumb ring. Not a respawn:
    /// nothing is counted and nothing is charged. `false` when the course or the spline
    /// has nothing to place on, and the craft stays where it is.
    pub fn relocate(&mut self, slot: usize, distance: f32, speed: f32) -> bool {
        let Some(course) = &self.sim.course else {
            return false;
        };
        let ship = &self.sim.world.ships[slot];
        let from = ship
            .standing
            .course_index
            .map(|index| index as usize)
            .or_else(|| {
                course
                    .locate(ship.physics.body.position, None)
                    .map(|located| located.index)
            });
        let Some(to) = from.and_then(|from| course.jump(from, distance)) else {
            return false;
        };
        let Some(target) = course.position(to) else {
            return false;
        };
        let Some(relocation) = crate::breadcrumbs::relocate(&self.sim.spline, target, speed) else {
            return false;
        };
        let ship = &mut self.sim.world.ships[slot];
        let body = &mut ship.physics.body;
        body.position = relocation.pose.position;
        body.orientation = relocation.pose.orientation;
        body.linear_velocity = relocation.velocity;
        body.angular_velocity = Vec3::ZERO;
        ship.standing.jumped(to);
        if let Some(race) = self.sim.world.race.get_mut(slot) {
            race.jumped(to);
        }
        self.sim.pad_previous_position[slot] = None;
        self.sim.breadcrumbs[slot] =
            crate::breadcrumbs::Breadcrumbs::seeded(relocation.pose.position);
        self.reseat(slot, relocation.pose.position);
        true
    }

    /// Whether slot `slot`'s reset triggers are held this tick, whatever their gates: a
    /// craft in a transit is out of the world and nothing may put it back.
    pub(super) fn reset_held(&self, slot: usize) -> bool {
        self.sim.world.ships[slot].transit.is_some()
    }
}
