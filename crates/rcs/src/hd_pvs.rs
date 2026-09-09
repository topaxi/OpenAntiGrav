//! `track.pvs`: the potentially-visible set a Wipeout HD circuit ships beside
//! its `.rcsmodel`.
//!
//! # What it is, and why a renderer that ignores it draws three times too much
//!
//! **The circuit is partitioned along its own racing line, and each cell
//! carries one bit per chunk of the sibling `.rcsmodel`.** Averaged over the
//! sampled cells of Talon's Junction and Anulpha Pass, **33% of the chunks are
//! set** - so a renderer with no PVS submits roughly three times the geometry
//! the original does, and every one of those extra chunks is something the
//! artists decided you cannot see from there. Seen from inside a dense
//! multi-level circuit that reads as scenery from the far side of the map
//! standing in the middle of the track.
//!
//! # Layout
//!
//! Big-endian throughout, like every other PS3 asset here.
//!
//! ```text
//! +0x00  u32  cells
//! +0x04  u32  chunks          - the sibling .rcsmodel's chunk count, exactly
//! +0x08  u32  16              - constant across all 28 files on the disc
//! +0x0c  u32  ?               - differs per file; not read
//! +0x10       cells x 4 f32   - one record per cell: x, y, z, and a repeat of z
//! ...         cells x (chunks / 8 + 1) bytes - one bitmap per cell, LSB first
//! ...         optional trailing bytes, see `Pvs::trailing`
//! ```
//!
//! **`cells` and the bitmap width are what the loader at `0x003c6f20` reads;
//! the other three header words it reads and throws away.** Word 1 lands in
//! the object and is then overwritten with a chunk count the *caller* passes
//! from the model, and words 2 and 3 land in stack slots nothing reads again -
//! they are consumed to advance the stream and nothing else. So the engine
//! trusts the model for the bitmap width, not the file. See
//! `docs/ghidra/functions/ps3-hdfury-eu/visibility.md`.
//!
//! # How each field was settled
//!
//! **`chunks` is the chunk count.** It equals `rcsmodel::Model::meshes.len()`
//! for **all 28** `.pvs` files on the disc - 983 for Talon's Junction, 1,125
//! for Anulpha Pass, 1,902 for Modesto Heights, and so on for every other
//! circuit and every `_reversed` variant. Confidence 97.
//!
//! **The bitmap is `chunks / 8 + 1` bytes.** The loader computes exactly
//! `(chunks >> 3) + 1`, so a chunk count that is a multiple of eight gets a
//! **whole spare byte** rather than a tight fit - `ceil(chunks / 8)` agrees on
//! every other count and is wrong on those. Confidence 84 from the
//! instruction pair; independently, every cell's last byte carries only the
//! bits `chunks` leaves valid, on all 28 files - Talon's 983 bits end seven
//! into byte 123 and no block's last byte reaches `0x80`, Anulpha's 1,125 end
//! five into byte 141 and none reaches `0x20`. 621 bytes all below `0x80` by
//! chance is `2^-621`.
//!
//! **This is why some files looked like they had a trailing section and do
//! not.** A `ceil` reader under-counts the table by exactly `cells` bytes on
//! every file whose chunk count divides by eight, and four of the disc's
//! twenty-eight do.
//!
//! **Bit `k` is chunk `k` in `.rcsmodel` file order, LSB first.** The
//! executable settles the bit order outright: the routine that clears one
//! chunk from the frame mask (`0x003fa0f8`) is `mask[i >> 3] &= ~(1 << (i &
//! 7))`. Confidence 84. That the index is the chunk's own file position is
//! measured spatially: for each sampled cell, take the 50 chunks whose bounding spheres
//! are nearest the cell and the 50 that are furthest, and count how many are
//! set. At offset zero, **84% of the nearest are visible and 10% of the
//! furthest** on Talon's Junction (90% / 11% on Anulpha Pass). Shifting the
//! index by one drops the near figure to 66% and 75% and raises the far figure,
//! and every further shift is worse still - a sharp peak at zero rather than a
//! plateau, which is what distinguishes the mapping from mere spatial
//! clustering of the file order. MSB-first scores 2:1 where LSB-first scores
//! 8:1. Confidence 92.
//!
//! **A cell's first three floats are a position on the racing line.** They
//! march along the track in even steps - Talon's Junction's first twelve are
//! 12 units apart - and [`Pvs::nearest_cell`] is the lookup that the near/far
//! measurement above is built on, so the position is confirmed by the same
//! evidence that confirms the bitmap. The fourth float repeats the third on
//! every record read so far and is deliberately not interpreted.
//! Confidence 90.
//!
//! # What is not read
//!
//! **Header word 3, and the trailing bytes.** 14 of the 28 files are exactly
//! `0x10 + cells * 16 + cells * width` bytes long and 14 carry more - between
//! 477 and 78,900 further bytes. That region is dense and high-entropy, it is
//! not a continuation of the bitmap table (the first block past the declared
//! cell count fails the padding test), and its size divides the cell count
//! evenly in some files and not others, so it is variable-length and its shape
//! is unknown. [`Pvs::trailing`] reports how much of it there is rather than
//! guessing at it; the visibility lookup does not need it.

