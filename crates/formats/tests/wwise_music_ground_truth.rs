//! Music objects over every bank of the Omega Collection's base and patch, and
//! the walk from a `Music_Track` state to its stems.
//!
//! **`#[ignore]`d and never run in CI**; needs `data/extracted/ps4`. The page
//! to update on a change is `docs/formats/wwise.md`, "Music".

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use oag_formats::wwise::hirc::StreamType;
use oag_formats::wwise::music::{RanSeq, Segment, Switch, track_clips, track_type};
use oag_formats::wwise::{Bank, Kind, Library, name_hash};

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

/// Every `.bnk` of an archive, read.
fn blobs(archive: &mut oag_assets::psarc::Archive) -> Vec<Vec<u8>> {
    let paths: Vec<String> = archive
        .paths()
        .iter()
        .filter(|p| p.ends_with(".bnk"))
        .cloned()
        .collect();
    paths
        .iter()
        .map(|p| archive.read_path(p).expect("the bank reads"))
        .collect()
}

#[derive(Debug, Default, PartialEq, Eq)]
struct Census {
    segments: usize,
    switches: usize,
    ranseqs: usize,
    tracks: usize,
    /// Segment children that are music tracks of the library, and not.
    segment_children: usize,
    segment_children_missing: usize,
    /// Switch leaves that name an object of the library, name `0`, and name
    /// something else.
    leaves_resolved: usize,
    leaves_zero: usize,
    leaves_missing: usize,
    /// Playlist items naming a segment of the library / not.
    items_resolved: usize,
    items_missing: usize,
    /// Tracks of type 0 with exactly one clip per source.
    normal_one_clip: usize,
    /// Tracks whose source is streamed (a loose `.wem`).
    streamed: usize,
    /// Switch rules skipped, and the ones with a transition object.
    rules: usize,
}

fn census(dir: &str, name: &str) -> Option<Census> {
    let path = archive_path(dir, name)?;
    let mut archive = oag_assets::psarc::Archive::open_file(&path).expect("opens");
    let blobs = blobs(&mut archive);
    let banks: Vec<Bank> = blobs
        .iter()
        .map(|b| Bank::parse(b).expect("parses"))
        .collect();
    let all_ids: BTreeSet<u32> = banks
        .iter()
        .flat_map(|b| b.objects().iter().map(|o| o.id))
        .collect();
    let mut c = Census::default();
    for bank in &banks {
        for o in bank.objects() {
            let body = bank.body(o);
            match o.kind {
                Kind::MusicSegment => {
                    c.segments += 1;
                    let s = Segment::parse(o.kind, body).expect("a segment reads to its end");
                    for child in &s.node.children {
                        if all_ids.contains(child) {
                            c.segment_children += 1;
                        } else {
                            c.segment_children_missing += 1;
                        }
                    }
                }
                Kind::MusicSwitch => {
                    c.switches += 1;
                    let s = Switch::parse(o.kind, body).expect("a switch reads to its end");
                    c.rules += s.rules as usize;
                    for (_, target) in s.leaves().expect("the tree walks") {
                        match target {
                            0 => c.leaves_zero += 1,
                            t if all_ids.contains(&t) => c.leaves_resolved += 1,
                            _ => c.leaves_missing += 1,
                        }
                    }
                }
                Kind::MusicRanSeq => {
                    c.ranseqs += 1;
                    let r = RanSeq::parse(o.kind, body).expect("a container reads to its end");
                    c.rules += r.rules as usize;
                    for segment in r.segments() {
                        if all_ids.contains(&segment) {
                            c.items_resolved += 1;
                        } else {
                            c.items_missing += 1;
                        }
                    }
                }
                Kind::MusicTrack => {
                    c.tracks += 1;
                    let sources = oag_formats::wwise::hirc::MusicTrack::parse(body)
                        .expect("sources")
                        .sources;
                    let clips = track_clips(body).expect("clips");
                    if track_type(body) == Some(0) && clips.len() == sources.len() {
                        c.normal_one_clip += 1;
                    }
                    c.streamed += sources
                        .iter()
                        .filter(|s| s.stream == StreamType::Streamed)
                        .count();
                }
                _ => {}
            }
        }
    }
    Some(c)
}

