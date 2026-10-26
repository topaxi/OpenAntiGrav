//! What a soundtrack entry is called: the artist and song name the plugin definition's
//! `PI_Music` node carries, for the in-race now-playing display.

use anyhow::Result;
use oag_assets::Archives;
use oag_title::Title;

use crate::Entry;

/// One song as the disc names it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SongName {
    /// The node's `Artist`, when it carries one.
    pub artist: Option<String>,
    /// The node's own `name`, which is the song title.
    pub title: String,
}

impl SongName {
    /// The text the display scrolls: `Artist<separator>Title`, or the title alone when the
    /// node has no artist.
    #[must_use]
    pub fn text(&self, separator: &str) -> String {
        match &self.artist {
            Some(artist) => format!("{artist}{separator}{}", self.title),
            None => self.title.clone(),
        }
    }
}

/// The name of each of `entries`, parallel to it. `None` for an entry the title's
/// declaration does not name, and for every entry when the title authors no display.
///
/// A title that declares its tracks ([`oag_title::Music::tracks`]) numbers an entry by its
/// position among the nodes that resolve; Pulse, whose entries are found by what they are,
/// is matched by the name hash of the node's audio file, so its playlist order is untouched.
///
/// # Errors
///
/// Never today; a `Result` so a title that reads its names from a bank can fail.
pub fn names(
    title: &Title,
    archives: &mut Archives,
    entries: &[Entry],
) -> Result<Vec<Option<SongName>>> {
    let Some(music) = title.music else {
        return Ok(vec![None; entries.len()]);
    };
    let Some(now_playing) = music.now_playing else {
        return Ok(vec![None; entries.len()]);
    };
    let declared = now_playing.names;
    if let Some(state_tracks) = music.state_tracks {
        return Ok(state_names(archives, state_tracks.declared_in, entries));
    }
    let nodes = crate::declared_nodes(archives, declared);
    let name_of = |node: &crate::playlist::Music| SongName {
        artist: node.artist.clone(),
        title: node.id.clone(),
    };
    if music.tracks.is_some() {
        return Ok(entries
            .iter()
            .map(|entry| nodes.get(entry.at as usize).map(name_of))
            .collect());
    }
    Ok(entries
        .iter()
        .map(|entry| {
            nodes
                .iter()
                .find(|node| {
                    oag_formats::wad::hash_name(&node.entry_name(declared.file)) == entry.at
                })
                .map(name_of)
        })
        .collect())
}

/// A soundtrack addressed by Wwise state: an entry's token is its `PI_Music` location number
/// ([`Entry::at`]), so the node is the one that declares that location.
fn state_names(
    archives: &mut Archives,
    declared_in: &str,
    entries: &[Entry],
) -> Vec<Option<SongName>> {
    let nodes = archives
        .read_name(declared_in)
        .ok()
        .and_then(|blob| oag_tables::fexml::text(&blob).ok())
        .map(|xml| crate::playlist::music(&xml))
        .unwrap_or_default();
    entries
        .iter()
        .map(|entry| {
            nodes
                .iter()
                .find(|node| node.location.parse() == Ok(entry.at))
                .map(|node| SongName {
                    artist: node.artist.clone(),
                    title: node.id.clone(),
                })
        })
        .collect()
}
