//! Reads logical sectors out of a CHD file.
//!
//! CHD stores data in fixed-size *hunks*, each holding several *units*. For
//! DVD and UMD images a unit is one 2048-byte sector. Decompressing a hunk is
//! expensive, so the most recently used one is cached: ISO 9660 walks are
//! strongly sequential, and without the cache a hunk gets re-decompressed once
//! per sector it contains.

use std::io::BufReader;
use std::path::{Path, PathBuf};

use chd::Chd;
use chd::metadata::Metadata;

use crate::error::{Error, Result};
use crate::source::{SECTOR_SIZE, SectorSource, check_range};

/// Container-level facts about a CHD.
///
/// Readable even when the image holds something we cannot mount, which is the
/// point: when [`ChdSource::open`] rejects a file, this says why in terms of
/// what the container actually claims to be.
#[derive(Debug, Clone)]
pub struct ChdInfo {
    /// CHD format version.
    pub version: u32,
    /// Number of hunks.
    pub hunk_count: u32,
    /// Bytes per hunk.
    pub hunk_bytes: u32,
    /// Bytes per unit. 2048 for DVD/UMD, 2448 for CD with subcode.
    pub unit_bytes: u32,
    /// Number of units.
    pub unit_count: u64,
    /// Total uncompressed size.
    pub logical_bytes: u64,
    /// Whether the hunks are compressed.
    pub compressed: bool,
    /// Metadata entries, as `(four-character tag, decoded text)`.
    pub metadata: Vec<(String, String)>,
}

/// Reads the header and metadata of a CHD without mounting a filesystem.
pub fn describe(path: impl AsRef<Path>) -> Result<ChdInfo> {
    let path = path.as_ref();
    let file = crate::mount::open(path).map_err(|e| Error::io(path, e))?;
    let mut chd = Chd::open(BufReader::new(file), None)?;

    let header = chd.header();
    let info_from_header = (
        header.version() as u32,
        header.hunk_count(),
        header.hunk_size(),
        header.unit_bytes(),
        header.unit_count(),
        header.logical_bytes(),
        header.is_compressed(),
    );

    // Metadata failing to parse must not hide the header, which is the part
    // that usually explains the problem.
    let metadata: Vec<Metadata> = chd.metadata_refs().try_into().unwrap_or_default();

    Ok(ChdInfo {
        version: info_from_header.0,
        hunk_count: info_from_header.1,
        hunk_bytes: info_from_header.2,
        unit_bytes: info_from_header.3,
        unit_count: info_from_header.4,
        logical_bytes: info_from_header.5,
        compressed: info_from_header.6,
        metadata: metadata
            .into_iter()
            .map(|m| (fourcc(m.metatag), decode_metadata(&m.value)))
            .collect(),
    })
}

/// Renders a metadata tag, which is a big-endian four-character code.
fn fourcc(tag: u32) -> String {
    tag.to_be_bytes()
        .iter()
        .map(|&b| {
            if b.is_ascii_graphic() || b == b' ' {
                b as char
            } else {
                '.'
            }
        })
        .collect()
}

/// CHD metadata values are mostly NUL-terminated ASCII key/value text.
fn decode_metadata(value: &[u8]) -> String {
    String::from_utf8_lossy(value)
        .trim_end_matches('\0')
        .trim()
        .to_string()
}

/// Where the 2048 bytes of user data sit inside a stored unit.
///
/// `chdman createdvd` stores bare 2048-byte user data and the offset is zero.
/// `chdman createcd` uses a fixed 2448-byte frame regardless of the source
/// track: the sector data is written at offset 0, zero-padded out to 2352
/// bytes, and followed by 96 bytes of subchannel space.
///
/// So the offset depends on the *track type*, not on the unit size. A cooked
/// track (`MODE1`, source sectors already 2048 bytes) puts user data at offset
/// 0. A raw track (`MODE1_RAW`, source sectors 2352 bytes) stores the whole CD
/// frame, so the user data sits after the 12-byte sync and 4-byte header.
///
/// This matters in practice: DVD images are routinely converted with
/// `createcd`, which is how a PS2 DVD ends up in a container that calls itself
/// a CD. The disc is fine, only the wrapper is unusual. Guessing offset 16
/// because the container says "CD" reads 16 bytes into every sector and
/// produces convincing garbage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct UnitLayout {
    /// Bytes per stored unit.
    unit_bytes: u32,
    /// Offset of the user data within a unit.
    user_offset: usize,
}

