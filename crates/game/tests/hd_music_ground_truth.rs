//! Validates Wipeout HD / Fury's soundtrack against its real disc.
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
//! HD's music is addressed by **name**, like Pure's and unlike Pulse's, and
//! every part of the name came off the disc: the decrypted `EBOOT.elf` spells
//! out `%s\%s_stereo%s` and `Data\Music\FEMusic\frontend%d_stereo.mp3`, and
//! `/data/plugins/frontend/definition.xml` declares one `PI_Music` node per
//! track. Each of those is a measurement, so each is asserted here.
//!
//! It is also the only place the **MP3 half** of `oag_game::music`'s container
//! dispatch is exercised against real streams. A synthetic MPEG frame is not
//! something a decoder will open, so the unit tests beside that module can only
//! assert what is *not* MPEG; the case that matters is here.
//!
//! # One image, and no pressing to compare against
//!
//! Unlike Pure, only one HD image is in the corpus. Where `pure_music_ground_truth`
//! cross-checks two pressings, this cross-checks two *routes* instead: what the
//! listing reports against what the declaration says, both read off the same
//! disc.

use std::path::{Path, PathBuf};

use oag_disc::Platform;
use oag_game::{audio, catalogue, music};

/// The decrypted HD/Fury image, if it is there.
///
/// Only the decrypted one: a PS3 disc reads as noise until layer 1 is
/// decrypted, so the encrypted image beside it is not a fallback. Skipped
/// rather than failed when absent, the way the other ground-truth tests do it.
fn image() -> Option<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/images/hdfury-ps3-eu-dec.iso");
    if path.exists() {
        return Some(path);
    }
    assert!(
        std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
        "OAG_REQUIRE_GAME_DATA is set but {} is missing",
        path.display()
    );
    println!("skipping: {} not present", path.display());
    None
}

/// What `oag_game::music` reports for the disc.
fn listing(image: &Path) -> Vec<music::Entry> {
    music::listing(&image.display().to_string())
        .expect("reading the soundtrack listing")
        .expect("a disc of a title this build knows")
}

/// How many tracks HD's front-end plugin declares.
const DECLARED: usize = 15;

/// `symphonia` reads the disc's own MP3s, and reads them the way the file
/// itself is written.
///
/// The first thing to establish, because everything else here assumes it:
/// these are 48 kHz stereo, and the stream's own declared frame count is what
/// states a length. Pinned on one track so that a decoder upgrade which starts
/// reporting zero-length streams fails here rather than in a load report
/// nobody reads.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_discs_own_mp3s_read_as_48_khz_stereo_with_a_declared_length() {
    let Some(image) = image() else { return };
    let mut archives = oag_hd::open(&image.display().to_string()).expect("opening the disc");
    let blob = archives
        .read_name(r"Data\Music\Gingy\music_stereo.mp3")
        .expect("reading one declared track");

    let stream = oag_game::mp3::describe(&blob).expect("an MPEG stream");
    assert_eq!(stream.channels, 2);
    assert_eq!(stream.sample_rate, 48_000);
    let seconds = stream.seconds.expect("a declared frame count");
    assert!((seconds - 325.1).abs() < 0.5, "{seconds} s");
}

/// And it decodes, in process, to the length it declares.
///
/// The claim `oag_game::mp3`'s header makes - that no `ffmpeg` is involved for
/// this codec - is only worth anything if the decode actually happens, so this
/// runs one. It is the slowest test here and the one that would catch a
/// `symphonia` feature set that resolves but cannot decode.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_declared_track_decodes_in_process_to_the_length_it_declares() {
    let Some(image) = image() else { return };
    let mut archives = oag_hd::open(&image.display().to_string()).expect("opening the disc");
    let blob = archives
        .read_name(r"Data\Music\Gingy\music_stereo.mp3")
        .expect("reading one declared track");

    let pcm = oag_game::mp3::decode(&blob).expect("decoding it");
    assert_eq!(pcm.channels, 2);
    assert_eq!(pcm.sample_rate, 48_000);
    let seconds = pcm.samples.len() as f64 / f64::from(pcm.channels) / f64::from(pcm.sample_rate);
    assert!(
        (seconds - 325.1).abs() < 1.0,
        "decoded to {seconds} s against the 325.1 s it declares"
    );
}

