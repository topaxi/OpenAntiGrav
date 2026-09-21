//! The built-in campaign: `PI_Grid`/`PI_Cell`, `Data\Plugins\grids\grid_00.xml`
//! .. `grid_15.xml` inside `Data.wad`.
//!
//! Wipeout Pulse's campaign is **authored data, not code**. Sixteen files, each
//! one `PI_Grid` holding 8-16 `PI_Cell` records; each cell names its track,
//! mode, speed class, lap count, weapons/damage switches, AI count, AI skill,
//! and its own gold/silver/bronze targets. There is no compiled table and no
//! procedural generator - the whole campaign is these 236 records. See
//! `docs/ghidra/functions/psp-pulse-usa/race-campaign.md` for the decompiled
//! law this schema was read against (`PI_Cell_ParseElement`, `0x088bf83c`, and
//! `PI_Grid_ParseElement`, `0x088c0240`).
//!
//! ```text
//! <PI_Grid name="grid0">
//!   <Values RequiredPoints="12" Locked="false"/>
//!   <Unlock Grid="Grid0"/>                          <!-- grid1 onwards -->
//!   <PI_Cell name="grid0_2_1">
//!     <Values track="16_Track" mode="Race" class="Venom" Weapons="on"
//!             damage="on" AICount="7" skillEasy="1.1" skill="1.75"
//!             skillHard="2.5" laps="3" ship="None" ShipChoice="Yes"/>
//!     <Gold Target="1"/> <Silver Target="2"/> <Bronze Target="3"/>
//!   </PI_Cell>
//!   ...
//! </PI_Grid>
//! ```
//!
//! # This crate knows the medal law, not who won
//!
//! [`Cell::evaluate_medal`] reimplements `Cell_EvaluateMedal` - the
//! three-way threshold compare against a cell's own targets, direction
//! flipped for `Zone`/`Elimination` - and [`Medal::points`] reimplements
//! `Cell_MedalPoints`. Both are properties of the law the disc's own code
//! applies to this data, not of the file, so they live beside the type they
//! evaluate. What they do **not** do is decide *which* cell a race was run
//! against, or *what value* a finished race actually scored - that mapping
//! (a race's result onto a campaign cell's mode-specific value) and the
//! unlock-points comparison across a whole grid are still a caller's job;
//! see `crates/game/src/records.rs` for where a race's own outcome is
//! captured and `docs/architecture/persistence.md` for what is and is not
//! wired yet. [`oag_pulse::campaign`](../../oag_pulse/campaign/index.html)
//! carries the sixteen entry names, per [ADR-0022] - a file's *shape* lives
//! here, what a title *ships* lives there.
//!
//! # `class="Zone"` is real, authored text, and it is not a speed class
//!
//! Every `PI_Cell`'s `class` attribute is required, but a `Zone` cell's own
//! value is the literal string `Zone` - not one of `Venom`/`Flash`/`Rapier`/
//! `Phantom`. Confirmed directly against `grid_00.xml` through `grid_15.xml`
//! on `pulse-psp-usa.chd`, and not previously recorded: `race-campaign.md`'s
//! own `g_class_name_table` reading (`0x08ab067c`) lists only the four speed
//! classes, so either that table has a fifth entry unread this pass or the
//! parser leaves an unrecognised name in the field rather than rejecting it -
//! either way `CellSelection_PopulateDetail` is documented to blank the class
//! label for `Zone`, which is consistent with the value being present but not
//! meaningfully a speed class. [`Cell::class`] is therefore the **raw**
//! string, never a forced [`crate::handling::SpeedClass`] - see
//! [`Cell::speed_class`] for the fallible mapping. Per the "a parser cannot
//! fail on a field it does not understand" lesson, an unrecognised `class`
//! is not an error.
//!
//! [ADR-0022]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0022-title-packages.md

use std::fmt;

use crate::fexml::{self, Node};
use crate::handling::SpeedClass;

/// The nine modes `g_mode_name_table` (`0x08ab062c`) names, in ordinal order,
/// plus a raw catch-all for a spelling that table has no entry for.
///
/// **`7` is absent from the table** - a retired or debug mode, per
/// `race-campaign.md` - so this type has no variant for it and
/// [`Mode::from_name`] returns `None` for anything not in the other nine.
/// [`Mode::Other`] is a *different* thing: not this crate refusing to guess
/// at ordinal `7`, but Wipeout HD authoring two spellings
/// (`"NitroBattle"`, `"Detonator"`) this table was never extended to cover,
/// on an executable this pass does not decompile - see
/// `docs/formats/race-campaign.md`'s HD section.
///
/// Carrying a `String` rather than adding a properly-ordinalled variant per
/// the "keep it raw" rule this crate already applies to [`Cell::class`]'s
/// `"Zone"` is why this type is no longer [`Copy`] - every existing match on
/// a [`Mode`] value that does not destructure [`Mode::Other`] is unaffected.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Mode {
    /// Ordinal `3`. A full grid, weapons optional.
    Race,
    /// Ordinal `4`. A series of races, `<TournamentTrack>` legs.
    Tournament,
    /// Ordinal `5`. `"Time Trial"` in the XML.
    TimeTrial,
    /// Ordinal `6`. No `class` that means anything and `laps="0"`.
    Zone,
    /// Ordinal `8`. A kill-count target rather than a lap count; `laps` is
    /// absent from the cell entirely.
    Elimination,
    /// Ordinal `9`. `"Head2Head"` in the XML, no space.
    Head2Head,
    /// Ordinal `10`. `"Speed Lap"` in the XML.
    SpeedLap,
    /// Ordinal `11`. What a player-built grid's own cells run as; not
    /// authored by any shipped `grid_NN.xml`.
    CustomGrid,
    /// Ordinal `12`. Not authored by any shipped `grid_NN.xml` either.
    AiRace,
    /// A `mode=` spelling absent from `g_mode_name_table` on the one
    /// executable this crate reads the table off - Wipeout HD's own
    /// `"NitroBattle"` and `"Detonator"`, both only on the Fury-tagged grids
    /// (`Campaign="Fury"`). Carries the exact text authored, never guessed
    /// at a Pulse ordinal it was not measured against.
    Other(String),
}

