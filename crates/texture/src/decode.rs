//! One door from a texture file to pixels: the container is chosen from the
//! file's resolved name ([`decode_named`]) or, for a blob that arrives
//! without one, from its bytes ([`decode_sniffed`]).
//!
//! Five containers ship a picture a 2D consumer draws: the PSP's `.mip`
//! ([`crate::texture`]), the PS2's `.pct` ([`crate::ps2_texture`]), the PS3's
//! `.gtf`, the Vita's `.gxt` and the PS4's `.gnf`. Every consumer that wants
//! flat RGBA (the HUD sheet, the exhaust and weapon sprites) goes through here;
//! a consumer that wants the disc's blocks and mip chain for the GPU (a
//! model's materials, `oag_mesh`) still reads the container's own module but
//! picks it with [`Container::of_name`].
//!
//! **Rows are as the file stores them.** A `.gtf`'s run bottom-up
//! ([`Picture::bottom_up`]); which way up a consumer wants them is the
//! consumer's, so the flip lives with the consumer that draws top-down
//! (`oag_hud::sprite`), exactly where it was measured
//! (`docs/formats/hd-loading.md`).

use crate::{gnf, gtf, gxt, ps2_texture, texture};

/// Which container a texture file is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Container {
    /// The PSP's palette-indexed `.mip`, and the `.tga` name some PSP assets
    /// keep for the same bytes.
    Mip,
    /// The PS2's GS upload packet, `.pct`.
    Pct,
    /// The PS3's `.gtf`.
    Gtf,
    /// The Vita's `.gxt`.
    Gxt,
    /// The PS4's `.gnf`.
    Gnf,
}

impl Container {
    /// The container a resolved texture name's extension says, or `None` for
    /// an extension no console's texture carries.
    #[must_use]
    pub fn of_name(name: &str) -> Option<Self> {
        let extension = name.rsplit_once('.')?.1;
        let is = |wanted: &str| extension.eq_ignore_ascii_case(wanted);
        if is("mip") || is("tga") {
            Some(Self::Mip)
        } else if is("pct") {
            Some(Self::Pct)
        } else if is("gtf") {
            Some(Self::Gtf)
        } else if is("gxt") {
            Some(Self::Gxt)
        } else if is("gnf") {
            Some(Self::Gnf)
        } else {
            None
        }
    }
}

impl Container {
    /// The extension a file of this container carries, with its dot, as a
    /// report names it.
    #[must_use]
    pub const fn extension(self) -> &'static str {
        match self {
            Self::Mip => ".mip",
            Self::Pct => ".pct",
            Self::Gtf => ".gtf",
            Self::Gxt => ".gxt",
            Self::Gnf => ".gnf",
        }
    }
}

/// A decoded texture: its base level as RGBA8888.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Picture {
    /// Base-level width in texels.
    pub width: u32,
    /// Base-level height in texels.
    pub height: u32,
    /// Bits per texel of the stored form (a block format's block bytes over
    /// its sixteen texels): a log field, nothing reads it for pixels.
    pub bits_per_pixel: u8,
    /// `width * height` texels, four bytes each, rows in the file's order.
    pub rgba: Vec<u8>,
    /// Which container it came out of.
    pub container: Container,
    /// Whether the rows run bottom-up (a `.gtf`'s do), so a top-down consumer
    /// must reverse them.
    pub bottom_up: bool,
}

/// Why a file produced no picture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The container's reader refused the bytes.
    Parse(String),
    /// A `.gtf` or `.gxt` that holds more than one texture, or none.
    NotSingle,
    /// The container parsed and its pixels would not convert (a cubemap, a
    /// format or tiling nobody has a formula for).
    Decode(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Parse(why) => write!(f, "does not parse ({why})"),
            Self::NotSingle => f.write_str("parses, but is not a single texture"),
            Self::Decode(why) => write!(f, "does not decode ({why})"),
        }
    }
}

impl std::error::Error for Error {}

/// Decodes `blob` as the container the resolved name `name` says, or by
/// [`decode_sniffed`] when the extension names none.
///
/// # Errors
///
/// [`Error`] naming which step refused.
pub fn decode_named(name: &str, blob: &[u8]) -> Result<Picture, Error> {
    match Container::of_name(name) {
        Some(container) => decode_as(container, blob),
        None => decode_sniffed(blob),
    }
}

/// Decodes `blob` as `container`.
///
/// # Errors
///
/// [`Error`] naming which step refused.
pub fn decode_as(container: Container, blob: &[u8]) -> Result<Picture, Error> {
    match container {
        Container::Mip => mip(blob),
        Container::Pct => pct(blob),
        Container::Gtf => gtf(blob),
        Container::Gxt => gxt(blob),
        Container::Gnf => gnf(blob),
    }
}

