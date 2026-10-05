//! Finding a title's disc image, opening it as whichever title it is, and mounting
//! downloadable content behind it.
//!
//! Split out of `oag-game` because three consumers sit on opposite sides of the
//! race: the front end's boot, the sound crate's host and `oag-raceplay`'s race load
//! all open a source, and none of them may depend on another. This crate knows every
//! title package and [`oag_assets::Archives`], which `oag-title` deliberately does
//! not, and knows no race, menu, window or settings file.
//!
//! - [`source`]: the image search path and the container rules.
//! - [`title`]: [`title::open_source`], a source opened as whichever title it is.
//! - [`dlc`]: the downloadable-content directories and the packs in them.
//! - [`remix`]: a race's track and craft sources, one or two.
//! - [`cache`]: where derived movie, audio and DLC copies live.

pub mod cache;
pub mod dlc;
pub mod remix;
pub mod source;
pub mod title;
