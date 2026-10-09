//! The box HD draws behind a menu entry: `Block_Item.cpp`, reimplemented.
//!
//! Every strip tab, settings row and pause-menu entry on Wipeout HD sits on
//! one of these, and everything a capture shows about it - a lighter
//! three-unit border, a darker inside, a chamfered corner or two, the
//! selected one being wider - is one widget class in `EBOOT.elf` plus a
//! single 64x64 texture. This module is that class's render method, function
//! by function; the addresses are on
//! [menu-blocks.md](../../../../docs/ghidra/functions/ps3-hdfury-eu/menu-blocks.md).
//!
//! # What is the disc's, what is the executable's, and what is ours
//!
//! The disc's: the texture, [`BlockArt::frame`], decoded off the served
//! archives by the caller, and the fill's alpha, [`BlockArt::fill_alpha`],
//! which is one texel of it - sampled by the caller off the decoded picture,
//! not written down here.
//!
//! The executable's: every number in this file. They are the constants of
//! `Block_Render`, `Block_DrawFill`, `Block_DrawTopBand`, `Block_DrawCorner`
//! and the two edge routines, each cited beside its constant. That is a
//! different provenance from the *measured* tab figures this module replaced
//! in `skin.rs` on 2026-09-14: those were read off captures at ruler accuracy,
//! these are read off the code that produced the captures. Where the two
//! disagree by a unit - a 62-tall tab from a 64-tall block - the page above
//! explains why, and the captures are what says the reading is right.
//!
//! Ours: nothing. A block with no art draws nothing, and says nothing; that
//! is the caller's `None`.
//!
//! # The two-pass fill, because it is the whole HD-versus-Fury difference
//!
//! `Block_Render` draws the fill **twice** over the same rectangle. The first
//! pass always samples a swatch texel whose alpha is `110/255`; the second
//! samples that same swatch on the Fury style and an opaque texel of the
//! outline on the HD style. So a Fury block's inside is the colour at
//! `1 - (1 - 0.431)^2 = 0.676` over the page and an HD block's is the colour
//! solid - which is what the captures measured (`102/150` on the black page,
//! `#646464` to the byte on the white one) and what read as two different
//! rules until the code was opened. See [`draw`].

use crate::frontend::{Draw, Placed};

/// The decoded art a block draws with, and which style it draws in.
///
/// Built by whoever decoded the sprite sheet - `oag_game::boot`, which is
/// where the texture's pixels are in hand to sample the swatch - and handed
/// down through [`super::Frame::blocks`]. `None` there means the title has
/// no blocks, or its texture did not decode; either way nothing draws a box,
/// the same rule [`super::read_frame`] applies to a frame mark whose texture
/// is missing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BlockArt {
    /// Where the nine-patch sits in the sprite sheet. `Block_Construct`'s
    /// `Data\FE\Images\file2.gtf`: 64x64, white, its alpha a three-texel
    /// outline of a box with a chamfered top-left corner and a stepped
    /// bottom-right one, plus a column of swatch texels.
    pub frame: Placed,
    /// The alpha of the swatch texel `Block_DrawFill`'s first pass samples,
    /// `0.0..=1.0`. `110/255` on the shipped texture; sampled rather than
    /// written so it stays the disc's. [`FILL_SWATCH_UV`].
    pub fill_alpha: f32,
    /// The alpha of the outline texel the second pass samples on the HD
    /// style, `0.0..=1.0`. `255/255` on the shipped texture, and sampled
    /// for the same reason. [`SOLID_SWATCH_UV`].
    pub solid_alpha: f32,
    /// Where the strip's underline mark sits in the sheet, when it decoded:
    /// `HorizMenu_Construct`'s `cursor.gtf`, a 21x8 bar at texel `(1, 1)` of
    /// 32x16.
    pub cursor: Option<Placed>,
    /// Where the settings rows' step arrow sits in the sheet, when it
    /// decoded: `List_Construct`'s `HD_options_arrow.gtf`, a right-pointing
    /// triangle in 32x32.
    pub arrow: Option<Placed>,
    /// `FrontEnd_IsFuryStyle`: chamfer the top-left corner and keep the
    /// second fill pass translucent. Its writer is unread, so the caller
    /// decides it off the served archive's own page colour - see
    /// `oag_game::boot`.
    pub fury: bool,
}

