//! The `.vex` texture block: what it holds, and how a row of it is laid out.
//!
//! Split out of `vex.rs` under the 1,000-line rule; no behaviour change.
//! Everything here reads the block after the node tree, the one section whose
//! PS3 layout is unread, so a big-endian file is answered with "absent".

use super::*;

/// A texture embedded in the file.
#[derive(Debug, Clone)]
pub struct EmbeddedTexture {
    /// Original asset path, from the node name: the **authoring** path on the
    /// artists' machine (`Z:/WipeoutPSP/X2/Data/Ships/Feisar/Textures/engine_general.tga`),
    /// present only when the node header is long enough to carry a name. Track
    /// `Texture` nodes have a 32-byte header with the name zeroed, so there
    /// [`asset_path`](Self::asset_path) is the only name.
    pub name: Option<String>,
    /// Runtime asset path, from payload `+0x38` (`Data\Ships\Feisar\Textures\engine_general.tga`).
    /// Present on ships *and* tracks, so the field to match a texture by;
    /// backslash-separated in the game's spelling, as `oag_formats::wad::hash_name`
    /// takes it.
    pub asset_path: Option<String>,
    /// Width in pixels.
    pub width: u16,
    /// Height in pixels.
    pub height: u16,
    /// 4 or 8.
    pub bits_per_pixel: u8,
    /// Number of mip levels the header declares.
    pub mip_count: u8,
    /// Palette, RGBA8888.
    pub palette: Vec<[u8; 4]>,
    /// Base-level pixel indices, one per pixel.
    pub indices: Vec<u8>,
    /// The disc's own levels below the base, level 1 first, each
    /// `max(width >> n, 1)` by `max(height >> n, 1)` indices into the same
    /// [`palette`](Self::palette).
    ///
    /// **Authored, not derived**: the bytes after the base level in the texel
    /// block, at the padded stride [`texture_row_stride`] establishes. Empty for a
    /// one-level texture. A pre-swizzled node (flags bit 0) of version 5 or later
    /// has each level unswizzled on its own first; a version-4 one keeps none (its
    /// chain is synthesised).
    pub levels: Vec<Vec<u8>>,
}

impl EmbeddedTexture {
    /// Expands the base level to RGBA8888.
    #[must_use]
    pub fn to_rgba(&self) -> Vec<u8> {
        self.expand(&self.indices)
    }

    /// Every level the disc authors, base first, as RGBA8888 (one entry when the
    /// texture declares one level or its levels are not read; see [`Self::levels`]).
    #[must_use]
    pub fn levels_rgba(&self) -> Vec<Vec<u8>> {
        std::iter::once(&self.indices)
            .chain(&self.levels)
            .map(|indices| self.expand(indices))
            .collect()
    }

    fn expand(&self, indices: &[u8]) -> Vec<u8> {
        let mut out = Vec::with_capacity(indices.len() * 4);
        for &i in indices {
            let c = self
                .palette
                .get(i as usize)
                .copied()
                .unwrap_or([255, 0, 255, 255]);
            out.extend_from_slice(&c);
        }
        out
    }
}

/// Offset of the runtime asset path inside a `Texture` node's payload: the last
/// header field that survives on disc (the pointers at `+0x10`/`+0x14` are zero
/// at rest and patched at load).
const TEXTURE_ASSET_PATH: usize = 0x38;

/// The runtime asset path out of a `Texture` node's payload.
///
/// Separate from [`textures`]: it needs only the node, so it also answers "what
/// does this file reference" on a PS2 scene whose texture block is empty.
#[must_use]
pub fn texture_asset_path(payload: &[u8]) -> Option<String> {
    cstr_at(payload, TEXTURE_ASSET_PATH)
}