impl Mode {
    /// Every variant, in the executable's own ordinal order.
    pub const ALL: [Self; 9] = [
        Self::Race,
        Self::Tournament,
        Self::TimeTrial,
        Self::Zone,
        Self::Elimination,
        Self::Head2Head,
        Self::SpeedLap,
        Self::CustomGrid,
        Self::AiRace,
    ];

    /// The integer `g_mode_name_table` associates with this mode, and the same
    /// one `Race_RecordResult` switches on (`mode - 3`). `None` for
    /// [`Mode::Other`], which this table has no entry for at all.
    #[must_use]
    pub fn ordinal(&self) -> Option<u32> {
        Some(match self {
            Self::Race => 3,
            Self::Tournament => 4,
            Self::TimeTrial => 5,
            Self::Zone => 6,
            Self::Elimination => 8,
            Self::Head2Head => 9,
            Self::SpeedLap => 10,
            Self::CustomGrid => 11,
            Self::AiRace => 12,
            Self::Other(_) => return None,
        })
    }

    /// The exact spelling a `grid_NN.xml` file's `mode=` attribute carries.
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            Self::Race => "Race",
            Self::Tournament => "Tournament",
            Self::TimeTrial => "Time Trial",
            Self::Zone => "Zone",
            Self::Elimination => "Elimination",
            Self::Head2Head => "Head2Head",
            Self::SpeedLap => "Speed Lap",
            Self::CustomGrid => "Custom Grid",
            Self::AiRace => "AI Race",
            Self::Other(name) => name,
        }
    }

    /// Parses a `mode=` value case-insensitively against the nine named
    /// ordinals only. `None` for anything else - a literal `"7"`-shaped mode
    /// the table itself has no name for, **and** a spelling like HD's
    /// `"NitroBattle"` this crate's own parser wraps in [`Mode::Other`]
    /// rather than passing through here; this function never produces that
    /// variant itself.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|m| m.as_str().eq_ignore_ascii_case(name))
    }
}

impl fmt::Display for Mode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Something wrong with a `PI_Grid` document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The blob could not be expanded out of its shortened form.
    Expand(fexml::Error),
    /// No `<PI_Grid>` root element.
    MissingGrid,
    /// A required element is absent on the element named.
    MissingElement {
        /// The element it should have been a child of.
        parent: &'static str,
        /// The element that was looked for.
        element: &'static str,
    },
    /// A required attribute is absent. Never defaulted - see the module docs.
    MissingAttribute {
        /// The element it should have been on.
        element: &'static str,
        /// The attribute that was looked for.
        attribute: &'static str,
    },
    /// An attribute is present but does not parse as the type it should.
    NotANumber {
        /// The element it was on.
        element: &'static str,
        /// The attribute it was on.
        attribute: &'static str,
        /// What was found, so the error names the offending text.
        value: String,
    },
    /// **No longer produced by this crate's own parser.** A `mode=` value
    /// outside [`Mode`]'s nine named ordinals is [`Mode::Other`] now rather
    /// than a parse failure, per the module docs' "a parser cannot fail on a
    /// field it does not understand" rule - kept in this enum for API
    /// stability rather than removed.
    UnknownMode {
        /// What was found.
        name: String,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Expand(e) => write!(f, "expanding shortened XML: {e}"),
            Self::MissingGrid => f.write_str("no <PI_Grid> root element"),
            Self::MissingElement { parent, element } => {
                write!(f, "<{parent}> has no <{element}> child")
            }
            Self::MissingAttribute { element, attribute } => {
                write!(f, "<{element}> has no {attribute} attribute")
            }
            Self::NotANumber {
                element,
                attribute,
                value,
            } => write!(f, "<{element}> {attribute}=\"{value}\" does not parse"),
            Self::UnknownMode { name } => write!(f, "unknown mode \"{name}\""),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Expand(e) => Some(e),
            _ => None,
        }
    }
}

impl From<fexml::Error> for Error {
    fn from(value: fexml::Error) -> Self {
        Self::Expand(value)
    }
}

/// Result alias for this module.
pub type Result<T> = std::result::Result<T, Error>;

