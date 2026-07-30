//! Runs Wipeout Pulse from the user's own disc image: the composition root.
//!
//! The binary is a thin shell around this library. Everything except the window
//! and the event loop lives here, so the boot sequence, the state machine, the
//! screen model and the headless capture can all be exercised from tests.
//!
//! Boots into `LogoFMV`, which is what the disc's own boot reaches and what
//! plays `Data\Movies\Intro.PMF`: the movie runs straight through, START or X
//! skips it, and then comes the Language Selection screen driven by the disc's
//! own front-end XML. `--reel` boots the other movie state instead - the
//! code-side `Intro Screen->IntroMovie1` with its frame-counted holds, which the
//! disc's boot never enters.
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
pub mod catalogue;
pub mod display;
pub mod font;
pub mod frontend;
pub mod hud;
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
pub mod menu;
pub mod movie;
pub mod perf;
pub mod race;
pub mod render;
pub mod screen;
pub mod settings;
pub mod source;
pub mod sprite;
pub mod state_machine;
pub mod upscale;

/// Frames of the reel `Intro Screen->IntroMovie1` can possibly show, plus one.
///
/// That state stops at frame 260 whatever the movie's length, so the reel leg
/// never needs more than this however long the entry it is pointed at. The
/// `LogoFMV` leg has no such cap and converts everything; `--movie-frames` sets
/// one by hand.
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