impl UnitLayout {
    /// Bare 2048-byte user data, as written by `chdman createdvd`.
    const COOKED: Self = Self {
        unit_bytes: 2048,
        user_offset: 0,
    };

    /// Works out the layout from the CHD header and track metadata.
    fn detect(unit_bytes: u32, track: Option<&TrackInfo>) -> Option<Self> {
        if unit_bytes as usize == SECTOR_SIZE {
            return Some(Self::COOKED);
        }

        // CD-style frames: 2352 bytes of sector space, plus 96 for subchannel.
        if unit_bytes != 2352 && unit_bytes != 2448 {
            return None;
        }

        let user_offset = match track?.mode.as_str() {
            // Cooked: the stored sector is the 2048-byte user area itself.
            "MODE1" | "MODE2_FORM1" => 0,
            // Raw MODE1: 12-byte sync, 4-byte header, then user data.
            "MODE1_RAW" => 16,
            // Raw MODE2: as above plus an 8-byte subheader.
            "MODE2_RAW" => 24,
            // AUDIO and MODE2_FORM2 have no 2048-byte user area at all.
            // MODE2 and MODE2_FORM_MIX do, but the layout is unverified here
            // and no in-scope title uses them, so they are refused rather than
            // guessed at.
            _ => return None,
        };

        Some(Self {
            unit_bytes,
            user_offset,
        })
    }

    /// How many sectors a hunk holds, or `None` if the geometry is unusable.
    ///
    /// `read_sector` divides the logical block address by this, and slices the
    /// decompressed hunk at `unit * unit_bytes + user_offset`. Both sizes come
    /// from the CHD header, so both are attacker-controlled: a hunk smaller than
    /// one unit makes the division zero, and a hunk that is not a whole number of
    /// units puts the last unit's user area past the end of the buffer. Neither
    /// is producible by `chdman`, and both are one edited header away.
    fn sectors_per_hunk(&self, hunk_bytes: u32) -> Option<u32> {
        let sectors = hunk_bytes.checked_div(self.unit_bytes)?;
        let last_unit_end = sectors
            .checked_sub(1)?
            .checked_mul(self.unit_bytes)?
            .checked_add(u32::try_from(self.user_offset + SECTOR_SIZE).ok()?)?;
        (sectors > 0 && last_unit_end <= hunk_bytes).then_some(sectors)
    }
}

/// One track from a CHD's `CHTR`/`CHT2` metadata.
#[derive(Debug, Clone)]
struct TrackInfo {
    mode: String,
    /// Sectors in this track, or `None` when the line does not say.
    ///
    /// **`Option`, not a zero**, since finding F5 of the 2026-08-18 review: a
    /// track line missing or mangling `FRAMES:` used to default to `0`, which
    /// became a `sector_count` of `0` and turned every read into
    /// `SectorOutOfRange { total: 0 }` - an image bricked with a confusing
    /// error rather than falling back to the header's own unit count, which is
    /// exactly what the no-metadata path already does.
    frames: Option<u32>,
    pregap: u32,
}

/// A CHD file exposed as a sector source.
pub struct ChdSource {
    chd: Chd<BufReader<crate::mount::ImageFile>>,
    path: PathBuf,
    layout: UnitLayout,
    sectors_per_hunk: u32,
    sector_count: u32,
    hunk_buf: Vec<u8>,
    compressed_buf: Vec<u8>,
    cached_hunk: Option<u32>,
}

impl std::fmt::Debug for ChdSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChdSource")
            .field("path", &self.path)
            .field("layout", &self.layout)
            .field("sector_count", &self.sector_count)
            .field("sectors_per_hunk", &self.sectors_per_hunk)
            .finish_non_exhaustive()
    }
}

impl ChdSource {
    /// Opens a CHD file.
    ///
    /// Handles both `createdvd` images (bare 2048-byte sectors) and single
    /// track `createcd` images holding MODE1 or MODE2/FORM1 frames.
    ///
    /// # Errors
    ///
    /// Returns [`Error::CdImageUnsupported`] for layouts with no 2048-byte user
    /// area to read, and [`Error::MultiTrackUnsupported`] for images with more
    /// than one track. Neither occurs for any in-scope Wipeout title.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let file = crate::mount::open(&path).map_err(|e| Error::io(&path, e))?;
        let mut chd = Chd::open(BufReader::new(file), None)?;

