//! `--menu-page endrace-results`/`endrace-rewards`/`endrace-menu`: the three
//! EndRace screens, split out of `menu_page.rs` under the 1,000-line rule
//! (`scripts/check-file-size.py`) - a move, with `endrace_kind`/
//! `endrace_page` unchanged, plus [`capture`], which folds in the
//! archive-opening and faces/skin/globals setup `capture.rs`'s own call
//! site used to carry inline (the same shape [`campaign_page`]'s own call
//! site still has - this one moved rather than duplicated it, since this
//! file had the room `capture.rs` did not).

use anyhow::{Context, Result};

use crate::capture::preview_pass::{PreviewRequest, open_for_previews};
use oag_tables::race_campaign::Medal;

/// Which EndRace screen a `--menu-page` name asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum EndRaceKind {
    Results,
    /// `medal` is the campaign cell's award the synthetic model carries:
    /// `None` is the one reference capture's no-medal run, `Some` draws the
    /// matching `TrophyPanel` model (`endrace-rewards-gold`/`-silver`/
    /// `-bronze`) - a capture-only knob, so an earned trophy can be looked
    /// at without racing to one.
    Rewards {
        medal: Option<Medal>,
    },
    /// `tournament_next_leg` swaps `RACE AGAIN` for `ER_NEXT_RACE` - see
    /// `crate::race_stage::endrace::menu_options`'s own doc for when this
    /// engine actually does that. A capture-only knob: nothing about the
    /// row's own drawing changes, only which option this synthetic model
    /// picks, so a Tournament leg's own `EndRace Menu` can be looked at
    /// without driving a real tournament leg to a finish.
    Menu {
        tournament_next_leg: bool,
    },
    /// A Tournament leg's own `EndRace Results` - the table
    /// `EndRaceResults_PopulateTournamentTable` (`0x088dad90`) draws in
    /// place of the ordinary per-lap table, cycling every three seconds
    /// between this leg's own placings and the running standings. `true`
    /// picks the standings page (`ER_TOUR_STAN`), `false` the leg page
    /// (`ER_RACE_STAN`) - see [`oag_ui_screens::endrace::TournamentResults`]'s own
    /// doc. A capture-only knob, the same reason `Menu`'s own
    /// `tournament_next_leg` is one: driving a real tournament leg to a
    /// finish is not (`docs/ghidra/functions/psp-pulse-usa/tournament.md`'s
    /// own "live verification" names the autopilot cost this would take).
    TournamentResults {
        standings: bool,
    },
    /// An Eliminator race's `EndRace Results` - `EndRaceResults_PopulateEliminationTable`
    /// (`0x088db1ec`). Fed the eight records a live PPSSPP race left in
    /// `g_endrace_result` (2026-09-30), so the still can be laid beside the frame
    /// it was taken from; only the team ids are this project's own spelling.
    EliminationResults,
    /// Wipeout HD/Fury's `EndRace Podium` - **a still only**, the live flow
    /// never enters it (untraced entry). Synthetic three-craft top three;
    /// the pilot names are this capture's own, not the disc's.
    Podium,
    /// A Zone race's `EndRace Results` - `EndRaceResults_PopulateZoneTable`
    /// (`0x088db574`). **Chosen numbers**, not measured: no Zone frame is
    /// reachable, and two of the six statistics draw blank on purpose, as this
    /// build's own race would leave them.
    ZoneResults,
    /// Wipeout 2048's `RaceSummary`/`ObjectiveSummary` - see
    /// [`super::endrace_touch_page`].
    Touch(super::endrace_touch_page::TouchPage),
}

