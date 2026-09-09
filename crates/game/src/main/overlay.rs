//! What a modal prompt over the menus is **for**, and the two lines that keep
//! that out of [`crate::menu_stage`].
//!
//! [`oag_ui::prompt`] is the model: a grid of keys, a buffer, a yes/no, and
//! no idea what any of it will be used for. This is the other half - which
//! pilot is being renamed, which file is about to be deleted - and it lives on
//! the binary side because only the composition root knows.
//!
//! **The purpose travels with the prompt, not beside it.** A `Purpose` on
//! [`crate::session::Session`] and a model on the stage would be two things to
//! keep in step, and the failure mode is the worst one this feature has: a
//! keyboard accepted against the purpose left over from the last time it was
//! opened, renaming a pilot the player is no longer looking at.

use oag_game::input::Input;
use oag_ui::frontend::Draw;
use oag_ui::menu::Skin;
use oag_ui::prompt::{Confirm, Keyboard, Outcome};

/// What to do with a prompt the player accepted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Purpose {
    /// Move `from`'s file to whatever the keyboard ends up holding.
    RenamePilot {
        /// The name the keyboard opened on.
        from: String,
    },
    /// Delete `name`'s file - which **restores** the built-in when `name` is
    /// one of the four the binary ships. See `oag_game::pilots::delete_pilot`.
    DeletePilot {
        /// Whose file.
        name: String,
    },
}

/// Which of [`oag_ui::prompt`]'s two models is on screen.
#[derive(Debug, Clone)]
pub(crate) enum Model {
    /// Typing something.
    Keyboard(Keyboard),
    /// Answering something.
    Confirm(Confirm),
}

/// A prompt on screen, and what accepting it means.
#[derive(Debug, Clone)]
pub(crate) struct Prompt {
    /// What accepting this does.
    pub(crate) purpose: Purpose,
    /// The model taking the input.
    pub(crate) model: Model,
}

impl Prompt {
    /// A keyboard prompt.
    pub(crate) fn typing(purpose: Purpose, keyboard: Keyboard) -> Self {
        Self {
            purpose,
            model: Model::Keyboard(keyboard),
        }
    }

    /// A yes/no prompt.
    pub(crate) fn asking(purpose: Purpose, confirm: Confirm) -> Self {
        Self {
            purpose,
            model: Model::Confirm(confirm),
        }
    }

    /// The keyboard, when this is one - for the caller's live note and for
    /// reading the text back out on accept.
    pub(crate) fn keyboard_mut(&mut self) -> Option<&mut Keyboard> {
        match &mut self.model {
            Model::Keyboard(keyboard) => Some(keyboard),
            Model::Confirm(_) => None,
        }
    }

    /// Consumes a tick of input. See [`Keyboard::update`] on why these are
    /// edges.
    pub(crate) fn update(&mut self, input: &mut Input) -> Outcome {
        match &mut self.model {
            Model::Keyboard(keyboard) => keyboard.update(input),
            Model::Confirm(confirm) => confirm.update(input),
        }
    }

    /// What this looks like, over whatever the menus already drew.
    pub(crate) fn draw(&self, skin: &Skin) -> Vec<Draw> {
        match &self.model {
            Model::Keyboard(keyboard) => keyboard.draw(skin),
            Model::Confirm(confirm) => confirm.draw(skin),
        }
    }
}

/// What a finished prompt asked for: its purpose, and the text it ended on.
///
/// The text is taken here rather than read off the model later because the
/// model is dropped the moment it finishes - carrying it out is what lets the
/// caller act with `&mut Session` and no borrow of the stage still live.
pub(crate) struct Finished {
    /// What was being done.
    pub(crate) purpose: Purpose,
    /// What the keyboard held, or empty for a confirm.
    pub(crate) text: String,
}
