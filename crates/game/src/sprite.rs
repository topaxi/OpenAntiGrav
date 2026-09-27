//! The front end's own images, decoded once into a single texture.
//!
//! An `Image` widget names a `.mip` entry, and [`oag_texture::texture`] decodes
//! that to RGBA. Everything referenced is put in **one** texture so the renderer
//! keeps one bind group, the same way text and solid fills already share the
//! glyph atlas.
//!
//! The packing is a vertical stack - width is the widest image, height is the
//! sum - and that is deliberate rather than lazy. A real packer earns its
//! complexity when the wasted area matters, and the front-end root references
//! three images. When a screen needs enough of them for the waste to show, the
//! lookup below is already in pixels, so only this file has to change.

use oag_texture::texture::Texture;
use oag_texture::{gnf, gtf, gxt, ps2_texture};
use oag_vex::vex;

/// One decoded image, from any of the four sources' texture formats.
///
/// Four different formats, not versions of one - a PSP `.mip` is a header, a
/// palette and pixels; a PS2 texture is the GS upload packet that would put
/// one in video memory; a PS3 `.gtf` is a big-endian RSX descriptor over DXT
/// or straight ARGB; a Vita `.gxt` is a little-endian GXM descriptor over a
/// Morton-tiled BC2 block grid - but a sheet only needs a size and RGBA, so
/// the difference stops here. See `docs/formats/psp-texture.md`,
/// `docs/formats/ps2-texture.md`, `docs/formats/gtf.md` and
/// `docs/formats/2048-hud.md`.
struct Image {
    width: u16,
    height: u16,
    bits_per_pixel: u8,
    rgba: Vec<u8>,
}

