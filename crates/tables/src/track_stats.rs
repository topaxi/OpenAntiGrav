//! `stats.xml` / `stats_reversed.xml`: a track's per-class numbers, in
//! `FEData.wad` at the directory [`crate::race_campaign`]'s `Cell::track`
//! resolves through a title's `PI_Track` catalogue
//! (`oag_raceplay::catalogue::Track::location`), `<that directory>\stats.xml`.
//! `pulse-psp-usa.chd`'s `FEData.wad` has an entry hashing to
//! `Data\Environments\16_Track\stats.xml`, whose content matches
//! `docs/formats/race-setup.md`'s live capture of `16_Track` exactly
//! (`Physical Length="5178"`, `SkillScaleValue` `0.9` at `Easy`/`Venom`).
//!
//! ```text
//! <RaceTimes Venom="117" Flash="138" Rapier="119" Phantom="128"/>
//! <LapTimes  Venom="38"  Flash="33"  Rapier="29"  Phantom="25"/>
//! <Targets Elimination="10" Zone="25"/>
//! <Physical Length="5178"/>
//! <SkillLevels>
//!   <Entry Difficulty="Easy" Class="Venom" SkillScaleValue="0.9"/>
//!   <!-- 12 rows, {Easy,Medium,Hard} x {Venom,Flash,Rapier,Phantom} -->
//!   <ModeModifiers Class="Venom" HeadToHead="1.7" FullGridWithWeapons="0.0"
//!                  HalfGridWithWeapons="0.0" FullGridWithoutWeapons="0.1"
//!                  HalfGridWithoutWeapons="0.0"/>
//!   <!-- 4 rows, one per Class -->
//! </SkillLevels>
//! ```
//!
//! Names are already expanded by [`crate::fexml`]; on disc they are `<k>`/`<l>`/
//! `<c>`/`<f>` with short attributes. **The dictionary, not the order, says which
//! class a figure is**: the file spells `<RaceTimes n="138" m="128" o="119"
//! p="117"/>` with `n`=Flash, `m`=Phantom, `o`=Rapier, `p`=Venom. (An earlier
//! comment had them rotated; a live PPSSPP read of `DAT_08b310b4+0xa0` gives
//! `(117.0, 138.0, 119.0, 128.0)` in Venom/Flash/Rapier/Phantom order.)
//! `RaceTimes` are **seconds**: `PlayerStatus_Update` multiplies by 100 into
//! centiseconds, and the HUD's `record` counts down from `1.57.0`.
//!
//! `TrackStats_Load` (`0x088c454c`) and `TrackStats_ParseElement` (`0x088c46f8`)
//! in `docs/ghidra/functions/psp-pulse-usa/race-campaign.md` are the law this is
//! read against, confidence 90 there; this reader adds only "the shipped file
//! matches this shape".
//!
//! # What this crate does with it
//!
//! [`resolve_skill_scale`] reimplements **only** the campaign-cell branch of
//! `AI_ResolveSkillScale` (`0x08834df4`, confidence 85): a cell's interpolation
//! replaces the mode-modifier arithmetic (the decompile reassigns the working
//! value outright), so [`TrackStats::skill_curve`]'s `ModeModifiers` are carried
//! but not read by it.
//!
//! `RaceTimes`/`LapTimes`/`Targets`/`Physical` are parsed and carried too. This
//! crate reads none; `oag_hud::RecordTarget` does, for the Time Trial and Speed
//! Lap `RECORD` readout (whole race, one lap; seconds). A single number per
//! class carries no gold/silver/bronze split (`docs/formats/race-setup.md`); that
//! split is a campaign cell's.

use crate::fexml::{self, Node};
use crate::handling::SpeedClass;
use crate::race_campaign::{Cell, Difficulty};

