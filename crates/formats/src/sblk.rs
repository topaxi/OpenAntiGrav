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
//! On the PSP, PS2 and Pure the sections abut, the first starts immediately
//! after the table, and the last ends exactly at the end of the file. **Wipeout
//! HD pads**: section 0 starts at 32 rather than at the table's own end of 24,
//! and section 1 starts 0-12 bytes later. HD's own values are all 16-aligned,
//! but the rule enforced here is the weaker one that also admits Pulse's
//! 8-aligned 24 - see [`SECTION_ALIGN`]. The tail is exact everywhere.
//!
//! # Two byte orders
//!
//! The container is byte-swapped **whole**, a `u32` at a time, so on HD the
//! magic arrives as `klBS` and every field reads big-endian. The order is
//! sniffed off the magic ([`byte_order_of`]) rather than passed in, the same
//! way `oag_vex::vex::byte_order` does it; nothing above this module needs to
//! know which disc a blob came from.
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
//! # The waveforms are PS-ADPCM, except where the descriptor says otherwise
//!
//! Sony's 16-byte block format: one predictor/shift byte, one flag byte, and 14
//! bytes holding 28 four-bit residuals. Nothing in the container declares it -
//! it is established by the flag byte, which has to be one of eight values and
//! is on 99.96% of the PSP disc's 530,916 blocks.
//!
//! A **waveform descriptor** does declare it, in the negative:
//! [`NOT_ADPCM_FLAG`]. Every waveform on every PSP and PS2 disc is PS-ADPCM
//! and none sets the bit; about a third of Wipeout HD's do. That third is
//! [`decode_pcm16`] - see its doc comment for the evidence. [`Sound::is_adpcm`]
//! is the predicate, and a caller that ignores it will feed the ADPCM decoder
//! noise.
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
//! # From a name to a sound
//!
//! [`Bank::cue_named`] resolves a cue string the way the runtime does, and
//! [`Bank::cue_sounds`] gives the waveforms that cue's own run of the command
//! table binds. That is the last link between `"SPEEDUPPAD"` and audio; see
//! [`cue`] for the rule and its evidence.
//!
//! # Cues that play cues
//!
//! Some cues bind no waveform of their own and play **other cues** instead.
//! Wipeout HD's `.COLLISIONS` is one: four grains, no key-on, and a tree of
//! `c_CShipShip` and `c_CShipWall` underneath it. [`Bank::cue_tree_sounds`]
//! follows them and [`child`] carries the record and the evidence.
//!
//! # The rate each waveform plays at
//!
//! Decoded, 2026-09-16. The descriptor's `+0x02`/`+0x03` are a centre note and
//! a centre fine-tune, and [`pitch`] is the port of the engine's own arithmetic
//! from those to the `sceSasSetPitch` word - confirmed live, 190 of 190 hits.
//! [`Sound::sample_rate`] is what a player wants; `oag_sound::sfx` plays
//! every waveform at it.
//!
//! # What is not decoded
//!
//! 37 of the 45 command opcodes, and the header's `+0x24`. See the format
//! page's open questions.

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

/// Bytes a bank may carry past the end of its waveform section: none anywhere
/// but Wipeout 2048's `Ship_NGP.bnk`, which carries 16.
pub const TAIL_SLACK: usize = ADPCM_BLOCK_LEN;

/// What a section start has to be a multiple of.
///
/// **Four, and not sixteen** - which is worth stating because HD's own values
/// are all 16-aligned and it is tempting to check that instead. The PSP and PS2
/// pad by nothing at all, and their descriptor section starts at the section
/// table's own end of **24**, which is 8-aligned and not 16-aligned. So a
/// 16-byte check would reject every Pulse and Pure bank while looking like a
/// statement about padding.
///
/// The padding *is* 16-byte alignment where it appears - HD starts section 0 at
/// 32 and section 1 at 0, 4, 8 or 12 bytes past section 0's end - but the rule
/// this module can enforce across the corpus is the weaker one: **4-aligned,
/// and within one [`ADPCM_BLOCK_LEN`] of where the section would sit with no
/// padding at all.**
pub const SECTION_ALIGN: usize = 4;

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
    /// Which end of a multi-byte field comes first in this bank.
    ///
    /// Little on every PSP and PS2 disc; big on Wipeout HD, whose container is
    /// byte-swapped a `u32` at a time - which is why its magic reads `klBS`.
    pub order: ByteOrder,
}