/// One `PI_Grid`: a tier of the campaign, 8-16 cells.
#[derive(Debug, Clone, PartialEq)]
pub struct Grid {
    /// The grid's own name, e.g. `"grid0"`. What an `<Unlock Grid="Grid0"/>`
    /// row elsewhere on the disc matches case-insensitively.
    pub name: String,
    /// Points needed, summed across this grid's own cells, to satisfy an
    /// `<Unlock Grid="..."/>` naming it. `0` on `grid15`, which renders as
    /// `FE_NA` rather than a number.
    pub required_points: u32,
    /// The `Locked` byte. **No consumer of it was traced** - see
    /// `race-campaign.md#what-is-not-determined`. Recorded, not interpreted.
    pub locked: bool,
    /// `Group="1"` marks `grid12`..`grid15` on the shipped disc; absent
    /// elsewhere. Consumer not traced.
    pub group: Option<u32>,
    /// The `<Unlock Grid="..."/>` child, when present. `grid1` onwards name
    /// the grid before it; `grid0` names none.
    pub unlock_grid: Option<String>,
    /// This grid's cells, in document order.
    pub cells: Vec<Cell>,
    /// `Campaign="HD"|"Fury"`, raw. Absent from every Pulse grid and from
    /// some of Wipeout HD's own copies of the same grid (`DATA02.PSARC`'s
    /// `grid_00.xml`..`grid_07.xml` carries neither this nor the
    /// per-difficulty cell schema; `DATA04.PSARC`'s copy of the same eight
    /// carries the schema but not this attribute; `DATA06.PSARC`'s carries
    /// both) - see `docs/formats/race-campaign.md`'s HD section for the full
    /// archive table this measures.
    pub campaign: Option<String>,
    /// `TitleColor="0xFF525252"`, raw hex text exactly as authored. Not
    /// parsed to a colour: this title's channel order (ARGB vs RGBA) has not
    /// been measured, and guessing it would be exactly the kind of invented
    /// reading this crate's own parsing rule forbids.
    pub title_color: Option<String>,
    /// `TextColor=`, the same raw treatment as [`Grid::title_color`].
    pub text_color: Option<String>,
    /// `FlyerName="01_uplift"`: the cell-selection screen's own flyer image
    /// for this grid, by its stem - HD-only, absent from every Pulse grid.
    pub flyer_name: Option<String>,
    /// `BillboardName="Data/Billboards/HD_Adverts/.../....vex"`: this grid's
    /// own advert billboard, HD-only.
    pub billboard_name: Option<String>,
}

impl Grid {
    /// The maximum a grid can earn: `Cell_MedalPoints(cell, 0)` is always `3`
    /// for a gold medal, so this is `3 * cells.len()`. Not something a shipped
    /// file authors directly - `Grid_PointsPossible` (`0x088c0570`) computes
    /// it the same way at runtime, which the ground-truth test checks this
    /// reproduces (24/30/36/42/48 for the sixteen shipped grids).
    #[must_use]
    pub fn max_points(&self) -> u32 {
        // Every cell is worth 3 at best; see `Cell_MedalPoints`.
        u32::try_from(self.cells.len())
            .unwrap_or(u32::MAX)
            .saturating_mul(3)
    }
}

