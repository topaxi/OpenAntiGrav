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
//! On the PSP, PS2 and Pure the sections abut and the last ends exactly at the
//! end of the file. **Wipeout HD pads**: section 0 starts at 32 rather than at
//! the table's end of 24, and section 1 starts 0-12 bytes later. The rule
//! enforced is the weaker one that also admits Pulse's 8-aligned 24 - see
//! [`SECTION_ALIGN`]. The tail is exact everywhere but 2048's `Ship_NGP.bnk`.
//!
//! # Two byte orders
//!
//! The container is byte-swapped whole, a `u32` at a time, so on HD the magic
//! reads `klBS` and every field is big-endian. The order is sniffed off the
//! magic ([`byte_order_of`]), as `oag_vex::vex::byte_order` does.
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
//! `+0x28`, `+0x2c` and the section table all state the length of section 1.
//! Agreement on all 39 banks says the header is read at the right offsets.
//!
//! # The waveforms are PS-ADPCM, except where the descriptor says otherwise
//!
//! Sony's 16-byte block: a predictor/shift byte, a flag byte, and 14 bytes
//! holding 28 four-bit residuals. Nothing declares it; the flag byte is one of
//! eight values on 99.96% of the PSP disc's 530,916 blocks.
//!
//! [`NOT_ADPCM_FLAG`] declares the exception. No PSP or PS2 waveform sets it;
//! about a third of Wipeout HD's do, and that third is [`decode_pcm16`]. A
//! caller that ignores [`Sound::is_adpcm`] feeds the ADPCM decoder noise.
//!
//! # From a bank to a sound
//!
//! [`Bank::sounds`] resolves every waveform-binding command to its 24-byte
//! descriptor; [`Bank::sound_names`] walks the `{name, cue}` table. Both rules
//! come from the PSP executable (`docs/ghidra/functions/psp-pulse-usa/sound.md`)
//! and tile every waveform section exactly on all 39 banks.
//!
//! [`Bank::cue_named`] resolves a cue string the way the runtime does and
//! [`Bank::cue_sounds`] gives the waveforms its run of the command table binds;
//! see [`cue`]. Some cues bind nothing and play **other cues** (HD's
//! `.COLLISIONS`); [`Bank::cue_tree_sounds`] follows them, see [`child`].
//!
//! # The rate each waveform plays at
//!
//! The descriptor's `+0x02`/`+0x03` are a centre note and fine-tune; [`pitch`]
//! ports the engine's arithmetic from those to the `sceSasSetPitch` word,
//! confirmed live (190 of 190 hits). [`Sound::sample_rate`] is the result.
//!
//! # What is not decoded
//!
//! 37 of the 45 command opcodes, and the header's `+0x24`. See the format
//! page's open questions.

pub const HEADER_LEN: usize = 8;

pub const SECTION_LEN: usize = 8;

pub const SBLK_HEADER_LEN: usize = 64;

pub const MAGIC: &[u8; 4] = b"SBlk";

pub const VERSION: u32 = 3;

pub const ADPCM_BLOCK_LEN: usize = 16;

/// Bytes a bank may carry past the end of its waveform section: none anywhere
/// but Wipeout 2048's `Ship_NGP.bnk`, which carries 16.
pub const TAIL_SLACK: usize = ADPCM_BLOCK_LEN;

/// What a section start has to be a multiple of.
///
/// **Four, not sixteen.** HD's own values are all 16-aligned, but the PSP and
/// PS2 descriptor section starts at 24, which is only 8-aligned, so a 16-byte
/// check would reject every Pulse and Pure bank. The rule enforced across the
/// corpus: **4-aligned, and within one [`ADPCM_BLOCK_LEN`] of where the section
/// would sit with no padding.**
pub const SECTION_ALIGN: usize = 4;

pub const ADPCM_BLOCK_SAMPLES: usize = 28;

pub const CUE_LEN: usize = 12;

pub const COMMAND_LEN: usize = 8;

pub const VOICE_LEN: usize = 16;

pub const DESCRIPTOR_LEN: usize = 24;

pub const NAME_ENTRY_LEN: usize = 0x14;

