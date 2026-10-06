//! How a source string becomes an opened title, supplied by the host.
//!
//! Turning `"data/images/foo.chd"` into "this is Wipeout HD, and these are its
//! archives" needs every title package and the directories a player keeps disc
//! images in, neither of which belongs in the sound crate. The host implements
//! [`Library`] and this crate asks it, as `oag_music` asks its caller to open
//! archives. The three functions below are all the music path does with an
//! opened source: read its soundtrack listing, decode one entry, decode the
//! front end's loop.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use oag_assets::Archives;
use oag_audio::Sound;
use oag_disc::Platform;
use oag_title::Title;

/// A source opened as a title this build knows.
#[derive(Debug)]
pub struct Opened {
    /// Which title the source is.
    pub title: &'static Title,
    /// Its archives, laid out as that title's platform ships them.
    pub archives: Archives,
}

/// What the sound crate needs of the host to find and open a title's music.
pub trait Library: std::fmt::Debug + Send + Sync {
    /// Opens `source` as whichever title it is, or [`None`] for a source that
    /// is not any title this build knows.
    fn open(&self, source: &str) -> Option<Opened>;

    /// Every disc image the host would search for a counterpart release, in
    /// search order (alphabetical within a directory, so two runs agree).
    fn containers(&self) -> Vec<PathBuf>;
}

/// A [`Library`] that knows no source: a [`crate::MusicDiscs`] built without a
/// survey, which has no discs to open anyway.
#[derive(Debug, Clone, Copy)]
pub(crate) struct NoLibrary;

impl Library for NoLibrary {
    fn open(&self, _source: &str) -> Option<Opened> {
        None
    }

    fn containers(&self) -> Vec<PathBuf> {
        Vec::new()
    }
}

fn open_archived(library: &dyn Library, source: &str) -> Option<(&'static Title, Archives)> {
    let opened = library.open(source)?;
    matches!(
        opened.archives.layout.platform,
        Platform::Psp | Platform::Ps3 | Platform::Vita | Platform::Ps4
    )
    .then_some((opened.title, opened.archives))
}

/// The soundtrack a source's archives list, or [`None`] for a source that is
/// not an archived title.
///
/// # Errors
///
/// An archive that opens and then will not parse.
pub fn listing(library: &dyn Library, source: &str) -> Result<Option<Vec<oag_music::Entry>>> {
    let Some((title, mut archives)) = open_archived(library, source) else {
        return Ok(None);
    };
    oag_music::listing(title, &mut archives).map(Some)
}

/// Decodes one soundtrack entry of `source` to PCM.
///
/// # Errors
///
/// A source that is not a known archived title, or an entry that will not
/// decode.
pub fn load_entry(library: &dyn Library, source: &str, at: u32, cache_dir: &Path) -> Result<Sound> {
    let (title, mut archives) = open_archived(library, source)
        .with_context(|| format!("opening {source} as a known title"))?;
    oag_music::load_entry(title, &mut archives, at, cache_dir)
}

/// Decodes the front end's own music loop, named alongside the sound.
///
/// # Errors
///
/// A loop that is authored and will not decode.
pub fn load_front_end(
    library: &dyn Library,
    source: &str,
    cache_dir: &Path,
) -> Result<Option<(String, Sound)>> {
    let Some((title, mut archives)) = open_archived(library, source) else {
        return Ok(None);
    };
    oag_music::load_front_end(title, &mut archives, cache_dir)
}
