//! Runs Wipeout Pulse from the user's own disc image: the composition root.
//!
//! The binary is a thin shell around this library. Everything except the window
//! and the event loop lives here, so the boot sequence, the state machine, the
//! screen model and the headless capture can all be exercised from tests.
//!
//! Boots the way the original does: the intro reel with its frame-counted pauses
//! and two-second holds, START to skip, then the Language Selection screen driven
//! by the disc's own front-end XML.
//!
//! Nothing is written to the disc image, and the only thing written anywhere is
//! the movie cache under `data/cache/`; see
//! `docs/architecture/adr/0004-asset-pipeline.md`.

pub mod boot;
pub mod capture;
pub mod font;
pub mod frontend;
pub mod input;
pub mod keys;
pub mod language;
pub mod movie;
pub mod render;
pub mod screen;
pub mod state_machine;

/// Frames of the reel the intro state can possibly show, plus one.
///
/// The intro stops at frame 260 whatever the movie's length, so converting the
/// whole 40-second intro would spend 235 MiB of cache on eight seconds of screen
/// time. `--full-movie` overrides it.
pub const INTRO_FRAMES_NEEDED: usize = frontend::FINISH_FRAME + 1;

/// Prints transitions, always for entries and only under `trace` for exits.
pub fn report(events: &[state_machine::Event], trace: bool) {
    for event in events {
        match event {
            state_machine::Event::Enter(name) => println!("-> {name}"),
            state_machine::Event::Exit(name) if trace => println!("<- {name}"),
            state_machine::Event::Exit(_) => {}
        }
    }
}