pub const NAME_BUCKETS_AT: usize = 0x18;

/// The bit of the header word at `+0x08` that says a name table is present.
///
/// `Scream_FindSoundInBank` refuses a bank with it clear before it reads
/// anything. Both values the disc uses, 772 and 260, have it set.
pub const HAS_NAME_TABLE: u32 = 0x100;

/// The two opcodes that bind a waveform to a voice.
///
/// Exactly these two entries of `g_scream_opcode_table` (45) point at
/// `Scream_OpKeyOn`. Any difference between them is in the command word.
pub const KEY_ON_OPCODES: [u8; 2] = [0x01, 0x09];

/// Something wrong with a sound bank.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    TooShort {
        got: usize,
    },
    /// The container version was not [`VERSION`].
    UnsupportedVersion {
        version: u32,
    },
    /// The section count was not 2.
    UnexpectedSectionCount {
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
        size: usize,
    },
    /// A table declared in the header does not fit the descriptor section.
    TableOutOfRange {
        name: &'static str,
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

pub type Result<T> = std::result::Result<T, Error>;

/// A parsed sound bank.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bank<'a> {
    /// The bank's own name, up to 7 characters.
    /// The bank's own name, up to 7 characters.
    ///
    /// An 8-byte field with a forced NUL, so longer names truncate (`basilic`,
    /// `metropi`, `talonsj`). Where short enough, `Data\Sound\<name>.bnk`
    /// hashes to the bank's WAD entry.
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
    /// The whole `SBlk` descriptor section. Every header offset indexes into
    /// this, not the file.
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
    /// The waveform size the section table and descriptor claim, where the file
    /// ships fewer bytes than that (see [`Bank::parse_as`]); [`None`] when they
    /// agree, which is every bank but eight of 2048's `env_*`.
    pub declared_waveform_len: Option<u32>,
    /// Which end of a multi-byte field comes first in this bank.
    /// Little on every PSP and PS2 disc; big on Wipeout HD (byte-swapped a
    /// `u32` at a time, hence `klBS`).
    pub order: ByteOrder,
}

/// Whether `data` looks like a sound bank, without parsing it.
///
/// # The magic is at `0x18`, not at `0`
///
/// Behind the container header and two section-table entries. **A scan for
/// `SBlk` at offset 0 finds nothing on any disc**, and reading that as "no
/// banks" is how Wipeout Pure was recorded for months as shipping none. See
/// `docs/formats/pure-status.md`.
#[must_use]
pub fn looks_like_bank(data: &[u8]) -> bool {
    byte_order_of(data).is_some()
}

/// Which byte order this blob's `SBlk` container is written in, if either.
///
/// Sniffed because the magic is a `u32` constant: `SBlk` little-endian, `klBS`
/// big-endian.
#[must_use]
pub fn byte_order_of(data: &[u8]) -> Option<ByteOrder> {
    [ByteOrder::Little, ByteOrder::Big]
        .into_iter()
        .find(|&order| looks_like_bank_as(data, order))
}

/// Whether `data` looks like a sound bank read in one particular order.
/// Whether `data` looks like a sound bank read in one particular order.
///
/// The magic's offset is **read out of the section table**, not assumed to
/// follow it: HD pads it to 32 where the PSP and PS2 have 24.
#[must_use]
pub fn looks_like_bank_as(data: &[u8], order: ByteOrder) -> bool {
    let table_end = HEADER_LEN + 2 * SECTION_LEN;
    if data.len() < table_end || order.u32(data, 0) != VERSION {
        return false;
    }
    let magic_at = order.u32(data, HEADER_LEN) as usize;
    magic_at >= table_end
        && data
            .get(magic_at..magic_at + 4)
            .is_some_and(|found| found == magic_bytes(order))
}

/// The magic as it appears in a file of this byte order.
/// The magic as it appears in a file of this byte order.
///
/// A `u32` constant, so HD's whole-word byte swap reverses it to `klBS`, as
/// for `.fnt`.
#[must_use]
pub fn magic_bytes(order: ByteOrder) -> [u8; 4] {
    let mut out = *MAGIC;
    if order == ByteOrder::Big {
        out.reverse();
    }
    out
}