#[must_use]
pub(super) fn endrace_kind(page: &str) -> Option<EndRaceKind> {
    match page {
        "endrace-results" | "endrace_results" => Some(EndRaceKind::Results),
        "endrace-rewards" | "endrace_rewards" => Some(EndRaceKind::Rewards { medal: None }),
        "endrace-rewards-gold" => Some(EndRaceKind::Rewards {
            medal: Some(Medal::Gold),
        }),
        "endrace-rewards-silver" => Some(EndRaceKind::Rewards {
            medal: Some(Medal::Silver),
        }),
        "endrace-rewards-bronze" => Some(EndRaceKind::Rewards {
            medal: Some(Medal::Bronze),
        }),
        "endrace-podium" | "endrace_podium" => Some(EndRaceKind::Podium),
        "endrace-menu" | "endrace_menu" => Some(EndRaceKind::Menu {
            tournament_next_leg: false,
        }),
        "endrace-menu-tournament" | "endrace_menu_tournament" => Some(EndRaceKind::Menu {
            tournament_next_leg: true,
        }),
        "endrace-results-tournament-leg" | "endrace_results_tournament_leg" => {
            Some(EndRaceKind::TournamentResults { standings: false })
        }
        "endrace-results-tournament-standings" | "endrace_results_tournament_standings" => {
            Some(EndRaceKind::TournamentResults { standings: true })
        }
        "endrace-results-eliminator" | "endrace_results_eliminator" => {
            Some(EndRaceKind::EliminationResults)
        }
        "endrace-results-zone" | "endrace_results_zone" => Some(EndRaceKind::ZoneResults),
        other => super::endrace_touch_page::touch_kind(other).map(EndRaceKind::Touch),
    }
}

/// This project's own fixed 60 Hz tick - see
/// [ADR-0007](../../../../docs/architecture/adr/0007-fixed-timestep-vs-original.md).
const TICKS_PER_SECOND: f64 = 60.0;

