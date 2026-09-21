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
    Event, EventKind, Objective, Track, WeaponSet, events, objective_for, objectives, track_for,
    tracks, typedef, weapon_set_for, weapon_sets,
};
pub use oag_tables::mjolnir::{Document, Field, Instance, Reference, parse};

/// The single-player campaign entry. 288 instances measured on the EU v1.04
/// package - see [`oag_tables::mjolnir`]'s own doc comment for the full
/// census.
pub const SP_XML: &str = r"Data\xml\SP.xml";

/// **Not `SP.xml`'s schema.** 289 instances measured, but only
/// [`typedef::GAME_MODE_OBJECTIVE`] (59 of them) is shared; the other 230
/// are two typedefs `SP.xml` carries none of - `1114956821` x210, a
/// "level" shape (`M_BASEOBJECTIVE`/`M_HARDOBJECTIVE`/`M_MEDIUMOBJECTIVE`/
/// `M_PNEXTLEVEL`/`M_PAR`/`M_X`/`M_Y`, names like `"MP_S12_E02"`) and
/// `425681076` x20, a season container (`M_PNEXTSEASON`/`M_PFIRSTLEVEL`,
/// names like `"MP_Season_01"`). Neither is named in-file the way
/// [`super::typedef`]'s table is, and neither is read by any function in
/// this module - [`events`]/[`tracks`]/[`weapon_sets`] filter by `SP.xml`'s
/// own typedef ids and simply find nothing in an `MP.xml` [`Document`],
/// rather than misreading one shape as another. Left for a pass that wants
/// the multiplayer season ladder specifically.
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

/// `M_OBJECTIVETYPE`'s own `ObjectiveValue` ordinals, given a meaning here
/// rather than in [`oag_tables::mjolnir::campaign`] - see [`Objective`]'s own
/// doc comment for why the split falls there.
///
/// **Measured against every `(pass, elite)` type pair `SP.xml`'s own 80
/// fully-authored events carry** (every event authors both objectives or
/// neither - measured, 80 of 141 with both, 61 with neither, never one
/// alone): `(FINISH, POSITION)` x2 (`"2048 - Event 1"`/`"2048 - Event 2"`,
/// pass on any finish, elite on a win), `(POSITION, POSITION)` x38 (the
/// ordinary numbered campaign, `"TopN"`/`"Win"` objectives), `(POSITION,
/// KILLS)` x2 (`"2050 - Event 3-4"`/`"6-4"`, pass on a win, elite on a
/// kill), `(BEAT_VALUE, BEAT_VALUE)` x13 `Race`-kind (a total race time, in
/// centiseconds, lower is better - confirmed on every one of the 13,
/// `elite < pass` with no exception), x10 `Zone`-kind (a zone counter,
/// higher is better - `elite > pass`, no exception) and x15
/// `Elimination`-kind (higher is better, `elite > pass`, no exception, but
/// **the metric itself is not identified** - see [`evaluate_tier`]'s own
/// doc). Confidence 85 for the four ordinals' own names; confidence 80 for
/// `BEAT_VALUE`'s per-kind metric (Race/Zone only - Elimination's own
/// metric is confidence under 50, so nothing is guessed there). See
/// `docs/formats/2048-campaign.md`'s "The objective law" section for the
/// full census this was read off.
pub mod objective_type {
    /// Satisfied by finishing at all - the two authored instances
    /// (`"FinishRaceAnyPosition"`) carry no target, so nothing is compared
    /// against a number for this type.
    pub const FINISH: i64 = 1;
    /// A numeric threshold whose metric is not named by the type alone -
    /// [`super::evaluate_tier`] picks it from the event's own [`EventKind`].
    pub const BEAT_VALUE: i64 = 2;
    /// The target is a finishing position, `1`-based; met when the player's
    /// own place is at most the target ("Nth or better").
    pub const POSITION: i64 = 4;
    /// The target is an opponent kill count; met when the player's own kill
    /// count is at least the target.
    pub const KILLS: i64 = 7;
}

/// One resolved `M_PASSOBJECTIVE`/`M_ELITEOBJECTIVE` - [`Objective`]'s own
/// type and target, carried apart from the [`Document`] that produced them
/// so [`evaluate_tier`] needs no archive access at grading time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectiveRule {
    /// See [`Objective::objective_type`].
    pub objective_type: Option<i64>,
    /// See [`Objective::target`].
    pub target: Option<i64>,
}

impl From<&Objective> for ObjectiveRule {
    fn from(objective: &Objective) -> Self {
        Self {
            objective_type: objective.objective_type,
            target: objective.target,
        }
    }
}

