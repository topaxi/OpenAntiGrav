//! The name of the song that started, decided where the song is read.
//!
//! The name travels with the decoded sound ([`crate::Loaded`]) and is stored when the voice
//! starts, so [`crate::Audio::now_playing`] reads it and no frame ever reads a listing.
//! [`name_song`] is the one place a disc track gets its name (`docs/formats/now-playing.md`,
//! "Names").

use oag_disc::Platform;
use oag_music::song_names::SongName;

use crate::listings::Listings;

/// A song's name, and who authored it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Song {
    pub(crate) name: SongName,
    /// The `Title::name` whose own listing names the song; `None` when no title authored the
    /// name (a file's tags, a file stem, a track's position).
    pub(crate) author: Option<&'static str>,
    /// The 1-based position of a disc track nothing names, for the display to word through its
    /// string table; `name.title` is the English wording of the same.
    pub(crate) track: Option<usize>,
}

/// Names track `position` of `source`'s listing, first by what applies:
///
/// 1. the source's own listing;
/// 2. the `counterpart` pressing's listing, at the track of the same length (the PS2 archive
///    names nothing; its PSP twin does, and the two pair within 11.4 ms on all sixteen tracks);
/// 3. the track's place in the source's own listing, under the title's name, which is the
///    disc's order and not a guessed name.
///
/// `author` is the title that owns `source`. `None` when the listing is not known or, for the
/// last level, when no title owns the source.
pub(crate) fn name_song(
    listings: &Listings,
    source: &str,
    platform: Platform,
    position: usize,
    author: Option<&'static str>,
    counterpart: Option<(&str, Platform)>,
) -> Option<Song> {
    let own = listings.known(source, platform)?;
    if let Some(name) = own.names.get(position).cloned().flatten() {
        return Some(Song {
            name,
            author,
            track: None,
        });
    }
    let paired = counterpart.and_then(|(other_source, other_platform)| {
        let seconds = own.tracks.get(position)?.seconds;
        let other = listings.known(other_source, other_platform)?;
        let (at, _) = other.nearest(seconds)?;
        other.names.get(at).cloned().flatten()
    });
    if let Some(name) = paired {
        return Some(Song {
            name,
            author,
            track: None,
        });
    }
    let number = position + 1;
    own.tracks.get(position)?;
    Some(Song {
        name: SongName {
            artist: Some(author?.to_string()),
            title: format!("Track {number}"),
        },
        author: None,
        track: Some(number),
    })
}

/// A folder file's song: its tags, with the file's stem for a title the tags lack.
pub(crate) fn tagged_song(path: &std::path::Path, tags: oag_music::file::Tags) -> Option<Song> {
    let title = tags.title.or_else(|| {
        path.file_stem()
            .and_then(|stem| stem.to_str())
            .map(str::to_string)
    })?;
    Some(Song {
        name: SongName {
            artist: tags.artist,
            title,
        },
        author: None,
        track: None,
    })
}
