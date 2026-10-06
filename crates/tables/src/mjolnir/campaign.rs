//! Typed view of [`super::Document`] for Wipeout 2048's campaign schema, the
//! shape `data/xml/SP.xml` and `data/xml/MP.xml` both author. The parent module
//! holds the typedef census and how each name was read.
//!
//! Entry names and what 2048 ships live in `oag_2048::campaign`, per
//! [ADR-0022]: this module knows the file's shape, not which title ships it.
//!
//! [ADR-0022]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0022-title-packages.md

use super::{Document, Instance, Reference};

/// Typedef ids this module resolves: named where `SP.xml`'s own
/// `type=`/`typedefid=` pairs name them, bare otherwise. Confidence per
/// [`super`]'s doc comment.
pub mod typedef {
    /// `GameModeObjective`, confidence 95 (named in-file). 96 instances: the
    /// pass/elite objectives an event's `M_PASSOBJECTIVE`/`M_ELITEOBJECTIVE`
    /// point at; see [`Objective`]. **What each `ObjectiveValue` ordinal (`1`,
    /// `2`, `4`, `7`, one empty) means is `oag_2048::campaign`'s reading**; see
    /// `docs/formats/2048-campaign.md`'s "The objective law".
    pub const GAME_MODE_OBJECTIVE: i64 = 380278911;
    /// `GameModeBase`, confidence 95 (named in-file, the abstract type of every
    /// polymorphic reference field). No `SP.xml` instance carries it as its own
    /// `typedefid`.
    pub const GAME_MODE_BASE: i64 = 366306753;
    /// `TrackDefinition`, confidence 95 (named in-file; its names match
    /// `data/plugins/tracks/Definition.xml`'s ten `<PI_Track name="...">` stems).
    /// 10 instances; see [`Track`]. An earlier pass guessed
    /// [`WEAPON_SET_DEFINITION`] was this ("20 track profiles"); wrong on the
    /// count alone.
    pub const TRACK_DEFINITION: i64 = 205052969;
    /// `WeaponSetDefinition`, confidence 95 (named in-file). 20 instances, each
    /// one `M_WEAPONAVAILABLEBITS` field. **All eleven `WeaponType` bits are
    /// pinned (2026-09-28)** from the enum declaration in `eboot.elf`; see
    /// [`WeaponSet::allowed_weapons`],
    /// `docs/ghidra/functions/vita-2048-eu-v104/weapon-type-bits.md` and
    /// `docs/formats/2048-campaign.md`'s "The weapon set gate".
    pub const WEAPON_SET_DEFINITION: i64 = -966434245;
    /// `WOShipModelData`, confidence 95 (named in-file). 21 instances: a
    /// team+livery craft catalogue (`M_TEAM`/`M_LIVERY`, e.g.
    /// `"Feisar2048"`/`"speed"`) with a rank-unlock ladder (`M_RANKUNLOCK`) or
    /// campaign-unlock edge (`M_PCAMPAIGNUNLOCK`). Forcing a craft is not rare:
    /// `M_PPLAYERSHIPMODELDATA` is authored on 14 of `SP.xml`'s 141 events and
    /// `M_PGRIDSHIPMODELDATA` on most numbered ones. See [`ShipModel`] and
    /// `oag_2048::campaign`.
    pub const SHIP_MODEL_DATA: i64 = 520725191;
    /// `GameMode_SpeedLapRace`'s typedef, **confidence 92 (2026-09-28)**:
    /// `oag_formats::wad::hash_name` of `"GameMode_ArcadeRace"` is
    /// `-1353052320` ([`RACE_B`]) and of `"GameMode_SpeedLapRace"` `-1915183557`
    /// (this), the case-folded CRC-32 every other named typedef here matches
    /// (ten names checked, no exceptions); see `docs/formats/2048-campaign.md`'s
    /// typedef table. 53 instances, the only typedef whose instances carry a
    /// non-empty `M_MAXGHOSTSHIPS` (all 53), and the only one with `laps == 0`:
    /// the 40 instances named `"<Track> Speed Lap - <Class>"` have `M_NUMOFLAPS`
    /// absent or zero; the other 13 are ordinary 2-3 lap `"20XX - Event N"`
    /// nodes. See [`EventKind::Race`] and [`Event::laps`].
    pub const RACE_A: i64 = -1915183557;
    /// `GameMode_ArcadeRace`, the other lap-race typedef (see [`RACE_A`]).
    /// 52 instances, laps always >= 1, no `M_MAXGHOSTSHIPS`; includes
    /// `E3_Demo_*` builds, `MP_*_Race_flash` templates and ten-lap
    /// `"* Ship Challenge"` events.
    ///
    /// **This project does not split `RACE_A`/`RACE_B` by mode.** `M_PNEXTEVENT`
    /// chains cross between them (`"2048 - Event 3"` is `RACE_A`, its next
    /// `"2048 - Event 4"` is `RACE_B`), which shows the unlock graph crosses
    /// concrete classes, not that they are one mode.
    /// `oag_2048::campaign::engine_mode` derives `"speed_lap"` vs
    /// `"single_race"` off the `laps == Some(0)` sentinel; [`Event::typedef_id`]
    /// keeps the raw id.
    ///
    /// `eboot.elf`'s other two `GameMode_*` names, `GameMode_CheckPointRace`
    /// (hash `375161732`) and `GameMode_ZombieRace` (`-1251488984`), match no
    /// typedef `SP.xml` carries.
    pub const RACE_B: i64 = -1353052320;
    /// Elimination-shaped. 26 instances, the only typedef with
    /// `M_ELIMINATENUMOFOPPONENTS`/`M_SCORETARGET`/`M_SURVIVEFORNUMOFLAPS`/
    /// `M_TIMELIMIT`/`M_BSEEKANDDESTROYTARGETID`/`M_BSOLOSCORING`, named
    /// `"Elim"`/`"Eliminator"`. **Confidence 92 (2026-09-28)**:
    /// `hash_name("GameMode_EliminatorRace")` equals this id, as for
    /// [`RACE_A`].
    pub const ELIMINATION: i64 = 1311982788;
    /// Zone-shaped. 10 instances, the only typedef with `M_ZONETIMECOUNTER`/
    /// `M_ZONETOADDMINES`/`M_STARTZONENUMBER`/`M_ENDZONENUMBER`/
    /// `M_NUMBEROFMINES`, and the only one whose `M_SPEEDCLASS` is always empty.
    /// **Confidence 92 (2026-09-28)**: `hash_name("GameMode_ZoneRace")` equals
    /// this id.
    pub const ZONE: i64 = 1018671239;
}