/// One track's `stats.xml` record.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TrackStats {
    /// `<RaceTimes>`, one figure per class, indexed by [`SpeedClass`]'s
    /// discriminant. Seconds.
    pub race_times: [f32; 4],
    /// `<LapTimes>`, the same shape.
    pub lap_times: [f32; 4],
    /// `<Targets Elimination="..."/>`: the kill count. `None` when absent (never
    /// missing across `pulse-psp-usa.chd`'s 24 files).
    pub elimination_target: Option<u32>,
    /// `<Targets Zone="..."/>` - the zone count.
    pub zone_target: Option<u32>,
    /// `<Physical Length="..."/>`, in the units `race-setup.md`'s PPSSPP capture
    /// measured metres in.
    pub length: Option<u32>,
    /// `<SkillLevels><Entry>`: `[class][difficulty]`, both by discriminant
    /// (`Difficulty::Easy` is `0`); see [`TrackStats::skill_curve`].
    pub skill_scale: [[f32; 3]; 4],
    /// `<SkillLevels><ModeModifiers>`, one row a class; read by
    /// [`ambient_skill_scale`], not [`resolve_skill_scale`].
    pub mode_modifiers: [ModeModifiers; 4],
}

/// One `<ModeModifiers>` row: `AI_ResolveSkillScale`'s non-campaign terms, added
/// to the plain `SkillScaleValue` when no cell is in play
/// ([`ambient_skill_scale`]).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ModeModifiers {
    /// Added when `mode == Head2Head`.
    pub head_to_head: f32,
    /// Added on a full, weapons-on grid.
    pub full_grid_with_weapons: f32,
    /// Added on a half grid, weapons on.
    pub half_grid_with_weapons: f32,
    /// Added on a full grid, weapons off.
    pub full_grid_without_weapons: f32,
    /// Added on a half grid, weapons off.
    pub half_grid_without_weapons: f32,
}

impl TrackStats {
    /// This track's `SkillScaleValue` curve for `class`, `[Easy, Medium, Hard]`:
    /// the points [`resolve_skill_scale`] interpolates between.
    #[must_use]
    pub fn skill_curve(&self, class: SpeedClass) -> [f32; 3] {
        self.skill_scale[class as usize]
    }
}

/// Something wrong with a `stats.xml` document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The blob could not be expanded out of its shortened form.
    Expand(fexml::Error),
    /// A required element is absent.
    MissingElement {
        /// The element that was looked for.
        element: &'static str,
    },
    /// A required attribute is absent.
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
    /// `<SkillLevels>` lacks an `<Entry>` for one of the twelve `{Easy,Medium,Hard}
    /// x {Venom,Flash,Rapier,Phantom}` combinations all 24 shipped files carry.
    MissingSkillEntry {
        difficulty: &'static str,
        class: &'static str,
    },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Expand(e) => write!(f, "expanding shortened XML: {e}"),
            Self::MissingElement { element } => write!(f, "no <{element}> element"),
            Self::MissingAttribute { element, attribute } => {
                write!(f, "<{element}> has no {attribute} attribute")
            }
            Self::NotANumber {
                element,
                attribute,
                value,
            } => write!(f, "<{element}> {attribute}=\"{value}\" does not parse"),
            Self::MissingSkillEntry { difficulty, class } => {
                write!(f, "<SkillLevels> has no Entry for {difficulty}/{class}")
            }
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

