//! The texture a Zone race shows on billboard slots 1 to 7.
//!
//! HD's `Billboard_LoadModelAndBind` binds, in a Zone race, one shared texture to every
//! slot but 8: the Environment stage block's 512 x 512 target (`+0x32b4`), which the stage
//! pass draws into (`docs/ghidra/functions/ps3-hdfury-eu/billboards.md`, "what a Zone
//! race's shared `billboard` texture is"). It is an equaliser: a 32 x 32 grid of 16-pixel
//! cells, each cell a ground colour and a 2-pixel black line on its right and bottom edge,
//! with sixteen bars two cells wide driven by the held band levels the `zoneTexVis` lookup
//! also reads.
//!
//! [`Equaliser::step`] is `Environment_UpdateStageBlend`'s per-frame draw into that target
//! (`docs/ghidra/functions/ps3-hdfury-eu/zone-equaliser.md`): a fade quad over the whole
//! target, then per band a body quad and a cap quad from the floor up to the held level,
//! then the grid. Only style 0 (stages 0 to 2) is read; the other eight styles draw the
//! grid alone.

#[cfg(feature = "view")]
use crate::drawable::Drawable;

/// The target's side in pixels.
pub const SIDE: u32 = 512;
/// One cell's side in pixels.
const CELL: u32 = 16;
/// The line on a cell's right and bottom edge, in pixels.
const LINE: u32 = 2;
/// A cell's ground, as read: `A8R8G8B8` `0x60010101`, the fixed point of the fade below.
/// Drawn opaque, the stage target's own alpha not being a coverage on this surface.
const GROUND: [u8; 4] = [1, 1, 1, 255];
/// The line, as read: `0xff000000`.
const LINE_COLOUR: [u8; 4] = [0, 0, 0, 255];
/// The bands, one bar each.
pub const BANDS: usize = 16;
/// The fade quad's alpha (`FUN_005e9e50(.., 0x60, 0x60, 0x60)` under blend state 4,
/// `SRC_ALPHA, ONE_MINUS_SRC_ALPHA` on colour): each frame keeps `1 - 0x60/255` of a texel.
const FADE_ALPHA: u32 = 0x60;
/// A bar's column pitch, left edge in its column and width: `i * 32.0 + 1.0`, `+ 30.0`
/// (`-0x5a38(r2)` 32.0, `-0x5a50(r2)` 1.0, `-0x5a34(r2)` 30.0).
const BAR_PITCH: u32 = 32;
const BAR_LEFT: u32 = 1;
const BAR_WIDTH: u32 = 30;
/// A bar's floor and full height: the body runs from `510 - 508 * held` down to `510`
/// (`-0x5a3c(r2)` 510.0, `-0x5a40(r2)` -508.0).
const FLOOR: f32 = 510.0;
const SPAN: f32 = 508.0;
/// The cap's height below the bar's top: `526 - 508 * held` (`-0x5a6c(r2)` 526.0).
const CAP: f32 = 16.0;

/// The equaliser style a stage draws: the word after the two stage entries (`+0x74`),
/// set every frame from entry 0's requested stage (`0x003db968`-`0x003dba00`). `None`
/// past stage 14, where no branch writes it.
#[must_use]
pub fn style(requested_stage: u32) -> Option<u8> {
    const STYLES: [u8; 15] = [0, 0, 0, 7, 7, 8, 8, 5, 1, 4, 3, 7, 5, 5, 5];
    STYLES.get(requested_stage as usize).copied()
}

/// What this texture is, for the load report.
pub const PROVENANCE: &str = "equaliser target read off the EBOOT and RPCS3 (Vineta K): grid, \
     fade and style 0's sixteen bars from the held bands, confidence 80; styles 1 to 8 \
     (stages 3 to 14) unread, drawn as the bare grid";

/// The target's per-frame state. A bar's column is uniform across its 30 pixels, so a
/// texel's history is one of [`BANDS`] stripes by 512 rows; everything else is ground
/// or line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Equaliser {
    /// `BANDS * SIDE` RGB texels, band-major.
    stripes: Vec<[u8; 3]>,
}

impl Default for Equaliser {
    fn default() -> Self {
        Self {
            stripes: vec![[GROUND[0], GROUND[1], GROUND[2]]; BANDS * SIDE as usize],
        }
    }
}

