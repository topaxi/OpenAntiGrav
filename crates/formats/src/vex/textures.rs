//! The `.vex` texture block: what it holds, and how a row of it is laid out.
//!
//! Split out of `vex.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change. What holds it
//! together is that everything here reads the block that follows the node tree -
//! the one section of the format whose PS3 layout is unread, and therefore the
//! one place a big-endian file is answered with "absent" rather than with bytes.

use super::*;

/// A texture embedded in the file.
#[derive(Debug, Clone)]
pub struct EmbeddedTexture {
    /// Original asset path, from the node name.
    ///
    /// This is the **authoring** path on the artists' machine, e.g.
    /// `Z:/WipeoutPSP/X2/Data/Ships/Feisar/Textures/engine_general.tga`, and it
    /// is only present when the node header is long enough to carry a name.
    /// Track `Texture` nodes have a 32-byte header with the name field zeroed,
    /// so on a track this is `None` for every texture and
    /// [`asset_path`](Self::asset_path) is the only name available.
    pub name: Option<String>,
    /// Runtime asset path, from payload `+0x38`, e.g.
    /// `Data\Ships\Feisar\Textures\engine_general.tga`.
    ///
    /// Present on ships *and* tracks, which is what makes it the field to match
    /// a texture by. Backslash-separated and in the game's own spelling, so it
    /// is also what [`crate::wad::hash_name`] would take.
    pub asset_path: Option<String>,
    /// Width in pixels.
    pub width: u16,
    /// Height in pixels.
    pub height: u16,
    /// 4 or 8.
    pub bits_per_pixel: u8,
    /// Number of mip levels present. Only the base level is decoded.
    pub mip_count: u8,
    /// Palette, RGBA8888.
    pub palette: Vec<[u8; 4]>,
    /// Base-level pixel indices, one per pixel.
    pub indices: Vec<u8>,
}

