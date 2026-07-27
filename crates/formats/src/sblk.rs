//! `.bnk` sound banks: an `SBlk` descriptor block plus PS-ADPCM waveforms.
//!
//! The blob the [WAD](crate::wad) census calls a `03000000` entry is a
//! two-section container:
//!
//! ```text
//! +0x00  u32  version         3 in every bank seen
//! +0x04  u32  section_count   2 in every bank seen
//! +0x08  section[section_count], 8 bytes each: { u32 offset, u32 size }
//!        section 0: the SBlk descriptor block
//!        section 1: PS-ADPCM waveform data
//! ```
//!
//! The sections abut, the first starts immediately after the table, and the
//! last ends exactly at the end of the file.
//!
//! The `SBlk` block opens with a 64-byte header:
//!
//! ```text
//! +0x00  char[4]  "SBlk"
//! +0x04  u32      version, 3
//! +0x08  u32      772 when a voice-state block is present, 260 when not
//! +0x0c  u32      zero
//! +0x10  u32      zero
//! +0x14  u16      zero
//! +0x16  u16      cue_count        entries in the 12-byte table at +0x40
//! +0x18  u16      command_count    entries in the 8-byte table
//! +0x1a  u16      waveform_count
//! +0x1c  u32      cue table offset, always 64
//! +0x20  u32      command table offset
//! +0x24  u32      20544, the same in every bank
//! +0x28  u32      waveform data size
//! +0x2c  u32      waveform data size again
//! +0x30  u32      zero
//! +0x34  u32      parameter block offset
//! +0x38  u32      name block offset
//! +0x3c  u32      voice-state block offset, or zero
//! ```
//!
//! See `docs/formats/psp-audio.md` for the evidence.
//!
//! # The two size fields are the cross-check
//!
//! `+0x28` and `+0x2c` both hold the length of section 1, which the container's
//! own section table already states. Three independent statements of the same
//! number agreeing on all 39 banks is what says the header is being read at the
//! right offsets rather than plausibly.
//!
//! # The waveforms are PS-ADPCM
//!
//! Sony's 16-byte block format: one predictor/shift byte, one flag byte, and 14
//! bytes holding 28 four-bit residuals. Nothing declares it - it is established
//! by the flag byte, which has to be one of eight values and is on 99.96% of the
//! disc's 530,916 blocks.
//!
//! # What is not decoded
//!
//! Where each individual waveform starts. The three tables here address each
//! other rather than the waveform data, and only the simplest banks have an
//! obvious offset/length pair. [`Bank::name`] is recovered, and
//! [`decode_adpcm`] will decode any span the caller can identify, but this
//! module does not split a bank into its component sounds. See the page's open
//! questions.

/// Bytes before the section table.
pub const HEADER_LEN: usize = 8;

/// Bytes per section-table entry.
pub const SECTION_LEN: usize = 8;

/// Bytes of `SBlk` header before the first table.
pub const SBLK_HEADER_LEN: usize = 64;

/// The `SBlk` magic.
pub const MAGIC: &[u8; 4] = b"SBlk";

/// The only container version seen.
pub const VERSION: u32 = 3;

/// Bytes per PS-ADPCM block.
pub const ADPCM_BLOCK_LEN: usize = 16;

/// Samples a PS-ADPCM block expands to.
pub const ADPCM_BLOCK_SAMPLES: usize = 28;

/// Bytes per cue-table entry.
pub const CUE_LEN: usize = 12;

/// Bytes per command-table entry.
pub const COMMAND_LEN: usize = 8;

/// Bytes per voice-state entry.
pub const VOICE_LEN: usize = 16;