/// The count, through the declared route.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn hd_carries_fifteen_soundtrack_tracks() {
    let Some(image) = image() else { return };
    assert_eq!(listing(&image).len(), DECLARED);
}

/// **Not one declaration is skipped.**
///
/// `music::declared` drops a `PI_Music` whose audio is absent or unreadable,
/// which is right - a declaration is only a declaration - and is also exactly
/// how a wrong `MUSIC_TRACK_FILE`, a renamed directory or a PSARC lookup that
/// stopped folding backslashes would go unnoticed. So the listing is asserted
/// against the *declaration count read off the same disc*, not against a number
/// written down here.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn every_declared_track_resolves_to_a_real_entry() {
    let Some(image) = image() else { return };
    let tracks = oag_hd::MUSIC.tracks.expect("HD declares its soundtrack");
    let mut archives = oag_hd::open(&image.display().to_string()).expect("opening the disc");
    let definition = String::from_utf8(
        archives
            .read_name(tracks.declared_in)
            .expect("reading the front-end plugin definition"),
    )
    .expect("the definition is UTF-8");

    let declared = catalogue::music(&definition);
    assert_eq!(declared.len(), DECLARED, "declarations");
    assert_eq!(
        listing(&image).len(),
        declared.len(),
        "every declared track must resolve to a playable entry"
    );

    // The join itself, on the disc rather than on a fixture. This is also what
    // says a `Data\Music\...` name reaches a `/data/music/...` PSARC entry:
    // the two differ in slash and in case, and `oag_assets::psarc` folds both.
    for track in &declared {
        let name = track.entry_name(tracks.file);
        assert!(
            archives.locate(&name).is_some(),
            "{name} is declared and absent"
        );
    }
}

/// Every soundtrack track is a real length rather than a zero.
///
/// The failure this guards is specific and silent: a listing whose entries all
/// resolve but all measure zero would still be fifteen entries long, and would
/// play as fifteen tracks of nothing.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn every_listed_track_has_a_plausible_length() {
    let Some(image) = image() else { return };
    for (index, entry) in listing(&image).iter().enumerate() {
        assert!(
            (60.0..600.0).contains(&entry.seconds),
            "track {index} is {} s",
            entry.seconds
        );
    }
}

/// The front end's own music resolves, and is **not** one of the fifteen.
///
/// The second half is what stops `MusicSource` ever moving it. Asserted over
/// the declared *names* rather than over `Entry::at`, which is an index into
/// the declaration and would compare two unrelated numbers.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_front_ends_own_music_resolves_and_is_not_a_soundtrack_track() {
    let Some(image) = image() else { return };
    let name = oag_hd::MUSIC.front_end;
    let tracks = oag_hd::MUSIC.tracks.expect("HD declares its soundtrack");
    let mut archives = oag_hd::open(&image.display().to_string()).expect("opening the disc");
    assert!(archives.locate(name).is_some(), "{name} is absent");

    let definition = String::from_utf8(
        archives
            .read_name(tracks.declared_in)
            .expect("reading the front-end plugin definition"),
    )
    .expect("the definition is UTF-8");
    assert!(
        catalogue::music(&definition)
            .iter()
            .all(|track| track.entry_name(tracks.file) != name),
        "the menu loop must not be one of the fifteen"
    );
}