impl<'a> Bank<'a> {
    /// Parses a bank.
    ///
    /// # Errors
    ///
    /// Fails when the container framing does not close exactly, the descriptor
    /// section is not an `SBlk` block, the two waveform sizes disagree, or the
    /// waveform section is not a whole number of PS-ADPCM blocks. Each is a
    /// statement the file makes about itself, so a failure means this is not a
    /// bank.
    pub fn parse(data: &'a [u8]) -> Result<Self> {
        let order = byte_order_of(data).unwrap_or_default();
        Self::parse_as(data, order)
    }

    /// Parses a bank read in a stated byte order.
    ///
    /// [`Bank::parse`] sniffs the order; this exists so a survey can assert a
    /// blob does *not* parse the other way round.
    ///
    /// # Errors
    ///
    /// The same set [`Bank::parse`] raises.
    pub fn parse_as(data: &'a [u8], order: ByteOrder) -> Result<Self> {
        if data.len() < HEADER_LEN {
            return Err(Error::TooShort { got: data.len() });
        }
        let version = order.u32(data, 0);
        if version != VERSION {
            return Err(Error::UnsupportedVersion { version });
        }
        let count = order.u32(data, 4);
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
            *section = (order.u32(data, at), order.u32(data, at + 4));
        }
        // Every PSP and PS2 bank has no slack: section 0 starts at the table's
        // end, section 1 where section 0 stops, and ends on the blob. HD pads,
        // so both starts are checked as "aligned, at or after, within one
        // block", with [`SECTION_ALIGN`]'s 4 rather than HD's 16 (the PSP's 24
        // is not 16-aligned).
        //
        // The tail stays exact (all 50 HD, 83 Pulse and 29 Pure banks): it is
        // the one check that the blob was read to its end. The exception is
        // 2048's `Ship_NGP.bnk`, 16 bytes (`00 07 00 00` then zeros) past
        // section 1, so up to one block is let through (`TAIL_SLACK`).
        //
        // **A waveform section may declare more than the file ships**: eight of
        // 2048's `env_*.bnk` (`env_altima`, `env_arena`, `env_bridge`,
        // `env_cathedral`, `env_subway`, `env_tower`, `env_sol`, `env_square`)
        // carry the section table's size and the descriptor's `+0x28`/`+0x2c`
        // in agreement, and all three are larger than the bytes behind them.
        // Every waveform those banks bind ends inside what is shipped, the last
        // at the file's end, so the size is a stale field and not a section the
        // file lacks: the section is then what the file holds, and
        // `declared_waveform_len` keeps the claim.
        let shipped = data.len().saturating_sub(sections[1].0 as usize);
        let overdeclared = sections[1].0 as usize <= data.len()
            && sections[1].1 as usize > shipped
            && shipped.is_multiple_of(ADPCM_BLOCK_LEN);
        let aligned_after = |from: usize, to: u32| {
            let to = to as usize;
            to >= from && to - from < ADPCM_BLOCK_LEN && to.is_multiple_of(SECTION_ALIGN)
        };
        let spans = aligned_after(table_end, sections[0].0)
            && sections[0]
                .0
                .checked_add(sections[0].1)
                .is_some_and(|end| aligned_after(end as usize, sections[1].0))
            && (overdeclared
                || sections[1].0.checked_add(sections[1].1).is_some_and(|end| {
                    (end as usize..=end as usize + TAIL_SLACK).contains(&data.len())
                }));
        if !spans {
            return Err(Error::BadSections);
        }
        let declared_len = sections[1].1;
        if overdeclared {
            sections[1].1 = shipped as u32;
        }

        let declared_waveform_len = overdeclared.then_some(declared_len);
        let block = &data[sections[0].0 as usize..][..sections[0].1 as usize];
        let waveforms = &data[sections[1].0 as usize..][..sections[1].1 as usize];

