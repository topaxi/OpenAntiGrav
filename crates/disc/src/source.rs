//! Random access to the logical sectors of a disc image.

use crate::error::Result;

/// Logical sector size for UMD and DVD images.
///
/// Both use 2048-byte user-data sectors. CD images do not, which is why
/// [`crate::Error::CdImageUnsupported`] exists.
pub const SECTOR_SIZE: usize = 2048;

/// A source of 2048-byte logical sectors.
///
/// Implemented by both the CHD reader and the raw-ISO reader so that
/// [`crate::iso9660`] never has to know which container it is walking.
///
/// `Debug` is a supertrait so that types holding a boxed source can derive it.
/// A blanket `Debug` impl on `Box<dyn SectorSource>` is not possible: it would
/// overlap the standard library's impl for `Box<T: Debug>`.
pub trait SectorSource: std::fmt::Debug {
    /// Total number of logical sectors in the image.
    fn sector_count(&self) -> u32;

    /// Reads one sector into `buf`, which must be exactly [`SECTOR_SIZE`] long.
    fn read_sector(&mut self, lba: u32, buf: &mut [u8]) -> Result<()>;

    /// Reads `count` consecutive sectors, returning them concatenated.
    ///
    /// The default implementation reads one sector at a time, which is fine
    /// because both implementations make repeated access to the same region
    /// cheap: the raw reader is buffered, and the CHD reader caches the last
    /// decompressed hunk, so a run of sectors within one hunk decompresses
    /// once.
    fn read_sectors(&mut self, lba: u32, count: u32) -> Result<Vec<u8>> {
        let mut out = vec![0u8; count as usize * SECTOR_SIZE];
        for i in 0..count {
            let start = i as usize * SECTOR_SIZE;
            self.read_sector(lba + i, &mut out[start..start + SECTOR_SIZE])?;
        }
        Ok(out)
    }

    /// Reads `len` bytes starting at the beginning of sector `lba`.
    ///
    /// ISO 9660 files always start on a sector boundary but rarely end on one,
    /// so this trims the tail of the final sector.
    fn read_range(&mut self, lba: u32, len: u64) -> Result<Vec<u8>> {
        if len == 0 {
            return Ok(Vec::new());
        }
        let sectors = len.div_ceil(SECTOR_SIZE as u64) as u32;
        let mut data = self.read_sectors(lba, sectors)?;
        data.truncate(len as usize);
        Ok(data)
    }
}
