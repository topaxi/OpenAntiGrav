//! `--menu-page endrace-summary` / `endrace-objectives`: Wipeout 2048's two
//! race-ending pages ([`oag_ui_screens::endrace::touch`]).
//!
//! **With `--event NAME` the still is a real race**: the event loads, the
//! autopilot flies it to its end and the page says what it came to
//! ([`crate::endrace::touch::finished_event`]), so the verdict, the medal and
//! the objective rows are the engine's own. Without it the still is fed
//! **chosen** facts - a third place against a "third or better" pass and a
//! "win" elite - at the verdict the page name's suffix asks for
//! (`-pass`, `-elite`, `-fail`), which is how each of the four tones can be
//! looked at without racing to it.

use anyhow::{Context, Result};

use oag_2048::campaign::{EventKind, EventObjectives, ObjectiveRule, Tier, objective_type};
use oag_ui::frontend::Draw;
use oag_ui_screens::endrace::touch::{self, Button, Page};

use crate::endrace::touch::{Facts, Standing, finished_event, load, summary, xp_result};

/// What a `--menu-page` name asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct TouchPage {
    page: Page,
    /// Ticks the page has been up, from a `@SECONDS` suffix: the XP block counts up with it.
    ticks: u32,
    /// The profile's XP before the race banked, from a `+TOTAL` suffix.
    prior_xp: u32,
    /// The verdict a synthetic still shows; `None` asks for a real race.
    tier: Option<Option<Tier>>,
}

pub(super) fn touch_kind(page: &str) -> Option<TouchPage> {
    let page = page.replace('_', "-");
    let (page, prior) = match page.split_once('+') {
        Some((page, prior)) => (page, prior.parse().ok()?),
        None => (page.as_str(), 0),
    };
    let (page, seconds) = match page.split_once('@') {
        Some((page, seconds)) => (page, seconds.parse::<f32>().ok()?),
        None => (page, 0.0),
    };
    let (page, tail) = match page.strip_prefix("endrace-summary") {
        Some(tail) => (Page::Summary, tail),
        None => (Page::Objectives, page.strip_prefix("endrace-objectives")?),
    };
    let tier = match tail {
        "" => None,
        "-pass" => Some(Some(Tier::Pass)),
        "-elite" => Some(Some(Tier::Elite)),
        "-fail" => Some(None),
        _ => return None,
    };
    Some(TouchPage {
        page,
        ticks: (seconds * 60.0) as u32,
        prior_xp: prior,
        tier,
    })
}

/// The chosen facts of a synthetic still.
fn chosen(tier: Option<Tier>) -> Facts {
    let rule = |target| ObjectiveRule {
        objective_type: Some(objective_type::POSITION),
        target: Some(target),
    };
    Facts {
        mode: oag_race::Mode::SingleRace,
        standing: Standing::Place(if tier == Some(Tier::Elite) { 1 } else { 3 }),
        objectives: Some(EventObjectives {
            kind: EventKind::Race,
            pass: rule(3),
            elite: rule(1),
        }),
        tier,
    }
}

#[allow(
    clippy::too_many_arguments,
    reason = "each is a separate fact the page needs"
)]
pub(super) fn draw(
    kind: TouchPage,
    archives: &mut oag_assets::Archives,
    race: &oag_raceplay::Options,
    event: Option<&str>,
    title: &'static oag_title::Title,
    strings: &oag_ui::language::StringTable,
    grid: [f32; 2],
    base: &mut oag_hud::sprite::Sheet,
    globals: &[(&str, &str)],
    face_scales: &[(String, f32)],
    line: f32,
) -> Result<Vec<Draw>> {
    let entry = title
        .front_end
        .and_then(|front_end| front_end.endrace_entry)
        .context("this title names no EndRace definition")?;
    let screens = load(
        archives,
        entry,
        strings,
        oag_ui_screens::picker::FaceScales::default(),
        grid,
        base,
        globals,
    )?;
    let (facts, xp) = match (kind.tier, event) {
        (None, Some(name)) => {
            let finished = finished_event(race, name, 30_000, kind.prior_xp, None)?;
            (
                finished.facts,
                finished
                    .xp
                    .and_then(|(banked, ranks)| xp_result(banked, &ranks, strings)),
            )
        }
        (Some(tier), _) => (chosen(tier), None),
        (None, None) => anyhow::bail!(
            "--menu-page endrace-summary needs --event NAME to race, or one of \
             endrace-summary-pass / -elite / -fail for a chosen result"
        ),
    };
    let model = summary(&facts, strings, xp);
    *base = screens.sprites;
    let sprites = &*base;
    Ok(touch::draw_list(
        &model,
        kind.page,
        kind.ticks,
        Button::Exit,
        &screens.layouts,
        &|src| sprites.get(src),
        face_scales,
        line,
    ))
}