/// Seconds to ticks, rounded to the nearest - the inverse of
/// `oag_ui_screens::endrace::draw`'s own tick-to-`M.SS.CC` formatter, used only to
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
    race: Option<&oag_raceplay::Options>,
    menu_font: Option<&oag_ui::font::Atlas>,
    font: &oag_ui::font::Atlas,
    menu_skin: &'static oag_title::MenuSkin,
    space: oag_display::space::Space,
    frontend_globals: &std::collections::HashMap<String, String>,
    strings: &oag_ui::language::StringTable,
    backdrop: Option<oag_ui::menu::Picture>,
    frame: &oag_ui::menu::Frame,
    sprites: &mut oag_hud::sprite::Sheet,
    title: &'static oag_title::Title,
    (face_scales, event): (&[(String, f32)], Option<&str>),
) -> Result<(Vec<oag_ui::frontend::Draw>, Option<PreviewRequest>)> {
    let mut archives = match race {
        Some(race) => open_for_previews(race)?,
        None => anyhow::bail!(
            "--menu-page endrace-results/endrace-rewards/endrace-menu needs --race options open"
        ),
    };
    let faces = oag_ui_screens::picker::FaceScales {
        default: menu_font.map_or(
            oag_ui_screens::picker::FaceScales::default().default,
            |menu| font.line_height / menu.line_height,
        ),
        ..oag_ui_screens::picker::FaceScales::default()
    };
    let skin = oag_ui::menu::Skin::new(menu_skin, space, menu_font.unwrap_or(font).line_height);
    let globals: Vec<(&str, &str)> = frontend_globals
        .iter()
        .map(|(key, value)| (key.as_str(), value.as_str()))
        .collect();
    if let EndRaceKind::Touch(page) = kind {
        let list = super::endrace_touch_page::draw(
            page,
            &mut archives,
            race.context("a touch EndRace page needs --race options open")?,
            event,
            title,
            strings,
            [space.size.0, space.size.1],
            sprites,
            &globals,
            face_scales,
            font.line_height,
        )?;
        return Ok((list, None));
    }
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
    faces: oag_ui_screens::picker::FaceScales,
    grid: [f32; 2],
    backdrop: Option<oag_ui::menu::Picture>,
    skin: &oag_ui::menu::Skin,
    frame: &oag_ui::menu::Frame,
    sprites: &mut oag_hud::sprite::Sheet,
    fallback_globals: &[(&str, &str)],
    title: &'static oag_title::Title,
) -> Result<(Vec<oag_ui::frontend::Draw>, Option<PreviewRequest>)> {
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
    if crate::endrace::dialect(title) == Some(oag_title::EndRaceDialect::Field) {
        return hd_endrace_page(
            kind,
            &screens.results,
            screens.rewards.as_ref(),
            screens.podium.as_ref(),
            &screens.menu,
            strings,
            skin,
            frame,
            backdrop,
            sprites,
        )
        .map(|list| (list, None));
    }
    let mut trophy = None;
    let layers = match kind {
        EndRaceKind::Touch(_) => anyhow::bail!("a touch page is drawn before this point"),
        EndRaceKind::Results => {
            // The reference capture's own three laps and their third column
            // (`6`/`9`/`10`, total `25`): `results-01.png`.
            let split = |lap, seconds, boosts| oag_ui_screens::endrace::LapSplit {
                lap,
                ticks: seconds_to_ticks(seconds),
                boosts: Some(boosts),
            };
            let model = oag_ui_screens::endrace::Results {
                headline: oag_ui_screens::endrace::Headline::TimeTrial,
                laps: vec![split(1, 92.49, 6), split(2, 49.34, 9), split(3, 49.93, 10)],
                total_ticks: u64::from(seconds_to_ticks(191.76)),
            };
            oag_ui_screens::endrace::results_draw_list(
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
        EndRaceKind::Rewards { medal } => {
            // The trophy is a 3D pass after the draw list, the one the
            // live `EndRaceRuntime::draw` makes - see
            // `crate::endrace::Trophy`.
            trophy = medal
                .and_then(|medal| screens.trophies.iter().find(|t| t.medal == medal))
                .map(|trophy| PreviewRequest {
                    hull_only: false,
                    ship_model: None,
                    entry: trophy.placement.model.src.clone(),
                    fallback: None,
                    skin: None,
                    rect: [0.0, 0.0, grid[0], grid[1]],
                    kind: oag_ui_screens::picker::Kind::Ship,
                    mode3d: Some(trophy.placement.model.clone()),
                    track_model: None,
                    seconds: 0.0,
                    hull_clock: (0.0, 0.0),
                });
            let model = oag_ui_screens::endrace::Rewards {
                medal,
                campaign: true,
                // The reference capture's own numbers
                // (`docs/ui/campaign-screens.md`'s "After a campaign race,
                // measured"): a fresh profile's first race,
                // `Race_ComputeLoyaltyAward`'s `3 laps * 30 = 90`, matching
                // `"Assegai Loyalty: 90 Points"` / `"Total loyalty: 90"`.
                loyalty: Some(oag_ui_screens::endrace::Loyalty {
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
            oag_ui_screens::endrace::rewards_draw_list(
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
        EndRaceKind::TournamentResults { standings } => {
            // A synthetic four-craft field, points consistent with
            // `oag_race::tournament::POINTS_BY_POSITION` (8/6/5/4/...): the
            // leg page shows this leg's own finish order and points, the
            // standings page a plausible running total after an earlier
            // leg, ranked descending - the same shape
            // `EndRaceResults_PopulateTournamentTable` draws, with numbers
            // chosen for legibility rather than measured (no live capture
            // of this screen exists yet - see the module's own doc).
            let row =
                |team: &str, points: u32, player: bool| oag_ui_screens::endrace::TournamentRow {
                    team_name: Some(team.to_string()),
                    points,
                    player,
                };
            let mut model = oag_ui_screens::endrace::TournamentResults::new(
                false,
                2,
                3,
                vec![
                    row("Assegai", 8, true),
                    row("Feisar", 6, false),
                    row("Qirex", 5, false),
                    row("AG-Systems", 4, false),
                ],
                // Leg 1 was Feisar, Qirex, Assegai, AG-Systems (8/6/5/4).
                vec![
                    row("Feisar", 14, false),
                    row("Assegai", 13, true),
                    row("Qirex", 11, false),
                    row("AG-Systems", 8, false),
                ],
            );
            if standings {
                // `TournamentResults` starts on the leg page - one tick
                // past its own 3-second threshold flips it, the same
                // `EndRaceResults_Update` toggle `tick` reimplements.
                model.tick(3.1);
            }
            oag_ui_screens::endrace::tournament_results_draw_list(
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
        EndRaceKind::EliminationResults => {
            // The live race's eight records (slot order is the grid's, the ranking
            // is `EliminationResults::new`'s): the player last with 0 kills.
            let craft = |team: &str, kills: u32, deaths: u32, player: bool| {
                oag_ui_screens::endrace::EliminationRow {
                    team_name: Some(team.to_string()),
                    kills,
                    deaths,
                    player,
                }
            };
            let model = oag_ui_screens::endrace::EliminationResults::new(vec![
                craft("Assegai", 0, 1, true),
                craft("Feisar", 2, 4, false),
                craft("EGX", 5, 5, false),
                craft("Qirex", 3, 3, false),
                craft("Goteki", 3, 1, false),
                craft("AG_Systems", 2, 4, false),
                craft("Triakis", 1, 2, false),
                craft("Piranha", 4, 1, false),
            ]);
            oag_ui_screens::endrace::elimination_results_draw_list(
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
        EndRaceKind::Podium => anyhow::bail!(
            "EndRace Podium is Wipeout HD/Fury's only; this title's screen file has none"
        ),
        EndRaceKind::ZoneResults => {
            let model = oag_ui_screens::endrace::ZoneResults {
                zones_cleared: 7,
                perfect_zones: Some(3),
                laps_cleared: None,
                perfect_laps: None,
                top_speed_kmh: Some(812),
                score: 4321,
            };
            oag_ui_screens::endrace::zone_results_draw_list(
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
        EndRaceKind::Menu {
            tournament_next_leg,
        } => {
            // The same list `crate::race_stage::endrace::menu_options`
            // builds in the binary crate this library cannot reach from
            // here - reproduced rather than shared, since a `--menu-page`
            // capture has no `Session` behind it either. See that
            // function's own doc for the redirect this mirrors.
            let second_row = if tournament_next_leg {
                oag_ui_screens::endrace::MenuOption::NextRace
            } else {
                oag_ui_screens::endrace::MenuOption::RaceAgain
            };
            let model = oag_ui_screens::endrace::EndRaceMenu::new(
                vec![
                    oag_ui_screens::endrace::MenuOption::ReturnToGrid,
                    second_row,
                    oag_ui_screens::endrace::MenuOption::ViewResultsAgain,
                ],
                Some(seconds_to_ticks(49.34)),
            );
            oag_ui_screens::endrace::endrace_menu_draw_list(
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
    Ok((layers.flatten(), trophy))
}

/// [`endrace_page`]'s Wipeout HD/Fury branch: `EndRace Results`/`EndRace
/// Rewards`/`EndRace Menu` - see `crate::endrace::load_hd`'s own doc for why
/// `EndRace Podium` is out of scope, and [`oag_ui_screens::endrace::hd`] for what
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
    results: &oag_ui_screens::endrace::Layout,
    rewards: Option<&oag_ui_screens::endrace::Layout>,
    podium: Option<&oag_ui_screens::endrace::Layout>,
    menu: &oag_ui_screens::endrace::Layout,
    strings: &oag_ui::language::StringTable,
    skin: &oag_ui::menu::Skin,
    frame: &oag_ui::menu::Frame,
    backdrop: Option<oag_ui::menu::Picture>,
    sprites: &mut oag_hud::sprite::Sheet,
) -> Result<Vec<oag_ui::frontend::Draw>> {
    let layers = match kind {
        EndRaceKind::Touch(_) => anyhow::bail!("a touch page is drawn before this point"),
        EndRaceKind::Results => {
            let model = oag_ui_screens::endrace::FieldResults {
                // **Chosen, not measured**: a capture has no race behind it, so
                // these are `oag_hd::loyalty`'s own three-lap single race on
                // the medium rung (`3 * 15 * 3`) banked onto a team that
                // already held 3885.
                loyalty: Some(oag_ui_screens::endrace::HdLoyalty {
                    award: 135,
                    total: 4_020,
                }),
                headline: oag_ui_screens::endrace::Headline::Position(1),
                rows: vec![
                    oag_ui_screens::endrace::FieldRow {
                        place: 1,
                        time_ticks: Some(u64::from(seconds_to_ticks(191.76))),
                        player: true,
                    },
                    oag_ui_screens::endrace::FieldRow {
                        place: 2,
                        time_ticks: Some(u64::from(seconds_to_ticks(203.11))),
                        player: false,
                    },
                ],
            };
            oag_ui_screens::endrace::hd::hd_results_draw_list(
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
        EndRaceKind::Podium => {
            let model = oag_ui_screens::endrace::hd::HdPodium {
                places: [
                    Some(oag_ui_screens::endrace::hd::PodiumSlot {
                        name: "PILOT ONE".to_string(),
                        player: true,
                    }),
                    Some(oag_ui_screens::endrace::hd::PodiumSlot {
                        name: "PILOT TWO".to_string(),
                        player: false,
                    }),
                    Some(oag_ui_screens::endrace::hd::PodiumSlot {
                        name: "PILOT THREE".to_string(),
                        player: false,
                    }),
                ],
            };
            let podium =
                podium.context("no copy of this title's screen file authors an EndRace Podium")?;
            oag_ui_screens::endrace::hd::hd_podium_draw_list(
                &model,
                podium,
                skin,
                frame,
                strings,
                backdrop,
                false,
                &|src| sprites.get(src),
            )
        }
        EndRaceKind::Rewards { .. } => {
            // The same synthetic race the Results arm above draws: the
            // player first, on a campaign cell whose law gave gold for it.
            let model = oag_ui_screens::endrace::HdRewards {
                place: Some(1),
                medal: Some(oag_tables::race_campaign::Medal::Gold),
                campaign: true,
            };
            let rewards = rewards.context(
                "the served copy of this title's screen file authors no EndRace Rewards",
            )?;
            oag_ui_screens::endrace::hd::hd_rewards_draw_list(
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
        // Tournament's own standings table is Pulse's own `EndRace_Definition.xml`
        // (`docs/formats/endrace-screens.md`) - HD's own copy
        // (`docs/formats/hd-endrace-screens.md`) authors no equivalent, and
        // this build implements no HD Tournament mode to draw one for
        // anyway.
        EndRaceKind::TournamentResults { standings: _ } => {
            anyhow::bail!(
                "Wipeout HD/Fury's own EndRace Results authors no Tournament standings table"
            )
        }
        // Zone's and Eliminator's tables are Pulse's own populate functions too.
        EndRaceKind::EliminationResults | EndRaceKind::ZoneResults => {
            anyhow::bail!(
                "Wipeout HD/Fury's own EndRace Results authors no Zone or Eliminator table"
            )
        }
        // Tournament is Pulse-only in this build so far - `tournament_next_leg`
        // never reaches this arm true, but the field still has to be bound.
        EndRaceKind::Menu {
            tournament_next_leg: _,
        } => {
            let model = oag_ui_screens::endrace::EndRaceMenu::new(
                vec![
                    oag_ui_screens::endrace::MenuOption::ReturnToGrid,
                    oag_ui_screens::endrace::MenuOption::RaceAgain,
                    oag_ui_screens::endrace::MenuOption::ViewResultsAgain,
                ],
                Some(seconds_to_ticks(49.34)),
            );
            // Two whole blink periods: the focused block has eased to its
            // full width and its arrow is on its lit phase, so a still
            // shows the cursor a player sees most of the time rather than
            // the first frame's narrow box. The tick count is this
            // capture's choice; the ease and blink are `Block_Update`'s.
            let mut model = oag_ui_screens::endrace::hd::hd_screen_order(&model, menu);
            for _ in 0..34 {
                model.tick();
            }
            oag_ui_screens::endrace::hd::hd_menu_draw_list(
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