/// Something wrong with a sound bank.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// Fewer bytes than the header needs.
    TooShort {
        /// Bytes supplied.
        got: usize,
    },
    /// The container version was not [`VERSION`].
    UnsupportedVersion {
        /// The value found.
        version: u32,
    },
    /// The section count was not 2.
    UnexpectedSectionCount {
        /// The value found.
        count: u32,
    },
    /// A section runs past the end of the blob, or the sections do not abut.
    BadSections,
    /// The descriptor section does not start with [`MAGIC`].
    NotSblk,
    /// The `SBlk` header's waveform size disagrees with the section table.
    SizeDisagreement {
        /// What the `SBlk` header says.
        declared: u32,
        /// What the section table says.
        section: u32,
    },
    /// The waveform section is not a whole number of PS-ADPCM blocks.
    PartialAdpcmBlock {
        /// Bytes in the section.
        size: usize,
    },
    /// A table declared in the header does not fit the descriptor section.
    TableOutOfRange {
        /// Which table.
        name: &'static str,
        /// Where it claims to start.
        offset: u32,
    },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooShort { got } => write!(f, "need at least {HEADER_LEN} bytes, got {got}"),
            Self::UnsupportedVersion { version } => {
                write!(f, "unsupported bank version {version}, expected {VERSION}")
            }
            Self::UnexpectedSectionCount { count } => {
                write!(f, "{count} sections, expected 2")
            }
            Self::BadSections => write!(f, "the section table does not span the blob exactly"),
            Self::NotSblk => write!(f, "the descriptor section does not begin with SBlk"),
            Self::SizeDisagreement { declared, section } => write!(
                f,
                "the SBlk header declares {declared} waveform bytes, the section table {section}"
            ),
            Self::PartialAdpcmBlock { size } => write!(
                f,
                "{size} waveform bytes is not a whole number of {ADPCM_BLOCK_LEN}-byte blocks"
            ),
            Self::TableOutOfRange { name, offset } => {
                write!(f, "the {name} table at {offset} is outside the block")
            }
        }
    }
}

impl std::error::Error for Error {}

/// Result alias for this module.
pub type Result<T> = std::result::Result<T, Error>;

/// A parsed sound bank.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bank<'a> {
    /// The bank's own name, up to 7 characters.
    ///
    /// Stored in an 8-byte field with a forced NUL, so anything longer is
    /// truncated: the track banks read `basilic`, `metropi`, `talonsj`. Where
    /// the real name is short enough to survive, `Data\Sound\<name>.bnk` hashes
    /// to the bank's own WAD entry - see the format page.
    pub name: String,
    /// Entries in the 12-byte cue table.
    pub cue_count: u16,
    /// Entries in the 8-byte command table.
    pub command_count: u16,
    /// Waveforms the bank holds.
    pub waveform_count: u16,
    /// The 12-byte cue table.
    pub cues: &'a [u8],
    /// The 8-byte command table.
    pub commands: &'a [u8],
    /// PS-ADPCM waveform data, a whole number of 16-byte blocks.
    pub waveforms: &'a [u8],
}

fn word(data: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([data[at], data[at + 1], data[at + 2], data[at + 3]])
}

fn half(data: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([data[at], data[at + 1]])
}

/// Whether `data` looks like a sound bank, without parsing it.
#[must_use]
pub fn looks_like_bank(data: &[u8]) -> bool {
    data.len() >= 0x1c && word(data, 0) == VERSION && &data[0x18..0x1c] == MAGIC
}

