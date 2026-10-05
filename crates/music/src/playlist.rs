//! The soundtrack a title's plugin definition declares.

use oag_tables::fexml::{Node, parse};

/// One track of the soundtrack, as the plugin declares it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Music {
    /// The plugin's own id. **This one is a title, not a folder id**, unlike
    /// `Track::id` and `Team::id` in `oag-game`'s catalogue - the disc spells the piece of music out
    /// here rather than keying a string table with it, which is why nothing in
    /// this repository may repeat it. Read at run time, printed at most in a
    /// load report.
    pub id: String,
    /// The directory the track lives in, e.g. `Data\Music\SomeArtist`.
    pub location: String,
}

impl Music {
    /// The archive entry name of the audio this track plays.
    ///
    /// `file` is [`oag_title::DeclaredTracks::file`] - `MusicManager.cpp`'s own
    /// `%s\%s` join, the title package supplying the second `%s`. It is a
    /// parameter rather than a constant because a title whose soundtrack this
    /// build has not recovered by name has no answer to give.
    #[must_use]
    pub fn entry_name(&self, file: &str) -> String {
        format!(r"{}\{file}", self.location)
    }
}

/// Reads every `PI_Music` out of a plugin definition, in file order.
///
/// File order on purpose, and it is the one thing this buys over finding the
/// same entries by what they *are*: it is the order the original's own
/// `MusicSelection` screen walks, so "the first track" means something the disc
/// decides rather than something the archive directory happens to do.
///
/// The `Artist` and `Label` an entry also carries are deliberately **not**
/// read, the same way `teams` skips `PI_TeamModel`: nothing displays them,
/// and a public field holding shipped text that no caller reads is worse than
/// one that appears when it is needed.
#[must_use]
pub fn music(definition_xml: &str) -> Vec<Music> {
    let root = parse(definition_xml);
    let mut out = Vec::new();
    collect_music(&root, &mut out);
    out
}

fn collect_music(node: &Node, out: &mut Vec<Music>) {
    for child in &node.children {
        if child.name == "PI_Music" {
            if let Some(track) = read_music(child) {
                out.push(track);
            }
        } else {
            collect_music(child, out);
        }
    }
}

fn read_music(node: &Node) -> Option<Music> {
    let id = node.attr("name")?.to_string();
    let values = node.children_named("Values").next()?;
    Some(Music {
        id,
        location: values.attr("location")?.to_string(),
    })
}

#[cfg(test)]
mod tests;
