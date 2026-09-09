//! The vocabulary the picture is configured in.
//!
//! One crate for one reason: [`display`]'s types are named by more of this
//! workspace than anything else the composition root held - the race, the
//! settings file, the menus, the upscaler, the dynamic-resolution loop - and a
//! shared vocabulary that lives inside one of its own consumers is not shared,
//! it is borrowed.
//!
//! [`space`] is the same question one level out: what coordinate grid a
//! source's own layouts are written in, and what that grid is displayed as -
//! two numbers that disagree by 7% on the PS2, which is why they are one type
//! and not one ratio. It came down here from `frontend` because
//! [`display::Aspect::Psp`] needs the PSP's panel size to state its ratio, and
//! a front end is the wrong place for the shape of a screen.
//!
//! **Its one workspace dependency is `oag-disc`, for `Platform`, and it must
//! not grow another.** `serde` is the
//! one third-party dependency, because every type here is also a row in
//! `settings.toml`. Every type here is a
//! *question* about the picture - which screen, what shape, how many pixels,
//! how bright - never an answer that needs a GPU to express. That is what lets
//! a settings file, a menu row and a headless test all name the same value
//! without any of them linking a renderer.

pub mod display;
pub mod space;
