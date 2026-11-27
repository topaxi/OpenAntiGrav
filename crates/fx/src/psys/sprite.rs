//! An emitter's own sprite, and the one sheet every loaded sprite packs into.
//!
//! # What the original draws a particle with
//!
//! `ParticleSystem_DrawParticle` binds the emitter's texture block through
//! `FUN_08928b10` before every quad: the sprite's level-0-and-down mip chain
//! off the loader-fixed pixel pointer, its 256-entry RGBA8888 CLUT off the
//! palette pointer. The texture function it draws under is the one
//! `Gfx_Init` sets once for the whole frame, `GU_TFX_MODULATE` with
//! `GU_TCC_RGBA` and colour doubling off (`docs/ghidra/functions/psp-pulse-usa/mesh-draw.md`,
//! confidence 90) - nothing on the particle path sets another. So a
//! particle's fragment is **the sprite texel times the particle's colour**,
//! alpha included, and the emitter's 256-entry colour table is authored on
//! that understanding: `WO_QUAKE`'s `fireballs` walk from near-white to
//! orange because their sprite is already an orange fire ring. Drawn over a
//! white procedural disc instead, the same table is a white blowout.
//!
//! # Where a sprite comes from
//!
//! PSP: [`oag_pob::texture`] reads the sprite a `.pob` embeds right after an
//! emitter's record. A PS2 `.pob` embeds none but names the sprite by path
//! (`Data\Psys\Tex\<stem>.tga`), which resolves on the PS2 disc as the `.pct`
//! entry ([`Sprite::from_ps2`]). HD's `.gtf` sprites load for the effects its
//! title lists; anything unread keeps the procedural profile in `psys.wesl`.
//!
//! # The atlas
//!
//! `+0x9a0` authors a grid of frames. `FUN_088f58a4` (the instance init)
//! copies it to the instance and multiplies it out as the frame count; under
//! [`oag_pob::flags::RANDOM_ATLAS_FRAME`] `ParticleSystem_InitParticle`
//! draws each particle's frame from `Psys_RandIntRange(0, count - 1)`, and
//! otherwise starts it at frame 0. `ParticleSystem_DrawParticle` then picks
//! the cell with `Gu_TexScale(1/columns, 1/rows)` and
//! `Gu_TexOffset(column, row)` from `frame % columns` and `frame / columns`.
//! The frame then advances over the particle's life on Pulse's PSP source -
//! see [`super::frames`].
//!
//! # The sheet
//!
//! Every sprite of every loaded effect is packed into one
//! [`SHEET_SIZE`]-square RGBA8 texture, so the pipeline keeps one bind group
//! and the two batched draws it always had. The packing is this project's
//! ([`packing`], sprites [`PAD`] texels apart), not anything the original does;
//! a sprite that does not fit is left unplaced and its emitter keeps the procedural
//! profile, which [`Sheet::place`] reports by returning `None`.
//!
//! A sprite that carries its file's mip chain ([`Sprite::chain`]) is laid out for it and
//! the sheet holds lower levels too ([`chain`]).

mod chain;
mod packing;

pub use chain::{ATLAS_LEVELS, GUTTER};

use std::sync::Arc;

use oag_pob::{self as pob, ParticleSystem};

/// The sheet's width and height in texels.
///
/// Chosen, not measured. 1024 held the whole PSP corpus (76 sprites of at
/// most 128x64) but not Omega's: its explosion effects author 1024x1024 and
/// 512x512 sprites, and at 1024 and 2048 some did not fit and drew the
/// procedural white disc instead. A Tech De Ra race needs more than 2048
/// (3 emitters unplaced) and fits all of its effects at 4096.
pub const SHEET_SIZE: u32 = 4096;

/// Empty texels between packed sprites, so bilinear filtering at one
/// sprite's edge never reads its neighbour.
pub const PAD: u32 = 2;

/// One decoded sprite: level 0, straight RGBA8888, row-major from the top, and its file's
/// lower levels where they are read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sprite {
    /// Texels across.
    pub width: u16,
    /// Texels down.
    pub height: u16,
    /// `width * height * 4` bytes.
    pub rgba: Arc<[u8]>,
    /// The levels below the base, each half the one above, largest first: `Some` for a
    /// `.gxt`, whose sampler reads down them (empty where the file authors one level),
    /// `None` where the base alone is sampled.
    pub chain: Option<Vec<Arc<[u8]>>>,
}

