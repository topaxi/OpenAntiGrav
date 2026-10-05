//! The last link: from a cue to the waveforms it plays.
//!
//! [`Bank::sounds`](super::Bank::sounds) walks the *whole* command table, which
//! answers "what waveforms does this bank hold" but not "what does
//! `SPEEDUPPAD` sound like". A cue owns a **contiguous run** of that table, and
//! this module is the arithmetic that finds the run.
//!
//! # The rule
//!
//! ```text
//! first command = *(u32 *)(cue + 0x08) / 8
//! command count =  *(u8 *)(cue + 0x04)
//! ```
//!
//! `Scream_StepCommandList` (`0x0898efd8`) reads the cue's `+0x08` as a
//! *pointer* and steps it by 8 bytes per command, and ends the list when the
//! program counter passes `*(i8 *)(cue + 4) - 1`. SCREAM fixes the field up
//! from a file offset to a pointer at load time, so on disc it is a **byte
//! offset into the command table, biased by nothing** - which is why the
//! division by [`COMMAND_LEN`](super::COMMAND_LEN) is the whole of it.
//!
//! That base was picked by enumeration rather than by reading the fixup: seven
//! candidate readings were run against every bank on both discs and this is the
//! only one that lands every cue inside the command table. See
//! `docs/formats/psp-audio.md`.
//!
//! # Why the runs tiling is the evidence
//!
//! The cues of a bank partition its command table **exactly** - no gap, no
//! overlap, ending on the last command - on all 83 banks across the PSP and PS2
//! discs. Nothing in the format states that, and a wrong base or a wrong stride
//! cannot produce it.
//!
//! The five exceptions on the two discs are cues whose count is zero, whose
//! `+0x08` reads `0xfffffff8`. Those are the cues `Scream_StartSound`
//! (`0x0898f864`) refuses to play at all - its `cue + 0x04` must be non-zero -
//! so they are excluded by the runtime's own gate rather than by a relaxation
//! invented here. See [`Cue::plays`].
//!
//! # What this still returns the whole set for
//!
//! **Which** of a cue's waveforms plays. 623 of the 1,282 playable cues bind
//! exactly one and are unambiguous; `.COLLISIONS` binds fifteen. The opcode
//! that chooses between them (`0x19`) is decoded and corroborated on both HD
//! and PSP - see `docs/ghidra/functions/psp-pulse-usa/sound.md`'s
//! `Scream_OpAlternate` - and it is a random draw, per play, that never
//! repeats the immediately previous pick. `oag_sound::sfx::Banks::pick`
//! implements that choice; [`Bank::cue_sounds`] still returns the whole set
//! in command order, because the decoded opcode is a *runtime* draw, not
//! something a static WAD parse can resolve to one waveform - the choice
//! stays the caller's to make, on every play, not this format layer's to
//! make once.

use super::{Bank, COMMAND_LEN, CUE_LEN, Sound};

/// The descriptor block's version word on a Wipeout 2048 bank.
const HASHED_VERSION: u32 = 5;

/// Where a hashed name table's first record sits in the name block.
const HASHED_FIRST: usize = 0x14;

/// Bytes in one hashed name record.
const HASHED_ENTRY_LEN: usize = 16;

/// The hash a Vita bank's name table is keyed by: FNV-1, **seeded with zero**.
///
/// `h = h * 0x01000193 ^ byte` per byte, from `h = 0`, not FNV's offset basis.
/// `FUN_81352b9c` on the v1.04 executable hashes the name this way and hands the
/// result to its table search. The 32 names of the engine tables and the
/// weapons bank checked against `shipHD.bnk`, `Ship_NGP_Zone.bnk` and
/// `Weapons_NGP.bnk` all land on a record whose cue is the one the
/// `xfship_*.xfx` layers play.
#[must_use]
pub fn name_hash(name: &str) -> u32 {
    name.bytes()
        .fold(0u32, |h, b| h.wrapping_mul(0x0100_0193) ^ u32::from(b))
}