/// One block to draw: position and size in the drawing grid, the colour the
/// executable would have set with `Block_SetColour`, and the corner style
/// `Block_SetLandingStyle` chooses.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Block {
    /// Top-left corner of the block's own rectangle. The visible box is one
    /// unit inside it on every side.
    pub x: f32,
    /// See [`Self::x`].
    pub y: f32,
    /// The block's width - the animated one, `+0xac`/`+0x148`.
    pub width: f32,
    /// The block's height, `+0xb0`.
    pub height: f32,
    /// RGBA, opaque; the fill's alpha is the art's, not this.
    pub color: [f32; 4],
    /// `+0x11c`: the top-right corner is a cut followed by a flat landing
    /// (a strip tab, a settings row's value box) rather than a plain cut
    /// (a settings row's label box).
    pub landing: bool,
}

/// Height of the top band `Block_DrawTopBand` draws, and so of both its
/// 45-degree cuts: `10.0`, TOC constant `0x008ac590`.
const BAND_HEIGHT: f32 = 10.0;
/// How far the fill is inset from the block's left and top edges: `2.0`,
/// `0x008ac58c`. Its width is inset by twice this (`4.0`, `0x008ac588`).
const FILL_INSET: f32 = 2.0;
/// How much shorter the top band is than the fill on a landing-style block:
/// `17.0`, `0x008ac594`. The landing is this plus the band's own one-unit
/// overhang, which is why a capture measures it at `17.5`.
const LANDING: f32 = 17.0;
/// The top band starts one unit left of the fill and, on a plain block, runs
/// one unit past it: the `+1.0` at `0x0018af60` and the `x - 1.0` at
/// `0x0018a654`.
const BAND_OVERHANG: f32 = 1.0;
/// A border corner piece is this wide, `40.0` at `0x008ac55c`.
const CORNER_WIDTH: f32 = 40.0;
/// A border corner piece is this tall, `18.0` at `0x008ac578`.
const CORNER_HEIGHT: f32 = 18.0;

/// The nine-patch's own size. Every UV below is one of `Block_DrawCorner`'s
/// sixty-fourths turned back into texels of a texture this wide and tall.
const PATCH: f32 = 64.0;

/// A corner piece's UV rectangle in the nine-patch, in the executable's own
/// texels of the file, as `[x, y, width, height]` with a negative width or
/// height meaning the piece is drawn mirrored on that axis. The six cases
/// of `Block_DrawCorner`'s jump table at `0x00189f48`, in its order;
/// [`patch_uv`] turns the file's rows into the sheet's.
///
/// `24 = 0.375 * 64`, `40 = 0.625 * 64`, `18 = 0.281 * 64`, `46 = 0.719 * 64`.
/// In the picture's own orientation the file holds a box with its step at
/// the top-right and its chamfer at the bottom-left, so:
const CORNER_UV: [[f32; 4]; 6] = [
    // 0: bottom-left - the picture's square bottom-right corner, mirrored.
    [64.0, 18.0, -40.0, -18.0],
    // 1: bottom-right - the same, as drawn.
    [24.0, 18.0, 40.0, -18.0],
    // 2: top-left on the Fury style - the picture's bottom-left chamfer,
    // turned over.
    [0.0, 0.0, 40.0, 18.0],
    // 3: top-right without a landing - the same, turned over and mirrored.
    [40.0, 0.0, -40.0, 18.0],
    // 4: top-right with a landing - the picture's own stepped corner.
    [24.0, 64.0, 40.0, -18.0],
    // 5: top-left on the HD style - the picture's square bottom-right
    // corner mirrored, as case 0 is but without the vertical flip.
    [64.0, 0.0, -40.0, 18.0],
];