/// One `PI_Cell`: a single event on the grid.
#[derive(Debug, Clone, PartialEq)]
pub struct Cell {
    /// The cell's own name, e.g. `"grid0_2_1"`. The trailing `_<x>_<y>` is the
    /// cell's position on the hex grid the front end draws - see
    /// [`Cell::grid_coords`].
    pub name: String,
    /// The circuit, e.g. `"16_Track"`. **Absent for `Tournament`**, which
    /// names its legs through [`Cell::tournament_tracks`] instead.
    pub track: Option<String>,
    /// The event type.
    pub mode: Mode,
    /// The raw `class=` text. Usually one of `Venom`/`Flash`/`Rapier`/
    /// `Phantom`; a `Zone` cell's own value is the literal `"Zone"`, which is
    /// not one of the four - see the module docs and [`Cell::speed_class`].
    pub class: String,
    /// `Weapons="on"|"off"`.
    pub weapons: bool,
    /// `damage="on"|"off"`.
    pub damage: bool,
    /// The `Locked` byte, when authored. Present on a handful of cells only;
    /// no consumer was traced (see [`Grid::locked`]'s doc comment).
    pub locked: Option<bool>,
    /// The `Status` byte. No shipped grid file sets it.
    pub status: Option<bool>,
    /// Opponent count. `7` on `Race`/`Tournament`/`Elimination`, `1` on
    /// `Head2Head`, absent on the solo modes (`Time Trial`, `Speed Lap`,
    /// `Zone`).
    pub ai_count: Option<u32>,
    /// A position on the track's own `SkillScaleValue` curve at the medium
    /// difficulty. Absent on the solo modes, alongside `ai_count`.
    ///
    /// **`skillMedium` on Wipeout HD**, read as an alias of Pulse's `skill` -
    /// the same value under a name that says which difficulty it is rather
    /// than leaving it implicit. Never both on the same cell in what this
    /// project has measured, so there is no precedence to pick between them;
    /// `skill` is tried first.
    pub skill: Option<f32>,
    /// The easy-difficulty position. Defaults to `skill - 1.0` in the
    /// original's own parser when the attribute is absent but `skill` is
    /// present - see [`Cell::skill_for_difficulty`], which applies that
    /// documented default rather than this field silently doing so.
    pub skill_easy: Option<f32>,
    /// The hard-difficulty position. Defaults to `skill + 1.0`; see
    /// [`Cell::skill_easy`].
    pub skill_hard: Option<f32>,
    /// Lap count. **Absent entirely for `Elimination`** (the kill target
    /// stands in for it); `"0"` for `Zone`, rendered as an infinity glyph;
    /// `7` for every `Speed Lap` cell; otherwise 3 Venom / 4 Flash / 4 Rapier
    /// / 5 Phantom with no exception across all 236 shipped cells.
    pub laps: Option<u32>,
    /// `ship=`. Every shipped cell carries `"None"` - the campaign never
    /// forces a craft.
    pub ship: Option<String>,
    /// `ShipChoice="Yes"|"No"`.
    pub ship_choice: Option<bool>,
    /// `<Gold Target="..."/>`. A finishing position for `Race`/`Tournament`/
    /// `Head2Head`, a time in centiseconds for `Time Trial`/`Speed Lap`, a
    /// zone count for `Zone`, a kill count for `Elimination`.
    ///
    /// **On a cell with [`Cell::difficulty_targets`], this is that rung's own
    /// `medium` value** - the medal law this crate reimplements
    /// ([`Cell::evaluate_medal`]) takes one target triple, and every existing
    /// caller of it wants the difficulty-agnostic value Pulse always had, so
    /// this field keeps meaning that on Wipeout HD too rather than becoming
    /// `None` the moment a cell authors three triples instead of one.
    pub gold: i64,
    /// `<Silver Target="..."/>`, or [`Cell::difficulty_targets`]'s `medium`
    /// rung - see [`Cell::gold`].
    pub silver: i64,
    /// `<Bronze Target="..."/>`, or [`Cell::difficulty_targets`]'s `medium`
    /// rung - see [`Cell::gold`].
    pub bronze: i64,
    /// `<TournamentTrack track="..."/>` rows, in document order. Empty for
    /// every mode but `Tournament`, which carries no [`Cell::track`] and
    /// names its legs here instead.
    pub tournament_tracks: Vec<String>,
    /// Three medal-target triples, one per difficulty -
    /// `<EasyGold>`/`<MediumGold>`/`<HardGold>` and the two element groups
    /// beside them. `None` on every Pulse cell and on the copy of
    /// `grid_00.xml`..`grid_07.xml` this project's own archive precedence
    /// reaches (`DATA02.PSARC`'s, plain `<Gold>`/`<Silver>`/`<Bronze>`);
    /// `Some` on `grid_08.xml`..`grid_15.xml` (`DATA00.PSARC`, Fury-only)
    /// and on the `DATA04.PSARC`/`DATA06.PSARC` copies of `grid_00`..`07`
    /// that precedence does not reach. See `docs/formats/race-campaign.md`'s
    /// HD section for the full archive table this measures.
    pub difficulty_targets: Option<DifficultyTargets>,
    /// `<NitroElimNovice>`/`<NitroElimSkilled>`/`<NitroElimElite>`: a fourth
    /// target triple, present on every cell of a grid that carries
    /// [`Cell::difficulty_targets`] - a plain `Race` or `Speed Lap` cell
    /// included, authored at a dummy `1` there - not only on `Elimination`
    /// or the HD-only `NitroBattle` mode whose name it carries.
    ///
    /// Named `(novice, skilled, elite)` in the ascending order the file
    /// itself authors the words - **chosen, not measured**: whether a caller
    /// should read `elite` as this triple's gold-equivalent target is not
    /// established against any executable. See
    /// `docs/formats/race-campaign.md`'s HD section.
    pub nitro_elimination_targets: Option<(i64, i64, i64)>,
}

/// One gold/silver/bronze target triple - [`Cell::gold`]/[`Cell::silver`]/
/// [`Cell::bronze`] at a single difficulty, and each rung of
/// [`DifficultyTargets`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MedalTargets {
    /// The gold-medal target.
    pub gold: i64,
    /// The silver-medal target.
    pub silver: i64,
    /// The bronze-medal target.
    pub bronze: i64,
}

/// [`Cell::difficulty_targets`]: one [`MedalTargets`] triple per difficulty.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DifficultyTargets {
    /// `<EasyGold>`/`<EasySilver>`/`<EasyBronze>`.
    pub easy: MedalTargets,
    /// `<MediumGold>`/`<MediumSilver>`/`<MediumBronze>`. Also
    /// [`Cell::gold`]/[`Cell::silver`]/[`Cell::bronze`]'s own value - see
    /// those fields' docs.
    pub medium: MedalTargets,
    /// `<HardGold>`/`<HardSilver>`/`<HardBronze>`.
    pub hard: MedalTargets,
}

impl Cell {
    /// [`Cell::class`], mapped onto [`crate::handling::SpeedClass`] when it is
    /// one of the four rungs the physics side knows. `None` for `Zone`'s own
    /// `"Zone"` value and for anything else unrecognised - never an error, per
    /// the module docs.
    #[must_use]
    pub fn speed_class(&self) -> Option<SpeedClass> {
        SpeedClass::from_name(&self.class)
    }

