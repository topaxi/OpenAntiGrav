//! The built-in campaign: `PI_Grid`/`PI_Cell`, `Data\Plugins\grids\grid_00.xml`
//! .. `grid_15.xml` inside `Data.wad`.
//!
//! Pulse's campaign is **authored data, not code**: sixteen files, each one
//! `PI_Grid` of 8-16 `PI_Cell` records (236 in all) naming track, mode, speed
//! class, laps, weapons/damage switches, AI count, AI skill and its own
//! gold/silver/bronze targets. See
//! `docs/ghidra/functions/psp-pulse-usa/race-campaign.md` for the law the schema
//! was read against (`PI_Cell_ParseElement`, `0x088bf83c`;
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
//! # The medal law lives here, who won does not
//!
//! [`Cell::evaluate_medal`] reimplements `Cell_EvaluateMedal` (a three-way
//! threshold compare, direction flipped for `Zone`/`Elimination`),
//! [`Medal::points`] `Cell_MedalPoints`, and [`grid_points_met`]
//! `Unlock_GridPointsMet` (a grid's [`Grid::points_earned`] against its
//! `required_points`). They do **not** decide which cell a race ran against or
//! what value it scored: that is the caller's job; see
//! `crates/game/src/records.rs` and `docs/architecture/persistence.md`.
//! [`oag_pulse::campaign`](../../oag_pulse/campaign/index.html) carries the
//! sixteen entry names, per [ADR-0022]: a file's *shape* lives here, what a
//! title *ships* there.
//!
//! # `class="Zone"` is authored text, not a speed class
//!
//! A `Zone` cell's `class` is the literal `Zone`, not `Venom`/`Flash`/`Rapier`/
//! `Phantom` (checked against `grid_00.xml`..`grid_15.xml` on
//! `pulse-psp-usa.chd`). `race-campaign.md`'s `g_class_name_table` reading
//! (`0x08ab067c`) lists only the four, so that table has an unread fifth entry
//! or the parser keeps unrecognised names; `CellSelection_PopulateDetail` blanks
//! the label for `Zone` either way. [`Cell::class`] is therefore the **raw**
//! string and [`Cell::speed_class`] the fallible mapping. An unrecognised
//! `class` is not an error: a parser cannot fail on a field it does not
//! understand.
//!

use std::fmt;

use crate::fexml::{self, Node};
use crate::handling::SpeedClass;

mod difficulty;
pub use difficulty::{Difficulty, DifficultyTargets, MedalTargets};
mod unlock;
pub use unlock::grid_points_met;

/// The nine modes `g_mode_name_table` (`0x08ab062c`) names, in ordinal order,
/// plus a raw catch-all.
///
/// **`7` is absent from the table** (a retired or debug mode, per
/// `race-campaign.md`), so there is no variant for it and [`Mode::from_name`]
/// returns `None` for anything outside the nine. [`Mode::Other`] is different:
/// Wipeout HD authors two spellings (`"NitroBattle"`, `"Detonator"`) this table
/// was never extended to cover; see `docs/formats/race-campaign.md`'s HD
/// section. It carries a `String`, which is why this type is not [`Copy`].
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
    /// A `mode=` spelling absent from `g_mode_name_table`: HD's `"NitroBattle"`
    /// and `"Detonator"`, only on the Fury-tagged grids (`Campaign="Fury"`). The
    /// exact text, never guessed onto a Pulse ordinal.
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

    /// The integer `g_mode_name_table` associates with this mode, as
    /// `Race_RecordResult` switches on (`mode - 3`); `None` for [`Mode::Other`].
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
    /// ordinals only; never produces [`Mode::Other`] (the parser wraps that).
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
        parent: &'static str,
        element: &'static str,
    },
    /// A required attribute is absent. Never defaulted - see the module docs.
    MissingAttribute {
        element: &'static str,
        attribute: &'static str,
    },
    /// An attribute is present but does not parse as the type it should.
    NotANumber {
        element: &'static str,
        attribute: &'static str,
        value: String,
    },
    /// **No longer produced.** A `mode=` outside the nine is [`Mode::Other`];
    /// kept for API stability.
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

pub type Result<T> = std::result::Result<T, Error>;

