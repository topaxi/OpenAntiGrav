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
    Event, EventKind, Objective, ShipModel, Track, WeaponSet, events, objective_for, objectives,
    ship_model_for, ship_models, track_for, tracks, typedef, weapon_set_for, weapon_sets,
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

/// **2048 restricts craft choice on some events - measured, not the "open
/// pick" this build assumed before this pass.** `WOShipCreatorParams`
/// (`M_PPLAYERSHIPCREATORPARAMS`/`M_PGRIDSHIPCREATORPARAMS`) is the
/// mechanism a prior pass expected to carry this and is authored on **zero**
/// of `SP.xml`'s 141 events, player or grid - its own shape stays unread.
/// The real mechanism is two other, already-authored fields
/// `GameModeBase_RegisterFields` (`docs/ghidra/functions/vita-2048-eu-v104/game-mode-base-fields.md`)
/// names but this crate had not read before now:
///
/// - **`M_PPLAYERSHIPMODELDATA`** forces one specific craft. Authored on 14
///   of the 141 events: all five per-team `"* P Ship Challenge"` events (the
///   team's own craft, e.g. `"AG-Systems P Ship Challenge"` forces
///   `AG_System_Proto`), all four `E3_Demo*` builds (Feisar Speed) and five
///   ordinary numbered events (`"2048 - Event 4-2"`, `"2049 - Event 3-3"`,
///   `"2050 - Event 3-4"`, `"2050 - Event 6-1"`). [`forced_craft`] resolves
///   it onto a `race::Options::team` id.
/// - **`M_bPreventCombatShips`/`M_bPreventAgilityShips`/`M_bPreventSpeedShips`/`M_bPreventProtoShips`**
///   restrict player choice to a subset of [`crate::race::SHIP_TYPES`]
///   without forcing one. Authored `true` on 6 events, all in the "2050"
///   story arc bar one (`"2049 - Event 2-3"`): `"2050 - Event 3"`/`"2050 -
///   Event 3-2"` (no Combat/Agility), `"2050 - Event 5"`/`"2050 - Event
///   5-4"`/`"2049 - Event 2-3"` (no Agility/Speed) and `"2050 - Event 7"`
///   (no Combat/Speed). None of these 6 also forces a craft. [`craft_restriction`]
///   reads the four flags into a mask.
///
/// **[`forced_craft`] and [`grid_craft`] are both wired into a launch,
/// `restriction` is not.** Both land in `race::load_event`
/// (`crates/game/src/race/load/campaign.rs`): `forced_craft` overrides
/// `race::Options::team`, `grid_craft` overrides `race::Options::grid_teams`,
/// which `oag_game::race::load::roster::grid` applies over
/// `oag_game::livery::teams_for_slots`'s own roster draw, per AI slot.
/// The restriction mask does not enforce anywhere in this crate, because
/// which native screen would enforce it (grey a tile, clamp the cursor,
/// refuse the launch) was not found in `eboot.elf` this pass - see
/// `docs/formats/2048-campaign.md`'s "What is not determined" section.
/// Reported by the loader instead of silently ignored.
pub mod craft {
    use super::{Document, Instance, Reference, ship_model_for};

    /// [`crate::race::SHIP_TYPES`] index for a [`super::ShipModel::livery`]
    /// string. **Not a string match against [`crate::race::SHIP_TYPES`]
    /// itself** - `"combat"` is that table's own `"fighter"` slot (Feisar and
    /// Qirex spell the same slot differently, see that constant's own doc
    /// comment) and would silently miss every combat craft if compared
    /// directly. Order corroborated by `GameModeBase_RegisterFields`' own
    /// four prevent-flag offsets, `0x80`..`0x83` for
    /// combat/agility/speed/proto in that order, matching [`crate::race::TEAM_VARIANTS`]'
    /// own suffix order `1`..`4`.
    #[must_use]
    pub fn variant_index(livery: &str) -> Option<usize> {
        match livery {
            "combat" => Some(0),
            "agility" => Some(1),
            "speed" => Some(2),
            "prototype" => Some(3),
            _ => None,
        }
    }