    /// The `(x, y)` position parsed out of the cell's own name, the way
    /// `PI_Cell_ParseElement` does it: everything after the **first**
    /// underscore in `name` is `x`, everything after the second is `y`. Only
    /// correct because a grid name contains no underscore of its own and both
    /// coordinates are a single digit - the same constraint the original's own
    /// `strchr`-based parser relies on. `None` if the name does not have that
    /// shape.
    #[must_use]
    pub fn grid_coords(&self) -> Option<(u32, u32)> {
        let mut parts = self.name.splitn(3, '_');
        let _grid = parts.next()?;
        let x = parts.next()?.parse().ok()?;
        let y = parts.next()?.parse().ok()?;
        Some((x, y))
    }

    /// The skill position for a difficulty, `0` easy through `2` hard,
    /// applying the original's own documented defaults
    /// (`skillEasy = skill - 1.0`, `skillHard = skill + 1.0`) when the
    /// specific attribute is absent. `None` when [`Cell::skill`] itself is
    /// absent - a solo-mode cell has no AI to scale.
    #[must_use]
    pub fn skill_for_difficulty(&self, difficulty: u8) -> Option<f32> {
        let skill = self.skill?;
        Some(match difficulty {
            0 => self.skill_easy.unwrap_or(skill - 1.0),
            1 => skill,
            _ => self.skill_hard.unwrap_or(skill + 1.0),
        })
    }

    /// The medal-target triple for a difficulty, `0` easy through `2` hard:
    /// [`Cell::difficulty_targets`]'s matching rung when the cell authors
    /// one, else [`Cell::gold`]/[`Cell::silver`]/[`Cell::bronze`]
    /// unconditionally - which is every Pulse cell, and every Wipeout HD
    /// cell this project's own archive precedence reaches for
    /// `grid_00.xml`..`grid_07.xml`. The mirror of [`Cell::skill_for_difficulty`]
    /// for the target side of a cell rather than the AI side.
    #[must_use]
    pub fn targets_for_difficulty(&self, difficulty: u8) -> MedalTargets {
        match &self.difficulty_targets {
            Some(dt) => match difficulty {
                0 => dt.easy,
                1 => dt.medium,
                _ => dt.hard,
            },
            None => MedalTargets {
                gold: self.gold,
                silver: self.silver,
                bronze: self.bronze,
            },
        }
    }

    /// The single per-rung target [`Cell::nitro_elimination_targets`]
    /// carries, `0` easy through `2` hard (`novice`/`skilled`/`elite`, the
    /// same rung words this title's own screens and executable use in place
    /// of Pulse's `Easy`/`Medium`/`Hard` - see
    /// `docs/ghidra/functions/ps3-hdfury-eu/race-campaign.md`'s medal-law
    /// section for the `HARD(ELITE)` string this equivalence is measured
    /// from). `None` when the cell carries no nitro triple at all.
    ///
    /// **This is one number, not a [`MedalTargets`] triple**, and deliberately
    /// has no sibling that forges one - synthesizing `gold`/`silver`/`bronze`
    /// out of a single measured value would be inventing data the disc does
    /// not author, exactly what this project's own reverse-engineering rule
    /// forbids. What the executable's medal law actually does with this one
    /// number - award a single pass/fail outcome, scale a threshold three
    /// ways at evaluate-time, or something else - is **not measured**; see
    /// that same evidence page's "what is not determined" section. A caller
    /// wiring an `Elimination`/`NitroBattle` cell's medal needs to answer
    /// that question first, not assume [`Cell::evaluate_medal`] already
    /// does - see its own doc comment.
    #[must_use]
    pub fn nitro_elimination_target_for_difficulty(&self, difficulty: u8) -> Option<i64> {
        let (novice, skilled, elite) = self.nitro_elimination_targets?;
        Some(match difficulty {
            0 => novice,
            1 => skilled,
            _ => elite,
        })
    }

