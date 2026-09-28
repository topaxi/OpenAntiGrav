//! Typed view of [`super::Document`] for Wipeout 2048's own campaign schema -
//! the shape `data/xml/SP.xml` and `data/xml/MP.xml` both author. See the
//! parent module's doc comment for the full typedef census and how each name
//! below was read.
//!
//! **Entry names and what 2048 ships live in `oag_2048::campaign`, per
//! [ADR-0022] - this module knows the file's shape, not that Wipeout 2048 is
//! the title that ships it.**
//!
//! [ADR-0022]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0022-title-packages.md

use super::{Document, Instance, Reference};

/// Typedef ids this module resolves, named where `SP.xml`'s own
/// `type=`/`typedefid=` pairs name them and left as bare constants where they
/// do not - see [`super`]'s own doc comment for which is which and the
/// confidence on each.
pub mod typedef {
    /// `GameModeObjective`, confidence 95 (named in-file). 96 instances:
    /// pass/elite objectives an event's own [`super::super::Field`] pair
    /// (`M_PASSOBJECTIVE`/`M_ELITEOBJECTIVE`) points at - see [`Objective`],
    /// which reads `M_OBJECTIVETYPE`/`M_OBJECTIVETARGET` as raw numbers.
    /// **What each `ObjectiveValue` ordinal (`1`, `2`, `4`, `7`, and one
    /// empty, measured across the 96) means is not this module's to say** -
    /// that is `oag_2048::campaign`'s own reading, measured against 2048's
    /// particular `(pass, elite)` type pairs per event kind; see
    /// `docs/formats/2048-campaign.md`'s "The objective law" section.
    pub const GAME_MODE_OBJECTIVE: i64 = 380278911;
    /// `GameModeBase`, confidence 95 (named in-file, as the abstract static
    /// type of every polymorphic reference field). No instance in `SP.xml`
    /// carries this as its own `typedefid` - every real event is one of
    /// [`RACE_A`], [`RACE_B`], [`ELIMINATION`] or [`ZONE`].
    pub const GAME_MODE_BASE: i64 = 366306753;
    /// `TrackDefinition`, confidence 95 (named in-file; also an exact name
    /// match against `data/plugins/tracks/Definition.xml`'s own ten
    /// `<PI_Track name="...">` stems). 10 instances - see [`Track`].
    ///
    /// **This is the typedef a prior pass guessed [`WEAPON_SET_DEFINITION`]
    /// was** ("2048 ships 20 track profiles"); the guess was wrong on the
    /// count alone (2048 ships ten circuits, not twenty) and the file's own
    /// field-type pairs settle it either way.
    pub const TRACK_DEFINITION: i64 = 205052969;
    /// `WeaponSetDefinition`, confidence 95 (named in-file). 20 instances,
    /// each one `M_WEAPONAVAILABLEBITS` field carrying a single value this
    /// module does not decode - `WeaponType`'s own bit meanings are
    /// unresolved. See [`WeaponSet`].
    pub const WEAPON_SET_DEFINITION: i64 = -966434245;
    /// `WOShipModelData`, confidence 95 (named in-file). 21 instances: a
    /// team+livery craft catalogue (`M_TEAM`/`M_LIVERY`, e.g.
    /// `"Feisar2048"`/`"speed"`) with its own rank-unlock ladder
    /// (`M_RANKUNLOCK`) or campaign-unlock edge (`M_PCAMPAIGNUNLOCK`) - the
    /// roster screen's own unlock table. **Not the rare case it first looked
    /// like**: `M_PPLAYERSHIPMODELDATA` is authored (non-empty) on 14 of
    /// `SP.xml`'s 141 events - forcing that event's player craft - and
    /// `M_PGRIDSHIPMODELDATA` on most numbered events, sizing the AI grid
    /// explicitly. See [`ShipModel`] and `oag_2048::campaign` for what
    /// forcing a craft this way means for a launch.
    pub const SHIP_MODEL_DATA: i64 = 520725191;
    /// One of two "lap race" typedefs. **2026-09-28: this is
    /// `GameMode_ArcadeRace`'s and `GameMode_SpeedLapRace`'s own typedef,
    /// confirmed at confidence 92** - `oag_formats::wad::hash_name` of the
    /// literal string `"GameMode_ArcadeRace"` hashes to `-1353052320`
    /// ([`RACE_B`]) and `"GameMode_SpeedLapRace"` to `-1915183557` (this
    /// one), the same case-folded-CRC-32 convention every other named
    /// typedef in this module's own census already matches, zero exceptions
    /// across ten names checked - see `docs/formats/2048-campaign.md`'s
    /// "Craft choice"-adjacent typedef table for the full cross-reference.
    /// This is `GameMode_SpeedLapRace`: 53 instances, the only typedef whose
    /// instances ever carry a non-empty `M_MAXGHOSTSHIPS` field (all 53 do;
    /// neither [`RACE_B`] nor [`ELIMINATION`] nor [`ZONE`] carries that field
    /// at all) - a real, checkable discriminator that now has a name to
    /// match it: a dedicated Speed Lap class plausibly needs its own ghost
    /// capacity where an ordinary race does not. Also the only typedef
    /// carrying `laps == 0`: all 40 of the `SP.xml` instances literally named
    /// `"<Track> Speed Lap - <Class>"` are `RACE_A` with `M_NUMOFLAPS`
    /// absent/zero; `RACE_A`'s other 13 instances are ordinary 2-3 lap
    /// `"20XX - Event N"` nodes. See [`EventKind::Race`] and [`Event::laps`].
    pub const RACE_A: i64 = -1915183557;
    /// The other "lap race" typedef - `GameMode_ArcadeRace`, per [`RACE_A`]'s
    /// own doc comment. 52 instances, laps always >= 1 (never a Speed Lap
    /// sentinel), no `M_MAXGHOSTSHIPS`. Its instance names include
    /// `E3_Demo_*` builds, `MP_*_Race_flash` templates and ten-lap
    /// `"* Ship Challenge"` events alongside ordinary numbered ones - the
    /// generic race class's own shape, corroborating the hash match from the
    /// data side.
    ///
    /// **`RACE_A` and `RACE_B` are two different concrete classes, but this
    /// project still does not split them by game mode.** `M_PNEXTEVENT`
    /// chains cross freely between them (`"2048 - Event 3"`, `RACE_A`, its
    /// own `M_PNEXTEVENT` names `"2048 - Event 4"`, `RACE_B`) - resolving the
    /// class names shows this was never evidence the two typedefs were the
    /// *same* mode, only that the unlock graph chains across concrete
    /// classes freely (an ordinary race unlocking a Speed Lap attraction is
    /// unremarkable). [`EventKind::Race`] still does not distinguish them,
    /// because `oag_2048::campaign::engine_mode` already derives
    /// `"speed_lap"` vs `"single_race"` independently off the `laps ==
    /// Some(0)` sentinel - the class name adds understanding, not a gate this
    /// project's own mode resolution needs; [`Event::typedef_id`] keeps the
    /// raw id for a caller that wants it.
    ///
    /// The remaining two `GameMode_*` names in `eboot.elf`'s string table,
    /// `GameMode_CheckPointRace` (hash `375161732`) and `GameMode_ZombieRace`
    /// (hash `-1251488984`), match **no** typedef id `SP.xml` carries - both
    /// classes ship in the executable but no campaign event in this file
    /// instantiates either.
    pub const RACE_B: i64 = -1353052320;
    /// Elimination-shaped. 26 instances, the only typedef carrying
    /// `M_ELIMINATENUMOFOPPONENTS`/`M_SCORETARGET`/`M_SURVIVEFORNUMOFLAPS`/
    /// `M_TIMELIMIT`/`M_BSEEKANDDESTROYTARGETID`/`M_BSOLOSCORING`, and
    /// instance names spelling `"Elim"`/`"Eliminator"` explicitly
    /// (`"MPElimination"`, `"MP_Arena_Eliminator_flash"`). **2026-09-28:
    /// confirmed structurally, not just by instance-name resemblance** -
    /// `hash_name("GameMode_EliminatorRace")` equals this id exactly, same
    /// evidence as [`RACE_A`]/[`RACE_B`]'s own confirmation. Confidence
    /// raised to 92.
    pub const ELIMINATION: i64 = 1311982788;
    /// Zone-shaped. 10 instances, the only typedef carrying
    /// `M_ZONETIMECOUNTER`/`M_ZONETOADDMINES`/`M_STARTZONENUMBER`/
    /// `M_ENDZONENUMBER`/`M_NUMBEROFMINES`, and the only one whose
    /// `M_SPEEDCLASS` is always empty (Zone has no speed class).
    /// **2026-09-28: confirmed structurally** - `hash_name("GameMode_ZoneRace")`
    /// equals this id exactly, same evidence as [`RACE_A`]/[`RACE_B`]'s own
    /// confirmation. Confidence raised to 92.
    pub const ZONE: i64 = 1018671239;
}

