//! Identification and parsing of the asset formats used by Wipeout titles.
//!
//! Two layers live here.
//!
//! [`signature`] and [`entropy`] are the triage layer: given a pile of files
//! pulled off a disc, say what is obviously identifiable, what is compressed,
//! and what clusters together and therefore probably shares a format.
//!
//! [`wad`] is the first decoded Wipeout format: the archive container used by
//! Pure and Pulse on both PSP and PS2.
//!
//! Parsers land here as they are recovered. Each one gets a page under
//! `docs/formats/` recording the evidence for the layout, so the documentation
//! and the implementation stay in step.
//!
//! Every *Wipeout* format here is hand-rolled, and so is the MD5 [`psarc`]
//! checks its directory with. The one
//! third-party dependency is `miniz_oxide`, for the deflate streams a
//! [`psarc`] block carries: a published standard whose failure mode is silent
//! garbage, and not a Wipeout format at all. It is the *only* one, in every
//! feature combination, which is what moving the video path to `oag-video`
//! bought - see
//! [ADR-0050](../../../docs/architecture/adr/0050-format-crates-split-by-format-family.md).

pub mod byte_order;
pub mod collision;
pub mod coverage;
pub mod entropy;
pub mod fog;
pub mod gxp;
pub mod hd_pvs;
pub mod kdcol;
pub mod lighting;
pub mod lzss;
pub mod pads;
pub mod pob;
pub mod ps2_music;
pub mod psarc;
pub mod pure_dlc;
pub mod pvs;
pub mod rcsmaterial;
pub mod rcsmodel;
pub mod sblk;
pub mod shadow_occluder;
pub mod signature;
pub mod sound_emitters;
pub mod swizzle;
pub mod track;
pub mod vex;
pub mod vif;
pub mod wad;

pub use byte_order::ByteOrder;
pub use signature::{Signature, identify};
