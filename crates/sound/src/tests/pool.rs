//! The music pool: folder scans that never block their caller, and a playlist that is
//! today's booted soundtrack until another source is switched on.

use std::io;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, channel};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use super::*;
use crate::folders::{FolderScan, Reader, Status};
use crate::pool::{MusicFolder, Pool, SourceStatus};
use crate::race_music::RacePick;

/// A reader whose `list` blocks until the test releases it: a mount that hangs.
#[derive(Debug)]
struct Hung {
    gate: Mutex<Receiver<()>>,
    files: Vec<PathBuf>,
}

impl Reader for Hung {
    fn list(&self, _root: &Path) -> io::Result<Vec<PathBuf>> {
        // Waits for the release; a dropped sender releases it too, so no thread outlives the test.
        let _ = self.gate.lock().expect("gate").recv();
        Ok(self.files.clone())
    }

    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        Err(io::Error::other(format!("{} is gone", path.display())))
    }
}

/// A reader that answers at once.
#[derive(Debug)]
struct Quick(Result<Vec<PathBuf>, String>);

impl Reader for Quick {
    fn list(&self, _root: &Path) -> io::Result<Vec<PathBuf>> {
        self.0.clone().map_err(io::Error::other)
    }

    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        Err(io::Error::other(format!("{} is gone", path.display())))
    }
}

fn wait_for(mut done: impl FnMut() -> bool) {
    let until = Instant::now() + Duration::from_secs(10);
    while !done() {
        assert!(Instant::now() < until, "the worker never answered");
        std::thread::sleep(Duration::from_millis(2));
    }
}

#[test]
fn a_hung_mount_does_not_block_the_caller_and_times_out() {
    let (release, gate) = channel();
    let reader: Arc<dyn Reader> = Arc::new(Hung {
        gate: Mutex::new(gate),
        files: vec![PathBuf::from("/mnt/a.mp3")],
    });
    let began = Instant::now();
    let scan = FolderScan::start_within(reader, PathBuf::from("/mnt"), Duration::from_millis(30));
    assert_eq!(scan.status(), Status::Scanning);
    assert!(
        began.elapsed() < Duration::from_secs(1),
        "starting a scan waited for the mount"
    );
    wait_for(|| scan.status() == Status::TimedOut);
    assert!(scan.files().is_empty());
    // The mount answers late: the scan recovers rather than staying timed out.
    release.send(()).expect("release");
    wait_for(|| scan.status() == Status::Ready(1));
    assert_eq!(scan.files(), vec![PathBuf::from("/mnt/a.mp3")]);
}

#[test]
fn a_folder_that_will_not_read_says_why() {
    let reader: Arc<dyn Reader> = Arc::new(Quick(Err("no such mount".to_string())));
    let scan = FolderScan::start(reader, PathBuf::from("/gone"));
    wait_for(|| matches!(scan.status(), Status::Failed(_)));
    assert_eq!(scan.status(), Status::Failed("no such mount".to_string()));
}

#[test]
fn only_mp3_files_are_songs() {
    assert!(crate::folders::is_song(Path::new("a/b/Song.MP3")));
    assert!(!crate::folders::is_song(Path::new("a/b/cover.jpg")));
    assert!(!crate::folders::is_song(Path::new("a/b/noextension")));
}

fn folder_pool(files: &[&str], enabled: bool) -> (Pool, PathBuf) {
    let pool = Pool::default();
    let path = PathBuf::from("/mnt/music");
    pool.configure(
        &Default::default(),
        &[MusicFolder {
            path: path.clone(),
            enabled,
        }],
    );
    let reader: Arc<dyn Reader> = Arc::new(Quick(Ok(files.iter().map(PathBuf::from).collect())));
    pool.scan_folders(&reader);
    wait_for(|| {
        pool.rows()
            .iter()
            .all(|row| row.status != SourceStatus::Scanning)
    });
    (pool, path)
}