    /// A [`super::ShipModel`] resolved onto a `race::Options::team` id -
    /// `crate::race::TEAM_VARIANTS`' own join, e.g. `("Feisar2048",
    /// "speed")` -> `Feisar2048\3`. `None` for `"WINGMAN"` (empty
    /// [`super::ShipModel::team`]) or a livery [`variant_index`] does not
    /// recognise.
    #[must_use]
    pub fn team_id(model: &super::ShipModel) -> Option<String> {
        if model.team.is_empty() {
            return None;
        }
        let index = variant_index(&model.livery)?;
        let variant = crate::race::TEAM_VARIANTS.variants[index];
        Some(
            crate::race::TEAM_VARIANTS
                .join
                .combine(&model.team, variant.suffix),
        )
    }

    /// `event`'s own `M_PPLAYERSHIPMODELDATA`, resolved straight onto a
    /// `race::Options::team` id - `None` when the field is unauthored (127 of
    /// 141 events) or the referenced [`super::ShipModel`] does not resolve
    /// (not observed on the real file).
    #[must_use]
    pub fn forced_craft(document: &Document, event: &Instance) -> Option<String> {
        let reference: Reference = event.field("M_PPLAYERSHIPMODELDATA")?.reference()?;
        let model = ship_model_for(document, reference)?;
        team_id(&model)
    }

    /// `event`'s own `M_PGRIDSHIPMODELDATA`, resolved slot-by-slot onto
    /// `race::Options::grid_teams` - `oag_game::race::load_event`'s own
    /// per-AI-slot override of `oag_game::livery::teams_for_slots`'s
    /// roster draw (Pulse's law, inherited). One entry per `<ARRAY>` child the
    /// field carries, in document order; `None` at an index whose slot is
    /// unauthored (an empty `value=`) or whose [`super::ShipModel`]
    /// reference does not resolve to a `(team, livery)` [`team_id`]
    /// recognises - a caller applies this as a **per-slot** overlay, so a
    /// `None` leaves that one slot exactly what it would have been anyway,
    /// rather than forcing anything.
    ///
    /// **Not [`super::Field::references`]**, which drops an unauthored
    /// slot's own position along with its empty value - the wrong shape
    /// here, where slot 3's team must stay slot 3's answer even when slot 1
    /// authors nothing. This reads `Instance::field`'s own `values` directly
    /// to keep every position.
    ///
    /// Empty when the field itself is entirely unauthored - 84 of `SP.xml`'s
    /// 141 events, measured directly against the real EU v1.04 file. Of the
    /// other 57: 55 author all 7 slots, each one resolving to a real craft;
    /// 2 (`"2050 - Event 3-4"`/`"2050 - Event 6-4"`) author only their own
    /// single `<ARRAY>` child (slot 0), the same "most fields carry exactly
    /// one even when `length` says more" shape `oag_tables::mjolnir`'s own
    /// module doc comment already records for other fields - no event
    /// measured leaves a *gap* inside an otherwise-full 7-array, but nothing
    /// in the schema forbids one, so this handles that shape too rather than
    /// assuming the measured cases are the only ones.
    #[must_use]
    pub fn grid_craft(document: &Document, event: &Instance) -> Vec<Option<String>> {
        let Some(field) = event.field("M_PGRIDSHIPMODELDATA") else {
            return Vec::new();
        };
        field
            .values
            .iter()
            .map(|value| {
                if value.value.is_empty() {
                    return None;
                }
                let reference = Reference {
                    instance_id: value.value.trim().parse().ok()?,
                    typedef_id: value.typedef_id,
                };
                let model = ship_model_for(document, reference)?;
                team_id(&model)
            })
            .collect()
    }

    /// Which of [`crate::race::SHIP_TYPES`] `event`'s own four
    /// `M_bPrevent*Ships` flags forbid, in that table's own order. Every flag
    /// defaults to allowed (`false`/absent) - 135 of 141 events return all
    /// four `false`.
    #[must_use]
    pub fn restriction(event: &Instance) -> [bool; 4] {
        [
            "M_BPREVENTCOMBATSHIPS",
            "M_BPREVENTAGILITYSHIPS",
            "M_BPREVENTSPEEDSHIPS",
            "M_BPREVENTPROTOSHIPS",
        ]
        .map(|tag| event.field(tag).and_then(super::Field::bool) == Some(true))
    }

