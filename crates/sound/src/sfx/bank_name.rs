//! [`BankName`]: which `.bnk` a cue lives in. Split out of `cue.rs` under the
//! 1,000-line rule.

/// Which bank a cue lives in.
///
/// The paths are literal strings in the PSP executable, each hashing to a real
/// archive entry on the PSP and the PS2 disc (`docs/formats/psp-audio.md`). The
/// PS2 build ships the same banks under the same names in `WADS2.WAD`, so
/// nothing branches on platform: `Archives::read_name` searches whichever
/// archives the source has.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BankName {
    /// `Data\Sound\hud.bnk`.
    Hud,
    /// `Data\Sound\ship.bnk`, or `ship_zone.bnk` in Zone mode.
    Ship,
    /// `Data\Sound\weapons.bnk`.
    Weapons,
    /// `Data\Sound\speech.bnk`, the announcer.
    ///
    /// The executable also carries `Data\Sound\speech_%s.bnk` and
    /// `speech_zone_%s.bnk` templates formatted with a language at runtime, and
    /// no expansion resolves on the EU disc (twelve language names tried, every
    /// one missed). So this reads the unsuffixed bank both discs carry
    /// (`docs/formats/psp-audio.md`).
    Speech,
    /// `Data\Sound\frontend.bnk`, the front end's navigation sounds. Only a
    /// title whose [`oag_title::SoundBanks::frontend`] is set has one.
    Frontend,
}

impl BankName {
    /// The archive entry to read, given a title's own table and whether this is a
    /// Zone race.
    ///
    /// Which file a cue is in is a per-title fact; the name is not. All three
    /// titles spell `SPEEDUPPAD` and `.COLLISIONS` identically, but HD keeps the
    /// first in `weapons.bnk` (no `hud.bnk`) and the second in `shiphd.bnk`
    /// ([`oag_title::SoundBanks`]).
    ///
    /// Only [`Self::Ship`] moves with `zone`, load-bearing on Pulse and Pure:
    /// `SHIP_ZM` has the same cue names with different audio (a nine-layer
    /// `~ENGINE` against one, ten collision alternates against fifteen), so a
    /// Zone race reading `ship.bnk` would play the wrong craft. HD has no
    /// separate Zone bank and its table repeats itself.
    #[must_use]
    pub fn entry(self, banks: &oag_title::SoundBanks, zone: bool) -> &'static str {
        match self {
            Self::Hud => banks.hud,
            Self::Ship if zone => banks.ship_zone,
            Self::Ship => banks.ship,
            Self::Weapons => banks.weapons,
            Self::Speech => banks.speech,
            Self::Frontend => banks.frontend.unwrap_or(""),
        }
    }
}
