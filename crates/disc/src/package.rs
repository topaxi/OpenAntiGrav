//! A game package that is a file tree rather than a disc: what a `.vpk` is.
//!
//! A [`PackageSource`] lists its files and reads byte ranges out of them, and
//! [`crate::DiscImage`] presents one through the same `entries` and
//! `read_entry_range` calls a disc answers, so everything above this crate that
//! mounts `<image>:<path>` specs works on a package unchanged. For a package an
//! [`crate::Entry`]'s `lba` is the file's index in [`PackageSource::files`], not
//! a sector.

use crate::error::Result;

/// One file in a package.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageFile {
    /// `/` separated, no leading slash. A package made of several archives
    /// prefixes each file with its archive's name, so no two files collide.
    pub path: String,
    /// Uncompressed size in bytes.
    pub size: u64,
}

/// A readable file tree.
pub trait PackageSource: std::fmt::Debug + Send {
    /// Every file, in a stable order.
    fn files(&self) -> &[PackageFile];

    /// Reads up to `len` bytes of file `index` from `offset`. Fewer come back
    /// only at the end of the file.
    fn read(&mut self, index: usize, offset: u64, len: u64) -> Result<Vec<u8>>;
}