/// Which of the four concrete event shapes an [`Event`] is.
///
/// **Deliberately coarser than the four `GameMode_*` C++ classes `SP.xml`'s
/// typedefs actually bind to**, now that binding is resolved (confidence 92 -
/// see [`typedef::RACE_A`]'s own doc comment for the hash cross-reference):
/// `GameMode_SpeedLapRace`, `GameMode_ArcadeRace`, `GameMode_EliminatorRace`
/// and `GameMode_ZoneRace`. [`Self::Race`] still merges the first two, since
/// nothing this project does needs them apart - `oag_2048::campaign::
/// engine_mode` derives `"speed_lap"` vs `"single_race"` independently off
/// [`Event::laps`]'s own sentinel, not off which class an event's typedef
/// names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventKind {
    /// [`typedef::RACE_A`] or [`typedef::RACE_B`]. A lap race, including the
    /// `laps == 0` Speed Lap sentinel - see [`Event::laps`].
    Race,
    /// [`typedef::ELIMINATION`].
    Elimination,
    /// [`typedef::ZONE`].
    Zone,
}

impl EventKind {
    fn from_typedef(typedef_id: i64) -> Option<Self> {
        match typedef_id {
            typedef::RACE_A | typedef::RACE_B => Some(Self::Race),
            typedef::ELIMINATION => Some(Self::Elimination),
            typedef::ZONE => Some(Self::Zone),
            _ => None,
        }
    }
}

