//! Wipeout 2048's race-ending pages, read off an open source and filled from
//! a finished race - the `oag_title::EndRaceDialect::Touch` half of
//! [`super`]. The model and the drawing are [`oag_ui_screens::endrace::touch`];
//! this reads the disc and words the result.

use anyhow::{Context, Result};
use oag_2048::campaign::{EventObjectives, Tier};
use oag_hud::sprite::Sheet;
use oag_ui::language::StringTable;
use oag_ui::screen::Screens;
use oag_ui_screens::endrace::touch::{Layouts, ObjectiveRow, Summary, Tone};
use oag_ui_screens::picker::FaceScales;

use crate::boot::campaign2048::objective_text;
use crate::boot::sprites::read_front_end_first;

/// What [`load`] reads: the pages' layout and a sheet holding every texture
/// they name.
#[derive(Debug)]
pub struct TouchScreens {
    /// The disc's layout for the shell and its pages.
    pub layouts: Layouts,
    /// The front end's sheet, extended with whatever the pages draw that it
    /// did not already hold.
    pub sprites: Sheet,
}

/// Reads `entry` (the title's [`oag_title::FrontEnd::endrace_entry`]) off
/// `archives`. `fallback_globals` is the front end's own `Skin.xml` colour
/// table, which the file's `FEGlobals->` colours resolve through.
///
/// A texture that will not read is logged and left out, so the widget it is for
/// draws nothing - never a stand-in.
///
/// # Errors
///
/// A missing or unreadable file, or one without the shell and first page.
pub fn load(
    archives: &mut oag_assets::Archives,
    entry: &str,
    strings: &StringTable,
    faces: FaceScales,
    grid: [f32; 2],
    base: &Sheet,
    fallback_globals: &[(&str, &str)],
) -> Result<TouchScreens> {
    let blob = archives
        .read_name(entry)
        .with_context(|| format!("reading {entry}"))?;
    let xml = oag_ui::xml::expand(&blob).context("expanding the EndRace definition")?;
    let screens = Screens::from_xml_folding_fill_offsets(&xml, fallback_globals);
    let layouts = Layouts::read(&screens, strings, faces, grid)
        .context("the EndRace definition has no shell, first page or colour globals")?;

    let mut srcs: Vec<&str> = Vec::new();
    for layout in [Some(&layouts.shell), Some(&layouts.summary), layouts.objectives.as_ref()]
        .into_iter()
        .flatten()
    {
        let screen = &layout.screen;
        let icons = screen.touch_buttons.iter().filter_map(|b| b.src.as_deref());
        for src in screen.images.iter().map(|i| i.src.as_str()).chain(icons) {
            if base.get(src).is_none() && !srcs.contains(&src) {
                srcs.push(src);
            }
        }
    }
    let mut blobs = Vec::new();
    for src in srcs {
        match read_front_end_first(archives, src) {
            Ok(blob) => blobs.push((src.to_string(), blob)),
            Err(error) => log::warn!("{src}: {error:#} - the widget it is for draws nothing"),
        }
    }
    let mut report = Vec::new();
    let sprites = base.extended(&blobs, &mut report);
    oag_raceplay::loader_log::lines(report.iter().map(|line| format!("endrace sprites {line}")));
    Ok(TouchScreens { layouts, sprites })
}

/// What the race came to, in the terms the pages say it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Standing {
    /// A finishing place, `1`-based.
    Place(u8),
    /// A total or best-lap time, in centiseconds.
    Time(i64),
    /// The zone counter reached.
    Zone(u16),
    /// Opponent kills scored.
    Kills(u32),
}

/// A finished race's facts, as [`summary`] takes them.
#[derive(Debug, Clone, Copy)]
pub struct Facts {
    /// The mode the race ran in.
    pub mode: oag_race::Mode,
    /// What it came to.
    pub standing: Standing,
    /// The event's own bars, when it authors them.
    pub objectives: Option<EventObjectives>,
    /// The bar the race cleared, by `oag_2048::campaign::evaluate_tier`.
    pub tier: Option<Tier>,
}

