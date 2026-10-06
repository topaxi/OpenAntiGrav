//! Which glyph a button prompt draws as, for the controller the player holds.
//!
//! The table here is **chosen, not measured**: PromptFont's own glyph for each
//! control on each family of pad, by the position of the control (the pad
//! layer maps the south button to cross whatever the pad prints on it, see
//! `oag_input::pad::map_button`). A Nintendo pad's south button is `B` and its
//! east one `A`, the reverse of an Xbox pad's; the keyboard family shows the
//! key the player has bound to the button.
//!
//! The title's own stand-in codepoints and what each depicts are
//! [`oag_title::prompts::Prompts`]; the art and the string rewrite are
//! [`oag_ui::prompt`]; this joins them to a pad family and a binding table.

use oag_gameplay::input::Button;
use oag_input::prompt::{PromptFamily, PromptStyle};
use oag_title::prompts::{Prompt, Prompts};
use oag_ui::prompt::{Substitution, art_index};

/// The abstract button a prompt glyph depicts.
#[must_use]
pub fn button_of(prompt: Prompt) -> Button {
    match prompt {
        Prompt::Cross => Button::Cross,
        Prompt::Circle => Button::Circle,
        Prompt::Square => Button::Square,
        Prompt::Triangle => Button::Triangle,
        Prompt::L => Button::L,
        Prompt::R => Button::R,
        Prompt::Up => Button::Up,
        Prompt::Down => Button::Down,
        Prompt::Left => Button::Left,
        Prompt::Right => Button::Right,
    }
}

/// The PromptFont code-name for a key's name as `oag_input::keys` spells it.
fn key_glyph(name: &str) -> String {
    match name {
        "UP" | "DOWN" | "LEFT" | "RIGHT" | "ENTER" | "BACKSPACE" | "SPACE" | "TAB" => {
            format!("keyboard-{}", name.to_ascii_lowercase())
        }
        letter if letter.len() == 1 => format!("keyboard-{}", letter.to_ascii_lowercase()),
        _ => "keyboard-key".to_string(),
    }
}

/// The PromptFont glyph for `prompt` on `family`, by code-name.
///
/// `key` is the name of the key bound to the prompt's button, for the
/// keyboard family; a button with no key bound shows PromptFont's blank key.
#[must_use]
pub fn glyph_name(prompt: Prompt, family: PromptFamily, key: Option<&str>) -> String {
    let fixed = |name: &str| name.to_string();
    match (family, prompt) {
        (PromptFamily::Keyboard, _) => key_glyph(key.unwrap_or("")),
        (_, Prompt::Up) => fixed("dpad-up"),
        (_, Prompt::Down) => fixed("dpad-down"),
        (_, Prompt::Left) => fixed("dpad-left"),
        (_, Prompt::Right) => fixed("dpad-right"),
        (PromptFamily::Playstation, Prompt::Cross) => fixed("sony-a"),
        (PromptFamily::Playstation, Prompt::Circle) => fixed("sony-b"),
        (PromptFamily::Playstation, Prompt::Square) => fixed("sony-x"),
        (PromptFamily::Playstation, Prompt::Triangle) => fixed("sony-y"),
        (PromptFamily::Playstation, Prompt::L) => fixed("sony-left-shoulder"),
        (PromptFamily::Playstation, Prompt::R) => fixed("sony-right-shoulder"),
        (PromptFamily::Xbox, Prompt::Cross) => fixed("xbox-a"),
        (PromptFamily::Xbox, Prompt::Circle) => fixed("xbox-b"),
        (PromptFamily::Xbox, Prompt::Square) => fixed("xbox-x"),
        (PromptFamily::Xbox, Prompt::Triangle) => fixed("xbox-y"),
        (PromptFamily::Xbox, Prompt::L) => fixed("xbox-left-shoulder"),
        (PromptFamily::Xbox, Prompt::R) => fixed("xbox-right-shoulder"),
        // The south button is labelled B, the east one A; PromptFont has no
        // Nintendo-coloured face glyphs, so the lettered discs stand in.
        (PromptFamily::Nintendo, Prompt::Cross) => fixed("xbox-b"),
        (PromptFamily::Nintendo, Prompt::Circle) => fixed("xbox-a"),
        (PromptFamily::Nintendo, Prompt::Square) => fixed("xbox-y"),
        (PromptFamily::Nintendo, Prompt::Triangle) => fixed("xbox-x"),
        (PromptFamily::Nintendo, Prompt::L) => fixed("nintendo-left-shoulder"),
        (PromptFamily::Nintendo, Prompt::R) => fixed("nintendo-right-shoulder"),
    }
}

/// The substitution a title's prompts draw with on `family`, or none for the
/// disc's own glyphs (`family` `None`).
///
/// `bound_keys` answers which keys the keyboard currently binds to a button;
/// only the keyboard family reads it.
#[must_use]
pub fn substitution(
    prompts: &Prompts,
    family: Option<PromptFamily>,
    bound_keys: &dyn Fn(Button) -> Vec<&'static str>,
) -> Substitution {
    let Some(family) = family else {
        return Substitution::none();
    };
    let mut out = Substitution::none();
    for (index, (_, prompt)) in prompts.glyphs.iter().enumerate() {
        let key = bound_keys(button_of(*prompt)).into_iter().next();
        let name = glyph_name(*prompt, family, key);
        out = out.with(prompts, index, art_index(&name));
    }
    out
}

/// The family a run with no device to watch draws: only a family forced by
/// `style` counts; `auto` has seen no input, so it is the disc's own.
#[must_use]
pub fn forced_family(style: PromptStyle) -> Option<PromptFamily> {
    match style {
        PromptStyle::Family(family) => Some(family),
        PromptStyle::Auto | PromptStyle::Original => None,
    }
}

#[cfg(test)]
mod tests;
