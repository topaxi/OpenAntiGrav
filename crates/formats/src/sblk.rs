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
//! # Splitting a bank into its sounds
//!
//! [`Bank::sounds`] walks the command table and resolves every command that
//! binds a waveform to the 24-byte descriptor holding its offset and length;
//! [`Bank::sound_names`] walks the name table for the `{name, cue}` pairs. Both
//! rules come out of the PSP executable - see
//! `docs/ghidra/functions/psp-pulse-usa/sound.md` - and both are checked
//! against all 39 banks on the disc, where the spans tile every waveform
//! section exactly.
//!
//! # What is not decoded
//!
//! 43 of the 45 command opcodes, the sample rate each waveform plays at, and
//! the header's `+0x24`. See the format page's open questions.

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

/// Bytes per waveform descriptor in the parameter block.
pub const DESCRIPTOR_LEN: usize = 24;

/// Bytes per name-table entry: a 16-byte name and a `u16` cue index.
pub const NAME_ENTRY_LEN: usize = 0x14;

/// Bytes of name block before the hash buckets.
pub const NAME_BUCKETS_AT: usize = 0x18;

/// The bit of the header word at `+0x08` that says a name table is present.
///
/// `Scream_FindSoundInBank` refuses a bank with it clear before it reads
/// anything, so it is a capability flag rather than part of a size. Both values
/// the disc uses - 772 and 260 - have it set.
pub const HAS_NAME_TABLE: u32 = 0x100;

