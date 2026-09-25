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
    /// `tournament_next_leg` swaps `RACE AGAIN` for `ER_NEXT_RACE` - see
    /// `crate::race_stage::endrace::menu_options`'s own doc for when this
    /// engine actually does that. A capture-only knob: nothing about the
    /// row's own drawing changes, only which option this synthetic model
    /// picks, so a Tournament leg's own `EndRace Menu` can be looked at
    /// without driving a real tournament leg to a finish.
    Menu {
        tournament_next_leg: bool,
    },
}

#[must_use]
pub(super) fn endrace_kind(page: &str) -> Option<EndRaceKind> {
    match page {
        "endrace-results" | "endrace_results" => Some(EndRaceKind::Results),
        "endrace-rewards" | "endrace_rewards" => Some(EndRaceKind::Rewards),
        "endrace-menu" | "endrace_menu" => Some(EndRaceKind::Menu {
            tournament_next_leg: false,
        }),
        "endrace-menu-tournament" | "endrace_menu_tournament" => Some(EndRaceKind::Menu {
            tournament_next_leg: true,
        }),
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
    title: &'static oag_title::Title,
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
        title,
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
    title: &'static oag_title::Title,
) -> Result<Vec<oag_ui::frontend::Draw>> {
    let screens = crate::endrace::load(
        archives,
        strings,
        faces,
        grid,
        sprites,
        fallback_globals,
        title,
    )
    .context("this source has no EndRace screens to show")?;
    *sprites = screens.sprites;
    if title.name == oag_hd::TITLE.name {
        return hd_endrace_page(
            kind,
            &screens.results,
            screens.rewards.as_ref(),
            &screens.menu,
            strings,
            skin,
            frame,
            backdrop,
            sprites,
        );
    }
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
                // The reference capture's own numbers
                // (`docs/ui/campaign-screens.md`'s "After a campaign race,
                // measured"): a fresh profile's first race,
                // `Race_ComputeLoyaltyAward`'s `3 laps * 30 = 90`, matching
                // `"Assegai Loyalty: 90 Points"` / `"Total loyalty: 90"`.
                loyalty: Some(oag_ui::endrace::Loyalty {
                    team_name: "Assegai".to_string(),
                    award: 90,
                    total: 90,
                }),
            };
            // `screens.rewards` is always `Some` on this arm: the early
            // `hd_endrace_page` return above is the only path that ever
            // leaves it `None` - see `EndRaceScreens::rewards`'s own doc.
            let rewards = screens
                .rewards
                .as_ref()
                .context("this title's EndRace Rewards is not read by this build")?;
            oag_ui::endrace::rewards_draw_list(
                &model,
                rewards,
                skin,
                frame,
                strings,
                backdrop,
                false,
                &|src| sprites.get(src),
            )
        }
        EndRaceKind::Menu {
            tournament_next_leg,
        } => {
            // The same list `crate::race_stage::endrace::menu_options`
            // builds in the binary crate this library cannot reach from
            // here - reproduced rather than shared, since a `--menu-page`
            // capture has no `Session` behind it either. See that
            // function's own doc for the redirect this mirrors.
            let second_row = if tournament_next_leg {
                oag_ui::endrace::MenuOption::NextRace
            } else {
                oag_ui::endrace::MenuOption::RaceAgain
            };
            let model = oag_ui::endrace::EndRaceMenu::new(
                vec![
                    oag_ui::endrace::MenuOption::ReturnToGrid,
                    second_row,
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

/// [`endrace_page`]'s Wipeout HD/Fury branch: `EndRace Results`/`EndRace
/// Rewards`/`EndRace Menu` - see `crate::endrace::load_hd`'s own doc for why
/// `EndRace Podium` is out of scope, and [`oag_ui::endrace::hd`] for what
/// the three implemented screens draw. `endrace-rewards` is the only way to
/// see HD's Rewards at all: the original never enters it, so the live flow
/// does not either (`docs/formats/hd-endrace-screens.md`). Fed a
/// synthetic two-craft field: this project holds no live HD capture to seed
/// real numbers from, the same gap [`endrace_page`]'s own doc names for
/// Pulse - except there this build has at least a reference frame to match
/// against, and here it does not.
///
/// # Errors
///
/// `--menu-page endrace-rewards` on an HD copy that authors no `EndRace
/// Rewards` (`DATA06`'s).
#[allow(
    clippy::too_many_arguments,
    reason = "the same argument endrace_page's own call sites make: each is a separate fact"
)]
fn hd_endrace_page(
    kind: EndRaceKind,
    results: &oag_ui::endrace::Layout,
    rewards: Option<&oag_ui::endrace::Layout>,
    menu: &oag_ui::endrace::Layout,
    strings: &oag_ui::language::StringTable,
    skin: &oag_ui::menu::Skin,
    frame: &oag_ui::menu::Frame,
    backdrop: Option<oag_ui::menu::Picture>,
    sprites: &mut crate::sprite::Sheet,
) -> Result<Vec<oag_ui::frontend::Draw>> {
    let layers = match kind {
        EndRaceKind::Results => {
            let model = oag_ui::endrace::FieldResults {
                headline: oag_ui::endrace::Headline::Position(1),
                rows: vec![
                    oag_ui::endrace::FieldRow {
                        place: 1,
                        time_ticks: Some(u64::from(seconds_to_ticks(191.76))),
                        player: true,
                    },
                    oag_ui::endrace::FieldRow {
                        place: 2,
                        time_ticks: Some(u64::from(seconds_to_ticks(203.11))),
                        player: false,
                    },
                ],
            };
            oag_ui::endrace::hd::hd_results_draw_list(
                &model,
                results,
                skin,
                frame,
                strings,
                backdrop,
                false,
                &|src| sprites.get(src),
            )
        }
        EndRaceKind::Rewards => {
            // The same synthetic race the Results arm above draws: the
            // player first, on a campaign cell whose law gave gold for it.
            let model = oag_ui::endrace::HdRewards {
                place: Some(1),
                medal: Some(oag_tables::race_campaign::Medal::Gold),
                campaign: true,
            };
            let rewards = rewards.context(
                "the served copy of this title's screen file authors no EndRace Rewards",
            )?;
            oag_ui::endrace::hd::hd_rewards_draw_list(
                &model,
                rewards,
                skin,
                frame,
                strings,
                backdrop,
                false,
                &|src| sprites.get(src),
            )
        }
        // Tournament is Pulse-only in this build so far - `tournament_next_leg`
        // never reaches this arm true, but the field still has to be bound.
        EndRaceKind::Menu {
            tournament_next_leg: _,
        } => {
            let model = oag_ui::endrace::EndRaceMenu::new(
                vec![
                    oag_ui::endrace::MenuOption::ReturnToGrid,
                    oag_ui::endrace::MenuOption::RaceAgain,
                    oag_ui::endrace::MenuOption::ViewResultsAgain,
                ],
                Some(seconds_to_ticks(49.34)),
            );
            oag_ui::endrace::hd::hd_menu_draw_list(
                &model,
                menu,
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