impl Sprite {
    /// The sprite a sprite template (`record`, an entry of
    /// `initial_particles`) embeds in its own record - what the original
    /// binds for a template particle, not its parent's.
    #[must_use]
    pub fn from_template(
        system: &ParticleSystem,
        data: &[u8],
        record: &pob::Emitter,
    ) -> Option<Self> {
        if system.order != oag_bytes::ByteOrder::Little {
            return None;
        }
        let texture = system.template_texture(data, record)?;
        Some(Self {
            width: texture.width,
            height: texture.height,
            rgba: texture.rgba8().into(),
            chain: None,
        })
    }

    /// A Vita `.gxt`'s one texture with its mip chain - the sprite Wipeout 2048 ships as
    /// a separate file beside its `.pob`. `None` for a blob that is not a
    /// `.gxt`, names a format this build does not decode, or is larger than a
    /// `u16` can say.
    #[must_use]
    pub fn from_gxt(blob: &[u8]) -> Option<Self> {
        let mut sprite = Self::from_container(oag_texture::decode::Container::Gxt, blob)?;
        let gxt = oag_texture::gxt::Gxt::parse(blob).ok()?;
        let levels = gxt.only()?.lower_levels_rgba(blob);
        sprite.chain = Some(levels.into_iter().map(Arc::from).collect());
        Some(sprite)
    }

    /// `blob` decoded as `container` by `oag_texture::decode`, base level, rows turned
    /// top-down where the file stores them bottom-up (`.gtf`); `None` when it will not
    /// decode or is larger than a `u16` can say.
    ///
    /// The original puts `v = 0`, a `.gtf`'s first stored row, on the bottom edge of a
    /// particle's quad; a billboard here puts row 0 on the top edge
    /// (`docs/rendering/particle-fill.md`, "HD's particle sprites are bottom-up").
    fn from_container(container: oag_texture::decode::Container, blob: &[u8]) -> Option<Self> {
        let picture = oag_texture::decode::decode_as(container, blob).ok()?;
        let width = u16::try_from(picture.width).ok()?;
        let rgba = if picture.bottom_up {
            let row = usize::from(width).max(1) * 4;
            picture.rgba.rchunks_exact(row).flatten().copied().collect()
        } else {
            picture.rgba
        };
        Some(Self {
            width,
            height: u16::try_from(picture.height).ok()?,
            rgba: rgba.into(),
            chain: None,
        })
    }

    /// An `OAGMASK` text mask (this project's own art, `oag_texture::mask`) as
    /// a white sprite whose alpha is the mask. `None` for a blob that is not
    /// a mask or does not decode.
    #[must_use]
    pub fn from_mask(blob: &[u8]) -> Option<Self> {
        let mask = oag_texture::mask::parse(blob).ok()?;
        Some(Self {
            width: mask.width,
            height: mask.height,
            rgba: mask.rgba.into(),
            chain: None,
        })
    }

    /// A PS2 `.pct` texture, the sprite a PS2 `.pob` names by path and embeds
    /// none of. `None` for a blob [`oag_texture::ps2_texture`] does not read.
    #[must_use]
    pub fn from_ps2(blob: &[u8]) -> Option<Self> {
        Self::from_container(oag_texture::decode::Container::Pct, blob)
    }

    /// A PS3 `.gtf`'s one texture, level 0 - the sprite Wipeout HD ships as a
    /// separate `/data/psys/tex/` entry. `None` for a blob that is not a `.gtf`,
    /// is not a single texture, or names a format [`oag_texture::gtf`] does not
    /// decode.
    #[must_use]
    pub fn from_gtf(blob: &[u8]) -> Option<Self> {
        Self::from_container(oag_texture::decode::Container::Gtf, blob)
    }

    /// A PS4 `.gnf`'s base level - the sprite Wipeout: Omega Collection ships
    /// beside its `.pob`, as 2048 ships `.gxt`. `None` for a blob that is not a
    /// `.gnf` or whose format or tiling [`oag_texture::gnf`] does not decode.
    #[must_use]
    pub fn from_gnf(blob: &[u8]) -> Option<Self> {
        Self::from_container(oag_texture::decode::Container::Gnf, blob)
    }

