//! Ship dynamics and collision queries.
//!
//! The part of the simulation that must be bit-reproducible, so it obeys [`oag_core::math`]'s
//! rules without exception: `f32` only, no `mul_add`, no SIMD, no wall clock, no `HashMap`
//! iteration feeding arithmetic. It depends on `oag-core` and nothing else (not `oag-formats`,
//! so an on-disc schema change cannot ripple into the force law).
//!
//! # What is specification and what is measurement
//!
//! The model comes from `docs/physics/README.md` and
//! `docs/ghidra/functions/psp-pulse-usa/engine.md`, originally static analysis; individual
//! constants have since been read at runtime, and each doc comment carries its own confidence and
//! evidence. The *structure* is what is reproduced: the clamped delta, three explicit Euler
//! sub-steps of `dt/3`, two raycast probes as spring-dampers along the ship's own up axis, four
//! accumulators drained once, groundedness one frame stale for every control term, and constants
//! read from the player's own `handlingstats.xml`. Tests assert structural invariants (a spring
//! at rest length produces no force, groundedness quantises to three values, a mirrored input
//! gives a mirrored trajectory) unless a doc comment names a ground-truth test.
//!
//! Where the evidence records a term without its magnitude, the shape is implemented with the
//! identity or zero rather than an invented number. Everything else is read from [`Handling`],
//! the **already-scaled in-memory form** of the parameter set ([`params`]): do not apply the
//! loader's scale factors twice.
//!
//! # Where to start reading
//!
//! [`integrate::step`] is the only entry point a caller needs: it clamps the frame delta,
//! evaluates every force **once** through [`forces::evaluate`], then integrates
//! [`ship::SUBSTEPS`] explicit Euler sub-steps. The force law is split by subsystem, one module
//! per group of terms in the evidence:
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
//! `engine.md` is `docs/ghidra/functions/psp-pulse-usa/engine.md`, the authority where it and
//! `docs/physics/README.md` disagree (it corrected that page on `ride_height`, the `slidegrip`
//! reading, the gravity assembly and which frame's groundedness each term sees).
//!
//! # What is not here
//!
//! Everything that would need a trigger nobody has decoded: turbo's boost lift, the four-corner
//! hover variant and the auto-speed law behind its selector, and the flag-gated engine and
//! steering variants. `docs/physics/README.md`'s "what is implemented" section is the full list
//! and its still-open section records each sign and reading that had to be picked.

pub mod airbrake;
pub mod barrel_roll;
pub mod collide;
pub mod controls;
pub mod damage;
pub mod engine;
pub mod forces;
pub mod hover;
pub mod integrate;
pub mod launch;
pub mod maglock;
pub mod pair;
pub mod params;
pub mod passive;
pub mod probe;
pub mod reset;
pub mod ship;
pub mod slowdown;
pub mod wall;

pub use collide::{
    Aabb, CollisionWorld, Ray, RaycastHit, Raycaster, Surface, TriangleSoup, WALL_FRICTION,
    combine_friction, segment_triangle,
};
pub use damage::{CraftState, DamageRules, Shield};
pub use forces::{Accumulators, Environment, Evaluated};
pub use integrate::{clamp_dt, integrate, step};
pub use maglock::{Hold, MagContact, TrackSample};
pub use params::{Handling, SpeedClass};
pub use reset::ResetContact;
pub use ship::{Body, MAX_DT, SUBSTEPS, ShipControls, ShipState, Sideshift};
pub use wall::{WallContact, WallResponse};