/// A cue: one playable sound, and the run of commands that plays it.
///
/// Reached by index from the name table ([`Bank::sound_names`]) or by name
/// through [`Bank::cue_named`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cue {
    /// Index into the bank's 12-byte cue table.
    pub index: u16,
    /// Index of this cue's first command in the command table.
    pub first_command: usize,
    /// How many commands the cue owns, from `+0x04`.
    ///
    /// Zero means the runtime will not play it - see [`Cue::plays`].
    pub commands: usize,
    /// The cue's `+0x06` flags word.
    ///
    /// `0x4000` is set on every play and `0x2` triggers a preload, per
    /// `Scream_StartSound`; the rest is unread, so this is the raw word.
    pub flags: u16,
    /// The cue's `+0x08` as stored, before the division.
    ///
    /// Kept so a survey can see the `0xfffffff8` an empty cue carries without
    /// re-reading the table.
    pub raw: u32,
    /// The cue's own authored volume, `+0x00`, one signed byte.
    ///
    /// `Scream_StartSound` (`0x0898f864`) reads this as the fallback for the
    /// voice's `+0xc` field - the term `Scream_PanVolumePair`
    /// (`0x08995a9c`) squares - whenever its caller passes `-1` for "no
    /// override", which is what every traced call site does (confirmed live,
    /// PPSSPP breakpoint on `Scream_StartSound`'s entry, 20/20 hits reading
    /// exactly `-1`; a second breakpoint at the voice struct's own `+0xc`
    /// read back an exact match against this byte on every hit taken). See
    /// `docs/ghidra/functions/psp-pulse-usa/positional-audio.md`'s
    /// "`Scream_PanVolumePair`'s four terms" section.
    ///
    /// **The `-1..=-5` sentinel codes documented there are not decoded here.**
    /// Every one of the 582 cues across all 36 banks on the PSP USA disc reads
    /// `20..=127`, so the gap is real but unexercised by this corpus - a bank
    /// that used a sentinel would read this field wrong rather than erroring.
    pub volume: i8,
}

impl Cue {
    /// Whether `Scream_StartSound` would play this cue at all.
    ///
    /// Its first check after the bounds test is that `cue + 0x04` is non-zero,
    /// and `Scream_StepCommandList` ends the list at `count - 1`, so a
    /// zero-count cue runs no commands. Five cues on the two discs are like
    /// this and every one of them stores `0xfffffff8` at `+0x08`.
    #[must_use]
    pub fn plays(&self) -> bool {
        self.commands > 0
    }

    /// The half-open range of command indices this cue owns.
    #[must_use]
    pub fn range(&self) -> std::ops::Range<usize> {
        self.first_command..self.first_command + self.commands
    }
}

