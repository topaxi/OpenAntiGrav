//! Wipeout 2048's end-of-race pages held open over a finished race: the runtime
//! behind [`EndRace::Touch`], and the [`EndRace`] enum that lets
//! `RaceStage::endrace` hold either title family's flow.
//!
//! The model and the drawing are `oag_ui_screens::endrace::touch`; what a
//! finished race says is `oag_game::endrace::touch::summary`; the flow that
//! builds and drives this is `crate::main::session::endrace_touch`. Like
//! [`super::endrace`], it draws inside `Stage::Race` over the race's own last
//! frame rather than moving to `Stage::Menu`.

use anyhow::Result;

use oag_game::render::Renderer;
use oag_ui_screens::endrace::touch::{self, Button, Layouts, Page, Summary};

use super::endrace::EndRaceRuntime;

/// Ticks a page holds before the next comes up on its own: `5.0` seconds at
/// the engine's fixed 60 Hz. Measured, `FUN_810cd0b8`.
const PAGE_TICKS: u32 = (touch::PAGE_SECONDS * 60.0) as u32;
/// Ticks after the finish before a confirm may leave. **Chosen, not
/// measured**: a throttle held across the line must not dismiss a page the
/// player has not seen.
const CONFIRM_AFTER_TICKS: u32 = 60;

/// What a confirmed tile asks the session to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Action {
    /// Race the same event again.
    Restart,
    /// Leave the finished race.
    Exit,
}

/// One press the flow reads, already separated from the device.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Press {
    /// Move the pad cursor to the neighbouring tile.
    Sideways,
    /// Previous page.
    Previous,
    /// Next page.
    Next,
    /// Activate the selected tile.
    Confirm,
}

/// Which page is up, which tile the pad is on, and the clocks: the GPU-free
/// half of [`TouchRuntime`], tested without a device.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Flow {
    pages: u8,
    page: u8,
    selected: Button,
    on_page: u32,
    since_finish: u32,
}

impl Flow {
    /// Opens on [`Page::Summary`] with the exit tile under the cursor.
    pub(crate) fn new(has_objectives_page: bool) -> Self {
        Self {
            pages: if has_objectives_page { 2 } else { 1 },
            page: 0,
            selected: Button::Exit,
            on_page: 0,
            since_finish: 0,
        }
    }

    pub(crate) fn page(&self) -> Page {
        if self.page == 0 {
            Page::Summary
        } else {
            Page::Objectives
        }
    }

    pub(crate) fn selected(&self) -> Button {
        self.selected
    }

    /// One tick. A page that has held [`PAGE_TICKS`] hands over to the next,
    /// and the last page stays.
    pub(crate) fn tick(&mut self) {
        self.since_finish = self.since_finish.saturating_add(1);
        self.on_page += 1;
        if self.on_page >= PAGE_TICKS && self.page + 1 < self.pages {
            self.go(self.page + 1);
        }
    }

    fn go(&mut self, page: u8) {
        self.page = page;
        self.on_page = 0;
    }

    /// Whether a confirm may leave yet.
    pub(crate) fn takes_confirm(&self) -> bool {
        self.since_finish >= CONFIRM_AFTER_TICKS
    }

    /// Applies one press. Only [`Press::Confirm`] answers with an action.
    pub(crate) fn press(&mut self, press: Press) -> Option<Action> {
        match press {
            Press::Sideways => {
                self.selected = match self.selected {
                    Button::Restart => Button::Exit,
                    Button::Exit => Button::Restart,
                };
                None
            }
            Press::Previous => {
                self.go(self.page.saturating_sub(1));
                None
            }
            Press::Next => {
                self.go((self.page + 1).min(self.pages - 1));
                None
            }
            Press::Confirm => self.takes_confirm().then(|| self.action_of(self.selected)),
        }
    }

    /// A tap on a tile: selects it and answers its action at once, held to
    /// the same guard as [`Press::Confirm`].
    pub(crate) fn tap_tile(&mut self, button: Button) -> Option<Action> {
        self.selected = button;
        self.takes_confirm().then(|| self.action_of(button))
    }

    /// A tap on the panel turns the page, wrapping, when there is more than one.
    pub(crate) fn tap_panel(&mut self) {
        if self.pages > 1 {
            self.go((self.page + 1) % self.pages);
        }
    }

    fn action_of(&self, button: Button) -> Action {
        match button {
            Button::Restart => Action::Restart,
            Button::Exit => Action::Exit,
        }
    }
}

/// The pages, drawn over the race.
pub(crate) struct TouchRuntime {
    layouts: Layouts,
    sprites: oag_hud::sprite::Sheet,
    model: Summary,
    renderer: Renderer,
    space: oag_display::space::Space,
    scales: Vec<(String, f32)>,
    line: f32,
    flow: Flow,
}