/// One campaign event: a `RACE_A`/`RACE_B`/`ELIMINATION`/`ZONE` instance,
/// with the fields a caller needs to launch it or walk the unlock graph.
///
/// Deliberately not every field the schema carries - AI tuning
/// (`M_AIABSORBMULTIPLIER` and siblings), UI placement
/// (`M_BUTTONSHAPE`/`M_CANVASTWEAK_X`/`M_CANVASTWEAK_Y`) and the explicit
/// grid/player ship references are left on [`Instance::field`] for a caller
/// that needs them rather than repeated here - see [`Event::instance_id`]
/// and look them up on the [`Document`] this came from.
#[derive(Debug, Clone, PartialEq)]
pub struct Event {
    /// This event's own `instanceid`.
    pub instance_id: i64,
    /// The raw `typedefid` - [`typedef::RACE_A`], [`typedef::RACE_B`],
    /// [`typedef::ELIMINATION`] or [`typedef::ZONE`]. [`Event::kind`]
    /// collapses the two race typedefs; this keeps them apart.
    pub typedef_id: i64,
    /// [`EventKind`] for `typedef_id`.
    pub kind: EventKind,
    /// The instance's own `name=`, e.g. `"2048 - Event 3"`,
    /// `"Altima Speed Lap - Flash"`, `"MPElimination"`.
    pub name: String,
    /// `M_DESCRIPTION`: an idstring like `"2048_EVENT_3"` - a language
    /// table lookup key, not human text; this module does not resolve it.
    pub description: Option<String>,
    /// `M_TRACKDEF`, an instanceid reference into [`typedef::TRACK_DEFINITION`].
    /// Resolve with [`Document::instance`] and [`Track::from_instance`], or
    /// [`track_for`]. `None` on every `ZONE` event this pass measured (Zone
    /// events author no track reference in `SP.xml` - the circuit a Zone run
    /// uses is a title fact, not authored per event; see `oag_2048::campaign`).
    pub track: Option<Reference>,
    /// `M_SPEEDCLASS`'s raw `eClass` ordinal, `0`-`4`. **Not named here** -
    /// see `oag_2048::campaign::EClass` for the five-class mapping, measured
    /// against this title's own `handlingstats.xml` and campaign event
    /// names. `None` on every `ZONE` event (no speed class authored).
    pub speed_class: Option<i64>,
    /// `M_NUMOFLAPS`. **`Some(0)` is Speed Lap's own sentinel** - see
    /// [`typedef::RACE_A`]'s own doc comment: all 40 `SP.xml` instances with
    /// `laps == Some(0)` are named `"<Track> Speed Lap - <Class>"` and none
    /// of the other 101 lap-race instances author it. `None` on every
    /// `ELIMINATION`/`ZONE` event, which use their own completion fields
    /// instead (`M_SURVIVEFORNUMOFLAPS`/`M_TIMELIMIT`/`M_ZONETIMECOUNTER`
    /// and siblings - not carried on [`Event`], see the struct's own doc
    /// comment).
    pub laps: Option<u32>,
    /// `M_WEAPONSET`, an instanceid reference into
    /// [`typedef::WEAPON_SET_DEFINITION`]. Resolve with [`Document::instance`]
    /// and [`WeaponSet::from_instance`], or [`weapon_set_for`].
    pub weapon_set: Option<Reference>,
    /// `M_PNEXTEVENT`: the campaign chain's own "what comes after this".
    pub next_event: Option<Reference>,
    /// `M_PBRANCHEVENT`: a second, sibling event off the same node -
    /// `"2048 - Event 3"`'s own branch is `"2048 - Event 3-1"`, both
    /// [`typedef::RACE_A`].
    pub branch_event: Option<Reference>,
    /// `M_PEVENTREQUIRED`: a prerequisite outside the `next`/`branch` chain.
    pub required_event: Option<Reference>,
    /// `M_PASSOBJECTIVE`, an instanceid reference into
    /// [`typedef::GAME_MODE_OBJECTIVE`].
    pub pass_objective: Option<Reference>,
    /// `M_ELITEOBJECTIVE`, the same as [`Event::pass_objective`] for the
    /// harder target.
    pub elite_objective: Option<Reference>,
    /// `M_X`/`M_Y`: the event's own map-grid position. **Not the campaign
    /// map's own pixel projection** - `docs/ghidra/functions/vita-2048-eu-v104/frontend-campaign-map.md`'s
    /// scale/bias/offset tables are the DLC `HD CAMPAIGN`/`FURY CAMPAIGN`
    /// tiers' own `FE3DCanvas` hotspots, a different node set from this
    /// one; nothing in `SP.xml` itself authors how `(x, y)` here becomes a
    /// screen position, and this module does not guess one.
    pub x: Option<i32>,
    /// See [`Event::x`].
    pub y: Option<i32>,
    /// `M_RankRequired`: the player rank an event needs, when authored.
    pub rank_required: Option<i32>,
    /// `M_bForceAlwaysUnlocked`.
    pub force_always_unlocked: Option<bool>,
    /// Whether `M_MAXGHOSTSHIPS` is present on this instance at all -
    /// [`typedef::RACE_A`]'s own discriminator, true for every `RACE_A`
    /// event and no other kind. Not the field's *value* (every instance
    /// this pass measured leaves it empty even when present) - just whether
    /// the schema offers it.
    pub has_ghost_capacity: bool,
}