        if block.len() < SBLK_HEADER_LEN || block[..4] != magic_bytes(order) {
            return Err(Error::NotSblk);
        }
        let declared = order.u32(block, 0x28);
        if declared != order.u32(block, 0x2c) || declared != declared_len {
            return Err(Error::SizeDisagreement {
                declared,
                section: declared_len,
            });
        }
        if !waveforms.len().is_multiple_of(ADPCM_BLOCK_LEN) {
            return Err(Error::PartialAdpcmBlock {
                size: waveforms.len(),
            });
        }

        let flags = order.u32(block, 0x08);
        let cue_count = order.u16(block, 0x16);
        let command_count = order.u16(block, 0x18);
        let waveform_count = order.u16(block, 0x1a);
        let cue_offset = order.u32(block, 0x1c);
        let command_offset = order.u32(block, 0x20);
        let parameter_offset = order.u32(block, 0x34);
        let name_offset = order.u32(block, 0x38);

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

        let bank = Self {
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
            declared_waveform_len,
            order,
        };
        // A short file is only the stale size field this reads it as when no
        // waveform it binds is cut off; a truncated download still is refused.
        if bank.declared_waveform_len.is_some()
            && bank
                .sounds()
                .iter()
                .any(|s| u64::from(s.offset) + u64::from(s.length) > waveforms.len() as u64)
        {
            return Err(Error::BadSections);
        }
        Ok(bank)
    }

    /// PS-ADPCM blocks in the waveform section.
    #[must_use]
    pub fn adpcm_blocks(&self) -> usize {
        self.waveforms.len() / ADPCM_BLOCK_LEN
    }

    /// Every waveform the command table binds, in command order.
    ///
    /// `Scream_OpKeyOn` computes `parameter_block + (command_word & 0xffffff)`
    /// and hands the last two words of the 24-byte record to `sceSasSetVoice`
    /// as an address and a size. A command whose descriptor does not fit the
    /// section is skipped, not reported: the caller's count of key-on commands
    /// against this length is the misfit measure.
    #[must_use]
    pub fn sounds(&self) -> Vec<Sound> {
        let mut out = Vec::new();
        for (index, command) in self
            .commands
            .as_chunks::<COMMAND_LEN>()
            .0
            .iter()
            .enumerate()
        {
            let first = self.order.u32(command, 0);
            // The opcode is the high byte of the first word (`cmd + 3`), as
            // `Scream_StepCommandList` reads it.
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
                volume: record[0x01] as i8,
                centre_note: record[0x02] as i8,
                centre_fine: record[0x03] as i8,
                bend_down: record[0x08] as i8,
                bend_up: record[0x09] as i8,
                mode: self.order.u16(record, 0x0e),
                offset: self.order.u32(record, 0x10),
                length: self.order.u32(record, 0x14),
                order: self.order,
            });
        }
        out
    }

    /// Every `{name, cue}` pair in the bank's name table.
    ///
    /// Empty when [`HAS_NAME_TABLE`] is clear in [`Bank::flags`], the gate
    /// `Scream_FindSoundInBank` applies. The runtime jumps into the entry array
    /// at a hash bucket; this walks every bucket's chain instead, so it needs no
    /// hash and returns exactly the set a lookup could find.
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
        // Relative to the name block, not the section: reads 0x98 on all 39
        // banks whatever the block's offset. The runtime fixes it up to a pointer.
        let entries = self.order.u32(block, 0x08) as usize;
        // The bucket count follows from the two offsets, not from the hash's
        // range, which is not known.
        let Some(bucket_bytes) = entries.checked_sub(NAME_BUCKETS_AT) else {
            return Vec::new();
        };

        let mut out = Vec::new();
        let mut seen = Vec::new();
        for bucket in 0..bucket_bytes / 2 {
            let head = usize::from(self.order.u16(block, NAME_BUCKETS_AT + bucket * 2));
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
                        cue: self.order.u16(entry, 0x10),
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
    pub opcode: u8,
    /// Offset of the 24-byte descriptor within the descriptor section.
    pub descriptor: u32,
    /// The descriptor's own authored volume, `+0x01`, one signed byte.
    ///
    /// `Scream_OpKeyOn` (`0x0898fc78`) feeds it to `Scream_PanVolumePair`
    /// (`0x08995a9c`) as the second squared term; the cue's
    /// [`Cue::volume`](super::cue::Cue::volume) is the first. See
    /// `docs/ghidra/functions/psp-pulse-usa/positional-audio.md`'s
    /// "`Scream_PanVolumePair`'s four terms". **The `-1..=-5` sentinel codes
    /// documented there are not decoded**, as for `Cue::volume`. All 880
    /// key-on descriptors on the 36 PSP USA banks read `60..=127`.
    pub volume: i8,
    /// The descriptor's centre note, `+0x02`, one signed byte.
    ///
    /// **Negative on every descriptor of every Pulse and Pure disc**, which
    /// routes the pitch through [`pitch::sas_pitch`]'s scaled branch. See
    /// [`pitch`] and [`Sound::pitch`].
    pub centre_note: i8,
    /// The descriptor's centre fine-tune, `+0x03`, in 1/128ths of a semitone
    /// with `127` in tune. `66` on almost every descriptor, `0` on the ones
    /// that play at 48 kHz.
    pub centre_fine: i8,
    /// The descriptor's pitch-bend range downwards, `+0x08`, in semitones.
    ///
    /// `Scream_ComputeVoiceNote` scales a negative bend by this and a positive
    /// one by [`Self::bend_up`]; `0` on most descriptors, so the random bend
    /// (`0x1b`) is inaudible there. See
    /// `docs/ghidra/functions/psp-pulse-usa/sound.md`.
    pub bend_down: i8,
    /// The descriptor's pitch-bend range upwards, `+0x09`, in semitones.
    pub bend_up: i8,
    /// The descriptor's `+0x0e` flags word.
    ///
    /// `Scream_KeyOnVoice` passes `0x40` to `sceSasSetVoice` as its loop mode
    /// and `0x80` as an argument with the **opposite** meaning to what it looks
    /// like - see [`Sound::is_adpcm`]. The rest is unread, so the raw word.
    pub mode: u16,
    /// Byte offset of the waveform within [`Bank::waveforms`].
    pub offset: u32,
    pub length: u32,
    /// The bank's byte order. [`Sound::pitch`] and [`Sound::sample_rate`] switch
    /// on it: HD's `Scream_KeyOnVoice` uses a different scale and base rate
    /// (see [`pitch`]'s "Wipeout HD" section).
    pub order: ByteOrder,
}

