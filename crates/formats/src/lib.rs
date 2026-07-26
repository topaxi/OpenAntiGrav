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

pub mod entropy;
pub mod fexml;
pub mod lzss;
pub mod png;
pub mod signature;
pub mod texture;
pub mod wad;

pub use signature::{Signature, identify};
