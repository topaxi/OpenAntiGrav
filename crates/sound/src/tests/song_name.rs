//! The name of the song that is playing: decided where the song is read, stored when the voice
//! starts, and read back by [`Audio::now_playing`] without a listing.

use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::*;
use crate::folders::Reader;
use crate::pool::{MusicFolder, SourceStatus};
use crate::race_music::{Loaded, fetch_track};
use crate::song::{Song, name_song};

fn name(artist: &str, title: &str) -> Option<SongName> {
    Some(SongName {
        artist: Some(artist.to_string()),
        title: title.to_string(),
    })
}

fn soundtrack(seconds: &[f64], names: Vec<Option<SongName>>) -> Soundtrack {
    Soundtrack {
        tracks: seconds
            .iter()
            .enumerate()
            .map(|(at, seconds)| Track {
                at: at as u32,
                seconds: *seconds,
            })
            .collect(),
        names,
    }
}

fn song(artist: &str, title: &str, author: Option<&'static str>) -> Song {
    Song {
        name: name(artist, title).expect("named"),
        author,
        track: None,
    }
}

#[test]
fn a_source_names_the_track_at_its_own_position() {
    let listings = Listings::default();
    listings.seed(
        "psp.chd",
        Platform::Psp,
        soundtrack(
            &[180.0, 190.0],
            vec![name("A", "First"), name("B", "Second")],
        ),
    );
    let named = name_song(
        &listings,
        "psp.chd",
        Platform::Psp,
        1,
        Some("Wipeout Pulse"),
        None,
    );
    assert_eq!(named, Some(song("B", "Second", Some("Wipeout Pulse"))));
}

#[test]
fn a_pressing_that_names_nothing_is_named_by_its_pair_at_the_track_of_the_same_length() {
    let listings = Listings::default();
    // The PSP lists the same recordings in another order, so the index is not the pairing.
    listings.seed(
        "psp.chd",
        Platform::Psp,
        soundtrack(
            &[190.0, 180.0],
            vec![name("B", "Second"), name("A", "First")],
        ),
    );
    listings.seed(
        "ps2.chd",
        Platform::Ps2,
        soundtrack(&[180.004, 190.003], Vec::new()),
    );
    let named = |position| {
        name_song(
            &listings,
            "ps2.chd",
            Platform::Ps2,
            position,
            Some("Wipeout Pulse"),
            Some(("psp.chd", Platform::Psp)),
        )
    };
    assert_eq!(named(0), Some(song("A", "First", Some("Wipeout Pulse"))));
    assert_eq!(named(1), Some(song("B", "Second", Some("Wipeout Pulse"))));
}

#[test]
fn a_track_nothing_names_is_the_disc_and_its_place_in_the_disc() {
    let listings = Listings::default();
    listings.seed(
        "ps2.chd",
        Platform::Ps2,
        soundtrack(&[180.0, 190.0, 200.0], Vec::new()),
    );
    let named = name_song(
        &listings,
        "ps2.chd",
        Platform::Ps2,
        2,
        Some("Wipeout Pulse"),
        None,
    )
    .expect("a locator");
    assert_eq!(named.name.artist.as_deref(), Some("Wipeout Pulse"));
    assert_eq!(named.name.title, "Track 3");
    assert_eq!(named.track, Some(3));
    assert_eq!(
        named.author, None,
        "a locator is not a name the title authored"
    );
    assert!(
        name_song(
            &listings,
            "ps2.chd",
            Platform::Ps2,
            3,
            Some("Wipeout Pulse"),
            None
        )
        .is_none(),
        "no such track"
    );
    assert!(name_song(&listings, "ps2.chd", Platform::Ps2, 2, None, None).is_none());
}

/// A reader of one folder whose `bad` song will not read and whose others are a decodable file.
#[derive(Debug)]
struct Folder {
    files: Vec<PathBuf>,
}

impl Reader for Folder {
    fn list(&self, _root: &Path) -> io::Result<Vec<PathBuf>> {
        Ok(self.files.clone())
    }

    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        if path.to_string_lossy().contains("bad") {
            return Err(io::Error::other("gone"));
        }
        let fixture =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../music/tests/fixtures/sine.flac");
        std::fs::read(fixture)
    }
}

