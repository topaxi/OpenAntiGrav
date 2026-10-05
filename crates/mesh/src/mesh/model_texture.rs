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
    /// How many mip levels the source asset itself declares, for a
    /// [`Texels::Rgba8`] texture whose chain this project synthesises.
    ///
    /// `None` means the count is unmeasured for this path, which keeps
    /// today's behaviour: the uploader synthesises a full box-filtered chain
    /// down to 1x1. **A `Some` caps the synthesised chain at the asset's own
    /// depth** - see
    /// `docs/ghidra/functions/psp-pulse-usa/mesh-draw.md`'s "the asset
    /// decides the chain depth" finding: `Texture_BuildBindList` never
    /// uploads more levels than the texture object's own `+0x05` byte, and
    /// the GE has nowhere to sample a level past what was uploaded. Wired for
    /// the PSP `.vex` embedded-texture path first.
    ///
    /// **A PSP `.vex` texture does not use this**: it arrives as
    /// [`Texels::Chain`], the disc's own levels, and the blur this field was
    /// added to chase was level *selection*, not chain depth - see
    /// `docs/rendering/frame-audit.md`. What still passes a `Some` here is a
    /// pre-swizzled Pure texture whose levels are not read.
    pub mip_count: Option<u32>,
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
    /// **The disc's own mip chain**, expanded to RGBA8, base first: level `n`
    /// is `max(width >> n, 1)` by `max(height >> n, 1)`.
    ///
    /// The PSP `.vex` textures, whose levels are authored bytes in the file
    /// (`oag_vex::vex::EmbeddedTexture::levels`) and were being thrown away and
    /// re-derived by box filter. **Its presence on a model is also what turns
    /// on the GE's slope level selection**, `texlod_slope` in `mesh.wgsl`: the
    /// levels are the game's and so is the rule that picks among them.
    Chain(Vec<Vec<u8>>),
    /// The disc's own blocks and the disc's own mip chain, base level first,
    /// uploaded without a decode.
    ///
    /// One entry per mip level, each tightly packed - a texture declaring a
    /// pitch takes the [`Self::Rgba8`] path instead, which is 5 files on the
    /// whole HD disc. An adapter without `TEXTURE_COMPRESSION_BC` decodes the
    /// base level back through [`oag_texture::gtf::decode_level`] rather than
    /// drawing nothing.
    Blocks {
        format: BlockFormat,
        levels: Vec<Vec<u8>>,
    },
    /// **Already on the GPU**, uploaded as the texture was decoded by an open
    /// [`crate::mesh_render::TextureSinkScope`], so the decoded texels never
    /// sat on the CPU beside every other texture of the circuit.
    ///
    /// The race load decodes every texture of every model before the scene
    /// builds a single `Drawable`; holding them all until then made the peak
    /// the sum of the whole circuit (Tech De Ra: 2.3 GiB of BC7 blocks).
    /// [`crate::mesh_render`]'s uploader hands this view back instead of
    /// uploading, so a model decoded under a sink binds exactly the picture a
    /// model decoded without one would have uploaded.
    ///
    /// **Tied to the device that was open when it was decoded.** The one
    /// caller that opens a scope builds the scene from the same device.
    Uploaded {
        view: wgpu::TextureView,
        /// What the upload occupies, mip chain included - see
        /// `mesh_render::texture::gpu_bytes`.
        gpu_bytes: u64,
        /// Whether it went up as the disc's own blocks.
        block_compressed: bool,
        /// How many of the source's leading mip levels were left out because
        /// the base was wider than the device's `max_texture_dimension_2d` -
        /// **chosen, not measured**, 0 for every texture on a device that
        /// holds it whole.
        dropped_levels: u32,
    },
}

/// Which way a `.gnf` went: kept as the disc's blocks, or one of the three
/// reasons it was decoded to RGBA8 and given a synthesised chain instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GnfForm {
    /// A BC7 chain of two levels or more, uploaded as authored.
    Blocks,
    /// The file authors one level only, so there is no chain to keep.
    SingleLevel,
    /// A base level that is not a whole number of 4x4 blocks, which WebGPU
    /// refuses for a compressed texture.
    OffGrid,
    /// [`oag_texture::gnf::Texture::block_levels`] refused it: a corrupt block
    /// in a level, or a file that is not one 2D chain.
    BlocksRefused,
}

/// How many `.gnf` textures of a build took each [`GnfForm`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GnfCounts {
    pub blocks: usize,
    pub single_level: usize,
    pub off_grid: usize,
    pub blocks_refused: usize,
}

