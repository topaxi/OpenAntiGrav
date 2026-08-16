//! What the `.vex` scene reader in [`super`] is asserted to do, by theme.
//!
//! Split out of `vex.rs`'s `#[cfg(test)] mod tests`, which was 1,108
//! lines - past the 200 an inline test module may hold, and past the 1,000
//! a file may. See `scripts/check-file-size.py`, which is the rule as a
//! gate.

mod primitives;
mod ps2;
mod tex_transform;
mod textures;
mod tree;
mod vertices;
