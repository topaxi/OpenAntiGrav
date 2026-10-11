//! This project's own bitmap format for art it authors itself: a plain-text
//! alpha mask.
//!
//! A game texture of the originals is a binary container this crate decodes; art
//! this project draws needs no such thing. It is a few hundred pixels of white
//! shape that the widget tints, the way the originals' own HUD masks are, and a
//! text file keeps it reviewable in a diff and plainly original.
//!
//! ```text
//! OAGMASK 4 2
//! .ff.
//! 0880
//! ```
//!
//! The first line is the magic and the width and height. Each following line is
//! one row, one character a pixel: `.` is clear and `0`-`9`, `a`-`f` are white at
//! that hex digit's share of full alpha (`f` opaque). Blank lines and lines
//! starting `#` are ignored. Colour is not stored; a widget's own tint supplies
//! it.

/// Why a mask would not decode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The first line is not `OAGMASK <width> <height>`.
    Header,
    /// A row's length is not the header's width, or the row count is not its height.
    Row {
        /// The zero-based row.
        row: usize,
        /// Its width.
        found: usize,
        /// The header's width.
        expected: usize,
    },
    /// The header promised more rows than the file holds.
    Rows {
        /// Rows found.
        found: usize,
        /// The header's height.
        expected: usize,
    },
    /// A character that is neither `.` nor a hex digit.
    Pixel {
        /// The zero-based row.
        row: usize,
        /// The character.
        character: char,
    },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Header => write!(f, "not an OAGMASK header"),
            Self::Row {
                row,
                found,
                expected,
            } => write!(f, "row {row} is {found} wide, expected {expected}"),
            Self::Rows { found, expected } => write!(f, "{found} row(s), expected {expected}"),
            Self::Pixel { row, character } => write!(
                f,
                "row {row} has {character:?}, which is not `.` or a hex digit"
            ),
        }
    }
}

impl std::error::Error for Error {}

/// A decoded mask: size and RGBA8888, white with the mask as alpha.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mask {
    /// Width in pixels.
    pub width: u16,
    /// Height in pixels.
    pub height: u16,
    /// `width * height * 4` bytes, top row first.
    pub rgba: Vec<u8>,
}

/// Whether `blob` opens with the magic, without decoding it.
#[must_use]
pub fn is_mask(blob: &[u8]) -> bool {
    blob.starts_with(b"OAGMASK ")
}

/// Decodes a mask.
///
/// # Errors
///
/// [`Error`] naming what is wrong with the header, a row or a pixel.
pub fn parse(blob: &[u8]) -> Result<Mask, Error> {
    let text = std::str::from_utf8(blob).map_err(|_| Error::Header)?;
    let mut lines = text
        .lines()
        .map(str::trim_end)
        .filter(|line| !line.is_empty() && !line.starts_with('#'));
    let header = lines.next().ok_or(Error::Header)?;
    let mut words = header.split_whitespace();
    if words.next() != Some("OAGMASK") {
        return Err(Error::Header);
    }
    let mut number = || {
        words
            .next()
            .and_then(|word| word.parse::<u16>().ok())
            .filter(|n| *n > 0)
            .ok_or(Error::Header)
    };
    let (width, height) = (number()?, number()?);

    let mut rgba = Vec::with_capacity(usize::from(width) * usize::from(height) * 4);
    let mut rows = 0;
    for (row, line) in lines.enumerate() {
        if line.chars().count() != usize::from(width) {
            return Err(Error::Row {
                row,
                found: line.chars().count(),
                expected: usize::from(width),
            });
        }
        for character in line.chars() {
            let alpha = match character {
                '.' => 0,
                digit => {
                    let value = digit.to_digit(16).ok_or(Error::Pixel {
                        row,
                        character: digit,
                    })?;
                    u8::try_from(value * 17).unwrap_or(255)
                }
            };
            rgba.extend_from_slice(&[255, 255, 255, alpha]);
        }
        rows += 1;
    }
    if rows != usize::from(height) {
        return Err(Error::Rows {
            found: rows,
            expected: usize::from(height),
        });
    }
    Ok(Mask {
        width,
        height,
        rgba,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_mask_decodes_to_white_with_the_digit_as_alpha() {
        let mask = parse(b"OAGMASK 3 2\n# a comment\n.f8\n0.a\n").unwrap();
        assert_eq!((mask.width, mask.height), (3, 2));
        let alpha: Vec<u8> = mask.rgba.chunks(4).map(|px| px[3]).collect();
        assert_eq!(alpha, [0, 255, 136, 0, 0, 170]);
        assert!(mask.rgba.chunks(4).all(|px| px[..3] == [255, 255, 255]));
    }

    #[test]
    fn a_ragged_or_short_mask_is_refused_rather_than_padded() {
        assert!(matches!(
            parse(b"OAGMASK 3 1\n.f\n"),
            Err(Error::Row { .. })
        ));
        assert!(matches!(
            parse(b"OAGMASK 2 2\n.f\n"),
            Err(Error::Rows { .. })
        ));
        assert!(matches!(
            parse(b"OAGMASK 2 1\n.z\n"),
            Err(Error::Pixel { .. })
        ));
        assert_eq!(parse(b"nope"), Err(Error::Header));
    }
}