/// Which of the [`CORNER_UV`] cases each block corner draws.
const BOTTOM_LEFT: usize = 0;
const BOTTOM_RIGHT: usize = 1;
const TOP_LEFT_FURY: usize = 2;
const TOP_RIGHT_PLAIN: usize = 3;
const TOP_RIGHT_LANDING: usize = 4;
const TOP_LEFT_HD: usize = 5;

/// The one texel row a vertical edge stretches: `v` `0.469..0.484`, file
/// rows `30..31`, of the left line's 40-texel-wide band.
/// `Block_DrawVerticalEdge`.
const VERTICAL_EDGE_UV: [f32; 4] = [0.0, 30.0, 40.0, 1.0];
/// The one texel column a horizontal edge stretches: `u` `0.219..0.234`,
/// columns `14..15`, of the picture's bottom line's 18-texel-tall band -
/// which, turned over, is the top edge. `Block_DrawHorizontalEdge`.
const HORIZONTAL_EDGE_UV: [f32; 4] = [14.0, 0.0, 1.0, 18.0];

/// The swatch texel `Block_DrawFill`'s first pass samples, as a fraction of
/// the nine-patch **in the sheet's own top-down rows**: `u = 0x3d900000 =
/// 0.0703`, and `v = 1 - 0x3f6a0000 = 1 - 0.914 = 0.086` - texel
/// `(4.5, 5.5)` of the sheet, `(4.5, 58.5)` of the file. See
/// [`patch_uv`] for why the one is the other. The caller samples the decoded
/// texture here for [`BlockArt::fill_alpha`]; `pub` so the sampling site and
/// the reading cannot drift apart.
pub const FILL_SWATCH_UV: (f32, f32) = (0.070_312_5, 1.0 - 0.914_062_5);

/// The texel the second pass samples on the HD style, `u = 0x3d000000 =
/// 0.03125` at the same `v`: texel `(2.5, 5.5)` of the sheet, two columns
/// left of the swatch, inside the outline. [`BlockArt::solid_alpha`].
pub const SOLID_SWATCH_UV: (f32, f32) = (0.031_25, FILL_SWATCH_UV.1);

/// Draws `block` as `Block_Render` would, appending to `out`.
///
/// `scale` is what multiplies the executable's own units - authored 1080p
/// units on HD - into the drawing grid, `(1.0, 1.0)` when the two agree.
/// `block`'s own fields are already in the drawing grid.
///
/// The order is the original's: fill, fill again, then the eight border
/// pieces over both. Nothing is drawn for a block with no width, which is
/// also the original's own `w > 0` guard on the border.
pub fn draw(block: &Block, art: &BlockArt, scale: (f32, f32), out: &mut Vec<Draw>) {
    if block.width <= 0.0 || block.height <= 0.0 {
        return;
    }
    draw_fill(block, art, scale, out);
    draw_border(block, art, scale, out);
}

/// [`draw`]'s two fill passes alone, with no border - for a caller that
/// composes one block's inside out of more than one colour, as HD's `Team
/// Selection` stat bars do (`oag_ui_screens::picker::hd`).
pub fn draw_fill(block: &Block, art: &BlockArt, scale: (f32, f32), out: &mut Vec<Draw>) {
    if block.width <= 0.0 || block.height <= 0.0 {
        return;
    }
    let color = block.color;
    // Pass one: the swatch's own alpha, whichever style.
    fill(block, art, scale, alpha(color, art.fill_alpha), out);
    // Pass two: the swatch again on Fury, the outline's texel on HD.
    let second = if art.fury {
        art.fill_alpha
    } else {
        art.solid_alpha
    };
    fill(block, art, scale, alpha(color, second), out);
}