impl TouchRuntime {
    /// # Errors
    ///
    /// Propagates the renderer's own pipeline-build errors.
    #[allow(
        clippy::too_many_arguments,
        reason = "one build site, each a separate fact"
    )]
    pub(crate) fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        screens: oag_game::endrace::touch::TouchScreens,
        model: Summary,
        space: oag_display::space::Space,
        atlas: oag_ui::font::Atlas,
        scales: Vec<(String, f32)>,
    ) -> Result<Self> {
        let mut renderer =
            Renderer::new(device, queue, format, None, atlas.clone(), &screens.sprites)?;
        renderer.set_space(space);
        Ok(Self {
            flow: Flow::new(model.has_objectives_page()),
            layouts: screens.layouts,
            sprites: screens.sprites,
            model,
            renderer,
            space,
            scales,
            line: atlas.line_height,
        })
    }

    pub(crate) fn space(&self) -> oag_display::space::Space {
        self.space
    }

    pub(crate) fn flow_mut(&mut self) -> &mut Flow {
        &mut self.flow
    }

    /// What a pointer did this tick, in the grid.
    pub(crate) fn pointer_hit(&self, pointer: &oag_ui::pointer::Pointer) -> Option<touch::Hit> {
        touch::pointer_hit(&self.layouts, &self.model, pointer)
    }

    pub(crate) fn draw(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        viewport: (f32, f32, f32, f32),
    ) {
        let sprites = &self.sprites;
        let list = touch::draw_list(
            &self.model,
            self.flow.page(),
            self.flow.selected(),
            &self.layouts,
            &|src| sprites.get(src),
            &self.scales,
            self.line,
        );
        self.renderer
            .overlay(device, queue, encoder, view, &list, viewport);
    }
}

/// Whichever family's end-of-race flow this race holds.
pub(crate) enum EndRace {
    /// Pulse's and HD's `Results`/`Rewards`/`Menu`.
    Disc(Box<EndRaceRuntime>),
    /// 2048's `RaceSummary` pages.
    Touch(Box<TouchRuntime>),
}

impl std::fmt::Debug for EndRace {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Disc(runtime) => runtime.fmt(f),
            Self::Touch(_) => f.write_str("EndRace::Touch"),
        }
    }
}

impl EndRace {
    /// The grid the screens are authored in.
    pub(crate) fn space(&self) -> oag_display::space::Space {
        match self {
            Self::Disc(runtime) => runtime.space(),
            Self::Touch(runtime) => runtime.space(),
        }
    }

    /// Whether `Race End Photo` is up - never, on the touch pages.
    pub(crate) fn is_photo(&self) -> bool {
        matches!(self, Self::Disc(runtime) if runtime.is_photo())
    }

    pub(crate) fn draw(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        viewport: (f32, f32, f32, f32),
        target_size: (u32, u32),
    ) {
        match self {
            Self::Disc(runtime) => {
                runtime.draw(device, queue, encoder, view, viewport, target_size);
            }
            Self::Touch(runtime) => runtime.draw(device, queue, encoder, view, viewport),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flow() -> Flow {
        let mut flow = Flow::new(true);
        for _ in 0..CONFIRM_AFTER_TICKS {
            flow.tick();
        }
        flow
    }

    #[test]
    fn a_confirm_in_the_first_second_leaves_nothing() {
        let mut flow = Flow::new(false);
        flow.tick();
        assert_eq!(flow.press(Press::Confirm), None);
        assert_eq!(flow.tap_tile(Button::Restart), None);
    }

    #[test]
    fn the_cursor_starts_on_the_exit_tile_and_confirm_answers_it() {
        let mut flow = flow();
        assert_eq!(flow.selected(), Button::Exit);
        assert_eq!(flow.press(Press::Confirm), Some(Action::Exit));
        flow.press(Press::Sideways);
        assert_eq!(flow.press(Press::Confirm), Some(Action::Restart));
    }

    #[test]
    fn a_page_holds_five_seconds_then_hands_over_and_the_last_stays() {
        let mut flow = Flow::new(true);
        for _ in 0..PAGE_TICKS - 1 {
            flow.tick();
        }
        assert_eq!(flow.page(), Page::Summary);
        flow.tick();
        assert_eq!(flow.page(), Page::Objectives);
        for _ in 0..PAGE_TICKS * 2 {
            flow.tick();
        }
        assert_eq!(flow.page(), Page::Objectives);
    }

    #[test]
    fn an_event_with_no_objective_has_one_page_and_a_panel_tap_changes_nothing() {
        let mut flow = Flow::new(false);
        flow.tap_panel();
        flow.press(Press::Next);
        assert_eq!(flow.page(), Page::Summary);
    }

    #[test]
    fn paging_by_pad_and_by_tap_turn_the_same_pages() {
        let mut flow = flow();
        flow.press(Press::Next);
        assert_eq!(flow.page(), Page::Objectives);
        flow.press(Press::Previous);
        assert_eq!(flow.page(), Page::Summary);
        flow.tap_panel();
        assert_eq!(flow.page(), Page::Objectives);
    }
}
