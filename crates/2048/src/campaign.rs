//! What Wipeout 2048 ships as its own campaign: `Data\xml\SP.xml`
//! (single-player) and `Data\xml\MP.xml` (multiplayer) inside
//! `PSP2/data.psarc`, both authored in the "mjolnir" instance-database shape
//! [`oag_tables::mjolnir`] reads - not a `PI_Grid`/`PI_Cell` grid the way
//! Pulse's, Pure's and Wipeout HD's campaigns all are (`oag_hd::campaign`,
//! `oag_tables::race_campaign`). See that module's own doc comment for the
//! schema and the full typedef census; **names only, no decoding** here, per
//! [ADR-0022] and the same split [`oag_hd::campaign`] and
//! [`oag_pulse::campaign`] keep.
//!
//! `data/plugins/grids/grid_00.xml`..`grid_15.xml`, `Campaign="HD"`/
//! `Campaign="Fury"`, are a **different** campaign - the two DLC tiers,
//! already reachable through `oag_tables::race_campaign` and
//! `oag_hd::campaign`'s own schema. This module is 2048's own, native
//! campaign, the one `SP.xml` authors.
//!
//! [ADR-0022]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0022-title-packages.md

pub use oag_tables::mjolnir::campaign::{
    Event, EventKind, Track, WeaponSet, events, track_for, tracks, typedef, weapon_set_for,
    weapon_sets,
};
pub use oag_tables::mjolnir::{Document, Field, Instance, Reference, parse};

/// The single-player campaign entry. 288 instances measured on the EU v1.04
/// package - see [`oag_tables::mjolnir`]'s own doc comment for the full
/// census.
pub const SP_XML: &str = r"Data\xml\SP.xml";

/// The multiplayer twin, same schema, 289 instances measured (one more than
/// `SP.xml`) - not otherwise probed this pass; every function in this module
/// works on either file's [`Document`] equally, since the schema does not
/// differ between them.
pub const MP_XML: &str = r"Data\xml\MP.xml";

/// `Data\xml\MjolnirData.xml`, 212 bytes: the editing tool's own
/// `<WORKSPACES>` list (`SP.xml`, `MP.xml`, `Profile.xml`). Names no schema
/// and nothing here reads it; recorded so a future pass does not re-extract
/// it hoping for a type table.
pub const MJOLNIR_DATA_XML: &str = r"Data\xml\MjolnirData.xml";

/// 2048's own five speed classes, **not** [`oag_tables::handling::SpeedClass`].
/// That type is deliberately four-wide (see `docs/formats/handling-stats.md`'s
/// "the enum was deliberately not widened" section) and this title's own
/// `Data\HandlingStats\<team>\<1..4>\handlingstats.xml` authors a fifth,
/// `SUPERPHANTOM`, confirmed directly:
/// `data/HandlingStats/feisar2048/3/handlingstats.xml` carries `<Class
/// name="VENOM">`, `FLASH`, `RAPIER`, `PHANTOM`, `SUPERPHANTOM`, in that
/// document order.
///
/// # The ordinal mapping
///
/// `SP.xml`'s own `M_SPEEDCLASS` (`eClass`) is a raw `0`-`4` ordinal.
/// Confidence **85** for `1..=4`: forty of `SP.xml`'s own Speed Lap events
/// are named `"<Track> Speed Lap - <Class>"` and every one measures
/// `M_SPEEDCLASS` consistent with its own name - `1` on every `- Flash`
/// event, `2` on every `- Rapier`, `3` on every `- Phantom`, `4` on every
/// `- Super P` (`SuperPhantom`) - across all ten circuits, no exception.
/// **`0` (`Venom`) is not directly observed**: no event in either `SP.xml`
/// or `MP.xml` authors `M_SPEEDCLASS="0"`, and no event name mentions Venom
/// at all - so `0 => Venom` is elimination (the other four ordinals are
/// pinned) corroborated by, not measured against, `handlingstats.xml`'s own
/// document order above. Confidence **70** for that one rung alone; record a
/// direct observation of an authored `M_SPEEDCLASS="0"` (`MP.xml` was
/// checked and does not have one either) before raising it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EClass {
    /// Ordinal `0`. See this enum's own doc comment on why this one rung is
    /// confidence 70 rather than 85.
    Venom,
    /// Ordinal `1`.
    Flash,
    /// Ordinal `2`.
    Rapier,
    /// Ordinal `3`.
    Phantom,
    /// Ordinal `4`. Not in [`oag_tables::handling::SpeedClass`] at all - see
    /// this enum's own doc comment.
    SuperPhantom,
}