#[test]
fn with_only_the_booted_disc_on_the_playlist_is_the_booted_soundtrack() {
    // A configured folder that is off adds nothing: the playlist is the booted
    // soundtrack's length and the pick wraps exactly as it always did.
    let (pool, _) = folder_pool(&["/mnt/music/a.mp3", "/mnt/music/b.mp3"], false);
    assert_eq!(crate::pool::playlist_len(&pool, 16), 16);
    let discs = MusicDiscs {
        pool,
        booted: Some(Platform::Psp),
        psp: Some("psp.chd".into()),
        ..MusicDiscs::default()
    };
    assert_eq!(RacePick::after(15).resolve_for_test(&discs, 16), 0);
    assert_eq!(RacePick::after(3).resolve_for_test(&discs, 16), 4);
}

#[test]
fn an_enabled_folder_extends_the_playlist_and_a_disabled_booted_disc_leaves_it() {
    let (pool, path) = folder_pool(&["/mnt/music/a.mp3", "/mnt/music/b.mp3"], true);
    assert_eq!(crate::pool::playlist_len(&pool, 16), 18);
    let id = format!("folder:{}", path.display());
    pool.set_enabled(&id, false);
    assert_eq!(crate::pool::playlist_len(&pool, 16), 16);
    pool.set_enabled(&id, true);
    pool.set_booted("Wipeout Pulse", Platform::Psp, "psp.chd");
    let booted = pool.rows().into_iter().next().expect("the booted row").id;
    pool.set_enabled(&booted, false);
    assert_eq!(
        crate::pool::playlist_len(&pool, 16),
        2,
        "with the booted disc off the playlist is the folder alone"
    );
}

#[test]
fn a_song_past_the_booted_soundtrack_comes_from_the_pool_and_a_vanished_mount_is_not_fatal() {
    let (pool, _) = folder_pool(&["/mnt/music/a.mp3"], true);
    let discs = MusicDiscs {
        pool,
        booted: Some(Platform::Psp),
        psp: Some("psp.chd".into()),
        ..MusicDiscs::default()
    };
    // With the booted disc off, index 0 is the folder's song; its file is gone (the fake
    // reader refuses), which comes back as an error naming the file, not a panic or silence
    // elsewhere in the playlist.
    discs.pool.set_booted("Wipeout Pulse", Platform::Psp, "psp.chd");
    let booted = discs.pool.rows().into_iter().next().expect("booted").id;
    discs.pool.set_enabled(&booted, false);
    let error = crate::race_music::fetch_track(&discs, MusicSource::Auto, Path::new("unused"), 0)
        .expect_err("a vanished file");
    assert!(
        format!("{error:#}").contains("a.mp3"),
        "the error names the file: {error:#}"
    );
}

#[test]
fn a_cached_walk_keeps_the_listing_and_follows_a_change_below_it() {
    let base = std::env::temp_dir().join(format!("oag-music-folder-{}", std::process::id()));
    let (library, cache) = (base.join("library"), base.join("cache"));
    std::fs::create_dir_all(library.join("sub")).expect("library");
    std::fs::write(library.join("a.mp3"), b"").expect("a");
    std::fs::write(library.join("sub/b.mp3"), b"").expect("b");
    std::fs::write(library.join("cover.jpg"), b"").expect("cover");
    let disk = crate::folders::Disk::cached_in(cache.clone());
    let sorted = |mut files: Vec<PathBuf>| {
        files.sort();
        files
    };
    let first = sorted(disk.list(&library).expect("first walk"));
    assert_eq!(first, vec![library.join("a.mp3"), library.join("sub/b.mp3")]);
    assert!(
        std::fs::read_dir(&cache).expect("cache dir").count() == 1,
        "the walk kept its cache"
    );
    // A new song two levels down moves only its own folder's time; the walk still finds it.
    std::thread::sleep(Duration::from_millis(30));
    std::fs::write(library.join("sub/c.mp3"), b"").expect("c");
    let second = sorted(disk.list(&library).expect("second walk"));
    assert_eq!(
        second,
        vec![
            library.join("a.mp3"),
            library.join("sub/b.mp3"),
            library.join("sub/c.mp3")
        ]
    );
    assert!(disk.list(&base.join("missing")).is_err(), "a missing folder is an error");
    std::fs::remove_dir_all(&base).expect("clean up");
}