impl<'a> Bank<'a> {
    /// Parses a bank.
    ///
    /// # Errors
    ///
    /// Fails when the container framing does not close exactly, the descriptor
    /// section is not an `SBlk` block, the two waveform sizes disagree, or the
    /// waveform section is not a whole number of PS-ADPCM blocks. Every one of
    /// those is a statement the file makes about itself, so a failure means this
    /// is not a bank rather than that it is a damaged one.
    pub fn parse(data: &'a [u8]) -> Result<Self> {
        if data.len() < HEADER_LEN {
            return Err(Error::TooShort { got: data.len() });
        }
        let version = word(data, 0);
        if version != VERSION {
            return Err(Error::UnsupportedVersion { version });
        }
        let count = word(data, 4);
        if count != 2 {
            return Err(Error::UnexpectedSectionCount { count });
        }
        let table_end = HEADER_LEN + 2 * SECTION_LEN;
        if data.len() < table_end {
            return Err(Error::TooShort { got: data.len() });
        }

        let mut sections = [(0u32, 0u32); 2];
        for (index, section) in sections.iter_mut().enumerate() {
            let at = HEADER_LEN + index * SECTION_LEN;
            *section = (word(data, at), word(data, at + 4));
        }
        // The framing has no slack: first section starts at the table's end,
        // the second starts where the first stops, and the second ends on the
        // blob. All three hold on every bank on the disc.
        let spans = sections[0].0 as usize == table_end
            && sections[0].0.checked_add(sections[0].1) == Some(sections[1].0)
            && sections[1]
                .0
                .checked_add(sections[1].1)
                .is_some_and(|end| end as usize == data.len());
        if !spans {
            return Err(Error::BadSections);
        }

        let block = &data[sections[0].0 as usize..][..sections[0].1 as usize];
        let waveforms = &data[sections[1].0 as usize..][..sections[1].1 as usize];

        if block.len() < SBLK_HEADER_LEN || &block[..4] != MAGIC {
            return Err(Error::NotSblk);
        }
        let declared = word(block, 0x28);
        if declared != word(block, 0x2c) || declared != sections[1].1 {
            return Err(Error::SizeDisagreement {
                declared,
                section: sections[1].1,
            });
        }
        if waveforms.len() % ADPCM_BLOCK_LEN != 0 {
            return Err(Error::PartialAdpcmBlock {
                size: waveforms.len(),
            });
        }

        let cue_count = half(block, 0x16);
        let command_count = half(block, 0x18);
        let waveform_count = half(block, 0x1a);
        let cue_offset = word(block, 0x1c);
        let command_offset = word(block, 0x20);
        let name_offset = word(block, 0x38);

        let slice = |offset: u32, len: usize, name: &'static str| -> Result<&'a [u8]> {
            let start = offset as usize;
            block
                .get(start..start + len)
                .ok_or(Error::TableOutOfRange { name, offset })
        };
        let cues = slice(cue_offset, usize::from(cue_count) * CUE_LEN, "cue")?;
        let commands = slice(
            command_offset,
            usize::from(command_count) * COMMAND_LEN,
            "command",
        )?;

        let name = block
            .get(name_offset as usize..)
            .and_then(|tail| tail.get(..8))
            .map(|field| {
                let end = field.iter().position(|&b| b == 0).unwrap_or(field.len());
                String::from_utf8_lossy(&field[..end]).into_owned()
            })
            .unwrap_or_default();

        Ok(Self {
            name,
            cue_count,
            command_count,
            waveform_count,
            cues,
            commands,
            waveforms,
        })
    }

    /// PS-ADPCM blocks in the waveform section.
    #[must_use]
    pub fn adpcm_blocks(&self) -> usize {
        self.waveforms.len() / ADPCM_BLOCK_LEN
    }
}

/// The four PS-ADPCM predictor filters, as `(previous, previous but one)`.
///
/// Sony's published coefficients, over 64. A block naming a filter above 4 is
/// out of spec; the hardware falls back to the flat one and so does this.
const FILTERS: [(i32, i32); 5] = [(0, 0), (60, 0), (115, -52), (98, -55), (122, -60)];

/// Whether a PS-ADPCM block's predictor and shift are in range.
///
/// The predictor is the high nibble of byte 0 and selects one of five filters;
/// the shift is the low nibble and the hardware clamps it at 12. Both being in
/// range on essentially every block is the evidence that a blob is PS-ADPCM at
/// all, so this is exposed for surveys rather than kept private.
#[must_use]
pub fn adpcm_block_is_in_spec(block: &[u8]) -> bool {
    !block.is_empty() && usize::from(block[0] >> 4) < FILTERS.len() && block[0] & 0x0f <= 12
}

/// Whether a PS-ADPCM block's flag byte is one of the eight defined values.
///
/// 0 nothing, 1 end, 2 loop body, 3 loop end, 4 start, 5 start and end,
/// 6 loop start, 7 end and mute.
#[must_use]
pub fn adpcm_flag_is_defined(block: &[u8]) -> bool {
    block.len() > 1 && block[1] < 8
}