#[test]
#[ignore = "needs data/extracted/ps4/omega-eu"]
fn base_data00_music_objects() {
    let Some(c) = census("omega-eu", "data00.psarc") else {
        return;
    };
    assert_eq!(
        c,
        Census {
            segments: 163,
            switches: 10,
            ranseqs: 61,
            tracks: 927,
            segment_children: 927,
            segment_children_missing: 0,
            leaves_resolved: 80,
            leaves_zero: 1,
            leaves_missing: 0,
            items_resolved: 1228,
            items_missing: 0,
            normal_one_clip: 927,
            streamed: 924,
            rules: 185,
        }
    );
}

#[test]
#[ignore = "needs data/extracted/ps4/omega-eu-patch"]
fn patch_data05_has_no_music_objects() {
    let Some(c) = census("omega-eu-patch", "data05.psarc") else {
        return;
    };
    assert_eq!(c, Census::default());
}

#[test]
#[ignore = "needs data/extracted/ps4/omega-eu-patch"]
fn patch_data08_music_objects() {
    let Some(c) = census("omega-eu-patch", "data08.psarc") else {
        return;
    };
    assert_eq!(
        c,
        Census {
            segments: 169,
            switches: 10,
            ranseqs: 63,
            tracks: 993,
            segment_children: 993,
            segment_children_missing: 0,
            leaves_resolved: 82,
            leaves_zero: 1,
            leaves_missing: 0,
            items_resolved: 1295,
            items_missing: 0,
            normal_one_clip: 993,
            streamed: 990,
            rules: 187,
        }
    );
}

/// `Set_Music_Track_N__frontend` for N in 1..=28 sets `Music_Track` to 28
/// distinct states; each walks, with the race flow set, to one segment whose
/// tracks are the stems of the song.
#[test]
#[ignore = "needs data/extracted/ps4/omega-eu-patch"]
fn patch_data08_every_event_state_walks_to_a_song() {
    let Some(path) = archive_path("omega-eu-patch", "data08.psarc") else {
        return;
    };
    let mut archive = oag_assets::psarc::Archive::open_file(&path).expect("opens");
    let music = archive
        .read_path("data/audio/sound/Music.bnk")
        .expect("reads");
    let lib = Library::new(vec![Bank::parse(&music).expect("parses")]);
    let group = name_hash("Music_Track");
    let flow = [
        (name_hash("Game_FLOW"), name_hash("Gameplay")),
        (name_hash("Gameplay_FLOW"), name_hash("Race")),
    ];
    let play = lib
        .play_targets(name_hash("Play_External_Music"))
        .expect("event");
    assert_eq!(play.len(), 1);

    let mut states = BTreeSet::new();
    let (mut stereo_songs, mut multichannel_songs, mut stems_total) = (0, 0, 0);
    for n in 1..=28 {
        let event = name_hash(&format!("Set_Music_Track_{n}__frontend"));
        let sets = lib
            .set_states(event)
            .unwrap_or_else(|| panic!("event for {n}"));
        let state = sets
            .iter()
            .find(|s| s.group == group)
            .unwrap_or_else(|| panic!("event {n} sets no Music_Track"))
            .state;
        assert!(states.insert(state), "state {state} twice");
        let mut walk = flow.to_vec();
        walk.push((group, state));
        let chain = lib
            .walk_music(play[0], &walk, name_hash("None"))
            .unwrap_or_else(|e| panic!("{n}: {e:?}"));
        assert_eq!(chain.segments.len(), 1, "{n}");
        let segment = &chain.segments[0];
        let mut last_end = f64::MIN;
        for track in &segment.tracks {
            assert_eq!(track.track_type, Some(0));
            assert_eq!((track.media.len(), track.clips.len()), (1, 1));
            let clip = track.clips[0];
            // The clip's end on the song's clock: the longest lands on the
            // segment's length.
            last_end = last_end.max(clip.play_at + clip.source_duration + clip.end_trim);
            stems_total += 1;
        }
        assert!(
            (last_end - segment.duration).abs() < 1.0,
            "{n}: stems end at {last_end} ms, segment is {} ms",
            segment.duration
        );
        if segment.tracks.len() == 1 {
            multichannel_songs += 1;
        } else {
            stereo_songs += 1;
        }
    }
    assert_eq!(states.len(), 28);
    // 17 songs of 7 to 11 stems, 11 of one track: 165 stems, the 165 loose
    // media the lead's census found (154 stereo, 11 eight-channel).
    assert_eq!(
        (stereo_songs, multichannel_songs, stems_total),
        (17, 11, 165)
    );
}
