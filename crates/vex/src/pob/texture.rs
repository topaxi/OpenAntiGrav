//! Embedded sprite textures inside a Pulse `.pob` blob.
//!
//! # An emitter's own texture sits at a fixed position, not through the slot table
//!
//! Every `Data\Psys\*.POB`'s slot table resolves a texture-path *string*
//! (`Z:\WipeoutPSP\X2\Data\Psys\Tex\<name>.tga`, see the parent module) but
//! the pixels behind that string were, until now, off the disc entirely -
//! `docs/formats/pob.md`'s "unlocated texture reference gap". They are not:
//! **the PSP corpus embeds a texture record directly after an emitter's own
//! fixed-size record**, at `resource_base + emitter.offset + EMITTER_LEN`,
//! addressed purely positionally - no slot, no fixup, no string lookup
//! needed to find *this* copy.
//!
//! Confirmed two ways. First, a corpus-wide structural scan
//! (`crates/assets/tests/pob_ground_truth.rs`,
//! `every_psp_root_emitter_texture_is_where_the_layout_says`) finds a
//! header-shaped record at exactly that offset for 29 of 35 PSP files' root
//! emitter (the other six - `WO_PLASMA_FLASH`, `WO_RAIN`, `WO_SNOW`,
//! `WO_LEACHBEAM_CHARGING`, `WO_REPULSER`, `WO_ROCKET_FLARE` - have none
//! there; the first three are continuous/ambient effects with no
//! per-particle sprite at all, plausibly drawing an untextured quad).
//! Second, `WO_SHIP_COLL_SPARK_DAMAGE`
//! (`docs/formats/pob.md`'s four-emitter sibling tree, offsets `+0x0`,
//! `+0xd20`, `+0x2320`, `+0x2fe0`) has a header at all four of
//! `resource_base + offset + EMITTER_LEN`, and the root one is uniquely
//! sized **32x32, 3 levels** - matching `quakesmoke32x32.tga`, the texture
//! `pob.md` already named for this emitter from the slot-resolved string -
//! while the other three all point at the *same* shared palette/pixel pool,
//! matching the documented "one shared glow for the bright three"
//! (`orange_glow2.tga`). The pool is not contiguous with any one header: a
//! header only carries its own `pixel_offset`/`palette_offset`, and multiple
//! headers legitimately share one target. **PS2's 41 `.pob` files embed
//! none of this at all** - the same positional scan finds zero header-shaped
//! records in the whole PS2 corpus, at any offset, not just the emitter
//! positions - consistent with the 62% vs 25.62% coverage gap `pob.md`
//! already measured between the two platforms. Wipeout HD/Fury does not use
//! this mechanism either: its sprites are separate `data/psys/tex/*.gtf`
//! PSARC entries (see `crates/render/src/psys.rs`), and this module has
//! never been run against a big-endian file.
//!
//! # The header
//!
//! ```text
//! +0x00  u16  width
//! +0x02  u16  height
//! +0x04  u8   bits per pixel (8 on every real header)
//! +0x05  u8   mip levels (3 or 4 on every real header)
//! +0x06  u8   flags - bit 0 is the GE swizzle bit, bits 3-4 select the
//!             level mode (see "What the runtime bind reads" below). Zero on
//!             most headers, 224 on one (WO_SHIP_COLL_SPARK_DAMAGE's `+0x1758`),
//!             which sets neither of the bits the bind reads
//! +0x07  u8   unknown - zero on every header seen
//! +0x08  u32  palette_bytes (1024 on every real header: 256 RGBA8888 entries)
//! +0x0c  u32  pixel_bytes, the whole mip chain
//! +0x10  u32  pixel offset, **from the resource base** - a fixup site
//! +0x14  u32  palette offset, the same
//! ```
//!
//! 32 bytes total. Only level 0 is exposed here, the same choice
//! `oag_texture::texture::Texture` makes for the standalone `.mip` container
//! this format otherwise resembles - `pixel_bytes` covers the whole mip
//! chain (validated) but [`EmbeddedTexture::indices`] is always exactly
//! `width * height` bytes.
//!
//! Confidence **85** for the header layout and the positional-addressing
//! rule (four independent corpus-wide checks agree: the root-emitter hit
//! rate, the sibling-tree hit rate, the quakesmoke/orange_glow2 size and
//! sharing match, and the PS2 zero-hit control); **40** for what, if
//! anything, distinguishes the six root emitters with no positional texture
//! from the twenty-nine that have one.
//!
//! # The two offsets are from the resource base, not the blob - 2026-09-24
//!
//! This module used to read `+0x10`/`+0x14` as plain offsets into the whole
//! blob. **They are not: both fields are pointer-fixup sites**, and the
//! loader's `*(resource_base + slot) += resource_base` (`FUN_088f8e38`, see
//! `docs/formats/pob.md`'s "The slot table is a pointer-fixup table") turns
//! each into a live pointer at `resource_base + value`. Three legs:
//!
//! - **The slot table names them.** On every PSP emitter carrying a header,
//!   the header's `+0x10` and `+0x14`, taken relative to the resource base,
//!   are both entries of the file's own slot table
//!   (`crates/assets/tests/pob_ground_truth.rs`,
//!   `every_psp_texture_pointer_is_a_fixup_site`). Relative to the emitter
//!   record those are `+0x9c8` and `+0x9cc` - the two slot targets
//!   `docs/ghidra/functions/psp-pulse-usa/particle-system.md` had listed as
//!   unexplained.
//! - **The runtime bind dereferences them as pointers.** `FUN_08928b10`,
//!   called from `ParticleSystem_DrawParticle` on the emitter's texture
//!   block, returns without binding when either word is zero, hands `+0x10`
//!   to `Gu_TexImage` and `+0x14` to the CLUT load, and reads `+0x06`'s
//!   bit 0 into the texture mode's swizzle argument.
//! - **The pixels only read as pictures this way.** Read at the old,
//!   unbased offset, `WO_QUAKE`'s `fireballs` sprite came out half a row
//!   shifted with its first bytes taken from the palette, and palette entry
//!   0 - the index covering most of the sprite - read as an opaque navy
//!   `(11, 30, 53, 88)`: 96 bytes (the resource base) before the real
//!   table, whose entry 0 is black. Read from the base, every sprite in
//!   `WO_QUAKE`, `WO_ROCKET_EXPLO` and `WO_ROCKET_EXPLO_TRACK` is a clean,
//!   centred picture on a black or transparent field.
//!
//! Confidence **92** for the base-relative reading: the fixup mechanism is
//! itself confirmed live (`pob.md`), and these two fields are in its table.
//!
//! # A positional texture is not proven to be the one a slot names
//!
//! `WO_SHIP_COLL_SPARK_DAMAGE` is the clean case: root's positional texture
//! is 32x32/3-level, matching `quakesmoke32x32.tga` in both size and the
//! slot-resolved string `pob.md` already named for it. `WO_PLASMA_HEAD` is
//! not: its root emitter's own slot (fixup site `+0x4c4`, the same field
//! `pob.md`'s collision-spark section already reads as "an emitter's own
//! texture slot") resolves to `shuriken_trail_anim.tga` - a leftover string
//! with no embedded pixels anywhere in the file - while the *positional*
//! texture at root's own `record_end` decodes to neither that name nor any
//! other string in the file; visually it reads as an unrelated dark
//! arc/checker pattern (`data/shots/plasma_head_root.png`, gitignored, not
//! committed). The file's other three positional textures **do** land on
//! their neighbouring strings by name (`WO_RING_BW_64x64.tga`,
//! `plasma_glow_64x64.tga`, `psysed_default_glow.tga`) and decode to
//! pictures matching those names. So positional presence is solid; positional
//! *identity* - "this is the texture named by emitter X's own slot" - is
//! confirmed only when a neighbouring string happens to agree, and is open
//! wherever one doesn't. Prefer the positional texture as the "does this
//! emitter have a sprite at all" signal; treat a name for it as unverified
//! unless a nearby string corroborates it.
//!
//! **The gap is real, not cosmetic**: checked by comparing
//! `(palette_offset, pixel_offset)` pairs across `WO_PLASMA_HEAD`'s four
//! headers - if root's positional texture were secretly the same bytes as
//! `plasma_glow_64x64.tga`'s header (`+0x1f40`), the two would share a
//! pool the way `WO_SHIP_COLL_SPARK_DAMAGE`'s three siblings do. They
//! don't; all four of this file's pools are distinct. Root's positional
//! texture is its own unique pixel data with no string anywhere in the
//! file naming it. Resolving what actually binds it (if anything does) is
//! a live-trace question - `FUN_08916610`'s texture bind, per
//! `docs/ghidra/functions/psp-pulse-usa/particle-system.md` - not a file-bytes one.