        let tracks = read_tracks(&mut chd);

        // A multi-track disc interleaves pregaps between tracks, so a plain
        // "LBA times unit size" mapping would silently read the wrong data.
        // Rather than get it subtly wrong, refuse.
        if tracks.len() > 1 {
            return Err(Error::MultiTrackUnsupported {
                path,
                tracks: tracks.len(),
            });
        }
        if let Some(track) = tracks.first()
            && track.pregap != 0
        {
            return Err(Error::MultiTrackUnsupported { path, tracks: 1 });
        }

        let header = chd.header();
        let unit_bytes = header.unit_bytes();

        let layout = UnitLayout::detect(unit_bytes, tracks.first()).ok_or_else(|| {
            Error::CdImageUnsupported {
                path: path.clone(),
                unit_bytes,
                mode: tracks.first().map(|t| t.mode.clone()),
            }
        })?;

        let hunk_bytes = header.hunk_size();
        let Some(sectors_per_hunk) = layout.sectors_per_hunk(hunk_bytes) else {
            return Err(Error::HunkGeometryUnsupported {
                path,
                hunk_bytes,
                unit_bytes: layout.unit_bytes,
            });
        };
        // `and_then`, so a track that declares no frame count takes the same
        // fallback as an image with no track metadata at all - see
        // [`TrackInfo::frames`].
        let sector_count = tracks
            .first()
            .and_then(|t| t.frames)
            .unwrap_or_else(|| u32::try_from(header.unit_count()).unwrap_or(u32::MAX));

        let hunk_buf = chd.get_hunksized_buffer();

        Ok(Self {
            chd,
            path,
            layout,
            sectors_per_hunk,
            sector_count,
            hunk_buf,
            compressed_buf: Vec::new(),
            cached_hunk: None,
        })
    }

    /// The path this source was opened from.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Decompresses `hunk` into the buffer unless it is already there.
    fn ensure_hunk(&mut self, hunk: u32) -> Result<()> {
        if self.cached_hunk == Some(hunk) {
            return Ok(());
        }
        // Clear first: a failed read must not leave the cache claiming to hold
        // a hunk whose buffer contents are actually from the previous one.
        self.cached_hunk = None;
        self.chd
            .hunk(hunk)?
            .read_hunk_in(&mut self.compressed_buf, &mut self.hunk_buf)?;
        self.cached_hunk = Some(hunk);
        Ok(())
    }
}

impl SectorSource for ChdSource {
    fn sector_count(&self) -> u32 {
        self.sector_count
    }

    fn read_sector(&mut self, lba: u32, buf: &mut [u8]) -> Result<()> {
        // `assert`, so misuse names itself here rather than surfacing as a
        // `copy_from_slice` length panic further down - and so both
        // implementations answer the same way in release. See
        // `SectorSource::read_sector`.
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

        let hunk = lba / self.sectors_per_hunk;
        let unit = (lba % self.sectors_per_hunk) as usize * self.layout.unit_bytes as usize;
        let start = unit + self.layout.user_offset;

        self.ensure_hunk(hunk)?;
        buf.copy_from_slice(&self.hunk_buf[start..start + SECTOR_SIZE]);
        Ok(())
    }

