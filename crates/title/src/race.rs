//! What a title's race path needs before it has opened anything.
//!
//! The third axis a second corpus has forced, on the same terms as [`crate::boot`]
//! and [`crate::menu`]: it exists because the composition root would otherwise
//! have to branch on which disc it opened, not because a race obviously needs a
//! table.
//!
//! | | Pulse | Pure |
//! | --- | --- | --- |
//! | default circuit | `Data\Environments\16_Track\track.vex` | `Data\Environments\01_Vineta_K\track.vex` |
//! | default team | `Assegai` | `Assegai` |
//!
//! **One field of the two actually diverges**, and that one is the whole reason
//! this type exists: `16_Track` resolves on no Pure pressing, so `oag-game`'s
//! `--track` carrying it as a compile-time default made `--race` on a Pure disc
//! fail inside the archive with a message about a name that hashes to nothing.
//! The team is here beside it because a caller that must name a circuit before
//! opening anything must name a team too, not because the two discs disagree -
//! they do not.
//!
//! # Deliberately two fields
//!
//! Not a home for everything a race loads. Three kinds of thing are kept out,
//! for three different reasons:
//!
//! - **Shared vocabulary.** `Data\Ships\<Team>\Ship.vex` resolves on both
//!   discs, so the hull's file name is not a title fact and a field for it would
//!   be a table with one value in it.
//! - **Unrecovered, not different.** The boost plume and the Zone hull are on
//!   Pure somewhere; what that disc calls them is unread. A field would be a
//!   `None` designed from one example, which is the failure [ADR-0009] named and
//!   [ADR-0022] does not license.
//! - **Absent by construction.** Pure ships no loading-screen wave and no `.mip`
//!   HUD atlas at all. An axis that is `Some` for one title and `None` for the
//!   other is the same one-example design in a different disguise.
//!
//! All three stay as constants in the title crate that knows them, or as an
//! honest report line, exactly as this crate's own module docs say.
//!
//! What is here is the set a caller needs **before** it can open the source: the
//! command line has to name a circuit and a team, and both are per-title. That
//! is the test for this type rather than "is it about racing".
//!
//! [ADR-0009]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0009-multi-game-fanout.md
//! [ADR-0022]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0022-title-packages.md

/// The circuit and team a race falls back to on one title.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RaceDefaults {
    /// Archive entry of the `.vex` a race loads when the caller names none.
    ///
    /// A full entry name rather than a directory, because a circuit is a *file*:
    /// Pulse ships `track.vex` and `track_reversed.vex` side by side and the
    /// menu treats them as two entries.
    pub track: &'static str,
    /// The team id a race uses when the caller names none.
    ///
    /// An **id** - the folder under `Data\Ships\` - and not the name a player
    /// reads, which comes from the string table. See `oag_game::catalogue`.
    pub team: &'static str,
}