    /// `record`'s embedded sprite, else the one `external` makes of its authored
    /// texture path.
    pub(super) fn of_record(
        system: &ParticleSystem,
        data: &[u8],
        record: &pob::Emitter,
        external: &mut dyn FnMut(&str) -> Option<Self>,
    ) -> Option<Self> {
        Self::from_pob(system, data, record).or_else(|| {
            let path = system.texture_path(data, record)?;
            external(path)
        })
    }

    /// The sprite `record` embeds, if it embeds one and the file is a PSP
    /// one - see the module documentation for why the byte order is the
    /// test.
    #[must_use]
    pub fn from_pob(system: &ParticleSystem, data: &[u8], record: &pob::Emitter) -> Option<Self> {
        if system.order != oag_bytes::ByteOrder::Little {
            return None;
        }
        let texture = system.embedded_texture(data, record)?;
        Some(Self {
            width: texture.width,
            height: texture.height,
            rgba: texture.rgba8().into(),
            chain: None,
        })
    }
}

/// How an emitter's sprite divides into frames.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Atlas {
    /// Cells across, at least 1.
    pub columns: u16,
    /// Cells down, at least 1.
    pub rows: u16,
    /// Each particle takes a random cell at spawn -
    /// [`oag_pob::flags::RANDOM_ATLAS_FRAME`]; otherwise cell 0.
    pub random_frame: bool,
}

impl Atlas {
    /// The whole sprite as its one frame.
    pub const SINGLE: Self = Self {
        columns: 1,
        rows: 1,
        random_frame: false,
    };

    /// `record`'s own grid and frame choice.
    #[must_use]
    pub fn of(record: &pob::Emitter) -> Self {
        Self {
            columns: record.atlas_grid.0.max(1),
            rows: record.atlas_grid.1.max(1),
            random_frame: record.flags & pob::flags::RANDOM_ATLAS_FRAME != 0,
        }
    }

    /// Frames in the grid - what `FUN_088f58a4` multiplies out.
    #[must_use]
    pub fn frames(self) -> u16 {
        self.columns.saturating_mul(self.rows)
    }

    /// Frame `frame`'s cell inside `rect`, a sprite's placement on the
    /// sheet, as `[u0, v0, u1, v1]` - pulled in by half a texel on every
    /// side so bilinear filtering never reads the neighbouring cell.
    #[must_use]
    pub fn cell(self, rect: [f32; 4], frame: u16) -> [f32; 4] {
        let frame = frame % self.frames().max(1);
        let (column, row) = (frame % self.columns, frame / self.columns);
        let width = (rect[2] - rect[0]) / f32::from(self.columns);
        let height = (rect[3] - rect[1]) / f32::from(self.rows);
        let u0 = rect[0] + width * f32::from(column);
        let v0 = rect[1] + height * f32::from(row);
        let inset = 0.5 / SHEET_SIZE as f32;
        [
            u0 + inset,
            v0 + inset,
            u0 + width - inset,
            v0 + height - inset,
        ]
    }
}

impl super::EmitterSpec {
    /// The frame a particle spawns on: `Psys_RandIntRange` over the grid under the
    /// random-frame flag, or over [`oag_pob::FrameRange::spawn`] where the emitter plays one;
    /// without the flag, that range's first frame. Drawn only when a sprite is placed, so an
    /// effect drawing the procedural profile consumes the generator exactly as it did before
    /// sprites.
    pub(super) fn random_frame(&self, rng: &mut oag_core::Rng) -> u16 {
        if self.sheet_rect.is_none() {
            return 0;
        }
        let spawn = self.frame_range.map_or((0, 0), |range| range.spawn);
        if !self.atlas.random_frame {
            return u16::from(spawn.0);
        }
        if self.atlas.frames() <= 1 {
            return 0;
        }
        let range = if spawn == (0, 0) {
            (0, u32::from(self.atlas.frames()) - 1)
        } else {
            (u32::from(spawn.0), u32::from(spawn.1))
        };
        super::random_range(rng, range) as u16
    }
}

/// Points a quad's `0..=1` texture coordinates at `cell` on the sheet and
/// marks it as sampled.
///
/// The mark rides the vertex's `normal.x`, which a particle never used -
/// they are emissive and never lit - so `psys.wesl` can tell a sprite from
/// the procedural profile in one pipeline: `1.0` samples the sheet, `0.0`
/// (what `psys::quad` writes) keeps the falloff.
pub fn map_to_cell(corners: &mut [oag_mesh::mesh::GpuVertex], cell: [f32; 4]) {
    for corner in corners {
        let [u, v] = corner.texcoord;
        corner.texcoord = [
            cell[0] + (cell[2] - cell[0]) * u,
            cell[1] + (cell[3] - cell[1]) * v,
        ];
        corner.normal = [1.0, 0.0, 0.0];
    }
}

