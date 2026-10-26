//! The in-race display of the song that is playing.
//!
//! Pulse draws a small marquee on the race screen each time a song starts: a translucent bar
//! and the text "Artist - song" scrolling through it. The display is authored in the title's
//! front-end XML (an `item` in the `InGame` state) and its strings are the soundtrack's own
//! `PI_Music` nodes; only the timing is in the executable. This type is where a title says so.
//! A title that does not author one has `None` in [`crate::Music::now_playing`].

use crate::DeclaredTracks;
use crate::boot::Provenance;

/// How a title shows the playing song.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NowPlaying {
    /// The `PI_Music` declaration the song's name and artist come from, and the file inside each
    /// declared location. For a title whose [`crate::Music::tracks`] is `Some` this is the same
    /// declaration; for Pulse, whose tracks are found by what the entries are, it is used for
    /// the display alone and the playlist order is left as it was.
    pub names: DeclaredTracks,
    /// The front-end XML entry that holds the layout.
    pub layout: &'static str,
    /// The `item` in the layout that groups the display; its `OffsetX`/`OffsetY` place it.
    pub item: &'static str,
    /// The background `Image` widget inside the item.
    pub bar: &'static str,
    /// The scrolling `Text` widget inside the item.
    pub text: &'static str,
    /// Between the artist and the song name.
    pub separator: &'static str,
    /// Alpha steps (of 255) the text fades by on each update of the race, in and out.
    pub fade_step: u8,
    /// Whole layout units the text moves left on each update of the race.
    pub scroll_step: u8,
    /// Whether the timing was read off the original's executable or inherited.
    pub provenance: Provenance,
}
