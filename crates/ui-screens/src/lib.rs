//! The front end's individual screens, built on [`oag_ui`]'s vocabulary:
//! the campaign map and its flyer, the end-of-race screens, the garage and
//! track pickers, the name-entry and confirm prompts, the ticker marquee and
//! the track panel.
//!
//! `oag-ui` holds the lower core a screen is made of (the `Frontend` draw
//! list, `menu`, `screen`, `language`, `pointer`, `font`); nothing in it
//! reaches up into this crate. Like `oag-ui` this crate draws nothing itself
//! and is classified in `NOT_TITLE_PACKAGES` in
//! `scripts/check-dependency-rules.py`: no gameplay crate may depend on it.

pub mod campaign;
pub mod endrace;
pub mod marquee;
pub mod picker;
pub mod prompt;
pub mod tag_entry;
pub mod track_panel;