/// [`draw`]'s border alone: four corner pieces and four edges.
pub fn draw_border(block: &Block, art: &BlockArt, scale: (f32, f32), out: &mut Vec<Draw>) {
    if block.width <= 0.0 || block.height <= 0.0 {
        return;
    }
    let (sx, sy) = scale;
    let color = block.color;

    // The border: four corner pieces and four edges, all cut from the
    // nine-patch and tinted the block's colour. The texture is white, so
    // the colour is the tint and the alpha is the outline's.
    let corner_w = CORNER_WIDTH * sx;
    let corner_h = CORNER_HEIGHT * sy;
    let right = block.x + block.width - corner_w;
    let bottom = block.y + block.height - corner_h;
    let top_left = if art.fury { TOP_LEFT_FURY } else { TOP_LEFT_HD };
    let top_right = if block.landing {
        TOP_RIGHT_LANDING
    } else {
        TOP_RIGHT_PLAIN
    };
    let piece = |case: usize, x: f32, y: f32| Draw::Sprite {
        rect: [x, y, corner_w, corner_h],
        uv: patch_uv(art.frame, CORNER_UV[case]),
        color,
    };
    out.push(piece(BOTTOM_LEFT, block.x, bottom));
    out.push(piece(BOTTOM_RIGHT, right, bottom));
    out.push(piece(top_left, block.x, block.y));
    out.push(piece(top_right, right, block.y));

    // Vertical edges run between the corner pieces, the right one mirrored;
    // horizontal edges likewise, the bottom one flipped. A block too short
    // or too narrow for its corners to leave a gap draws no edge there,
    // rather than a negative one.
    let edge_h = block.height - 2.0 * corner_h;
    if edge_h > 0.0 {
        let mut uv = VERTICAL_EDGE_UV;
        out.push(Draw::Sprite {
            rect: [block.x, block.y + corner_h, corner_w, edge_h],
            uv: patch_uv(art.frame, uv),
            color,
        });
        uv[0] += uv[2];
        uv[2] = -uv[2];
        out.push(Draw::Sprite {
            rect: [right, block.y + corner_h, corner_w, edge_h],
            uv: patch_uv(art.frame, uv),
            color,
        });
    }
    let edge_w = block.width - 2.0 * corner_w;
    if edge_w > 0.0 {
        let mut uv = HORIZONTAL_EDGE_UV;
        out.push(Draw::Sprite {
            rect: [block.x + corner_w, block.y, edge_w, corner_h],
            uv: patch_uv(art.frame, uv),
            color,
        });
        uv[1] += uv[3];
        uv[3] = -uv[3];
        out.push(Draw::Sprite {
            rect: [block.x + corner_w, bottom, edge_w, corner_h],
            uv: patch_uv(art.frame, uv),
            color,
        });
    }
}

/// One fill pass: the body below the band, then the band with its cuts.
/// `Block_DrawFill` and `Block_DrawTopBand`.
fn fill(block: &Block, art: &BlockArt, scale: (f32, f32), color: [f32; 4], out: &mut Vec<Draw>) {
    let (sx, sy) = scale;
    let inset_x = FILL_INSET * sx;
    let inset_y = FILL_INSET * sy;
    let band_h = BAND_HEIGHT * sy;
    let cut = BAND_HEIGHT * sx;
    let overhang = BAND_OVERHANG * sx;

    let fill_x = block.x + inset_x;
    let fill_y = block.y + inset_y;
    let fill_w = (block.width - 2.0 * inset_x).max(0.0);
    let fill_h = block.height - 2.0 * inset_y;

    // The body, from the band's foot to the bottom inset.
    let body_h = fill_h - band_h;
    if body_h > 0.0 {
        out.push(Draw::Fill {
            rect: [fill_x, fill_y + band_h, fill_w, body_h],
            color,
        });
    }

    // The band: its right edge is one unit past the fill's or seventeen
    // short of it, and its right end is always the 45-degree cut. Its left
    // end is a cut starting one unit left of the fill on the Fury style
    // (`0x0018a67c`), and a square edge on the fill's own line on HD
    // (`0x0018aad0`).
    let band_right = if block.landing {
        fill_x + fill_w - LANDING * sx
    } else {
        fill_x + fill_w + overhang
    };
    let (band_x, left_cut) = if art.fury {
        (fill_x - overhang, cut)
    } else {
        (fill_x, 0.0)
    };
    let band_w = band_right - band_x;
    if band_w > 0.0 {
        out.push(Draw::ChamferedFill {
            rect: [band_x, fill_y, band_w, band_h],
            chamfer: [left_cut, cut],
            color,
        });
    }
}