/// Every placed sprite's texels on one [`SHEET_SIZE`]-square RGBA8 image.
#[derive(Debug, Clone, Default)]
pub struct Sheet {
    /// The rows placed so far, `SHEET_SIZE * 4` bytes each, top row first:
    /// grown as sprites reach down rather than allocated whole, and shared
    /// rather than copied when a [`super::Library`] is cloned. A row below the
    /// last one placed is transparent, as the uploaded texture is before a
    /// sync.
    pixels: Option<Arc<Vec<u8>>>,
    /// Each distinct sprite once, with where it went.
    placed: Vec<(Sprite, [f32; 4])>,
    /// Where the next sprite may go.
    free: packing::Free,
    /// The levels below [`Self::pixels`], where a sprite carries a chain.
    lower: chain::Lower,
    /// Bumped on every change to [`Self::pixels`], so an uploader can tell
    /// its copy is stale.
    generation: u64,
}

impl Sheet {
    /// Places `sprite` - or finds it, when an identical one is already
    /// there - and returns its rectangle as `[u0, v0, u1, v1]` in `0..=1`
    /// sheet coordinates, on its outer texel edges.
    ///
    /// `None` when it does not fit; the caller draws that emitter with the
    /// procedural profile instead.
    pub fn place(&mut self, sprite: &Sprite) -> Option<[f32; 4]> {
        if let Some((_, rect)) = self.placed.iter().find(|(placed, _)| placed == sprite) {
            return Some(*rect);
        }
        let (w, h) = (u32::from(sprite.width), u32::from(sprite.height));
        // A chained sprite sits `GUTTER` inside a cell on the `GUTTER` grid.
        let gutter = if sprite.chain.is_some() { GUTTER } else { 0 };
        let (x, y) = if gutter == 0 {
            self.free.take(w, h)?
        } else {
            let cell = |side: u32| (side + 2 * gutter).next_multiple_of(gutter) - PAD;
            let (x, y) = self.free.take_aligned(cell(w), cell(h), gutter)?;
            (x + gutter, y + gutter)
        };
        let pixels = Arc::make_mut(self.pixels.get_or_insert_with(Arc::default));
        chain::blit(pixels, SHEET_SIZE, &sprite.rgba, (w, h), (x, y), gutter);
        if let Some(levels) = &sprite.chain {
            self.lower.write(sprite, levels, (x, y));
        }
        let size = SHEET_SIZE as f32;
        let rect = [
            x as f32 / size,
            y as f32 / size,
            (x + w) as f32 / size,
            (y + h) as f32 / size,
        ];
        self.placed.push((sprite.clone(), rect));
        self.generation += 1;
        Some(rect)
    }

    /// The sheet's texels down to the last row any sprite reaches
    /// ([`Self::rows`] rows of `SHEET_SIZE * 4` bytes), or `None` while nothing
    /// is placed. Every row below is transparent.
    #[must_use]
    pub fn pixels(&self) -> Option<&[u8]> {
        self.pixels.as_deref().map(Vec::as_slice)
    }

    /// The rows placed so far on `level` of a chained sheet (1 up to [`ATLAS_LEVELS`] - 1),
    /// [`SHEET_SIZE`] >> `level` texels a row; `None` on a sheet whose sprites carry none.
    #[must_use]
    pub fn lower_pixels(&self, level: u32) -> Option<&[u8]> {
        self.lower.level(level)
    }

    /// How many rows [`Self::pixels`] holds, 0 while nothing is placed.
    #[must_use]
    pub fn rows(&self) -> u32 {
        self.pixels.as_ref().map_or(0, |pixels| {
            (pixels.len() / (SHEET_SIZE as usize * 4)) as u32
        })
    }

    /// Changes whenever [`Self::pixels`] does.
    #[must_use]
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// How many distinct sprites are placed.
    #[must_use]
    pub fn len(&self) -> usize {
        self.placed.len()
    }

    /// Whether nothing is placed.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.placed.is_empty()
    }
}

#[cfg(test)]
mod tests;