use oag_formats::ByteOrder;

/// Bytes of the header this module reads.
pub const TEXTURE_HEADER_LEN: usize = 0x20;

/// A texture embedded in a `.pob` blob, borrowed from the same bytes
/// [`crate::pob::ParticleSystem::parse`] was given.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EmbeddedTexture<'a> {
    /// `+0x00`.
    pub width: u16,
    /// `+0x02`.
    pub height: u16,
    /// `+0x04`, always 8 on a real header.
    pub bits_per_pixel: u8,
    /// `+0x05`, mip levels in the chain `pixel_bytes` covers. Only level 0
    /// is exposed as [`EmbeddedTexture::indices`].
    pub levels: u8,
    /// `+0x06`: bit 0 is the GE swizzle flag ([`EmbeddedTexture::swizzled`]),
    /// bits 3-4 the runtime bind's level-mode selector.
    pub flags: u8,
    /// `+0x07`, zero on every header seen.
    pub unknown: u8,
    /// `+0x08..+0x0c` bytes of [`EmbeddedTexture::palette`].
    pub palette_bytes: u32,
    /// Raw palette bytes, `palette_bytes` long: 256 RGBA8888 entries on
    /// every real header. Not yet chunked into `[u8; 4]` here, so a caller
    /// that wants entries can `.as_chunks::<4>()` - kept raw rather than
    /// parsed so this module stays free of a colour type of its own.
    pub palette: &'a [u8],
    /// Where [`EmbeddedTexture::palette`] starts in the blob [`parse_at`] was
    /// given: the stored `+0x14` value plus the resource base - see the
    /// module documentation. Exists so a caller (`pob_coverage`) can claim
    /// the span without recovering it from the slice's own address.
    pub palette_offset: usize,
    /// Level 0's palette indices, exactly `width * height` bytes - one byte
    /// per pixel, since every real header is 8 bits per pixel.
    pub indices: &'a [u8],
    /// Where [`EmbeddedTexture::indices`] starts. See
    /// [`EmbeddedTexture::palette_offset`].
    pub pixel_offset: usize,
}