/// Whether `data` looks like a sound bank, without parsing it.
///
/// # The magic is at `0x18`, not at `0`
///
/// Behind the eight-byte container header and the two eight-byte section-table
/// entries. **A scan for `SBlk` at offset 0 finds nothing on any disc**,
/// including the ones this project has decoded 39 banks from - and reading
/// such a scan as "this title has no banks" is how Wipeout Pure was recorded
/// for months as shipping none. See `docs/formats/pure-status.md`.
#[must_use]
pub fn looks_like_bank(data: &[u8]) -> bool {
    byte_order_of(data).is_some()
}

/// Which byte order this blob's `SBlk` container is written in, if either.
///
/// Sniffed rather than passed in, the same way `oag_vex::vex::byte_order`
/// does it, because the magic is a `u32` constant and so tells the two apart
/// by itself: `SBlk` little-endian, `klBS` big-endian.
#[must_use]
pub fn byte_order_of(data: &[u8]) -> Option<ByteOrder> {
    [ByteOrder::Little, ByteOrder::Big]
        .into_iter()
        .find(|&order| looks_like_bank_as(data, order))
}

/// Whether `data` looks like a sound bank read in one particular order.
///
/// The magic's offset is **read out of the section table** rather than assumed
/// to follow it. On the PSP and PS2 those are the same number - the descriptor
/// section starts at the table's own end, 24 - but Wipeout HD pads it to 32,
/// and a check that assumed 24 would reject every HD bank while looking like a
/// statement about the magic.
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
///
/// Wipeout HD stores `SBlk` as `klBS`: the container is byte-swapped whole, a
/// `u32` at a time, which is the same one-word reversal `.fnt` has. So the
/// magic is not a character array that survives the swap - it is a `u32`
/// constant that does not.
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
    /// waveform section is not a whole number of PS-ADPCM blocks. Every one of
    /// those is a statement the file makes about itself, so a failure means this
    /// is not a bank rather than that it is a damaged one.
    pub fn parse(data: &'a [u8]) -> Result<Self> {
        let order = byte_order_of(data).unwrap_or_default();
        Self::parse_as(data, order)
    }

    /// Parses a bank read in a stated byte order.
    ///
    /// [`Bank::parse`] sniffs the order off the magic and is what every caller
    /// wants; this exists so a survey can assert that a blob does *not* parse
    /// the other way round.
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
        // # The framing, and the one place it is not exact
        //
        // On every PSP and PS2 bank there is no slack at all: section 0 starts
        // at the table's end, section 1 starts where section 0 stops, and
        // section 1 ends on the blob.
        //
        // Wipeout HD pads. Section 0 starts at **32** rather than 24, and
        // section 1 starts 0, 4, 8 or 12 bytes after section 0 ends. So the two
        // starts are checked as "aligned, at or after, within one block"
        // instead of "exactly at" - and the alignment is [`SECTION_ALIGN`]'s 4
        // rather than the 16 HD's own values happen to satisfy, because the
        // PSP's section 0 sits at 24 and a 16-byte check would reject it.
        //
        // **The tail is exact on every bank but one**, and deliberately: it
        // holds on all 50 HD banks as well as all 83 Pulse and 29 Pure ones,
        // so relaxing it gives up the one check that says the blob has been
        // read to its end rather than into the middle of something else. The
        // exception is Wipeout 2048's `Ship_NGP.bnk`, which ends 16 bytes
        // (`00 07 00 00` then zeros) past its section 1; the other three 2048
        // banks end exactly. So a tail of at most one block is let through,
        // and `TAIL_SLACK` names the bound.
        let aligned_after = |from: usize, to: u32| {
            let to = to as usize;
            to >= from && to - from < ADPCM_BLOCK_LEN && to.is_multiple_of(SECTION_ALIGN)
        };
        let spans = aligned_after(table_end, sections[0].0)
            && sections[0]
                .0
                .checked_add(sections[0].1)
                .is_some_and(|end| aligned_after(end as usize, sections[1].0))
            && sections[1].0.checked_add(sections[1].1).is_some_and(|end| {
                (end as usize..=end as usize + TAIL_SLACK).contains(&data.len())
            });
        if !spans {
            return Err(Error::BadSections);
        }

        let block = &data[sections[0].0 as usize..][..sections[0].1 as usize];
        let waveforms = &data[sections[1].0 as usize..][..sections[1].1 as usize];

        if block.len() < SBLK_HEADER_LEN || block[..4] != magic_bytes(order) {
            return Err(Error::NotSblk);
        }
        let declared = order.u32(block, 0x28);
        if declared != order.u32(block, 0x2c) || declared != sections[1].1 {
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
            order,
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
        for (index, command) in self
            .commands
            .as_chunks::<COMMAND_LEN>()
            .0
            .iter()
            .enumerate()
        {
            let first = self.order.u32(command, 0);
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
        let entries = self.order.u32(block, 0x08) as usize;
        // The buckets fill the gap between the name block's fixed head and the
        // entry array, so their count follows from the two offsets rather than
        // from the hash's range, which is not known.
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
    /// The opcode that bound it, one of [`KEY_ON_OPCODES`].
    pub opcode: u8,
    /// Offset of the 24-byte descriptor within the descriptor section.
    pub descriptor: u32,
    /// The descriptor's own authored volume, `+0x01`, one signed byte.
    ///
    /// `Scream_OpKeyOn` (`0x0898fc78`) reads this waveform-level byte into the
    /// value it passes as `Scream_PanVolumePair`'s (`0x08995a9c`) other
    /// squared term - the cue's own [`Cue::volume`](super::cue::Cue::volume)
    /// is the first, this is the second. See
    /// `docs/ghidra/functions/psp-pulse-usa/positional-audio.md`'s
    /// "`Scream_PanVolumePair`'s four terms" section.
    ///
    /// **The `-1..=-5` sentinel codes documented there are not decoded
    /// here**, the same gap [`Cue::volume`](super::cue::Cue::volume) carries.
    /// Every one of the 880 key-on descriptors across all 36 banks on the PSP
    /// USA disc reads `60..=127`.
    pub volume: i8,
    /// The descriptor's centre note, `+0x02`, one signed byte.
    ///
    /// The MIDI note the waveform plays at its own rate on, as far as the
    /// engine is concerned - and **negative on every descriptor of every Pulse
    /// and Pure disc**, which routes the pitch through [`pitch::sas_pitch`]'s
    /// scaled branch. See [`pitch`] for the arithmetic and [`Sound::pitch`]
    /// for the result.
    pub centre_note: i8,
    /// The descriptor's centre fine-tune, `+0x03`, in 1/128ths of a semitone
    /// with `127` in tune. `66` on almost every descriptor, `0` on the ones
    /// that play at 48 kHz.
    pub centre_fine: i8,
    /// The descriptor's pitch-bend range downwards, `+0x08`, in semitones.
    ///
    /// `Scream_ComputeVoiceNote` scales a negative bend by this byte and a
    /// positive one by [`Self::bend_up`]; `0` on most descriptors, so a random
    /// bend (`0x1b`) is inaudible on those. See
    /// `docs/ghidra/functions/psp-pulse-usa/sound.md`.
    pub bend_down: i8,
    /// The descriptor's pitch-bend range upwards, `+0x09`, in semitones.
    pub bend_up: i8,
    /// The descriptor's `+0x0e` flags word.
    ///
    /// `Scream_KeyOnVoice` passes `0x40` to `sceSasSetVoice` as its loop mode,
    /// and `0x80` as an argument whose meaning is the **opposite** of what it
    /// looks like - see [`Sound::is_adpcm`]. The rest is unread, so this is
    /// exposed as the raw word rather than as booleans.
    pub mode: u16,
    /// Byte offset of the waveform within [`Bank::waveforms`].
    pub offset: u32,
    /// Length of the waveform in bytes.
    pub length: u32,
    /// The bank's own byte order - [`Sound::pitch`] and [`Sound::sample_rate`]
    /// switch on it, since Wipeout HD's `Scream_KeyOnVoice` walks the same
    /// table on a different scale and a different base rate. See
    /// [`pitch`]'s "Wipeout HD" section.
    pub order: ByteOrder,
}

/// The `+0x0e` bit that selects a looping voice.
pub const LOOP_FLAG: u16 = 0x40;

/// The `+0x0e` bit that marks a waveform as **not** PS-ADPCM.
///
/// The polarity is the reverse of the obvious reading, and this project had it
/// backwards until 2026-08-23. `Scream_KeyOnVoice` passes `(wf+0x0e & 0x80) != 0`
/// as `Sas_QueueSetVoice`'s fifth argument, and that function's whole body for
/// a non-zero fifth argument is:
///
/// ```text
/// printf("SCREAM ERROR: THIS SYSTEM ONLY SUPPORTS ADPCM VOICE DATA!")
/// ```
///
/// So the bit says "this one is not ADPCM" and the PSP complains. Confirmed
/// from the data on four discs: **0 of 916 Pulse PSP spans, 0 of 985 Pulse PS2,
/// 0 of 461 on each Pure pressing** carry it - a build that refuses the bit
/// ships nothing that sets it. And on Wipeout HD, which runs on hardware that
/// can decode more than one codec, the bit predicts the payload exactly: spans
/// with it clear are 100% in PS-ADPCM spec and spans with it set are 0-44%.
///
/// See `docs/formats/psp-audio.md`.
pub const NOT_ADPCM_FLAG: u16 = 0x80;

impl Sound {
    /// Whether this waveform is PS-ADPCM, and so whether [`decode_adpcm`] means
    /// anything for it.
    ///
    /// `false` only on Wipeout HD, whose second voice type is
    /// [`decode_pcm16`] - see [`NOT_ADPCM_FLAG`].
    #[must_use]
    pub fn is_adpcm(&self) -> bool {
        self.mode & NOT_ADPCM_FLAG == 0
    }

    /// Whether the voice loops.
    #[must_use]
    pub fn is_looping(&self) -> bool {
        self.mode & LOOP_FLAG != 0
    }

    /// The pitch word this waveform is keyed on with, played at the default
    /// note with no bend or offset: `0x1000` means "the sample's own rate" on
    /// both platforms this reads (44,100 Hz on the PSP's SAS core, 48,000 Hz
    /// on Wipeout HD's).
    ///
    /// The value the original hands the hardware for an unmodulated play -
    /// `Scream_StartSound`/HD's `Scream_OpKeyOn` start every play at note 60,
    /// and the game's pitch modulation (the engine note, for one) is added on
    /// top by the caller, not by the bank. See [`pitch`], which this switches
    /// on [`Sound::order`]: [`ByteOrder::Little`] is the PSP/PS2/Pure walk,
    /// [`ByteOrder::Big`] is Wipeout HD's own scale.
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
    /// [`Sound::pitch`] scaled onto the platform's own core rate and rounded
    /// to the nearest Hz - 44,100 on the PSP/PS2/Pure, 48,000 on Wipeout HD;
    /// see [`Sound::pitch`]. The PSP disc's banks come out at 11,025, 22,050
    /// and 44,100 exactly and at a spread of others (18,002 for every speech
    /// clip, 15,569 for the circuit ambiences); see [`pitch`] for why "the
    /// authored rate" is not what this is.
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

/// The part of a PS-ADPCM span a voice ever plays: everything up to and
/// including its terminator block.
///
/// **A span is longer than its audio.** The encoder flags the last block it
/// wrote and then appends a block of run-out, so the bytes the command table
/// hands over end one block past the sound - see
/// [`psp-audio.md`](../../../docs/formats/psp-audio.md#the-codecs-own-terminator-agrees),
/// where 595 of 595 spans carry their terminator on the last block or the one
/// before it and none carries one earlier. The hardware stops at the
/// terminator, so the run-out is never heard on a console.
///
/// It is heard here if nothing trims it, and on a **looping** waveform it is
/// heard once per loop rather than once. Measured on `pulse-psp-eu.chd`, over
/// every waveform a Talon's Junction race loads: all 18 looping ones are
/// flagged `[(1, 6), (n-2, 3), (n-1, 255)]` and all 25 one-shots
/// `[(n-2, 1), (n-1, 7)]` - so the run-out is always present, and on a loop it
/// carries `255`, which is not one of the eight defined flag values at all
/// ([`adpcm_flag_is_defined`]). What it decodes to is not silence either:
/// `~hum`'s run-out peaks at 0.474 of full scale and `~FLUID_PUMP`'s at 0.365,
/// against a `~hum` loop period of 128 ms.
///
/// Surveyed across all four PSP images this checkout holds - Pulse EU and USA
/// at 595 spans each, Pure EU and USA at 395 each - **1,980 spans carry their
/// terminator on the last block or the one before it, none earlier and none
/// absent.** The PS2 and PS3 discs were not surveyed for this, which is why
/// the trim only ever looks at those last two blocks: anything else is
/// returned whole rather than acted on.
#[must_use]
pub fn adpcm_played(data: &[u8]) -> &[u8] {
    let blocks = data.as_chunks::<ADPCM_BLOCK_LEN>().0;
    // **Only the last two blocks are looked at**, which is the shape every
    // measured span has and not a general rule about PS-ADPCM. A terminator
    // earlier than that would mean a span longer than the sound by more than
    // its run-out - something no disc surveyed here does - and trimming on one
    // would silently shorten a cue to a tick. Left whole instead, so an
    // unmeasured disc keeps exactly today's behaviour.
    for (index, block) in blocks
        .iter()
        .enumerate()
        .skip(blocks.len().saturating_sub(2))
    {
        // 1 end, 3 loop end, 5 start and end, 7 end and mute - the four
        // defined values with the low bit set. Matched by value rather than by
        // that bit, because the run-out block's own `255` has it set too and
        // is not a terminator.
        if matches!(block[1], 1 | 3 | 5 | 7) {
            return &data[..(index + 1) * ADPCM_BLOCK_LEN];
        }
    }
    data
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

    for block in data.as_chunks::<ADPCM_BLOCK_LEN>().0 {
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

/// Bytes of header at the front of a [`NOT_ADPCM_FLAG`] span, before the
/// sample data starts.
pub const PCM16_HEADER_LEN: usize = 16;

/// Decodes Wipeout HD's second waveform codec: 16-bit PCM, big-endian, behind
/// a 16-byte header.
///
/// # The evidence
///
/// `0x007d0818` in the PS3 executable (`ps3-hdfury-eu/EBOOT.elf`) holds the
/// string `"SCREAM: ERROR! Unknown voice type in bank - must be ADPCM or
/// PCM\n\tSCREAM does not know how to handle this data"` - SCREAM's own error
/// message names exactly two voice types, and [`NOT_ADPCM_FLAG`] already
/// established that a span carrying it is not the first one. The function
/// that owns the string is `CellMs_QueueVoice` (`0x00633c80`), reached
/// through `Scream_OpKeyOn` and `Scream_KeyOnVoice` - PS3 analogues of the
/// same-named PSP functions. Its decompiled body skips a 16-byte header and
/// multiplies a header field by 2 (bytes per sample) exactly when
/// `NOT_ADPCM_FLAG` is set, and walks 16-byte PS-ADPCM blocks for a loop/mute
/// flag when it is clear. See
/// `docs/ghidra/functions/ps3-hdfury-eu/sound.md`'s "`0x01`/`0x09` -
/// `Scream_OpKeyOn`, and the codec dispatch chain" for the full decompile.
///
/// What is measured, over every one of Wipeout HD's 1,167 [`NOT_ADPCM_FLAG`]
/// spans, skipping this header and reading the rest as big-endian `i16`:
///
/// - **Mean roughness (mean absolute sample-to-sample step, divided by the
///   span's RMS) is 0.257**, against about 1.41 for white noise and with only
///   one span over 1.0 at all. Real audio sits far below the noise figure;
///   this corpus does.
/// - A labelled span - the `~SHIELD` cue's two non-ADPCM waveforms in
///   `weapons.bnk` - decodes to a spectrogram of stable horizontal harmonic
///   bands, which is what a sustained shield-hum effect looks like and not
///   what noise looks like.
/// - For the 315 of 1,167 spans whose descriptor also carries
///   [`LOOP_FLAG`](Sound::is_looping), the header's second big-endian `u32`
///   (bytes 4..8) equals the sample count exactly: `header_word * 2 + 16 ==
///   span length`, on every one of those 315. Spans without the loop flag
///   read zero there instead - what that word means when it is not the
///   sample count is not known, and does not need to be to decode the audio,
///   since decoding does not depend on it.
///
/// The other 12 header bytes (0..4 and 8..16) are zero on every span sampled
/// and their meaning, if any, is not known.
///
/// Confidence: 90. Two independent quantitative measurements, a
/// primary-source string and a decompiled call site (`CellMs_QueueVoice`,
/// above) all agree; not runtime-verified and this codec has no PSP or PS2
/// build to corroborate against as a second binary, so capped below the 95
/// band per the confidence rubric.
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