use oag_formats::ByteOrder;

/// Where the cell records begin.
const CELLS_AT: usize = 0x10;

/// Bytes per cell record: four big-endian `f32`.
const CELL_SIZE: usize = 16;

/// The value header word 2 carries on every file on the disc.
///
/// Read and checked rather than skipped, because it is the one field whose
/// meaning is unknown *and* constant - if a future title's file differs there,
/// the difference should surface as a refusal rather than as a silently
/// misparsed table.
const EXPECTED_STRIDE_WORD: u32 = CELLS_AT as u32;

/// Why a `.pvs` could not be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// Shorter than the header, or shorter than the table the header declares.
    TooShort { need: usize, got: usize },
    /// Header word 2 is not [`EXPECTED_STRIDE_WORD`].
    UnknownLayout { word: u32 },
    /// The header declares no cells or no chunks, so nothing can be looked up.
    Empty { cells: u32, chunks: u32 },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooShort { need, got } => {
                write!(f, "a .pvs of {got} bytes cannot hold its own {need}")
            }
            Self::UnknownLayout { word } => write!(
                f,
                "header word 2 is {word}, not {EXPECTED_STRIDE_WORD}; this is not the layout \
                 docs/formats/hd-pvs.md describes"
            ),
            Self::Empty { cells, chunks } => {
                write!(
                    f,
                    "a .pvs declaring {cells} cell(s) over {chunks} chunk(s) can answer nothing"
                )
            }
        }
    }
}

impl std::error::Error for Error {}

/// A circuit's authored visibility partition.
#[derive(Debug, Clone, PartialEq)]
pub struct Pvs {
    positions: Vec<[f32; 3]>,
    chunks: usize,
    width: usize,
    bits: Vec<u8>,
    trailing: usize,
}

impl Pvs {
    /// Reads a whole `track.pvs`.
    ///
    /// # Errors
    ///
    /// [`Error`] when the file is too short for the table its own header
    /// declares, when header word 2 is not the constant every shipped file
    /// carries, or when it declares an empty partition.
    pub fn parse(data: &[u8]) -> Result<Self, Error> {
        if data.len() < CELLS_AT {
            return Err(Error::TooShort {
                need: CELLS_AT,
                got: data.len(),
            });
        }
        let order = ByteOrder::Big;
        let word = |i: usize| order.u32(data, i * 4);
        let (cells, chunks) = (word(0) as usize, word(1) as usize);
        if word(2) != EXPECTED_STRIDE_WORD {
            return Err(Error::UnknownLayout { word: word(2) });
        }
        if cells == 0 || chunks == 0 {
            return Err(Error::Empty {
                cells: cells as u32,
                chunks: chunks as u32,
            });
        }
        let width = chunks / 8 + 1;
        let bits_at = CELLS_AT + cells * CELL_SIZE;
        let need = bits_at + cells * width;
        if data.len() < need {
            return Err(Error::TooShort {
                need,
                got: data.len(),
            });
        }
        let positions = (0..cells)
            .map(|cell| {
                let at = CELLS_AT + cell * CELL_SIZE;
                std::array::from_fn(|axis| order.f32(data, at + axis * 4))
            })
            .collect();
        Ok(Self {
            positions,
            chunks,
            width,
            bits: data[bits_at..need].to_vec(),
            trailing: data.len() - need,
        })
    }