/// One `PI_Grid`: a tier of the campaign, 8-16 cells.
#[derive(Debug, Clone, PartialEq)]
pub struct Grid {
    /// The grid's name, e.g. `"grid0"`; what an `<Unlock Grid="Grid0"/>` matches
    /// case-insensitively.
    pub name: String,
    /// Points needed, summed across this grid's cells, to satisfy an
    /// `<Unlock Grid="..."/>` naming it. `0` on `grid15`, shown as `FE_NA`.
    pub required_points: u32,
    /// The `Locked` byte: the tier's *static default*, driving the lock glyph
    /// and, here, the Confirm gate with [`Self::points_earned`]/
    /// [`grid_points_met`]; see `docs/ghidra/functions/psp-pulse-usa/race-campaign.md`'s
    /// "Unlock rules, cell and tier".
    pub locked: bool,
    /// `Group="1"` marks `grid12`..`grid15`; consumer not traced.
    pub group: Option<u32>,
    /// The `<Unlock Grid="..."/>` child, the `target` for [`grid_points_met`].
    /// `grid1` onwards name the previous grid; `grid0` names none.
    pub unlock_grid: Option<String>,
    /// This grid's cells, in document order.
    pub cells: Vec<Cell>,
    /// `Campaign="HD"|"Fury"`, raw. Absent from every Pulse grid and some of
    /// HD's copies (`DATA02.PSARC`'s `grid_00`..`grid_07` has neither this nor
    /// the per-difficulty schema; `DATA04.PSARC`'s has the schema only;
    /// `DATA06.PSARC`'s both). See `docs/formats/race-campaign.md`'s HD section.
    pub campaign: Option<String>,
    /// `TitleColor="0xFF525252"`, raw hex text. Not parsed: the channel order
    /// (ARGB vs RGBA) is unmeasured and a guess would be an invented reading.
    pub title_color: Option<String>,
    /// `TextColor=`, the same raw treatment as [`Grid::title_color`].
    pub text_color: Option<String>,
    /// `FlyerName="01_uplift"`: the cell-selection flyer image stem. HD only.
    pub flyer_name: Option<String>,
    /// `BillboardName="Data/Billboards/HD_Adverts/.../....vex"`: HD-only advert.
    pub billboard_name: Option<String>,
}

impl Grid {
    /// The most a grid can earn: `3 * cells.len()`, since `Cell_MedalPoints(cell,
    /// 0)` is always `3`. `Grid_PointsPossible` (`0x088c0570`) computes it the
    /// same way; the ground-truth test checks 24/30/36/42/48 across the sixteen
    /// grids.
    #[must_use]
    pub fn max_points(&self) -> u32 {
        // Every cell is worth 3 at best.
        u32::try_from(self.cells.len())
            .unwrap_or(u32::MAX)
            .saturating_mul(3)
    }
}