    /// [`variant_index`]'s own order, the other way round - `SHIP_TYPES`'
    /// four slots spelled the way `M_LIVERY`/[`restriction`] spell them
    /// (`"combat"`, not [`crate::race::SHIP_TYPES`]'s own `"fighter"`).
    const LIVERY_BY_INDEX: [&str; 4] = ["combat", "agility", "speed", "prototype"];

    /// Every one of [`crate::race::NATIVE_TEAMS`]' twenty craft `event`'s own
    /// [`restriction`] refuses, as `team_id` spells one (`crate::race::TEAM_VARIANTS`'
    /// own join, e.g. `"Qirex2048\4"`) - empty for every one of `SP.xml`'s 135
    /// unrestricted events. **Never names a guest (HD-roster) team.**
    /// `GameModeBase_IsShipTypeAllowed` (`0x812b41da`,
    /// `docs/ghidra/functions/vita-2048-eu-v104/game-mode-base-fields.md`)
    /// falls through to its own `return true` once none of `"combat"`/
    /// `"agility"`/`"speed"`/`"prototype"` match the queried livery - a guest
    /// team's own `M_LIVERY` resolves to `"fe_hd_livery_normal"`
    /// (`FUN_8105b6c2`), which matches none of the four, so a guest craft is
    /// always allowed on every event this file authors, restricted or not.
    ///
    /// **A prototype craft is not a fifth, independent class.** The same
    /// function only refuses a `"prototype"` query outright when
    /// `M_bPreventProtoShips` itself is set; otherwise it looks up the
    /// querying team's own prototype [`super::ShipModel`] and re-checks its
    /// [`super::ShipModel::prototype_livery`] instead - measured off the real
    /// `SP.xml`: `AG_System_Proto`/`Auricom_Proto`/`Feisar_Proto`/
    /// `Piranha_Proto`/`Qirex_Proto` each carry exactly one of `"Agility"`/
    /// `"Combat"`/`"Speed"`, never `"Prototype"` itself. So Qirex's own proto
    /// craft (`"Combat"`) is refused by `"2050 - Event 7"` (no Combat, no
    /// Speed) exactly as `Qirex_Combat` is, and allowed by `"2050 - Event 5"`
    /// (no Agility, no Speed) exactly as `Qirex_Combat` is - confidence 85,
    /// the decompile is literal but this was not watched running.
    #[must_use]
    pub fn refused_craft(document: &Document, event: &Instance) -> Vec<String> {
        let restriction = restriction(event);
        if restriction == [false; 4] {
            return Vec::new();
        }
        let models = super::ship_models(document);
        let mut refused = Vec::new();
        for team in crate::race::NATIVE_TEAMS {
            for (index, variant) in crate::race::TEAM_VARIANTS.variants.iter().enumerate() {
                if !class_allowed(&models, restriction, team, LIVERY_BY_INDEX[index]) {
                    refused.push(
                        crate::race::TEAM_VARIANTS
                            .join
                            .combine(team, variant.suffix),
                    );
                }
            }
        }
        refused
    }

