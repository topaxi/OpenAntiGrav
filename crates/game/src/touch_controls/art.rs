//! The overlay's four shapes as antialiased textures, generated in code.
//!
//! A disc, a ring, a triangle and a shield, white on transparent, built once
//! at startup by supersampling a shape test and placed in the overlay
//! renderer's sprite sheet beside the pointer (see [`crate::cursor::sheet`]).
//! Every button is then a handful of tinted, scaled quads rather than
//! stair-stepped rows of fills. **Procedural, not art**: nothing here is drawn by
//! hand or taken from a title, and none of it is measured.

use oag_hud::sprite::{DecodedImage, Placed, Sheet};

/// A texture's side in pixels: more than any button draws at on a 1080p
/// window, so minifying it stays smooth.
const SIDE: u32 = 256;
/// Samples per axis per pixel.
const SAMPLES: u32 = 3;
/// The ring's thickness as a share of its radius, shared by every button so a
/// body can be inset by exactly one ring.
pub(super) const RING_RATIO: f32 = 0.04;

const DISC: &str = "touch-disc";
const RING: &str = "touch-ring";
const TRIANGLE: &str = "touch-triangle";
const SHIELD: &str = "touch-shield";

/// Where the four textures sit in the sheet, in sheet pixels.
#[derive(Debug, Clone, Copy)]
pub struct Art {
    pub(super) disc: [f32; 4],
    pub(super) ring: [f32; 4],
    pub(super) triangle: [f32; 4],
    pub(super) shield: [f32; 4],
}

impl Art {
    /// The textures in `sheet`, or `None` when it was built without them.
    #[must_use]
    pub fn from_sheet(sheet: &Sheet) -> Option<Self> {
        let uv = |name: &str| {
            sheet
                .get(name)
                .map(|p: Placed| [p.x as f32, p.y as f32, p.width as f32, p.height as f32])
        };
        Some(Self {
            disc: uv(DISC)?,
            ring: uv(RING)?,
            triangle: uv(TRIANGLE)?,
            shield: uv(SHIELD)?,
        })
    }

    /// Made-up placements for a test that never draws a pixel: four
    /// distinguishable rectangles.
    #[cfg(test)]
    pub(super) fn flat() -> Self {
        let at = |i: f32| [0.0, i * 256.0, 256.0, 256.0];
        Self {
            disc: at(0.0),
            ring: at(1.0),
            triangle: at(2.0),
            shield: at(3.0),
        }
    }
}

fn texture(name: &str, inside: impl Fn(f32, f32) -> bool) -> DecodedImage {
    let mut rgba = Vec::with_capacity((SIDE * SIDE * 4) as usize);
    let step = 1.0 / (SIDE * SAMPLES) as f32;
    for py in 0..SIDE {
        for px in 0..SIDE {
            let mut hit = 0u32;
            for sy in 0..SAMPLES {
                for sx in 0..SAMPLES {
                    let x = (px * SAMPLES + sx) as f32 * step + step / 2.0;
                    let y = (py * SAMPLES + sy) as f32 * step + step / 2.0;
                    hit += u32::from(inside(x, y));
                }
            }
            let alpha = (hit * 255 + SAMPLES * SAMPLES / 2) / (SAMPLES * SAMPLES);
            rgba.extend_from_slice(&[255, 255, 255, alpha as u8]);
        }
    }
    DecodedImage {
        src: name.to_string(),
        width: SIDE,
        height: SIDE,
        rgba,
        quad_extent: None,
        blend: None,
    }
}

/// The four textures, for [`Sheet::build_with`].
#[must_use]
pub fn images() -> Vec<DecodedImage> {
    let radius = 0.5 - 1.0 / SIDE as f32;
    let from_centre = |x: f32, y: f32| ((x - 0.5).powi(2) + (y - 0.5).powi(2)).sqrt();
    vec![
        texture(DISC, |x, y| from_centre(x, y) <= radius),
        texture(RING, |x, y| {
            let d = from_centre(x, y);
            d <= radius && d >= radius * (1.0 - RING_RATIO)
        }),
        texture(TRIANGLE, |x, y| {
            let (top, bottom, half) = (0.03, 0.97, 0.47);
            if !(top..=bottom).contains(&y) {
                return false;
            }
            let reach = half * (y - top) / (bottom - top);
            (x - 0.5).abs() <= reach
        }),
        texture(SHIELD, |x, y| {
            let (top, bottom, half) = (0.03, 0.97, 0.45);
            if !(top..=bottom).contains(&y) {
                return false;
            }
            let shoulder = 0.45;
            let t = (y - top) / (bottom - top);
            let reach = if t < shoulder {
                half
            } else {
                half * (1.0 - (t - shoulder) / (1.0 - shoulder))
            };
            (x - 0.5).abs() <= reach
        }),
    ]
}
