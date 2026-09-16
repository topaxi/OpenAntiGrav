//! Asset containers, and what every decoder needs before it can start.
//!
//! Given a blob off a disc: say what it is, get it out of the archive it lives
//! in, decompress it, and undo the console's byte layout. What the bytes then
//! *mean* is somebody else's job - see below.
//!
//! [`signature`] and [`entropy`] are the triage layer: given a pile of files
//! pulled off a disc, say what is obviously identifiable, what is compressed,
//! and what clusters together and therefore probably shares a format.
//!
//! [`wad`] is the archive container Pure and Pulse use on PSP and PS2, and
//! [`psarc`] the one Wipeout HD uses on PS3; [`lzss`] is the compression a PS2
//! WAD entry carries and [`pure_dlc`] the encryption a Pure PSN pack does.
//! [`sblk`] and [`ps2_music`] are archives too - of sound rather than of files.
//!
//! [`byte_order`] and [`swizzle`] are the layer under all of them: which end of
//! a word comes first, and where in memory a texel actually sits. Both are byte
//! *layout*, which is why they are here and not with the decoders that consume
//! them - three different crates undo the same swizzle.
//!
//! The decoders themselves live in sibling crates, split by format family per
//! [ADR-0050](../../../docs/architecture/adr/0050-format-crates-split-by-format-family.md):
//! `oag-vex` for the `.vex` scene tree and its payloads, `oag-rcs` for the
//! `RCSMODEL` scene, `oag-texture` for pixel formats, `oag-tables` for the XML
//! a title authors, and `oag-video` for movie containers. Each keeps its
//! evidence page under `docs/formats/`.
//!
//! Every *Wipeout* format here is hand-rolled, and so is the MD5 [`psarc`]
//! checks its directory with. The one third-party dependency is `miniz_oxide`,
//! for the deflate streams a [`psarc`] block carries: a published standard
//! whose failure mode is silent garbage, and not a Wipeout format at all. It is
//! the only one in *every* feature combination, this crate having no features
//! at all - which is what moving the video path out bought.

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

pub use byte_order::ByteOrder;
pub use signature::{Signature, identify};