impl Bank<'_> {
    /// The cue at `index`, or `None` past [`Bank::cue_count`](Bank).
    ///
    /// The run is **not** bounds-checked against the command table here: a cue
    /// that overruns is a real thing to be able to see, and
    /// [`Bank::cue_sounds`] filters rather than trusting it. Every cue on both
    /// discs is in range.
    #[must_use]
    pub fn cue(&self, index: u16) -> Option<Cue> {
        let at = usize::from(index) * CUE_LEN;
        let record = self.cues.get(at..)?.get(..CUE_LEN)?;
        let raw = self.order.u32(record, 0x08);
        Some(Cue {
            index,
            // A byte offset into the command table, biased by nothing. The
            // count gates whether it means anything: an empty cue's
            // `0xfffffff8` divides to a nonsense index that nothing reads.
            first_command: raw as usize / COMMAND_LEN,
            commands: usize::from(record[0x04]),
            flags: self.order.u16(record, 0x06),
            raw,
            volume: record[0x00] as i8,
        })
    }

    /// Every cue in the bank, in table order.
    #[must_use]
    pub fn cues(&self) -> Vec<Cue> {
        (0..self.cue_count).filter_map(|i| self.cue(i)).collect()
    }

    /// The cue a name resolves to, exactly as `Scream_FindSoundInBank` would.
    ///
    /// The comparison is the runtime's: a 16-byte `memcmp`, so the name is
    /// matched whole and **`"COLLISIONS"` does not find `".COLLISIONS"`**. The
    /// leading dot marks a child sound in SCREAM's own error strings; nothing
    /// here strips it, because doing so would resolve a name the original
    /// would have rejected.
    ///
    /// A Vita bank ([`Bank::is_hashed`]) keys its table by [`name_hash`]
    /// instead and keeps no 16-byte names at all, so this finds nothing there
    /// on purpose: [`Bank::cue_named_or_hashed`] is the lookup for the one
    /// caller whose hashed binding is measured. Resolving every 2048 cue by
    /// hash played cues whose triggers were never checked (a perfect-lap
    /// announcement mid-lap, and noise), reported from play on 2026-10-05.
    #[must_use]
    pub fn cue_named(&self, name: &str) -> Option<Cue> {
        self.sound_names()
            .into_iter()
            .find(|entry| entry.name == name)
            .and_then(|entry| self.cue(entry.cue))
    }

    /// [`Bank::cue_named`], and on a [hashed](Bank::is_hashed) bank the cue
    /// whose name hashes to `name` by [`name_hash`].
    ///
    /// Only the crossfade engine uses this, where the hashed names are checked
    /// against the cues the `xfship_*.xfx` layers play.
    #[must_use]
    pub fn cue_named_or_hashed(&self, name: &str) -> Option<Cue> {
        if self.is_hashed() {
            return self.cue_by_hash(name_hash(name)).and_then(|c| self.cue(c));
        }
        self.cue_named(name)
    }

    /// Whether the name table is keyed by [`name_hash`] rather than by name.
    ///
    /// True when the descriptor block's own version word (`+0x04`) is `5`, which
    /// is every Wipeout 2048 bank (`Ship_NGP`, `Ship_NGP_Zone`, `shipHD`,
    /// `Weapons_NGP`, all four measured) and no bank on a PSP, PS2 or PS3 disc,
    /// where it is `3`.
    #[must_use]
    pub fn is_hashed(&self) -> bool {
        self.order.u32(self.block, 4) == HASHED_VERSION
    }

    /// The cue whose name hashes to `hash`, on a [hashed](Bank::is_hashed) bank.
    ///
    /// The table is [`Bank::cue_count`] records of [`HASHED_ENTRY_LEN`] bytes
    /// from `+0x14` of the name block: `{u32 hash, u16 cue, u16 next, u32, u32}`.
    /// The scan is linear and ignores `next`, the hash-collision chain: an
    /// exact 32-bit match is the lookup, and the chain only matters to a
    /// runtime that probes a bucket.
    #[must_use]
    pub fn cue_by_hash(&self, hash: u32) -> Option<u16> {
        if !self.is_hashed() || self.flags & super::HAS_NAME_TABLE == 0 {
            return None;
        }
        let block = self.block.get(self.name_offset as usize..)?;
        (0..usize::from(self.cue_count)).find_map(|n| {
            let at = HASHED_FIRST + n * HASHED_ENTRY_LEN;
            let entry = block.get(at..at + HASHED_ENTRY_LEN)?;
            let cue = self.order.u16(entry, 4);
            (self.order.u32(entry, 0) == hash && cue < self.cue_count).then_some(cue)
        })
    }

    /// The waveforms a cue's commands bind, in command order.
    ///
    /// Empty for a cue that [does not play](Cue::plays) and for one whose
    /// commands are all opcodes that bind nothing - 242 of the 1,282 playable
    /// cues on the two discs, which run some part of the 41 unread opcodes
    /// instead. Some of those bind a waveform one level down, by playing
    /// another cue; [`Bank::cue_tree_sounds`] is the walk that follows them.
    ///
    /// **All of them, not the one that would sound.** See the module docs: the
    /// selection opcode is not decoded, so returning a set is the honest shape
    /// and picking from it is the caller's decision to document.
    #[must_use]
    pub fn cue_sounds(&self, cue: &Cue) -> Vec<Sound> {
        if !cue.plays() {
            return Vec::new();
        }
        let range = cue.range();
        self.sounds()
            .into_iter()
            .filter(|sound| range.contains(&sound.command))
            .collect()
    }

    /// A waveform's PS-ADPCM bytes, or `None` if its span leaves the section.
    #[must_use]
    pub fn waveform(&self, sound: &Sound) -> Option<&[u8]> {
        self.waveforms
            .get(sound.offset as usize..)?
            .get(..sound.length as usize)
    }
}
