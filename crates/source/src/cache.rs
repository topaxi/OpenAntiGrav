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
/// See `crate::dlc_cache`.
#[must_use]
pub fn default_dlc_cache_dir() -> std::path::PathBuf {
    cache_dir_named("dlc")
}

/// `data/cache/<what>` in a checkout, `<cache dir>/oag/<what>` anywhere else.
///
/// A checkout is a folder with a `justfile` and a `data/` directory, or one
/// where `data/cache` already exists (a cache an earlier version put there is
/// kept, not orphaned). A player who only made `data/images/` to drop a disc
/// in is not a checkout: their cache goes to the user cache directory rather
/// than beside their own folders, which a read-only mount would also refuse.
fn cache_dir_named(what: &str) -> std::path::PathBuf {
    let checkout = Path::new("data/cache").join(what);
    if is_checkout(
        Path::new("data/cache").is_dir(),
        Path::new("data").is_dir(),
        Path::new("justfile").is_file(),
    ) {
        return checkout;
    }
    dirs::cache_dir().map_or_else(|| checkout, |cache| cache.join("oag").join(what))
}

fn is_checkout(has_cache: bool, has_data: bool, has_justfile: bool) -> bool {
    has_cache || (has_data && has_justfile)
}

#[cfg(test)]
mod tests {
    use super::is_checkout;

    #[test]
    fn a_players_data_images_folder_alone_is_not_a_checkout() {
        assert!(!is_checkout(false, true, false));
    }

    #[test]
    fn a_repository_with_data_and_an_old_cache_are_checkouts() {
        assert!(is_checkout(false, true, true));
        assert!(is_checkout(true, true, false));
    }
}