impl Event {
    /// Builds an [`Event`] from an [`Instance`] of one of the four event
    /// typedefs. `None` for any other typedef.
    #[must_use]
    pub fn from_instance(instance: &Instance) -> Option<Self> {
        let kind = EventKind::from_typedef(instance.typedef_id)?;

        Some(Self {
            instance_id: instance.instance_id,
            typedef_id: instance.typedef_id,
            kind,
            name: instance.name.clone(),
            description: instance
                .field("M_DESCRIPTION")
                .and_then(super::Field::value)
                .filter(|v| !v.is_empty())
                .map(str::to_string),
            track: instance
                .field("M_TRACKDEF")
                .and_then(super::Field::reference),
            speed_class: instance.field("M_SPEEDCLASS").and_then(super::Field::int),
            laps: instance
                .field("M_NUMOFLAPS")
                .and_then(super::Field::int)
                .and_then(|v| u32::try_from(v).ok()),
            weapon_set: instance
                .field("M_WEAPONSET")
                .and_then(super::Field::reference),
            next_event: instance
                .field("M_PNEXTEVENT")
                .and_then(super::Field::reference),
            branch_event: instance
                .field("M_PBRANCHEVENT")
                .and_then(super::Field::reference),
            required_event: instance
                .field("M_PEVENTREQUIRED")
                .and_then(super::Field::reference),
            pass_objective: instance
                .field("M_PASSOBJECTIVE")
                .and_then(super::Field::reference),
            elite_objective: instance
                .field("M_ELITEOBJECTIVE")
                .and_then(super::Field::reference),
            x: instance
                .field("M_X")
                .and_then(super::Field::int)
                .and_then(|v| i32::try_from(v).ok()),
            y: instance
                .field("M_Y")
                .and_then(super::Field::int)
                .and_then(|v| i32::try_from(v).ok()),
            rank_required: instance
                .field("M_RankRequired")
                .and_then(super::Field::int)
                .and_then(|v| i32::try_from(v).ok()),
            force_always_unlocked: instance
                .field("M_bForceAlwaysUnlocked")
                .and_then(super::Field::bool),
            has_ghost_capacity: instance.field("M_MAXGHOSTSHIPS").is_some(),
        })
    }
}