/// Extracts the textures appended after the node tree.
///
/// No pointer reaches their data (the header's pointers are zero at rest):
/// each texture's palette and texels are packed back to back in node order from
/// the end of the tree. That the sizes add up exactly to the header's declared
/// texture length confirms the packing, and this function checks it.
///
/// # Position is the identity
///
/// Materials name a texture by its **ordinal among the `Texture` nodes**, so the
/// result is positional, one entry per node, `None` for nodes this build cannot
/// decode. Dropping them would silently renumber every later texture: a model
/// wearing the wrong skins, not an error. Use `.iter().flatten()` for a plain
/// list.
pub fn textures(data: &[u8]) -> Result<Vec<Option<EmbeddedTexture>>> {
    let block = FILE_HEADER_LEN + tree_len(data)?;
    let mut at = block;
    let mut out = Vec::new();

    // A zero-length texture block is not malformed: PS2 `.vex` scenes carry
    // `Texture` nodes with dimensions but no palette/texel bytes after the tree
    // (their pixels' location is unknown), and reading the declared sizes would
    // walk past the end of the file. Every texture comes back `None`.
    //
    // **A big-endian block counts as absent for the same reason.** HD's
    // `ship.vex` declares 576 bytes of one, but everything below reads the node
    // header through [`le`], so that 13,792-byte file claimed a texture ending at
    // byte 1,073,886,112. Its real texels are in `.gtf`, which nothing here
    // decodes; see `docs/formats/hd-status.md`.
    let embedded = texture_len(data)? > 0 && byte_order(data) == ByteOrder::Little;

    // **The `Texture` id from this file's version**, not version 6's `0x3c1`: a
    // version-4 file numbers it `0x373`, so the constant matched nothing and a
    // whole track drew white (what a Pure race looked like).
    let classes = classes_of(data)?;
    let texture_class = classes.texture;
    for node in nodes(data)?
        .into_iter()
        .filter(|n| Some(n.class_id) == texture_class)
    {
        if !embedded {
            out.push(None);
            continue;
        }

        let p = &data[node.payload()];
        if p.len() < 0x10 {
            out.push(None);
            continue;
        }

        let width = u16_at(p, 0);
        let height = u16_at(p, 2);
        let bits_per_pixel = p[4];
        let mip_count = p[5];
        let clut_size = u32_at(p, 8) as usize;
        let texel_size = u32_at(p, 12) as usize;

        let end = at + clut_size + texel_size;
        if end > data.len() {
            return Err(Error::OutOfBounds {
                what: "embedded texture",
                end,
                len: data.len(),
            });
        }
        if !matches!(bits_per_pixel, 4 | 8) {
            // The data is still there and is stepped over, keeping the next
            // texture positioned. Only this one is unavailable.
            at = end;
            out.push(None);
            continue;
        }

        let palette: Vec<[u8; 4]> = data[at..at + clut_size].as_chunks::<4>().0.to_vec();

        // Only the base level; mips follow it and are not needed for viewing.
        let pixels = usize::from(width) * usize::from(height);
        let row = texture_row_bytes(width, bits_per_pixel);
        let stride = texture_row_stride(width, bits_per_pixel);
        // **The pre-swizzle flag.** Bit 0 of the flags byte at `+0x06` means the
        // texel block is already in the GE's 16-byte by 8-row block order, so a
        // literal read comes out scrambled.
        //
        // **Acted on for every version** (levels below the base only from version
        // 5). It was once gated to version 4 and below because the bit appears on
        // 88 Pulse PSP nodes with no known meaning. A live PPSSPP capture of
        // Pulse's shield shell (`pulse_shield_test_ADD`, flags `0xe5`) settled it:
        // the GE reads every Pulse texture swizzled (`TEXMODE` bit 0 on every
        // draw), the shell's RAM bytes are the file's at every level, while an
        // unflagged hull texture is reordered by the loader. Rendering the shell
        // from the dump with the unswizzled texture reproduces the original's
        // pixels (blue within 0.1 %). See
        // `docs/ghidra/functions/psp-pulse-usa/shield-pickup.md`.
        //
        // **Each level is swizzled on its own**, at its own padded stride (the GE
        // gets a separate address per level), so each is unswizzled separately and
        // laid out like an unflagged block. A level under 8 rows tall with a
        // stride past 16 bytes is copied through (none exists on a flagged Pulse
        // texture; unverified).
        let swizzled = p
            .get(6)
            .is_some_and(|flags| flags & oag_formats::swizzle::FLAG_SWIZZLED != 0);
        // **Version 4 and below (Pure, Pulse's Zone wrecks)** keep the base level
        // unswizzled as one block and no authored levels, so the renderer
        // synthesises their chain (`docs/rendering/frame-audit.md`). Their levels
        // are swizzled like any other (236 of 236 flagged levels agree), but
        // handing them on would switch Pure onto the slope level rule, which
        // nothing here measured.
        let authored_levels = classes.version >= 5;
        let linear;
        let texels = if swizzled && !authored_levels {
            linear = oag_formats::swizzle::unswizzle(
                &data[at + clut_size..end],
                stride,
                usize::from(height),
            );
            &linear[..]
        } else if swizzled {
            linear = unswizzle_levels(
                &data[at + clut_size..end],
                width,
                height,
                bits_per_pixel,
                mip_count,
            );
            &linear[..]
        } else {
            &data[at + clut_size..end]
        };

        let mut indices = Vec::with_capacity(pixels);
        for y in 0..usize::from(height) {
            let Some(line) = texels.get(y * stride..y * stride + row) else {
                break;
            };
            if bits_per_pixel == 8 {
                indices.extend_from_slice(line);
            } else {
                for &b in line {
                    indices.push(b & 0x0f);
                    indices.push(b >> 4);
                }
            }
        }
        // `to_rgba` promises `width * height` texels and its consumers index
        // against them, so a block too short for every row (no Pulse PSP texture
        // is; see `texture_row_stride`) is padded out. The same call caps a 4-bit
        // odd width, where the last byte carries a pixel past the end.
        indices.resize(pixels, 0);

        // The levels below the base, when texels are in row order: each follows
        // the last at its own padded stride, stopping at the first level the block
        // cannot hold (fewer levels, not padded ones).
        let mut levels = Vec::new();
        if !swizzled || authored_levels {
            let mut offset = stride * usize::from(height);
            for level in 1..u32::from(mip_count) {
                let (w, h) = (
                    (width >> level).max(1),
                    usize::from((height >> level).max(1)),
                );
                let (row, stride) = (
                    texture_row_bytes(w, bits_per_pixel),
                    texture_row_stride(w, bits_per_pixel),
                );
                let mut plane = Vec::with_capacity(usize::from(w) * h);
                for y in 0..h {
                    let at = offset + y * stride;
                    let Some(line) = texels.get(at..at + row) else {
                        break;
                    };
                    if bits_per_pixel == 8 {
                        plane.extend_from_slice(line);
                    } else {
                        for &b in line {
                            plane.push(b & 0x0f);
                            plane.push(b >> 4);
                        }
                    }
                }
                if plane.len() < usize::from(w) * h {
                    break;
                }
                plane.truncate(usize::from(w) * h);
                levels.push(plane);
                offset += stride * h;
            }
        }

        out.push(Some(EmbeddedTexture {
            name: node.name,
            asset_path: cstr_at(p, TEXTURE_ASSET_PATH),
            width,
            height,
            bits_per_pixel,
            mip_count,
            palette,
            indices,
            levels,
        }));
        at = end;
    }

    Ok(out)
}

