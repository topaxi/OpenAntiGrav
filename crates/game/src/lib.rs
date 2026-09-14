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
pub mod at3;
pub mod audio;
pub mod boot;
pub mod campaign;
pub mod capture;
pub mod catalogue;
pub mod cursor;
pub mod dlc;
pub mod drs;
pub mod endrace;
pub mod hud;
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
pub mod livery;
pub mod loading;
pub mod movie;
pub mod mp3;
pub mod music;
pub mod perf;
pub mod pilots;
pub mod prefetch;
pub mod preview;
pub mod race;
pub mod records;
pub mod remix;
pub mod render;
pub mod scoreboard;
pub mod settings;
pub mod source;
pub mod sprite;
pub mod title;
pub mod upscale;

use log::info;

/// Frames of the reel `Intro Screen->IntroMovie1` can possibly show, plus one.
///
/// That state stops at frame 260 whatever the movie's length, so the reel leg
/// never needs more than this however long the entry it is pointed at. The
/// `LogoFMV` leg has no such cap and converts everything; `--movie-frames` sets
/// one by hand.
pub const INTRO_FRAMES_NEEDED: usize = oag_ui::frontend::FINISH_FRAME + 1;

/// Logs transitions, always for entries and only under `trace` for exits.
///
/// Both at `info` rather than the exits at `debug`: `trace` is `--trace`, and a
/// level that hid what the flag was asked for would make the flag do nothing at
/// the default filter.
pub fn report(events: &[oag_ui::state_machine::Event], trace: bool) {
    for event in events {
        match event {
            oag_ui::state_machine::Event::Enter(name) => info!("-> {name}"),
            oag_ui::state_machine::Event::Exit(name) if trace => info!("<- {name}"),
            oag_ui::state_machine::Event::Exit(_) => {}
        }
    }
}