impl GnfCounts {
    /// Counts one decoded `.gnf`.
    pub fn record(&mut self, form: GnfForm) {
        match form {
            GnfForm::Blocks => self.blocks += 1,
            GnfForm::SingleLevel => self.single_level += 1,
            GnfForm::OffGrid => self.off_grid += 1,
            GnfForm::BlocksRefused => self.blocks_refused += 1,
        }
    }

    /// The textures that fell back to RGBA8: everything but [`GnfForm::Blocks`].
    #[must_use]
    pub const fn fell_back(&self) -> usize {
        self.single_level + self.off_grid + self.blocks_refused
    }

    /// One clause for a loader report, empty when the build decoded no `.gnf`.
    #[must_use]
    pub fn describe(&self) -> String {
        if self.blocks + self.fell_back() == 0 {
            return String::new();
        }
        format!(
            "; {} .gnf texture(s) kept as BC7 blocks, {} decoded to RGBA8 with a synthesised chain \
             ({} single-level, {} off the block grid, {} refused as blocks)",
            self.blocks,
            self.fell_back(),
            self.single_level,
            self.off_grid,
            self.blocks_refused
        )
    }
}

/// A block-compressed texel format, as the disc stores it.
///
/// The three the DXT family covers and the three Wipeout HD ships, and BC7,
/// which the PS4's `.gnf` textures are almost all in; the names are the
/// hardware's rather than the file's, because that is the side this binds to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockFormat {
    /// DXT1: 8 bytes per 4x4 block.
    Bc1,
    /// DXT2/3: 16 bytes, four bits of alpha per texel.
    Bc2,
    /// DXT4/5: 16 bytes, interpolated alpha.
    Bc3,
    /// BC7: 16 bytes, the PS4 Omega Collection's own.
    Bc7,
}

impl BlockFormat {
    /// The one this file format names, or `None` for an uncompressed one.
    #[must_use]
    pub const fn of_gtf(format: oag_texture::gtf::Format) -> Option<Self> {
        match format {
            oag_texture::gtf::Format::Dxt1 => Some(Self::Bc1),
            oag_texture::gtf::Format::Dxt23 => Some(Self::Bc2),
            oag_texture::gtf::Format::Dxt45 => Some(Self::Bc3),
            _ => None,
        }
    }

    /// A level's blocks decoded back to RGBA8 texels, for an adapter without
    /// block compression and for a consumer that reads texels.
    ///
    /// `None` for blocks that stop short of `width` x `height`.
    #[must_use]
    pub fn decode_level(self, blocks: &[u8], width: u32, height: u32) -> Option<Vec<[u8; 4]>> {
        let gtf = match self {
            Self::Bc1 => oag_texture::gtf::Format::Dxt1,
            Self::Bc2 => oag_texture::gtf::Format::Dxt23,
            Self::Bc3 => oag_texture::gtf::Format::Dxt45,
            Self::Bc7 => return oag_texture::gnf::decode_bc7_level(blocks, width, height),
        };
        // `linear` is inert: the decoder does not consult the flag for a
        // block-compressed format.
        oag_texture::gtf::decode_level(gtf, blocks, width, height, 0, true)
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
            Self::Bc7 => wgpu::TextureFormat::Bc7RgbaUnorm,
        }
    }

    /// Bytes one 4x4 block occupies.
    #[must_use]
    pub const fn block_len(self) -> u32 {
        match self {
            Self::Bc1 => 8,
            Self::Bc2 | Self::Bc3 | Self::Bc7 => 16,
        }
    }
}

impl ModelTexture {
    /// Decodes one `.gtf` blob, or says `None` for one this build cannot.
    ///
    /// **A `.gtf` that will not decode draws nothing rather than something.**
    /// `Texture::to_rgba` refuses the RSX's Morton-swizzled layouts and
    /// cubemaps - 53 of the disc's 7,333 files - and a refusal here is an
    /// answer, not a fallback to a stand-in picture.
    ///
    /// **A blob already in DXT blocks keeps them**, with the mip chain the
    /// file authors; see [`Self::gtf_blocks`] for exactly when. Decoding those
    /// to RGBA8 was costing 4.5x the memory for the same picture.
    ///
    /// Lives here rather than beside its first caller because the block/RGBA
    /// choice is a property of [`Texels`], and a second caller now wants it.
    #[must_use]
    pub fn from_gtf(label: &str, blob: &[u8]) -> Option<Self> {
        let parsed = oag_texture::gtf::Gtf::parse(blob).ok()?;
        let texture = parsed.only()?;
        let (width, height) = texture.level_size(0);
        let texels = match Self::gtf_blocks(texture, blob) {
            Some(texels) => texels,
            None => Texels::Rgba8(texture.to_rgba(blob).ok()?.into_iter().flatten().collect()),
        };
        Some(Self {
            label: label.to_string(),
            width,
            height,
            texels,
            // Unmeasured for the decode-and-box-filter fallback: the same gap
            // `mip_count`'s own doc names, just not chased here - this path
            // is HD's, and the thread that measured the PSP one is Pulse's.
            mip_count: None,
        })
    }