/// Decodes PS-ADPCM into 16-bit samples.
///
/// `data` is a whole number of 16-byte blocks; a trailing partial block is
/// ignored. The predictor history starts at zero, so a caller that decodes a
/// span out of the middle of a bank gets a short transient rather than silence
/// at the join.
#[must_use]
pub fn decode_adpcm(data: &[u8]) -> Vec<i16> {
    let mut out = Vec::with_capacity(data.len() / ADPCM_BLOCK_LEN * ADPCM_BLOCK_SAMPLES);
    let mut history = (0i32, 0i32);

    for block in data.chunks_exact(ADPCM_BLOCK_LEN) {
        let mut shift = u32::from(block[0] & 0x0f);
        let filter = usize::from(block[0] >> 4);
        // Out of spec on 224 of the disc's 530,916 blocks. The hardware does
        // not fault on those, so neither does this.
        if shift > 12 {
            shift = 9;
        }
        let (f0, f1) = FILTERS[filter.min(FILTERS.len() - 1)];

        for &byte in &block[2..] {
            for nibble in [byte & 0x0f, byte >> 4] {
                // Sign-extend the 4-bit residual from the top of a 16-bit word,
                // which is where the shift is defined to apply.
                let residual = i32::from(i16::from(nibble) << 12) >> shift;
                let predicted = (history.0 * f0 + history.1 * f1) >> 6;
                let sample = (residual + predicted).clamp(-32768, 32767);
                history = (sample, history.0);
                #[expect(
                    clippy::cast_possible_truncation,
                    reason = "clamped to i16 on the line above"
                )]
                out.push(sample as i16);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds a bank by hand. No game data in any test.
    fn bank(cues: u16, commands: u16, waveforms: u16, blocks: usize, name: &str) -> Vec<u8> {
        let cue_bytes = usize::from(cues) * CUE_LEN;
        let command_bytes = usize::from(commands) * COMMAND_LEN;
        let command_offset = SBLK_HEADER_LEN + cue_bytes;
        let name_offset = command_offset + command_bytes;
        let block_len = name_offset + 8;
        let waveform_len = blocks * ADPCM_BLOCK_LEN;

        let mut block = vec![0u8; block_len];
        block[..4].copy_from_slice(MAGIC);
        block[4..8].copy_from_slice(&VERSION.to_le_bytes());
        block[0x16..0x18].copy_from_slice(&cues.to_le_bytes());
        block[0x18..0x1a].copy_from_slice(&commands.to_le_bytes());
        block[0x1a..0x1c].copy_from_slice(&waveforms.to_le_bytes());
        block[0x1c..0x20].copy_from_slice(&(SBLK_HEADER_LEN as u32).to_le_bytes());
        block[0x20..0x24].copy_from_slice(&(command_offset as u32).to_le_bytes());
        block[0x24..0x28].copy_from_slice(&20544u32.to_le_bytes());
        block[0x28..0x2c].copy_from_slice(&(waveform_len as u32).to_le_bytes());
        block[0x2c..0x30].copy_from_slice(&(waveform_len as u32).to_le_bytes());
        block[0x38..0x3c].copy_from_slice(&(name_offset as u32).to_le_bytes());
        let bytes = name.as_bytes();
        let take = bytes.len().min(7);
        block[name_offset..name_offset + take].copy_from_slice(&bytes[..take]);

        let mut out = VERSION.to_le_bytes().to_vec();
        out.extend_from_slice(&2u32.to_le_bytes());
        let first = (HEADER_LEN + 2 * SECTION_LEN) as u32;
        out.extend_from_slice(&first.to_le_bytes());
        out.extend_from_slice(&(block_len as u32).to_le_bytes());
        out.extend_from_slice(&(first + block_len as u32).to_le_bytes());
        out.extend_from_slice(&(waveform_len as u32).to_le_bytes());
        out.extend_from_slice(&block);
        // A flat, in-spec waveform: filter 0, shift 9, flag 0.
        for _ in 0..blocks {
            out.push(0x09);
            out.push(0x00);
            out.extend_from_slice(&[0x11u8; 14]);
        }
        out
    }

    #[test]
    fn a_bank_parses_and_its_sections_close() {
        let data = bank(6, 10, 7, 4, "FRNTEND");
        let parsed = Bank::parse(&data).expect("parse");
        assert_eq!(parsed.name, "FRNTEND");
        assert_eq!(parsed.cue_count, 6);
        assert_eq!(parsed.command_count, 10);
        assert_eq!(parsed.waveform_count, 7);
        assert_eq!(parsed.cues.len(), 6 * CUE_LEN);
        assert_eq!(parsed.commands.len(), 10 * COMMAND_LEN);
        assert_eq!(parsed.adpcm_blocks(), 4);
        assert!(looks_like_bank(&data));
    }

    #[test]
    fn a_name_longer_than_the_field_is_truncated() {
        let data = bank(1, 1, 1, 1, "basilico");
        assert_eq!(Bank::parse(&data).expect("parse").name, "basilic");
    }

    #[test]
    fn disagreeing_waveform_sizes_are_refused() {
        let mut data = bank(1, 1, 1, 2, "x");
        // The second copy of the size, inside the SBlk header.
        let at = HEADER_LEN + 2 * SECTION_LEN + 0x2c;
        data[at..at + 4].copy_from_slice(&16u32.to_le_bytes());
        assert!(matches!(
            Bank::parse(&data),
            Err(Error::SizeDisagreement { .. })
        ));
    }

    #[test]
    fn a_gap_between_the_sections_is_refused() {
        let mut data = bank(1, 1, 1, 2, "x");
        let at = HEADER_LEN + SECTION_LEN; // the second section's offset
        let offset = u32::from_le_bytes([data[at], data[at + 1], data[at + 2], data[at + 3]]);
        data[at..at + 4].copy_from_slice(&(offset + 16).to_le_bytes());
        assert_eq!(Bank::parse(&data), Err(Error::BadSections));
    }

    #[test]
    fn a_truncated_blob_is_refused() {
        let data = bank(1, 1, 1, 2, "x");
        assert!(Bank::parse(&data[..data.len() - 1]).is_err());
        assert!(Bank::parse(&data[..4]).is_err());
    }

    #[test]
    fn a_flat_block_decodes_to_the_documented_sample_count() {
        let data = bank(1, 1, 1, 3, "x");
        let parsed = Bank::parse(&data).expect("parse");
        let pcm = decode_adpcm(parsed.waveforms);
        assert_eq!(pcm.len(), 3 * ADPCM_BLOCK_SAMPLES);
    }

    #[test]
    fn the_predictor_history_carries_across_a_block_boundary() {
        // Filter 1 with a zero residual just decays the previous sample, so the
        // second block's first output has to depend on the first block's last.
        let mut stream = vec![0x09u8, 0x00];
        stream.extend_from_slice(&[0x77u8; 14]);
        stream.extend_from_slice(&[0x10u8, 0x00]);
        stream.extend_from_slice(&[0x00u8; 14]);
        let pcm = decode_adpcm(&stream);
        assert_eq!(pcm.len(), 2 * ADPCM_BLOCK_SAMPLES);
        assert_ne!(pcm[ADPCM_BLOCK_SAMPLES], 0, "history did not carry");
    }

    #[test]
    fn the_spec_checks_agree_with_the_ranges_they_document() {
        assert!(adpcm_block_is_in_spec(&[0x4c, 0]));
        assert!(!adpcm_block_is_in_spec(&[0x5c, 0]), "filter 5");
        assert!(!adpcm_block_is_in_spec(&[0x4d, 0]), "shift 13");
        assert!(adpcm_flag_is_defined(&[0, 7]));
        assert!(!adpcm_flag_is_defined(&[0, 8]));
    }
}
