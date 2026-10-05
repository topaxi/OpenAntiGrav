//! Music objects over every bank of the Omega Collection's base and patch.
//!
//! **`#[ignore]`d and never run in CI**; needs `data/extracted/ps4`.

use std::path::{Path, PathBuf};

use oag_formats::wwise::music::{RanSeq, Segment, Switch, track_clips, track_type};
use oag_formats::wwise::{Bank, Kind, Library};

fn archive_path(dir: &str, name: &str) -> Option<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/extracted/ps4")
        .join(dir)
        .join("uroot")
        .join(name);
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

#[test]
#[ignore = "needs data/extracted/ps4"]
fn explore() {
    for (dir, name) in [
        ("omega-eu", "data00.psarc"),
        ("omega-eu-patch", "data05.psarc"),
        ("omega-eu-patch", "data08.psarc"),
    ] {
        let Some(path) = archive_path(dir, name) else {
            return;
        };
        let mut archive = oag_assets::psarc::Archive::open_file(&path).unwrap();
        let paths: Vec<String> = archive
            .paths()
            .iter()
            .filter(|p| p.ends_with(".bnk"))
            .cloned()
            .collect();
        let blobs: Vec<Vec<u8>> = paths
            .iter()
            .map(|p| archive.read_path(p).unwrap())
            .collect();
        let banks: Vec<Bank> = blobs.iter().map(|b| Bank::parse(b).unwrap()).collect();
        let (mut seg, mut sw, mut rs, mut tr, mut bad) = (0, 0, 0, 0, 0);
        let mut types = std::collections::BTreeMap::new();
        for (p, bank) in paths.iter().zip(&banks) {
            for o in bank.objects() {
                let body = bank.body(o);
                match o.kind {
                    Kind::MusicSegment => {
                        seg += 1;
                        if let Err(e) = Segment::parse(o.kind, body) {
                            bad += 1;
                            println!("{p} seg {:#x}: {e}", o.id);
                        }
                    }
                    Kind::MusicSwitch => {
                        sw += 1;
                        if let Err(e) = Switch::parse(o.kind, body) {
                            bad += 1;
                            println!("{p} sw {:#x}: {e}", o.id);
                        }
                    }
                    Kind::MusicRanSeq => {
                        rs += 1;
                        if let Err(e) = RanSeq::parse(o.kind, body) {
                            bad += 1;
                            println!("{p} rs {:#x}: {e}", o.id);
                        }
                    }
                    Kind::MusicTrack => {
                        tr += 1;
                        let _ = track_clips(body).unwrap();
                        *types.entry(track_type(body)).or_insert(0) += 1;
                    }
                    _ => {}
                }
            }
        }
        println!("{name}: seg {seg} sw {sw} rs {rs} tracks {tr} bad {bad} types {types:?}");
        let _ = Library::new(banks);
    }
}

#[test]
#[ignore = "needs data/extracted/ps4"]
fn walk() {
    let Some(path) = archive_path("omega-eu-patch", "data08.psarc") else {
        return;
    };
    let mut archive = oag_assets::psarc::Archive::open_file(&path).unwrap();
    let paths: Vec<String> = archive.paths().to_vec();
    let bank_paths: Vec<&String> = paths.iter().filter(|p| p.ends_with(".bnk")).collect();
    let blobs: Vec<Vec<u8>> = bank_paths
        .iter()
        .map(|p| archive.read_path(p).unwrap())
        .collect();
    let banks: Vec<Bank> = blobs.iter().map(|b| Bank::parse(b).unwrap()).collect();
    let lib = Library::new(banks);
    let group = fnv1("music_track");
    let switches = lib.music_switches(group).unwrap();
    println!("switches keyed on music_track: {}", switches.len());
    let mut states = std::collections::BTreeSet::new();
    for (_, s) in &switches {
        for (keys, _) in s.leaves().unwrap() {
            states.insert(keys[1]);
        }
    }
    println!("states {}", states.len());
    for st in states {
        let songs = lib.songs_for_state(group, st).unwrap();
        for song in &songs {
            let mut line = format!("{st} sw {} segs {}", song.switch, song.segments.len());
            for seg in &song.segments {
                line += &format!(
                    " dur {:.1}s tracks {}:",
                    seg.duration / 1000.0,
                    seg.tracks.len()
                );
                for t in &seg.tracks {
                    for m in &t.media {
                        let name = format!("data/audio/sound/{}.wem", m.source.media_id);
                        let hit = paths.iter().find(|p| p.eq_ignore_ascii_case(&name));
                        match hit {
                            Some(p) => {
                                let bytes = archive.read_path(p).unwrap();
                                let w = oag_formats::wwise::wem::Wem::parse(&bytes).unwrap();
                                line += &format!(
                                    " {}ch/{}",
                                    w.format().channels,
                                    w.format().sample_rate
                                );
                            }
                            None => line += " (no loose)",
                        }
                    }
                }
            }
            println!("{line}");
        }
    }
}

fn fnv1(s: &str) -> u32 {
    let mut h = 2_166_136_261u32;
    for b in s.to_ascii_lowercase().bytes() {
        h = h.wrapping_mul(16_777_619) ^ u32::from(b);
    }
    h
}

#[test]
#[ignore = "probe"]
fn paths() {
    let path = archive_path("omega-eu-patch", "data08.psarc").unwrap();
    let archive = oag_assets::psarc::Archive::open_file(&path).unwrap();
    for p in archive
        .paths()
        .iter()
        .filter(|p| p.to_lowercase().contains("music") && !p.ends_with(".wem"))
    {
        println!("{p}");
    }
}

#[test]
#[ignore = "probe"]
fn musictxt() {
    let path = archive_path("omega-eu-patch", "data08.psarc").unwrap();
    let mut archive = oag_assets::psarc::Archive::open_file(&path).unwrap();
    let b = archive.read_path("data/audio/sound/Music.txt").unwrap();
    std::fs::write("../../data/scratch/omega-music-lane/Music.txt", &b).unwrap();
    println!("{} bytes", b.len());
}