    /// As the trait's own, and **split across threads when the range spans
    /// [`PARALLEL_BYTES`] per worker or more**: every hunk decompresses on its own,
    /// so each worker opens the file again and reads a run of whole hunks into
    /// its own slice of the result. The bytes are the serial read's. A Pulse
    /// PS2 race track is one 40 MiB read of LZMA hunks, and it was over half
    /// that race's load (`docs/architecture/load-time.md`).
    fn read_sectors(&mut self, lba: u32, count: u32) -> Result<Vec<u8>> {
        check_range(self.sector_count, lba, count)?;
        let mut out = vec![0u8; count as usize * SECTOR_SIZE];
        let per_hunk = self.sectors_per_hunk;
        let hunks = (lba + count).div_ceil(per_hunk) - lba / per_hunk;
        let workers = std::thread::available_parallelism()
            .map_or(1, std::num::NonZero::get)
            .min(MAX_WORKERS)
            .min(count as usize * SECTOR_SIZE / PARALLEL_BYTES);
        if workers < 2 {
            self.read_into(lba, &mut out)?;
            return Ok(out);
        }
        // Whole hunks per worker, so no two decompress the same one.
        let run = hunks.div_ceil(workers as u32) * per_hunk;
        let first_hunk_start = lba / per_hunk * per_hunk;
        let path = &self.path;
        std::thread::scope(|scope| {
            let mut rest = out.as_mut_slice();
            let mut at = lba;
            let mut handles = Vec::with_capacity(workers);
            while at < lba + count {
                let end =
                    (first_hunk_start + (at - first_hunk_start) / run * run + run).min(lba + count);
                let (mine, tail) = rest.split_at_mut((end - at) as usize * SECTOR_SIZE);
                rest = tail;
                let start = at;
                handles.push(scope.spawn(move || ChdSource::open(path)?.read_into(start, mine)));
                at = end;
            }
            handles
                .into_iter()
                .try_for_each(|handle| handle.join().expect("a CHD read worker panicked"))
        })?;
        Ok(out)
    }
}

/// The fewest bytes a read must give each worker before it is split at all:
/// every worker opens the file again and decodes its hunk map, which a
/// split at 64 hunks per worker paid for on a PSP boot's medium reads (0.301
/// to 0.320 s, five interleaved runs). Chosen, not measured beyond that.
const PARALLEL_BYTES: usize = 4 << 20;

/// The most threads one read is split across.
const MAX_WORKERS: usize = 8;

impl ChdSource {
    /// Reads whole sectors from `lba` on into `out`, one at a time.
    fn read_into(&mut self, lba: u32, out: &mut [u8]) -> Result<()> {
        for (i, sector) in (lba..).zip(out.as_chunks_mut::<SECTOR_SIZE>().0) {
            self.read_sector(i, sector)?;
        }
        Ok(())
    }
}

/// Reads the track table from a CHD's `CHTR`/`CHT2` metadata.
///
/// Returns an empty list for images with no track metadata, which is the
/// normal case for `createdvd` output.
fn read_tracks(chd: &mut Chd<BufReader<crate::mount::ImageFile>>) -> Vec<TrackInfo> {
    let entries: Vec<Metadata> = match chd.metadata_refs().try_into() {
        Ok(entries) => entries,
        Err(_) => return Vec::new(),
    };

    entries
        .iter()
        .filter_map(|m| parse_track(&String::from_utf8_lossy(&m.value)))
        .collect()
}