/// One `PI_Cell`: a single event on the grid.
#[derive(Debug, Clone, PartialEq)]
pub struct Cell {
    /// The cell's name, e.g. `"grid0_2_1"`; the trailing `_<x>_<y>` is its hex
    /// grid position, see [`Cell::grid_coords`].
    pub name: String,
    /// The circuit, e.g. `"16_Track"`. **Absent for `Tournament`**, which uses
    /// [`Cell::tournament_tracks`].
    pub track: Option<String>,
    /// The event type.
    pub mode: Mode,
    /// The raw `class=` text: a speed class name, or `"Zone"` on a Zone cell;
    /// see the module docs and [`Cell::speed_class`].
    pub class: String,
    /// `Weapons="on"|"off"`.
    pub weapons: bool,
    /// `damage="on"|"off"`.
    pub damage: bool,
    /// The `Locked` byte, on a handful of cells; no consumer traced (see
    /// [`Grid::locked`]).
    pub locked: Option<bool>,
    /// The `Status` byte. No shipped grid file sets it.
    pub status: Option<bool>,
    /// Opponent count: `7` on `Race`/`Tournament`/`Elimination`, `1` on
    /// `Head2Head`, absent on solo modes.
    pub ai_count: Option<u32>,
    /// A position on the track's `SkillScaleValue` curve at medium difficulty;
    /// absent on solo modes. Also read from HD's `skillMedium` (`skill` first;
    /// never both on one cell).
    pub skill: Option<f32>,
    /// The easy position. The original's parser defaults it to `skill - 1.0`;
    /// [`Cell::skill_for_difficulty`] applies that default.
    pub skill_easy: Option<f32>,
    /// The hard position, defaulting to `skill + 1.0`; see [`Cell::skill_easy`].
    pub skill_hard: Option<f32>,
    /// Lap count. **Absent for `Elimination`** (the kill target stands in);
    /// `"0"` for `Zone` (shown as infinity); `7` for every `Speed Lap`;
    /// otherwise 3 Venom / 4 Flash / 4 Rapier / 5 Phantom across all 236 cells.
    pub laps: Option<u32>,
    /// `ship=`. Every shipped cell carries `"None"`: the campaign never forces a
    /// craft.
    pub ship: Option<String>,
    /// `ShipChoice="Yes"|"No"`.
    pub ship_choice: Option<bool>,
    /// `<Gold Target="..."/>`: a finishing position for `Race`/`Tournament`/
    /// `Head2Head`, a time in centiseconds for `Time Trial`/`Speed Lap`, a zone
    /// count for `Zone`, a kill count for `Elimination`.
    ///
    /// **With [`Cell::difficulty_targets`], this is the `medium` rung**, so
    /// [`Cell::evaluate_medal`]'s callers keep the difficulty-agnostic value
    /// Pulse always had.
    pub gold: i64,
    /// `<Silver Target="..."/>`, or the `medium` rung; see [`Cell::gold`].
    pub silver: i64,
    /// `<Bronze Target="..."/>`, or the `medium` rung; see [`Cell::gold`].
    pub bronze: i64,
    /// `<TournamentTrack track="..."/>` rows, in order. Empty except for
    /// `Tournament`, which has no [`Cell::track`].
    pub tournament_tracks: Vec<String>,
    /// Three medal-target triples, one per difficulty (`<EasyGold>`/
    /// `<MediumGold>`/`<HardGold>` and siblings). `None` on every Pulse cell and
    /// on the `grid_00`..`grid_07` copy this project's archive precedence reaches
    /// (`DATA02.PSARC`, plain `<Gold>`); `Some` on `grid_08`..`grid_15`
    /// (`DATA00.PSARC`, Fury-only) and the `DATA04`/`DATA06.PSARC` copies of
    /// `grid_00`..`07`. See `docs/formats/race-campaign.md`'s HD section.
    pub difficulty_targets: Option<DifficultyTargets>,
    /// `<NitroElimNovice>`/`<NitroElimSkilled>`/`<NitroElimElite>`: a fourth
    /// triple on every cell of a grid carrying [`Cell::difficulty_targets`]
    /// (authored as a dummy `1` on a plain `Race` or `Speed Lap`).
    ///
    /// Ordered `(novice, skilled, elite)` as the file authors the words -
    /// **chosen, not measured**: whether `elite` is this triple's gold-equivalent
    /// is not established. See `docs/formats/race-campaign.md`'s HD section.
    pub nitro_elimination_targets: Option<(i64, i64, i64)>,
}

impl Cell {
    /// [`Cell::class`] as a [`crate::handling::SpeedClass`] when it is one of the
    /// four; `None` for `"Zone"` or anything unrecognised, never an error.
    #[must_use]
    pub fn speed_class(&self) -> Option<SpeedClass> {
        SpeedClass::from_name(&self.class)
    }

    /// The `(x, y)` parsed out of the cell's name as `PI_Cell_ParseElement`
    /// does: after the **first** underscore is `x`, after the second `y`. Only
    /// right because grid names have no underscore and each coordinate is one
    /// digit, the original's own `strchr` constraint. `None` for any other shape.
    #[must_use]
    pub fn grid_coords(&self) -> Option<(u32, u32)> {
        let mut parts = self.name.splitn(3, '_');
        let _grid = parts.next()?;
        let x = parts.next()?.parse().ok()?;
        let y = parts.next()?.parse().ok()?;
        Some((x, y))
    }

    /// The skill position for a [`Difficulty`], with the original's defaults
    /// (`skillEasy = skill - 1.0`, `skillHard = skill + 1.0`) when the attribute
    /// is absent. `None` without [`Cell::skill`] (a solo cell has no AI).
    #[must_use]
    pub fn skill_for_difficulty(&self, difficulty: Difficulty) -> Option<f32> {
        let skill = self.skill?;
        Some(match difficulty {
            Difficulty::Easy => self.skill_easy.unwrap_or(skill - 1.0),
            Difficulty::Medium => skill,
            Difficulty::Hard => self.skill_hard.unwrap_or(skill + 1.0),
        })
    }

