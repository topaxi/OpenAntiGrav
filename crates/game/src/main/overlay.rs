//! What a modal prompt over the menus is **for**, and the two lines that keep
//! that out of [`crate::menu_stage`].
//!
//! [`oag_ui_screens::prompt`] is the model: a grid of keys, a buffer, a yes/no, and
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
use oag_ui_screens::prompt::{Confirm, Edit, Keyboard, Outcome};
use oag_ui_screens::tag_entry::TagEntry;

/// What to do with a prompt the player accepted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Purpose {
    /// Move `from`'s file to whatever the keyboard ends up holding.
    RenamePilot {
        /// The name the keyboard opened on.
        from: String,
    },
    /// Delete `name`'s file - which **restores** the built-in when `name` is
    /// one of the four the binary ships. See `oag_raceplay::pilots::delete_pilot`.
    DeletePilot {
        /// Whose file.
        name: String,
    },
}

/// Which text-entry or confirm model is on screen.
///
/// **Two typing shapes, not one** - see `docs/formats/fexml.md`'s `TagInput`
/// section. [`Keyboard`] is this project's own grid, used for a title whose
/// disc authors no `<TagInput>`, or whose authored alphabet cannot spell the
/// text being edited. [`TagEntry`] plays Pulse's own widget - the row of
/// cells the alphabet scrolls through - when the data says it can: see
/// `session::pilot_editor::Session::tag_entry_for_rename`'s own gate.
#[derive(Debug, Clone)]
pub(crate) enum Model {
    /// Typing something, this project's own grid.
    Keyboard(Keyboard),
    /// Typing something, Pulse's own cell row. Boxed: `TagEntry` carries
    /// its own `Geometry` (several `Vec`s), 576 bytes against `Keyboard`'s
    /// 160 - `clippy::large_enum_variant`.
    TagEntry(Box<TagEntry>),
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
    /// A keyboard prompt: this project's own grid.
    pub(crate) fn typing(purpose: Purpose, keyboard: Keyboard) -> Self {
        Self {
            purpose,
            model: Model::Keyboard(keyboard),
        }
    }

    /// A `TagEntry` prompt: Pulse's own cell row. See [`Model`]'s own doc.
    pub(crate) fn tagging(purpose: Purpose, tag_entry: TagEntry) -> Self {
        Self {
            purpose,
            model: Model::TagEntry(Box::new(tag_entry)),
        }
    }

    /// A yes/no prompt.
    pub(crate) fn asking(purpose: Purpose, confirm: Confirm) -> Self {
        Self {
            purpose,
            model: Model::Confirm(confirm),
        }
    }

    /// Whether this is a typing model (either shape), as opposed to
    /// [`Confirm`] - which has no buffer for a desk key or a live note to
    /// reach.
    pub(crate) fn is_typing(&self) -> bool {
        !matches!(self.model, Model::Confirm(_))
    }

    /// Sets, or clears, the live remark drawn under whichever typing model
    /// this is. A no-op on [`Confirm`].
    pub(crate) fn set_note(&mut self, note: Option<String>) {
        match &mut self.model {
            Model::Keyboard(keyboard) => keyboard.set_note(note),
            Model::TagEntry(tag_entry) => tag_entry.set_note(note),
            Model::Confirm(_) => {}
        }
    }

    /// Applies one desk-keyboard edit to whichever typing model this is, and
    /// says whether there was one to apply it to.
    pub(crate) fn edit(&mut self, edit: Edit) -> bool {
        match &mut self.model {
            Model::Keyboard(keyboard) => {
                keyboard.edit(edit);
                true
            }
            Model::TagEntry(tag_entry) => {
                tag_entry.edit(edit);
                true
            }
            Model::Confirm(_) => false,
        }
    }

    /// What the buffer currently holds, for whichever typing model this is -
    /// `None` for [`Confirm`], which has none.
    pub(crate) fn typed_text(&self) -> Option<String> {
        match &self.model {
            Model::Keyboard(keyboard) => Some(keyboard.text().to_string()),
            Model::TagEntry(tag_entry) => Some(tag_entry.text()),
            Model::Confirm(_) => None,
        }
    }

    /// Consumes a tick of input. See [`Keyboard::update`] on why these are
    /// edges.
    pub(crate) fn update(&mut self, input: &mut Input) -> Outcome {
        match &mut self.model {
            Model::Keyboard(keyboard) => keyboard.update(input),
            Model::TagEntry(tag_entry) => tag_entry.update(input),
            Model::Confirm(confirm) => confirm.update(input),
        }
    }

    /// Consumes a tick of pointer input. [`Keyboard`] and [`Confirm`] take
    /// the skin because their targets are wherever [`Self::draw`] just put
    /// them; [`TagEntry`] does not, because it draws at the disc's own
    /// authored positions rather than a skin-relative panel.
    pub(crate) fn pointer(&mut self, pointer: &oag_ui::pointer::Pointer, skin: &Skin) -> Outcome {
        match &mut self.model {
            Model::Keyboard(keyboard) => keyboard.pointer(pointer, skin),
            Model::TagEntry(tag_entry) => tag_entry.pointer(pointer),
            Model::Confirm(confirm) => confirm.pointer(pointer, skin),
        }
    }

    /// What this looks like, over whatever the menus already drew.
    pub(crate) fn draw(&self, skin: &Skin) -> Vec<Draw> {
        match &self.model {
            Model::Keyboard(keyboard) => keyboard.draw(skin),
            Model::TagEntry(tag_entry) => tag_entry.draw(skin),
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
