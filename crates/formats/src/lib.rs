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
//! feature, off by default, so a crate that depends on this one for Wipeout
//! containers alone pulls in no dependencies at all. It is not linked above
//! because the link would not resolve with the feature off, which is how this
//! crate is documented by default.

#[cfg(feature = "av1")]
pub mod av1;
pub mod collision;
pub mod entropy;
pub mod fexml;
pub mod fnt;
pub mod handling;
pub mod ipf;
pub mod ivf;
pub mod lzss;
pub mod pmf;
pub mod png;
pub mod ps2_music;
pub mod ps2_texture;
pub mod sblk;
pub mod signature;
pub mod texture;
pub mod track;
pub mod vex;
pub mod vif;
pub mod wad;

pub use signature::{Signature, identify};
