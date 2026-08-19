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
//! | Zone's circuit | the race one's directory, `zone_`-prefixed | `Data\Zone\01_Zone\track.vex`, a circuit of its own |
//!
//! **Two fields of the three actually diverge**, and the first of them is the
//! reason this type exists at all: `16_Track` resolves on no Pure pressing, so
//! `oag-game`'s `--track` carrying it as a compile-time default made `--race` on
//! a Pure disc fail inside the archive with a message about a name that hashes
//! to nothing. The team is here beside it because a caller that must name a
//! circuit before opening anything must name a team too, not because the two
//! discs disagree - they do not.
//!
//! [`ZoneCircuit`] is the third, and it is the one measured across all three
//! titles rather than two; its own docs carry the probe.
//!
//! # Deliberately three fields
//!
//! Not a home for everything a race loads. Three kinds of thing are kept out,
//! for three different reasons:
//!
//! - **Shared vocabulary.** `Data\Ships\<Team>\Ship.vex` resolves on both
//!   discs, so the hull's file name is not a title fact and a field for it would
//!   be a table with one value in it.
//! - **Unrecovered, not different.** The boost plume is on Pure somewhere; what
//!   that disc calls it is unread. A field would be a `None` designed from one
//!   example, which is the failure [ADR-0009] named and [ADR-0022] does not
//!   license. **The Zone *hull* used to be on this list and is not any more** -
//!   see `oag_pure::race::ZONE_TEAM` - but it is still not a field here, for the
//!   same reason: HD's is unread, so the axis has two measurements and a hole,
//!   which is exactly the shape being refused. The circuit axis below has three
//!   measurements and no hole, which is what makes it a type instead.
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
    /// How this title names the circuit a Zone race runs on. See
    /// [`ZoneCircuit`].
    pub zone: ZoneCircuit,
}