/// Which of the four concrete event shapes an [`Event`] is.
///
/// **Coarser than the four `GameMode_*` classes the typedefs bind to**
/// (confidence 92, see [`typedef::RACE_A`]): [`Self::Race`] merges
/// `GameMode_SpeedLapRace` and `GameMode_ArcadeRace`, since
/// `oag_2048::campaign::engine_mode` tells them apart off [`Event::laps`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventKind {
    /// [`typedef::RACE_A`] or [`typedef::RACE_B`], including the `laps == 0`
    /// Speed Lap sentinel ([`Event::laps`]).
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

/// One campaign event: a `RACE_A`/`RACE_B`/`ELIMINATION`/`ZONE` instance with
/// the fields needed to launch it or walk the unlock graph.
///
/// Not every schema field: AI tuning (`M_AIABSORBMULTIPLIER` and siblings), UI
/// placement (`M_BUTTONSHAPE`/`M_CANVASTWEAK_X`/`_Y`) and the grid/player ship
/// references stay on [`Instance::field`].
#[derive(Debug, Clone, PartialEq)]
pub struct Event {
    /// This event's own `instanceid`.
    pub instance_id: i64,
    /// The raw `typedefid`; [`Event::kind`] collapses the two race typedefs, this
    /// keeps them apart.
    pub typedef_id: i64,
    /// [`EventKind`] for `typedef_id`.
    pub kind: EventKind,
    /// The instance's `name=`, e.g. `"2048 - Event 3"`, `"MPElimination"`.
    pub name: String,
    /// `M_DESCRIPTION`: a language-table key like `"2048_EVENT_3"`, not text.
    pub description: Option<String>,
    /// `M_TRACKDEF`, a reference into [`typedef::TRACK_DEFINITION`]; resolve with
    /// [`track_for`]. `None` on every `ZONE` event: the Zone circuit is a title
    /// fact (`oag_2048::campaign`).
    pub track: Option<Reference>,
    /// `M_SPEEDCLASS`'s raw `eClass` ordinal, `0`-`4`; named in
    /// `oag_2048::campaign::EClass`. `None` on every `ZONE` event.
    pub speed_class: Option<i64>,
    /// `M_NUMOFLAPS`. **`Some(0)` is Speed Lap's sentinel**: all 40 `SP.xml`
    /// instances with `laps == Some(0)` are `"<Track> Speed Lap - <Class>"` and
    /// none of the other 101 lap races author it. `None` on `ELIMINATION`/`ZONE`,
    /// which use their own completion fields (`M_SURVIVEFORNUMOFLAPS`/
    /// `M_TIMELIMIT`/`M_ZONETIMECOUNTER`).
    pub laps: Option<u32>,
    /// `M_WEAPONSET`, a reference into [`typedef::WEAPON_SET_DEFINITION`];
    /// resolve with [`weapon_set_for`].
    pub weapon_set: Option<Reference>,
    /// `M_PNEXTEVENT`: what comes after this event.
    pub next_event: Option<Reference>,
    /// `M_PBRANCHEVENT`: a sibling event off the same node (`"2048 - Event 3"`
    /// branches to `"2048 - Event 3-1"`).
    pub branch_event: Option<Reference>,
    /// `M_PEVENTREQUIRED`: a prerequisite outside the `next`/`branch` chain.
    pub required_event: Option<Reference>,
    /// `M_PASSOBJECTIVE`, a reference into [`typedef::GAME_MODE_OBJECTIVE`].
    pub pass_objective: Option<Reference>,
    /// `M_ELITEOBJECTIVE`, as [`Event::pass_objective`] for the harder target.
    pub elite_objective: Option<Reference>,
    /// `M_X`/`M_Y`: the event's map-grid position. **Not the campaign map's
    /// pixel projection**: `docs/ghidra/functions/vita-2048-eu-v104/frontend-campaign-map.md`'s
    /// scale/bias/offset tables belong to the DLC tiers' `FE3DCanvas` hotspots,
    /// a different node set. Nothing in `SP.xml` authors how `(x, y)` becomes a
    /// screen position, and this module does not guess one.
    pub x: Option<i32>,
    /// See [`Event::x`].
    pub y: Option<i32>,
    /// `M_RankRequired`: the player rank an event needs, when authored.
    pub rank_required: Option<i32>,
    /// `M_bForceAlwaysUnlocked`.
    pub force_always_unlocked: Option<bool>,
    /// Whether `M_MAXGHOSTSHIPS` is present at all ([`typedef::RACE_A`]'s
    /// discriminator). Not its value: every measured instance leaves it empty.
    pub has_ghost_capacity: bool,
}

