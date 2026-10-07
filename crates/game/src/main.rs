//! Runs Wipeout Pulse from the user's own disc image.
//!
//! ```sh
//! oag-game data/images/pulse-psp-eu.chd
//! ```
//!
//! Boots into `LogoFMV`, the screen the disc's own boot reaches: it plays
//! `Data\Movies\Intro.PMF` straight through, START or X skips it, and then the
//! Language Selection screen driven
//! by the disc's own front-end XML. Arrow keys move, Return or X selects, which
//! fires `Launch Game` - and `Launch Game` loads a track and a ship and hands the
//! window over to a race, in the same window and on the same GPU device.
//!
//! ```sh
//! # No display needed. Runs the sequence headless and writes one frame.
//! oag-game data/images/pulse-psp-eu.chd --screenshot /tmp/menu.png \
//!     --until "Language Selection" --hold start
//! ```
//!
//! `--race` is the shortcut into the second half: the same race, without booting
//! the front end first.
//!
//! ```sh
//! oag-game --race
//! oag-game --race --screenshot /tmp/race.png --ticks 600 --hold cross
//! ```
//!
//! Named no image at all, it looks for one - and if the search path holds
//! several, it says so on screen instead of picking one quietly:
//!
//! ```sh
//! oag-game            # one image found: boots it, as it always did
//! oag-game --launcher # the chooser, whatever is there
//! ```
//!
//! See [`oag_game::launcher`] and `just launch`.
//!
//! This file is only `main` itself: read the command line, resolve everything
//! that can fail before anything is loaded, and hand off to one of the runs in
//! [`headless`] or to the window in [`app`]. **"Everything that can fail" no
//! longer includes the source on the windowed route**: which disc image a run
//! opens may be a screen away, so the load that depends on it lives in
//! [`prepare`] and runs either here or from `Session::finish_launcher`. The
//! rest of the binary is the modules declared below; everything that can be
//! tested without a GPU is in [`oag_game`] rather than in any of them.

// The body is shared with the Android library (`android.rs`, the `oag_android`
// example in Cargo.toml) by `include!` rather than by pointing two targets at
// this one file, which Cargo warns about on every build. The rustdoc header
// above stays here because an included file may not carry inner attributes.
include!("main_body.rs");
