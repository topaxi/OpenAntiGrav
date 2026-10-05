//! Validates Wipeout Pure's soundtrack against its real discs.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! ```
//!
//! # What this is for
//!
//! Pure's music is addressed by **name**, not by a population argument: its
//! `BOOT.BIN` spells out `Data\Music\frontend.at3` for the front end and joins
//! `%s\%s` with `music.at3` for a soundtrack track, and its own
//! `Data\Plugins\PI001\Definition.xml` declares one `PI_Music` node per track
//! naming the directory. Every one of those claims is a measurement, so every
//! one is asserted here.
//!
//! The load-bearing assertion is
//! [`every_declared_track_resolves_to_a_real_entry`]: a name that stops
//! resolving is exactly the failure a `Vec` of nineteen silently becoming a
//! `Vec` of twelve would hide, because playback would carry on sounding fine.
//!
//! # Both pressings, and Pulse as the control
//!
//! Pure ships two pressings and they must agree, since the names come out of one
//! executable. Pulse runs through the same entry points as a **control**: it
//! declares its sixteen the same way but this build does not read them yet, so
//! it must still be found by what its entries are and must still come back
//! sixteen. See `oag_title::Music::tracks`.

use std::path::{Path, PathBuf};

use oag_disc::Platform;
use oag_game::sound::GameLibrary;
use oag_sound::{self as audio, library};

/// Every Pure pressing present, as `(label, path)`.
///
/// A missing image is skipped rather than failed, the way the other
/// ground-truth tests do it, unless `OAG_REQUIRE_GAME_DATA` says the caller
/// expects them.
fn images(names: &[(&'static str, &str)]) -> Vec<(&'static str, PathBuf)> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut found = Vec::new();
    for (label, name) in names {
        let path = root.join(name);
        if path.exists() {
            found.push((*label, path));
        } else {
            assert!(
                std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
                "OAG_REQUIRE_GAME_DATA is set but {} is missing",
                path.display()
            );
            println!("skipping: {} not present", path.display());
        }
    }
    found
}

fn pure_images() -> Vec<(&'static str, PathBuf)> {
    images(&[
        ("pure-psp-eu", "data/images/pure-psp-eu.chd"),
        ("pure-psp-usa", "data/images/pure-psp-usa.chd"),
    ])
}

/// What `oag_sound::library` reports for one source.
fn listing(image: &Path) -> Vec<oag_music::Entry> {
    library::listing(&GameLibrary, &image.display().to_string())
        .expect("reading the soundtrack listing")
        .expect("a PSP disc of a title this build knows")
}

/// How many tracks Pure's own plugin definition declares.
const DECLARED: usize = 19;

/// The count, on both pressings and through the declared route.
///
/// Nineteen to Pulse's sixteen, which is the difference
/// `oag_sound`'s pairing test leans on to refuse a Pure disc as Pulse's
/// counterpart.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn pure_carries_nineteen_soundtrack_tracks() {
    for (label, image) in pure_images() {
        assert_eq!(listing(&image).len(), DECLARED, "{label}");
    }
}

/// **Not one declaration is skipped.**
///
/// `music::declared` drops a `PI_Music` whose audio is absent or is not a
/// stereo 44,100 Hz stream, which is right - a declaration is only a
/// declaration - and is also exactly how a wrong `track_file`, a changed hash
/// or a renamed directory would go unnoticed. So the listing is asserted
/// against the *declaration count read off the same disc*, not against a
/// number written down here.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn every_declared_track_resolves_to_a_real_entry() {
    let tracks = oag_pure::MUSIC
        .tracks
        .expect("Pure declares its soundtrack");
    for (label, image) in pure_images() {
        let mut archives = oag_pure::open(&image.display().to_string()).expect("opening the disc");
        let definition = String::from_utf8(
            archives
                .read_name(tracks.declared_in)
                .expect("reading the plugin definition"),
        )
        .expect("the definition is UTF-8");

        let declared = oag_music::playlist::music(&definition);
        assert_eq!(declared.len(), DECLARED, "{label}: declarations");
        assert_eq!(
            listing(&image).len(),
            declared.len(),
            "{label}: every declared track must resolve to a playable entry"
        );

        // The join itself, on the disc rather than on a fixture: each declared
        // location plus the title package's own file name is an entry that is
        // really there.
        for track in &declared {
            let name = track.entry_name(tracks.file);
            assert!(
                archives.locate(&name).is_some(),
                "{label}: {name} is declared and absent"
            );
        }
    }
}

/// The order is the **disc's**, not the archive directory's.
///
/// Pure's longest track is 326 s where every other is between 205 and 229, and
/// the declaration puts it first. In `Data.wad` offset order it is not first,
/// so this one assertion separates the two orderings - which is the whole
/// reason the declared route is preferred over finding the entries by what they
/// are.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_listing_is_in_the_declared_order_and_not_the_archives() {
    for (label, image) in pure_images() {
        let listing = listing(&image);
        let longest = listing
            .iter()
            .map(|entry| entry.seconds)
            .fold(f64::MIN, f64::max);
        assert!((longest - 326.1).abs() < 0.1, "{label}: {longest} s");
        assert!(
            (listing[0].seconds - longest).abs() < f64::EPSILON,
            "{label}: the disc declares its longest track first, and this is \
             what says the order came from the declaration"
        );
    }
}

