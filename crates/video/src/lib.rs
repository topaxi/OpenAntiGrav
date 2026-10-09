//! Video containers, and the one codec this project decodes itself.
//!
//! Two kinds of thing live here, and the split is worth stating because they
//! point in opposite directions.
//!
//! [`pmf`], [`ipf`], [`pss`], [`bik`] and [`mp4`] are what the *originals*
//! ship: the PSP movie file, the PS2 IPU wrapper, the PS2's loose MPEG-2
//! program streams, RAD's Bink container as Wipeout HD uses it, and the
//! ISOBMFF container Wipeout 2048 uses. They are read-only, and none of them
//! is decoded here - each one demuxes to a bitstream that something outside
//! this crate plays.
//!
//! [`ivf`] and `av1` are what *this project* writes. Per
//! [ADR-0008](../../../docs/architecture/adr/0008-av1-movie-cache.md) a `.PMF`
//! is transcoded once into an AV1-in-IVF cache rather than an H.264 decoder
//! being reproduced to show two logo cards at boot. `av1` is behind the `av1`
//! cargo feature, off by default, which is why it is not linked above: the link
//! would not resolve in the default build, and that is how this crate is
//! documented.
//!
//! Nothing here is a Wipeout *asset* format - those are in `oag-formats` and
//! its siblings. Video was separated out under
//! [ADR-0050](../../../docs/architecture/adr/0050-format-crates-split-by-format-family.md)
//! so that the asset-format crates carry no codec dependency in any feature
//! combination.

pub mod av1;
pub mod bik;
pub mod ipf;
pub mod ivf;
pub mod mp4;
pub mod pmf;
pub mod pss;