/// The two opcodes that bind a waveform to a voice.
///
/// Of `g_scream_opcode_table`'s 45 entries, exactly these two point at
/// `Scream_OpKeyOn`. Whether they differ from each other is not known: they
/// share a handler, so any difference has to come out of the command word.
pub const KEY_ON_OPCODES: [u8; 2] = [0x01, 0x09];

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
    /// The header word at `+0x08`, a capability mask. See [`HAS_NAME_TABLE`].
    pub flags: u32,
    /// The whole `SBlk` descriptor section.
    ///
    /// Every offset the header states is an index into this, not into the file:
    /// the cue table's offset is always 64, which is where the header stops.
    pub block: &'a [u8],
    /// The 12-byte cue table.
    pub cues: &'a [u8],
    /// The 8-byte command table.
    pub commands: &'a [u8],
    /// Offset of the parameter block within [`Bank::block`], header `+0x34`.
    pub parameter_offset: u32,
    /// Offset of the name block within [`Bank::block`], header `+0x38`.
    pub name_offset: u32,
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
        if !waveforms.len().is_multiple_of(ADPCM_BLOCK_LEN) {
            return Err(Error::PartialAdpcmBlock {
                size: waveforms.len(),
            });
        }

        let flags = word(block, 0x08);
        let cue_count = half(block, 0x16);
        let command_count = half(block, 0x18);
        let waveform_count = half(block, 0x1a);
        let cue_offset = word(block, 0x1c);
        let command_offset = word(block, 0x20);
        let parameter_offset = word(block, 0x34);
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
            flags,
            block,
            cues,
            commands,
            parameter_offset,
            name_offset,
            waveforms,
        })
    }

    /// PS-ADPCM blocks in the waveform section.
    #[must_use]
    pub fn adpcm_blocks(&self) -> usize {
        self.waveforms.len() / ADPCM_BLOCK_LEN
    }

    /// Every waveform the command table binds, in command order.
    ///
    /// `Scream_OpKeyOn` computes `parameter_block + (command_word & 0xffffff)`
    /// and hands the last two words of the 24-byte record it lands on to
    /// `sceSasSetVoice` as an address and a size, so walking the command table
    /// for the two opcodes that reach that handler yields the bank's waveform
    /// spans without a runtime.
    ///
    /// A command whose descriptor does not fit the descriptor section is
    /// skipped rather than reported: the caller's own count of key-on commands
    /// against this length is the misfit measure, and a silent `None` would
    /// hide it.
    #[must_use]
    pub fn sounds(&self) -> Vec<Sound> {
        let mut out = Vec::new();
        for (index, command) in self.commands.chunks_exact(COMMAND_LEN).enumerate() {
            let first = word(command, 0);
            // The opcode is the high byte of the first word, which is byte 3 in
            // memory: `Scream_StepCommandList` reads `*(u8 *)(cmd + 3)`.
            let opcode = (first >> 24) as u8;
            if !KEY_ON_OPCODES.contains(&opcode) {
                continue;
            }
            let Some(at) = self.parameter_offset.checked_add(first & 0x00ff_ffff) else {
                continue;
            };
            let Some(record) = self
                .block
                .get(at as usize..)
                .and_then(|tail| tail.get(..DESCRIPTOR_LEN))
            else {
                continue;
            };
            out.push(Sound {
                command: index,
                opcode,
                descriptor: at,
                mode: half(record, 0x0e),
                offset: word(record, 0x10),
                length: word(record, 0x14),
            });
        }
        out
    }

    /// Every `{name, cue}` pair in the bank's name table.
    ///
    /// Empty when [`HAS_NAME_TABLE`] is clear in [`Bank::flags`], which is the
    /// gate `Scream_FindSoundInBank` applies before it reads anything.
    ///
    /// The runtime jumps into the entry array at a hash bucket and walks
    /// forward to a NUL-named terminator. Reproducing the lookup would need the
    /// hash; enumerating the table does not, because every bucket's chain is a
    /// run of the same array. This walks all of the buckets' chains, so it
    /// returns exactly the set a lookup could find.
    #[must_use]
    pub fn sound_names(&self) -> Vec<SoundName> {
        if self.flags & HAS_NAME_TABLE == 0 {
            return Vec::new();
        }
        let names = self.name_offset as usize;
        let Some(block) = self.block.get(names..) else {
            return Vec::new();
        };
        if block.len() < NAME_BUCKETS_AT {
            return Vec::new();
        }
        // Relative to the name block, not to the section: the word reads 0x98
        // on all 39 banks even though their name blocks sit at wildly different
        // offsets. The runtime fixes it up to a pointer before using it.
        let entries = word(block, 0x08) as usize;
        // The buckets fill the gap between the name block's fixed head and the
        // entry array, so their count follows from the two offsets rather than
        // from the hash's range, which is not known.
        let Some(bucket_bytes) = entries.checked_sub(NAME_BUCKETS_AT) else {
            return Vec::new();
        };

        let mut out = Vec::new();
        let mut seen = Vec::new();
        for bucket in 0..bucket_bytes / 2 {
            let head = usize::from(half(block, NAME_BUCKETS_AT + bucket * 2));
            let mut at = entries + head * NAME_ENTRY_LEN;
            while let Some(entry) = block.get(at..).and_then(|tail| tail.get(..NAME_ENTRY_LEN)) {
                if entry[0] == 0 {
                    break;
                }
                if !seen.contains(&at) {
                    seen.push(at);
                    let end = entry[..16].iter().position(|&b| b == 0).unwrap_or(16);
                    out.push(SoundName {
                        name: String::from_utf8_lossy(&entry[..end]).into_owned(),
                        cue: half(entry, 0x10),
                    });
                }
                at += NAME_ENTRY_LEN;
            }
        }
        out
    }
}

/// A waveform bound by one command in the bank's command table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sound {
    /// Index of the binding command in the command table.
    pub command: usize,
    /// The opcode that bound it, one of [`KEY_ON_OPCODES`].
    pub opcode: u8,
    /// Offset of the 24-byte descriptor within the descriptor section.
    pub descriptor: u32,
    /// The descriptor's `+0x0e` flags word.
    ///
    /// `Scream_KeyOnVoice` passes `0x40` to `sceSasSetVoice` as its loop mode
    /// and treats `0x80` as an assertion that the data is ADPCM. The rest is
    /// unread, so this is exposed as the raw word rather than as booleans.
    pub mode: u16,
    /// Byte offset of the waveform within [`Bank::waveforms`].
    pub offset: u32,
    /// Length of the waveform in bytes.
    pub length: u32,
}

/// One entry of a bank's name table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SoundName {
    /// The sound's name, up to 16 characters.
    pub name: String,
    /// The cue this name resolves to, an index into the 12-byte cue table.
    pub cue: u16,
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
mod tests;