/// The front end's music loads **through the path the engine would use**, and
/// not merely through a name that resolves.
///
/// The only thing that reaches `music::load_front_end` for an HD source. HD's
/// `Title::front_end` is `None`, so `Audio::start_music` is never called on an
/// HD boot and the race playlist is all a player hears - which makes this
/// branch unreachable in practice and a test the only thing that can exercise
/// it. Without this, "HD's front-end music works" would rest on `locate`
/// returning `Some`, which is true of any real entry.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_front_ends_music_loads_through_the_engines_own_path() {
    let Some(image) = image() else { return };
    let cache = std::env::temp_dir().join("oag-hd-music-ground-truth");
    let (name, sound) = music::load_front_end(&image.display().to_string(), &cache)
        .expect("loading the front end's music")
        .expect("HD names one");

    assert_eq!(name, oag_hd::MUSIC.front_end);
    assert!(
        (sound.seconds() - 170.7).abs() < 1.0,
        "the base stereo cut is {} s",
        sound.seconds()
    );
}

/// All four front-end candidates are on the disc, and the two the base/Fury
/// axis chooses between are **different pieces of music**.
///
/// The size gap is the evidence, and it is why picking one unconditionally is
/// recorded as a limitation rather than passed over: stereo against surround is
/// one recording in two channel counts, but base against Fury is not one
/// recording at all. See `oag_hd::names::FRONT_END_MUSIC_VARIANTS`.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn all_four_front_end_variants_exist_and_the_fury_one_is_different_music() {
    let Some(image) = image() else { return };
    let mut archives = oag_hd::open(&image.display().to_string()).expect("opening the disc");

    let mut seconds = Vec::new();
    for (axis, name) in oag_hd::names::FRONT_END_MUSIC_VARIANTS {
        assert!(archives.locate(name).is_some(), "{axis}: {name} is absent");
        let blob = archives.read_name(name).expect("reading a variant");
        let stream = oag_game::mp3::describe(&blob).expect("an MPEG stream");
        seconds.push((*axis, stream.seconds.expect("a declared frame count")));
    }

    let base = seconds
        .iter()
        .find(|(axis, _)| *axis == "base stereo")
        .expect("the base variant")
        .1;
    let fury = seconds
        .iter()
        .find(|(axis, _)| *axis == "fury stereo")
        .expect("the fury variant")
        .1;
    assert!(
        (base - fury).abs() > 30.0,
        "base is {base} s and fury is {fury} s - if these ever converge, the \
         'different music' claim in oag_hd::names needs re-measuring"
    );
}

/// The listing is in the **declaration's** order.
///
/// Asserted by reading the same disc twice through the two routes and matching
/// them up: entry *n* of the listing is the length of declared track *n*. That
/// is the whole claim `Entry::at` being a declaration index rests on, and a
/// listing that quietly reordered would still pass every count assertion above.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_listing_is_in_the_declared_order() {
    let Some(image) = image() else { return };
    let tracks = oag_hd::MUSIC.tracks.expect("HD declares its soundtrack");
    let mut archives = oag_hd::open(&image.display().to_string()).expect("opening the disc");
    let definition = String::from_utf8(
        archives
            .read_name(tracks.declared_in)
            .expect("reading the front-end plugin definition"),
    )
    .expect("the definition is UTF-8");

    let listing = listing(&image);
    for (index, track) in catalogue::music(&definition).iter().enumerate() {
        let blob = archives
            .read_name(&track.entry_name(tracks.file))
            .expect("reading a declared track");
        let declared = oag_game::mp3::describe(&blob)
            .expect("an MPEG stream")
            .seconds
            .expect("a declared frame count");
        assert!(
            (listing[index].seconds - declared).abs() < 0.01,
            "listing entry {index} is {} s where the declaration's is {declared} s",
            listing[index].seconds
        );
    }
}

/// MUSIC SOURCE stays inert on an HD boot, because there is no second release
/// of Wipeout HD for it to choose between.
///
/// And the load report says which disc it found, rather than "neither release" -
/// the arm that would otherwise lie while the music played.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn an_hd_boot_reports_its_own_disc_and_offers_no_choice() {
    let Some(image) = image() else { return };
    let source = image.display().to_string();
    let discs = audio::MusicDiscs::survey(&source);

    assert_eq!(discs.booted(), Some(Platform::Ps3));
    assert!(!discs.both(), "{}", discs.describe());
    assert!(
        discs.describe().contains("PS3"),
        "the load report must name the disc it found: {}",
        discs.describe()
    );
}