impl EClass {
    /// All five, slowest first - the ordinal order `M_SPEEDCLASS` uses.
    pub const ALL: [Self; 5] = [
        Self::Venom,
        Self::Flash,
        Self::Rapier,
        Self::Phantom,
        Self::SuperPhantom,
    ];

    /// The raw `M_SPEEDCLASS` ordinal for this class.
    #[must_use]
    pub fn ordinal(self) -> i64 {
        match self {
            Self::Venom => 0,
            Self::Flash => 1,
            Self::Rapier => 2,
            Self::Phantom => 3,
            Self::SuperPhantom => 4,
        }
    }

    /// The class for a raw `M_SPEEDCLASS` ordinal. `None` outside `0..=4`.
    #[must_use]
    pub fn from_ordinal(ordinal: i64) -> Option<Self> {
        Self::ALL.into_iter().find(|c| c.ordinal() == ordinal)
    }

    /// The name `handlingstats.xml`'s own `<Class name="...">` spells.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Venom => "VENOM",
            Self::Flash => "FLASH",
            Self::Rapier => "RAPIER",
            Self::Phantom => "PHANTOM",
            Self::SuperPhantom => "SUPERPHANTOM",
        }
    }

    /// This class on [`oag_tables::handling::SpeedClass`]'s own four-wide
    /// ladder. `None` for [`Self::SuperPhantom`], the same "not every rung a
    /// title offers is one the shared enum has a slot for" shape
    /// [`oag_tables::race_campaign::Cell::speed_class`] already applies to
    /// Pulse's `Zone` cells.
    #[must_use]
    pub fn speed_class(self) -> Option<oag_tables::handling::SpeedClass> {
        match self {
            Self::Venom => Some(oag_tables::handling::SpeedClass::Venom),
            Self::Flash => Some(oag_tables::handling::SpeedClass::Flash),
            Self::Rapier => Some(oag_tables::handling::SpeedClass::Rapier),
            Self::Phantom => Some(oag_tables::handling::SpeedClass::Phantom),
            Self::SuperPhantom => None,
        }
    }
}

/// [`Event::speed_class`] resolved to an [`EClass`]. `None` when the event
/// authors no speed class at all (every `Zone` event) or an ordinal outside
/// `0..=4` (not observed on either shipped file).
#[must_use]
pub fn event_class(event: &Event) -> Option<EClass> {
    event.speed_class.and_then(EClass::from_ordinal)
}

/// The circuit `Event::track` resolves to, as `oag_2048::race::DEFAULT_TRACK`
/// spells one: `Data\art\published\environments\<stem>\track.vex`, `<stem>`
/// being [`Track::track_name`] lower-cased (every measured `track_name` is
/// already lower-case; the call is defensive, not corrective). Confidence 90:
/// [`Track::track_name`] matches `data/plugins/tracks/Definition.xml`'s own
/// ten `<PI_Track name="...">` stems exactly, and every one of those in turn
/// names a real `Data\art\published\environments\<name>\` directory this
/// title's manifest carries - the same path shape [`DEFAULT_TRACK`] uses for
/// `altima`. Not itself checked against `oag_title::Title::track_plugin_definition`'s
/// own `location=` attribute at runtime; a caller that has the plugin
/// definition open should prefer resolving through it over trusting this
/// convention blindly.
///
/// [`DEFAULT_TRACK`]: crate::race::DEFAULT_TRACK
#[must_use]
pub fn track_vex_entry(track: &Track) -> String {
    format!(
        r"Data\art\published\environments\{}\track.vex",
        track.track_name.to_lowercase()
    )
}

