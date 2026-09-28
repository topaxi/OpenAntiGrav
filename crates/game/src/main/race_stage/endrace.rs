//! The EndRace flow a finished race holds open: `EndRace Results` ->
//! (`EndRace Rewards`, campaign only) -> `EndRace Menu`. See
//! `oag_ui::endrace` for the model and the drawing, `oag_game::endrace` for
//! the disc read, and `crate::main::session::endrace` for the flow that
//! drives one - the same split `crate::campaign_stage`/
//! `crate::main::session::campaign` make for the Race Campaign's own
//! screens.
//!
//! **Drawn inside `Stage::Race`, not `Stage::Menu`.** A finished race's own
//! scene is already left exactly as the finishing tick left it -
//! `RaceStage::draw_hud`'s own doc, unchanged by this - so the "screens over
//! a frozen race" picture the reference frames show falls out for free from
//! the world simply not being ticked any more, the same mechanism
//! `oag_game::scoreboard::Overlay` already draws its own table over. This
//! stays there rather than moving to `MenuStage`'s own `frozen_race`
//! compositing, which holds a *captured* backdrop texture for a race left
//! behind entirely - a different mechanism for a picture this one already
//! has. **Chosen**, not the only shape this could have taken.

use anyhow::Result;

use oag_tables::race_campaign::Medal;
use oag_ui::endrace::{
    EndRaceMenu, FieldResults, Headline, MenuOption, Results, Rewards, TournamentResults,
    TournamentRow,
};

use oag_game::render::Renderer;

/// Which of the three screens is on top.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Which {
    Results,
    Rewards,
    Menu,
}

/// `EndRace Results`' own model, title-dispatched: Pulse's per-lap table, or
/// Wipeout HD/Fury's whole-field standings grid
/// ([`oag_ui::endrace::hd`]). Carrying both behind one enum, rather than an
/// `Option` of each, is what keeps [`EndRaceRuntime::draw`] from being able
/// to hold a Pulse model against an HD screen layout (or the reverse) -
/// exactly the mismatch `oag_game::endrace::load`'s own title dispatch
/// already prevents on the read side.
#[derive(Debug)]
pub(crate) enum ResultsModel {
    Pulse(Results),
    /// A Tournament leg's own results - Pulse only, off
    /// `EndRaceResults_OnEnter`'s `case 4: case 0x10:` block, cycling every
    /// three seconds between this leg's own placings and the running
    /// standings. See [`oag_ui::endrace::TournamentResults`].
    PulseTournament(TournamentResults),
    Hd(FieldResults),
}

/// The screens' own content, built once when the race finishes, and which
/// one is current. `rewards` is `None` on Wipeout HD/Fury, whose original
/// never enters its own `EndRace Rewards` (`docs/formats/hd-endrace-screens.md`)
/// even though the layout is read; [`Which::Rewards`] is then simply never
/// reached, the same way [`Which::Menu`]'s own
/// `Endrace Difficulty` list is authored but never driven.
pub(crate) struct EndRaceRuntime {
    screens: oag_game::endrace::EndRaceScreens,
    skin: oag_ui::menu::Skin,
    frame: oag_ui::menu::Frame,
    strings: oag_ui::language::StringTable,
    renderer: Renderer,
    results: ResultsModel,
    rewards: Option<Rewards>,
    menu: EndRaceMenu,
    which: Which,
}

impl std::fmt::Debug for EndRaceRuntime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EndRaceRuntime")
            .field("which", &self.which)
            .finish()
    }
}