/// Every event in a [`Document`] - every instance of [`typedef::RACE_A`],
/// [`typedef::RACE_B`], [`typedef::ELIMINATION`] and [`typedef::ZONE`], in
/// document order. **Document order, not campaign order** - nothing in this
/// schema authors a play sequence directly; walk [`Event::next_event`]/
/// [`Event::branch_event`]/[`Event::required_event`] for that, starting from
/// whichever event a caller already knows is first (this module does not
/// pick one - `SP.xml` names no "root" event either).
#[must_use]
pub fn events(document: &Document) -> Vec<Event> {
    document
        .instances
        .iter()
        .filter_map(Event::from_instance)
        .collect()
}

/// One `TrackDefinition`: a circuit's campaign-facing name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Track {
    /// This instance's own `instanceid` - what an [`Event::track`] reference
    /// names.
    pub instance_id: i64,
    /// The instance's own `name=`, e.g. `"Bridge"`.
    pub name: String,
    /// `M_TRACKNAME`, e.g. `"bridge"` - the lowercase stem
    /// `data/plugins/tracks/Definition.xml`'s own `<PI_Track name="...">`
    /// and `oag_title::Title::track_plugin_definition` both key on. See
    /// `oag_2048::campaign` for the join.
    pub track_name: String,
    /// `M_DISPLAYNAME`, e.g. `"CAPITAL REACH"` - the in-game circuit name.
    pub display_name: String,
}

impl Track {
    /// Builds a [`Track`] from an [`Instance`] of [`typedef::TRACK_DEFINITION`].
    /// `None` for any other typedef, or if either field is absent.
    #[must_use]
    pub fn from_instance(instance: &Instance) -> Option<Self> {
        if instance.typedef_id != typedef::TRACK_DEFINITION {
            return None;
        }
        Some(Self {
            instance_id: instance.instance_id,
            name: instance.name.clone(),
            track_name: instance.field("M_TRACKNAME")?.value()?.to_string(),
            display_name: instance
                .field("M_DISPLAYNAME")
                .and_then(super::Field::value)
                .unwrap_or_default()
                .to_string(),
        })
    }
}

