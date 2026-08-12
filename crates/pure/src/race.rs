//! What a Pure race loads: the default circuit and team, and how a ship's model
//! entry is spelled.
//!
//! Names and defaults, in the [ADR-0022] sense, and the counterpart to
//! `oag_pulse::race`. Everything here was probed against `pure-psp-eu.chd` on
//! 2026-08-12 by asking the archive for the name and seeing whether an entry
//! answered; confidence **94**, since a name that resolves to a decodable entry
//! of the right shape is about as direct as this gets short of watching the
//! original load it.
//!
//! # There is no `ships` module here, and that is a measurement
//!
//! `oag_pulse::race::ships` names five things. **One of them resolves on this
//! disc and it resolves identically**, so it is shared vocabulary rather than a
//! title fact and `oag_game::race` keeps reading Pulse's spelling of it on both
//! sources. Restating it here would be a second copy of one string.
//!
//! The other four find nothing, and their absence is recorded here rather than
//! left to be rediscovered:
//!
//! | Pulse name | On Pure |
//! | --- | --- |
//! | `Data\Ships\<Team>\Ship.vex` | **present** |
//! | `Data\Ships\<Team>\shipboost.vex` | absent |
//! | `Data\Ships\<Team>\Zone.vex` | absent |
//! | `Data\Ships\<Team>\Zoneboost.vex` | absent |
//! | `<environment>\track_reversed.vex` | absent, on every circuit |
//!
//! The last one is corroborated by the disc's own plugin definition, which
//! carries **no `Reversed` attribute on any `PI_Track`** - so Pure ships no
//! reversed circuits at all, where a third of Pulse's menu is them.
//!
//! The other three are facts about *entry names*, not about the title: Pure
//! certainly has a boost plume and a Zone mode - its definition declares four
//! `Zone` circuits and a `Zone_01` team - and **what it calls those files is
//! unrecovered**. So nothing here guesses at a spelling, and no constant records
//! the absence either: `oag_game::race::load` reports a boost model it cannot
//! find and carries on, which makes the consequence a race with no plume rather
//! than no race. An absent entry is a fact; a table saying "Pure's boost is
//! called X" would be an invention.
//!
//! [ADR-0022]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0022-title-packages.md

/// The three things a race needs before it has opened anything, as
/// [`oag_title::Title::race`] carries them.
///
/// The constants below are the same values, kept as named items because that is
/// where their evidence is written down; this is the table the engine reads.
pub const DEFAULTS: &oag_title::RaceDefaults = &oag_title::RaceDefaults {
    track: DEFAULT_TRACK,
    team: DEFAULT_TEAM,
};

/// The circuit a race loads when the caller names none.
///
/// **Vineta K**, the first `PI_Track` the disc's own definition declares and the
/// first circuit of the `Alpha` league - so it is this title's own opening
/// circuit rather than an arbitrary pick, which is the same reasoning that put
/// Talon's Junction in `oag_pulse::race::DEFAULT_TRACK` (there it was the
/// reference-capture circuit; here there are no captures yet and the disc's own
/// ordering is the best available answer).
///
/// Unlike Pulse's, this path is **not** numbered independently of its name:
/// Pure's environment directories carry the circuit's name (`01_Vineta_K`), so
/// the mapping Pulse needs a comment to explain does not arise here.
pub const DEFAULT_TRACK: &str = r"Data\Environments\01_Vineta_K\track.vex";

/// The team whose `handlingstats.xml` and model a race uses by default.
///
/// **Assegai, which is Pulse's default too - and the reason is Pulse's, not
/// Pure's.** It is the team every PPSSPP capture under `data/traces/` was taken
/// with, so keeping it here means the two titles' `--race` defaults stay
/// directly comparable rather than differing in two things at once.
///
/// The only claim being made *about Pure* is that the disc carries the team:
/// `Data\Ships\Assegai\handlingstats.xml` resolves on `pure-psp-eu.chd`,
/// checked 2026-08-12, as do ten other ship directories. Nothing about Assegai
/// is Pure's own preference - its definition declares Feisar first - so change
/// this freely; no measurement is calibrated to it.
pub const DEFAULT_TEAM: &str = "Assegai";

#[cfg(test)]
mod tests {
    use super::{DEFAULT_TEAM, DEFAULT_TRACK};

    /// The circuit is this title's own; the team deliberately is not.
    ///
    /// **Not a style point.** This module exists because `oag_game::race`
    /// re-exported Pulse's circuit constant, so a Pure race asked the archive
    /// for an environment directory that is not on the disc. The team is the
    /// opposite case and is asserted as such: both discs carry Assegai, so
    /// sharing the default is correct rather than a leftover, and a future
    /// reader should not "fix" it into a divergence.
    #[test]
    fn the_circuit_is_pures_own_and_the_team_is_shared_on_purpose() {
        assert_ne!(
            DEFAULT_TRACK,
            oag_pulse::race::DEFAULT_TRACK,
            "16_Track is on no Pure pressing; if these ever match, someone reached \
             for the other title's table again"
        );
        assert_eq!(
            DEFAULT_TEAM,
            oag_pulse::race::DEFAULT_TEAM,
            "both discs carry Assegai, and sharing it is what keeps the two titles' \
             --race defaults comparable - see this constant's own docs"
        );
    }
}
