//! [`ModelTexture`] and the two forms its texels come in.
//!
//! Split out of `mesh.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`, in the change that stopped copying a texture
//! per material slot that names it.

/// One texture per material slot, positional and never compacted.
///
/// The shape [`crate::mesh::Model::textures`] and
/// [`crate::mesh::Model::lightmaps`] both have, and what every function that
/// builds or reads a set of them passes around. `Option` because a slot whose
/// texture this build cannot decode has to stay in place - see
/// [`ModelTexture`] for why the sharing is an `Arc`.
pub type TextureSlots = Vec<Option<std::sync::Arc<ModelTexture>>>;

/// A texture decoded from the model.
///
/// **Held by `Arc` everywhere, because a slot is not a texture.** The slots in
/// [`Model::textures`] are positional - a chunk names its material by ordinal -
/// so the same texture has to appear in as many of them as name it, and a
/// circuit names far fewer distinct textures than it has slots: Talon's
/// Junction has 442 material slots over 175 textures, with one lightmap atlas
/// named by 275 of them. Filling those slots by value retained **1,858 MiB**
/// where 277 MiB of distinct texels existed, and handed the same duplication to
/// the GPU as 884 uploads of 175 pictures. Sharing costs a pointer and leaves
/// the positional layout exactly as it was.
#[derive(Debug, Clone)]
pub struct ModelTexture {
    pub label: String,
    pub width: u32,
    pub height: u32,
    /// The texels, in the form they will be uploaded in.
    pub texels: Texels,
}

/// How a [`ModelTexture`] carries its texels.
///
/// **Two forms because the disc has two.** Wipeout HD's `.gtf` textures are
/// block-compressed with a mip chain already authored - 6,482 of the 7,131
/// block-compressed 2D files on the disc carry one - and the RSX sampled those
/// blocks directly. Decoding them to RGBA8 to upload costs 4.5x the memory for
/// a picture no better than the one the disc ships: 348 MiB against 78 MiB for
/// the textures one race touches. Every other title's textures arrive already
/// decoded, and so does an HD one whose file authors no chain, so both forms
/// stay.
#[derive(Debug, Clone)]
pub enum Texels {
    /// Straight RGBA8, base level only. The renderer box-filters its own mip
    /// chain from this.
    Rgba8(Vec<u8>),
    /// The disc's own blocks and the disc's own mip chain, base level first,
    /// uploaded without a decode.
    ///
    /// One entry per mip level, each tightly packed - a texture declaring a
    /// pitch takes the [`Self::Rgba8`] path instead, which is 5 files on the
    /// whole HD disc. An adapter without `TEXTURE_COMPRESSION_BC` decodes the
    /// base level back through [`oag_formats::gtf::decode_level`] rather than
    /// drawing nothing.
    Blocks {
        format: BlockFormat,
        levels: Vec<Vec<u8>>,
    },
}

/// A block-compressed texel format, as the disc stores it.
///
/// The three the DXT family covers and the three Wipeout HD ships; the names
/// are the hardware's rather than the file's, because that is the side this
/// binds to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockFormat {
    /// DXT1: 8 bytes per 4x4 block.
    Bc1,
    /// DXT2/3: 16 bytes, four bits of alpha per texel.
    Bc2,
    /// DXT4/5: 16 bytes, interpolated alpha.
    Bc3,
}

impl BlockFormat {
    /// The one this file format names, or `None` for an uncompressed one.
    #[must_use]
    pub const fn of_gtf(format: oag_formats::gtf::Format) -> Option<Self> {
        match format {
            oag_formats::gtf::Format::Dxt1 => Some(Self::Bc1),
            oag_formats::gtf::Format::Dxt23 => Some(Self::Bc2),
            oag_formats::gtf::Format::Dxt45 => Some(Self::Bc3),
            _ => None,
        }
    }

    /// The `.gtf` format byte this came from, for a decode back to RGBA8.
    #[must_use]
    pub const fn as_gtf(self) -> oag_formats::gtf::Format {
        match self {
            Self::Bc1 => oag_formats::gtf::Format::Dxt1,
            Self::Bc2 => oag_formats::gtf::Format::Dxt23,
            Self::Bc3 => oag_formats::gtf::Format::Dxt45,
        }
    }

    /// What the texture is bound as.
    ///
    /// **Unorm rather than `*Srgb`**, the same choice and the same reason as
    /// the `Rgba8Unorm` every other upload uses: this pipeline works in gamma
    /// space throughout and nothing linearises. See
    /// [ADR-0020](../../../docs/architecture/adr/0020-gamma-authoritative-colour-space.md).
    #[must_use]
    pub const fn wgpu(self) -> wgpu::TextureFormat {
        match self {
            Self::Bc1 => wgpu::TextureFormat::Bc1RgbaUnorm,
            Self::Bc2 => wgpu::TextureFormat::Bc2RgbaUnorm,
            Self::Bc3 => wgpu::TextureFormat::Bc3RgbaUnorm,
        }
    }

    /// Bytes one 4x4 block occupies.
    #[must_use]
    pub const fn block_len(self) -> u32 {
        match self {
            Self::Bc1 => 8,
            Self::Bc2 | Self::Bc3 => 16,
        }
    }
}

impl ModelTexture {
    /// One already-decoded RGBA8 texture.
    #[must_use]
    pub fn rgba8(label: String, width: u32, height: u32, rgba: Vec<u8>) -> Self {
        Self {
            label,
            width,
            height,
            texels: Texels::Rgba8(rgba),
        }
    }

    /// The base level as RGBA8, or `None` for one still in its blocks.
    ///
    /// For a consumer that reads texels on the CPU rather than binding them and
    /// knows it is holding a decoded one - the HUD's sight sheet is the only
    /// one, and every texture it reads is a PSP `.vex`, which never takes the
    /// block path. [`Self::to_rgba`] is the one that decodes.
    #[must_use]
    pub fn rgba(&self) -> Option<&[u8]> {
        match &self.texels {
            Texels::Rgba8(rgba) => Some(rgba),
            Texels::Blocks { .. } => None,
        }
    }

    /// The base level as RGBA8, decoding the disc's blocks if that is what it
    /// holds.
    ///
    /// For a consumer that has to read texels whatever form they arrived in -
    /// the `hd_*` probes under `crates/render/examples`, which measure a
    /// texture's own colours rather than binding it. Allocates only on the
    /// block path; `None` there for a layout
    /// [`oag_formats::gtf::decode_level`] refuses.
    #[must_use]
    pub fn to_rgba(&self) -> Option<std::borrow::Cow<'_, [u8]>> {
        match &self.texels {
            Texels::Rgba8(rgba) => Some(std::borrow::Cow::Borrowed(rgba)),
            Texels::Blocks { format, levels } => {
                let decoded = oag_formats::gtf::decode_level(
                    format.as_gtf(),
                    levels.first()?,
                    self.width,
                    self.height,
                    0,
                )?;
                Some(std::borrow::Cow::Owned(
                    decoded.into_iter().flatten().collect(),
                ))
            }
        }
    }
}
