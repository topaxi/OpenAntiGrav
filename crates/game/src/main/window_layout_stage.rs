//! The window layout screen over the menu stage: the choice being made and the join
//! screen it came from. The flow that opens, answers and closes it is
//! `crate::session::window_layout`; the model and its drawing are
//! `oag_ui_screens::window_layout`'s.
//!
//! Like [`crate::join_stage::JoinStage`] it takes the tick whole while open and is drawn over
//! the page behind it. It keeps the join stage it came from, so BACK lands on the join screen
//! with everybody still seated.

use oag_ui_screens::window_layout::Picker;

use crate::join_stage::JoinStage;

/// The screen, open.
pub(crate) struct WindowLayoutStage {
    join: JoinStage,
    screen: Picker,
}

impl WindowLayoutStage {
    pub(crate) fn new(join: JoinStage, screen: Picker) -> Self {
        Self { join, screen }
    }

    /// The join screen this one came from, which BACK restores.
    pub(crate) fn into_join(self) -> JoinStage {
        self.join
    }

    pub(crate) fn join(&self) -> &JoinStage {
        &self.join
    }

    pub(crate) fn screen(&self) -> &Picker {
        &self.screen
    }

    pub(crate) fn screen_mut(&mut self) -> &mut Picker {
        &mut self.screen
    }
}