/// An event's own pass/elite gate, resolved once at load so grading it never
/// needs the [`Document`] again.
///
/// **Never partially built.** [`event_objectives`] is the only constructor
/// and it requires both `M_PASSOBJECTIVE` and `M_ELITEOBJECTIVE` to resolve -
/// matching the measured "both or neither" shape every one of `SP.xml`'s
/// event instances carries (see [`objective_type`]'s own doc comment).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EventObjectives {
    /// [`Event::kind`] - what [`evaluate_tier`] uses to pick
    /// [`objective_type::BEAT_VALUE`]'s own metric.
    pub kind: EventKind,
    /// The bar this build's own two-tier career counts as a completion -
    /// see [`Tier::Pass`].
    pub pass: ObjectiveRule,
    /// The harder bar - see [`Tier::Elite`].
    pub elite: ObjectiveRule,
}

/// Resolves `event`'s own pass/elite objective pair. `None` when it authors
/// neither (every Speed Lap and every `MP_*`/generic-template instance -
/// see [`objective_type`]'s own doc comment) or when a reference it does
/// carry does not resolve inside `document`.
#[must_use]
pub fn event_objectives(document: &Document, event: &Event) -> Option<EventObjectives> {
    let pass = objective_for(document, event.pass_objective?)?;
    let elite = objective_for(document, event.elite_objective?)?;
    Some(EventObjectives {
        kind: event.kind,
        pass: ObjectiveRule::from(&pass),
        elite: ObjectiveRule::from(&elite),
    })
}

/// What a finished (or abandoned) attempt at an event actually did - the
/// generic shape [`evaluate_tier`] grades against an [`EventObjectives`].
///
/// Every field is read the way [`crate`]'s own caller already has it in
/// hand (`oag_race::Standing`'s own place/kills, `RaceState::zone`, a total
/// tick count) - unconditionally, the same way `RaceStage::campaign_medal`
/// (Pulse/HD's own evaluator) always has a place and a kill count even in a
/// mode neither means anything for. [`evaluate_tier`] is what decides which
/// field a given event's own objective actually reads.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EventOutcome {
    /// Whether the attempt reached its own finish condition - gates
    /// [`objective_type::FINISH`] and [`objective_type::POSITION`], and
    /// [`objective_type::BEAT_VALUE`] on a [`EventKind::Race`].
    pub finished: bool,
    /// The player's own finishing place, `1`-based - meaningless unless
    /// `finished` is `true`, and [`evaluate_tier`] never reads it otherwise.
    pub place: u8,
    /// The total race time, in centiseconds - the unit `SP.xml`'s own
    /// `BEAT_VALUE` targets for a `Race`-kind event are authored in. `None`
    /// unless `finished` is `true`.
    pub finish_centiseconds: Option<i64>,
    /// The zone counter reached so far - meaningful only on
    /// [`EventKind::Zone`], gated on nothing else the same way Pulse's own
    /// Zone medal is not: the counter only ever grows.
    pub zone: u16,
    /// Opponent kills scored so far - meaningful only for
    /// [`objective_type::KILLS`], gated on nothing else for the same reason.
    pub kills: u32,
}

/// Which of an event's own two authored bars an [`EventOutcome`] cleared.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tier {
    /// Met [`EventObjectives::pass`] but not [`EventObjectives::elite`].
    Pass,
    /// Met [`EventObjectives::elite`] - always the better of the two, since
    /// [`evaluate_tier`] checks it first.
    Elite,
}

/// The best [`Tier`] `outcome` earns against `objectives`'s own pass/elite
/// rules, or `None` when neither is met.
///
/// **One case never awards a tier at all, on purpose.** An
/// [`EventKind::Elimination`] event's own [`objective_type::BEAT_VALUE`]
/// objective (all 15 measured instances, targets `25`-`100`) has no metric
/// this engine can name with any confidence: `oag_race::Standing::kills`
/// (the only per-race count 2048's own simulation tracks) never approaches
/// those numbers in a real match, and nothing else - a damage total, a
/// points score - is tracked at all. Confidence under 50 for any specific
/// guess, so none is made; see `docs/formats/2048-campaign.md`'s "what is
/// not determined" section. An Elimination event still plays and still
/// unlocks whatever names it as a prerequisite (gated on `finished`, not on
/// a medal - see `oag_game`'s own unlock-gate evaluator), it simply never
/// carries a medal.
#[must_use]
pub fn evaluate_tier(objectives: &EventObjectives, outcome: &EventOutcome) -> Option<Tier> {
    if objective_met(objectives.kind, objectives.elite, outcome) == Some(true) {
        Some(Tier::Elite)
    } else if objective_met(objectives.kind, objectives.pass, outcome) == Some(true) {
        Some(Tier::Pass)
    } else {
        None
    }
}