impl EndRaceRuntime {
    /// # Errors
    ///
    /// Propagates [`crate::render::Renderer::new`]'s own pipeline-build
    /// errors.
    #[allow(
        clippy::too_many_arguments,
        reason = "one build site, each a separate fact"
    )]
    pub(crate) fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        screens: oag_game::endrace::EndRaceScreens,
        skin: oag_ui::menu::Skin,
        frame: oag_ui::menu::Frame,
        strings: oag_ui::language::StringTable,
        atlas: oag_ui::font::Atlas,
        results: ResultsModel,
        rewards: Option<Rewards>,
        menu: EndRaceMenu,
    ) -> Result<Self> {
        let mut renderer = Renderer::new(device, queue, format, None, atlas, &screens.sprites)?;
        renderer.set_space(skin.space());
        Ok(Self {
            screens,
            skin,
            frame,
            strings,
            renderer,
            results,
            rewards,
            menu,
            which: Which::Results,
        })
    }

    /// Advances to the next screen on a `Confirmed` event - `Results` goes to
    /// `Rewards` when a campaign cell was in play (the disc's own
    /// `EndRaceRewardsRedirect` branch), straight to `Menu` otherwise (the
    /// `EndRaceMenuRedirect` branch); `Rewards` always goes to `Menu`.
    /// `Menu` does not advance here - a row's own action is
    /// `crate::main::session::endrace`'s job.
    pub(crate) fn advance(&mut self) {
        self.which = match self.which {
            Which::Results
                if self
                    .rewards
                    .as_ref()
                    .is_some_and(|rewards| rewards.campaign) =>
            {
                Which::Rewards
            }
            Which::Results | Which::Rewards => Which::Menu,
            Which::Menu => Which::Menu,
        };
    }

    /// `VIEW RESULTS AGAIN` - back to `Results`, the identical
    /// `<Entry item="Endrace Options" equals="ER_VIEW_AGAIN" goto="EndRace Results">`
    /// redirect the disc authors.
    pub(crate) fn view_results_again(&mut self) {
        self.which = Which::Results;
    }

    #[must_use]
    pub(crate) fn menu(&self) -> &EndRaceMenu {
        &self.menu
    }

    #[must_use]
    pub(crate) fn menu_mut(&mut self) -> &mut EndRaceMenu {
        &mut self.menu
    }

    /// `EndRace Menu`'s own row targets for a pointer tick, title-dispatched
    /// the same way [`Self::draw`] is: Pulse's `<Menu>` list stride
    /// (`oag_ui::endrace::pointer::menu_targets`), or Wipeout HD/Fury's own
    /// per-`<Block>` positions (`oag_ui::endrace::hd::hd_menu_targets`).
    /// Kept here rather than at the call site so a caller never has to know
    /// which title it is driving - the same seam [`Self::draw`] already
    /// draws for it.
    #[must_use]
    pub(crate) fn menu_targets(&self) -> Vec<oag_ui::endrace::pointer::Target> {
        match &self.results {
            ResultsModel::Pulse(_) | ResultsModel::PulseTournament(_) => {
                oag_ui::endrace::pointer::menu_targets(&self.menu, &self.screens.menu)
            }
            ResultsModel::Hd(_) => {
                oag_ui::endrace::hd::hd_menu_targets(&self.menu, &self.screens.menu)
            }
        }
    }

    /// One tick of the Tournament leg/standings toggle -
    /// [`TournamentResults::tick`], a no-op unless [`Which::Results`] is
    /// current and [`ResultsModel::PulseTournament`] is the model, driven
    /// once a tick from `Session::tick_endrace` the same way
    /// `EndRaceResults_Update`'s own per-frame toggle runs only while that
    /// screen is on top.
    pub(crate) fn tick_tournament_table(&mut self) {
        if self.which != Which::Results {
            return;
        }
        if let ResultsModel::PulseTournament(results) = &mut self.results {
            results.tick(1.0 / 60.0);
        }
    }

    /// The grid these screens are authored in - what
    /// `crate::main::session::pointer::in_grid` resolves a window-pixel
    /// pointer against before `Session::tick_endrace` reads it.
    #[must_use]
    pub(crate) fn space(&self) -> oag_display::space::Space {
        self.skin.space()
    }

    #[must_use]
    pub(crate) fn is_menu(&self) -> bool {
        self.which == Which::Menu
    }

    /// Draws whichever screen is current, over the race's own already-drawn
    /// frame.
    pub(crate) fn draw(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        viewport: (f32, f32, f32, f32),
    ) {
        let sprites = &self.screens.sprites;
        let layers = match (self.which, &self.results) {
            (Which::Results, ResultsModel::Pulse(results)) => oag_ui::endrace::results_draw_list(
                results,
                &self.screens.results,
                &self.skin,
                &self.frame,
                &self.strings,
                None,
                true,
                &|src| sprites.get(src),
            ),
            (Which::Results, ResultsModel::PulseTournament(results)) => {
                oag_ui::endrace::tournament_results_draw_list(
                    results,
                    &self.screens.results,
                    &self.skin,
                    &self.frame,
                    &self.strings,
                    None,
                    true,
                    &|src| sprites.get(src),
                )
            }
            (Which::Results, ResultsModel::Hd(results)) => {
                oag_ui::endrace::hd::hd_results_draw_list(
                    results,
                    &self.screens.results,
                    &self.skin,
                    &self.frame,
                    &self.strings,
                    None,
                    true,
                    &|src| sprites.get(src),
                )
            }
            // `Which::Rewards` is only ever entered from `advance` when
            // `self.rewards` is `Some` and `campaign` - see that function's
            // own doc, and `EndRaceScreens::rewards`'s for why that is
            // always true on the title this arm ever actually reaches
            // (Pulse; HD never builds a `Rewards` at all).
            (Which::Rewards, _) => {
                let Some(rewards) = &self.rewards else {
                    return;
                };
                let Some(layout) = &self.screens.rewards else {
                    return;
                };
                oag_ui::endrace::rewards_draw_list(
                    rewards,
                    layout,
                    &self.skin,
                    &self.frame,
                    &self.strings,
                    None,
                    true,
                    &|src| sprites.get(src),
                )
            }
            (Which::Menu, ResultsModel::Pulse(_) | ResultsModel::PulseTournament(_)) => {
                oag_ui::endrace::endrace_menu_draw_list(
                    &self.menu,
                    &self.screens.menu,
                    &self.skin,
                    &self.frame,
                    &self.strings,
                    None,
                    true,
                    &|src| sprites.get(src),
                )
            }
            (Which::Menu, ResultsModel::Hd(_)) => oag_ui::endrace::hd::hd_menu_draw_list(
                &self.menu,
                &self.screens.menu,
                &self.skin,
                &self.frame,
                &self.strings,
                None,
                true,
                &|src| sprites.get(src),
            ),
        };
        self.renderer
            .overlay(device, queue, encoder, view, &layers.flatten(), viewport);
    }
}