    /// How many cells the partition has.
    #[must_use]
    pub fn cells(&self) -> usize {
        self.positions.len()
    }

    /// How many chunks each cell's bitmap covers - the sibling `.rcsmodel`'s
    /// own chunk count.
    #[must_use]
    pub fn chunks(&self) -> usize {
        self.chunks
    }

    /// Bytes after the bitmap table that this parser does not read.
    ///
    /// **The loader cannot reach them.** `0x003c6f20` has six read sites - four
    /// header words, the cell records, and the per-cell bitmaps - and no seek,
    /// and it destroys the stream immediately afterwards. So whatever this
    /// region is, the retail executable never looks at it. Zero on 18 of the
    /// disc's 28 files. See the module docs.
    #[must_use]
    pub fn trailing(&self) -> usize {
        self.trailing
    }

    /// Where a cell sits, in world space.
    #[must_use]
    pub fn position(&self, cell: usize) -> Option<[f32; 3]> {
        self.positions.get(cell).copied()
    }

    /// The cell whose position is nearest `point`.
    ///
    /// A linear scan: the largest partition on the disc is 763 cells of three
    /// floats, which is one 9 KiB pass per frame against the thousand-odd
    /// bounding-sphere tests it saves.
    #[must_use]
    pub fn nearest_cell(&self, point: [f32; 3]) -> Option<usize> {
        let mut best = (0usize, f32::INFINITY);
        for (cell, p) in self.positions.iter().enumerate() {
            let d =
                (p[0] - point[0]).powi(2) + (p[1] - point[1]).powi(2) + (p[2] - point[2]).powi(2);
            if d < best.1 {
                best = (cell, d);
            }
        }
        best.1.is_finite().then_some(best.0)
    }

    /// One cell's bitmap, LSB first over chunk index.
    #[must_use]
    pub fn cell_bits(&self, cell: usize) -> Option<&[u8]> {
        let at = cell.checked_mul(self.width)?;
        self.bits.get(at..at + self.width)
    }

    /// The width of one cell's bitmap in bytes, as the loader computes it.
    #[must_use]
    pub fn bitmap_bytes(&self) -> usize {
        self.width
    }

    /// Whether `chunk` may be drawn from `cell`.
    ///
    /// `true` for a chunk or cell out of range: the error has to point towards
    /// drawing too much, exactly as `oag_render::pvs::ALWAYS` does for the
    /// PSP's partition.
    #[must_use]
    pub fn visible(&self, cell: usize, chunk: usize) -> bool {
        let Some(bits) = self.cell_bits(cell) else {
            return true;
        };
        if chunk >= self.chunks {
            return true;
        }
        bits[chunk >> 3] >> (chunk & 7) & 1 == 1
    }

    /// How many chunks one cell can see.
    #[must_use]
    pub fn visible_count(&self, cell: usize) -> u32 {
        self.cell_bits(cell)
            .map_or(0, |bits| bits.iter().map(|b| b.count_ones()).sum())
    }
}

/// Whether `chunk`'s bit is set in one cell's bitmap.
///
/// The free function the renderer's inner loop calls, so a frame holds the
/// current cell's slice once rather than indexing the whole table per draw.
/// Out of range answers `true`, for the reason [`Pvs::visible`] gives.
#[must_use]
pub fn allows(bits: &[u8], chunk: usize) -> bool {
    match bits.get(chunk >> 3) {
        None => true,
        Some(byte) => byte >> (chunk & 7) & 1 == 1,
    }
}

#[cfg(test)]
mod tests;