    /// `Cell_EvaluateMedal` (`0x088bf620`): a three-way threshold compare of
    /// `value` against this cell's own gold/silver/bronze targets. `value`
    /// is a finishing position for `Race`/`Tournament`/`Head2Head`, a time
    /// in centiseconds for `Time Trial`/`Speed Lap`, a zone count for
    /// `Zone`, or a kill count for `Elimination` - never derived here; the
    /// caller supplies whatever that mode measures.
    ///
    /// **On a Wipeout HD/Fury cell that also carries
    /// [`Cell::nitro_elimination_targets`], this function's result is not
    /// meaningful for `Elimination`/`NitroBattle`.** Its own
    /// `gold`/`silver`/`bronze` fields (and every rung of
    /// [`Cell::difficulty_targets`]) are authored dummy `1`/`2`/`3` on such a
    /// cell - confirmed directly against `DATA00.PSARC`'s eight Fury grids,
    /// see `crates/hd/tests/campaign_grids_ground_truth.rs`'s
    /// `eliminationfamily_cells_carry_a_real_nitro_triple_and_a_dummy_flat_one`,
    /// so this always compares `value` against that placeholder rather than
    /// the cell's real target. What replaces it - reading
    /// [`Cell::nitro_elimination_target_for_difficulty`] instead, and by what
    /// rule that single number becomes a medal - is unmeasured; see that
    /// method's own doc comment. Left unchanged here rather than guessed at,
    /// since Pulse's own `Eliminator` cells (whose kill target lives in the
    /// flat `gold` field, per `docs/ghidra/functions/psp-pulse-usa/race-campaign.md`)
    /// still need this exact behaviour and are pinned by this crate's own
    /// ground-truth tests.
    ///
    /// **The comparison direction flips for `Zone` and `Elimination`.**
    /// Every other mode wants a *lower* value than the target (`<=`: a
    /// better finishing position or a faster time); these two counting
    /// modes want a *higher* value (`>=`: more zones crossed or more
    /// kills). Dropping the flip would score a Zone run of 16 zones against
    /// `20`/`17`/`15` targets a *gold* (`16 <= 20`), when 16 in fact clears
    /// only the bronze floor going the right direction (`16 >= 15`, but
    /// `16 < 17`) - see this module's tests for the three cases that would
    /// go wrong each way, both if the flip is dropped and if it is wrongly
    /// applied to a non-counting mode.
    ///
    /// `None` when no tier is met, or when `value` reads as "no result
    /// yet": the literal `0xFFFF_FFFF` the original stores as an unset
    /// `u32` register, or **any non-positive value**. The original checks
    /// only the exact value `0` against that `u32` register - this
    /// reimplementation's `<= 0` is a deliberately wider, **chosen, not
    /// measured**, guard: no mode here ever legitimately measures a
    /// negative position, time, zone count or kill count, so treating every
    /// non-positive `i64` as "unset" changes no real value the original
    /// could ever have produced, and closes off a negative value reaching
    /// the comparison loop by accident (a `<=` mode would otherwise score a
    /// negative "time" a gold).
    #[must_use]
    pub fn evaluate_medal(&self, value: i64) -> Option<Medal> {
        if value <= 0 || value == 0xFFFF_FFFF {
            return None;
        }
        let flipped = matches!(self.mode, Mode::Zone | Mode::Elimination);
        for (tier, target) in [
            (Medal::Gold, self.gold),
            (Medal::Silver, self.silver),
            (Medal::Bronze, self.bronze),
        ] {
            let met = if flipped {
                value >= target
            } else {
                value <= target
            };
            if met {
                return Some(tier);
            }
        }
        None
    }
}

/// The medal tier [`Cell::evaluate_medal`] (`Cell_EvaluateMedal`,
/// `0x088bf620`) produces: ordinal `0 = gold`, `1 = silver`, `2 = bronze`.
/// [`Ord`] is derived in this declaration order, so `Gold < Silver < Bronze`
/// and the *smaller* value is the *better* medal - a caller comparing two
/// medals with `min` picks the better one, matching the original's own
/// "0 is best" convention.
///
/// **Not the in-race HUD's own medal-tier ordinal**, which runs the
/// opposite way (`0 = BRONZE, 1 = SILVER, 2 = GOLD, 3 = RECORD`) and is a
/// separate, still-unread value - see
/// `docs/ghidra/functions/psp-pulse-usa/race-campaign.md`'s "what is not
/// determined" section.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Medal {
    /// The best tier. Declared first for the [`Ord`] reasoning above.
    Gold,
    /// The middle tier.
    Silver,
    /// The worst tier a race can still be awarded.
    Bronze,
}

impl Medal {
    /// `Cell_MedalPoints` (`0x088bf530`): gold 3, silver 2, bronze 1. Called
    /// with a stored medal it is the value that cell contributed to a
    /// grid's earned points; called with [`Medal::Gold`] it is what any
    /// cell is worth at best, which is always `3`.
    #[must_use]
    pub fn points(self) -> u32 {
        match self {
            Self::Gold => 3,
            Self::Silver => 2,
            Self::Bronze => 1,
        }
    }
}

/// Reads an **already expanded** `grid_NN.xml`.
///
/// Use [`from_blob`] for bytes straight out of an archive.
pub fn parse(expanded: &str) -> Result<Grid> {
    let root = fexml::parse(expanded);
    let grid = root
        .children_named("PI_Grid")
        .next()
        .ok_or(Error::MissingGrid)?;

    let name = grid
        .attr("name")
        .ok_or(Error::MissingAttribute {
            element: "PI_Grid",
            attribute: "name",
        })?
        .to_string();

    let values = child(grid, "PI_Grid", "Values")?;

    let required_points = required_u32(values, "Values", "RequiredPoints")?;
    let locked = required_bool(values, "Values", "Locked")?;
    let group = optional_u32(values, "Group")?;

    let unlock_grid = grid
        .children_named("Unlock")
        .next()
        .and_then(|u| u.attr("Grid"))
        .map(str::to_string);

    let cells = grid
        .children_named("PI_Cell")
        .map(parse_cell)
        .collect::<Result<Vec<_>>>()?;

    let campaign = values.attr("Campaign").map(str::to_string);
    let title_color = values.attr("TitleColor").map(str::to_string);
    let text_color = values.attr("TextColor").map(str::to_string);
    let flyer_name = values.attr("FlyerName").map(str::to_string);
    let billboard_name = values.attr("BillboardName").map(str::to_string);

    Ok(Grid {
        name,
        required_points,
        locked,
        group,
        unlock_grid,
        cells,
        campaign,
        title_color,
        text_color,
        flyer_name,
        billboard_name,
    })
}