impl Event {
    /// Builds an [`Event`] from one of the four event typedefs; `None` otherwise.
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

/// Every event in a [`Document`], in **document order, not campaign order**:
/// the schema authors no play sequence and names no root event. Walk
/// [`Event::next_event`]/[`Event::branch_event`]/[`Event::required_event`] from
/// a known first event.
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
    /// This instance's `instanceid`, what an [`Event::track`] names.
    pub instance_id: i64,
    /// The instance's own `name=`, e.g. `"Bridge"`.
    pub name: String,
    /// `M_TRACKNAME`, e.g. `"bridge"`: the lowercase stem
    /// `data/plugins/tracks/Definition.xml`'s `<PI_Track name="...">` and
    /// `oag_title::Title::track_plugin_definition` key on; see
    /// `oag_2048::campaign`.
    pub track_name: String,
    /// `M_DISPLAYNAME`, e.g. `"CAPITAL REACH"`: the in-game circuit name.
    pub display_name: String,
}

impl Track {
    /// Builds a [`Track`] from a [`typedef::TRACK_DEFINITION`] instance; `None`
    /// for any other typedef or an absent field.
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

/// [`Event::track`] resolved against its document; `None` if absent or dangling.
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
    /// `M_WEAPONAVAILABLEBITS`'s raw value; see [`Self::allowed_weapons`] and
    /// `docs/formats/2048-campaign.md`'s "The weapon set gate".
    pub available_bits: Option<i64>,
}

