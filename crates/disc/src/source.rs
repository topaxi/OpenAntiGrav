//! Random access to the logical sectors of a disc image.

use crate::error::{Error, Result};

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
///
/// `Send` is a supertrait because an opened image gets moved onto a worker
/// thread: the game loads its movies off one while the window it is about to
/// fill is already open (`oag_game::boot::MediaWorker`). Both implementors are
/// a file handle and a decompression buffer, so this costs nothing and is
/// stated here rather than discovered as an error at the one call site that
/// needs it. It is deliberately **not** `Sync`: one thread owns a source at a
/// time, because reading one seeks it.
pub trait SectorSource: std::fmt::Debug + Send {
    /// Total number of logical sectors in the image.
    fn sector_count(&self) -> u32;

    /// Reads one sector into `buf`, which must be exactly [`SECTOR_SIZE`] long.
    ///
    /// # Panics
    ///
    /// **Both implementations panic on a `buf` of any other length**, and that
    /// is the contract rather than an accident. This trait is public, so the
    /// length is a caller error one implementation used to `debug_assert` (and
    /// so, in release, read across a sector boundary and hand back silently
    /// wrong data) while the other panicked - two behaviours for one misuse,
    /// and the quiet one is the worse. Finding F6 of the 2026-08-18 review.
    fn read_sector(&mut self, lba: u32, buf: &mut [u8]) -> Result<()>;

    /// Reads `count` consecutive sectors, returning them concatenated.
    ///
    /// The default implementation reads one sector at a time, which is fine
    /// because both implementations make repeated access to the same region
    /// cheap: the raw reader is buffered, and the CHD reader caches the last
    /// decompressed hunk, so a run of sectors within one hunk decompresses
    /// once.
    fn read_sectors(&mut self, lba: u32, count: u32) -> Result<Vec<u8>> {
        check_range(self.sector_count(), lba, count)?;
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
        // **Checked, not cast.** `len` reaches here from an ISO 9660 directory
        // record, and multi-extent joins sum: a hostile image can name 8 TiB,
        // whose sector count passes 2^32, and `as u32` truncated it to `0` - so
        // `read_sectors(lba, 0)` returned an empty `Vec` with `Ok`, which is
        // silently wrong data rather than an error. Finding F3 of the
        // 2026-08-18 review, and the same root cause as
        // `Image::read_entry_range`'s start LBA.
        let sectors = u32::try_from(len.div_ceil(SECTOR_SIZE as u64)).map_err(|_| {
            Error::SectorOutOfRange {
                sector: u32::MAX,
                total: self.sector_count(),
            }
        })?;
        let mut data = self.read_sectors(lba, sectors)?;
        data.truncate(len as usize);
        Ok(data)
    }
}

/// Refuses a range that runs past the image, before anything allocates for it.
/// `count` derives from a directory record's size field, so a hostile image can
/// ask for 4 GiB and get it committed before the first out-of-range read fails.
pub(crate) fn check_range(total: u32, lba: u32, count: u32) -> Result<()> {
    let end = lba
        .checked_add(count)
        .ok_or(Error::SectorOutOfRange { sector: lba, total })?;
    if end > total {
        return Err(Error::SectorOutOfRange {
            sector: end.saturating_sub(1),
            total,
        });
    }
    Ok(())
}