/// `Line1`'s own headline, off this run's outcome - see
/// `docs/formats/endrace-screens.md`'s "Line1's headline text is
/// mode-driven".
#[must_use]
pub(crate) fn headline(mode: oag_race::Mode, place: Option<u8>) -> Headline {
    match mode {
        oag_race::Mode::TimeTrial => Headline::TimeTrial,
        oag_race::Mode::SpeedLap => Headline::SpeedLap,
        // A leg's own headline is its own finishing place - the final
        // standings rank is a `Rewards`/medal fact, not this screen's, per
        // `tournament.md`'s own "the medal-eligible value is the final
        // standings rank... not the last leg's own finishing position"
        // distinction: that distinction is about what earns the medal, not
        // about what this headline reports.
        // `Head2Head` groups with `Race`/`Tournament` here too -
        // `EndRaceResults_OnEnter`'s own mode-shaped switch puts mode `9` in
        // the same `{3,4,9,0x10}`-adjacent family this project's
        // `campaign_medal` already reads a finishing position for, and
        // `docs/ghidra/functions/psp-pulse-usa/endrace-screens.md`'s
        // `lapTerm`/`difficultyMult` grouping names `Race,Tournament,Head2Head`
        // outright.
        oag_race::Mode::SingleRace | oag_race::Mode::Tournament | oag_race::Mode::Head2Head => {
            match place {
                Some(place) => Headline::Position(place),
                None => Headline::NoPosition,
            }
        }
        // `Zone`/`Eliminator` go through a populate helper this project has
        // not decompiled - see `oag_ui::endrace::Headline::Unresolved`'s own
        // doc.
        oag_race::Mode::Zone | oag_race::Mode::Eliminator => Headline::Unresolved,
    }
}

/// `EndRace Menu`'s own option list for this run - see
/// `docs/formats/endrace-screens.md`'s numbered populate list.
/// `ER_SAVE_QUIT` (mid-Tournament save-and-quit) never appears: this engine
/// implements no `Tournament_SaveProgress`/`_LoadProgress` -
/// `Session::tournament`'s own doc says why. `ER_SAVE_GHOST`/`MSC_DEL_DATA`
/// never appear either - see `oag_ui::endrace`'s own module doc for why.
///
/// **`tournament_next_leg`**: `true` on every leg but a Tournament cell's
/// own last one, offering `ER_NEXT_RACE` in place of `RACE AGAIN` - the
/// last leg "instead resolves to the ordinary `EndRace Menu` options",
/// per `docs/gameplay/race-modes.md#tournament`. `false` for every
/// non-Tournament race, and for a Tournament's own last leg.
#[must_use]
pub(crate) fn menu_options(campaign: bool, tournament_next_leg: bool) -> Vec<MenuOption> {
    vec![
        if campaign {
            MenuOption::ReturnToGrid
        } else {
            MenuOption::ReturnToMenu
        },
        if tournament_next_leg {
            MenuOption::NextRace
        } else {
            MenuOption::RaceAgain
        },
        MenuOption::ViewResultsAgain,
    ]
}