impl EmbeddedTexture {
    /// Expands the base level to RGBA8888.
    #[must_use]
    pub fn to_rgba(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.indices.len() * 4);
        for &i in &self.indices {
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

/// Offset of the runtime asset path inside a `Texture` node's payload.
///
/// The two pointer fields at `+0x10` and `+0x14` are zero at rest and patched at
/// load, so the path is the last field of the header that survives on disc.
const TEXTURE_ASSET_PATH: usize = 0x38;

/// The runtime asset path out of a `Texture` node's payload.
///
/// Separate from [`textures`] because it needs only the node, not the embedded
/// pixel block, so it also answers "what does this file reference" on a PS2
/// scene whose texture block is empty.
#[must_use]
pub fn texture_asset_path(payload: &[u8]) -> Option<String> {
    cstr_at(payload, TEXTURE_ASSET_PATH)
}

/// Extracts the textures appended after the node tree.
///
/// Their data is not pointed at from anywhere: the header's pointer fields are
/// zero at rest and patched at load. Instead each texture's palette and texels
/// are packed back to back in node order, starting immediately after the tree.
///
/// That the sizes add up exactly to the header's declared texture length is
/// what confirms the packing, and this function checks it.
///
/// # Position is the identity
///
/// Materials name a texture by its **ordinal among the `Texture` nodes**, so the
/// returned vector is positional and one entry per node, including nodes this
/// build cannot decode. Those come back as `None`.
///
/// Dropping them instead would silently renumber every later texture, which is
/// not a decoding error that shows up as an error: it shows up as a model wearing
/// the wrong skins. Use `.iter().flatten()` for a plain list.
pub fn textures(data: &[u8]) -> Result<Vec<Option<EmbeddedTexture>>> {
    let block = FILE_HEADER_LEN + tree_len(data)?;
    let mut at = block;
    let mut out = Vec::new();

    // A zero-length texture block is not a malformed file: PS2 `.vex` scenes
    // carry `Texture` nodes with real dimensions but no palette/texel bytes
    // after the tree at all, unlike PSP's. Every node's declared clut/texel
    // size would otherwise be read as if the bytes were there and walk past
    // the end of the file. Where the actual pixels live on PS2 is not yet
    // known; every texture in the model comes back `None` until it is.
    //
    // **A big-endian block counts as absent for the same reason.** HD's
    // `ship.vex` really does declare 576 bytes of one, but everything below
    // reads the per-node header through [`le`], so that 13,792-byte file
    // claimed a texture ending at byte 1,073,886,112. Its real texels are in
    // `.gtf`, which nothing here decodes; see `docs/formats/hd-status.md`.
    let embedded = texture_len(data)? > 0 && byte_order(data) == ByteOrder::Little;

    // **The `Texture` id from this file's version**, not version 6's `0x3c1`.
    // A version-4 file numbers it `0x373`, so the constant matched nothing and
    // every material resolved to no texture at all - which draws a whole track
    // white rather than failing, and is exactly what a Pure race looked like.
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
            // The data is still there and still has to be stepped over, so the
            // *next* texture stays correctly positioned in the block. Only this
            // one is unavailable.
            at = end;
            out.push(None);
            continue;
        }

        let palette: Vec<[u8; 4]> = data[at..at + clut_size].as_chunks::<4>().0.to_vec();

        // Only the base level; mips follow it and are not needed for viewing.
        let pixels = usize::from(width) * usize::from(height);
        let row = texture_row_bytes(width, bits_per_pixel);
        let stride = texture_row_stride(width, bits_per_pixel);
        // **The pre-swizzle flag, read on version 4 and below only.**
        //
        // Bit 0 of the flags byte at `+0x06` means the texels are already in the
        // GE's 16-byte by 8-row block order, so a literal read comes out
        // scrambled - which is what every Pure model texture looked like, the
        // flag being `0x61` there against Pulse's `0xe4`.
        //
        // **Gated on the version rather than read unconditionally**, and that is
        // not caution for its own sake: a corpus sweep found bit 0 set on 88 of
        // Pulse PSP's 5,375 `Texture` nodes and 120 of the PS2 pressing's 8,972,
        // on ship liveries and effects rather than on the font atlases the
        // original claim was about. So reading it on version 6 would change what
        // those 88 decode to, and no ground-truth screenshot covers the one model
        // that changed - the regression would pass `just test-data`. Whether
        // those nodes really are swizzled is an open question with its own row on
        // `docs/formats/pure-status.md`; this change deliberately does not
        // settle it, and version 4 is the generation where the evidence is
        // unambiguous.
        let swizzled = matches!(classes.version, 0..=4)
            && p.get(6)
                .is_some_and(|flags| flags & crate::texture::FLAG_SWIZZLED != 0);
        let linear;
        let texels = if swizzled {
            linear =
                crate::texture::unswizzle(&data[at + clut_size..end], stride, usize::from(height));
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
        // `to_rgba` promises exactly `width * height` texels and its consumers
        // index against those dimensions, so a block too short to hold every
        // row (which no Pulse PSP texture is - see the closure argument in
        // `texture_row_stride` - but a file sized by some other rule could be)
        // is padded out rather than handed on short. The same call caps a
        // 4-bit odd width, where the last byte carries a pixel past the end.
        indices.resize(pixels, 0);

        out.push(Some(EmbeddedTexture {
            name: node.name,
            asset_path: cstr_at(p, TEXTURE_ASSET_PATH),
            width,
            height,
            bits_per_pixel,
            mip_count,
            palette,
            indices,
        }));
        at = end;
    }

    Ok(out)
}

/// How many bytes of one texture row hold actual picture, before padding.
#[must_use]
pub fn texture_row_bytes(width: u16, bits_per_pixel: u8) -> usize {
    (usize::from(width) * usize::from(bits_per_pixel)).div_ceil(8)
}

/// The stride between texture rows: [`texture_row_bytes`] rounded up to 16.
///
/// **The GE's texture buffer width is in units of 16 bytes**, so a row narrower
/// than that is padded out and the next row starts on the next 16-byte
/// boundary, not immediately after the picture. A decoder that reads rows back
/// to back is right only where the picture already fills the stride - which is
/// every 4-bit texture 32 pixels or wider and every 8-bit one 16 or wider, i.e.
/// nearly all of them, which is exactly why this went unnoticed.
///
/// **Confidence 90.** Measured rather than assumed: every `Texture` node
/// declares its own total texel-block size at payload `+0x0c`, and summing this
/// stride over each texture's declared mip levels reproduces that number for
/// **135 of 135** textures on `16_Track`. The unpadded formula reproduces
/// **22** of them - it agrees only where the padding is a no-op. See
/// `crates/formats/tests/texture_stride_ground_truth.rs`, which re-measures it
/// across circuits and ships, and `docs/formats/vex.md`.
///
/// What this is *not*: swizzling. A swizzled PSP texture is reordered into
/// 16-byte by 8-row blocks, and reading it row-wise would corrupt every texture
/// wider than the block rather than only the narrow ones. Every 64-pixel-wide
/// 4-bit texture on the disc decodes correctly read row-wise, so this data is
/// linear with padded rows.
#[must_use]
pub fn texture_row_stride(width: u16, bits_per_pixel: u8) -> usize {
    texture_row_bytes(width, bits_per_pixel).next_multiple_of(16)
}