/// Parses a line like
/// `TRACK:1 TYPE:MODE1 SUBTYPE:NONE FRAMES:1900848 PREGAP:0 ...`.
fn parse_track(text: &str) -> Option<TrackInfo> {
    let field = |key: &str| {
        text.split_whitespace()
            .find_map(|kv| kv.strip_prefix(key)?.into())
            .map(str::to_string)
    };

    let mode = field("TYPE:")?;
    Some(TrackInfo {
        frames: field("FRAMES:").and_then(|v| v.parse().ok()),
        pregap: field("PREGAP:").and_then(|v| v.parse().ok()).unwrap_or(0),
        mode,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_cht2_track_line() {
        let t = parse_track(
            "TRACK:1 TYPE:MODE1 SUBTYPE:NONE FRAMES:1900848 PREGAP:0 PGTYPE:MODE1 \
             PGSUB:NONE POSTGAP:0",
        )
        .unwrap();
        assert_eq!(t.mode, "MODE1");
        assert_eq!(t.frames, Some(1_900_848));
        assert_eq!(t.pregap, 0);
    }

    #[test]
    fn ignores_metadata_that_is_not_a_track() {
        assert!(parse_track("CYLS:16383,HEADS:16,SECS:63,BPS:512").is_none());
    }

    #[test]
    fn cooked_dvd_units_need_no_offset() {
        let l = UnitLayout::detect(2048, None).unwrap();
        assert_eq!(l, UnitLayout::COOKED);
    }

    /// A hunk that cannot hold whole sectors has to be refused at open, because
    /// every later read divides by the count and slices by it.
    #[test]
    fn hunk_geometry_that_cannot_divide_into_sectors_is_refused() {
        let cooked = UnitLayout::COOKED;
        // What chdman actually writes: 8 sectors of 2048.
        assert_eq!(cooked.sectors_per_hunk(16_384), Some(8));
        assert_eq!(cooked.sectors_per_hunk(2_048), Some(1));

        // A hunk smaller than one unit divides to zero, which was a panic in
        // `read_sector` rather than an error at open.
        assert_eq!(cooked.sectors_per_hunk(0), None);
        assert_eq!(cooked.sectors_per_hunk(1), None);
        assert_eq!(cooked.sectors_per_hunk(2_047), None);

        // Trailing bytes are fine: the extra is simply never addressed.
        assert_eq!(cooked.sectors_per_hunk(2_049), Some(1));

        // With a user offset, the last unit's user area must still fit. A raw
        // MODE1 frame is 2352 bytes with 2048 of user data 16 bytes in, so a
        // hunk holding one frame is fine, but one truncated mid-frame is not.
        let raw = UnitLayout {
            unit_bytes: 2352,
            user_offset: 16,
        };
        assert_eq!(raw.sectors_per_hunk(2_352), Some(1));
        assert_eq!(raw.sectors_per_hunk(2_352 * 4), Some(4));
        assert_eq!(raw.sectors_per_hunk(2_064), None, "user area would be cut");
        assert_eq!(raw.sectors_per_hunk(u32::MAX), Some(u32::MAX / 2_352));
    }

    fn track(mode: &str) -> TrackInfo {
        TrackInfo {
            mode: mode.into(),
            frames: Some(100),
            pregap: 0,
        }
    }

    #[test]
    fn cooked_tracks_in_cd_frames_start_at_offset_zero() {
        // This is the case that matters for the PS2 Pulse image: a DVD
        // converted with `chdman createcd`, so the container reports 2448-byte
        // CD units while the sectors inside are ordinary 2048-byte user data.
        for mode in ["MODE1", "MODE2_FORM1"] {
            for unit_bytes in [2352, 2448] {
                let l = UnitLayout::detect(unit_bytes, Some(&track(mode))).unwrap();
                assert_eq!(l.user_offset, 0, "{mode} at {unit_bytes}");
                assert_eq!(l.unit_bytes, unit_bytes);
            }
        }
    }

    #[test]
    fn raw_tracks_skip_sync_and_header() {
        assert_eq!(
            UnitLayout::detect(2448, Some(&track("MODE1_RAW")))
                .unwrap()
                .user_offset,
            16
        );
        assert_eq!(
            UnitLayout::detect(2352, Some(&track("MODE2_RAW")))
                .unwrap()
                .user_offset,
            24
        );
    }

    #[test]
    fn unverified_track_types_are_refused_rather_than_guessed() {
        for mode in ["MODE2", "MODE2_FORM_MIX", "MODE2_FORM2"] {
            assert!(
                UnitLayout::detect(2448, Some(&track(mode))).is_none(),
                "{mode} should be refused until its layout is verified"
            );
        }
    }

    /// Finding F5's own guard: a line whose `FRAMES:` is missing or unparseable
    /// says "unknown", not "zero". A zero here became `sector_count == 0`, so
    /// every read of an otherwise-fine image failed
    /// `SectorOutOfRange { total: 0 }`.
    #[test]
    fn a_track_line_with_no_frame_count_says_so_rather_than_zero() {
        let missing = parse_track("TRACK:1 TYPE:MODE1 SUBTYPE:NONE PREGAP:0").unwrap();
        assert_eq!(missing.frames, None);
        assert_eq!(missing.mode, "MODE1", "the rest of the line still reads");

        let mangled = parse_track("TRACK:1 TYPE:MODE1 FRAMES:lots PREGAP:0").unwrap();
        assert_eq!(mangled.frames, None);
    }

    #[test]
    fn audio_tracks_have_no_user_data_area() {
        assert!(UnitLayout::detect(2352, Some(&track("AUDIO"))).is_none());
    }

    #[test]
    fn raw_frames_without_track_metadata_are_rejected() {
        // Without a mode there is no way to know where the user data starts,
        // and guessing would produce plausible-looking garbage.
        assert!(UnitLayout::detect(2352, None).is_none());
    }

    #[test]
    fn an_unfamiliar_unit_size_is_rejected() {
        assert!(UnitLayout::detect(512, None).is_none());
    }
}
