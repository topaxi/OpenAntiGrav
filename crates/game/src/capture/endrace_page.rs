//! `--menu-page endrace-results`/`endrace-rewards`/`endrace-menu`: the three
//! EndRace screens, split out of `menu_page.rs` under the 1,000-line rule
//! (`scripts/check-file-size.py`) - a move, with `endrace_kind`/
//! `endrace_page` unchanged, plus [`capture`], which folds in the
//! archive-opening and faces/skin/globals setup `capture.rs`'s own call
//! site used to carry inline (the same shape [`campaign_page`]'s own call
//! site still has - this one moved rather than duplicated it, since this
//! file had the room `capture.rs` did not).

use anyhow::{Context, Result};

use crate::capture::menu_page::open_for_previews;

/// Which EndRace screen a `--menu-page` name asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum EndRaceKind {
    Results,
    Rewards,
    Menu,
}

#[must_use]
pub(super) fn endrace_kind(page: &str) -> Option<EndRaceKind> {
    match page {
        "endrace-results" | "endrace_results" => Some(EndRaceKind::Results),
        "endrace-rewards" | "endrace_rewards" => Some(EndRaceKind::Rewards),
        "endrace-menu" | "endrace_menu" => Some(EndRaceKind::Menu),
        _ => None,
    }
}

/// This project's own fixed 60 Hz tick - see
/// [ADR-0007](../../../../docs/architecture/adr/0007-fixed-timestep-vs-original.md).
const TICKS_PER_SECOND: f64 = 60.0;

/// Seconds to ticks, rounded to the nearest - the inverse of
/// `oag_ui::endrace::draw`'s own tick-to-`M.SS.CC` formatter, used only to
/// seed the synthetic content below from the reference capture's own
/// seconds figures. The two do not round-trip losslessly (a 60 Hz tick and
/// a centisecond do not divide evenly), so a drawn split may land one
/// centisecond off the reference frame's own digit - an accepted, documented
/// artifact of the conversion, not a drawing bug - see
/// `docs/ui/endrace-screens.md`.
fn seconds_to_ticks(seconds: f64) -> u32 {
    (seconds * TICKS_PER_SECOND).round() as u32
}

/// Opens `options.race`'s own source and draws one EndRace screen - the
/// setup [`campaign_page`]'s own caller in `capture.rs` keeps inline;
/// pulled into a function here rather than duplicated there, since this
/// file has the room `capture.rs` does not (`scripts/check-file-size.py`).
///
/// [`campaign_page`]: super::menu_page::campaign_page
#[allow(
    clippy::too_many_arguments,
    reason = "the same argument capture::run's own call site makes: each is a separate fact"
)]
pub(super) fn capture(
    kind: EndRaceKind,
    race: Option<&crate::race::Options>,
    menu_font: Option<&oag_ui::font::Atlas>,
    font: &oag_ui::font::Atlas,
    menu_skin: &'static oag_title::MenuSkin,
    space: oag_display::space::Space,
    frontend_globals: &std::collections::HashMap<String, String>,
    strings: &oag_ui::language::StringTable,
    backdrop: Option<oag_ui::menu::Picture>,
    frame: &oag_ui::menu::Frame,
    sprites: &mut crate::sprite::Sheet,
) -> Result<Vec<oag_ui::frontend::Draw>> {
    let mut archives = match race {
        Some(race) => open_for_previews(race)?,
        None => anyhow::bail!(
            "--menu-page endrace-results/endrace-rewards/endrace-menu needs --race options open"
        ),
    };
    let faces = oag_ui::picker::FaceScales {
        default: menu_font.map_or(oag_ui::picker::FaceScales::default().default, |menu| {
            font.line_height / menu.line_height
        }),
        ..oag_ui::picker::FaceScales::default()
    };
    let skin = oag_ui::menu::Skin::new(menu_skin, space, menu_font.unwrap_or(font).line_height);
    let globals: Vec<(&str, &str)> = frontend_globals
        .iter()
        .map(|(key, value)| (key.as_str(), value.as_str()))
        .collect();
    endrace_page(
        kind,
        &mut archives,
        strings,
        faces,
        [space.size.0, space.size.1],
        backdrop,
        &skin,
        frame,
        sprites,
        &globals,
    )
}

/// Draws one of the three EndRace screens, fed the one live capture this
/// project holds (`docs/ui/campaign-screens.md`'s "After a campaign race,
/// measured": `grid0_3_2`, Time Trial, 3 laps, no medal) rather than a real
/// race's own outcome - a `--menu-page` capture has no `Session`/`RaceStage`
/// behind it to read one from, the same reason `campaign_page` feeds
/// `GridSelection`/`CellSelection` the fresh-profile numbers rather than a
/// save. See `crate::main::session::endrace` for the live flow this is a
/// still of.
#[allow(
    clippy::too_many_arguments,
    reason = "the same argument campaign_page makes: each is a separate fact the page needs"
)]
fn endrace_page(
    kind: EndRaceKind,
    archives: &mut oag_assets::Archives,
    strings: &oag_ui::language::StringTable,
    faces: oag_ui::picker::FaceScales,
    grid: [f32; 2],
    backdrop: Option<oag_ui::menu::Picture>,
    skin: &oag_ui::menu::Skin,
    frame: &oag_ui::menu::Frame,
    sprites: &mut crate::sprite::Sheet,
    fallback_globals: &[(&str, &str)],
) -> Result<Vec<oag_ui::frontend::Draw>> {
    let screens = crate::endrace::load(archives, strings, faces, grid, sprites, fallback_globals)
        .context("this source has no EndRace screens to show")?;
    *sprites = screens.sprites;
    let layers = match kind {
        EndRaceKind::Results => {
            let model = oag_ui::endrace::Results {
                headline: oag_ui::endrace::Headline::TimeTrial,
                laps: vec![
                    oag_ui::endrace::LapSplit {
                        lap: 1,
                        ticks: seconds_to_ticks(92.49),
                    },
                    oag_ui::endrace::LapSplit {
                        lap: 2,
                        ticks: seconds_to_ticks(49.34),
                    },
                    oag_ui::endrace::LapSplit {
                        lap: 3,
                        ticks: seconds_to_ticks(49.93),
                    },
                ],
                total_ticks: u64::from(seconds_to_ticks(191.76)),
            };
            oag_ui::endrace::results_draw_list(
                &model,
                &screens.results,
                skin,
                frame,
                strings,
                backdrop,
                false,
                &|src| sprites.get(src),
            )
        }
        EndRaceKind::Rewards => {
            let model = oag_ui::endrace::Rewards {
                medal: None,
                campaign: true,
            };
            oag_ui::endrace::rewards_draw_list(
                &model,
                &screens.rewards,
                skin,
                frame,
                strings,
                backdrop,
                false,
                &|src| sprites.get(src),
            )
        }
        EndRaceKind::Menu => {
            let model = oag_ui::endrace::EndRaceMenu::new(
                vec![
                    oag_ui::endrace::MenuOption::ReturnToGrid,
                    oag_ui::endrace::MenuOption::RaceAgain,
                    oag_ui::endrace::MenuOption::ViewResultsAgain,
                ],
                Some(seconds_to_ticks(49.34)),
            );
            oag_ui::endrace::endrace_menu_draw_list(
                &model,
                &screens.menu,
                skin,
                frame,
                strings,
                backdrop,
                false,
                &|src| sprites.get(src),
            )
        }
    };
    Ok(layers.flatten())
}
