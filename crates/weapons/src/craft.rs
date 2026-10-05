//! The craft, as the weapons see it.
//!
//! `oag_gameplay::Ship` carries a driver, a standing, handling and more, none of
//! which a weapon reads. This trait is the handful of fields a weapon does read
//! or write, so the weapons crate can sit below the crate that owns `Ship`.
//! Generic use, not `dyn`: monomorphisation keeps the float operations exactly
//! as they were when the code named `Ship` directly.

use oag_physics::{ShipState, params::Dimensions};

use crate::disruption::Disruption;

/// What a weapon needs of a craft.
pub trait Craft {
    /// Whether the slot holds a craft at all. An inactive slot is neither hit
    /// nor drained.
    fn active(&self) -> bool;

    /// The craft's dynamics: pose, velocity, shield.
    fn physics(&self) -> &ShipState;

    /// [`Self::physics`], writable.
    fn physics_mut(&mut self) -> &mut ShipState;

    /// The authored hull dimensions, which size a blast's reach and a beam's
    /// anchor.
    fn dimensions(&self) -> Dimensions;

    /// Seconds of slowdown credited this tick and not yet drained into the
    /// engine's timer; see [`crate::slowdown::drain`].
    fn pending_slowdown(&self) -> f32;

    /// [`Self::pending_slowdown`], writable.
    fn pending_slowdown_mut(&mut self) -> &mut f32;

    /// The thrust scale the craft's next engine step consumes; a beam's slow
    /// effect arms it.
    fn pending_thrust_scale_mut(&mut self) -> &mut f32;

    /// The disruption currently running on the craft.
    fn disruption(&self) -> &Disruption;

    /// [`Self::disruption`], writable.
    fn disruption_mut(&mut self) -> &mut Disruption;

    /// How far round the circuit the craft is, as the standings last computed
    /// it, or `None` before it has been located.
    fn progress(&self) -> Option<f32>;

    /// The ring point the craft was last located at on the course, which is
    /// where a repulser reads the corridor width.
    fn course_index(&self) -> Option<u32>;
}
