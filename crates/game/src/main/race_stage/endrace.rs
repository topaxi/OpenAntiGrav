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
use oag_ui::endrace::{EndRaceMenu, Headline, MenuOption, Results, Rewards};

use oag_game::render::Renderer;

/// Which of the three screens is on top.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Which {
    Results,
    Rewards,
    Menu,
}

/// The three screens' own content, built once when the race finishes, and
/// which one is current.
pub(crate) struct EndRaceRuntime {
    screens: oag_game::endrace::EndRaceScreens,
    skin: oag_ui::menu::Skin,
    frame: oag_ui::menu::Frame,
    strings: oag_ui::language::StringTable,
    renderer: Renderer,
    results: Results,
    rewards: Rewards,
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
        results: Results,
        rewards: Rewards,
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
            Which::Results if self.rewards.campaign => Which::Rewards,
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

    /// `EndRace Menu`'s own layout - what
    /// `oag_ui::endrace::pointer::menu_targets` reads its row rects from.
    #[must_use]
    pub(crate) fn menu_layout(&self) -> &oag_ui::endrace::Layout {
        &self.screens.menu
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
        let layers = match self.which {
            Which::Results => oag_ui::endrace::results_draw_list(
                &self.results,
                &self.screens.results,
                &self.skin,
                &self.frame,
                &self.strings,
                None,
                true,
                &|src| sprites.get(src),
            ),
            Which::Rewards => oag_ui::endrace::rewards_draw_list(
                &self.rewards,
                &self.screens.rewards,
                &self.skin,
                &self.frame,
                &self.strings,
                None,
                true,
                &|src| sprites.get(src),
            ),
            Which::Menu => oag_ui::endrace::endrace_menu_draw_list(
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
        oag_race::Mode::SingleRace => match place {
            Some(place) => Headline::Position(place),
            None => Headline::NoPosition,
        },
        // `Zone`/`Eliminator` go through a populate helper this project has
        // not decompiled - see `oag_ui::endrace::Headline::Unresolved`'s own
        // doc.
        oag_race::Mode::Zone | oag_race::Mode::Eliminator => Headline::Unresolved,
    }
}

/// `EndRace Menu`'s own option list for this run - see
/// `docs/formats/endrace-screens.md`'s numbered populate list.
/// `ER_NEXT_RACE`/`ER_SAVE_QUIT` (mid-Tournament) never appear: this engine
/// implements no Tournament mode. `ER_SAVE_GHOST`/`MSC_DEL_DATA` never
/// appear either - see `oag_ui::endrace`'s own module doc for why.
#[must_use]
pub(crate) fn menu_options(campaign: bool) -> Vec<MenuOption> {
    vec![
        if campaign {
            MenuOption::ReturnToGrid
        } else {
            MenuOption::ReturnToMenu
        },
        MenuOption::RaceAgain,
        MenuOption::ViewResultsAgain,
    ]
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
        oag_race::Mode::SingleRace => (15, 25),
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
    if inputs.mode == oag_race::Mode::SingleRace {
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
    use super::{LoyaltyInputs, loyalty_award};

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
