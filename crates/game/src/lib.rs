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
//! Picking a language fires `Launch Game`, and `Launch Game` starts a [`race`]: a
//! track and a ship loaded off the same disc, the simulation stepped at a fixed
//! 60 Hz from the keyboard, drawn from the chase camera the ship's own data
//! describes. The two share the disc access, the timestep, the keyboard and - in
//! the binary - one window and one GPU device; they share no drawing code, because
//! a ribbon and a menu have nothing in common but a surface. `--race` is the same
//! race entered without booting the front end first.
//!
//! Nothing is written to the disc image, and the only thing written anywhere is
//! the movie cache under `data/cache/`; see
//! `docs/architecture/adr/0004-asset-pipeline.md`.

pub mod boot;
pub mod capture;
pub mod font;
pub mod frontend;
/// The abstract button layer, which lives in `oag-gameplay` because the
/// simulation owns the input snapshot type and everything that produces one
/// depends on it. Re-exported here so the front end's own call sites read the
/// same as they did when it was a module of this crate.
pub use oag_gameplay::input;
/// Keyboard mapping, which lives in `oag-input` for the mirror-image reason: it
/// is a device concern, and the front end is one of its consumers rather than
/// its owner.
pub use oag_input::keys;
pub mod language;
pub mod movie;
pub mod race;
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