/// The mode token [`oag_race::Mode::from_name`] (`crates/race/src/mode.rs`)
/// accepts for `event`, or `None` when this engine has no mode for it at all.
///
/// - [`EventKind::Zone`] -> `"zone"`.
/// - [`EventKind::Race`] with [`Event::laps`] `== Some(0)` -> `"speed_lap"`,
///   the sentinel [`oag_tables::mjolnir::campaign::typedef::RACE_A`]'s own doc
///   comment measures: every one of `SP.xml`'s 40 `"<Track> Speed Lap -
///   <Class>"` events carries it and no other lap-race event does.
/// - [`EventKind::Race`] otherwise -> `"single_race"`. **No `SP.xml` event
///   maps to `"time_trial"`** - no campaign event is named or shaped like a
///   Time Trial; that mode exists only for a caller choosing it directly,
///   never resolved from this campaign.
/// - [`EventKind::Elimination`] -> `"eliminator"`. **Checked directly, not
///   assumed**: `oag_race::Mode::Eliminator` is a real, simulated mode
///   (`crates/game/src/race/eliminator.rs`, `tick.rs`'s own kill-count
///   ending), this title's own `TITLE.weapons.elimination` already names
///   `Data\XML\weaponstats_Elimination_2048.xml`, and `cargo run -p oag-game
///   -- <2048 source> --race --mode eliminator --track
///   Data\art\published\environments\park\track.vex --class flash --dry-run`
///   loads end to end (exit 0, `Elimination_HUD.xml` composes 111 sprites) -
///   this engine's CLI help text just never lists `eliminator` alongside the
///   other three tokens, which reads as an omission in the help string, not
///   a real restriction, since `Mode::from_name("eliminator")` resolves it
///   like any other token.
#[must_use]
pub fn engine_mode(event: &Event) -> Option<&'static str> {
    match event.kind {
        EventKind::Zone => Some("zone"),
        EventKind::Race if event.laps == Some(0) => Some("speed_lap"),
        EventKind::Race => Some("single_race"),
        EventKind::Elimination => Some("eliminator"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eclass_ordinals_round_trip() {
        for class in EClass::ALL {
            assert_eq!(EClass::from_ordinal(class.ordinal()), Some(class));
        }
        assert_eq!(EClass::from_ordinal(5), None);
        assert_eq!(EClass::from_ordinal(-1), None);
    }

    #[test]
    fn only_superphantom_has_no_four_wide_speed_class() {
        assert_eq!(
            EClass::Venom.speed_class(),
            Some(oag_tables::handling::SpeedClass::Venom)
        );
        assert_eq!(
            EClass::Phantom.speed_class(),
            Some(oag_tables::handling::SpeedClass::Phantom)
        );
        assert_eq!(EClass::SuperPhantom.speed_class(), None);
    }

    fn race_event(kind: EventKind, typedef_id: i64, laps: Option<u32>) -> Event {
        Event {
            instance_id: 1,
            typedef_id,
            kind,
            name: String::new(),
            description: None,
            track: None,
            speed_class: None,
            laps,
            weapon_set: None,
            next_event: None,
            branch_event: None,
            required_event: None,
            pass_objective: None,
            elite_objective: None,
            x: None,
            y: None,
            rank_required: None,
            force_always_unlocked: None,
            has_ghost_capacity: false,
        }
    }

    #[test]
    fn engine_mode_maps_zero_laps_to_speed_lap_and_elimination_to_eliminator() {
        assert_eq!(
            engine_mode(&race_event(EventKind::Race, typedef::RACE_A, Some(0))),
            Some("speed_lap")
        );
        assert_eq!(
            engine_mode(&race_event(EventKind::Race, typedef::RACE_A, Some(3))),
            Some("single_race")
        );
        assert_eq!(
            engine_mode(&race_event(EventKind::Race, typedef::RACE_B, Some(5))),
            Some("single_race")
        );
        assert_eq!(
            engine_mode(&race_event(EventKind::Zone, typedef::ZONE, None)),
            Some("zone")
        );
        assert_eq!(
            engine_mode(&race_event(
                EventKind::Elimination,
                typedef::ELIMINATION,
                Some(1)
            )),
            Some("eliminator")
        );
    }

    #[test]
    fn track_vex_entry_matches_default_tracks_own_spelling() {
        let altima = Track {
            instance_id: 1,
            name: "Altima".to_string(),
            track_name: "altima".to_string(),
            display_name: "ALTIMA".to_string(),
        };
        assert_eq!(
            track_vex_entry(&altima),
            crate::race::DEFAULT_TRACK,
            "must match oag_2048::race::DEFAULT_TRACK's own spelling exactly"
        );
    }
}