impl Image {
    /// Decodes whichever of the three formats this blob is.
    ///
    /// The PSP is tried first and its error is the one reported, because a
    /// `.mip` that will not parse is the far commoner failure and naming
    /// another parser's complaint about it would send a reader the wrong way.
    ///
    /// **The PS3 branch is why the report line used to say `1281x0`.** Wipeout
    /// HD's front end names three `.gtf` textures, and with only the two PSP-era
    /// parsers here the `.mip` one got there first, read a `.gtf`'s big-endian
    /// header as a `.mip`'s little-endian one and came back with a zero-sized
    /// texture - so all three failed, the sheet came out 1x1, and the error
    /// blamed a format the file is not. [`oag_texture::gtf`] has decoded these
    /// since well before the front end asked for one; nothing was missing but
    /// the branch.
    ///
    /// **The Vita branch is [`Self::decode_gxt`]**, tried last for the same
    /// reason `.gtf` is tried after the two PSP-era formats: 2048 is the one
    /// title whose textures reach it, and every other title's blob fails it
    /// immediately on the magic check.
    ///
    /// **The PS4 branch is [`Self::decode_gnf`]**, tried last of all - Omega
    /// is the only title whose front end reaches it, and `boot::sprites`'s
    /// own `gnf_sibling` already decides *whether* to hand a `.gnf` blob
    /// here at all, reporting a precise reason (a genuinely tiled surface
    /// this project has no formula for, or `Error::CorruptBlocks`) rather
    /// than letting a doomed one fall through to this function's generic
    /// `.mip`-parser error. See `docs/formats/gnf.md`.
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
                Err(_) => Self::decode_gtf(blob)
                    .or_else(|| Self::decode_gxt(blob))
                    .or_else(|| Self::decode_gnf(blob))
                    .ok_or_else(|| psp.to_string()),
            },
        }
    }

    /// The PS3 branch, or `None` for a blob that is not a `.gtf` this build
    /// draws.
    ///
    /// `None` rather than an error because the caller reports the *`.mip`*
    /// parser's complaint for anything none of the three recognise, and a `.gtf`
    /// error on a file that was never a `.gtf` is the misdirection this whole
    /// function exists to avoid. A `.gtf` that parses and then will not
    /// convert - the sky cubemaps [`oag_texture::gtf::Texture::to_rgba`]
    /// refuses - takes the same route out, which is the honest one: the sheet
    /// reports the image as undecoded rather than drawing a substitute for it.
    fn decode_gtf(blob: &[u8]) -> Option<Self> {
        let gtf = gtf::Gtf::parse(blob).ok()?;
        let texture = gtf.only()?;
        let rgba = texture.to_rgba(blob).ok()?;
        Some(Self {
            width: texture.width,
            height: texture.height,
            // Bits per *texel*, which for a block format means dividing the
            // block's bytes over the sixteen texels it covers: DXT1 reads 4 and
            // DXT45 reads 8, against 32 for straight ARGB. Reporting the block
            // instead would put "128bpp" on the boot line, which is a true
            // sentence about a unit nobody asked about. A log field; nothing
            // reads it.
            bits_per_pixel: {
                let texels = if texture.format.is_block_compressed() {
                    16
                } else {
                    1
                };
                u8::try_from(texture.format.unit_len() * 8 / texels).unwrap_or(u8::MAX)
            },
            // **Flipped, because a `.gtf`'s rows run bottom-up and a sheet is
            // top-down.** Measured against a screenshot of the running game:
            // `Pilot_Assist_fury.gtf` decoded straight into a sheet draws the
            // craft inverted, and reversed it matches the original's own
            // loading screen exactly. See `docs/formats/hd-loading.md`.
            //
            // **Here and not in [`oag_texture::gtf`]**, which is the part that
            // took measuring. Every other consumer of that decoder is a *3D*
            // one - `mesh::rcs::skin`, `mesh::sky_cube`,
            // `race::assets`'s trail noise - and those are already right: the
            // hull lettering on an HD craft reads the correct way up, which is
            // the check `docs/formats/rcsmodel.md` used to confirm the texture
            // coordinate in the first place. Their sampling convention and the
            // file's row order already agree, so flipping the decoder would
            // invert every one of them to fix one. The row order is a property
            // of the file; which way up a consumer wants it is the consumer's.
            rgba: rgba
                .chunks_exact(usize::from(texture.width).max(1))
                .rev()
                .flat_map(|row| row.iter().flatten().copied())
                .collect(),
        })
    }

    /// The Vita branch, or `None` for a blob that is not a `.gxt` this build
    /// draws.
    ///
    /// **Not flipped**, unlike [`Self::decode_gtf`]'s PS3 texel rows: the
    /// checkerboard-composited renders `gxt_ground_truth.rs` writes to
    /// `data/shots/` came out the right way up as decoded -
    /// `missile_reticule.gxt`'s crosshair sits above its dashed arc and
    /// `hud_2048.gxt`'s warning triangle points up - so the Vita's own GXM
    /// convention is top-down like a sheet already is, and no consumer here
    /// has been checked against a running frame to say otherwise. See
    /// `docs/formats/2048-hud.md`.
    fn decode_gxt(blob: &[u8]) -> Option<Self> {
        let parsed = gxt::Gxt::parse(blob).ok()?;
        let texture = parsed.only()?;
        let rgba = texture.to_rgba(blob).ok()?;
        Some(Self {
            width: texture.width,
            height: texture.height,
            // BC2 is 16 bytes per 4x4 block, 8 bits per texel.
            bits_per_pixel: 8,
            rgba: rgba.into_iter().flatten().collect(),
        })
    }

    /// The PS4 branch, or `None` for a blob that is not a `.gnf` this build
    /// draws.
    ///
    /// **Not flipped.** Checked directly against the boot report and
    /// against `Data/fe/images/presents_finnish.gnf`, `wipeout_omega_logo.gnf`
    /// and `hex_select.gnf` decoded to PNG (see `docs/formats/gnf.md`'s
    /// "Tiling" section) - the wordmark, the localized "esittää" text and
    /// the hexagon-select icon all read the right way up straight out of
    /// [`gnf::Texture::decode`], unlike [`Self::decode_gtf`]'s PS3 rows.
    fn decode_gnf(blob: &[u8]) -> Option<Self> {
        let texture = gnf::Texture::parse(blob).ok()?;
        let rgba = texture.decode(blob).ok()?;
        Some(Self {
            width: u16::try_from(texture.width).ok()?,
            height: u16::try_from(texture.height).ok()?,
            // BC7 is 16 bytes per 4x4 block, 8 bits per texel - the only
            // surface format this build's `gnf::Texture::decode` reaches.
            bits_per_pixel: 8,
            rgba: rgba.into_iter().flatten().collect(),
        })
    }

    /// An image somebody else already decoded, for [`Sheet::build_with`].
    ///
    /// `None` when the dimensions and the buffer disagree, or when either
    /// exceeds what a sheet coordinate can hold - a caller handing over pixels
    /// is doing so *because* it decoded them elsewhere, so the one thing worth
    /// checking here is that the two halves match.
    ///
    /// `bits_per_pixel` is reported as 32 because that is what the caller hands
    /// over, whatever the source packed it as; the field only reaches a report
    /// line.
    fn from_rgba(width: u32, height: u32, rgba: Vec<u8>) -> Option<Self> {
        let width = u16::try_from(width).ok()?;
        let height = u16::try_from(height).ok()?;
        if rgba.len() != usize::from(width) * usize::from(height) * 4 {
            return None;
        }
        Some(Self {
            width,
            height,
            bits_per_pixel: 32,
            rgba,
        })
    }

    /// The decoded pixels, consuming the image: the sheet copies them out and
    /// nothing wants them twice.
    fn into_rgba(self) -> Vec<u8> {
        self.rgba
    }
}

