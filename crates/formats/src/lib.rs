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
//! [`ivf`] and `av1` are the exception to all of the above: they are not
//! Wipeout formats but the container and codec the project's **own** movie
//! cache uses, holding what `.PMF` video is transcoded into. They live here so
//! the whole video path stays in one crate. `av1` is behind the `av1` cargo
//! feature, off by default. It is not linked above because the link would not
//! resolve with the feature off, which is how this crate is documented by
//! default.
//!
//! Every *Wipeout* format here is hand-rolled, and so are the two checksums
//! [`png`] needs and the MD5 [`psarc`] checks its directory with. The one
//! third-party dependency in the default build is `miniz_oxide`, for the
//! deflate streams a [`psarc`] block carries: a published standard whose
//! failure mode is silent garbage, and not a Wipeout format at all.

#[cfg(feature = "av1")]
pub mod av1;
mod bcn;
pub mod bik;
pub mod byte_order;
pub mod collision;
pub mod coverage;
pub mod entropy;
pub mod envsettings;
pub mod fexml;
pub mod fnt;
pub mod fog;
pub mod gtf;
pub mod gxt;
pub mod handling;
pub mod hd_pvs;
pub mod ipf;
pub mod ivf;
pub mod kdcol;
pub mod lighting;
pub mod lzss;
pub mod pads;
pub mod pmf;
pub mod png;
pub mod pob;
pub mod ps2_music;
pub mod ps2_texture;
pub mod psarc;
mod pvrtc;
pub mod pvs;
pub mod rcsmaterial;
pub mod rcsmodel;
pub mod sblk;
pub mod signature;
pub mod texture;
pub mod track;
pub mod trackstartup;
pub mod vex;
pub mod vif;
pub mod wad;
pub mod weapons;

pub use byte_order::ByteOrder;
pub use signature::{Signature, identify};
