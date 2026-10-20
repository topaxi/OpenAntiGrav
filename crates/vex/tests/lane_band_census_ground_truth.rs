//! The census of the three spline-point bytes the original's AI reads and no
//! reader carried: `+0x5c`/`+0x5d` (a lane band) and `+0x66` (a speed floor);
//! and, over the same walk, the collision cage's header flag and its eight
//! `+0x68..+0x6f` bytes (`check_cage`).
//!
//! **`#[ignore]`d, needs a disc image** (`just test-data`; ADR-0006). Skips when
//! it is absent; `OAG_REQUIRE_GAME_DATA=1` makes absence a failure.
//!
//! Every `WO Track` node in `Data.wad` of both Pulse PSP pressings is read, 40
//! per disc and 34,261 control points, and each point is classed by what it
//! authors: a lane band is `+0x5c < +0x5d` (the shipped default `0xff`/`0x00`
//! disables it), a speed-floor candidate is `+0x66 < 35`, and a point is
//! "off-default" when `+0x66` is not the shipped default. The result and what
//! it means are `docs/formats/track.md`, "Three bytes the AI reads"; the
//! readings are `docs/ghidra/functions/psp-pulse-usa/ai-controls.md`.

use std::collections::BTreeMap;

use oag_assets::Archive;
use oag_formats::wad;
use oag_vex::{track, vex};

const TRACKS: usize = 40;
const POINTS: usize = 34_261;

/// `+0x66` on every point of a circuit that authors nothing there.
const DEFAULT_FLOOR: u8 = 253;

#[derive(Default, Debug, PartialEq, Eq)]
struct Tally {
    points: usize,
    banded: usize,
    floor_below_35: usize,
    floor_off_default: usize,
    /// Values `+0x66` takes below 35.
    low_values: Vec<u8>,
    /// Points authoring any collision-cage byte (`+0x68..+0x6f`).
    caged: usize,
}

/// The collision cage over the same walk: tracks whose header sets the flag
/// (`track::cage_flag`), and per track the points authoring any cage byte.
#[derive(Default, Debug)]
struct Cage {
    flagged: usize,
    caged: BTreeMap<String, usize>,
}

fn name_of(hash: u32) -> String {
    for n in 1..=16 {
        for f in [
            "track.vex",
            "track_reversed.vex",
            "zone_track.vex",
            "zone_track_reversed.vex",
        ] {
            let name = format!("Data\\Environments\\{n:02}_Track\\{f}");
            if wad::hash_name(&name) == hash {
                return name;
            }
        }
    }
    format!("hash {hash:#010x}")
}

/// Per track: its tally. Only tracks that author anything are kept in
/// `interesting`; `tracks` and `points` count the sweep.
fn census(image: &str) -> Option<(usize, usize, BTreeMap<String, Tally>, Cage)> {
    let path = oag_testdata::image(image)?;
    let spec = format!("{}:PSP_GAME/USRDIR/Data.wad", path.display());
    let mut data = Archive::open(&spec).unwrap_or_else(|e| panic!("{spec}: {e}"));
    let (mut tracks, mut points) = (0, 0);
    let mut interesting = BTreeMap::new();
    let mut cage = Cage::default();
    for index in 0..data.directory().entries.len() {
        if data.directory().entries[index].size == 0 {
            continue;
        }
        let hash = data.directory().entries[index].name_hash;
        let Ok(bytes) = data.read(index) else {
            continue;
        };
        if !vex::has_magic(&bytes) {
            continue;
        }
        let Ok(nodes) = vex::nodes(&bytes) else {
            continue;
        };
        let Some(node) = track::find_node(&bytes, &nodes) else {
            continue;
        };
        let ai = track::parse(&bytes[node.payload()]).unwrap_or_else(|e| panic!("{index}: {e}"));
        cage.flagged += usize::from(track::cage_flag(&bytes[node.payload()]));
        assert_eq!(ai.version, 0x105, "entry {index}");
        tracks += 1;
        let mut t = Tally::default();
        for p in ai.paths.iter().flat_map(|p| &p.points) {
            let (lo, hi, floor) = (
                p.unk_0x5c.expect("a long point"),
                p.unk_0x5d.expect("a long point"),
                p.unk_0x66.expect("a long point"),
            );
            t.points += 1;
            t.banded += usize::from(lo < hi);
            t.floor_below_35 += usize::from(floor < 35);
            t.floor_off_default += usize::from(floor != DEFAULT_FLOOR);
            if floor < 35 && !t.low_values.contains(&floor) {
                t.low_values.push(floor);
            }
            t.caged += usize::from(p.cage != [0; 8]);
        }
        points += t.points;
        cage.caged.insert(name_of(hash), t.caged);
        if t.banded > 0 || t.floor_off_default > 0 {
            interesting.insert(name_of(hash), t);
        }
    }
    Some((tracks, points, interesting, cage))
}

fn check(image: &str) {
    let Some((tracks, points, found, cage)) = census(image) else {
        return;
    };
    check_cage(image, &cage);
    assert_eq!((tracks, points), (TRACKS, POINTS), "{image}");
    let summary: Vec<(&str, usize, usize, usize, &[u8])> = found
        .iter()
        .map(|(n, t)| {
            (
                n.rsplit("Environments\\").next().unwrap(),
                t.banded,
                t.floor_below_35,
                t.floor_off_default,
                t.low_values.as_slice(),
            )
        })
        .collect();
    // Only 07_Track authors a lane band, and only 06_Track authors +0x66 at
    // all; every other circuit, zone track and reversal ships both defaults.
    assert_eq!(
        summary,
        vec![
            ("06_Track\\track.vex", 0, 63, 713, &[0u8][..]),
            ("06_Track\\track_reversed.vex", 0, 17, 819, &[0u8][..]),
            ("07_Track\\track.vex", 161, 0, 0, &[][..]),
            ("07_Track\\track_reversed.vex", 150, 0, 0, &[][..]),
        ],
        "{image}"
    );
}

/// Every Pulse track runs the collision cage, and authors the bytes it reads on every
/// point: the header flag (`FUN_0887d36c`) is set on all 40 and no point leaves its
/// eight `+0x68..+0x6f` bytes at zero. See
/// `docs/ghidra/functions/psp-pulse-usa/collision-cage.md`.
fn check_cage(image: &str, cage: &Cage) {
    assert_eq!(cage.flagged, TRACKS, "{image}");
    assert_eq!(cage.caged.values().sum::<usize>(), POINTS, "{image}");
}

#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn pulse_usa_authors_a_lane_band_on_one_circuit_and_a_floor_value_on_another() {
    check("pulse-psp-usa.chd");
}

#[test]
#[ignore = "needs data/images/pulse-psp-eu.chd"]
fn pulse_eu_authors_the_same_bytes() {
    check("pulse-psp-eu.chd");
}