/// The two pressings carry the same recordings, one for one.
///
/// The same test `oag_sound` applies to a candidate counterpart disc,
/// turned on the pair that really is one release twice. It is what says the
/// names recovered from the USA executable describe the EU pressing too.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn both_pressings_list_the_same_soundtrack() {
    let found = pure_images();
    let [(_, eu), (_, usa)] = found.as_slice() else {
        println!("skipping: needs both pressings");
        return;
    };
    let (eu, usa) = (listing(eu), listing(usa));
    assert_eq!(eu.len(), usa.len());
    for (index, (eu, usa)) in eu.iter().zip(&usa).enumerate() {
        assert!(
            (eu.seconds - usa.seconds).abs() < 1.0,
            "track {index}: {} s against {} s",
            eu.seconds,
            usa.seconds
        );
    }
}

/// The front end's own music is the **short loop** the name promises, and is
/// not one of the nineteen.
///
/// Locating the name is not enough on its own: a hash that resolved to some
/// other entry would still locate, and would still not be one of the nineteen,
/// so both of those assertions pass on a wrong-but-present entry. The length is
/// what pins it - 40.1 s, against 205-326 s for every soundtrack track - and it
/// is the number that would change silently if the name ever resolved
/// elsewhere. Pulse's counterpart is 29.5 s; see `oag_pulse::MUSIC`.
///
/// Not being one of the nineteen is the separate claim that stops `MusicSource`
/// ever moving it: the row is scoped to the tracks alone.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_front_ends_own_music_is_a_short_loop_and_not_a_soundtrack_track() {
    let name = oag_pure::MUSIC
        .front_end
        .expect("Pure names its front-end music");
    for (label, image) in pure_images() {
        let mut archives = oag_pure::open(&image.display().to_string()).expect("opening the disc");
        assert!(archives.locate(name).is_some(), "{label}: {name} is absent");

        let at3 = archives.read_name(name).expect("reading the menu loop");
        let seconds = oag_music::at3::describe(&at3)
            .expect("a RIFF/WAVE")
            .seconds()
            .expect("a fact chunk");
        assert!(
            (seconds - 40.1).abs() < 0.1,
            "{label}: the menu loop is {seconds} s"
        );

        // Over the declared **names** rather than over the listing's tokens:
        // `Entry::at` is an index into the declaration now, so comparing it to
        // a name hash would compare two unrelated numbers and pass for that
        // reason rather than for the right one.
        let tracks = oag_pure::MUSIC
            .tracks
            .expect("Pure declares its soundtrack");
        let definition = String::from_utf8(
            archives
                .read_name(tracks.declared_in)
                .expect("reading the plugin definition"),
        )
        .expect("the definition is UTF-8");
        assert!(
            oag_music::playlist::music(&definition)
                .iter()
                .all(|track| track.entry_name(tracks.file) != name),
            "{label}: the menu loop must not be one of the nineteen"
        );
    }
}

/// MUSIC SOURCE stays inert on a Pure boot, because there is no Pure PS2
/// release for it to choose between.
///
/// The one place a title-aware survey could go wrong silently: with the Pulse
/// deny-list no longer stopping a Pure disc dead, nothing but the same-title
/// check stops `pure-psp-eu.chd` being offered as `pure-psp-usa.chd`'s
/// counterpart, and both really do carry nineteen matching lengths.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_pure_boot_finds_no_counterpart_release() {
    for (label, image) in pure_images() {
        let discs = audio::MusicDiscs::survey(&image.display().to_string(), &GameLibrary);
        assert_eq!(
            discs.booted(),
            Some(Platform::Psp),
            "{label}: the boot is identified even though it is not Pulse"
        );
        assert!(
            !discs.both(),
            "{label}: {}",
            discs.describe() // names what it wrongly paired with
        );
    }
}

/// The control: Pulse is unaffected by any of this.
///
/// It declares its sixteen in the same schema and this build still does not
/// read them, so the population route has to keep returning sixteen - and the
/// survey has to keep finding the PS2 release when it is there.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn pulse_is_still_found_by_what_its_entries_are() {
    assert!(
        oag_pulse::MUSIC.tracks.is_none(),
        "this test is the record of that choice, so it fails when it changes"
    );
    for (label, image) in images(&[
        ("pulse-psp-eu", "data/images/pulse-psp-eu.chd"),
        ("pulse-psp-usa", "data/images/pulse-psp-usa.chd"),
    ]) {
        assert_eq!(listing(&image).len(), 16, "{label}");
    }
}