/// Every [`Track`] in a document, in document order.
#[must_use]
pub fn tracks(document: &Document) -> Vec<Track> {
    document
        .instances
        .iter()
        .filter_map(Track::from_instance)
        .collect()
}

/// [`Event::track`] resolved against its own document - `None` if the
/// reference is absent or dangling.
#[must_use]
pub fn track_for(document: &Document, reference: Reference) -> Option<Track> {
    Track::from_instance(document.instance(reference.instance_id)?)
}

/// One `WeaponSetDefinition`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WeaponSet {
    /// This instance's own `instanceid`.
    pub instance_id: i64,
    /// The instance's own `name=`, e.g. `"Rockets Only"`,
    /// `"Cannons, Missile, Plasma"`.
    pub name: String,
    /// `M_WEAPONAVAILABLEBITS`'s raw value. **Unresolved**: `WeaponType`'s
    /// own bit-to-weapon mapping was not chased this pass - the name alone
    /// (`"Rockets Only"` carries `1`, for instance) is the only evidence
    /// recorded, not a decoded mask.
    pub available_bits: Option<i64>,
}

impl WeaponSet {
    /// Builds a [`WeaponSet`] from an [`Instance`] of
    /// [`typedef::WEAPON_SET_DEFINITION`]. `None` for any other typedef.
    #[must_use]
    pub fn from_instance(instance: &Instance) -> Option<Self> {
        if instance.typedef_id != typedef::WEAPON_SET_DEFINITION {
            return None;
        }
        Some(Self {
            instance_id: instance.instance_id,
            name: instance.name.clone(),
            available_bits: instance
                .field("M_WEAPONAVAILABLEBITS")
                .and_then(super::Field::int),
        })
    }
}

/// Every [`WeaponSet`] in a document, in document order.
#[must_use]
pub fn weapon_sets(document: &Document) -> Vec<WeaponSet> {
    document
        .instances
        .iter()
        .filter_map(WeaponSet::from_instance)
        .collect()
}

/// [`Event::weapon_set`] resolved against its own document.
#[must_use]
pub fn weapon_set_for(document: &Document, reference: Reference) -> Option<WeaponSet> {
    WeaponSet::from_instance(document.instance(reference.instance_id)?)
}

/// One `GameModeObjective`: what [`Event::pass_objective`]/
/// [`Event::elite_objective`] point at.
///
/// **Names only, no decoding of what the ordinal or the target mean** - per
/// this module's own split with `oag_2048::campaign`, which is where
/// `M_OBJECTIVETYPE`'s own `1`/`2`/`4`/`7` ordinals are given a meaning,
/// against 2048's own measured `(pass, elite)` type pairs per
/// [`EventKind`]. See `docs/formats/2048-campaign.md`'s "The objective law"
/// section for the census this was read off.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Objective {
    /// This instance's own `instanceid`.
    pub instance_id: i64,
    /// The instance's own `name=`, e.g. `"FinishRaceAnyPosition"`,
    /// `"Top5"`, `"2048 - Event 3 Pass"`.
    pub name: String,
    /// `M_OBJECTIVETYPE`'s raw `ObjectiveValue` ordinal. `None` on the one
    /// measured instance (`"MostDamage"`) that authors no type at all.
    pub objective_type: Option<i64>,
    /// `M_OBJECTIVETARGET`. `None` both for a type that authors no target at
    /// all (the `FINISH` type's own instances leave it empty) and for an
    /// absent field.
    pub target: Option<i64>,
}

impl Objective {
    /// Builds an [`Objective`] from an [`Instance`] of
    /// [`typedef::GAME_MODE_OBJECTIVE`]. `None` for any other typedef.
    #[must_use]
    pub fn from_instance(instance: &Instance) -> Option<Self> {
        if instance.typedef_id != typedef::GAME_MODE_OBJECTIVE {
            return None;
        }
        Some(Self {
            instance_id: instance.instance_id,
            name: instance.name.clone(),
            objective_type: instance
                .field("M_OBJECTIVETYPE")
                .and_then(super::Field::int),
            target: instance
                .field("M_OBJECTIVETARGET")
                .and_then(super::Field::int),
        })
    }
}