/// A block's inside in two colours, split at `split` (a drawing-grid `x`):
/// `block.color` left of it, `right` after it - HD's `Team Selection` stat
/// bars (`oag_ui_screens::picker::hd`). **The composition is this build's**; each
/// half is [`fill`]'s own shape and its own two passes, collapsed into the
/// one alpha those two passes leave over whatever is below
/// (`1 - (1 - a1)(1 - a2)`), so the two halves can meet without either
/// being drawn over the other. The left half's top band ends in the landing
/// a shaped block has, and the right half's band runs from under that
/// landing's cut to the block's own right end.
pub fn draw_split_fill(
    block: &Block,
    split: f32,
    right: [f32; 4],
    art: &BlockArt,
    out: &mut Vec<Draw>,
) {
    let second = if art.fury {
        art.fill_alpha
    } else {
        art.solid_alpha
    };
    let coverage = 1.0 - (1.0 - art.fill_alpha) * (1.0 - second);
    let fill_x = block.x + FILL_INSET;
    let fill_y = block.y + FILL_INSET;
    let fill_right = block.x + block.width - FILL_INSET;
    let body_y = fill_y + BAND_HEIGHT;
    let body_h = block.height - 2.0 * FILL_INSET - BAND_HEIGHT;
    let split = split.clamp(fill_x, fill_right);
    let left = alpha(block.color, coverage);
    let right = alpha(right, coverage);
    if body_h > 0.0 {
        out.push(Draw::Fill {
            rect: [fill_x, body_y, split - fill_x, body_h],
            color: left,
        });
        out.push(Draw::Fill {
            rect: [split, body_y, fill_right - split, body_h],
            color: right,
        });
    }
    let (band_x, left_cut) = if art.fury {
        (fill_x - BAND_OVERHANG, BAND_HEIGHT)
    } else {
        (fill_x, 0.0)
    };
    // The left band's landing, capped so it never runs past the block's
    // own - a full bar is the ordinary shaped block.
    let landing = (split - LANDING).min(fill_right - LANDING);
    if landing > band_x {
        out.push(Draw::ChamferedFill {
            rect: [band_x, fill_y, landing - band_x, BAND_HEIGHT],
            chamfer: [left_cut, BAND_HEIGHT],
            color: left,
        });
    }
    let right_band_x = landing.max(band_x) - BAND_HEIGHT;
    let right_band_end = if block.landing {
        fill_right - LANDING
    } else {
        fill_right + BAND_OVERHANG
    };
    if split < fill_right && right_band_end > right_band_x {
        out.push(Draw::ChamferedFill {
            rect: [
                right_band_x.max(band_x),
                fill_y,
                right_band_end - right_band_x.max(band_x),
                BAND_HEIGHT,
            ],
            chamfer: [0.0, BAND_HEIGHT],
            color: right,
        });
    }
}

/// A stretch of a block's inside drawn in one opaque colour, from drawing-grid
/// `from` to `to` (clamped to the fill's own extent), top band to bottom edge.
/// HD's `Team Selection` stat bar marks what a Fury model adds over the classic
/// hull this way. Opaque, not [`draw_split_fill`]'s translucent coverage: the
/// original's frame reads the palette entry itself (`HD_Blue`, `0xffac0717`)
/// with no black showing through. Drawn between the fill and the border so the
/// frame still sits on top of it.
pub fn draw_span(block: &Block, from: f32, to: f32, color: [f32; 4], out: &mut Vec<Draw>) {
    let fill_x = block.x + FILL_INSET;
    let fill_right = block.x + block.width - FILL_INSET;
    let (from, to) = (from.clamp(fill_x, fill_right), to.clamp(fill_x, fill_right));
    if to > from {
        out.push(Draw::Fill {
            rect: [
                from,
                block.y + FILL_INSET,
                to - from,
                block.height - 2.0 * FILL_INSET,
            ],
            color,
        });
    }
}