/// Where a title keeps the environment a Zone race flies through.
///
/// **Zone's whole look is authored, not computed.** A Zone circuit ships its own
/// meshes, its own textures, its own lights, its own `fogCube` and its own
/// `Skycube` - and the sky is the giveaway, because the twelve one-material
/// skies `docs/formats/skycube.md` counts across the PSP disc are exactly the
/// Zone variants against five or six materials for every race circuit. So there
/// is nothing to tint, desaturate or otherwise invent: loading the file the disc
/// authors *is* the aesthetic, per `CLAUDE.md`'s rule about not authoring a
/// stand-in for what the data already carries.
///
/// **Where that file lives is the part that diverges**, and unlike the two
/// fields beside it this one was measured on all three titles rather than two:
///
/// | Title | Zone's environment | Measured on |
/// | --- | --- | --- |
/// | Pulse | `Data\Environments\16_Track\zone_track.vex` - the race circuit's own directory, one extra file | `pulse-psp-usa.chd`, `pulse-ps2-eu.chd` |
/// | Pure | `Data\Zone\01_Zone\track.vex` - four circuits of its own, declared `type="Zone"` | `pure-psp-eu.chd` |
/// | HD / Fury | `/data/environments/zone_1/track.vex` - four circuits of its own | `hdfury-ps3-eu-dec.iso` |
///
/// Two shapes across three titles, so neither variant is designed from one
/// example and there is no third state for a title nobody has looked at. That
/// is what [ADR-0022] licenses and what the `Option` fields this module's docs
/// refuse do not have.
///
/// **The selector in the executable is unread.** `docs/formats/track.md` records
/// the binary's own `%s\%strack%s.vex` template, whose two `%s`s are exactly the
/// prefix and the `_reversed` suffix [`Self::Prefixed`] composes - but the
/// branch that puts `zone_` in the first one has not been found, the way
/// `Ship_LoadModel`'s `case 6` was found for the hull. Everything here is name
/// resolution against shipped archives, which is why the confidence is the 94 of
/// a direct probe and not higher.
///
/// [ADR-0022]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0022-title-packages.md
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZoneCircuit {
    /// A Zone race runs the *same* circuit, out of a second file in its
    /// directory whose name is the race one with this prefix on the front.
    ///
    /// Pulse, on both its platforms: `track.vex` beside `zone_track.vex`, and
    /// `track_reversed.vex` beside `zone_track_reversed.vex`. The prefix is
    /// carried rather than hardcoded because it is the first `%s` of the
    /// binary's own path template, and a title that spelled it differently would
    /// change this string and nothing else.
    ///
    /// **Not every circuit has one**, which is a load-correctness fact and not a
    /// menu one - see `oag_game::catalogue::Track::available_in_zone`.
    Prefixed(&'static str),
    /// A Zone race runs a circuit of its own, sharing nothing with the race
    /// ones, and this is the one it opens when the caller names none.
    ///
    /// Pure and HD. There is deliberately **no mapping** from a race circuit to
    /// a Zone one here, because the discs author none: Pure ships eight race
    /// circuits and four Zone circuits, HD sixteen environments of which four
    /// are Zone. Asking "what is Vineta K's Zone variant" is a question with no
    /// answer in the data, so this variant answers the only question that does
    /// have one.
    Separate(&'static str),
}

impl ZoneCircuit {
    /// The circuit a Zone race opens when the caller named one.
    ///
    /// [`Self::Prefixed`] rewrites it, because on such a title the name the
    /// caller gave is a race circuit and its Zone twin is derivable.
    /// [`Self::Separate`] hands it back untouched, because there is nothing to
    /// derive - a caller naming a circuit on Pure or HD has named the one it
    /// wants, and rewriting it would invent a path the disc does not carry.
    ///
    /// **Idempotent under `Prefixed`**: a name already carrying the prefix comes
    /// back unchanged, so `--track ...\zone_track.vex --mode zone` asks for
    /// `zone_track.vex` rather than `zone_zone_track.vex`.
    #[must_use]
    pub fn variant_of(self, track: &str) -> String {
        match self {
            Self::Prefixed(prefix) => {
                let split = track.rfind(['\\', '/']).map_or(0, |i| i + 1);
                let (directory, file) = track.split_at(split);
                if file.starts_with(prefix) {
                    return track.to_string();
                }
                format!("{directory}{prefix}{file}")
            }
            Self::Separate(_) => track.to_string(),
        }
    }

    /// The circuit a Zone race opens when the caller named none.
    ///
    /// `race_default` is [`RaceDefaults::track`], which is what a race would
    /// have opened. [`Self::Prefixed`] derives its Zone twin from it, so the
    /// title's opening circuit stays the opening circuit in both modes;
    /// [`Self::Separate`] ignores it and answers with its own, because a race
    /// circuit is not a Zone one on those titles and opening it would be a
    /// normal race wearing the Zone rules.
    #[must_use]
    pub fn default_track(self, race_default: &str) -> String {
        match self {
            Self::Prefixed(_) => self.variant_of(race_default),
            Self::Separate(track) => track.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ZoneCircuit;

    const PULSE: ZoneCircuit = ZoneCircuit::Prefixed("zone_");
    const PURE: ZoneCircuit = ZoneCircuit::Separate(r"Data\Zone\01_Zone\track.vex");

    /// The prefix goes on the *file*, not the front of the whole entry name,
    /// and both separators the corpus uses are directory separators.
    #[test]
    fn the_prefix_lands_on_the_file_name() {
        assert_eq!(
            PULSE.variant_of(r"Data\Environments\16_Track\track.vex"),
            r"Data\Environments\16_Track\zone_track.vex"
        );
        assert_eq!(
            PULSE.variant_of("/data/environments/talons_junction/track.vex"),
            "/data/environments/talons_junction/zone_track.vex"
        );
    }

    /// The `_reversed` suffix is part of the file name, so prefixing composes
    /// with it and produces the fourth of the four names a circuit ships.
    #[test]
    fn the_prefix_composes_with_the_reversed_suffix() {
        assert_eq!(
            PULSE.variant_of(r"Data\Environments\16_Track\track_reversed.vex"),
            r"Data\Environments\16_Track\zone_track_reversed.vex"
        );
    }

    /// `--track` naming a Zone circuit outright must not be prefixed twice.
    #[test]
    fn prefixing_an_already_prefixed_name_changes_nothing() {
        let zone = r"Data\Environments\16_Track\zone_track.vex";
        assert_eq!(PULSE.variant_of(zone), zone);
    }

    /// A title whose Zone circuits are their own has nothing to derive: the
    /// caller's name stands, and the default is the title's own Zone circuit
    /// rather than a rewrite of its race one.
    #[test]
    fn separate_circuits_are_never_rewritten() {
        let named = r"Data\Zone\03_Zone\track.vex";
        assert_eq!(PURE.variant_of(named), named);
        assert_eq!(
            PURE.default_track(r"Data\Environments\01_Vineta_K\track.vex"),
            r"Data\Zone\01_Zone\track.vex"
        );
        assert_eq!(
            PULSE.default_track(r"Data\Environments\16_Track\track.vex"),
            r"Data\Environments\16_Track\zone_track.vex"
        );
    }
}