/// Booted Pulse (PSP, three named songs) with a folder of `files` switched on.
fn discs_with_a_folder(files: &[&str]) -> MusicDiscs {
    let pool = crate::pool::Pool::default();
    pool.set_booted("Wipeout Pulse", Platform::Psp, "psp.chd");
    pool.configure(
        &Default::default(),
        &[MusicFolder {
            path: PathBuf::from("/mnt/music"),
            enabled: true,
        }],
    );
    let reader: Arc<dyn Reader> = Arc::new(Folder {
        files: files.iter().map(PathBuf::from).collect(),
    });
    pool.scan_folders(&reader);
    let until = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while pool
        .rows()
        .iter()
        .any(|row| row.status == SourceStatus::Scanning)
    {
        assert!(std::time::Instant::now() < until, "the scan never answered");
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    let listings = Listings::default();
    listings.seed(
        "psp.chd",
        Platform::Psp,
        soundtrack(
            &[180.0, 190.0, 200.0],
            vec![
                name("A", "Booted 0"),
                name("B", "Booted 1"),
                name("C", "Booted 2"),
            ],
        ),
    );
    MusicDiscs {
        pool,
        listings,
        psp: Some("psp.chd".into()),
        booted: Some(Platform::Psp),
        ..MusicDiscs::default()
    }
}

fn fetch(discs: &MusicDiscs, index: usize) -> Loaded {
    fetch_track(discs, MusicSource::Auto, Path::new("unused"), index)
        .expect("a fetch")
        .expect("a song")
}

#[test]
fn a_pool_absolute_index_names_the_pool_song_at_its_own_position() {
    let discs = discs_with_a_folder(&["/mnt/music/one.flac", "/mnt/music/two.flac"]);
    // Index 4 is the pool's second song, past the booted three: the booted listing's names are
    // not consulted at that index (it has none there) or at the pool's own position 1.
    let loaded = fetch(&discs, 4);
    let named = loaded.song.expect("a file is named");
    assert_eq!(named.name.title, "two");
    assert_eq!(named.author, None);
}

#[test]
fn a_pool_song_that_will_not_decode_is_skipped_and_the_one_that_did_is_named() {
    let discs = discs_with_a_folder(&["/mnt/music/a_bad.flac", "/mnt/music/b_good.flac"]);
    // Asked for the first of the pool; the bad file is passed over.
    let loaded = fetch(&discs, 3);
    assert_eq!(loaded.song.expect("named").name.title, "b_good");
}

#[test]
fn a_pool_disc_names_its_songs_from_its_own_listing_at_the_position_it_lists_them() {
    let pool = crate::pool::Pool::default();
    pool.seed_disc(
        "Wipeout HD",
        Platform::Ps3,
        "hd.iso",
        soundtrack(&[100.0, 110.0], Vec::new()).tracks,
    );
    let listings = Listings::default();
    listings.seed(
        "hd.iso",
        Platform::Ps3,
        soundtrack(&[100.0, 110.0], vec![name("X", "Zero"), name("Y", "One")]),
    );
    let extras = pool.extras();
    assert_eq!(extras.len(), 2);
    let crate::pool::Extra::Disc {
        source,
        platform,
        position,
        title,
        ..
    } = &extras[1]
    else {
        panic!("a disc song");
    };
    let named = name_song(&listings, source, *platform, *position, Some(title), None);
    assert_eq!(named, Some(song("Y", "One", Some("Wipeout HD"))));
}

fn loaded(sound: Arc<Sound>, from: Option<Platform>, song: Option<Song>) -> Loaded {
    Loaded {
        from,
        sound,
        what: "a test song".to_string(),
        song,
    }
}

#[test]
fn now_playing_reads_the_stored_name_and_no_listing() {
    let (mut audio, _, race_sound) = psp_boot_fixture();
    let heard = song("A", "Heard", Some("Wipeout Pulse"));
    audio.start_race_voice(
        0,
        loaded(race_sound, Some(Platform::Psp), Some(heard.clone())),
        None,
    );
    // No context, no listing: nothing to look a name up in.
    assert!(audio.race_context.is_none());
    let playing = audio.now_playing().expect("a song started");
    assert_eq!(playing.name, Some(heard.name));
    assert_eq!(playing.author, Some("Wipeout Pulse"));
    assert_eq!((playing.serial, playing.started), (1, 1));
}

#[test]
fn a_cached_track_starts_with_the_song_it_was_cached_with() {
    let (mut audio, ..) = psp_boot_fixture();
    let cached = song("A", "Cached", Some("Wipeout Pulse"));
    let sound = silence(180.0, 2, 44_100);
    audio.race_cache = Some((Platform::Psp, MUSIC_TRACK, sound, Some(cached.clone())));
    audio.start_race_music(
        &MusicDiscs {
            psp: Some("psp.chd".into()),
            booted: Some(Platform::Psp),
            ..MusicDiscs::default()
        },
        MusicSource::Auto,
        Path::new("unused"),
    );
    assert!(audio.race_voice.is_some());
    assert_eq!(
        audio.now_playing().expect("started").name,
        Some(cached.name)
    );
}

/// A live `MUSIC SOURCE` switch restarts the marquee with the other pressing's name, without
/// starting another song; one that lands the same name leaves the marquee alone.
#[test]
fn a_live_source_switch_renames_the_song_and_bumps_the_serial_but_not_the_count() {
    let (mut audio, _, race_sound) = psp_boot_fixture();
    let psp = song("A", "On the PSP", Some("Wipeout Pulse"));
    let ps2 = song("A", "On the PS2", Some("Wipeout Pulse"));
    audio.start_race_voice(
        MUSIC_TRACK,
        loaded(race_sound, Some(Platform::Psp), Some(psp)),
        None,
    );
    let discs = MusicDiscs {
        psp: Some("psp.chd".into()),
        ps2: Some("ps2.chd".into()),
        booted: Some(Platform::Psp),
        ..MusicDiscs::default()
    };
    audio.race_index = Some(MUSIC_TRACK);
    audio.race_cache = Some((
        Platform::Ps2,
        MUSIC_TRACK,
        silence(180.0, 2, 48_000),
        Some(ps2.clone()),
    ));
    audio.set_music_source(&discs, MusicSource::Ps2, Path::new("unused"));
    let playing = audio.now_playing().expect("playing");
    assert_eq!(playing.name, Some(ps2.name.clone()));
    assert_eq!((playing.serial, playing.started), (2, 1));

    // Back onto the PSP with the same name cached: the marquee is not restarted.
    audio.race_cache = Some((
        Platform::Psp,
        MUSIC_TRACK,
        silence(180.0, 2, 44_100),
        Some(ps2.clone()),
    ));
    audio.set_music_source(&discs, MusicSource::Psp, Path::new("unused"));
    let playing = audio.now_playing().expect("playing");
    assert_eq!((playing.serial, playing.started), (2, 1));
}

/// A switch made while the song waits for the light lands the other pressing's name; the release
/// is the start, so the serial is one.
#[test]
fn a_parked_swap_lands_the_new_pressings_name_at_the_release() {
    let (mut audio, ..) = psp_boot_fixture();
    let psp = song("A", "On the PSP", Some("Wipeout Pulse"));
    let ps2 = song("A", "On the PS2", Some("Wipeout Pulse"));
    let discs = MusicDiscs {
        psp: Some("psp.chd".into()),
        ps2: Some("ps2.chd".into()),
        booted: Some(Platform::Psp),
        ..MusicDiscs::default()
    };
    audio.race_cache = Some((
        Platform::Psp,
        MUSIC_TRACK,
        silence(180.0, 2, 44_100),
        Some(psp),
    ));
    audio.hold_race_song(true);
    audio.start_race_music(&discs, MusicSource::Auto, Path::new("unused"));
    assert!(audio.race_voice.is_none(), "parked");
    audio.race_cache = Some((
        Platform::Ps2,
        MUSIC_TRACK,
        silence(180.0, 2, 48_000),
        Some(ps2.clone()),
    ));
    audio.set_music_source(&discs, MusicSource::Ps2, Path::new("unused"));
    assert!(audio.now_playing().is_none(), "held for the light");

    audio.release_race_song();
    let playing = audio.now_playing().expect("started");
    assert_eq!(playing.name, Some(ps2.name));
    assert_eq!((playing.serial, playing.started), (1, 1));
}

/// A song picked on a pause screen starts as a new song when the race resumes, so the now-playing
/// display shows it (`docs/ghidra/functions/psp-pulse-usa/music-list.md`, "Resume starts the pick");
/// the playing song picked again resumes where it was and is not shown again.
#[test]
fn a_paused_pick_resumes_as_a_new_song_and_the_playing_song_does_not() {
    let (mut audio, _, race_sound) = psp_boot_fixture();
    let first = song("A", "First", Some("Wipeout Pulse"));
    let second = song("B", "Second", Some("Wipeout Pulse"));
    let discs = MusicDiscs {
        psp: Some("psp.chd".into()),
        booted: Some(Platform::Psp),
        ..MusicDiscs::default()
    };
    audio.start_race_voice(
        0,
        loaded(race_sound, Some(Platform::Psp), Some(first)),
        None,
    );
    audio.race_index = Some(0);
    let started = |audio: &Audio| {
        let playing = audio.now_playing().expect("a song");
        (playing.serial, playing.started)
    };
    assert_eq!(started(&audio), (1, 1));

    // Paused part way in, the playing song picked again: it resumes, not a new song.
    audio.hold_race_music();
    audio.race_position = 30.0;
    assert!(!audio.pick_race_song(0));
    audio.start_race_music(&discs, MusicSource::Auto, Path::new("unused"));
    assert!(audio.race_voice.is_some());
    assert_eq!(started(&audio), (1, 1));

    // Paused again, another song picked: it starts from its beginning as a new song.
    audio.hold_race_music();
    audio.race_position = 30.0;
    assert!(audio.pick_race_song(1));
    audio.race_cache = Some((
        Platform::Psp,
        1,
        silence(180.0, 2, 44_100),
        Some(second.clone()),
    ));
    audio.start_race_music(&discs, MusicSource::Auto, Path::new("unused"));
    assert_eq!(started(&audio), (2, 2));
    assert_eq!(audio.now_playing().expect("a song").name, Some(second.name));
}
