//! Cues that play other cues: the grains behind `.COLLISIONS`.
//!
//! [`Bank::cue_sounds`](super::Bank::cue_sounds) answers "what waveforms does
//! this cue's own run of the command table bind", and for most cues that is the
//! whole story. For some it is empty, and the reason is not that the cue is
//! broken: it binds no waveform *directly* because its grains play **other
//! cues**, which bind the waveforms.
//!
//! Wipeout HD's `.COLLISIONS` is the case that forced this. Four grains, no
//! key-on, and this project recorded it for a day as "all four commands are
//! among the 43 unread opcodes" - which was true and was also a dead end
//! wearing the shape of a finding.
//!
//! # The record
//!
//! Two opcodes, [`CHILD_OPCODES`], carry a 24-bit operand that is an offset
//! from the bank's parameter block, exactly as a key-on's is. It lands on a
//! 32-byte record:
//!
//! ```text
//! +0x00  u32       volume, 0..127
//! +0x04  i32       unread; 0, or a small negative number
//! +0x08  u32       unread; zero in every record seen
//! +0x0c  u32       the child's cue index, or 0xffffffff
//! +0x10  char[16]  the child's name, empty when +0x0c is an index
//! ```
//!
//! **The two forms are exclusive**, and SCREAM names both of them in its own
//! error strings. Wipeout HD's `EBOOT.elf` carries
//! `"SCREAM: Didn't find child sound named -> %s\n"` at `0x007cfaa0` and
//! `"SCREAM: snd_SFX_GRAIN_TYPE_BRANCH invalid sound index %d\n"` at
//! `0x007cfad0`: one message per form, and the second is a message about
//! exactly the malformed data the corpus turns out to contain.
//!
//! # The evidence
//!
//! Across all 230 bank entries on the five PSP/PS2 discs and Wipeout HD, of
//! 1,461 child grains:
//!
//! - **1,153** carry an in-range cue index with an empty name field. Every PSP
//!   and PS2 grain is of this form; not one of them is named.
//! - **300** carry `0xffffffff` and a name the same bank's own name table
//!   holds. Every one of these is on Wipeout HD.
//! - **1** carries a name that is *not* in its own bank: `env0_det.bnk` asks
//!   for `".COLLISIONS"`, which lives in `shiphd.bnk`. A name is resolved
//!   against whatever is loaded, so this is a cross-bank reference rather than
//!   a miss - and it is why [`Bank::resolve_child`] returns `None` for it
//!   rather than treating it as damage.
//! - **7** are neither: six grains in `speech_results.bnk` set `0xffffffff`
//!   with no name at all, and one in `weapons_det.bnk` holds index 65 in a
//!   55-cue bank. That last one is precisely what
//!   `snd_SFX_GRAIN_TYPE_BRANCH invalid sound index %d` exists to print.
//!
//! A wrong record length or a wrong field offset does not produce 1,153
//! in-range indices and 300 exact name-table hits; it produces noise.
//!
//! # What this still does not decide
//!
//! **Which child plays.** A parent's grains are guarded by opcode `0x22`,
//! whose operand is not decoded, so [`Bank::cue_tree_sounds`] returns every
//! reachable leaf's waveforms rather than the one the original would have
//! chosen. That is the same shape - and the same honest gap - as the choice
//! among a single cue's alternates, which is also undecoded. See the module
//! docs of [`super::cue`].

use super::{Bank, COMMAND_LEN, Cue, Sound};

/// The two opcodes that play another cue.
///
/// Whether they differ from each other is **not** known. The record they point
/// at is the same 32 bytes either way and both resolve at the same rate, so any
/// difference has to come from the handler, which this project has not located.
/// `0x08` is the one Wipeout HD's `.COLLISIONS` uses.
pub const CHILD_OPCODES: [u8; 2] = [0x05, 0x08];

/// Bytes per child record in the parameter block.
pub const CHILD_RECORD_LEN: usize = 32;

/// Offset of the child's cue index within its record.
pub const CHILD_INDEX_AT: usize = 0x0c;

/// Offset of the child's 16-byte name within its record.
pub const CHILD_NAME_AT: usize = 0x10;

/// What [`CHILD_INDEX_AT`] holds when the child is named instead of indexed.
pub const CHILD_BY_NAME: u32 = u32::MAX;

/// How deep [`Bank::cue_tree_sounds`] will follow children.
///
/// Wipeout HD's deepest chain is two - `.COLLISIONS` to `c_CShipWall` to
/// `c_CShipWallM` - so this is slack rather than a limit anything reaches. It
/// exists because a bank is data off a disc and nothing in the format prevents
/// a chain from being longer than any bank shipped is.
pub const MAX_CHILD_DEPTH: usize = 8;