impl EmbeddedTexture<'_> {
    /// Whether `+0x06`'s bit 0 asks the GE for its block swizzle - the bit
    /// the runtime bind passes to the texture mode.
    #[must_use]
    pub fn swizzled(&self) -> bool {
        self.flags & 1 != 0
    }

    /// Level 0 as straight RGBA8888, `width * height * 4` bytes, row-major
    /// from the top: every index looked up in the palette, unswizzled first
    /// when [`Self::swizzled`] says so. A palette shorter than an index
    /// reaches reads as transparent black rather than panicking.
    #[must_use]
    pub fn rgba8(&self) -> Vec<u8> {
        let (width, height) = (usize::from(self.width), usize::from(self.height));
        let linear;
        let indices = if self.swizzled() {
            linear = oag_formats::swizzle::unswizzle(self.indices, width, height);
            &linear[..]
        } else {
            self.indices
        };
        let mut out = Vec::with_capacity(width * height * 4);
        for &index in indices {
            let at = usize::from(index) * 4;
            match self.palette.get(at..at + 4) {
                Some(entry) => out.extend_from_slice(entry),
                None => out.extend_from_slice(&[0, 0, 0, 0]),
            }
        }
        out
    }
}

/// Reads the texture positioned at `base + offset`, the way
/// [`crate::pob::ParticleSystem::embedded_texture`] calls it: `base` is the
/// resource base, `offset` is `emitter.offset + `[`crate::pob::EMITTER_LEN`].
///
/// `None` for anything that does not look like a real header - out of
/// range, an implausible width/height/depth, or a palette/pixel region that
/// does not fit `data`. There is no [`crate::pob::Error`] variant for this:
/// unlike every other field this module's sibling reads, a missing embedded
/// texture is not a corrupt file, it is the documented common case (six of
/// the PSP corpus's thirty-five root emitters, and all of PS2's).
#[must_use]
pub fn parse_at(
    data: &[u8],
    order: ByteOrder,
    base: usize,
    offset: usize,
) -> Option<EmbeddedTexture<'_>> {
    let start = base.checked_add(offset)?;
    if data.len() < start.checked_add(TEXTURE_HEADER_LEN)? {
        return None;
    }

    let width = order.u16(data, start);
    let height = order.u16(data, start + 2);
    let bits_per_pixel = data[start + 4];
    let levels = data[start + 5];
    let flags = data[start + 6];
    let unknown = data[start + 7];
    let palette_bytes = order.u32(data, start + 8);
    let pixel_bytes = order.u32(data, start + 0x0c);
    // Both are fixup sites the loader adds the resource base to - see the
    // module documentation's 2026-09-24 section.
    let pixel_offset = base.checked_add(order.u32(data, start + 0x10) as usize)?;
    let palette_offset = base.checked_add(order.u32(data, start + 0x14) as usize)?;

    if !width.is_power_of_two() || !(8..=256).contains(&width) {
        return None;
    }
    if !height.is_power_of_two() || !(8..=256).contains(&height) {
        return None;
    }
    if bits_per_pixel != 8 || !(1..=6).contains(&levels) {
        return None;
    }
    if palette_bytes == 0 || pixel_bytes == 0 {
        return None;
    }

    let palette_end = palette_offset.checked_add(palette_bytes as usize)?;
    let level0_len = usize::from(width) * usize::from(height);
    let pixel_end = pixel_offset.checked_add(pixel_bytes as usize)?;
    if palette_end > data.len() || pixel_end > data.len() || level0_len > pixel_bytes as usize {
        return None;
    }

    Some(EmbeddedTexture {
        width,
        height,
        bits_per_pixel,
        levels,
        flags,
        unknown,
        palette_bytes,
        palette: &data[palette_offset..palette_end],
        palette_offset,
        indices: &data[pixel_offset..pixel_offset + level0_len],
        pixel_offset,
    })
}

#[cfg(test)]
mod tests;
