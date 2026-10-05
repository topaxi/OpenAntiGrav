//! [`BankName`]: which `.bnk` a cue lives in.
//!
//! Split out of `cue.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change.

/// Which bank a cue lives in.
///
/// The paths are literal strings in the PSP executable, every one of which
/// hashes to a real archive entry on the PSP *and* the PS2 disc - see
/// `docs/formats/psp-audio.md`'s table. The PS2 build ships the same banks
/// under the same names in `WADS2.WAD`, which is why nothing here branches on
/// the platform: `Archives::read_name` searches whichever archives the source
/// turned out to have.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BankName {
    /// `Data\Sound\hud.bnk`.
    Hud,
    /// `Data\Sound\ship.bnk`, or `ship_zone.bnk` in Zone mode.
    Ship,
    /// `Data\Sound\weapons.bnk`.
    Weapons,
    /// `Data\Sound\speech.bnk` - the announcer.
    ///
    /// The executable also carries `Data\Sound\speech_%s.bnk` and
    /// `speech_zone_%s.bnk` templates formatted with a language at runtime, and
    /// **no expansion of either resolves on the EU disc** - twelve language
    /// names were tried and every one missed. So this port reads the unsuffixed
    /// bank, which both discs do carry, and the localised path stays open. See
    /// `docs/formats/psp-audio.md`.
    Speech,
}

impl BankName {
    /// The archive entry to read, given a title's own table and whether this is
    /// a Zone race.
    ///
    /// **Which file a cue is in is a per-title fact and the cue's name is not.**
    /// All three titles spell `SPEEDUPPAD` and `.COLLISIONS` identically; HD
    /// keeps the first in `weapons.bnk` because it has no `hud.bnk` at all, and
    /// the second in `shiphd.bnk`. See [`oag_title::SoundBanks`].
    ///
    /// Only [`Self::Ship`] moves with `zone`, and on Pulse and Pure that is
    /// load-bearing: `SHIP_ZM` is a different bank with the same cue names and
    /// different audio - a nine-layer `~ENGINE` against one, ten collision
    /// alternates against fifteen - so a Zone race reading `ship.bnk` would be
    /// quietly playing the wrong craft. HD ships no separate Zone bank and its
    /// table says so by repeating itself.
    #[must_use]
    pub fn entry(self, banks: &oag_title::SoundBanks, zone: bool) -> &'static str {
        match self {
            Self::Hud => banks.hud,
            Self::Ship if zone => banks.ship_zone,
            Self::Ship => banks.ship,
            Self::Weapons => banks.weapons,
            Self::Speech => banks.speech,
        }
    }
}