/// One image on its way into a sheet, with the two model-only properties that
/// travel beside it.
///
/// The blob path and the [`DecodedImage`] path converge here, which is the
/// only reason it exists - a tuple of four said nothing about which `Option`
/// was which.
struct Staged {
    src: String,
    image: Image,
    quad_extent: Option<[f32; 2]>,
    blend: Option<vex::BlendClass>,
}

/// Transparent rows left between stacked images.
///
/// Not optional. The sheet is sampled with linear filtering, so without a gutter
/// a sample at an image's top edge blends in the last row of the image above it,
/// which showed up immediately as a faint line across the Pulse logo drawn from
/// the backdrop stacked over it.
const GUTTER: u32 = 1;

// `Placed` moved to `oag_ui::frontend::placement`: it is pure data, and the
// front end needs to name it without pulling in this module's decode step.
// `pub use` rather than a private import: `crate::race::hud` names
// `crate::sprite::Placed::quad_extent`/`::blend` in its own doc comments, and
// this keeps that path resolving without editing a file this change does not
// otherwise touch. Safe to drop once `crate::race::hud`'s doc links are
// repointed at `oag_ui::frontend::Placed` directly.
pub use oag_ui::frontend::Placed;

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

/// One [`Sheet::build_with`] entry: an image somebody else already decoded,
/// plus the two model-only properties that ride along with it.
///
/// A struct rather than the tuple this was, because both trailing fields are
/// `Option`s of the same shape of thing - something read off a `.vex` model
/// and carried to the draw - and a five-element tuple stopped saying which was
/// which at the call site.
#[derive(Debug, Clone)]
pub struct DecodedImage {
    /// The reference the layout spells, and the key the sheet is looked up by.
    pub src: String,
    /// Pixel width of `rgba`.
    pub width: u32,
    /// Pixel height of `rgba`.
    pub height: u32,
    /// RGBA8888, row-major from the top left.
    pub rgba: Vec<u8>,
    /// Becomes [`Placed::quad_extent`].
    pub quad_extent: Option<[f32; 2]>,
    /// Becomes [`Placed::blend`].
    pub blend: Option<oag_vex::vex::BlendClass>,
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
        Self::build_with(blobs, Vec::new(), report)
    }

    /// [`Self::build`], plus images somebody else already decoded.
    ///
    /// `extra` is a [`DecodedImage`] apiece, appended after the blobs in the
    /// order given, so a sheet built the same way twice is byte-identical and a
    /// screenshot stays comparable. Its `quad_extent` and `blend` carry
    /// straight through to the matching [`Placed::quad_extent`] and
    /// [`Placed::blend`] - see those fields for what they are and why they
    /// travel separately from the pixel dimensions beside them.
    ///
    /// **It exists for art that is not in an image file.** The lock-on
    /// reticle's three pieces and Pure's ten weapon-icon pieces are `.vex`
    /// *models* whose texture is embedded in the model, so there is no `.mip`
    /// for [`Image::decode`] to read - `oag_render::mesh` is what unpacks
    /// them, and it hands back pixels rather than a blob. See
    /// `oag_race::sight` and `crate::race::hud::model_art`.
    #[must_use]
    pub fn build_with(
        blobs: &[(String, Vec<u8>)],
        extra: Vec<DecodedImage>,
        report: &mut Vec<String>,
    ) -> Self {
        let mut decoded: Vec<Staged> = Vec::new();

        for (src, blob) in blobs {
            match Image::decode(blob) {
                Ok(image) => decoded.push(Staged {
                    src: src.clone(),
                    image,
                    quad_extent: None,
                    blend: None,
                }),
                Err(e) => report.push(format!("image {src}: {e}")),
            }
        }

        for image in extra {
            let DecodedImage {
                src,
                width,
                height,
                rgba,
                quad_extent,
                blend,
            } = image;
            match Image::from_rgba(width, height, rgba) {
                Some(image) => decoded.push(Staged {
                    src,
                    image,
                    quad_extent,
                    blend,
                }),
                None => report.push(format!(
                    "image {src}: {width}x{height} does not match the pixels handed over"
                )),
            }
        }

        if decoded.is_empty() {
            return Self::default();
        }

        let width = decoded
            .iter()
            .map(|staged| u32::from(staged.image.width))
            .max()
            .unwrap_or(1);
        let height: u32 = decoded
            .iter()
            .map(|staged| u32::from(staged.image.height) + GUTTER)
            .sum();

        let mut rgba = vec![0u8; (width * height * 4) as usize];
        let mut placed = Vec::with_capacity(decoded.len());
        let mut pen = 0u32;

        for Staged {
            src,
            image: texture,
            quad_extent,
            blend,
        } in decoded
        {
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
                    quad_extent,
                    blend,
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

    /// This sheet with `blobs` decoded and stacked **under** it.
    ///
    /// Every placement this sheet already holds keeps its rectangle - the
    /// new images go below the last gutter, and the sheet only ever gets
    /// wider - so a renderer that swaps to the extended sheet
    /// (`crate::render::Renderer::set_sprites`) keeps drawing every widget
    /// it already drew, and nothing has to be swapped back. What the
    /// selection screens need: their stills are per circuit and read when
    /// the circuit is selected, long after the front end's own sheet was
    /// built at boot. A `src` this sheet already holds is not added twice;
    /// an image that will not decode is reported and skipped, as
    /// [`Self::build`] does.
    #[must_use]
    pub fn extended(&self, blobs: &[(String, Vec<u8>)], report: &mut Vec<String>) -> Self {
        let fresh: Vec<(String, Vec<u8>)> = blobs
            .iter()
            .filter(|(src, _)| self.get(src).is_none())
            .cloned()
            .collect();
        if fresh.is_empty() {
            return self.clone();
        }
        // An empty sheet is one transparent texel with nothing placed in it,
        // so there is nothing to keep.
        if self.placed.is_empty() {
            return Self::build(&fresh, report);
        }
        let below = Self::build(&fresh, report);
        if below.placed.is_empty() {
            return self.clone();
        }
        let width = self.width.max(below.width);
        let height = self.height + below.height;
        let mut rgba = vec![0u8; (width * height * 4) as usize];
        let copy_rows = |rgba: &mut Vec<u8>, from: &Self, at: u32| {
            for row in 0..from.height {
                let src = (row * from.width * 4) as usize;
                let dst = ((at + row) * width * 4) as usize;
                rgba[dst..dst + (from.width * 4) as usize]
                    .copy_from_slice(&from.rgba[src..src + (from.width * 4) as usize]);
            }
        };
        copy_rows(&mut rgba, self, 0);
        copy_rows(&mut rgba, &below, self.height);
        let mut placed = self.placed.clone();
        placed.extend(below.placed.into_iter().map(|(src, mut at)| {
            at.y += self.height;
            (src, at)
        }));
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

    /// The alpha of one texel of `placed`, addressed as a fraction of the
    /// image on each axis, `0.0..=1.0`.
    ///
    /// Nearest texel, no filtering: what the menu blocks' fill samples off
    /// its nine-patch is a swatch the texture's author painted flat two
    /// texels wide, so the bilinear tap the original takes at its centre
    /// reads the same. `None` for a point outside the sheet, which a
    /// placement this sheet handed out cannot produce.
    #[must_use]
    pub fn alpha_at(&self, placed: Placed, u: f32, v: f32) -> Option<f32> {
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "a texel index inside a placement, clamped below"
        )]
        let texel = |fraction: f32, size: u32| -> u32 {
            ((fraction * size as f32).floor().max(0.0) as u32).min(size.saturating_sub(1))
        };
        let x = placed.x + texel(u, placed.width);
        let y = placed.y + texel(v, placed.height);
        if x >= self.width || y >= self.height {
            return None;
        }
        let at = ((y * self.width + x) * 4 + 3) as usize;
        self.rgba.get(at).map(|alpha| f32::from(*alpha) / 255.0)
    }

    /// Every placement this sheet holds, name and rectangle together.
    ///
    /// The same shape `crate::frontend::Frontend`'s own `placements` field
    /// is - `crate::menu::frame::read` takes this rather than `&Sheet`
    /// itself, so it can look widgets up the same way `Frontend` already
    /// looks its own up, without `oag-ui` naming this crate's decode type.
    #[must_use]
    pub fn entries(&self) -> &[(String, Placed)] {
        &self.placed
    }

    /// The one image this sheet holds, when it holds exactly one.
    ///
    /// For a sheet built to carry a single picture - the loading screen's
    /// backdrop is the only one so far - where naming the entry again to look it
    /// up would mean carrying the name beside the sheet for no other reason.
    /// `None` for a sheet with none or several, so a caller cannot silently get
    /// "the first of many". Mirrors `oag_texture::gtf::Gtf::only`.
    #[must_use]
    pub fn only(&self) -> Option<(&str, Placed)> {
        match self.placed.as_slice() {
            [(name, placed)] => Some((name.as_str(), *placed)),
            _ => None,
        }
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

    /// A sheet with these placements and no pixels, for a test about geometry.
    ///
    /// The alternative is fabricating a `.mip` per entry to make the packer put
    /// an image somewhere known, which is a lot of bytes to say "this one is 200
    /// rows down".
    #[cfg(test)]
    pub(crate) fn placed_at(entries: &[(&str, Placed)]) -> Self {
        Self {
            placed: entries
                .iter()
                .map(|(name, placed)| ((*name).to_string(), *placed))
                .collect(),
            ..Self::default()
        }
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
        out.resize(oag_texture::texture::HEADER_LEN, 0);
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
    fn an_extended_sheet_keeps_every_placement_it_had() {
        let mut report = Vec::new();
        let base = Sheet::build(&[("a".to_string(), mip(4, 2, 10))], &mut report);
        let extended = base.extended(
            &[
                ("b".to_string(), mip(8, 3, 20)),
                ("a".to_string(), mip(2, 2, 30)),
            ],
            &mut report,
        );
        assert_eq!(base.get("a"), extended.get("a"), "a keeps its rectangle");
        assert_eq!(extended.width, 8, "the wider new image widens the sheet");
        let b = extended.get("b").expect("b");
        assert_eq!(
            (b.x, b.y, b.width, b.height),
            (0, base.height, 8, 3),
            "b starts where the old sheet ended, past its own last gutter"
        );
        assert_eq!(extended.len(), 2, "a is not added twice");
        // The old pixels are where they were, re-strided to the new width.
        let old_row = &base.rgba[0..4 * 4];
        let new_row = &extended.rgba[0..4 * 4];
        assert_eq!(old_row, new_row);
        // Extending with nothing new is a copy.
        let same = extended.extended(&[("a".to_string(), mip(1, 1, 1))], &mut report);
        assert_eq!(
            (same.width, same.height, same.len()),
            (8, extended.height, 2)
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
