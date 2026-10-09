//! Reads logical sectors out of a raw `.iso` file.

use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use crate::error::{Error, Result};
use crate::mount::{self, ImageFile};
use crate::source::{SECTOR_SIZE, SectorSource};

/// A flat ISO file exposed as a sector source.
#[derive(Debug)]
pub struct RawSource {
    file: BufReader<ImageFile>,
    path: PathBuf,
    sector_count: u32,
}

impl RawSource {
    /// Opens a raw ISO file.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let file = mount::open(&path).map_err(|e| Error::io(&path, e))?;
        let len = mount::len(&file).map_err(|e| Error::io(&path, e))?;

        // Truncated final sectors are ignored rather than rejected. A partially
        // downloaded image should still list what it does contain.
        let sector_count = u32::try_from(len / SECTOR_SIZE as u64).unwrap_or(u32::MAX);

        Ok(Self {
            file: BufReader::new(file),
            path,
            sector_count,
        })
    }

    /// The path this source was opened from.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl SectorSource for RawSource {
    fn sector_count(&self) -> u32 {
        self.sector_count
    }

    fn read_sector(&mut self, lba: u32, buf: &mut [u8]) -> Result<()> {
        // `assert`, not `debug_assert`: this was the quiet half of a contract
        // the CHD source already panicked on, so in release a short or long
        // buffer read across a sector boundary and returned wrong data with
        // `Ok`. See `SectorSource::read_sector`.
        assert_eq!(
            buf.len(),
            SECTOR_SIZE,
            "read_sector needs a buffer of exactly one sector"
        );
        if lba >= self.sector_count {
            return Err(Error::SectorOutOfRange {
                sector: lba,
                total: self.sector_count,
            });
        }

        let offset = u64::from(lba) * SECTOR_SIZE as u64;
        self.file
            .seek(SeekFrom::Start(offset))
            .map_err(|e| Error::io(&self.path, e))?;
        self.file
            .read_exact(buf)
            .map_err(|e| Error::io(&self.path, e))?;
        Ok(())
    }
}
