//! Where an image's bytes come from: a file on native, a registered blob on
//! the web.
//!
//! Every reader that opens the image itself (the CHD and raw-ISO sources, the
//! container sniff) opens it through [`open`], so the rest of this crate keeps
//! one path-based shape on every target. On native [`ImageFile`] *is*
//! [`std::fs::File`] and [`open`] is [`std::fs::File::open`]: nothing changes
//! there.
//!
//! `wasm32-unknown-unknown` has no filesystem. The page reads the image the
//! player picked and the web entry point registers it under a path with
//! [`register`]; [`open`] then hands out a [`Read`] + [`Seek`] cursor over it.
//! A [`Blob`] is random-access and synchronous: the web entry point registers
//! either the whole image or a reader that fetches slices of the picked file
//! on demand, and the readers above cannot tell which. See docs/tools/web.md.

#[cfg(not(target_arch = "wasm32"))]
use std::io;
#[cfg(not(target_arch = "wasm32"))]
use std::path::Path;

/// The handle a reader holds on the image.
#[cfg(not(target_arch = "wasm32"))]
pub type ImageFile = std::fs::File;

/// Opens the image at `path`.
///
/// # Errors
/// As [`std::fs::File::open`].
#[cfg(not(target_arch = "wasm32"))]
pub fn open(path: &Path) -> io::Result<ImageFile> {
    std::fs::File::open(path)
}

/// The image's length in bytes.
///
/// # Errors
/// As [`std::fs::File::metadata`].
#[cfg(not(target_arch = "wasm32"))]
pub fn len(file: &ImageFile) -> io::Result<u64> {
    file.metadata().map(|metadata| metadata.len())
}

#[cfg(target_arch = "wasm32")]
pub use web::{Blob, ImageFile, len, open, register};

#[cfg(target_arch = "wasm32")]
mod web {
    use std::io::{self, Read, Seek, SeekFrom};
    use std::path::{Path, PathBuf};
    use std::sync::{Arc, Mutex};

    /// Random-access, synchronous bytes: what a registered image is.
    pub trait Blob: Send + Sync {
        /// Total length in bytes.
        fn size(&self) -> u64;
        /// Copies bytes from `offset` into `buf`, returning how many; fewer
        /// than `buf.len()` only at the end.
        ///
        /// # Errors
        /// The backing store failed.
        fn read_at(&self, offset: u64, buf: &mut [u8]) -> io::Result<usize>;
    }

    impl Blob for Vec<u8> {
        fn size(&self) -> u64 {
            self.len() as u64
        }

        fn read_at(&self, offset: u64, buf: &mut [u8]) -> io::Result<usize> {
            let start = usize::try_from(offset)
                .unwrap_or(usize::MAX)
                .min(self.len());
            let n = buf.len().min(self.len() - start);
            buf[..n].copy_from_slice(&self[start..start + n]);
            Ok(n)
        }
    }

    type Mounts = Vec<(PathBuf, Arc<dyn Blob>)>;
    static MOUNTS: Mutex<Mounts> = Mutex::new(Vec::new());

    /// Makes `blob` openable at `path`, replacing whatever was there.
    pub fn register(path: impl Into<PathBuf>, blob: Arc<dyn Blob>) {
        let path = path.into();
        let mut mounts = MOUNTS
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        mounts.retain(|(at, _)| *at != path);
        mounts.push((path, blob));
    }

    /// A cursor over a registered blob.
    pub struct ImageFile {
        blob: Arc<dyn Blob>,
        pos: u64,
    }

    impl std::fmt::Debug for ImageFile {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("ImageFile")
                .field("size", &self.blob.size())
                .field("pos", &self.pos)
                .finish()
        }
    }

    /// Opens the blob registered at `path`.
    ///
    /// # Errors
    /// [`io::ErrorKind::NotFound`] when nothing is registered there.
    pub fn open(path: &Path) -> io::Result<ImageFile> {
        let mounts = MOUNTS
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        mounts
            .iter()
            .find(|(at, _)| at == path)
            .map(|(_, blob)| ImageFile {
                blob: Arc::clone(blob),
                pos: 0,
            })
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no image registered here"))
    }

    /// The blob's length in bytes.
    ///
    /// # Errors
    /// Never; the signature matches the native twin.
    #[allow(clippy::unnecessary_wraps)]
    pub fn len(file: &ImageFile) -> io::Result<u64> {
        Ok(file.blob.size())
    }

    impl Read for ImageFile {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            let n = self.blob.read_at(self.pos, buf)?;
            self.pos += n as u64;
            Ok(n)
        }
    }

    impl Seek for ImageFile {
        fn seek(&mut self, from: SeekFrom) -> io::Result<u64> {
            let target = match from {
                SeekFrom::Start(at) => Some(at),
                SeekFrom::End(delta) => self.blob.size().checked_add_signed(delta),
                SeekFrom::Current(delta) => self.pos.checked_add_signed(delta),
            };
            self.pos = target.ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidInput, "seek before the start")
            })?;
            Ok(self.pos)
        }
    }
}
