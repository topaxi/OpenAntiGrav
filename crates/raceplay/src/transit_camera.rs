//! The chase camera of a craft in a transit (`crate::transit`).
//!
//! The craft is frozen where it went in, off the road's middle and turned however it flew, and
//! comes out on the middle facing down the road (`crate::breadcrumbs::emerge`). A camera left on
//! the frozen pose would be cut across that difference at the exit, and a mod's view through a
//! portal pair (`crate::portal_view`), which carries this camera from the entrance to the exit,
//! would show the road from somewhere the camera never stands. So while a craft is out of the
//! world its camera eases onto its seat: the pose the craft has on its exit tick, carried back
//! through the pair of portal frames at the craft's own lap progress and at its exit. A
//! `portal-view` from that progress to that exit then shows, by the end of the transit, the
//! picture the camera has when the craft is out.
//!
//! View-side: nothing here is simulation state.

use oag_race::chase_rig::Target;

/// Ticks the camera takes to come round onto the seat.
const SEAT_TICKS: f32 = 12.0;

/// How many times faster the camera's spring closes on a seat than on a craft: the spring's
/// rate is authored for a craft that moves, and a seat has to be reached before the transit
/// ends.
const SEAT_SPRING: f32 = 4.0;

/// Where a craft in a transit is followed from, and how far the camera has come round to it.
#[derive(Debug, Clone, Copy)]
pub(super) struct Seat {
    target: Target,
    share: f32,
}

impl crate::Race {
    /// Steps every craft's seat one tick: opened when its transit starts, dropped when it ends.
    /// Before the cameras advance.
    pub(super) fn advance_transit_seats(&mut self) {
        for slot in 0..oag_gameplay::MAX_SHIPS {
            let ship = &self.sim.world.ships[slot];
            let seat = match (ship.transit, self.view.transit_seats[slot]) {
                (None, _) => None,
                (Some(_), Some(seat)) => Some(Seat {
                    share: (seat.share + 1.0 / SEAT_TICKS).min(1.0),
                    ..seat
                }),
                (Some(transit), None) => self.transit_seat(slot, transit),
            };
            self.view.transit_seats[slot] = seat;
        }
    }

    /// The step the spring of `slot`'s camera takes this tick: the tick's own, or
    /// [`SEAT_SPRING`] of them while it is closing on a seat.
    pub(super) fn camera_step(&self, slot: usize) -> f32 {
        match self.view.transit_seats.get(slot) {
            Some(Some(_)) => self.sim.dt * SEAT_SPRING,
            _ => self.sim.dt,
        }
    }

    /// The seat of `slot`'s craft as its transit starts. `None` with no course or road to
    /// frame the pair on, which leaves the camera on the frozen craft.
    fn transit_seat(&self, slot: usize, transit: oag_gameplay::Transit) -> Option<Seat> {
        use oag_core::math::Vec3;
        use oag_render::camera::portal::{Face, through};

        let course = self.sim.course.as_ref()?;
        let stands = self.sim.world.ships[slot].standing.progress?;
        let entrance = crate::transit::Exit::ring(course.index_at_progress(stands)?);
        let exit = crate::transit::Exit {
            to: transit.to as usize,
            route: transit.route,
        };
        let back = through(
            self.portal_frame(entrance, Face::Oncoming)?,
            self.portal_frame(exit, Face::Departing)?,
        )
        .inverse();
        let placed = crate::breadcrumbs::emerge(
            &self.sim.spline,
            self.exit_position(exit)?,
            transit.exit_speed,
        )?;
        // It flies the exit tick's step before its camera first sees it.
        let position = placed.pose.position + placed.velocity * self.sim.dt;
        Some(Seat {
            target: Target {
                position: back.transform_point3(position),
                forward: back.transform_vector3(placed.pose.orientation * Vec3::NEG_Z),
                up: back.transform_vector3(placed.pose.orientation * Vec3::Y),
            },
            share: 1.0 / SEAT_TICKS,
        })
    }

    /// `target`, `slot`'s craft as a camera follows it, eased onto its seat while it is in a
    /// transit; `target` itself for a craft in the world.
    pub(super) fn seated(&self, slot: usize, target: Target) -> Target {
        let Some(seat) = self.view.transit_seats.get(slot).copied().flatten() else {
            return target;
        };
        let share = seat.share * seat.share * (3.0 - 2.0 * seat.share);
        let turned = |from: oag_core::math::Vec3, to: oag_core::math::Vec3| {
            let blended = from + (to - from) * share;
            if blended.length_squared() > 0.0 {
                blended.normalize()
            } else {
                to
            }
        };
        Target {
            position: target.position + (seat.target.position - target.position) * share,
            forward: turned(target.forward, seat.target.forward),
            up: turned(target.up, seat.target.up),
        }
    }
}