/// Unswizzles a texel block level by level, returning it in row order.
///
/// Each level of a pre-swizzled block is swizzled separately at its own padded
/// stride (`texture_row_stride`), as the GE reads it. A level the block is too
/// short to hold ends the walk; the rest is copied through.
fn unswizzle_levels(
    block: &[u8],
    width: u16,
    height: u16,
    bits_per_pixel: u8,
    mip_count: u8,
) -> Vec<u8> {
    let mut out = Vec::with_capacity(block.len());
    let mut offset = 0;
    for level in 0..u32::from(mip_count.max(1)) {
        let w = (width >> level).max(1);
        let h = usize::from((height >> level).max(1));
        let stride = texture_row_stride(w, bits_per_pixel);
        let Some(plane) = block.get(offset..offset + stride * h) else {
            break;
        };
        out.extend_from_slice(&oag_formats::swizzle::unswizzle(plane, stride, h));
        offset += stride * h;
    }
    out.extend_from_slice(&block[offset.min(block.len())..]);
    out
}

/// How many bytes of one texture row hold actual picture, before padding.
#[must_use]
pub fn texture_row_bytes(width: u16, bits_per_pixel: u8) -> usize {
    (usize::from(width) * usize::from(bits_per_pixel)).div_ceil(8)
}

/// The stride between texture rows: [`texture_row_bytes`] rounded up to 16.
///
/// **The GE's texture buffer width is in units of 16 bytes**: a narrower row is
/// padded and the next starts on the next 16-byte boundary. Reading rows back to
/// back is right only where the picture fills the stride (every 4-bit texture 32
/// pixels or wider, every 8-bit one 16 or wider), which is why this went
/// unnoticed.
///
/// **Confidence 90.** Every `Texture` node declares its total texel-block size at
/// payload `+0x0c`; summing this stride over the declared mip levels reproduces
/// it for **135 of 135** textures on `16_Track` (the unpadded formula: **22**).
/// See `crates/vex/tests/texture_stride_ground_truth.rs`, which re-measures it
/// across circuits and ships, and `docs/formats/vex.md`.
///
/// Not swizzling: a swizzled PSP texture is reordered into 16-byte by 8-row
/// blocks and a row-wise read corrupts every texture wider than the block. Every
/// **unflagged** 64-pixel-wide 4-bit texture decodes correctly row-wise (linear,
/// padded rows); flags bit 0 is the swizzled case (`pulse_bomb.tga`, 64 wide,
/// 4-bit, flagged, reads as sheared noise row-wise; see `unswizzle_levels`).
#[must_use]
pub fn texture_row_stride(width: u16, bits_per_pixel: u8) -> usize {
    texture_row_bytes(width, bits_per_pixel).next_multiple_of(16)
}