    /// Decodes one `.gnf` blob, or says `None` for one this build cannot.
    ///
    /// **A BC7 chain the file authors keeps its blocks**, the way
    /// [`Self::from_gtf`] keeps a DXT one: 1 byte a texel on the CPU and on the
    /// GPU where the decoded picture is 4, and the authored chain in place of a
    /// box-filtered one. The conditions are [`Self::gtf_blocks`]'s: a chain of
    /// two levels or more, and a base on the block grid, which
    /// [`oag_texture::gnf::Texture::block_levels`] adds its own refusals to
    /// (a corrupt block in any level, a file that is not one 2D chain). Any
    /// other texture the decoder accepts falls back to RGBA8, and one it
    /// refuses draws nothing.
    #[must_use]
    pub fn from_gnf(label: &str, blob: &[u8]) -> Option<Self> {
        Self::from_gnf_form(label, blob).map(|(texture, _)| texture)
    }

    /// [`Self::from_gnf`], and which of its four outcomes the file took.
    ///
    /// What a loader counts to say how many textures of a circuit fell back to
    /// RGBA8 with a box-filtered chain, and why - see [`GnfForm`].
    #[must_use]
    pub fn from_gnf_form(label: &str, blob: &[u8]) -> Option<(Self, GnfForm)> {
        let parsed = oag_texture::gnf::Texture::parse(blob).ok()?;
        let (blocks, form) = if parsed.width % 4 != 0 || parsed.height % 4 != 0 {
            (None, GnfForm::OffGrid)
        } else {
            match parsed.block_levels(blob) {
                Ok(levels) if levels.len() > 1 => (Some(levels), GnfForm::Blocks),
                Ok(_) => (None, GnfForm::SingleLevel),
                Err(_) => (None, GnfForm::BlocksRefused),
            }
        };
        let texels = match blocks {
            Some(levels) => Texels::Blocks {
                format: BlockFormat::Bc7,
                levels,
            },
            None => Texels::Rgba8(parsed.decode(blob).ok()?.into_iter().flatten().collect()),
        };
        let texture = Self {
            label: label.to_string(),
            width: parsed.width,
            height: parsed.height,
            texels,
            mip_count: None,
        };
        Some((texture, form))
    }

    /// The disc's own blocks and mip chain, for a texture this can bind
    /// untouched.
    ///
    /// Four conditions, each of which the RGBA path handles instead:
    ///
    /// - a block-compressed format, which is 7,131 of the disc's 7,333 files;
    /// - a chain the file authors (`mip_levels > 1`), because a single-level
    ///   upload would leave the surface with no minification filter at all;
    /// - a tight pitch, since a declared one is a *base*-level row repeated
    ///   down the chain (see `oag_texture::gtf`) and only 5 files on the disc
    ///   declare one;
    /// - base dimensions on the block grid, which WebGPU requires of a
    ///   compressed texture and 15 files miss.
    #[must_use]
    pub fn gtf_blocks(texture: &oag_texture::gtf::Texture, blob: &[u8]) -> Option<Texels> {
        let format = BlockFormat::of_gtf(texture.format)?;
        if texture.cubemap || texture.mip_levels < 2 || texture.pitch != 0 {
            return None;
        }
        let (width, height) = texture.level_size(0);
        if width % 4 != 0 || height % 4 != 0 {
            return None;
        }
        let levels = (0..texture.mip_levels)
            .map(|level| blob.get(texture.level_range(level)).map(<[u8]>::to_vec))
            .collect::<Option<Vec<_>>>()?;
        Some(Texels::Blocks { format, levels })
    }

    /// One already-decoded RGBA8 texture.
    ///
    /// `mip_count` is the source asset's own declared depth, where the caller
    /// knows it - see [`Self::mip_count`]'s doc for what a `Some` changes and
    /// why every other call site still passes `None`.
    #[must_use]
    pub fn rgba8(
        label: String,
        width: u32,
        height: u32,
        rgba: Vec<u8>,
        mip_count: Option<u32>,
    ) -> Self {
        Self {
            label,
            width,
            height,
            texels: Texels::Rgba8(rgba),
            mip_count,
        }
    }

