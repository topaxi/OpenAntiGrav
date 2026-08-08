//! The front end's own images, decoded once into a single texture.
//!
//! An `Image` widget names a `.mip` entry, and [`oag_formats::texture`] decodes
//! that to RGBA. Everything referenced is put in **one** texture so the renderer
//! keeps one bind group, the same way text and solid fills already share the
//! glyph atlas.
//!
//! The packing is a vertical stack - width is the widest image, height is the
//! sum - and that is deliberate rather than lazy. A real packer earns its
//! complexity when the wasted area matters, and the front-end root references
//! three images. When a screen needs enough of them for the waste to show, the
//! lookup below is already in pixels, so only this file has to change.

use oag_formats::ps2_texture;
use oag_formats::texture::Texture;

/// One decoded image, from either disc's texture format.
///
/// The two are different formats, not two versions of one - a PSP `.mip` is a
/// header, a palette and pixels, a PS2 texture is the GS upload packet that
/// would put one in video memory - but a sheet only needs a size and RGBA, so
/// the difference stops here. See `docs/formats/psp-texture.md` and
/// `docs/formats/ps2-texture.md`.
struct Image {
    width: u16,
    height: u16,
    bits_per_pixel: u8,
    rgba: Vec<u8>,
}

impl Image {
    /// Decodes whichever of the two formats this blob is.
    ///
    /// The PSP is tried first and its error is the one reported, because a
    /// `.mip` that will not parse is the far commoner failure and naming the
    /// PS2 parser's complaint about it would send a reader the wrong way.
    fn decode(blob: &[u8]) -> Result<Self, String> {
        match Texture::parse(blob) {
            Ok(t) => Ok(Self {
                width: t.width,
                height: t.height,
                bits_per_pixel: t.bits_per_pixel,
                rgba: t.to_rgba(),
            }),
            Err(psp) => match ps2_texture::parse(blob) {
                Ok(t) => Ok(Self {
                    width: t.width,
                    height: t.height,
                    bits_per_pixel: t.bits_per_pixel,
                    rgba: t.to_rgba(),
                }),
                Err(_) => Err(psp.to_string()),
            },
        }
    }

    /// The decoded pixels, consuming the image: the sheet copies them out and
    /// nothing wants them twice.
    fn into_rgba(self) -> Vec<u8> {
        self.rgba
    }
}

/// Transparent rows left between stacked images.
///
/// Not optional. The sheet is sampled with linear filtering, so without a gutter
/// a sample at an image's top edge blends in the last row of the image above it,
/// which showed up immediately as a faint line across the Pulse logo drawn from
/// the backdrop stacked over it.
const GUTTER: u32 = 1;

/// Where one image sits in the sheet, in pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Placed {
    /// Left edge in the sheet.
    pub x: u32,
    /// Top edge in the sheet.
    pub y: u32,
    /// The image's own width.
    pub width: u32,
    /// The image's own height.
    pub height: u32,
}

/// Every front-end image, in one RGBA buffer.
///
/// `Clone` because the menus draw through the same pipeline the picker does and
/// build their own renderer from the same sheet - see `main.rs`'s `Shell`. It
/// is a few hundred kilobytes copied once at startup, against tying the menus
/// to a stage that must have run first.
#[derive(Debug, Clone)]
pub struct Sheet {
    /// Sheet width in pixels.
    pub width: u32,
    /// Sheet height in pixels.
    pub height: u32,
    /// RGBA8888, row-major from the top left.
    pub rgba: Vec<u8>,
    /// Entry name to placement, in the order the images were loaded.
    ///
    /// A `Vec` rather than a map: there are a handful of entries, and a stable
    /// order keeps the sheet byte-identical from one run to the next, which is
    /// what makes a screenshot comparable.
    placed: Vec<(String, Placed)>,
}

impl Default for Sheet {
    /// An empty sheet is one transparent texel, not nothing.
    ///
    /// The renderer binds this unconditionally, so it has to be a real texture
    /// even when no screen asked for an image.
    fn default() -> Self {
        Self {
            width: 1,
            height: 1,
            rgba: vec![0, 0, 0, 0],
            placed: Vec::new(),
        }
    }
}