/// Decodes a blob that has no name: the PSP's `.mip`, then the PS2's `.pct`,
/// then `.gtf`, `.gxt` and `.gnf`.
///
/// **The `.mip` parser's error is the one reported** when nothing reads the
/// blob: a `.mip` that will not parse is far and away the commoner failure,
/// and the complaint of a parser for a format the file never was sends a reader
/// after the wrong decoder.
///
/// # Errors
///
/// [`Error::Parse`] with the `.mip` reader's complaint.
pub fn decode_sniffed(blob: &[u8]) -> Result<Picture, Error> {
    let first = match mip(blob) {
        Ok(picture) => return Ok(picture),
        Err(error) => error,
    };
    [pct, gtf, gxt, gnf]
        .into_iter()
        .find_map(|decode| decode(blob).ok())
        .ok_or(first)
}

fn mip(blob: &[u8]) -> Result<Picture, Error> {
    let texture = texture::Texture::parse(blob).map_err(|e| Error::Parse(e.to_string()))?;
    Ok(Picture {
        width: u32::from(texture.width),
        height: u32::from(texture.height),
        bits_per_pixel: texture.bits_per_pixel,
        rgba: texture.to_rgba(),
        container: Container::Mip,
        bottom_up: false,
    })
}

fn pct(blob: &[u8]) -> Result<Picture, Error> {
    let texture = ps2_texture::parse(blob).map_err(|e| Error::Parse(e.to_string()))?;
    Ok(Picture {
        width: u32::from(texture.width),
        height: u32::from(texture.height),
        bits_per_pixel: texture.bits_per_pixel,
        rgba: texture.to_rgba(),
        container: Container::Pct,
        bottom_up: false,
    })
}

fn gtf(blob: &[u8]) -> Result<Picture, Error> {
    let parsed = gtf::Gtf::parse(blob).map_err(|e| Error::Parse(e.to_string()))?;
    let texture = parsed.only().ok_or(Error::NotSingle)?;
    let rgba = texture
        .to_rgba(blob)
        .map_err(|e| Error::Decode(e.to_string()))?;
    let (width, height) = texture.level_size(0);
    let texels = if texture.format.is_block_compressed() {
        16
    } else {
        1
    };
    Ok(Picture {
        width,
        height,
        bits_per_pixel: u8::try_from(texture.format.unit_len() * 8 / texels).unwrap_or(u8::MAX),
        rgba: rgba.into_iter().flatten().collect(),
        container: Container::Gtf,
        bottom_up: true,
    })
}

fn gxt(blob: &[u8]) -> Result<Picture, Error> {
    let parsed = gxt::Gxt::parse(blob).map_err(|e| Error::Parse(e.to_string()))?;
    let texture = parsed.only().ok_or(Error::NotSingle)?;
    let rgba = texture
        .to_rgba(blob)
        .map_err(|e| Error::Decode(e.to_string()))?;
    Ok(Picture {
        width: u32::from(texture.width),
        height: u32::from(texture.height),
        bits_per_pixel: 8,
        rgba: rgba.into_iter().flatten().collect(),
        container: Container::Gxt,
        bottom_up: false,
    })
}

fn gnf(blob: &[u8]) -> Result<Picture, Error> {
    let texture = gnf::Texture::parse(blob).map_err(|e| Error::Parse(e.to_string()))?;
    let rgba = texture
        .decode(blob)
        .map_err(|e| Error::Decode(e.to_string()))?;
    Ok(Picture {
        width: texture.width,
        height: texture.height,
        bits_per_pixel: 8,
        rgba: rgba.into_iter().flatten().collect(),
        container: Container::Gnf,
        bottom_up: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_extension_picks_the_container_whatever_the_case() {
        for (name, want) in [
            ("a/b.MIP", Some(Container::Mip)),
            ("a\\b.tga", Some(Container::Mip)),
            ("b.pct", Some(Container::Pct)),
            ("a/textures/b.GTF", Some(Container::Gtf)),
            ("b.gxt", Some(Container::Gxt)),
            ("b.gnf", Some(Container::Gnf)),
            ("b.png", None),
            ("b", None),
        ] {
            assert_eq!(Container::of_name(name), want, "{name}");
        }
    }

    #[test]
    fn a_blob_nothing_reads_reports_the_mip_readers_complaint() {
        let error = decode_sniffed(&[0u8; 64]).unwrap_err();
        assert_eq!(error, decode_as(Container::Mip, &[0u8; 64]).unwrap_err());
    }

    #[test]
    fn a_name_with_no_container_falls_back_to_the_bytes() {
        assert_eq!(
            decode_named("unknown.bin", &[0u8; 64]),
            decode_sniffed(&[0u8; 64])
        );
    }
}