fn parse_cell(cell: &Node) -> Result<Cell> {
    let name = cell
        .attr("name")
        .ok_or(Error::MissingAttribute {
            element: "PI_Cell",
            attribute: "name",
        })?
        .to_string();

    let values = child(cell, "PI_Cell", "Values")?;

    let mode_name = values.attr("mode").ok_or(Error::MissingAttribute {
        element: "Values",
        attribute: "mode",
    })?;
    // Anything not one of the nine named ordinals is kept raw rather than
    // rejected - see `Mode::Other`'s own docs for why, and the module docs'
    // "a parser cannot fail on a field it does not understand" rule.
    let mode = Mode::from_name(mode_name).unwrap_or_else(|| Mode::Other(mode_name.to_string()));

    let class = values
        .attr("class")
        .ok_or(Error::MissingAttribute {
            element: "Values",
            attribute: "class",
        })?
        .to_string();

    let (gold, silver, bronze, difficulty_targets) = parse_targets(cell)?;
    let nitro_elimination_targets = parse_nitro_elimination_targets(cell)?;

    let tournament_tracks = cell
        .children_named("TournamentTrack")
        .map(|t| {
            t.attr("track")
                .ok_or(Error::MissingAttribute {
                    element: "TournamentTrack",
                    attribute: "track",
                })
                .map(str::to_string)
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(Cell {
        name,
        track: values.attr("track").map(str::to_string),
        mode,
        class,
        weapons: required_bool(values, "Values", "Weapons")?,
        damage: required_bool(values, "Values", "damage")?,
        locked: optional_bool_result(values, "Locked")?,
        status: optional_bool_result(values, "Status")?,
        ai_count: optional_u32(values, "AICount")?,
        skill: optional_f32_alias(values, "skill", "skillMedium")?,
        skill_easy: optional_f32(values, "skillEasy")?,
        skill_hard: optional_f32(values, "skillHard")?,
        laps: optional_u32(values, "laps")?,
        ship: values.attr("ship").map(str::to_string),
        ship_choice: optional_bool_result(values, "ShipChoice")?,
        gold,
        silver,
        bronze,
        tournament_tracks,
        difficulty_targets,
        nitro_elimination_targets,
    })
}

/// A cell's own gold/silver/bronze, either shape: Pulse's (and some of
/// Wipeout HD's) single `<Gold>`/`<Silver>`/`<Bronze>`, or Wipeout HD's own
/// per-difficulty nine elements. Returns the flat triple either way - the
/// per-difficulty case's own `medium` rung, per [`Cell::gold`]'s docs - plus
/// the full per-difficulty structure when that is the shape authored.
fn parse_targets(cell: &Node) -> Result<(i64, i64, i64, Option<DifficultyTargets>)> {
    if cell.children_named("Gold").next().is_some() {
        let gold = required_i64(child(cell, "PI_Cell", "Gold")?, "Gold", "Target")?;
        let silver = required_i64(child(cell, "PI_Cell", "Silver")?, "Silver", "Target")?;
        let bronze = required_i64(child(cell, "PI_Cell", "Bronze")?, "Bronze", "Target")?;
        return Ok((gold, silver, bronze, None));
    }

    let targets = |gold_el, silver_el, bronze_el| -> Result<MedalTargets> {
        Ok(MedalTargets {
            gold: required_i64(child(cell, "PI_Cell", gold_el)?, gold_el, "Target")?,
            silver: required_i64(child(cell, "PI_Cell", silver_el)?, silver_el, "Target")?,
            bronze: required_i64(child(cell, "PI_Cell", bronze_el)?, bronze_el, "Target")?,
        })
    };
    let easy = targets("EasyGold", "EasySilver", "EasyBronze")?;
    let medium = targets("MediumGold", "MediumSilver", "MediumBronze")?;
    let hard = targets("HardGold", "HardSilver", "HardBronze")?;

    Ok((
        medium.gold,
        medium.silver,
        medium.bronze,
        Some(DifficultyTargets { easy, medium, hard }),
    ))
}

/// `<NitroElimNovice>`/`<NitroElimSkilled>`/`<NitroElimElite>`, when a cell
/// authors them - see [`Cell::nitro_elimination_targets`].
fn parse_nitro_elimination_targets(cell: &Node) -> Result<Option<(i64, i64, i64)>> {
    if cell.children_named("NitroElimNovice").next().is_none() {
        return Ok(None);
    }
    Ok(Some((
        required_i64(
            child(cell, "PI_Cell", "NitroElimNovice")?,
            "NitroElimNovice",
            "Target",
        )?,
        required_i64(
            child(cell, "PI_Cell", "NitroElimSkilled")?,
            "NitroElimSkilled",
            "Target",
        )?,
        required_i64(
            child(cell, "PI_Cell", "NitroElimElite")?,
            "NitroElimElite",
            "Target",
        )?,
    )))
}

/// Reads an archive blob, expanding it first if it needs it.
///
/// PSP stores these shortened; a future release found to ship them plain would
/// still read correctly, per [`fexml::text`].
pub fn from_blob(data: &[u8]) -> Result<Grid> {
    parse(&fexml::text(data)?)
}

/// The `Src=` of every `<LoadXML>` a `Data\Plugins\grids\Definition.xml`
/// lists, in document order.
///
/// A generic reader rather than a hard-coded `grid_00..15` list, since only
/// the USA PSP pressing has been checked - a different release naming a
/// different count or a different naming scheme is exactly the kind of thing
/// this function exists to surface rather than assume away.
#[must_use]
pub fn definition_entries(expanded: &str) -> Vec<String> {
    let root = fexml::parse(expanded);
    let mut out = Vec::new();
    collect_load_xml(&root, &mut out);
    out
}

fn collect_load_xml(node: &Node, out: &mut Vec<String>) {
    for child in &node.children {
        if child.name.eq_ignore_ascii_case("LoadXML")
            && let Some(src) = child.value("Src")
        {
            out.push(src.to_string());
        }
        collect_load_xml(child, out);
    }
}

fn child<'a>(
    parent: &'a Node,
    parent_name: &'static str,
    element: &'static str,
) -> Result<&'a Node> {
    parent
        .children_named(element)
        .next()
        .ok_or(Error::MissingElement {
            parent: parent_name,
            element,
        })
}

