//! [`Options`]: what to load and how much of the movie to convert.
//!
//! Split out of `boot.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change.

/// What to load and how much of the movie to convert.
#[derive(Debug, Clone)]
pub struct Options {
    /// A disc image, or a directory extracted with `oag-unpack`.
    pub source: String,
    /// Directories to look in for [downloadable content](crate::dlc), mounted
    /// behind `source`'s own archives and independent of which release it is.
    pub dlc: Vec<std::path::PathBuf>,
    /// Which movie leg the sequence boots into.
    pub leg: oag_ui::frontend::Leg,
    /// The language to load the string table for, by the XML's own English
    /// name, when one has been chosen on an earlier run.
    ///
    /// `None` falls back to English and then to whatever the source lists
    /// first, which is what a first run gets. A name this source does not carry
    /// falls back the same way rather than failing.
    pub language: Option<String>,
    /// Which `Data.wad` entry that leg plays, as a name or a name hash.
    ///
    /// Three of the disc's movies have no recovered name, and one of them is
    /// the reel `Intro Screen->IntroMovie1`'s frame counters describe, so a name
    /// is not enough to address every candidate. See [`super::EntryRef`].
    ///
    /// `None` is not "no movie" - it means "this leg's own default", resolved
    /// in [`super::load`] once the source's title is known (`screens` is parsed by
    /// then). Before this existed the default was baked into `Options` at
    /// construction, in `main.rs`, which is exactly where the title is *not*
    /// yet known - that was fine while every source was Pulse and wrong the
    /// moment Pure could open at all, since Pulse's default names an entry
    /// Pure does not have. `Some(name)` is a request, same as always, and is
    /// used verbatim regardless of title.
    pub movie: Option<String>,
    /// Where converted frames are cached.
    pub cache: std::path::PathBuf,
    /// Where decoded PCM is cached - see [`super::default_audio_cache_dir`].
    ///
    /// A second directory rather than the one above, and deliberately: the two
    /// hold different things with different lifetimes, and a player clearing
    /// one should not lose the other.
    pub audio_cache: std::path::PathBuf,
    /// How much of the movie to convert.
    pub extent: crate::movie::Extent,
    /// Skip conversion entirely.
    pub no_video: bool,
    /// Convert every movie again even when the cache already holds it, and
    /// overwrite what is there. See [`crate::movie::Decode::refresh`].
    pub refresh_video: bool,
    /// Take the AV1 cache path even where a platform decoder is available. See
    /// [`crate::movie::Decode::prefer_cache`].
    pub prefer_av1_cache: bool,
}

impl Options {
    /// How the movie loaders should open a picture, from these options.
    pub(super) fn decode(&self) -> crate::movie::Decode {
        crate::movie::Decode {
            no_video: self.no_video,
            refresh: self.refresh_video,
            prefer_cache: self.prefer_av1_cache,
        }
    }
}
