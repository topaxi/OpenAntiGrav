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
//! Nothing is written to the disc image, and the only things written anywhere
//! are the movie and audio caches under `data/cache/`; see
//! `docs/architecture/adr/0004-asset-pipeline.md` and
//! `docs/architecture/adr/0019-atrac3plus-out-of-process.md`.

pub mod adapter;
pub mod boot;
pub mod campaign;
pub mod capture;
pub mod cursor;
pub mod endrace;
pub mod flyer;
pub mod ghosts;
pub mod hud_countdown;
pub mod hud_overlay;
pub mod icon;
/// The abstract button layer, which lives in `oag-gameplay` because the
/// simulation owns the input snapshot type and everything that produces one
/// depends on it. Re-exported here so call sites that predate `oag-ui`
/// (`main/*`, and this crate's own modules) still read `crate::input`;
/// `oag-ui` itself names `oag_gameplay::input` directly, since it has no
/// `oag-game` re-export to reach through.
pub use oag_gameplay::input;
/// Keyboard mapping, which lives in `oag-input` for the mirror-image reason: it
/// is a device concern, and the front end is one of its consumers rather than
/// its owner.
pub use oag_input::keys;
pub mod launcher;
pub mod loading;
pub mod medal_watch;
pub mod movie;
pub mod prefetch;
pub mod preview;
pub mod race_capture;
pub mod records;
pub mod remix;
pub mod render;
pub mod scoreboard;
pub mod screen;
pub mod settings;
pub mod sound;
pub mod track_panel;
pub mod unlock;

/// Frames of the reel `Intro Screen->IntroMovie1` can possibly show, plus one.
///
/// That state stops at frame 260 whatever the movie's length, so the reel leg
/// never needs more than this however long the entry it is pointed at. The
/// `LogoFMV` leg has no such cap and converts everything; `--movie-frames` sets
/// one by hand.
pub const INTRO_FRAMES_NEEDED: usize = oag_ui::frontend::FINISH_FRAME + 1;

/// Logs screen transitions: entries at `debug`, and entries and exits both at
/// `info` under `--trace`.
///
/// `info` under the flag rather than `debug`: `--trace` is the request to see
/// these, and a level that hid them at the default filter would make the flag do
/// nothing.
pub fn report(events: &[oag_ui::state_machine::Event], trace: bool) {
    let level = if trace {
        log::Level::Info
    } else {
        log::Level::Debug
    };
    for event in events {
        match event {
            oag_ui::state_machine::Event::Enter(name) => log::log!(level, "-> {name}"),
            oag_ui::state_machine::Event::Exit(name) if trace => log::log!(level, "<- {name}"),
            oag_ui::state_machine::Event::Exit(_) => {}
        }
    }
}