/// Every spelling a shipped `grid_NN.xml` uses for a boolean attribute:
/// `Weapons`/`damage` write `on`/`off`, `Locked`/`Status` write `true`/`false`,
/// `ShipChoice` writes `Yes`/`No`. Recognised explicitly rather than "anything
/// not true-shaped is false" - an attribute carrying some fourth spelling is a
/// finding, not a silent `false`.
fn parse_bool(element: &'static str, attribute: &'static str, raw: &str) -> Result<bool> {
    match raw.trim() {
        v if v.eq_ignore_ascii_case("true")
            || v == "1"
            || v.eq_ignore_ascii_case("on")
            || v.eq_ignore_ascii_case("yes") =>
        {
            Ok(true)
        }
        v if v.eq_ignore_ascii_case("false")
            || v == "0"
            || v.eq_ignore_ascii_case("off")
            || v.eq_ignore_ascii_case("no") =>
        {
            Ok(false)
        }
        _ => Err(Error::NotANumber {
            element,
            attribute,
            value: raw.to_string(),
        }),
    }
}

fn required_bool(node: &Node, element: &'static str, attribute: &'static str) -> Result<bool> {
    let raw = node
        .attr(attribute)
        .ok_or(Error::MissingAttribute { element, attribute })?;
    parse_bool(element, attribute, raw)
}

fn optional_bool_result(node: &Node, attribute: &'static str) -> Result<Option<bool>> {
    let Some(raw) = node.attr(attribute) else {
        return Ok(None);
    };
    parse_bool("Values", attribute, raw).map(Some)
}

fn required_u32(node: &Node, element: &'static str, attribute: &'static str) -> Result<u32> {
    let raw = node
        .attr(attribute)
        .ok_or(Error::MissingAttribute { element, attribute })?;
    raw.trim().parse().map_err(|_| Error::NotANumber {
        element,
        attribute,
        value: raw.to_string(),
    })
}

fn optional_u32(node: &Node, attribute: &'static str) -> Result<Option<u32>> {
    let Some(raw) = node.attr(attribute) else {
        return Ok(None);
    };
    raw.trim().parse().map(Some).map_err(|_| Error::NotANumber {
        element: "Values",
        attribute,
        value: raw.to_string(),
    })
}

fn optional_f32(node: &Node, attribute: &'static str) -> Result<Option<f32>> {
    let Some(raw) = node.attr(attribute) else {
        return Ok(None);
    };
    let parsed: f32 = raw.trim().parse().map_err(|_| Error::NotANumber {
        element: "Values",
        attribute,
        value: raw.to_string(),
    })?;
    if parsed.is_finite() {
        Ok(Some(parsed))
    } else {
        Err(Error::NotANumber {
            element: "Values",
            attribute,
            value: raw.to_string(),
        })
    }
}

/// [`optional_f32`], trying `primary` first and `alias` when `primary` is
/// absent - Pulse's `skill` and Wipeout HD's own `skillMedium` name the same
/// value; see [`Cell::skill`]'s docs.
fn optional_f32_alias(
    node: &Node,
    primary: &'static str,
    alias: &'static str,
) -> Result<Option<f32>> {
    match optional_f32(node, primary)? {
        Some(value) => Ok(Some(value)),
        None => optional_f32(node, alias),
    }
}

fn required_i64(node: &Node, element: &'static str, attribute: &'static str) -> Result<i64> {
    let raw = node
        .attr(attribute)
        .ok_or(Error::MissingAttribute { element, attribute })?;
    raw.trim().parse().map_err(|_| Error::NotANumber {
        element,
        attribute,
        value: raw.to_string(),
    })
}

#[cfg(test)]
mod tests;