/// The page title's string id: Measured, `FUN_810cf5fe`.
fn title_id(mode: oag_race::Mode) -> &'static str {
    match mode {
        oag_race::Mode::SpeedLap => "FE_END_SPEED_LAP_SUMMARY",
        oag_race::Mode::TimeTrial => "FE_END_TIMETRIAL_SUMMARY",
        oag_race::Mode::Zone => "FE_END_ZONE_SUMMARY",
        oag_race::Mode::Eliminator => "FE_END_COMBAT_SUMMARY",
        _ => "FE_ENDRACE_SUMMARY",
    }
}

/// `M:SS.CC` for a time in centiseconds. **Chosen**: the original's string is
/// not traced.
fn clock(centiseconds: i64) -> String {
    let c = centiseconds.max(0);
    format!("{}:{:02}.{:02}", c / 6000, (c / 100) % 60, c % 100)
}

fn standing_text(strings: &StringTable, standing: Standing) -> String {
    match standing {
        Standing::Place(place) => {
            let id = match place {
                1 => "IG_HUD_1ST",
                2 => "IG_HUD_2ND",
                3 => "IG_HUD_3RD",
                4 => "IG_HUD_4TH",
                5 => "IG_HUD_5TH",
                6 => "IG_HUD_6TH",
                7 => "IG_HUD_7TH",
                _ => "IG_HUD_8TH",
            };
            strings
                .get(id)
                .map_or_else(|| place.to_string(), str::to_string)
        }
        Standing::Time(centiseconds) => clock(centiseconds),
        Standing::Zone(zone) => zone.to_string(),
        Standing::Kills(kills) => kills.to_string(),
    }
}

/// The `typedef` an objective is worded under: `objective_text` words a
/// `BEAT_VALUE` by the event's own game mode, and `EventKind` is as much of
/// that as a finished race still knows.
fn typedef_of(kind: oag_2048::campaign::EventKind) -> i64 {
    use oag_2048::campaign::EventKind;
    use oag_tables::mjolnir::campaign::typedef;
    match kind {
        EventKind::Zone => typedef::ZONE,
        EventKind::Race => typedef::RACE_A,
        _ => 0,
    }
}

/// Words a finished race for the pages. Strings that do not resolve are left
/// out rather than shown as ids.
#[must_use]
pub fn summary(facts: &Facts, strings: &StringTable) -> Summary {
    let speed_shape = facts.mode == oag_race::Mode::SpeedLap;
    let word = |id: &str| strings.get(id).map(str::to_string);
    let tone = match (facts.objectives, facts.tier) {
        (_, Some(Tier::Elite)) => Tone::Elite,
        (_, Some(Tier::Pass)) => Tone::Pass,
        (Some(_), None) if !speed_shape => Tone::Fail,
        _ => Tone::Unjudged,
    };
    let line = |rule: &oag_2048::campaign::ObjectiveRule| {
        let kind = facts.objectives?.kind;
        objective_text(
            strings,
            typedef_of(kind),
            rule.objective_type?,
            rule.target.unwrap_or(0),
        )
    };
    let objective = facts.objectives.as_ref().and_then(|o| line(&o.pass));
    let rows = facts
        .objectives
        .as_ref()
        .map(|o| {
            [
                (line(&o.pass), Tone::Pass, facts.tier.is_some()),
                (line(&o.elite), Tone::Elite, facts.tier == Some(Tier::Elite)),
            ]
            .into_iter()
            .filter_map(|(text, tone, met)| {
                Some(ObjectiveRow {
                    text: text?,
                    state: if met { tone } else { Tone::Fail },
                })
            })
            .collect()
        })
        .unwrap_or_default();
    let (message, medal_label) = match tone {
        Tone::Pass => (word("ER_CONGRAT"), word("FE_PASS")),
        Tone::Elite => (word("ER_CONGRAT"), word("FE_ELITE_PASS")),
        Tone::Fail => (word("ER_END_TOUR_7"), word("FE_FAIL")),
        Tone::Unjudged => (speed_shape.then(|| word("IG_HUD_BESTLAP")).flatten(), None),
    };
    Summary {
        title: word(title_id(facts.mode)),
        message,
        objective,
        result: Some(standing_text(strings, facts.standing)),
        speed_shape,
        tone,
        medal_label,
        rows,
    }
}
