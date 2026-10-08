//! How `Cell Selection` colours and cases its text, as RPCS3 draws it.
//!
//! The screen authors every one of these `color="FEGlobals->TitleColor"`
//! (`0xFF646464`), and the settled frame of `09_blitzed`'s first cell shows
//! two things the XML does not say: the headings (`EVENT TYPE`, `LAPS`,
//! `EVENT 01/08`) at `150,150,150` and the values under them (`SINGLE RACE`,
//! `3`, `NONE`) at `255,255,255`; and the four emblem values and the event
//! counter in capitals where the string table holds `Single Race`/`Venom`.
//! **Measured on one frame, confidence 60**: which native state drives the
//! colour (a selected or focused state, a role per widget) is unread, so the
//! rule here is by widget name, and only for text that authors `TitleColor`.

use oag_ui::frontend::{Align, Draw};
use oag_ui::screen::{Text, argb_to_rgba};

use super::super::Layout;
use super::super::draw::text_draw;

/// `FEGlobals->TitleColor`, as `skin.xml` authors it.
const TITLE_COLOR: u32 = 0xFF64_6464;

/// A heading's colour in the frame: `150,150,150`.
const HEADING: u32 = 0xFF96_9696;

/// A value's colour in the frame: white.
const VALUE: u32 = 0xFFFF_FFFF;

/// The texts that are headings: everything else that authors `TitleColor`
/// is a value.
fn is_heading(name: &str) -> bool {
    (name.ends_with(" Title") && name != "EPoints Title") || name == "GridNum" || name == "EventNum"
}

/// The texts the frame draws in capitals.
fn is_capitalised(name: &str) -> bool {
    matches!(
        name,
        "GridNum" | "EventNum" | "Event" | "Track" | "Speed Class" | "Weapons"
    )
}

/// `text`'s draw on `Cell Selection`.
pub(super) fn draw(text: &Text, content: &str, layout: &Layout, recolour: bool) -> Draw {
    let name = text.name.as_deref().unwrap_or("");
    let mut text = text.clone();
    if recolour && text.color == TITLE_COLOR {
        text.color = if is_heading(name) { HEADING } else { VALUE };
    }
    let content = if is_capitalised(name) {
        content.to_uppercase()
    } else {
        content.to_string()
    };
    // `font="Title"` is the bold face the page's own title uses; `text_draw`
    // has no role for it and would draw the menu face, which is what made
    // `EVENT 01/08` small and light beside the frame's.
    if text.font.eq_ignore_ascii_case("title") {
        return Draw::in_role(
            Some(oag_ui::language::roles::TITLE),
            text.x,
            text.y,
            text.scale,
            argb_to_rgba(text.color),
            Align::Left,
            content,
        );
    }
    text_draw(&text, &content, layout)
}

#[cfg(test)]
mod tests;