    /// One PSP `.vex` texture with the levels the disc authors, base first.
    #[must_use]
    pub fn chain(label: String, width: u32, height: u32, levels: Vec<Vec<u8>>) -> Self {
        Self {
            label,
            width,
            height,
            texels: Texels::Chain(levels),
            mip_count: None,
        }
    }

    /// Bytes of texel data this texture holds on the CPU.
    #[must_use]
    pub fn cpu_bytes(&self) -> u64 {
        match &self.texels {
            Texels::Rgba8(rgba) => rgba.len() as u64,
            Texels::Chain(levels) => levels.iter().map(|level| level.len() as u64).sum(),
            Texels::Blocks { levels, .. } => levels.iter().map(|level| level.len() as u64).sum(),
            Texels::Uploaded { .. } => 0,
        }
    }

    /// The base level as RGBA8, or `None` for one still in its blocks or
    /// already uploaded.
    ///
    /// For a consumer that reads texels on the CPU rather than binding them and
    /// knows it is holding a decoded one - the HUD's sight sheet is the only
    /// one, and every texture it reads is a PSP `.vex`, which never takes the
    /// block path. [`Self::to_rgba`] is the one that decodes.
    #[must_use]
    pub fn rgba(&self) -> Option<&[u8]> {
        match &self.texels {
            Texels::Rgba8(rgba) => Some(rgba),
            Texels::Chain(levels) => levels.first().map(Vec::as_slice),
            Texels::Blocks { .. } | Texels::Uploaded { .. } => None,
        }
    }

    /// The base level as RGBA8, decoding the disc's blocks if that is what it
    /// holds.
    ///
    /// For a consumer that has to read texels whatever form they arrived in -
    /// the `hd_*` probes under `crates/render/examples`, which measure a
    /// texture's own colours rather than binding it. Allocates only on the
    /// block path; `None` there for a layout
    /// [`oag_texture::gtf::decode_level`] refuses.
    #[must_use]
    pub fn to_rgba(&self) -> Option<std::borrow::Cow<'_, [u8]>> {
        match &self.texels {
            Texels::Rgba8(rgba) => Some(std::borrow::Cow::Borrowed(rgba)),
            Texels::Chain(levels) => levels.first().map(|l| std::borrow::Cow::Borrowed(&l[..])),
            Texels::Uploaded { .. } => None,
            Texels::Blocks { format, levels } => {
                let decoded = format.decode_level(levels.first()?, self.width, self.height)?;
                Some(std::borrow::Cow::Owned(
                    decoded.into_iter().flatten().collect(),
                ))
            }
        }
    }
}

impl ModelTexture {
    /// This texture's name and size with none of its texels.
    ///
    /// What a [`Model`](super::Model) keeps in a slot once the GPU holds the
    /// picture: `label` and the dimensions stay for anything that reports on the
    /// slot, and [`Self::cpu_bytes`] reads 0.
    #[must_use]
    fn released(&self) -> Self {
        Self {
            label: self.label.clone(),
            width: self.width,
            height: self.height,
            // An uploaded texture holds no texels to drop, and its view is the
            // one thing the uploader still needs from the stub.
            texels: match &self.texels {
                Texels::Uploaded { .. } => self.texels.clone(),
                _ => Texels::Rgba8(Vec::new()),
            },
            mip_count: self.mip_count,
        }
    }
}

impl super::Model {
    /// Drops the CPU copy of every texture, keeping each slot occupied.
    ///
    /// **Call it once the model has been uploaded** (`mesh_render::build`):
    /// after that the texels exist on the GPU and the model only needs the slot
    /// to say a texture was decoded there. Left alone, a model keeps its whole
    /// decoded set alive for the life of the race - Tech De Ra's was 2.3 GiB of
    /// BC7 blocks a second time, and 6.8 GiB before those were passed through.
    /// A slot stays `Some` so every count and `all(is_none)` check a caller
    /// makes still reads what it did; a texture another owner still holds
    /// (`Arc`) is freed when that owner lets go.
    pub fn release_texels(&mut self) {
        let release = |slots: &mut TextureSlots| {
            let mut stubs: std::collections::HashMap<usize, std::sync::Arc<ModelTexture>> =
                std::collections::HashMap::new();
            for slot in slots.iter_mut().flatten() {
                let key = std::sync::Arc::as_ptr(slot) as usize;
                *slot = stubs
                    .entry(key)
                    .or_insert_with(|| std::sync::Arc::new(slot.released()))
                    .clone();
            }
        };
        release(&mut self.textures);
        release(&mut self.lightmaps);
        release(&mut self.pad_masks);
        release(&mut self.wave_maps);
    }
}

#[cfg(test)]
mod tests;
