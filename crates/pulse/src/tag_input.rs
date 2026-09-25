//! The alphabet Pulse's own `<TagInput>` scrolls through - a
//! [`crate::hashes::TAG_INPUT_SCREENS`] cell steps one glyph at a time.
//!
//! **Not in any WAD.** It is 70 bytes inside `BOOT.BIN` itself, so unlike
//! everything else this crate names, there is no runtime path that loads it:
//! this project's own binary never maps the PSP executable, the same reason
//! `oag_vex`'s recovered `CLASS_*` IDs are hardcoded constants rather than
//! read off the disc every boot. [`ALPHABET`] is that same shape - a
//! recovered constant, checked against the real bytes by a disc-backed test
//! rather than read at runtime - not an invented stand-in: see
//! `crates/pulse/tests/tag_input_alphabet_ground_truth.rs`, which is the
//! thing that keeps this honest if a future pressing's bytes ever disagree.

/// The 70 characters a Pulse `<TagInput>` cell cycles through, in the order
/// the executable lists them.
///
/// **Confidence 85.** Five `lui 0x002b` / `lo16 0xdc10` pairs reference this
/// address, all inside one `0x0c6xxx`-`0x0c7xxx` code region that also holds
/// the `TagInput` element-name reference - a glyph *map* (one entry per key
/// position rather than a scrolled alphabet) is ruled out by Pure's own copy
/// of this string carrying no lowercase at all while its front end
/// demonstrably draws lowercase. No Ghidra function was traced to a
/// confidence that would earn a rename, so there is no `names.tsv` row for
/// this - see CLAUDE.md's confidence rule.
///
/// **USA** (`pulse-psp-usa.chd`, `PSP_GAME/SYSDIR/BOOT.BIN`): file offset
/// 2,808,976, vaddr `0x08AB1C10` at the standard PSP load base.
///
/// **EU** (`pulse-psp-eu.chd`): byte-identical, at file offset 2,806,800.
/// The vaddr this implies, `0x08AB1390`, is *derived* from the file-offset
/// delta between the two builds (2,176 bytes) rather than independently
/// read out of Ghidra - the byte content and file offset are what the test
/// actually checks.
pub const ALPHABET: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz 0123456789!-+@:?*";

/// Where [`ALPHABET`] sits in each pressing's `BOOT.BIN`, as a file byte
/// offset - not a vaddr, since that is what a test reading the extracted
/// file can seek to directly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Location {
    /// Which disc.
    pub region: &'static str,
    /// Byte offset into `PSP_GAME/SYSDIR/BOOT.BIN`.
    pub file_offset: usize,
}

/// Both pressings this has been checked on. See [`ALPHABET`]'s own doc for
/// the vaddr each implies.
pub const LOCATIONS: [Location; 2] = [
    Location {
        region: "USA",
        file_offset: 2_808_976,
    },
    Location {
        region: "EU",
        file_offset: 2_806_800,
    },
];