/// Reads an **already expanded** `stats.xml`; see [`from_blob`] for raw bytes.
pub fn parse(expanded: &str) -> Result<TrackStats> {
    let root = fexml::parse(expanded);
    let stats = root
        .children_named("Stats")
        .next()
        .ok_or(Error::MissingElement { element: "Stats" })?;

    let race_times = class_row(stats, "RaceTimes")?;
    let lap_times = class_row(stats, "LapTimes")?;

    let targets = stats.children_named("Targets").next();
    let elimination_target = targets
        .and_then(|t| optional_u32(t, "Elimination"))
        .transpose()?;
    let zone_target = targets.and_then(|t| optional_u32(t, "Zone")).transpose()?;

    let length = stats
        .children_named("Physical")
        .next()
        .and_then(|p| optional_u32(p, "Length"))
        .transpose()?;

    let levels = stats.children_named("SkillLevels").next();
    let mut skill_scale = [[1.0, 2.0, 3.0]; 4];
    let mut mode_modifiers = [ModeModifiers::default(); 4];
    if let Some(levels) = levels {
        for entry in levels.children_named("Entry") {
            let difficulty_name = entry.attr("Difficulty").ok_or(Error::MissingAttribute {
                element: "Entry",
                attribute: "Difficulty",
            })?;
            let class_name = entry.attr("Class").ok_or(Error::MissingAttribute {
                element: "Entry",
                attribute: "Class",
            })?;
            let difficulty = difficulty_from_name(difficulty_name).ok_or(Error::NotANumber {
                element: "Entry",
                attribute: "Difficulty",
                value: difficulty_name.to_string(),
            })?;
            let class = SpeedClass::from_name(class_name).ok_or(Error::NotANumber {
                element: "Entry",
                attribute: "Class",
                value: class_name.to_string(),
            })?;
            let value = required_f32(entry, "Entry", "SkillScaleValue")?;
            skill_scale[class as usize][difficulty as usize] = value;
        }
        for row in levels.children_named("ModeModifiers") {
            let class_name = row.attr("Class").ok_or(Error::MissingAttribute {
                element: "ModeModifiers",
                attribute: "Class",
            })?;
            let class = SpeedClass::from_name(class_name).ok_or(Error::NotANumber {
                element: "ModeModifiers",
                attribute: "Class",
                value: class_name.to_string(),
            })?;
            mode_modifiers[class as usize] = ModeModifiers {
                head_to_head: required_f32(row, "ModeModifiers", "HeadToHead")?,
                full_grid_with_weapons: required_f32(row, "ModeModifiers", "FullGridWithWeapons")?,
                half_grid_with_weapons: required_f32(row, "ModeModifiers", "HalfGridWithWeapons")?,
                full_grid_without_weapons: required_f32(
                    row,
                    "ModeModifiers",
                    "FullGridWithoutWeapons",
                )?,
                half_grid_without_weapons: required_f32(
                    row,
                    "ModeModifiers",
                    "HalfGridWithoutWeapons",
                )?,
            };
        }
    }

    Ok(TrackStats {
        race_times,
        lap_times,
        elimination_target,
        zone_target,
        length,
        skill_scale,
        mode_modifiers,
    })
}

/// [`parse`], off bytes straight out of an archive.
pub fn from_blob(data: &[u8]) -> Result<TrackStats> {
    parse(&fexml::text(data)?)
}

fn difficulty_from_name(name: &str) -> Option<Difficulty> {
    match name {
        s if s.eq_ignore_ascii_case("Easy") => Some(Difficulty::Easy),
        s if s.eq_ignore_ascii_case("Medium") => Some(Difficulty::Medium),
        s if s.eq_ignore_ascii_case("Hard") => Some(Difficulty::Hard),
        _ => None,
    }
}

fn class_row(stats: &Node, element: &'static str) -> Result<[f32; 4]> {
    let node = stats
        .children_named(element)
        .next()
        .ok_or(Error::MissingElement { element })?;
    let mut row = [0.0; 4];
    for class in SpeedClass::ALL {
        row[class as usize] = required_f32(node, element, class.as_str())?;
    }
    Ok(row)
}

fn required_f32(node: &Node, element: &'static str, attribute: &'static str) -> Result<f32> {
    let raw = node
        .attr(attribute)
        .ok_or(Error::MissingAttribute { element, attribute })?;
    let parsed: f32 = raw.trim().parse().map_err(|_| Error::NotANumber {
        element,
        attribute,
        value: raw.to_string(),
    })?;
    if parsed.is_finite() {
        Ok(parsed)
    } else {
        Err(Error::NotANumber {
            element,
            attribute,
            value: raw.to_string(),
        })
    }
}

fn optional_u32(node: &Node, attribute: &'static str) -> Option<Result<u32>> {
    let raw = node.attr(attribute)?;
    Some(raw.trim().parse().map_err(|_| Error::NotANumber {
        element: "Targets",
        attribute,
        value: raw.to_string(),
    }))
}

