//! The chase camera of a craft in a transit (`crate::transit`), and the craft as that camera
//! sees it.
//!
//! The craft is frozen where it went in, off the road's middle and turned however it flew, and
//! comes out on the middle facing down the road (`crate::breadcrumbs::emerge`). Three things
//! follow from one pose, the craft's *seat*: the pose it has on its exit tick, carried back
//! through the pair of portal frames at the craft's own lap progress and at its exit.
//!
//! - **The camera flies on.** Its target eases from the frozen craft onto the seat and keeps
//!   going along the seat's heading at the speed the craft went in at, so the world does not
//!   stop at the entrance: the camera goes through where the craft did.
//! - **The pair is seen from the seat.** A `portal-view` of a slot whose craft is in a transit
//!   (`crate::portal_view`) is framed by that camera carried back by the distance flown, so by
//!   the end of the transit it shows, from the craft's progress to its exit, the picture the
//!   camera has when the craft is out.
//! - **The craft is drawn there for its own view under a backdrop** ([`Race::ship_drawn`]): a
//!   mod's backdrop fill takes the world out of the picture, and the craft flying in front of
//!   its camera stays in it. No other view draws it: it is out of the world.
//!
//! View-side: nothing here is simulation state.

use oag_core::math::{Mat4, Vec3};
use oag_race::chase_rig::Target;

/// Ticks the camera takes to come round onto the seat.
const SEAT_TICKS: f32 = 12.0;

/// How many times faster the camera's spring closes on a seat than on a craft: the spring's
/// rate is authored for a craft that moves, and a seat has to be reached before the transit
/// ends.
const SEAT_SPRING: f32 = 4.0;

/// Where a craft in a transit is followed from, how far the camera has come round to it, and
/// how far the craft has flown on since it went in.
#[derive(Debug, Clone, Copy)]
pub(super) struct Seat {
    target: Target,
    share: f32,
    /// Ticks since the transit began.
    ticks: f32,
    /// What the craft went in at, units a tick.
    speed: f32,
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
                    ticks: seat.ticks + 1.0,
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
        use oag_render::camera::portal::{Face, through};

        let course = self.sim.course.as_ref()?;
        let ship = &self.sim.world.ships[slot];
        let stands = ship.standing.progress?;
        let speed = ship.physics.body.linear_velocity.length() * self.sim.dt;
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
            ticks: 0.0,
            speed,
        })
    }

    /// `target`, `slot`'s craft as a camera follows it: for a craft in a transit, eased onto
    /// its seat and flown on from there; `target` itself for a craft in the world.
    pub(super) fn seated(&self, slot: usize, target: Target) -> Target {
        let Some(seat) = self.view.transit_seats.get(slot).copied().flatten() else {
            return target;
        };
        let share = seat.share * seat.share * (3.0 - 2.0 * seat.share);
        let turned = |from: Vec3, to: Vec3| {
            let blended = from + (to - from) * share;
            if blended.length_squared() > 0.0 {
                blended.normalize()
            } else {
                to
            }
        };
        let forward = turned(target.forward, seat.target.forward);
        let eased = target.position + (seat.target.position - target.position) * share;
        Target {
            position: eased + forward * (seat.speed * seat.ticks),
            forward,
            up: turned(target.up, seat.target.up),
        }
    }

    /// `slot`'s craft as a camera follows it this tick.
    pub(super) fn followed(&self, slot: usize) -> Target {
        let mut target = crate::camera::target_of(&self.sim.world.ships[slot]);
        target.position = self.drawn_position(slot);
        self.seated(slot, target)
    }

    /// How far `slot`'s craft has flown on from its seat at the instant being drawn, and
    /// which way: `None` for a craft in the world.
    fn flown(&self, slot: usize) -> Option<(Vec3, f32)> {
        let seat = self.view.transit_seats.get(slot).copied().flatten()?;
        // A draw between two ticks stands that far short of this tick's distance.
        let alpha = self.draw_alpha();
        let ticks = if alpha < 1.0 && seat.ticks >= 1.0 {
            seat.ticks - 1.0 + alpha
        } else {
            seat.ticks
        };
        Some((self.followed(slot).forward, seat.speed * ticks))
    }

    /// The view the pair of the craft being viewed is seen from: [`Self::view`], carried back
    /// by what the craft has flown on while it is in a transit, which is the camera on its
    /// seat.
    pub(crate) fn pair_view(&self) -> Mat4 {
        match self.flown(self.viewed_slot()) {
            Some((forward, distance)) => self.view() * Mat4::from_translation(forward * distance),
            None => self.view(),
        }
    }

    /// The model matrix of `slot`'s craft where its camera follows it in a transit; `None`
    /// for a craft in the world, which is where the simulation has it.
    pub(super) fn flown_model(&self, slot: usize) -> Option<Mat4> {
        self.view.transit_seats.get(slot).copied().flatten()?;
        Some(crate::drawable::posed_model_matrix(&self.followed(slot)))
    }

    /// Whether the view being drawn is `slot`'s own, of a craft in a transit, under a backdrop
    /// fill: the one view that draws a craft that is out of the world.
    #[must_use]
    pub fn ship_flown_in_view(&self, slot: usize) -> bool {
        matches!(self.view.transit_seats.get(slot), Some(Some(_)))
            && self.viewed_slot() == slot
            && self.mod_fill().is_some_and(|fill| {
                self.mod_shaders()
                    .get(fill.shader)
                    .and_then(Option::as_ref)
                    .is_some_and(|shader| shader.compiled.backdrop)
            })
    }

    /// Whether `slot`'s hull is drawn in the view being drawn: a craft in the world
    /// ([`Self::ship_active`]), or the view's own craft flown through a backdrop
    /// ([`Self::ship_flown_in_view`]).
    #[must_use]
    pub fn ship_drawn(&self, slot: usize) -> bool {
        self.ship_active(slot) || self.ship_flown_in_view(slot)
    }

    /// The rigid frame of the craft the view being drawn follows, in the world: its right, up
    /// and back, and where it is, with no scale, lean or shake. For a craft in a transit, the
    /// line it is flown along (its seat, flown on), which the drawn hull eases onto over its
    /// first ticks: a frame that stands still on the road it left, not one that swings with the
    /// ease. What a mod's shader reads as `craft_frame` once it is in eye space.
    pub(crate) fn craft_frame(&self) -> Mat4 {
        let slot = self.viewed_slot();
        let seat = self.view.transit_seats.get(slot).copied().flatten();
        let (target, place) = match (seat, self.flown(slot)) {
            (Some(seat), Some((_, distance))) => {
                let forward = seat.target.forward.normalize_or_zero();
                (seat.target, seat.target.position + forward * distance)
            }
            // The drawn hull's place, which a draw between two ticks blends as the camera's.
            _ => (
                self.followed(slot),
                self.ship_model_matrix_of(slot).w_axis.truncate(),
            ),
        };
        let forward = target.forward.normalize_or_zero();
        let right = forward.cross(target.up).normalize_or_zero();
        let up = right.cross(forward);
        Mat4::from_cols(
            right.extend(0.0),
            up.extend(0.0),
            (-forward).extend(0.0),
            place.extend(1.0),
        )
    }
}
