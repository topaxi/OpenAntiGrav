//! What the boot assembly's own pieces are asserted to do, with no disc.
//!
//! Its own file rather than a `#[cfg(test)]` block at the end of `boot.rs`:
//! `boot.rs` is a baselined file that may shrink but not grow, so a test
//! module living inside it is a ceiling nobody can spend. See
//! `scripts/check-file-size.py`.

use super::{DEVPUB_REEL, EntryRef, MediaPlan, pulse};

/// The two picture flags reach the loaders, and neither implies the other.
///
/// The wiring rather than the behaviour: `Decode::refresh` is three lines
/// inside [`super::super::movie::transcode`] and its friends, but which
/// `Options` field arrives there is the part that silently rots - a new
/// caller building `Decode` by hand would not fail to compile.
#[test]
fn the_picture_flags_reach_the_movie_loaders_independently() {
    let base = super::Options {
        source: String::new(),
        dlc: Vec::new(),
        language: None,
        leg: oag_ui::frontend::Leg::LogoFmv,
        movie: None,
        cache: std::path::PathBuf::new(),
        audio_cache: std::path::PathBuf::new(),
        extent: crate::movie::Extent::Whole,
        no_video: false,
        refresh_video: false,
        prefer_av1_cache: false,
    };
    assert_eq!(
        base.decode(),
        crate::movie::Decode::default(),
        "the ordinary run decodes the picture and reuses the cache"
    );

    let refreshing = super::Options {
        refresh_video: true,
        ..base.clone()
    };
    assert_eq!(
        refreshing.decode(),
        crate::movie::Decode {
            no_video: false,
            refresh: true,
            prefer_cache: false,
        },
        "--refresh-video must not imply --no-video"
    );

    let cached = super::Options {
        prefer_av1_cache: true,
        ..base.clone()
    };
    assert_eq!(
        cached.decode(),
        crate::movie::Decode {
            no_video: false,
            refresh: false,
            prefer_cache: true,
        },
        "--prefer-av1-cache asks for the cache path, not for a reconversion"
    );

    let silent = super::Options {
        no_video: true,
        ..base
    };
    assert_eq!(
        silent.decode(),
        crate::movie::Decode {
            no_video: true,
            refresh: false,
            prefer_cache: false,
        },
        "--no-video must not imply --refresh-video"
    );
}

/// Each named movie is its picture and its track; the backdrop has no track.
///
/// The denominator of the loading screen's bar, so an off-by-one here is a
/// bar that never reaches its end or reaches it early. Pulse's own plan -
/// both movies and a backdrop - is the five-load case.
#[test]
fn the_media_plan_counts_two_loads_per_movie_and_one_backdrop() {
    let nothing = MediaPlan {
        movie_name: None,
        second_movie_name: None,
        menu_backdrop: None,
    };
    assert_eq!(nothing.loads(), 0, "nothing named is nothing to count");

    let one_reel = MediaPlan {
        movie_name: Some(r"Data\Movies\Intro.PMF".to_string()),
        ..nothing.clone()
    };
    assert_eq!(one_reel.loads(), 2, "the picture and its ATRAC3+ track");

    let backdrop_only = MediaPlan {
        menu_backdrop: Some(r"Data\Movies\Bg.PMF"),
        ..nothing.clone()
    };
    assert_eq!(backdrop_only.loads(), 1, "a backdrop has no track");

    let whole = MediaPlan {
        second_movie_name: Some(r"Data\Movies\IntroMovieP2.PMF"),
        ..MediaPlan {
            menu_backdrop: backdrop_only.menu_backdrop,
            ..one_reel
        }
    };
    assert_eq!(whole.loads(), 5);
}

/// The CLI default names the reel, and the title package holds its hash as a
/// number. Two spellings of one fact can drift silently - nothing would fail,
/// `--reel` would simply address an entry that is not there - so the name is
/// checked against the number rather than trusted to stay in step.
///
/// This used to assert a `hash:` spelling. The name was recovered from the
/// `_<REGION>` suffix rule the localised movie widgets use, so the constant is
/// now what the disc itself calls the file.
#[test]
fn the_reel_default_names_the_entry_the_title_package_hashes() {
    assert_eq!(DEVPUB_REEL, r"Data\Movies\IntroMovieP1_EU.PMF");
    assert_eq!(
        EntryRef::parse(DEVPUB_REEL).hash(),
        pulse::hashes::DEVPUB_REEL_SCEE,
        "the recovered name has to hash to the reel this title ships"
    );
}

#[test]
fn a_name_resolves_through_the_wad_hash() {
    let entry = EntryRef::parse(pulse::names::INTRO_MOVIE);
    assert_eq!(entry, EntryRef::Name(pulse::names::INTRO_MOVIE.to_string()));
    assert_eq!(entry.hash(), 0x71d3_c1ec);
}

#[test]
fn the_default_boot_movie_is_the_one_the_disc_opens() {
    // The regression guard for the reported bug: booting into the dev/pub
    // reel showed Pure-era logo cards, because those three reels ship
    // byte-identically on Wipeout Pure's disc. What LogoFMV names is this.
    assert_eq!(super::DEFAULT_BOOT_MOVIE, r"Data\Movies\Intro.PMF");
    assert_eq!(
        EntryRef::parse(super::DEFAULT_BOOT_MOVIE).hash(),
        0x71d3_c1ec
    );
}

#[test]
fn the_reel_default_is_the_european_cut() {
    assert_eq!(
        EntryRef::parse(super::DEVPUB_REEL).hash(),
        pulse::hashes::DEVPUB_REEL_SCEE
    );
}

#[test]
fn a_hash_addresses_an_entry_with_no_recovered_name() {
    // The SCEA dev/pub reel, which has no name to ask for.
    let entry = EntryRef::parse("hash:3d2c85f8");
    assert_eq!(entry, EntryRef::Hash(0x3d2c_85f8));
    assert_eq!(entry.hash(), 0x3d2c_85f8);
    assert_eq!(entry.to_string(), "hash:3d2c85f8");
    assert_eq!(EntryRef::parse("hash:0x3d2c85f8").hash(), 0x3d2c_85f8);
}

#[test]
fn a_mistyped_hash_stays_a_name_rather_than_matching_something_else() {
    let entry = EntryRef::parse("hash:zzz");
    assert_eq!(entry, EntryRef::Name("hash:zzz".to_string()));
}

#[test]
fn a_name_that_looks_like_a_hash_is_still_a_name() {
    // No prefix, so no hash. Names are never bare hex on these discs, but
    // the rule has to be the prefix rather than the shape.
    assert_eq!(
        EntryRef::parse("3d2c85f8"),
        EntryRef::Name("3d2c85f8".to_string())
    );
}