/// The `+0x0e` bit that selects a looping voice.
pub const LOOP_FLAG: u16 = 0x40;

/// The `+0x0e` bit that marks a waveform as **not** PS-ADPCM.
///
/// The polarity is the reverse of the obvious reading. `Scream_KeyOnVoice`
/// passes `(wf+0x0e & 0x80) != 0` as `Sas_QueueSetVoice`'s fifth argument, and
/// for a non-zero value that function only prints
/// `SCREAM ERROR: THIS SYSTEM ONLY SUPPORTS ADPCM VOICE DATA!`. Confirmed from
/// the data: **0 of 916 Pulse PSP spans, 0 of 985 Pulse PS2, 0 of 461 on each
/// Pure pressing** carry it. On Wipeout HD the bit predicts the payload
/// exactly: spans with it clear are 100% in PS-ADPCM spec, spans with it set
/// 0-44%.
pub const NOT_ADPCM_FLAG: u16 = 0x80;

impl Sound {
    /// Whether this waveform is PS-ADPCM, so whether [`decode_adpcm`] applies.
    /// `false` only on Wipeout HD, whose other voice type is [`decode_pcm16`].
    #[must_use]
    pub fn is_adpcm(&self) -> bool {
        self.mode & NOT_ADPCM_FLAG == 0
    }

    /// Whether the voice loops.
    #[must_use]
    pub fn is_looping(&self) -> bool {
        self.mode & LOOP_FLAG != 0
    }