impl Sheet {
    /// Decodes `blobs` and stacks them.
    ///
    /// One that does not decode is reported and skipped rather than made fatal:
    /// an unreadable image should cost that image, not the front end. Reading
    /// the bytes is the caller's job because the images are spread across more
    /// than one archive.
    #[must_use]
    pub fn build(blobs: &[(String, Vec<u8>)], report: &mut Vec<String>) -> Self {
        let mut decoded: Vec<(String, Image)> = Vec::new();

        for (src, blob) in blobs {
            match Image::decode(blob) {
                Ok(image) => decoded.push((src.clone(), image)),
                Err(e) => report.push(format!("image {src}: {e}")),
            }
        }

        if decoded.is_empty() {
            return Self::default();
        }

        let width = decoded
            .iter()
            .map(|(_, t)| u32::from(t.width))
            .max()
            .unwrap_or(1);
        let height: u32 = decoded
            .iter()
            .map(|(_, t)| u32::from(t.height) + GUTTER)
            .sum();

        let mut rgba = vec![0u8; (width * height * 4) as usize];
        let mut placed = Vec::with_capacity(decoded.len());
        let mut pen = 0u32;

        for (src, texture) in decoded {
            let (w, h) = (u32::from(texture.width), u32::from(texture.height));
            let bits_per_pixel = texture.bits_per_pixel;
            let pixels = texture.into_rgba();

            // Copied row by row because the sheet is wider than the image
            // whenever another image is wider than this one.
            for row in 0..h {
                let from = (row * w * 4) as usize;
                let to = ((pen + row) * width * 4) as usize;
                rgba[to..to + (w * 4) as usize]
                    .copy_from_slice(&pixels[from..from + (w * 4) as usize]);
            }

            report.push(format!("image {src}: {w}x{h}, {bits_per_pixel}bpp"));
            placed.push((
                src,
                Placed {
                    x: 0,
                    y: pen,
                    width: w,
                    height: h,
                },
            ));
            pen += h + GUTTER;
        }

        Self {
            width,
            height,
            rgba,
            placed,
        }
    }

    /// Where `src` sits, if it was loaded.
    #[must_use]
    pub fn get(&self, src: &str) -> Option<Placed> {
        self.placed
            .iter()
            .find(|(name, _)| name == src)
            .map(|(_, placed)| *placed)
    }

    /// How many images the sheet holds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.placed.len()
    }

    /// Whether no image was loaded.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.placed.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A `.mip` of `width` by `height`, 8bpp, in a palette of one opaque colour.
    fn mip(width: u16, height: u16, value: u8) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&width.to_le_bytes());
        out.extend_from_slice(&height.to_le_bytes());
        out.push(8);
        out.resize(oag_formats::texture::HEADER_LEN, 0);
        for index in 0..256u32 {
            let level = u8::try_from(index).unwrap_or(255);
            out.extend_from_slice(&[level, level, level, 255]);
        }
        out.extend(std::iter::repeat_n(
            value,
            usize::from(width) * usize::from(height),
        ));
        out
    }

    #[test]
    fn images_stack_with_a_gutter_between_them() {
        let mut report = Vec::new();
        let sheet = Sheet::build(
            &[
                ("a".to_string(), mip(4, 2, 10)),
                ("b".to_string(), mip(8, 3, 20)),
            ],
            &mut report,
        );

        assert_eq!(sheet.width, 8, "the widest image sets the width");
        assert_eq!(sheet.height, 2 + GUTTER + 3 + GUTTER);

        let a = sheet.get("a").expect("a");
        let b = sheet.get("b").expect("b");
        assert_eq!((a.x, a.y, a.width, a.height), (0, 0, 4, 2));
        assert_eq!(
            (b.x, b.y, b.width, b.height),
            (0, 2 + GUTTER, 8, 3),
            "the second image starts past the first plus its gutter"
        );

        // The gutter row really is transparent, which is the whole point of it.
        let row = (2 * sheet.width * 4) as usize;
        assert!(
            sheet.rgba[row..row + (sheet.width * 4) as usize]
                .iter()
                .all(|&b| b == 0),
            "the gutter must not carry either image's pixels"
        );
    }

    #[test]
    fn an_image_that_does_not_decode_is_reported_and_skipped() {
        let mut report = Vec::new();
        let sheet = Sheet::build(
            &[
                ("bad".to_string(), vec![1, 2, 3]),
                ("good".to_string(), mip(2, 2, 7)),
            ],
            &mut report,
        );
        assert_eq!(sheet.len(), 1);
        assert!(sheet.get("bad").is_none());
        assert!(sheet.get("good").is_some());
        assert!(
            report.iter().any(|line| line.starts_with("image bad:")),
            "the failure has to be visible: {report:?}"
        );
    }

    #[test]
    fn an_empty_sheet_is_still_a_texture() {
        let sheet = Sheet::default();
        assert!(sheet.is_empty());
        assert_eq!((sheet.width, sheet.height), (1, 1));
        assert_eq!(sheet.rgba.len(), 4, "one transparent texel");
        assert_eq!(sheet.get(r"Data\FE\Images\pulse_logo.mip"), None);
    }
}
