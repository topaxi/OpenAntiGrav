//! Magic-number identification for files pulled off a disc.
//!
//! The table covers standard formats only. Wipeout's own containers are not in
//! it: the [WAD archive](crate::wad) has no magic at all, and blobs inside it
//! are identified by structure rather than by a leading tag. When a file comes
//! back [`Signature::Unknown`], that is the interesting case: it is a candidate
//! for reverse engineering, and the point of this module is to make the
//! unknowns easy to spot among the noise.

/// What a file appears to be, judged from its leading bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Signature {
    Known {
        /// Short label, for example `PSP EBOOT (PBP)`.
        label: &'static str,
        /// The magic that matched, rendered for display.
        magic: String,
    },
    Unknown {
        /// The first bytes, so a listing can show what was actually there.
        leading: String,
    },
}

impl Signature {
    /// A short label for display.
    #[must_use]
    pub fn label(&self) -> &str {
        match self {
            Self::Known { label, .. } => label,
            Self::Unknown { .. } => "unknown",
        }
    }

    /// Whether this file is a candidate for reverse engineering.
    #[must_use]
    pub fn is_unknown(&self) -> bool {
        matches!(self, Self::Unknown { .. })
    }
}

impl std::fmt::Display for Signature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Known { label, .. } => f.write_str(label),
            Self::Unknown { leading } => write!(f, "unknown [{leading}]"),
        }
    }
}

/// `(offset, magic, label)`.
///
/// Ordered most specific first so that a container is not mistaken for the
/// generic format it is built on.
const SIGNATURES: &[(usize, &[u8], &str)] = &[
    (0, b"\x00PBP", "PSP PBP (EBOOT)"),
    (0, b"~PSP", "PSP compressed executable"),
    (0, b"~SCE", "PSP SCE header"),
    (0, b"\x7fELF", "ELF executable"),
    (0, b"PSMF", "PSP PSMF video"),
    (0, b"RIFF", "RIFF container (WAV / AT3)"),
    (0, b"OggS", "Ogg"),
    (0, b"\xff\xfb", "MPEG audio"),
    (0, b"ID3", "MP3 with ID3 tag"),
    (0, b"BIKi", "Bink video"),
    (0, b"BIKb", "Bink video"),
    (0, b"SShd", "PS2 VAG stream header"),
    (0, b"VAGp", "PS2 VAG audio"),
    (0, b"pQES", "PS2 SPU container"),
    (0, b"GIF8", "GIF image"),
    (0, b"\x89PNG", "PNG image"),
    (0, b"\xff\xd8\xff", "JPEG image"),
    (0, b"DDS ", "DirectDraw Surface"),
    (0, b"BM", "BMP image"),
    (0, b"PK\x03\x04", "ZIP archive"),
    (0, b"\x1f\x8b", "gzip stream"),
    (0, b"MComprHD", "CHD disc image"),
    (0, b"CISO", "Compressed ISO"),
    (0, b"#!", "Script"),
];

/// Identifies `data` from its leading bytes.
#[must_use]
pub fn identify(data: &[u8]) -> Signature {
    for &(offset, magic, label) in SIGNATURES {
        let end = offset + magic.len();
        if data.len() >= end && &data[offset..end] == magic {
            return Signature::Known {
                label,
                magic: render(magic),
            };
        }
    }

    Signature::Unknown {
        leading: render(&data[..data.len().min(8)]),
    }
}

/// Renders bytes as printable ASCII where possible, hex otherwise.
///
/// Mixed rendering beats pure hex here: much game-disc magic is a four
/// character tag, and `WAD0` is instantly recognisable where `57 41 44 30` is
/// not.
fn render(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|&b| {
            if b.is_ascii_graphic() {
                (b as char).to_string()
            } else {
                format!("\\x{b:02x}")
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifies_an_elf() {
        let sig = identify(b"\x7fELF\x01\x01\x01\x00");
        assert_eq!(sig.label(), "ELF executable");
        assert!(!sig.is_unknown());
    }

    #[test]
    fn identifies_a_psp_eboot() {
        assert_eq!(
            identify(b"\x00PBP\x00\x00\x01\x00").label(),
            "PSP PBP (EBOOT)"
        );
    }

    #[test]
    fn reports_unknown_data_with_its_leading_bytes() {
        let sig = identify(b"WAD0\x01\x00\x00\x00");
        assert!(sig.is_unknown());
        assert_eq!(sig.to_string(), "unknown [WAD0\\x01\\x00\\x00\\x00]");
    }

    #[test]
    fn handles_data_shorter_than_any_magic() {
        let sig = identify(b"\x7f");
        assert!(sig.is_unknown(), "one byte cannot match a 4-byte magic");
    }

    #[test]
    fn handles_empty_data() {
        let sig = identify(&[]);
        assert!(sig.is_unknown());
        assert_eq!(sig.to_string(), "unknown []");
    }

    #[test]
    fn renders_mixed_printable_and_binary() {
        assert_eq!(render(b"RIFF"), "RIFF");
        assert_eq!(render(b"\x00PBP"), "\\x00PBP");
    }
}
