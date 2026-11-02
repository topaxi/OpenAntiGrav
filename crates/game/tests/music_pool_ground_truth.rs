//! The music pool lists every other soundtrack this machine can open, and the playlist
//! draws from the ones switched on.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content (`just test-data`).
//!
//! Booted on Pulse PSP: Wipeout HD and Wipeout Pure are listed from their own discs, the Pulse
//! PS2 release (the booted title's other pressing, `MUSIC SOURCE`'s business) is not, and with
//! the default switches nothing beyond the booted soundtrack is in the playlist. Switching HD
//! and Pure on adds exactly each title's own soundtrack, once, though Pure is on two pressings
//! here. Listings only: nothing is decoded.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use oag_game::sound::GameLibrary;
use oag_sound::library::{Library, Opened};
use oag_sound::pool::SourceStatus;
use oag_sound::{self as audio, folders};

#[derive(Debug)]
struct Workspace;

impl Library for Workspace {
    fn open(&self, source: &str) -> Option<Opened> {
        GameLibrary.open(source)
    }

    fn containers(&self) -> Vec<PathBuf> {
        let images = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/images");
        let mut found: Vec<PathBuf> = std::fs::read_dir(images)
            .into_iter()
            .flatten()
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| path.is_file() && oag_source::source::is_container(path))
            .collect();
        found.sort();
        found
    }

    fn sources(&self) -> Vec<PathBuf> {
        let extracted = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/extracted");
        let mut found = self.containers();
        found.extend(
            ["vita", "ps4"]
                .map(|name| extracted.join(name))
                .into_iter()
                .filter(|dir| dir.is_dir()),
        );
        found
    }
}

#[test]
#[ignore = "needs data/images/ (just test-data)"]
fn two_titles_soundtracks_both_appear_in_the_pool() {
    let Some(pulse) = oag_testdata::image("pulse-psp-eu.chd") else {
        return;
    };
    let Some(_hd) = oag_testdata::image("hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let discs = audio::MusicDiscs::survey_in_background(&pulse.display().to_string(), &Workspace);
    let reader: Arc<dyn folders::Reader> = Arc::new(folders::Disk::default());
    discs.start_pool(&Default::default(), &[], &reader);
    let until = Instant::now() + Duration::from_secs(300);
    while discs.pool().scanning_discs() {
        assert!(Instant::now() < until, "the disc scan never finished");
        std::thread::sleep(Duration::from_millis(50));
    }

    let rows = discs.pool().rows();
    let labelled = |name: &str| {
        rows.iter()
            .filter(|row| row.id.contains(&slug(name)))
            .count()
    };
    assert_eq!(
        labelled(oag_pulse::TITLE.name),
        1,
        "only the booted Pulse: {rows:#?}"
    );
    assert!(
        labelled(oag_hd::TITLE.name) >= 1,
        "HD's disc is listed: {rows:#?}"
    );
    // 2048 and Omega are extracted directories, not disc images: the pool reads them through
    // the host's own source list (`Library::sources`), so their soundtracks are listed too.
    for (dir, id) in [("vita", "2048"), ("ps4", "omega")] {
        let extracted = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/extracted");
        if extracted.join(dir).is_dir() {
            assert!(
                rows.iter().any(|row| row.id.contains(id)
                    && matches!(row.status, SourceStatus::Ready(songs) if songs > 0)),
                "{id} is extracted and lists songs: {rows:#?}"
            );
        }
    }
    assert!(
        rows.iter().filter(|row| row.enabled).count() == 1 && rows[0].enabled,
        "by default only the booted disc is on: {rows:#?}"
    );
    assert_eq!(
        audio::Audio::playlist_len_for_test(&discs, 16),
        16,
        "with defaults the playlist is the booted soundtrack"
    );

    for row in &rows {
        if let SourceStatus::Ready(songs) = row.status
            && !row.enabled
        {
            discs.pool().set_enabled(&row.id, true);
            assert!(songs > 0, "{} lists no songs", row.label);
        }
    }
    let on = discs.pool().rows();
    let per_title = |name: &str| -> usize {
        on.iter()
            .filter(|row| row.id.contains(&slug(name)))
            .filter_map(|row| match row.status {
                SourceStatus::Ready(n) => Some(n),
                _ => None,
            })
            .max()
            .unwrap_or(0)
    };
    let (hd, pure) = (
        per_title(oag_hd::TITLE.name),
        per_title(oag_pure::TITLE.name),
    );
    assert!(hd > 0, "HD contributes songs");
    let extras = audio::Audio::playlist_len_for_test(&discs, 0);
    assert!(
        extras >= hd,
        "HD's soundtrack is in the pool: {extras} songs from {on:#?}"
    );
    if pure > 0 {
        assert_eq!(
            extras,
            on.iter()
                .map(|row| match row.status {
                    SourceStatus::Ready(n) if !row.id.contains(&slug(oag_pulse::TITLE.name)) => n,
                    _ => 0,
                })
                .sum::<usize>()
                - duplicate_pressings(&on),
            "a title on two pressings contributes once"
        );
    }
}

fn slug(name: &str) -> String {
    name.to_lowercase().replace(' ', "-")
}

/// Songs counted twice by summing every row: the smaller pressing of each title listed twice.
fn duplicate_pressings(rows: &[oag_sound::pool::SourceRow]) -> usize {
    let mut seen: Vec<(String, usize)> = Vec::new();
    let mut doubled = 0;
    for row in rows {
        let SourceStatus::Ready(n) = row.status else {
            continue;
        };
        if row.id.contains(&slug(oag_pulse::TITLE.name)) {
            continue;
        }
        let title = row.id.split(':').nth(1).unwrap_or_default().to_string();
        if seen.iter().any(|(t, _)| *t == title) {
            doubled += n;
        } else {
            seen.push((title, n));
        }
    }
    doubled
}