impl WeaponSet {
    /// Builds a [`WeaponSet`] from a [`typedef::WEAPON_SET_DEFINITION`] instance.
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

/// `(bit index, weapon)` for all eleven `M_WEAPONAVAILABLEBITS` bits, from
/// `WeaponType`'s enum declaration in `eboot.elf`
/// (`docs/ghidra/functions/vita-2048-eu-v104/weapon-type-bits.md`; history in
/// `docs/formats/2048-campaign.md`'s "The weapon set gate").
///
/// **Confidence 95 for eight** (`Rocket`, `Missile`, `Quake`, `Turbo`, `Cannon`,
/// `Autopilot`, `Plasma`, `LeachBeam`): each is the sole bit on a
/// `"<Weapon> Only"` `SP.xml` instance (`"Rockets Only"` = `1` ... `"Leech Beam
/// Only"` = `1024`), every multi-weapon value is the sum of its bits with no
/// residue across all 20 instances, and the enum ordinals match.
///
/// **Confidence 95 for `Shield`** (bit 4): the data only suggested it; the enum
/// declaration names ordinal 4 `SHIELD`.
///
/// **Confidence 90 for `Bomb` (bit 8) and `Mine` (bit 9)**: read off the enum
/// (`BOMB` = 8, `MINE` = 9), one tier lower because no `SP.xml` instance sets
/// one without the other. An earlier joint `Mine`+`Bomb` gate at confidence 75
/// is superseded. This order does **not** match the `Hud_UpdatePickupIcon`
/// held-weapon id table (`pickup-icon-uv-table.md`: id 8 `FE_MINES`, 9
/// `FE_BOMB`), a different enum that disagrees from id 5 on (`WeaponType` bit 5
/// is `Cannon`, held-id 5 is `Shield`).
pub const WEAPON_BITS: &[(u32, crate::weapons::Weapon)] = &[
    (0, crate::weapons::Weapon::Rocket),
    (1, crate::weapons::Weapon::Missile),
    (2, crate::weapons::Weapon::Quake),
    (3, crate::weapons::Weapon::Turbo),
    (4, crate::weapons::Weapon::Shield),
    (5, crate::weapons::Weapon::Cannon),
    (6, crate::weapons::Weapon::Autopilot),
    (7, crate::weapons::Weapon::Plasma),
    (8, crate::weapons::Weapon::Bomb),
    (9, crate::weapons::Weapon::Mine),
    (10, crate::weapons::Weapon::LeachBeam),
];

impl WeaponSet {
    /// The weapons `M_WEAPONAVAILABLEBITS` allows, via [`WEAPON_BITS`]. Empty
    /// when [`Self::available_bits`] is `None` or sets none of them.
    #[must_use]
    pub fn allowed_weapons(&self) -> Vec<crate::weapons::Weapon> {
        let Some(bits) = self.available_bits else {
            return Vec::new();
        };
        WEAPON_BITS
            .iter()
            .filter(|&&(bit, _)| bits & (1_i64 << bit) != 0)
            .map(|&(_, weapon)| weapon)
            .collect()
    }
}

/// One `GameModeObjective`: what [`Event::pass_objective`]/
/// [`Event::elite_objective`] point at.
///
/// **Names only**: the meaning of the `M_OBJECTIVETYPE` ordinals (`1`/`2`/`4`/
/// `7`) is `oag_2048::campaign`'s, read off the measured `(pass, elite)` type
/// pairs per [`EventKind`]; see `docs/formats/2048-campaign.md`'s "The objective
/// law".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Objective {
    /// This instance's own `instanceid`.
    pub instance_id: i64,
    /// The instance's `name=`, e.g. `"FinishRaceAnyPosition"`, `"Top5"`.
    pub name: String,
    /// `M_OBJECTIVETYPE`'s raw `ObjectiveValue` ordinal; `None` on the one
    /// instance (`"MostDamage"`) that authors none.
    pub objective_type: Option<i64>,
    /// `M_OBJECTIVETARGET`; `None` for a type with no target (`FINISH`) or an
    /// absent field.
    pub target: Option<i64>,
}

impl Objective {
    /// Builds an [`Objective`] from a [`typedef::GAME_MODE_OBJECTIVE`] instance.
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

/// [`Event::pass_objective`]/[`Event::elite_objective`] resolved against their
/// document.
#[must_use]
pub fn objective_for(document: &Document, reference: Reference) -> Option<Objective> {
    Objective::from_instance(document.instance(reference.instance_id)?)
}

/// One `WOShipModelData`: a team+livery craft the roster offers, or what an
/// event's `M_PPLAYERSHIPMODELDATA`/`M_PGRIDSHIPMODELDATA` points at; see
/// `oag_2048::campaign` for what forcing one means and how `(team, livery)`
/// resolves onto `race::Options::team`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShipModel {
    /// This instance's own `instanceid`.
    pub instance_id: i64,
    /// The instance's `name=`, e.g. `"Feisar_Speed"`. `"WINGMAN"` is the one
    /// instance with an empty [`Self::team`]/[`Self::livery`].
    pub name: String,
    /// `M_TEAM`, e.g. `"Feisar2048"`, matching `oag_2048::race::NATIVE_TEAMS`.
    pub team: String,
    /// `M_LIVERY`, e.g. `"speed"`, `"combat"`, `"agility"`, `"prototype"`.
    pub livery: String,
    /// `M_PROTOTYPELIVERY`, authored only on the five `livery == "prototype"`
    /// instances (`"Agility"`/`"Combat"`/`"Speed"`, title-cased unlike
    /// [`Self::livery`]). The class a team's prototype "counts as" for a category
    /// restriction; see `oag_2048::campaign::craft`.
    pub prototype_livery: String,
}

impl ShipModel {
    /// Builds a [`ShipModel`] from a [`typedef::SHIP_MODEL_DATA`] instance.
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

/// A [`ShipModel`] reference (`M_PPLAYERSHIPMODELDATA`, or a slot of
/// `M_PGRIDSHIPMODELDATA`) resolved against its document.
#[must_use]
pub fn ship_model_for(document: &Document, reference: Reference) -> Option<ShipModel> {
    ShipModel::from_instance(document.instance(reference.instance_id)?)
}

#[cfg(test)]
mod tests;
