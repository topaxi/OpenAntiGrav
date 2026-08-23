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
//! # What this still does not decide
//!
//! **Which** of a cue's waveforms plays. 623 of the 1,282 playable cues bind
//! exactly one and are unambiguous; `.COLLISIONS` binds fifteen. The opcode
//! that chooses between them (`0x19`, whose low operand byte equals the key-ons
//! that follow it in 61 of 87 occurrences and so is *not* a finding) is
//! unread, along with 42 others. [`Bank::cue_sounds`] therefore returns the
//! whole set in command order and leaves the choice to the caller.

use super::{Bank, COMMAND_LEN, CUE_LEN, Sound, half, word};

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
        let raw = word(record, 0x08);
        Some(Cue {
            index,
            // A byte offset into the command table, biased by nothing. The
            // count gates whether it means anything: an empty cue's
            // `0xfffffff8` divides to a nonsense index that nothing reads.
            first_command: raw as usize / COMMAND_LEN,
            commands: usize::from(record[0x04]),
            flags: half(record, 0x06),
            raw,
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
    #[must_use]
    pub fn cue_named(&self, name: &str) -> Option<Cue> {
        self.sound_names()
            .into_iter()
            .find(|entry| entry.name == name)
            .and_then(|entry| self.cue(entry.cue))
    }

    /// The waveforms a cue's commands bind, in command order.
    ///
    /// Empty for a cue that [does not play](Cue::plays) and for one whose
    /// commands are all opcodes that bind nothing - 242 of the 1,282 playable
    /// cues on the two discs, which run some part of the 43 unread opcodes
    /// instead.
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