    /// The pitch word this waveform is keyed on with, at the default note with
    /// no bend or offset: `0x1000` is "the sample's own rate" (44,100 Hz on the
    /// PSP's SAS core, 48,000 Hz on HD's).
    ///
    /// The original starts every play at note 60 and the caller adds pitch
    /// modulation (the engine note) on top. Switches on [`Sound::order`]:
    /// [`ByteOrder::Little`] is the PSP/PS2/Pure walk, [`ByteOrder::Big`] HD's
    /// own scale; see [`pitch`].
    #[must_use]
    pub fn pitch(&self) -> u16 {
        match self.order {
            ByteOrder::Big => pitch::sas_pitch_scaled(
                self.centre_note,
                self.centre_fine,
                pitch::DEFAULT_NOTE,
                0,
                pitch::HD_NEGATIVE_CENTRE_SCALE,
            ),
            ByteOrder::Little => {
                pitch::sas_pitch(self.centre_note, self.centre_fine, pitch::DEFAULT_NOTE, 0)
            }
        }
    }

    /// The rate this waveform plays at, in Hz, at the default note.
    ///
    /// [`Sound::pitch`] scaled onto the platform's core rate and rounded. The
    /// PSP banks give 11,025, 22,050 and 44,100 exactly plus a spread of others
    /// (18,002 for every speech clip, 15,569 for the circuit ambiences); see
    /// [`pitch`] for why that is not "the authored rate".
    #[must_use]
    pub fn sample_rate(&self) -> u32 {
        match self.order {
            ByteOrder::Big => pitch::sample_rate_hz_at(self.pitch(), pitch::HD_SAMPLE_RATE),
            ByteOrder::Little => pitch::sample_rate_hz(self.pitch()),
        }
    }
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
/// Sony's published coefficients, over 64. A filter above 4 is out of spec;
/// the hardware falls back to the flat one and so does this.
const FILTERS: [(i32, i32); 5] = [(0, 0), (60, 0), (115, -52), (98, -55), (122, -60)];

/// Whether a PS-ADPCM block's predictor and shift are in range.
///
/// The predictor is the high nibble of byte 0 and selects one of five filters;
/// the shift is the low nibble and the hardware clamps it at 12. Both in range
/// on essentially every block is the evidence a blob is PS-ADPCM at all.
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

/// The part of a PS-ADPCM span a voice ever plays: everything up to and
/// including its terminator block.
///
/// **A span is longer than its audio.** The encoder flags the last block it
/// wrote and then appends a block of run-out, so a span ends one block past
/// the sound: 595 of 595 spans carry their terminator on the last block or the
/// one before it and none earlier (see
/// [`psp-audio.md`](../../../docs/formats/psp-audio.md#the-codecs-own-terminator-agrees)).
/// The hardware stops at the terminator; here the run-out would be heard, on a
/// loop once per loop.
///
/// Measured on `pulse-psp-eu.chd` over every waveform a Talon's Junction race
/// loads: all 18 looping ones are flagged `[(1, 6), (n-2, 3), (n-1, 255)]` and
/// all 25 one-shots `[(n-2, 1), (n-1, 7)]`. The loop's run-out carries `255`,
/// not one of the eight defined flags ([`adpcm_flag_is_defined`]), and is not
/// silent: `~hum`'s peaks at 0.474 of full scale, `~FLUID_PUMP`'s at 0.365,
/// against a `~hum` loop period of 128 ms.
///
/// Across all four PSP images here (Pulse EU/USA 595 spans each, Pure EU/USA
/// 395 each) **1,980 spans carry their terminator on the last block or the one
/// before it, none earlier and none absent.** The PS2 and PS3 discs were not
/// surveyed, so only those last two blocks are looked at; anything else is
/// returned whole.
#[must_use]
pub fn adpcm_played(data: &[u8]) -> &[u8] {
    let blocks = data.as_chunks::<ADPCM_BLOCK_LEN>().0;
    // Only the last two blocks are looked at: the shape every measured span
    // has, not a general PS-ADPCM rule. Acting on an earlier terminator could
    // shorten a cue to a tick on an unmeasured disc.
    for (index, block) in blocks
        .iter()
        .enumerate()
        .skip(blocks.len().saturating_sub(2))
    {
        // 1, 3, 5 and 7: the defined values with the low bit set. Matched by
        // value because the run-out block's own `255` has the bit set too and is
        // not a terminator.
        if matches!(block[1], 1 | 3 | 5 | 7) {
            return &data[..(index + 1) * ADPCM_BLOCK_LEN];
        }
    }
    data
}

/// Decodes PS-ADPCM into 16-bit samples.
///
/// `data` is a whole number of 16-byte blocks; a trailing partial block is
/// ignored. Predictor history starts at zero, so a span decoded from the
/// middle of a bank starts with a short transient.
#[must_use]
pub fn decode_adpcm(data: &[u8]) -> Vec<i16> {
    let mut out = Vec::with_capacity(data.len() / ADPCM_BLOCK_LEN * ADPCM_BLOCK_SAMPLES);
    let mut history = (0i32, 0i32);

    for block in data.as_chunks::<ADPCM_BLOCK_LEN>().0 {
        let mut shift = u32::from(block[0] & 0x0f);
        let filter = usize::from(block[0] >> 4);
        // Out of spec on 224 of 530,916 blocks; the hardware does not fault,
        // so neither does this.
        if shift > 12 {
            shift = 9;
        }
        let (f0, f1) = FILTERS[filter.min(FILTERS.len() - 1)];

        for &byte in &block[2..] {
            for nibble in [byte & 0x0f, byte >> 4] {
                // Sign-extend the 4-bit residual from the top of a 16-bit word,
                // where the shift applies.
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

pub const PCM16_HEADER_LEN: usize = 16;

/// Decodes Wipeout HD's second waveform codec: 16-bit PCM, big-endian, behind
/// a 16-byte header.
///
/// # The evidence
///
/// `0x007d0818` in the PS3 executable (`ps3-hdfury-eu/EBOOT.elf`) holds
/// `"SCREAM: ERROR! Unknown voice type in bank - must be ADPCM or PCM"`, which
/// names exactly two voice types. Its owner is `CellMs_QueueVoice`
/// (`0x00633c80`), reached through `Scream_OpKeyOn` and `Scream_KeyOnVoice`. Its
/// body skips a 16-byte header and multiplies a header field by 2 (bytes per
/// sample) exactly when `NOT_ADPCM_FLAG` is set, and walks 16-byte PS-ADPCM
/// blocks for a loop/mute flag when clear. See
/// `docs/ghidra/functions/ps3-hdfury-eu/sound.md`'s "`0x01`/`0x09` -
/// `Scream_OpKeyOn`, and the codec dispatch chain".
///
/// Measured over all 1,167 [`NOT_ADPCM_FLAG`] spans, header skipped, rest read
/// as big-endian `i16`:
///
/// - **Mean roughness (mean absolute step over the span's RMS) is 0.257**,
///   against about 1.41 for white noise; one span is over 1.0.
/// - `~SHIELD`'s two non-ADPCM waveforms in `weapons.bnk` decode to stable
///   horizontal harmonic bands, a sustained hum rather than noise.
/// - On the 315 spans with [`LOOP_FLAG`](Sound::is_looping), the header's
///   second big-endian `u32` (bytes 4..8) equals the sample count:
///   `header_word * 2 + 16 == span length`. Without the loop flag it reads zero;
///   decoding does not depend on it.
///
/// The other 12 header bytes (0..4 and 8..16) are zero on every span sampled;
/// meaning unknown.
///
/// Confidence: 90. Two quantitative measurements, a primary-source string and a
/// decompiled call site agree; not runtime-verified and no PSP or PS2 build
/// corroborates, so capped below 95.
#[must_use]
pub fn decode_pcm16(data: &[u8]) -> Vec<i16> {
    let body = data.get(PCM16_HEADER_LEN..).unwrap_or(&[]);
    body.as_chunks::<2>()
        .0
        .iter()
        .map(|b| i16::from_be_bytes(*b))
        .collect()
}

use crate::byte_order::ByteOrder;

pub mod child;
pub mod cue;
pub mod pitch;
pub mod runner;
pub mod timeline;
pub use child::Child;
pub use cue::Cue;

#[cfg(test)]
mod tests;
