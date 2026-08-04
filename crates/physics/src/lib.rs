//! Ship dynamics and collision queries.
//!
//! This crate is the part of the simulation that must be bit-reproducible, so it
//! obeys [`oag_core::math`]'s rules without exception: `f32` only, no
//! `mul_add`, no SIMD, no wall clock, no `HashMap` iteration feeding arithmetic.
//! It depends on `oag-core` and nothing else - in particular not on
//! `oag-formats`, so that a change to an on-disc schema cannot ripple into the
//! force law.
//!
//! # What is specification and what is measurement
//!
//! The model implemented here comes from `docs/physics/README.md` and
//! `docs/ghidra/functions/psp-pulse-usa/engine.md`, both of which are **static analysis of
//! the original that has never been run**. The *structure* is what this crate
//! reproduces: the clamped delta, the three explicit Euler sub-steps of `dt/3`, two
//! raycast probes as spring-dampers along the ship's own up axis, four accumulators
//! drained once, groundedness that is one frame stale for every control term, and
//! constants read from the player's own `handlingstats.xml`. Nothing here claims
//! fidelity, and the tests assert structural invariants (a spring at rest length
//! produces no force, groundedness quantises to three values, a mirrored input produces
//! a mirrored trajectory) rather than that anything feels right. Tuning waits for M3's
//! trace comparison, per `docs/overview/roadmap.md`.
//!
//! Where the evidence records that a term exists without recording its magnitude, the
//! shape is implemented and the coefficient is deliberately the identity or zero rather
//! than an invented number: [`engine::ENGINE_OUTPUT_SCALE`] is what is left of that
//! list. Three have left it since it was written - [`hover::TARGET_GLOBAL_SCALE`] and
//! [`hover::DOWNFORCE_SCALE`] are both read now, the second of them turning out
//! to be the crate's missing pitch damping as well as its missing force, and
//! [`forces::Environment::class_gravity_scale`] is supplied from the disc rather than
//! defaulted. Everything else is read from
//! [`Handling`], which holds
//! the **already-scaled in-memory form** of the parameter set - see [`params`], and do
//! not apply the loader's scale factors twice.
//!
//! # Where to start reading
//!
//! [`integrate::step`] is the only entry point a caller needs: it clamps the frame
//! delta, evaluates every force **once** through [`forces::evaluate`], and then
//! integrates [`ship::SUBSTEPS`] explicit Euler sub-steps. The force law itself is
//! split by subsystem, one module per group of terms in the evidence:
//!
//! | Module | Terms | Evidence |
//! | --- | --- | --- |
//! | [`controls`] | The five control states and their ramps | `engine.md` |
//! | [`engine`] | Engine, brakes, steering, pitch | `engine.md` |
//! | [`passive`] | Drag, rolling resistance, weathervane, angular and vertical damping, gravity | `engine.md` |
//! | [`hover`] | The two-probe air cushion and the grounded-only terms | `physics/README.md` |
//! | [`maglock`] | The magstrip hold: a kinematic basis rewrite, not a force | `engine.md` |
//! | [`airbrake`] | Airbrakes, lateral grip, sideshift | both |
//! | [`collide`] | The segment queries the probes ask | `collision.md` |
//! | [`wall`] | Hull-versus-wall contact and its response | `collision.md` |
//! | [`reset`] | Contact with `Reset` trigger geometry | `collision.md` |
//! | [`forces`] | Assembly, and the four accumulators | `engine.md` |
//!
//! `engine.md` is `docs/ghidra/functions/psp-pulse-usa/engine.md` and is the authority
//! where it and `docs/physics/README.md` disagree; it corrected that page on
//! `ride_height`, on the `slidegrip` reading, on the gravity assembly and on which
//! frame's groundedness each term sees.
//!
//! # What is not here
//!
//! Everything that would need a trigger nobody has decoded: turbo and the boost lift,
//! the four-corner hover variant and the auto-speed law behind its selector,
//! and the flag-gated engine and steering
//! variants. `docs/physics/README.md`'s "what is implemented" section is the full
//! list, and the still-open section is where each sign and reading that had to be
//! picked is recorded.

pub mod airbrake;
pub mod collide;
pub mod controls;
pub mod engine;
pub mod forces;
pub mod hover;
pub mod integrate;
pub mod maglock;
pub mod params;
pub mod passive;
pub mod probe;
pub mod reset;
pub mod ship;
pub mod wall;

pub use collide::{
    Aabb, CollisionWorld, Ray, RaycastHit, Raycaster, Surface, TriangleSoup, WALL_FRICTION,
    combine_friction, segment_triangle,
};
pub use forces::{Accumulators, Environment, Evaluated};
pub use integrate::{clamp_dt, integrate, step};
pub use maglock::{Hold, MagContact, TrackSample};
pub use params::{Handling, SpeedClass};
pub use reset::ResetContact;
pub use ship::{Body, MAX_DT, SUBSTEPS, ShipControls, ShipState, Sideshift};
pub use wall::{WallContact, WallResponse};
