//! The seat assignment screen over the menu stage: the assignment being made and the window
//! layout screen it came from. The flow that opens, answers and closes it is
//! `crate::session::seat_assign`; the model and its drawing are `oag_ui_screens::seat_assign`'s.
//!
//! Like [`crate::window_layout_stage::WindowLayoutStage`] it takes the tick whole while open
//! and is drawn over the page behind it. It keeps the layout stage it came from, so BACK lands
//! on the layout screen with the choice as it was.

use oag_ui_screens::seat_assign::SeatAssign;

use crate::window_layout_stage::WindowLayoutStage;

/// The screen, open.
pub(crate) struct SeatAssignStage {
    layout: WindowLayoutStage,
    screen: SeatAssign,
}

impl SeatAssignStage {
    /// Opened only where windows can be (a desktop build).
    #[cfg_attr(
        any(target_arch = "wasm32", target_os = "android", target_os = "ios"),
        allow(dead_code)
    )]
    pub(crate) fn new(layout: WindowLayoutStage, screen: SeatAssign) -> Self {
        Self { layout, screen }
    }

    /// The layout screen this one came from, which BACK restores.
    pub(crate) fn into_layout(self) -> WindowLayoutStage {
        self.layout
    }

    pub(crate) fn layout(&self) -> &WindowLayoutStage {
        &self.layout
    }

    pub(crate) fn screen(&self) -> &SeatAssign {
        &self.screen
    }

    pub(crate) fn screen_mut(&mut self) -> &mut SeatAssign {
        &mut self.screen
    }
}
