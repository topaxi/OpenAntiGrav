//! A mod's vector icons: parsed once at load, rasterised at the exact pixel size the
//! pickup slot has on screen (and, where the title draws incoming-weapon warnings, in the
//! warning slots), and put in the HUD sheet beside the PNG icons.
//!
//! Nothing here draws a vector per frame. An icon is a texture like any other once
//! [`place`] has run; a change of resolution or HUD scale runs [`place`] again over the
//! sheet it started from. `<text>` is not supported (`resvg` is built without its font
//! stack) and a node of it draws nothing; an `<image>` is never read.

use resvg::{tiny_skia, usvg};

use crate::incoming::{Side, warning_src};
use crate::pickup::{IconFrame, mod_icon_size, warning_icon_size};
use crate::sprite::{DecodedImage, Sheet};

/// A parsed `.svg` icon, ready to be rasterised at any size.
#[derive(Debug, Clone)]
pub struct VectorIcon {
    /// The sheet key ([`Sheet::get`]) its raster is stored under.
    pub src: String,
    tree: usvg::Tree,
}

impl VectorIcon {
    /// Parses `bytes`.
    ///
    /// # Errors
    ///
    /// The parser's own message, when the document is not a usable SVG.
    pub fn parse(src: String, bytes: &[u8]) -> Result<Self, String> {
        let tree =
            usvg::Tree::from_data(bytes, &usvg::Options::default()).map_err(|e| e.to_string())?;
        let size = tree.size();
        if !(size.width() > 0.0 && size.height() > 0.0) {
            return Err("the drawing has no size".into());
        }
        Ok(Self { src, tree })
    }

    /// The drawing's own size, in the document's user units.
    #[must_use]
    pub fn size(&self) -> [f32; 2] {
        [self.tree.size().width(), self.tree.size().height()]
    }

    /// The drawing at exactly `width` by `height` pixels: RGBA8, straight alpha. The
    /// drawing is laid on the whole pixel grid, so a hard edge on a unit boundary is one
    /// texel wide at any size.
    #[must_use]
    pub fn rasterise(&self, width: u32, height: u32) -> Option<Vec<u8>> {
        let mut pixmap = tiny_skia::Pixmap::new(width, height)?;
        let size = self.tree.size();
        let transform = tiny_skia::Transform::from_scale(
            width as f32 / size.width(),
            height as f32 / size.height(),
        );
        resvg::render(&self.tree, transform, &mut pixmap.as_mut());
        Some(
            pixmap
                .pixels()
                .iter()
                .flat_map(|p| {
                    let c = p.demultiply();
                    [c.red(), c.green(), c.blue(), c.alpha()]
                })
                .collect(),
        )
    }
}

/// The longest side an icon is rasterised at, pixels. A drawing with no slot to fit is drawn
/// at its own size, and a document may claim any size: past this the raster is coarser, at
/// the same size on screen, never larger.
const MAX_SIDE: f32 = 1024.0;

/// `base` with every icon of `icons` rasterised for a screen showing `pixels_per_unit`
/// pixels of every layout unit, and marked with that density
/// ([`Sheet::texel_density`]) so the draw lays it on whole pixels. An icon past
/// [`MAX_SIDE`] is rasterised at a lower density and named in `report`.
///
/// An icon whose slot cannot be measured (a layout with no pickup frame) is rasterised at its
/// own size in layout units, for `hud-icon`.
#[must_use]
pub fn place(
    base: &Sheet,
    layout: &crate::Layout,
    art: &oag_title::HudArt,
    icons: &[VectorIcon],
    pixels_per_unit: f32,
    report: &mut Vec<String>,
) -> Sheet {
    let mut images = Vec::new();
    let mut sizes = Vec::new();
    for icon in icons {
        let frame = IconFrame {
            layout,
            sheet: base,
            art,
        };
        // Where the HUD has no pickup slot (a mode without one), the drawing's own size in
        // layout units, as a PNG is drawn at its own size.
        let size = mod_icon_size(frame, icon.size()).unwrap_or_else(|| icon.size());
        let longest = size[0].max(size[1]) * pixels_per_unit;
        let density = if longest > MAX_SIDE {
            report.push(format!(
                "{}: {longest:.0} pixels on its longest side, rasterised at {MAX_SIDE:.0}",
                icon.src
            ));
            pixels_per_unit * (MAX_SIDE / longest)
        } else {
            pixels_per_unit
        };
        let (width, height) = (
            (size[0] * density).round().max(1.0) as u32,
            (size[1] * density).round().max(1.0) as u32,
        );
        let Some(rgba) = icon.rasterise(width, height) else {
            report.push(format!(
                "{}: {width}x{height} is not a size; draws nothing",
                icon.src
            ));
            continue;
        };
        images.push(DecodedImage {
            src: icon.src.clone(),
            width,
            height,
            rgba,
            // The whole canvas is the art: the fit must not move with the antialiased fringe.
            quad_extent: Some([width as f32, height as f32]),
            quad_centre: Some([0.0, 0.0]),
            blend: None,
        });
        sizes.push((icon.src.clone(), density));
        // The incoming-warning slots are not the pickup's size: a title that draws warnings
        // gets the drawing again at each slot's own pixels.
        let sides: &[Side] = if art.incoming_warnings {
            &[Side::Rear, Side::Forward]
        } else {
            &[]
        };
        for &side in sides {
            let Some(size) = warning_icon_size(frame, side, icon.size()) else {
                continue;
            };
            let longest = size[0].max(size[1]) * pixels_per_unit;
            let density = pixels_per_unit * (MAX_SIDE / longest).min(1.0);
            let (width, height) = (
                (size[0] * density).round().max(1.0) as u32,
                (size[1] * density).round().max(1.0) as u32,
            );
            let Some(rgba) = icon.rasterise(width, height) else {
                continue;
            };
            let src = warning_src(&icon.src, side);
            images.push(DecodedImage {
                src: src.clone(),
                width,
                height,
                rgba,
                quad_extent: Some([width as f32, height as f32]),
                quad_centre: Some([0.0, 0.0]),
                blend: None,
            });
            sizes.push((src, density));
        }
    }
    let mut sheet = base.extended_with(&[], images, report);
    for (src, density) in sizes {
        sheet = sheet.with_texel_density(&src, density);
    }
    sheet
}