    /// [`refused_craft`]'s own recursion, once a `(team, livery)` pair is
    /// already resolved - see that function's own doc for the prototype
    /// substitution this repeats for. `livery` is matched case-insensitively
    /// against [`variant_index`]'s own lower-case spelling: [`super::ShipModel::prototype_livery`]
    /// is title-cased (`"Combat"`) where every other caller of this already
    /// spells it lower-case.
    fn class_allowed(
        models: &[super::ShipModel],
        restriction: [bool; 4],
        team: &str,
        livery: &str,
    ) -> bool {
        let lower = livery.to_ascii_lowercase();
        let Some(index) = variant_index(&lower) else {
            return true;
        };
        if lower != "prototype" {
            return !restriction[index];
        }
        if restriction[3] {
            return false;
        }
        let Some(base) = models
            .iter()
            .find(|model| model.team.eq_ignore_ascii_case(team) && model.livery == "prototype")
            .map(|model| model.prototype_livery.trim())
            .filter(|base| !base.is_empty())
        else {
            return false;
        };
        if base.eq_ignore_ascii_case("prototype") {
            return false;
        }
        class_allowed(models, restriction, team, base)
    }
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

pub mod callout;

/// The art and words of the event card's trophy page (page kind `2`,
/// `CampaignEventCard_DrawTrophyPage_q`, `0x81052fb4`), chosen the way the
/// original chooses them: by event name, then by `M_BUTTONSHAPE`.
///
/// The nine elite-trophy events are matched by name, in this order, and the
/// year's ordinal is the position within its year (`FUN_8104d1d0` loads
/// `trophy/{2048+i/3}_elite_{i%3+1:02}`). Any other event whose button shape
/// is `9`, `10` or `11` is that year's cup final.
pub mod trophy {
    /// The nine events that award an elite trophy, in the original's order.
    pub const NAMED: [&str; 9] = [
        "2048 - Event 2-2",
        "2048 - Event 3-2",
        "2048 - Event 5-2",
        "2049 - Event 2-3",
        "2049 - Event 5-3",
        "2049 - Event 6-3",
        "2050 - Event 2-4",
        "2050 - Event 5-4",
        "2050 - Event 6-4",
    ];

    /// Button shapes that give an event a trophy page at all
    /// (`CampaignEventCard_BuildPageList`): `1` and `2` plus the three cups.
    #[must_use]
    pub fn has_page(button_shape: Option<i64>) -> bool {
        matches!(button_shape, Some(1 | 2 | 9 | 10 | 11))
    }

    /// What the page draws.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct Art {
        /// The image, spelled the way every card texture is.
        pub texture: String,
        /// The heading's string id (`TROPHY_2048_1_1`, `Cup_Name_2048`).
        pub header_id: String,
        /// The callout's string id (`TROPHY_2048_1_2`, `Cup_Callout_2048`).
        pub callout_id: String,
    }

    /// The trophy page's art for the event called `name` with `button_shape`.
    /// `None` is the original drawing nothing: a shape-`1`/`2` event that is
    /// not one of the nine still has the (blank) page.
    #[must_use]
    pub fn art_for(name: &str, button_shape: Option<i64>) -> Option<Art> {
        if let Some(i) = NAMED.iter().position(|named| *named == name) {
            let (year, n) = (2048 + i / 3, i % 3 + 1);
            return Some(Art {
                texture: format!(r"Data\FE\NewImages\trophy\{year}_elite_{n:02}.gtf"),
                header_id: format!("TROPHY_{year}_{n}_1"),
                callout_id: format!("TROPHY_{year}_{n}_2"),
            });
        }
        let year = match button_shape? {
            9 => 2048,
            10 => 2049,
            11 => 2050,
            _ => return None,
        };
        Some(Art {
            texture: format!(r"Data\FE\NewImages\trophy\Cup{year}.gtf"),
            header_id: format!("Cup_Name_{year}"),
            callout_id: format!("Cup_Callout_{year}"),
        })
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

/// The single campaign event whose completion unlocks each event, by name -
/// `(event name, its own prerequisite's name)`, `None` for an event open
/// from the start. **Every event in the document, not only the map's own
/// 115 with a cell** - `MP_*`/`E3_*` instances are included here and read as
/// gate-free, since nothing ever names one as a prerequisite either; a
/// caller building the map (`oag_game`'s own `boot::campaign2048::map_events`)
/// applies its own cell/`E3_*` filter afterward, the same as it always has.
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
/// Of the map's own 115 cell-bearing, non-`E3_*` events, a fresh save opens
/// 16 from the start on the EU v1.04 package: `"2048 - Event 1"` and 15 side
/// events with no gate at all (a per-team `"* P Ship Challenge"` and a
/// per-track `"* - S Phantom Challenge"`) - `M_bForceAlwaysUnlocked` is
/// authored on no event at all (measured, zero non-empty/non-zero
/// instances), so it is not what opens these; the absence of any incoming
/// edge or `M_PEVENTREQUIRED` is. See
/// `crates/game/tests/vita_2048_campaign_progress_ground_truth.rs`, which
/// pins this count against the map's own filtered set rather than this
/// function's raw one.
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
mod tests;