/// Every [`Objective`] in a document, in document order.
#[must_use]
pub fn objectives(document: &Document) -> Vec<Objective> {
    document
        .instances
        .iter()
        .filter_map(Objective::from_instance)
        .collect()
}

/// [`Event::pass_objective`]/[`Event::elite_objective`] resolved against
/// their own document.
#[must_use]
pub fn objective_for(document: &Document, reference: Reference) -> Option<Objective> {
    Objective::from_instance(document.instance(reference.instance_id)?)
}

/// One `WOShipModelData`: a team+livery craft the roster screen offers, or
/// what an event's own `M_PPLAYERSHIPMODELDATA`/`M_PGRIDSHIPMODELDATA`
/// reference points at when authored - see `oag_2048::campaign` for what
/// forcing a specific one onto an event means and how a `(team, livery)` pair
/// resolves onto `race::Options::team`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShipModel {
    /// This instance's own `instanceid`.
    pub instance_id: i64,
    /// The instance's own `name=`, e.g. `"Feisar_Speed"`. `"WINGMAN"` is the
    /// one instance with an empty [`Self::team`]/[`Self::livery`] - not part
    /// of any of the five native teams' four-craft roster.
    pub name: String,
    /// `M_TEAM`, e.g. `"Feisar2048"` - matches `oag_2048::race::NATIVE_TEAMS`
    /// exactly. Empty on `"WINGMAN"`.
    pub team: String,
    /// `M_LIVERY`, e.g. `"speed"`, `"combat"`, `"agility"`, `"prototype"`.
    /// Empty on `"WINGMAN"`.
    pub livery: String,
    /// `M_PROTOTYPELIVERY` - authored only on the five `livery == "prototype"`
    /// instances, one per team (`"Agility"`/`"Combat"`/`"Speed"`, title-cased
    /// unlike [`Self::livery`]'s own lower-case spelling). Empty everywhere
    /// else. See `oag_2048::campaign::craft`'s own doc comment for what this
    /// is: the class a team's own prototype craft "counts as" wherever a
    /// category restriction is checked against it, rather than a fifth,
    /// independent axis.
    pub prototype_livery: String,
}

impl ShipModel {
    /// Builds a [`ShipModel`] from an [`Instance`] of
    /// [`typedef::SHIP_MODEL_DATA`]. `None` for any other typedef.
    #[must_use]
    pub fn from_instance(instance: &Instance) -> Option<Self> {
        if instance.typedef_id != typedef::SHIP_MODEL_DATA {
            return None;
        }
        Some(Self {
            instance_id: instance.instance_id,
            name: instance.name.clone(),
            team: instance
                .field("M_TEAM")
                .and_then(super::Field::value)
                .unwrap_or_default()
                .to_string(),
            livery: instance
                .field("M_LIVERY")
                .and_then(super::Field::value)
                .unwrap_or_default()
                .to_string(),
            prototype_livery: instance
                .field("M_PROTOTYPELIVERY")
                .and_then(super::Field::value)
                .unwrap_or_default()
                .to_string(),
        })
    }
}

/// Every [`ShipModel`] in a document, in document order.
#[must_use]
pub fn ship_models(document: &Document) -> Vec<ShipModel> {
    document
        .instances
        .iter()
        .filter_map(ShipModel::from_instance)
        .collect()
}

/// A [`ShipModel`] reference (an event's own `M_PPLAYERSHIPMODELDATA`, or one
/// slot of `M_PGRIDSHIPMODELDATA`) resolved against its own document.
#[must_use]
pub fn ship_model_for(document: &Document, reference: Reference) -> Option<ShipModel> {
    ShipModel::from_instance(document.instance(reference.instance_id)?)
}

#[cfg(test)]
mod tests;
