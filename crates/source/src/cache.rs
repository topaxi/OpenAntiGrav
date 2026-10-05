//! Where the caches this project derives from a player's own files live.

use std::path::Path;

/// The default movie cache directory: `data/cache/movies` in a repository
/// checkout, and `<cache dir>/oag/movies` anywhere else - see
/// [`cache_dir_named`] for how the two are told apart.
///
/// Deleting either directory is always safe; see
/// `docs/architecture/adr/0004-asset-pipeline.md`.
#[must_use]
pub fn default_cache_dir() -> std::path::PathBuf {
    cache_dir_named("movies")
}

/// The default decoded-audio directory: `data/cache/audio` in a repository
/// checkout, and `<cache dir>/oag/audio` anywhere else.
///
/// A sibling of [`default_cache_dir`] rather than the same directory, because
/// the two hold different things with different lifetimes - lossless AV1 of a
/// movie, and PCM decoded out of ATRAC3+ - and a player clearing one should not
/// have to re-transcode the other. `--cache` names the movie directory only,
/// which is what its help text has always said. See
/// `docs/architecture/adr/0019-atrac3plus-out-of-process.md`.
#[must_use]
pub fn default_audio_cache_dir() -> std::path::PathBuf {
    cache_dir_named("audio")
}

/// The default unpacked-DLC directory: `data/cache/dlc` in a repository
/// checkout, and `<cache dir>/oag/dlc` anywhere else.
///
/// A third sibling for the same reason the second one exists: what lands here
/// is `PACKn.edat` copied out of a downloaded zip, which is derived from a file
/// the player already has and is therefore always safe to delete. Keeping it
/// out of the movie and audio directories means clearing one cache never costs
/// the others - and, unlike those two, nothing here is transcoded, so a stale
/// entry is cheap to spot: it is a byte copy or it is wrong.
///
/// See [`crate::dlc_cache`].
#[must_use]
pub fn default_dlc_cache_dir() -> std::path::PathBuf {
    cache_dir_named("dlc")
}

/// `data/cache/<what>` in a checkout, `<cache dir>/oag/<what>` anywhere else.
///
/// A checkout is recognised by having a `data/` directory, which is where
/// everything user-supplied already lives and what `just` recipes and
/// `data/README.md` document. A packaged build has no checkout around it, and
/// writing beside wherever it happens to have been run from would scatter a
/// cache through a player's folders - or fail outright, if that is a read-only
/// mount.
fn cache_dir_named(what: &str) -> std::path::PathBuf {
    let checkout = Path::new("data/cache").join(what);
    if Path::new("data").is_dir() {
        return checkout;
    }
    dirs::cache_dir().map_or_else(|| checkout, |cache| cache.join("oag").join(what))
}
