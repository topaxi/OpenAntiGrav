//! Asset containers, and what every decoder needs before it can start.
//!
//! Given a blob off a disc: say what it is, get it out of its archive,
//! decompress it, and undo the console's byte layout. What the bytes then
//! *mean* is somebody else's job.
//!
//! [`signature`] and [`entropy`] are the triage layer: what is identifiable,
//! what is compressed, and what clusters and so probably shares a format.
//!
//! [`wad`] is the archive Pure and Pulse use on PSP and PS2, [`psarc`] the one
//! Wipeout HD uses on PS3; [`lzss`] is the compression a PS2 WAD entry carries
//! and [`pure_dlc`] the encryption a Pure PSN pack does. [`sblk`] and
//! [`ps2_music`] are archives of sound, and [`wwise`] is the PS4 Omega
//! Collection's Audiokinetic banks. [`xfx`] is the table HD drives a ship's
//! engine note with.
//!
//! [`byte_order`] and [`swizzle`] sit under all of them: byte *layout*, which is
//! why they are here and not with the decoders that consume them (three crates
//! undo the same swizzle).
//!
//! The decoders live in sibling crates, split by format family per
//! [ADR-0050](../../../docs/architecture/adr/0050-format-crates-split-by-format-family.md):
//! `oag-vex`, `oag-rcs`, `oag-texture`, `oag-tables` and `oag-video`, each
//! keeping its evidence page under `docs/formats/`.
//!
//! Every *Wipeout* format here is hand-rolled, as is the MD5 [`psarc`] checks
//! its directory with. The one third-party dependency is `miniz_oxide`, for
//! [`psarc`]'s deflate blocks: a published standard whose failure mode is silent
//! garbage. This crate has no features, so it is the only one in every
//! combination.

pub mod byte_order;
pub mod coverage;
pub mod entropy;
pub mod lzss;
pub mod ps2_music;
pub mod psarc;
pub mod pure_dlc;
pub mod sblk;
pub mod sblk_coverage;
pub mod signature;
pub mod swizzle;
pub mod wad;
pub mod wwise;
pub mod xfx;

pub use byte_order::ByteOrder;
pub use signature::{Signature, identify};
