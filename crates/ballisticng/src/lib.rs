//! BallisticNG's Workshop content, read on top of the generic Unity reader.
//!
//! [`oag_unity`] knows Unity and no game. This crate knows what one game's
//! `MonoBehaviour` classes mean: the `.wtrk` and `.csf` item wrappers, a track's
//! `TrackDataStorage`, its `Section` chain, `Tile`s and AI line, and a whole
//! texture decoded through `oag_texture::bcn`. Nothing here maps a track into
//! this engine; that is a choice, not a reading, and lives in the consumer
//! (ADR-0062).

pub mod error;
mod read;
pub mod texture;
pub mod track;
pub mod wtrk;

pub use error::{Error, Result};