/// `AI_ResolveSkillScale`'s campaign-cell branch (`0x08834df4`, confidence 85,
/// `docs/ghidra/functions/psp-pulse-usa/race-campaign.md`):
///
/// ```text
/// t = Cell_SkillForDifficulty(cell, difficulty)
/// skill = t < 2.0 ? lerp(curve[Easy], curve[Medium], t - 1.0)
///                 : lerp(curve[Medium], curve[Hard], t - 2.0)
/// ```
///
/// `curve` is [`TrackStats::skill_curve`] for `cell`'s class, or `[1.0, 2.0,
/// 3.0]` when `stats` is `None`, as the original substitutes when its track
/// table has not loaded (`DAT_08b310b4 == 0`).
///
/// `None` when `cell` has no [`Cell::skill`] (a solo-mode cell has no AI).
///
/// **Only this branch.** The decompile computes the mode-modifier terms first and
/// *discards* them once a campaign cell is in play (the working value is
/// reassigned, not added to), so a campaign launch never reaches
/// [`TrackStats::mode_modifiers`].
#[must_use]
pub fn resolve_skill_scale(
    cell: &Cell,
    difficulty: Difficulty,
    stats: Option<&TrackStats>,
) -> Option<f32> {
    let t = cell.skill_for_difficulty(difficulty)?;
    let curve = cell
        .speed_class()
        .zip(stats)
        .map_or([1.0, 2.0, 3.0], |(class, stats)| stats.skill_curve(class));
    Some(if t < 2.0 {
        lerp(curve[0], curve[1], t - 1.0)
    } else {
        lerp(curve[1], curve[2], t - 2.0)
    })
}

/// Which of `AI_ResolveSkillScale`'s mode terms a race with no campaign cell
/// adds to its track's plain `SkillScaleValue`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModeTerm {
    /// `g_game_mode == 9`: `+ HeadToHead`.
    HeadToHead,
    /// `g_game_mode == 3`, the Single Race (read `3` live on a Single Race,
    /// 2026-10-04): `+` the one of the four grid/weapons fields that matches.
    /// A full grid is eight craft.
    Race {
        /// `g_weapons_enabled`.
        weapons: bool,
        /// Eight craft on the grid.
        full_grid: bool,
    },
    /// Every other mode: nothing added.
    None,
}

/// `AI_ResolveSkillScale`'s branch for a race **with no campaign cell**
/// (`0x08834df4`, confidence 85, `docs/ghidra/functions/psp-pulse-usa/race-campaign.md`):
///
/// ```text
/// skill = stats.SkillScaleValue[class][difficulty]      // 2.0 with no table
/// skill += the ModeModifiers field `mode` selects
/// ```
///
/// Read live on a Venom Easy Single Race on Talon's Junction, 2026-10-04:
/// `0.9 + FullGridWithWeapons 0.0 = 0.9`
/// (`docs/ghidra/functions/psp-pulse-usa/race-finish.md`). With `stats` absent
/// this gives the original's `2.0` and no mode term (the original would read the
/// terms through a null table).
#[must_use]
pub fn ambient_skill_scale(
    stats: Option<&TrackStats>,
    class: SpeedClass,
    difficulty: Difficulty,
    mode: ModeTerm,
) -> f32 {
    let Some(stats) = stats else {
        return 2.0;
    };
    let base = stats.skill_curve(class)[difficulty as usize];
    let terms = stats.mode_modifiers[class as usize];
    base + match mode {
        ModeTerm::HeadToHead => terms.head_to_head,
        ModeTerm::Race {
            weapons: true,
            full_grid: true,
        } => terms.full_grid_with_weapons,
        ModeTerm::Race {
            weapons: true,
            full_grid: false,
        } => terms.half_grid_with_weapons,
        ModeTerm::Race {
            weapons: false,
            full_grid: true,
        } => terms.full_grid_without_weapons,
        ModeTerm::Race {
            weapons: false,
            full_grid: false,
        } => terms.half_grid_without_weapons,
        ModeTerm::None => 0.0,
    }
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

#[cfg(test)]
mod tests;