#[cfg(test)]
mod swizzle_tests {
    use super::unswizzle_levels;

    /// The GE's block order, written the loader's way round.
    fn swizzle(linear: &[u8], stride: usize, height: usize) -> Vec<u8> {
        let mut out = Vec::new();
        for block_row in 0..height / 8 {
            for block_col in 0..stride / 16 {
                for row in 0..8 {
                    let at = (block_row * 8 + row) * stride + block_col * 16;
                    out.extend_from_slice(&linear[at..at + 16]);
                }
            }
        }
        out
    }

    /// Each level is swizzled on its own at its own padded stride: a 64x32 4-bit
    /// texture has levels of stride 32, 16, 16 and 16 bytes.
    #[test]
    fn every_level_is_unswizzled_at_its_own_stride() {
        let levels: [(usize, usize); 4] = [(32, 32), (16, 16), (16, 8), (16, 8)];
        let mut linear = Vec::new();
        let mut swizzled = Vec::new();
        let mut seed = 1u32;
        for (stride, height) in levels {
            let plane: Vec<u8> = (0..stride * height)
                .map(|_| {
                    seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                    (seed >> 24) as u8
                })
                .collect();
            swizzled.extend(swizzle(&plane, stride, height));
            linear.extend(plane);
        }
        assert_ne!(swizzled, linear, "level 0 is a real permutation");
        assert_eq!(unswizzle_levels(&swizzled, 64, 32, 4, 4), linear);
    }

    /// A block shorter than its declared levels is returned whole, never cut.
    #[test]
    fn a_short_block_is_not_truncated() {
        let block = vec![7u8; 100];
        assert_eq!(unswizzle_levels(&block, 64, 32, 4, 4).len(), 100);
    }
}