/// `None` means "cannot be graded" (an unfinished attempt against a rule
/// that needs one, or a rule this function does not know how to read) -
/// never "failed". [`evaluate_tier`] treats both the same way (no tier),
/// but keeping them apart here is what let this function's own doc comment
/// state the Elimination gap as "never guessed" rather than "always fails".
fn objective_met(kind: EventKind, rule: ObjectiveRule, outcome: &EventOutcome) -> Option<bool> {
    match rule.objective_type {
        Some(objective_type::FINISH) => Some(outcome.finished),
        Some(objective_type::POSITION) => {
            if !outcome.finished {
                return None;
            }
            Some(i64::from(outcome.place) <= rule.target?)
        }
        Some(objective_type::KILLS) => Some(i64::from(outcome.kills) >= rule.target?),
        Some(objective_type::BEAT_VALUE) => match kind {
            EventKind::Race => {
                if !outcome.finished {
                    return None;
                }
                Some(outcome.finish_centiseconds? <= rule.target?)
            }
            EventKind::Zone => Some(i64::from(outcome.zone) >= rule.target?),
            EventKind::Elimination => None,
        },
        Some(_) | None => None,
    }
}

/// The single campaign event whose completion unlocks each event with a
/// cell, by name - `(event name, its own prerequisite's name)`, `None` for
/// an event open from the start.
///
/// **A single name is enough for every case `SP.xml` authors.** Measured
/// directly: no event is ever the `M_PNEXTEVENT`/`M_PBRANCHEVENT` target of
/// more than one other event, and the two events that carry both an
/// incoming chain edge *and* their own `M_PEVENTREQUIRED`
/// (`"2049 - Event 1"`, `"2050 - Event 1"`) name the identical prerequisite
/// either way - so collapsing "what unlocks this" to one optional name
/// loses no case this file authors, rather than being a simplification that
/// happens to work on the sample checked. Confidence 90: structural, over
/// every one of `SP.xml`'s own events, not a sample.
///
/// A fresh `SP.xml` read this way names 16 events open from the start on
/// the EU v1.04 package: `"2048 - Event 1"` and 15 side events with no
/// gate at all (a per-team `"* P Ship Challenge"` and a per-track
/// `"* - S Phantom Challenge"`) - `M_bForceAlwaysUnlocked` is authored on
/// no event at all (measured, zero non-empty/non-zero instances), so it is
/// not what opens these; the absence of any incoming edge or
/// `M_PEVENTREQUIRED` is.
#[must_use]
pub fn unlock_gates(document: &Document) -> Vec<(String, Option<String>)> {
    let all = events(document);
    let mut incoming: Vec<(i64, i64)> = Vec::new();
    for event in &all {
        if let Some(next) = event.next_event {
            incoming.push((event.instance_id, next.instance_id));
        }
        if let Some(branch) = event.branch_event {
            incoming.push((event.instance_id, branch.instance_id));
        }
    }
    all.iter()
        .map(|event| {
            let predecessor_id = incoming
                .iter()
                .find(|(_, target)| *target == event.instance_id)
                .map(|(predecessor, _)| *predecessor)
                .or_else(|| event.required_event.map(|r| r.instance_id));
            let gate = predecessor_id
                .and_then(|id| document.instance(id))
                .map(|instance| instance.name.clone());
            (event.name.clone(), gate)
        })
        .collect()
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

    /// Three chained events, shaped after the real `SP.xml` census this
    /// module's own doc comments cite: a `RACE_B` opener with
    /// `FINISH`/`POSITION` objectives and no gate of its own, a `RACE_A`
    /// time-based follow-on reached by `M_PNEXTEVENT`, and a `ZONE` side
    /// event reached by `M_PEVENTREQUIRED` alone (never chained).
    const CAMPAIGN_FIXTURE: &str = r#"<mjolnir>
<instance instanceid="1" typedefid="-1353052320" name="2048 - Event 1"><DATA>
<M_PNEXTEVENT name="m_pNextEvent" type="GameModeBase" length="1" typedefid="366306753"><ARRAY value="2" typedefid="-1915183557"/></M_PNEXTEVENT>
<M_PASSOBJECTIVE name="m_passObjective" type="GameModeObjective" length="1" typedefid="380278911"><ARRAY value="10" typedefid="380278911"/></M_PASSOBJECTIVE>
<M_ELITEOBJECTIVE name="m_eliteObjective" type="GameModeObjective" length="1" typedefid="380278911"><ARRAY value="11" typedefid="380278911"/></M_ELITEOBJECTIVE>
</DATA></instance>
<instance instanceid="2" typedefid="-1915183557" name="2048 - Event 2"><DATA>
<M_PEVENTREQUIRED name="m_pEventRequired" type="GameModeBase" length="1" typedefid="366306753"><ARRAY value="1" typedefid="-1353052320"/></M_PEVENTREQUIRED>
<M_PASSOBJECTIVE name="m_passObjective" type="GameModeObjective" length="1" typedefid="380278911"><ARRAY value="20" typedefid="380278911"/></M_PASSOBJECTIVE>
<M_ELITEOBJECTIVE name="m_eliteObjective" type="GameModeObjective" length="1" typedefid="380278911"><ARRAY value="21" typedefid="380278911"/></M_ELITEOBJECTIVE>
</DATA></instance>
<instance instanceid="3" typedefid="1018671239" name="2049 - Event 1"><DATA>
<M_PEVENTREQUIRED name="m_pEventRequired" type="GameModeBase" length="1" typedefid="366306753"><ARRAY value="1" typedefid="-1353052320"/></M_PEVENTREQUIRED>
<M_PASSOBJECTIVE name="m_passObjective" type="GameModeObjective" length="1" typedefid="380278911"><ARRAY value="30" typedefid="380278911"/></M_PASSOBJECTIVE>
<M_ELITEOBJECTIVE name="m_eliteObjective" type="GameModeObjective" length="1" typedefid="380278911"><ARRAY value="31" typedefid="380278911"/></M_ELITEOBJECTIVE>
</DATA></instance>
<instance instanceid="10" typedefid="380278911" name="FinishRaceAnyPosition"><DATA>
<M_OBJECTIVETYPE name="m_ObjectiveType" type="ObjectiveValue" length="1" typedefid="-66037811"><ARRAY value="1" typedefid="-66037811"/></M_OBJECTIVETYPE>
<M_OBJECTIVETARGET name="m_ObjectiveTarget" type="u32" length="1" typedefid="-1854316044"><ARRAY value="" typedefid="-1854316044"/></M_OBJECTIVETARGET>
</DATA></instance>
<instance instanceid="11" typedefid="380278911" name="Win"><DATA>
<M_OBJECTIVETYPE name="m_ObjectiveType" type="ObjectiveValue" length="1" typedefid="-66037811"><ARRAY value="4" typedefid="-66037811"/></M_OBJECTIVETYPE>
<M_OBJECTIVETARGET name="m_ObjectiveTarget" type="u32" length="1" typedefid="-1854316044"><ARRAY value="1" typedefid="-1854316044"/></M_OBJECTIVETARGET>
</DATA></instance>
<instance instanceid="20" typedefid="380278911" name="2048 - Event 2 Pass"><DATA>
<M_OBJECTIVETYPE name="m_ObjectiveType" type="ObjectiveValue" length="1" typedefid="-66037811"><ARRAY value="2" typedefid="-66037811"/></M_OBJECTIVETYPE>
<M_OBJECTIVETARGET name="m_ObjectiveTarget" type="u32" length="1" typedefid="-1854316044"><ARRAY value="13000" typedefid="-1854316044"/></M_OBJECTIVETARGET>
</DATA></instance>
<instance instanceid="21" typedefid="380278911" name="2048 - Event 2 Elite"><DATA>
<M_OBJECTIVETYPE name="m_ObjectiveType" type="ObjectiveValue" length="1" typedefid="-66037811"><ARRAY value="2" typedefid="-66037811"/></M_OBJECTIVETYPE>
<M_OBJECTIVETARGET name="m_ObjectiveTarget" type="u32" length="1" typedefid="-1854316044"><ARRAY value="11000" typedefid="-1854316044"/></M_OBJECTIVETARGET>
</DATA></instance>
<instance instanceid="30" typedefid="380278911" name="Zone Pass"><DATA>
<M_OBJECTIVETYPE name="m_ObjectiveType" type="ObjectiveValue" length="1" typedefid="-66037811"><ARRAY value="2" typedefid="-66037811"/></M_OBJECTIVETYPE>
<M_OBJECTIVETARGET name="m_ObjectiveTarget" type="u32" length="1" typedefid="-1854316044"><ARRAY value="10" typedefid="-1854316044"/></M_OBJECTIVETARGET>
</DATA></instance>
<instance instanceid="31" typedefid="380278911" name="Zone Elite"><DATA>
<M_OBJECTIVETYPE name="m_ObjectiveType" type="ObjectiveValue" length="1" typedefid="-66037811"><ARRAY value="2" typedefid="-66037811"/></M_OBJECTIVETYPE>
<M_OBJECTIVETARGET name="m_ObjectiveTarget" type="u32" length="1" typedefid="-1854316044"><ARRAY value="20" typedefid="-1854316044"/></M_OBJECTIVETARGET>
</DATA></instance>
</mjolnir>"#;

    #[test]
    fn finish_and_position_objectives_grade_a_race_event() {
        let doc = parse(CAMPAIGN_FIXTURE);
        let event = events(&doc)
            .into_iter()
            .find(|e| e.name == "2048 - Event 1")
            .unwrap();
        let objectives = event_objectives(&doc, &event).unwrap();

        // Never finished: neither bar is decidable, so no tier at all.
        assert_eq!(evaluate_tier(&objectives, &EventOutcome::default()), None);
        // Finished, but outside the podium: FINISH's own bar has no target,
        // so finishing at all clears it.
        assert_eq!(
            evaluate_tier(
                &objectives,
                &EventOutcome {
                    finished: true,
                    place: 8,
                    ..EventOutcome::default()
                }
            ),
            Some(Tier::Pass)
        );
        // First place clears Win (target 1) too.
        assert_eq!(
            evaluate_tier(
                &objectives,
                &EventOutcome {
                    finished: true,
                    place: 1,
                    ..EventOutcome::default()
                }
            ),
            Some(Tier::Elite)
        );
    }

    #[test]
    fn beat_value_reads_finish_time_on_a_race_and_zone_count_on_a_zone() {
        let doc = parse(CAMPAIGN_FIXTURE);

        let race = events(&doc)
            .into_iter()
            .find(|e| e.name == "2048 - Event 2")
            .unwrap();
        let race_objectives = event_objectives(&doc, &race).unwrap();
        // Slower than both bars (pass 13000cs, elite 11000cs).
        assert_eq!(
            evaluate_tier(
                &race_objectives,
                &EventOutcome {
                    finished: true,
                    finish_centiseconds: Some(15000),
                    ..EventOutcome::default()
                }
            ),
            None
        );
        // Between the two: lower is better for a Race, so this clears Pass
        // but not Elite.
        assert_eq!(
            evaluate_tier(
                &race_objectives,
                &EventOutcome {
                    finished: true,
                    finish_centiseconds: Some(12000),
                    ..EventOutcome::default()
                }
            ),
            Some(Tier::Pass)
        );
        // A Zone medal never applies to a Race event even if the zone field
        // happened to carry a value that would clear it.
        assert_eq!(
            evaluate_tier(
                &race_objectives,
                &EventOutcome {
                    zone: 999,
                    ..EventOutcome::default()
                }
            ),
            None
        );

        let zone = events(&doc)
            .into_iter()
            .find(|e| e.name == "2049 - Event 1")
            .unwrap();
        let zone_objectives = event_objectives(&doc, &zone).unwrap();
        // Higher is better for Zone, and it is graded with no `finished` at
        // all - the counter only ever grows.
        assert_eq!(
            evaluate_tier(
                &zone_objectives,
                &EventOutcome {
                    zone: 5,
                    ..EventOutcome::default()
                }
            ),
            None
        );
        assert_eq!(
            evaluate_tier(
                &zone_objectives,
                &EventOutcome {
                    zone: 20,
                    ..EventOutcome::default()
                }
            ),
            Some(Tier::Elite)
        );
    }

    #[test]
    fn unlock_gates_collapse_a_chain_edge_and_a_required_edge_to_one_name() {
        let doc = parse(CAMPAIGN_FIXTURE);
        let gates = unlock_gates(&doc);

        let gate_of = |name: &str| {
            gates
                .iter()
                .find(|(event, _)| event == name)
                .and_then(|(_, gate)| gate.clone())
        };

        assert_eq!(
            gate_of("2048 - Event 1"),
            None,
            "no incoming edge and no M_PEVENTREQUIRED: open from the start"
        );
        assert_eq!(
            gate_of("2048 - Event 2"),
            Some("2048 - Event 1".to_string()),
            "reached by M_PNEXTEVENT alone"
        );
        assert_eq!(
            gate_of("2049 - Event 1"),
            Some("2048 - Event 1".to_string()),
            "reached by M_PEVENTREQUIRED alone, never chained"
        );
    }
}
