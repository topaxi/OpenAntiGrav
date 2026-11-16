//! Wipeout 2048's race-ending pages, read off an open source and filled from
//! a finished race - the `oag_title::EndRaceDialect::Touch` half of
//! [`super`]. The model and the drawing are [`oag_ui_screens::endrace::touch`];
//! this reads the disc and words the result.

use anyhow::{Context, Result};
use oag_2048::campaign::{EventObjectives, Tier};
use oag_hud::sprite::Sheet;
use oag_tables::xp::Ranks;
use oag_ui::language::StringTable;
use oag_ui::screen::Screens;
use oag_ui_screens::endrace::touch::xp::{XpResult, XpWords};
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
    for layout in [
        Some(&layouts.shell),
        Some(&layouts.summary),
        layouts.objectives.as_ref(),
    ]
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

impl Facts {
    /// A race's own facts: what it came to in its own mode, graded against the
    /// event's `objectives` by `oag_2048::campaign::evaluate_tier` - the same
    /// law `RaceStage::campaign_2048_medal` grades the stored medal by, fed
    /// the same outcome.
    ///
    /// **What a mode "comes to" is chosen, not measured**: a place for a race,
    /// the total time for a time trial, the best lap for a speed lap, the zone
    /// reached, the kills scored.
    #[must_use]
    pub fn from_race(race: &oag_raceplay::Race, objectives: Option<EventObjectives>) -> Self {
        // The race's own mode, not the menu's: an event launched off the map
        // carries its own, and `race_options.mode` still says what the RACE
        // page held.
        let mode = race.sim.world.mode();
        let standing = race.player_standing();
        let finished = race.finished();
        let total = oag_game_ticks(oag_race::race_clock_ticks(
            standing.finish_tick.unwrap_or(race.sim.world.tick),
        ));
        let zone = race.sim.world.primary_race().zone;
        let place = race.player_place();
        let shown = match mode {
            oag_race::Mode::TimeTrial => Standing::Time(total),
            oag_race::Mode::SpeedLap => standing
                .best_lap_ticks
                .map_or(Standing::Place(place), |ticks| {
                    Standing::Time(oag_game_ticks(u64::from(ticks)))
                }),
            oag_race::Mode::Zone => Standing::Zone(zone),
            oag_race::Mode::Eliminator => Standing::Kills(standing.kills),
            _ => Standing::Place(place),
        };
        let outcome = oag_2048::campaign::EventOutcome {
            finished,
            place,
            finish_centiseconds: finished.then_some(total),
            zone,
            kills: standing.kills,
        };
        Self {
            mode,
            standing: shown,
            objectives,
            tier: objectives.and_then(|o| oag_2048::campaign::evaluate_tier(&o, &outcome)),
        }
    }
}

fn oag_game_ticks(ticks: u64) -> i64 {
    crate::medal_watch::ticks_to_centiseconds(ticks)
}

/// A raced event's result and what banking it paid.
#[derive(Debug)]
pub struct FinishedEvent {
    /// What the race came to.
    pub facts: Facts,
    /// What banking its tally into a profile at `prior_xp` paid; `None` when the event carries no
    /// XP table.
    pub xp: Option<(XpBanked, Ranks)>,
}

/// Races a 2048 campaign event to its end under the autopilot and returns what
/// it came to - the real result a `--menu-page endrace-summary --event` still
/// and the disc-backed ground truth are fed from. The race's XP tally is armed
/// as the session arms it and banked, through [`crate::records::Store::bank_event_xp`], into a
/// profile that already held `prior_xp`.
///
/// # Errors
///
/// The event does not load, or the race is still running after `cap` ticks.
pub fn finished_event(
    options: &oag_raceplay::Options,
    name: &str,
    cap: u64,
    prior_xp: u32,
    prior_medal: Option<crate::records::Medal>,
) -> Result<FinishedEvent> {
    let loaded = oag_raceplay::load_event(options, name)?;
    let progress = loaded
        .campaign_2048_event
        .clone()
        .context("the event carries no progress")?;
    let mut race = oag_raceplay::Race::start(loaded.setup);
    race.arm_event_xp(&progress);
    race.set_autopilot(true);
    while !race.finished() && race.sim.world.tick < cap {
        race.tick(&oag_gameplay::PlayerInputs::none());
    }
    anyhow::ensure!(
        race.finished(),
        "{name} was still running after {cap} ticks"
    );
    let facts = Facts::from_race(&race, progress.objectives);
    let medal = match facts.tier {
        Some(Tier::Elite) => Some(crate::records::Medal::Gold),
        Some(Tier::Pass) => Some(crate::records::Medal::Bronze),
        None => None,
    };
    let mut profile = crate::records::Store::default();
    profile.bank_xp(prior_xp);
    let place = Some(race.player_place());
    let banked = race.xp_tally_mut().map(|tally| {
        let earned = profile.bank_event_xp(tally, prior_medal, medal, place);
        XpBanked {
            earned,
            bonus: tally.completion_xp(),
            total_after: profile.xp(),
        }
    });
    Ok(FinishedEvent {
        facts,
        xp: banked.zip(progress.xp_ranks),
    })
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

/// What a finish banked into the profile's XP total, as the page needs it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct XpBanked {
    /// What the race paid in all: its tally, the completion and finish awards included.
    pub earned: u32,
    /// The completion award among it (`PassBonusXP`), `0` for none.
    pub bonus: u32,
    /// The profile's total after banking.
    pub total_after: u32,
}

/// The XP block of a finished race, worded from the disc's own strings. `None` when a string does
/// not resolve: the block then draws nothing rather than a stand-in.
#[must_use]
pub fn xp_result(banked: XpBanked, ranks: &Ranks, strings: &StringTable) -> Option<XpResult> {
    let word = |id: &str| strings.get(id).map(str::to_string);
    Some(XpResult {
        race_xp: banked.earned.saturating_sub(banked.bonus),
        bonus_xp: banked.bonus,
        event_total: banked.earned,
        total_after: banked.total_after,
        ranks: ranks.clone(),
        words: XpWords {
            event_total: word("RESULTS_EVENT_TOTAL")?,
            profile_total: word("TOTAL_NUMBER_XP")?,
            new_rank: word("RESULTS_NEW_RANK")?,
        },
    })
}

/// Words a finished race for the pages. Strings that do not resolve are left
/// out rather than shown as ids.
#[must_use]
pub fn summary(facts: &Facts, strings: &StringTable, xp: Option<XpResult>) -> Summary {
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
    // **The chain the pages show.** `FUN_810cf5fe` points its objective list at
    // the event's elite chain once the elite bar is met and at the pass chain
    // otherwise, words the first objective of that chain on both pages, and
    // `FUN_810d0c46` ticks its row: `Pass`/`ElitePass` when met (by which
    // chain it is), `Fail` when not. Measured.
    let chain = facts.objectives.as_ref().map(|o| {
        if facts.tier == Some(Tier::Elite) {
            (&o.elite, Tone::Elite)
        } else {
            (&o.pass, Tone::Pass)
        }
    });
    let objective = chain.and_then(|(rule, _)| line(rule));
    let rows = chain
        .and_then(|(rule, met)| {
            Some(ObjectiveRow {
                text: line(rule)?,
                state: if facts.tier.is_some() {
                    met
                } else {
                    Tone::Fail
                },
            })
        })
        .into_iter()
        .collect();
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
        xp,
    }
}
