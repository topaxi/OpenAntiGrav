//! Which glyphs the on-screen button prompts draw this frame.
//!
//! The family follows the device last used (`[controls] prompt_style`'s
//! `auto`) or whatever the player forced; the keyboard family reads the live
//! bindings. The result is handed to whichever renderer the stage draws
//! its text through - see [`oag_game::prompts`] and `docs/ui/button-prompts.md`.

use oag_game::prompts::substitution;

use crate::stage::Stage;

use super::Session;

impl Session {
    /// Sets this frame's prompt substitution on the stage's renderer.
    ///
    /// Every frame rather than on change: a stage built since the last frame
    /// has a fresh renderer, and the substitution is a handful of pairs.
    pub(super) fn refresh_prompts(&mut self) {
        let Some(title) = self.shell.as_ref().map(|shell| shell.title) else {
            return;
        };
        let style = self.settings.controls.prompt_style_value();
        let style = self.prompt_style_override.unwrap_or(style);
        let family = self.controls.prompt_family(style);
        let controls = &self.controls;
        let substitution =
            substitution(title.prompts, family, &|button| controls.bound_keys(button));
        match &mut self.stage {
            Stage::Frontend(stage) => stage.renderer.set_prompt_substitution(substitution),
            Stage::Menu(stage) => stage.renderer.set_prompt_substitution(substitution),
            _ => {}
        }
    }
}
