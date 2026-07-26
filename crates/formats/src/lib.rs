//! Identification and parsing of the asset formats used by Wipeout titles.
//!
//! At this point **no Pulse format has been decoded**. What this crate provides
//! is the triage layer that comes first: given a pile of files pulled off a
//! disc, say what is obviously identifiable, what is compressed, and what
//! clusters together and therefore probably shares a format.
//!
//! Parsers land here as they are recovered. Each one gets a page under
//! `docs/formats/` recording the evidence for the layout, so the format
//! documentation and the implementation stay in step.

pub mod entropy;
pub mod signature;

pub use signature::{Signature, identify};