    /// The medal-target triple for a [`Difficulty`]: the matching
    /// [`Cell::difficulty_targets`] rung, else the flat gold/silver/bronze
    /// (every Pulse cell, and HD's `grid_00`..`grid_07` as this project's
    /// archive precedence reaches them).
    #[must_use]
    pub fn targets_for_difficulty(&self, difficulty: Difficulty) -> MedalTargets {
        match &self.difficulty_targets {
            Some(dt) => match difficulty {
                Difficulty::Easy => dt.easy,
                Difficulty::Medium => dt.medium,
                Difficulty::Hard => dt.hard,
            },
            None => MedalTargets {
                gold: self.gold,
                silver: self.silver,
                bronze: self.bronze,
            },
        }
    }

    /// The single target [`Cell::nitro_elimination_targets`] carries for a
    /// [`Difficulty`] (`novice`/`skilled`/`elite`, HD's words for Pulse's
    /// `Easy`/`Medium`/`Hard`; measured from the `HARD(ELITE)` string in
    /// `docs/ghidra/functions/ps3-hdfury-eu/race-campaign.md`'s medal-law
    /// section). `None` without a nitro triple.
    ///
    /// **One number, not a [`MedalTargets`] triple**, and no sibling forges one:
    /// that would invent data the disc does not author. How the executable turns
    /// it into a medal is **not measured** (same page, "what is not
    /// determined"), so a caller wiring an `Elimination`/`NitroBattle` medal must
    /// answer that first; [`Cell::evaluate_medal`] does not.
    #[must_use]
    pub fn nitro_elimination_target_for_difficulty(&self, difficulty: Difficulty) -> Option<i64> {
        let (novice, skilled, elite) = self.nitro_elimination_targets?;
        Some(match difficulty {
            Difficulty::Easy => novice,
            Difficulty::Medium => skilled,
            Difficulty::Hard => elite,
        })
    }

    /// Whether this cell's medal law is one this build has not measured:
    /// an `Elimination` or `NitroBattle` cell on Wipeout HD/Fury, whose
    /// `gold`/`silver`/`bronze` and every [`Cell::difficulty_targets`] rung are
    /// the dummy `1`/`2`/`3` and whose real target is the single
    /// [`Cell::nitro_elimination_targets`] number per rung (`200`..`300` on an
    /// `Elimination` cell, `12`..`26` on a `NitroBattle` one). Neither the unit
    /// (kills or score points) nor the way one number becomes a tier is
    /// measured (`docs/ghidra/functions/ps3-hdfury-eu/race-campaign.md`, "what
    /// is not determined"), so [`Cell::evaluate_medal_for_difficulty`] awards
    /// nothing for it rather than gold for one kill against the dummy `1`.
    /// Pulse's `Eliminator` cells keep their kill target in `gold` and carry no
    /// nitro triple, so they are not affected.
    #[must_use]
    pub fn medal_law_is_unmeasured(&self) -> bool {
        let nitro_mode = matches!(&self.mode, Mode::Elimination)
            || matches!(&self.mode, Mode::Other(name) if name == "NitroBattle");
        nitro_mode
            && self
                .nitro_elimination_targets
                .is_some_and(|t| t != (1, 1, 1))
    }

    /// `Cell_EvaluateMedal` (`0x088bf620`): a three-way threshold compare of
    /// `value` against this cell's gold/silver/bronze targets. `value` is a
    /// finishing position (`Race`/`Tournament`/`Head2Head`), a time in
    /// centiseconds (`Time Trial`/`Speed Lap`), a zone count (`Zone`) or a kill
    /// count (`Elimination`); the caller supplies it.
    ///
    /// **Not meaningful for `Elimination`/`NitroBattle` on an HD/Fury cell with
    /// [`Cell::nitro_elimination_targets`]**: its flat targets and every
    /// [`Cell::difficulty_targets`] rung are dummy `1`/`2`/`3` there (checked
    /// against `DATA00.PSARC`'s eight Fury grids,
    /// `crates/hd/tests/campaign_grids_ground_truth.rs`'s
    /// `eliminationfamily_cells_carry_a_real_nitro_triple_and_a_dummy_flat_one`).
    /// What replaces it is unmeasured; see
    /// [`Cell::nitro_elimination_target_for_difficulty`]. Left unchanged because
    /// Pulse's `Eliminator` cells keep the kill target in the flat `gold` field
    /// (`docs/ghidra/functions/psp-pulse-usa/race-campaign.md`) and are pinned by
    /// ground-truth tests.
    ///
    /// **The comparison direction flips for `Zone` and `Elimination`.** Other
    /// modes want a *lower* value (`<=`: better position or faster time); these
    /// counting modes want a *higher* one (`>=`). Without the flip, 16 zones
    /// against `20`/`17`/`15` would score gold (`16 <= 20`) when it clears only
    /// bronze (`16 >= 15`, `16 < 17`). The tests cover each wrong direction.
    ///
    /// `None` when no tier is met, or when `value` means "no result yet": the
    /// original's unset `u32` register `0xFFFF_FFFF`, or **any non-positive
    /// value**. The original checks only exactly `0`; the wider `<= 0` is
    /// **chosen, not measured**: no mode measures a negative, and it stops a
    /// negative "time" scoring gold.
    #[must_use]
    pub fn evaluate_medal(&self, value: i64) -> Option<Medal> {
        self.evaluate_medal_for_difficulty(value, Difficulty::Medium)
    }

