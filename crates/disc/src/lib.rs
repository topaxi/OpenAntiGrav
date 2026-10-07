//! Reading PSP UMD, PS2 DVD and PS3 BD-ROM disc images.
//!
//! Two container formats are supported, sniffed by magic rather than by file
//! extension:
//!
//! - **CHD** (MAME Compressed Hunks of Data), via the `chd` crate.
//! - **Raw ISO**, a flat sequence of 2048-byte sectors.
//!
//! Both are presented as a [`SectorSource`], and [`iso9660`] walks the
//! filesystem on top of that. The split matters because it keeps decompression
//! and filesystem parsing independently testable.
//!
//! # Why a hand-written ISO 9660 reader
//!
//! ISO 9660 is small, frozen since 1988, and the subset used by UMD and DVD
//! images is smaller still. The available crates are either unmaintained or
//! GPL-licensed, and neither trade is worth making for a few hundred lines of
//! well-specified structure parsing.
//!
//! ```no_run
//! use oag_disc::DiscImage;
//!
//! let mut disc = DiscImage::open("data/images/pulse-psp-usa.chd")?;
//! for entry in disc.entries()? {
//!     println!("{:>10}  {}", entry.size, entry.path);
//! }
//! # Ok::<(), oag_disc::Error>(())
//! ```

pub mod chd_source;
pub mod error;
pub mod image;
pub mod iso9660;
pub mod platform;
pub mod ps3_crypt;
pub mod raw_source;
pub mod sfo;
pub mod source;

pub use error::{Error, Result};
pub use image::{DiscImage, Ps3State};
pub use iso9660::Entry;
pub use platform::{Platform, TitleInfo};
pub use source::{SECTOR_SIZE, SectorSource};
