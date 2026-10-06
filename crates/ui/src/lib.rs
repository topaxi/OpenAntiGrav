//! The front end: boot movies, menus, the HUD's font, and the strings a
//! screen draws.
//!
//! Menus draw, so unlike a gameplay crate this one may reach for a renderer -
//! see `NOT_TITLE_PACKAGES` in `scripts/check-dependency-rules.py` for the
//! argument. Nothing here does today: every type this crate exports is
//! read-only vocabulary (the [`frontend::Draw`] list a title's screens turn
//! into) or a pure state machine, and `oag_game::render` is the wgpu
//! rasteriser that consumes the vocabulary. `oag-game` is the composition
//! root and depends on this crate, never the other way round - see
//! `docs/architecture/workspace-layout.md`.

pub mod anim;
pub mod backdrop;
pub mod font;
pub mod frontend;
pub mod language;
pub mod menu;
pub mod placeholder;
pub mod pointer;
pub mod prompt;
pub mod scene_backdrop;
pub mod screen;
pub mod state_machine;
pub mod strings;
pub mod xml;