/// One grain that plays another cue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Child {
    /// Index of the grain in the command table.
    pub command: usize,
    /// The opcode that named it, one of [`CHILD_OPCODES`].
    pub opcode: u8,
    /// Offset of the 32-byte record within the descriptor section.
    pub record: u32,
    /// The record's `+0x00`, 0..127.
    pub volume: u32,
    /// The child's cue index, when the record carries one.
    pub cue: Option<u16>,
    /// The child's name, when the record carries one instead.
    ///
    /// Empty for an indexed child. Never both - see the module docs.
    pub name: String,
}

impl Child {
    /// Whether this grain names a child rather than indexing one.
    #[must_use]
    pub fn is_named(&self) -> bool {
        self.cue.is_none() && !self.name.is_empty()
    }

    /// Whether the record resolves to a child at all.
    ///
    /// False for the seven malformed grains the module docs count: an
    /// unresolved index with no name to fall back on, or an index past the end
    /// of the bank.
    #[must_use]
    pub fn is_resolvable(&self) -> bool {
        self.cue.is_some() || !self.name.is_empty()
    }
}

impl Bank<'_> {
    /// Every child a cue's own grains play, in command order.
    ///
    /// Empty for a cue that plays no child, which is most of them. A record
    /// that leaves the descriptor section is skipped rather than reported, the
    /// same way [`Bank::sounds`] skips a key-on whose descriptor does not fit.
    #[must_use]
    pub fn cue_children(&self, cue: &Cue) -> Vec<Child> {
        if !cue.plays() {
            return Vec::new();
        }
        let mut out = Vec::new();
        for command in cue.range() {
            let at = command * COMMAND_LEN;
            let Some(grain) = self.commands.get(at..at + COMMAND_LEN) else {
                continue;
            };
            let word = self.order.u32(grain, 0);
            let opcode = (word >> 24) as u8;
            if !CHILD_OPCODES.contains(&opcode) {
                continue;
            }
            let Some(record) = self.parameter_offset.checked_add(word & 0x00ff_ffff) else {
                continue;
            };
            let Some(bytes) = self
                .block
                .get(record as usize..)
                .and_then(|tail| tail.get(..CHILD_RECORD_LEN))
            else {
                continue;
            };

            let raw = self.order.u32(bytes, CHILD_INDEX_AT);
            // An index past the cue table is dropped here rather than carried
            // as a `Some` that every caller would have to re-check. The bank
            // that does it is named in the module docs.
            let cue = (raw != CHILD_BY_NAME)
                .then(|| u16::try_from(raw).ok())
                .flatten()
                .filter(|&index| index < self.cue_count);
            let field = &bytes[CHILD_NAME_AT..CHILD_NAME_AT + 16];
            let end = field.iter().position(|&b| b == 0).unwrap_or(field.len());
            let name = if cue.is_none() && field[..end].iter().all(u8::is_ascii_graphic) {
                String::from_utf8_lossy(&field[..end]).into_owned()
            } else {
                String::new()
            };

            out.push(Child {
                command,
                opcode,
                record,
                volume: self.order.u32(bytes, 0),
                cue,
                name,
            });
        }
        out
    }

    /// The cue a child record points at, resolved the way the runtime would.
    ///
    /// `None` when the record is malformed, and also when it names a cue this
    /// bank does not hold - a name is resolved against every loaded bank, and
    /// one grain on the Wipeout HD disc does reach across. Reading a
    /// cross-bank name as damage would be wrong; so would silently resolving it
    /// here, because this type only has the one bank.
    #[must_use]
    pub fn resolve_child(&self, child: &Child) -> Option<Cue> {
        match child.cue {
            Some(index) => self.cue(index),
            None if child.name.is_empty() => None,
            None => self.cue_named(&child.name),
        }
    }

    /// Every waveform a cue binds, following the cues it plays.
    ///
    /// Identical to [`Bank::cue_sounds`] for a cue that plays no child, which
    /// is what makes it safe to use everywhere: on the PSP, PS2 and Pure discs
    /// no wired cue has children, so this changes nothing there.
    ///
    /// **Every reachable leaf, not the one that would sound.** The grain that
    /// chooses between a parent's children is `0x22`, which is not decoded -
    /// see the module docs. A cue is visited once however many parents reach
    /// it, so a diamond yields its waveforms once and a cycle terminates.
    #[must_use]
    pub fn cue_tree_sounds(&self, cue: &Cue) -> Vec<Sound> {
        let mut seen = vec![cue.index];
        let mut frontier = vec![(*cue, 0usize)];
        let mut out = Vec::new();
        while let Some((cue, depth)) = frontier.pop() {
            out.extend(self.cue_sounds(&cue));
            if depth == MAX_CHILD_DEPTH {
                continue;
            }
            for child in self.cue_children(&cue) {
                let Some(resolved) = self.resolve_child(&child) else {
                    continue;
                };
                if seen.contains(&resolved.index) {
                    continue;
                }
                seen.push(resolved.index);
                frontier.push((resolved, depth + 1));
            }
        }
        out.sort_by_key(|sound| sound.command);
        out
    }
}

#[cfg(test)]
mod tests;
