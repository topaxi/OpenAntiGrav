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