/// Wipeout HD/Fury's own `EndRace Results`: the whole field, ordered by
/// place, off the field's own `Board` - the same feed
/// `oag_game::scoreboard::Overlay` draws when no `EndRace_Definition.xml` is
/// read at all. `board` is `None` only when `Session::build_endrace` is
/// called before `Race::capture_results` has run, which does not happen -
/// `build_endrace`'s own guard requires `stage.race.finished()` first, and
/// that is the same tick `capture_results` takes the board on - so this
/// draws an empty field rather than failing outright, on the same "an
/// honest absence over a guess" reasoning as every other gap on this
/// screen, not because the `None` case is expected to fire.
#[must_use]
pub(crate) fn hd_field_rows(
    board: Option<&oag_game::scoreboard::Board>,
) -> Vec<oag_ui::endrace::FieldRow> {
    board
        .map(|board| {
            board
                .rows
                .iter()
                .map(|row| oag_ui::endrace::FieldRow {
                    place: row.place,
                    time_ticks: row.finish_tick,
                    player: row.player,
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Pulse's own Tournament standings, off this project's `oag_race::tournament`
/// law: [`Board`](oag_game::scoreboard::Board) for this leg's own finish
/// order and leg points (`oag_race::tournament::points_for_finish`, the same
/// law already folded into the running totals a moment earlier -
/// `crate::main::session::tournament::record_finished_leg` runs before
/// `Session::build_endrace` in the same frame), and
/// `crate::race::tournament::Progress` for the cumulative totals and rank.
/// `slot_teams` is this leg's own grid roster, in slot order -
/// `crate::main::session::endrace::build_endrace` recomputes it off
/// `crate::race::slot_teams`, since which team flew which slot is not
/// carried on `Board`/`Progress` themselves. `None` team entries draw the
/// name absent rather than a placeholder - see
/// [`oag_ui::endrace::TournamentRow::team_name`]'s own doc.
#[must_use]
pub(crate) fn tournament_results(
    board: Option<&oag_game::scoreboard::Board>,
    progress: &crate::race::tournament::Progress,
    slot_teams: Option<&[String]>,
    last_leg: bool,
) -> Option<TournamentResults> {
    let board = board?;
    let team_name = |slot: u8| slot_teams.and_then(|teams| teams.get(usize::from(slot)).cloned());
    let leg: Vec<TournamentRow> = board
        .rows
        .iter()
        .map(|row| TournamentRow {
            team_name: team_name(row.slot),
            points: oag_race::tournament::points_for_finish(row.place, row.finished()),
            player: row.player,
        })
        .collect();

    let mut slots: Vec<u8> = board.rows.iter().map(|row| row.slot).collect();
    slots.sort_by_key(|&slot| progress.rank(usize::from(slot)));
    let standings: Vec<TournamentRow> = slots
        .into_iter()
        .map(|slot| TournamentRow {
            team_name: team_name(slot),
            points: progress.points(usize::from(slot)),
            player: slot == 0,
        })
        .collect();

    Some(TournamentResults::new(
        last_leg,
        progress.leg_number(),
        progress.leg_count(),
        leg,
        standings,
    ))
}

/// `oag_game::records::Medal` restated as `oag_tables::race_campaign::Medal` -
/// the identical conversion `crate::campaign_stage::to_campaign_medal`
/// makes, duplicated rather than shared per that function's own doc.
#[must_use]
pub(crate) fn to_campaign_medal(medal: oag_game::records::Medal) -> Medal {
    match medal {
        oag_game::records::Medal::Gold => Medal::Gold,
        oag_game::records::Medal::Silver => Medal::Silver,
        oag_game::records::Medal::Bronze => Medal::Bronze,
    }
}

/// What this race's own outcome feeds `loyalty_award` - one field per term
/// `Race_ComputeLoyaltyAward` reads. Two of the seven are always `0`/`false`
/// because this project keeps no running tally for them: `perfect_laps` (no
/// per-lap "was this one perfect" accumulator exists past the disc's own
/// unread `perfectlap{n}` flag direction - see
/// `docs/formats/endrace-screens.md`) and `perfect_zones` (`oag_race::state::Outcome::perfect_zone`
/// is a per-tick event, not a running count, and `crates/race` is outside
/// this lane). `suggested_ship` is always `false` - `docs/ui/campaign-screens.md`'s
/// own "Not launched from a cell: the AI difficulty curve" already records
/// that this project implements no per-cell suggested-ship mechanism at all.
#[derive(Debug, Clone, Copy)]
pub(crate) struct LoyaltyInputs {
    pub(crate) mode: oag_race::Mode,
    pub(crate) laps: u32,
    pub(crate) perfect_laps: u32,
    pub(crate) kills: u32,
    pub(crate) zones: u32,
    pub(crate) perfect_zones: u32,
    /// `0`/`1`/`2` (easy/medium/hard) - `None` when this project has no
    /// honest mapping to offer. See [`loyalty_award`]'s own doc on why this
    /// is `None` for every race today.
    pub(crate) difficulty: Option<u8>,
    pub(crate) suggested_ship: bool,
}

/// `Race_ComputeLoyaltyAward` (`0x0880ac50`), confidence 95, confirmed on
/// two independent live `Time Trial` races -
/// `docs/ghidra/functions/psp-pulse-usa/endrace-screens.md`. The
/// `SingleRace`/Tournament/Head2Head branch (the only one with a difficulty
/// multiplier) and the `Zone`/`Eliminator` branch are decompiled under the
/// same page but not independently live-verified - `oag_ui::endrace::Loyalty`
/// carries no flag for this, so `docs/ui/endrace-screens.md` states the
/// caveat in prose rather than on screen, the same way this project already
/// treats every other confidence gap that does not change what draws.
///
/// **`inputs.difficulty` is always `None` in this build**: the original's
/// own `easy`/`medium`/`hard` three-tier scale is a campaign cell's own
/// `skill`/`skillEasy`/`skillHard` (`AI_ResolveSkillScale`, not implemented -
/// see `docs/ui/campaign-screens.md`), not this project's four-tier
/// `[ai] difficulty` (`novice`/`skilled`/`elite`/`ace`) - the two scales do
/// not correspond, so mapping one onto the other would be inventing a
/// correspondence the disc does not author. `None` reads as `SingleRace`'s
/// own multiplier defaulting to `1` (no multiplier) - a real absence, not a
/// guessed "medium".
#[must_use]
pub(crate) fn loyalty_award(inputs: LoyaltyInputs) -> u32 {
    let (lap_rate, perfect_lap_rate) = match inputs.mode {
        // The same branch the original's own decompile puts Head2Head in
        // too - see this function's own doc comment.
        oag_race::Mode::SingleRace | oag_race::Mode::Tournament | oag_race::Mode::Head2Head => {
            (15, 25)
        }
        oag_race::Mode::TimeTrial | oag_race::Mode::SpeedLap => (30, 50),
        oag_race::Mode::Zone | oag_race::Mode::Eliminator => (10, 20),
    };
    let lap_term = inputs.laps * lap_rate + inputs.perfect_laps * perfect_lap_rate;
    let kill_rate = if inputs.mode == oag_race::Mode::Eliminator {
        30
    } else {
        15
    };
    let kill_term = inputs.kills * kill_rate;
    let zone_term = inputs.zones * 10 + inputs.perfect_zones * 20;

    let mut multiplier = 1;
    if matches!(
        inputs.mode,
        oag_race::Mode::SingleRace | oag_race::Mode::Tournament | oag_race::Mode::Head2Head
    ) {
        multiplier = match inputs.difficulty {
            Some(0) => 2,
            Some(1) => 3,
            Some(2) => 4,
            // No honest mapping - see this function's own doc.
            _ => 1,
        };
    }
    if inputs.suggested_ship {
        multiplier *= 2;
    }

    multiplier * (lap_term + kill_term + zone_term)
}

#[cfg(test)]
mod tests {
    use super::{LoyaltyInputs, hd_field_rows, loyalty_award};
    use oag_game::scoreboard::{Board, Row};

    /// `hd_field_rows` reads place/finish tick/player straight off the
    /// field's own `Board`, in the board's own order - no reordering, no
    /// renumbering, the same "the places are used exactly as assigned"
    /// rule `Race::capture_results`'s own doc states for the board itself.
    /// A craft still racing when the board was taken (`finish_tick: None`)
    /// keeps that absence rather than a guessed time.
    #[test]
    fn hd_field_rows_reads_place_finish_tick_and_player_off_the_board_unchanged() {
        let board = Board {
            rows: vec![
                Row {
                    place: 1,
                    slot: 2,
                    laps_completed: 3,
                    finish_tick: Some(1234),
                    best_lap_ticks: Some(400),
                    player: false,
                },
                Row {
                    place: 2,
                    slot: 0,
                    laps_completed: 2,
                    finish_tick: None,
                    best_lap_ticks: Some(410),
                    player: true,
                },
            ],
            laps_target: Some(3),
            tick: 1234,
        };
        let rows = hd_field_rows(Some(&board));
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].place, 1);
        assert_eq!(rows[0].time_ticks, Some(1234));
        assert!(!rows[0].player);
        assert_eq!(rows[1].place, 2);
        assert_eq!(rows[1].time_ticks, None);
        assert!(rows[1].player);
    }

    #[test]
    fn hd_field_rows_is_empty_rather_than_panicking_with_no_board() {
        assert!(hd_field_rows(None).is_empty());
    }

    /// The two live races
    /// (`docs/ghidra/functions/psp-pulse-usa/endrace-screens.md`'s
    /// "Runtime-confirmed 2026-09-14"): a 3-lap Time Trial, zero of
    /// everything else, is `90` both times, independent of pace.
    #[test]
    fn a_three_lap_time_trial_with_nothing_else_reproduces_ninety() {
        let award = loyalty_award(LoyaltyInputs {
            mode: oag_race::Mode::TimeTrial,
            laps: 3,
            perfect_laps: 0,
            kills: 0,
            zones: 0,
            perfect_zones: 0,
            difficulty: None,
            suggested_ship: false,
        });
        assert_eq!(award, 90);
    }

    /// `SingleRace`'s own difficulty multiplier - decompiled, not
    /// independently live-verified (see the module's own doc on
    /// `loyalty_award`) - doubles/triples/quadruples the lap term, and a
    /// suggested-ship race doubles whatever that already is.
    #[test]
    fn single_race_carries_a_difficulty_multiplier_the_other_modes_do_not() {
        let base = |difficulty| {
            loyalty_award(LoyaltyInputs {
                mode: oag_race::Mode::SingleRace,
                laps: 3,
                perfect_laps: 0,
                kills: 0,
                zones: 0,
                perfect_zones: 0,
                difficulty,
                suggested_ship: false,
            })
        };
        // 3 laps * 15 = 45, times the multiplier.
        assert_eq!(base(None), 45, "no honest mapping - defaults to x1");
        assert_eq!(base(Some(0)), 90, "easy - x2");
        assert_eq!(base(Some(1)), 135, "medium - x3");
        assert_eq!(base(Some(2)), 180, "hard - x4");

        let suggested = loyalty_award(LoyaltyInputs {
            mode: oag_race::Mode::SingleRace,
            laps: 3,
            perfect_laps: 0,
            kills: 0,
            zones: 0,
            perfect_zones: 0,
            difficulty: Some(0),
            suggested_ship: true,
        });
        assert_eq!(
            suggested, 180,
            "easy x2, doubled again for a suggested ship"
        );
    }

    /// `Zone`/`Eliminator` take the fixed `10`/`20` lap rate and their own
    /// kill/zone terms, and carry no difficulty multiplier at all.
    #[test]
    fn zone_and_eliminator_use_the_low_rate_and_their_own_terms() {
        let zone = loyalty_award(LoyaltyInputs {
            mode: oag_race::Mode::Zone,
            laps: 0,
            perfect_laps: 0,
            kills: 0,
            zones: 5,
            perfect_zones: 1,
            difficulty: Some(2), // ignored outside SingleRace
            suggested_ship: false,
        });
        assert_eq!(zone, 5 * 10 + 20);

        let eliminator = loyalty_award(LoyaltyInputs {
            mode: oag_race::Mode::Eliminator,
            laps: 2,
            perfect_laps: 0,
            kills: 3,
            zones: 0,
            perfect_zones: 0,
            difficulty: None,
            suggested_ship: false,
        });
        assert_eq!(eliminator, 2 * 10 + 3 * 30);
    }
}