    /// [`Cell::evaluate_medal`] against one rung of
    /// [`Cell::targets_for_difficulty`]; `evaluate_medal(v)` is
    /// `evaluate_medal_for_difficulty(v, Difficulty::Medium)`. **Which
    /// difficulty the executable's award path compares against is not
    /// measured**: `docs/ghidra/functions/ps3-hdfury-eu/race-campaign.md`'s
    /// medal-law section finds the targets selected by a `GameState` difficulty
    /// index but not the comparison consumer. The caller supplies the rung.
    #[must_use]
    pub fn evaluate_medal_for_difficulty(
        &self,
        value: i64,
        difficulty: Difficulty,
    ) -> Option<Medal> {
        if value <= 0 || value == 0xFFFF_FFFF || self.medal_law_is_unmeasured() {
            return None;
        }
        let targets = self.targets_for_difficulty(difficulty);
        let flipped = matches!(self.mode, Mode::Zone | Mode::Elimination);
        for (tier, target) in [
            (Medal::Gold, targets.gold),
            (Medal::Silver, targets.silver),
            (Medal::Bronze, targets.bronze),
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

/// The medal tier [`Cell::evaluate_medal`] (`Cell_EvaluateMedal`, `0x088bf620`)
/// produces: ordinal `0 = gold`, `1 = silver`, `2 = bronze`.
///
/// [`Ord`] follows declaration order, so the *smaller* value is the *better*
/// medal (the original's "0 is best"). **Not the in-race HUD's tier ordinal**,
/// which runs `0 = BRONZE .. 3 = RECORD` and is unread; see
/// `docs/ghidra/functions/psp-pulse-usa/race-campaign.md`'s "what is not
/// determined".
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Medal {
    /// The best tier; declared first for [`Ord`].
    Gold,
    /// The middle tier.
    Silver,
    /// The worst tier a race can still be awarded.
    Bronze,
}

impl Medal {
    /// `Cell_MedalPoints` (`0x088bf530`): gold 3, silver 2, bronze 1. With
    /// [`Medal::Gold`] it is what any cell is worth at best, always `3`.
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
    // Anything outside the nine ordinals is kept raw, see `Mode::Other`.
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

/// A cell's gold/silver/bronze in either shape: the flat `<Gold>`/`<Silver>`/
/// `<Bronze>` or HD's nine per-difficulty elements. Returns the flat triple
/// (the `medium` rung, per [`Cell::gold`]) plus the full structure when authored.
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

/// `<NitroElimNovice>`/`<NitroElimSkilled>`/`<NitroElimElite>`, when authored;
/// see [`Cell::nitro_elimination_targets`].
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

/// Reads an archive blob, expanding it first if needed (PSP stores these
/// shortened; see [`fexml::text`]).
pub fn from_blob(data: &[u8]) -> Result<Grid> {
    parse(&fexml::text(data)?)
}

/// The `Src=` of every `<LoadXML>` in a `Data\Plugins\grids\Definition.xml`, in
/// document order.
///
/// Generic rather than a hard-coded `grid_00..15`: only the USA PSP pressing has
/// been checked.
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

/// Every spelling a shipped `grid_NN.xml` uses for a boolean: `on`/`off`
/// (`Weapons`/`damage`), `true`/`false` (`Locked`/`Status`), `Yes`/`No`
/// (`ShipChoice`). Recognised explicitly: a fourth spelling is a finding, not a
/// silent `false`.
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

/// [`optional_f32`], trying `primary` then `alias`: Pulse's `skill` and HD's
/// `skillMedium` are the same value; see [`Cell::skill`].
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
