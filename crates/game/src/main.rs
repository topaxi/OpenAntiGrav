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
//! This file is only the process entry: everything else, from reading the
//! command line to the window, is [`oag_game::app`].

/// The counting allocator behind `oag-render`'s off-by-default `perf-probe`
/// feature, and the reason it is installed here rather than in a test: it has
/// to be the process's allocator to see the frame path's allocations at all.
///
/// Absent entirely without the feature - not merely inert - so a default build
/// keeps the system allocator with no wrapper in front of it.
#[cfg(feature = "perf-probe")]
#[global_allocator]
static ALLOCATOR: oag_gpu::perfprobe::Counting = oag_gpu::perfprobe::Counting;

fn main() -> anyhow::Result<()> {
    oag_game::app::main()
}
