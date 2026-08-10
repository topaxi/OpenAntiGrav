//! The simulation: one `World` of plain data, an input snapshot in, a state
//! snapshot out.
//!
//! This crate is the seam the whole architecture is built around. It knows about
//! `oag-physics` and `oag-formats` and nothing else - no renderer, no audio, no
//! windowing, no disc I/O - which is what makes a race reproducible on a machine
//! with no GPU and comparable against a trace from the original. See
//! `docs/architecture/workspace-layout.md`.
//!
//! Gameplay state is a single struct of fixed-size arrays rather than an ECS, so
//! that snapshotting a race for a replay or a golden test is one
//! `memcpy`-shaped operation and there is no scheduler whose ordering could
//! become a determinism hazard. See
//! `docs/architecture/adr/0003-no-ecs.md`.

pub mod collision;
pub mod controls;
pub mod handling;
pub mod input;
pub mod spawn;
pub mod world;

pub use collision::collision_world;
pub use controls::{ControlScheme, ship_controls};
pub use handling::{AirbrakeGraphics, airbrake_graphics_for, handling_for, to_format_class};
pub use input::InputSnapshot;
pub use spawn::{GRID_COLUMN_OFFSET, GRID_ROW_PITCH, GRID_SLOTS, Pose, grid_pose};
pub use world::{MAX_SHIPS, Ship, World, damage_rules};