impl Equaliser {
    /// One frame of the stage pass. `held` is the sixteen held band levels
    /// (`oag_mesh::mesh_render::zone::Hold`), `tint` the stage's cross-faded
    /// `EQ colour tint` as RGBA, `style` [`style`] of the requested stage. Returns whether
    /// a texel changed.
    ///
    /// The body is the tint halved, the cap the whole tint. A style other than 0 is
    /// unread and puts the target back to the bare grid. A missing tint draws no bar and
    /// the fade runs on.
    pub fn step(&mut self, held: &[f32], tint: Option<[u8; 4]>, style: Option<u8>) -> bool {
        let before = self.stripes.clone();
        if style == Some(0) {
            for texel in &mut self.stripes {
                *texel = texel.map(fade);
            }
            if let Some([r, g, b, _]) = tint {
                let body = [r >> 1, g >> 1, b >> 1];
                for (band, &level) in held.iter().take(BANDS).enumerate() {
                    let top = FLOOR - SPAN * level;
                    self.fill(band, top, FLOOR, body);
                    self.fill(band, top, top + CAP, [r, g, b]);
                }
            }
        } else {
            *self = Self::default();
        }
        self.stripes != before
    }

    /// Rows `top..bottom` of `band`'s stripe, by pixel centre.
    fn fill(&mut self, band: usize, top: f32, bottom: f32, colour: [u8; 3]) {
        let stripe = &mut self.stripes[band * SIDE as usize..][..SIDE as usize];
        for (y, texel) in stripe.iter_mut().enumerate() {
            #[expect(clippy::cast_precision_loss, reason = "y < 512")]
            let centre = y as f32 + 0.5;
            if top <= centre && centre < bottom {
                *texel = colour;
            }
        }
    }

    /// The target, `SIDE * SIDE` RGBA8 texels: the stripes under the grid.
    #[must_use]
    pub fn texels(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity((SIDE * SIDE * 4) as usize);
        for y in 0..SIDE {
            for x in 0..SIDE {
                let line = x % CELL >= CELL - LINE || y % CELL >= CELL - LINE;
                let texel = if line {
                    LINE_COLOUR
                } else if (BAR_LEFT..BAR_LEFT + BAR_WIDTH).contains(&(x % BAR_PITCH)) {
                    let [r, g, b] = self.stripes[(x / BAR_PITCH * SIDE + y) as usize];
                    [r, g, b, 255]
                } else {
                    GROUND
                };
                out.extend_from_slice(&texel);
            }
        }
        out
    }
}

/// One channel under the fade: black at alpha `0x60` over it. Rounded to nearest, which
/// holds the race-start ground of 1 at 1: **chosen**, the original's rounding is not read and
/// its ground reads 0 after a minute of silence (`zone-equaliser.md`, "Live reads").
fn fade(channel: u8) -> u8 {
    let out = (u32::from(channel) * (255 - FADE_ALPHA) + 127) / 255;
    #[expect(clippy::cast_possible_truncation, reason = "at most 255")]
    let out = out as u8;
    out
}

/// The grid with no bar lit: the target in silence.
#[must_use]
pub fn texels() -> Vec<u8> {
    Equaliser::default().texels()
}

/// The target on the GPU and the state behind it, stepped once a frame.
#[cfg(feature = "view")]
#[derive(Debug)]
pub struct Live {
    texture: wgpu::Texture,
    equaliser: Equaliser,
    /// The title's `zone_equaliser_bars`: without it every frame is the bare grid.
    bars: bool,
}

#[cfg(feature = "view")]
impl Live {
    /// [`Equaliser::step`], uploading the target when a texel changed.
    pub fn step(
        &mut self,
        queue: &dyn oag_gpu::writes::Writes,
        held: &[f32],
        tint: Option<[u8; 4]>,
        style: Option<u8>,
    ) {
        let style = style.filter(|_| self.bars);
        if self.equaliser.step(held, tint, style) {
            upload(queue, &self.texture, &self.equaliser.texels());
        }
    }
}

#[cfg(feature = "view")]
fn upload(queue: &dyn oag_gpu::writes::Writes, texture: &wgpu::Texture, texels: &[u8]) {
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        texels,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(SIDE * 4),
            rows_per_image: Some(SIDE),
        },
        wgpu::Extent3d {
            width: SIDE,
            height: SIDE,
            depth_or_array_layers: 1,
        },
    );
}

#[cfg(feature = "view")]
/// Points every placeholder material of `track` whose slot is in `slots` at the target,
/// the bare grid until the first [`Live::step`], and for good unless `bars`. `None` when
/// no slot shows it.
pub(super) fn bind_into(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    track: &mut Drawable,
    placeholders: &[(usize, u32)],
    slots: &[u32],
    bars: bool,
) -> Option<Live> {
    if slots.is_empty() {
        return None;
    }
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("zone equaliser"),
        size: wgpu::Extent3d {
            width: SIDE,
            height: SIDE,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
        texture_binding_view_dimension: None,
    });
    let equaliser = Equaliser::default();
    upload(queue, &texture, &equaliser.texels());
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    for &(texture_slot, number) in placeholders {
        if slots.contains(&number) {
            track.set_albedo(device, queue, texture_slot, &view, true);
        }
    }
    Some(Live {
        texture,
        equaliser,
        bars,
    })
}

#[cfg(test)]
mod tests;
