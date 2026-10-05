//! Which entries of a disc hold music, found by opening a source as whichever
//! title it turned out to be.
//!
//! The reading and decoding live in [`oag_music`]; this module is the half that
//! has to open a source, which needs [`crate::title::open_source`] and so cannot
//! move out of the composition root.

use std::path::Path;

use anyhow::{Context, Result};
use oag_assets::Archives;
use oag_audio::Sound;
use oag_disc::Platform;
use oag_music::Entry;
use oag_title::Title;

/// A source opened as whichever title it turned out to be, for a music read.
///
/// `None` rather than an error for every reason a source is not a disc of a
/// title this knows - a partial extract, a path that will not open. All of them
/// mean "no soundtrack here", which is what the caller does something with;
/// distinguishing them would give it nothing to do differently.
///
/// **The PS2 release is excluded here and not by accident**: its music is loose
/// in the filesystem rather than in an archive, so `crate::audio` reads it
/// through its own `PS2MUSIC.WAD` path and never arrives in this module.
fn open_archived(source: &str) -> Option<(&'static Title, Archives)> {
    // Resolved through the layout rather than a literal `PSP_GAME/USRDIR/...`
    // path, so a directory somebody extracted with `oag-unpack` answers the
    // same as a disc image does.
    let opened = crate::title::open_source(source, Vec::new(), Vec::new()).ok()?;
    matches!(
        opened.archives.layout.platform,
        Platform::Psp | Platform::Ps3 | Platform::Vita | Platform::Ps4
    )
    .then_some((opened.title, opened.archives))
}

/// The soundtrack `source` carries, in the order that source's own title
/// decides.
///
/// `Ok(None)` when the source is not a disc of a title this knows, and
/// `Ok(Some(vec![]))` when it is one whose music could not be found at all -
/// two different answers, because the first means "ask something else" and the
/// second means "this disc has none".
///
/// # Errors
///
/// An archive that opens and then will not read its own directory.
pub fn listing(source: &str) -> Result<Option<Vec<Entry>>> {
    let Some((title, mut archives)) = open_archived(source) else {
        return Ok(None);
    };
    oag_music::listing(title, &mut archives).map(Some)
}

/// Reads one soundtrack entry of `source` and decodes it; see
/// [`oag_music::load_entry`].
///
/// # Errors
///
/// A source that will not open, an entry that will not read, and a decode that
/// failed.
pub fn load_entry(source: &str, at: u32, cache_dir: &Path) -> Result<Sound> {
    let (title, mut archives) =
        open_archived(source).with_context(|| format!("opening {source} as a known title"))?;
    oag_music::load_entry(title, &mut archives, at, cache_dir)
}

/// The front end's own music, decoded, and the name it came off; see
/// [`oag_music::load_front_end`].
///
/// `Ok(None)` when the source is not a disc of a title this knows, or is one
/// that holds no such entry - an ordinary outcome, not an error.
///
/// # Errors
///
/// An entry that is not a readable stream, or a decode that failed.
pub fn load_front_end(source: &str, cache_dir: &Path) -> Result<Option<(String, Sound)>> {
    let Some((title, mut archives)) = open_archived(source) else {
        return Ok(None);
    };
    oag_music::load_front_end(title, &mut archives, cache_dir)
}