/// A nine-patch UV rectangle, in the executable's own texels of the patch,
/// placed in the sheet.
///
/// **`v` is flipped on the way.** The executable's `v` counts from the
/// file's first row, and a `.gtf`'s rows run bottom-up - which is why
/// `oag_hud::sprite::Sheet` reverses them into its top-down sheet (measured
/// on the loading screen's craft, see that module). So the file's row `r`
/// is the sheet's row `63 - r`, and a rectangle authored against the file
/// is the same rectangle against the sheet with its top and bottom swapped.
/// Checked both ways: read in the file's rows, the six corner cases land
/// on nonsense (the landing piece on a plain square); read in the sheet's,
/// every one lands on the feature its name says, and the strip's underline
/// mark - whose bar is at file row 1, sheet row 7 - lands on the capture's
/// `y + 42.6` from the executable's `y + 35` with no residual at all.
fn patch_uv(frame: Placed, uv: [f32; 4]) -> [f32; 4] {
    // The patch is addressed in sixty-fourths whatever its decoded size, so
    // a sheet that scaled it still samples the same features.
    let scale_x = frame.width as f32 / PATCH;
    let scale_y = frame.height as f32 / PATCH;
    [
        frame.x as f32 + uv[0] * scale_x,
        frame.y as f32 + (PATCH - uv[1]) * scale_y,
        uv[2] * scale_x,
        -uv[3] * scale_y,
    ]
}

/// `color` with its alpha replaced.
fn alpha(color: [f32; 4], alpha: f32) -> [f32; 4] {
    [color[0], color[1], color[2], alpha]
}

/// One standalone `<Block>`'s focus state, as `Block_Update` (`0x0018d588`)
/// keeps it for a block parsed with `selectable="true"`: the eased focus
/// fraction (`+0x15c`) that grows the block's width, and the 17-tick counter
/// (`+0x158`) that blinks its marker arrow. Both start at zero, which is what
/// `Block_Construct` writes.
///
/// A strip tab or a settings row eases the same way through its parent menu
/// (`HorizMenu_LayoutBlocks`, `List_Update`); this is the block doing it for
/// itself, which is what `EndRace Menu`'s free-standing option blocks do.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Focus {
    fraction: f32,
    blink: u8,
}

/// How far a focused standalone block grows: `60.0`, TOC `0x008ac60c` in
/// `Block_Update`. The width drawn is the authored one plus this times the
/// focus fraction.
pub const FOCUS_GROWTH: f32 = 60.0;
/// The share of the remaining distance the focus fraction closes each tick:
/// `0x3e2aaaab = 1/6`, TOC `0x008ac608`.
const FOCUS_EASE: f32 = 1.0 / 6.0;
/// The blink counter wraps past this, `0x11`.
const BLINK_PERIOD: u8 = 17;
/// Counter values from this one up draw the arrow hidden: `7 < counter`.
const BLINK_OFF_FROM: u8 = 8;

impl Focus {
    /// One tick of `Block_Update`. A focused block counts its blink and eases
    /// toward `1.0`; an unfocused one eases toward `0.0` and leaves its
    /// counter where it was, as the original does.
    pub fn tick(&mut self, focused: bool) {
        let target = if focused { 1.0 } else { 0.0 };
        if focused {
            self.blink += 1;
            if self.blink >= BLINK_PERIOD {
                self.blink = 0;
            }
        }
        let step = (target - self.fraction).abs() * FOCUS_EASE;
        if target > self.fraction {
            self.fraction = (self.fraction + step).min(target);
        } else if target < self.fraction {
            self.fraction = (self.fraction - step).max(target);
        }
    }

    /// The eased focus fraction, `0.0..=1.0`.
    #[must_use]
    pub fn fraction(&self) -> f32 {
        self.fraction
    }

    /// Whether the marker arrow is lit this tick - eight ticks on, nine off,
    /// the same duty cycle `HorizMenu_LayoutBlocks`' underline and
    /// `List_Update`'s marker run on. Only meaningful for the focused block;
    /// an unfocused one hides its arrow outright.
    #[must_use]
    pub fn arrow_lit(&self) -> bool {
        self.blink < BLINK_OFF_FROM
    }
}
