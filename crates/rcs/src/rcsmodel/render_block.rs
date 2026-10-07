//! The per-chunk render-block record a chunk header's `+0x08` word points
//! at, and the relocation table that word is listed in.
//!
//! Two words `rcsmodel.md` had placed but not explained, read out of the
//! executable on 2026-09-15 while chasing who writes the Scene/Track bit HD's
//! Zone mode branches on. Nobody writes it: it is authored in the file. See
//! `docs/formats/rcsmodel.md`, "The header's `+0x04` is a relocation table",
//! and the thirtieth pass of
//! `docs/ghidra/functions/ps3-hdfury-eu/zone-effectsettings-loader.md`.
//!
//! # The relocation table
//!
//! The header's `+0x04` is the offset of `{u32 count; u32 offsets[count]}`.
//! The original's generic RCS loader reads the file whole and then, for every
//! offset in that table, turns the word there into a pointer by adding the
//! buffer base - a zero word stays zero. It is why the engine never parses
//! the header: every offset in the file is a pointer by the time anything
//! reads it. **On disc the words are still file offsets**, so a reader of the
//! file has nothing to relocate; the table is a check that a word *is* an
//! offset, and [`super::Model::relocations`] reads it for that.
//!
//! # The render block
//!
//! Every chunk header's `+0x08` word is in that table on all 41,861 chunks of
//! all 643 files, and names a 0x40-byte record, one per chunk in chunk order,
//! contiguous - Talon's Junction's run at `0x400c0`-`0x4f680`, immediately
//! before the string pool:
//!
//! ```text
//! +0x00  u32   0 on disc; the engine stores the chunk's runtime transform
//!              object here
//! +0x04  u16   0 on disc, and nothing located reads it on its own
//! +0x06  u16   authored flags - see `RENDER_TRACK`
//! +0x20  f32x4, f32x4   two vectors, unread
//! ```
//!
//! Confidence 85 on the record and the relocation, 80 on bit 0's meaning.

use oag_formats::ByteOrder;

use super::{Error, Mesh, Model, Result};

/// The chunk header word naming the chunk's render-block record: a file
/// offset, or zero for a chunk with none.
pub(super) const RENDER_BLOCK: usize = 0x08;

/// Bytes of one render-block record.
pub const RENDER_BLOCK_LEN: usize = 0x40;

/// The authored flags halfword, relative to the record. Big-endian, like
/// everything else in the file: bit 0 is the low bit of byte `+0x07`.
const RENDER_FLAGS: usize = 0x06;

/// Bit 0 of [`Mesh::render_flags`]: **this chunk is track surface.**
///
/// Two independent consumers in the executable say so. In Zone mode
/// `FUN_003ff860` publishes the `Track` colour set and `zoneModeTrack*.gtf`
/// for a chunk with it set and the `Scene` set (`zoneMode*.gtf`) otherwise;
/// `Scene_BuildStaticChunkMask` ORs the same chunks into the static mask the
/// shadowed-track redraw passes draw. On disc it is set on 4,365 of 41,861
/// chunks, in exactly the 37 track-shaped models (`track`,
/// `track_reversed`, `pvs_blocker`, mode pads); Talon's Junction has 124 of
/// 983. Confidence 80 on the meaning, from
/// `docs/ghidra/functions/ps3-hdfury-eu/zone-effectsettings-loader.md`'s
/// thirtieth pass.
///
/// The other bits the same pass read - the five-bucket draw routing in bits
/// 2-5, the PVS scrub in bits 9-10 - are carried in the halfword and are not
/// interpreted here, except bit 4 ([`RENDER_BEHIND_GLASS`]).
pub const RENDER_TRACK: u16 = 1 << 0;

/// Bit 4 of [`Mesh::render_flags`]: **this chunk is drawn behind the glass,
/// never in the main view.**
///
/// The original routes it to its sort bucket B3, and that bucket is drawn into
/// a 640x360 target of its own, with the sky, under a projection 4/3 wider in
/// tangent than the main view's; the tunnel glass reads that target. Measured
/// on Vineta K's RPCS3 captures, each draw tied to its chunk by vertex offset:
/// 93 of 93 draws into that target are chunks with this bit, and none of 357
/// main-view draws is, over three frames. Confidence 88, from
/// `docs/ghidra/functions/ps3-hdfury-eu/visibility.md`, 2026-10-07. On disc it
/// is set on 420 chunks, all in `01_vineta_k`.
pub const RENDER_BEHIND_GLASS: u16 = 1 << 4;

/// Reads the flags halfword of the record a chunk header at `at` names.
///
/// Zero - no flags, the Scene set - for a chunk whose word is zero, which is
/// the "no record" value the original's relocation leaves alone.
///
/// # Errors
///
/// [`Error::OutOfBounds`] for a non-zero word whose record leaves the file,
/// which is what a wrongly-read word looks like.
pub(super) fn flags(data: &[u8], at: usize) -> Result<u16> {
    let record = ByteOrder::Big.u32(data, at + RENDER_BLOCK) as usize;
    if record == 0 {
        return Ok(0);
    }
    let end = record + RENDER_BLOCK_LEN;
    if end > data.len() {
        return Err(Error::OutOfBounds {
            what: "a chunk's render block",
            end,
            len: data.len(),
        });
    }
    Ok(ByteOrder::Big.u16(data, record + RENDER_FLAGS))
}

impl Mesh {
    /// Whether this chunk is track surface - bit 0 of [`Self::render_flags`],
    /// see [`RENDER_TRACK`].
    ///
    /// The bit HD's Zone mode selects a chunk's colour set and stage texture
    /// on: the `Track` pair when set, the `Scene` pair when clear.
    #[must_use]
    pub fn is_track(&self) -> bool {
        self.render_flags & RENDER_TRACK != 0
    }

    /// Whether this chunk belongs to the behind-the-glass target rather than
    /// the main view - bit 4 of [`Self::render_flags`], see
    /// [`RENDER_BEHIND_GLASS`].
    #[must_use]
    pub fn is_behind_glass(&self) -> bool {
        self.render_flags & RENDER_BEHIND_GLASS != 0
    }
}

impl Model {
    /// The file's relocation table: every offset of a word the original's
    /// loader turns into a pointer.
    ///
    /// Read from `data` on demand rather than kept on the model - it is
    /// 26,450 entries on Talon's Junction and nothing at runtime needs it.
    /// What it is for is checking that a word this module reads as an offset
    /// is one the file itself calls an offset; the disc-wide test does that
    /// for the chunk header's `+0x08`.
    ///
    /// # Errors
    ///
    /// [`Error::TooShort`] for a blob without a header and
    /// [`Error::OutOfBounds`] for a table that leaves the file.
    pub fn relocations(data: &[u8]) -> Result<Vec<u32>> {
        if data.len() < super::HEADER_LEN {
            return Err(Error::TooShort { got: data.len() });
        }
        let at = ByteOrder::Big.u32(data, 0x04) as usize;
        if at + 4 > data.len() {
            return Err(Error::OutOfBounds {
                what: "the relocation table's count",
                end: at + 4,
                len: data.len(),
            });
        }
        let count = ByteOrder::Big.u32(data, at) as usize;
        super::table(data, at + 4, count, "the relocation table")
    }
}
