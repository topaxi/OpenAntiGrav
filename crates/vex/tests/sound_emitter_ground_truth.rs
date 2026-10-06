//! Validates the [`sound_emitters`](oag_vex::sound_emitters) decoder
//! against every circuit on the PSP disc.
//!
//! **`#[ignore]`d, needs a disc image** (`just test-data`; ADR-0006). Skips when it
//! is absent; `OAG_REQUIRE_GAME_DATA=1` makes absence a failure.
//!
//! # What this is for
//!
//! The layout in `docs/ghidra/functions/psp-pulse-usa/track-sound-emitters.md`
//! is read off `VexSound_Init`, and every field in it would still "parse" if
//! the boundaries were a byte or a field out. Three things here are evidence
//! that they are not, and none of them is a number written down elsewhere:
//!
//! 1. **The bank and cue fields resolve against a format this crate parses
//!    separately.** `sblk` reads a bank's own label and name table with no
//!    knowledge of `.vex` at all, so 1,277 of 1,298 authored pairs naming a cue
//!    that really is in the bank they name is two decoders agreeing, not one
//!    decoder agreeing with itself.
//! 2. **The radius encoding comes out of the evaluator, not out of a fit.**
//!    `5000/65535` reproduces every stored key from the `f32` beside it; the
//!    `13.10` a regression on this same data produces does not.
//! 3. **`speaker` `0x3cc` is authored nowhere.** That is a claim about the whole
//!    disc, so only a sweep of the whole disc can hold it up.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use oag_disc::DiscImage;
use oag_formats::sblk;
use oag_formats::wad::{self, Compression, Directory};
use oag_vex::sound_emitters::{self, CLASS_SPEAKER, SoundEmitter, encode_radius};
use oag_vex::vex;

/// Track directories present on the PSP disc, from the plugin definitions.
const TRACK_DIRS: &[&str] = &[
    "01_Track", "02_Track", "03_Track", "04_Track", "05_Track", "06_Track", "07_Track", "08_Track",
    "09_Track", "10_Track", "11_Track", "12_Track", "13_Track", "14_Track", "15_Track", "16_Track",
];

/// The four models a track directory can hold.
const TRACK_FILES: &[&str] = &[
    "track.vex",
    "track_reversed.vex",
    "zone_track.vex",
    "zone_track_reversed.vex",
];

/// Fewest track files that must be read for the sweep to mean anything.
const MIN_FILES: usize = 20;

/// Emitters the twelve race circuits author between them, both directions.
const TOTAL_EMITTERS: usize = 1298;

/// How many of those are `soundcone`.
const TOTAL_CONES: usize = 134;

/// The bank labels the nodes name, every one of them a real bank in `Data.wad`.
const BANKS: &[&str] = &[
    "amphise", "arcprim", "basilic", "dekonst", "fortcle", "gentrak", "metropi", "moather",
    "outpost", "platinu", "talonsj", "techder", "vertica",
];

/// Every authored reference that does not resolve, with how many nodes make it.
///
/// Four name a cue no bank has and one names a bank that cannot exist - see the
/// evidence page's "Twenty-one references are dangling". Pinned as a list rather
/// than a count so that a decode which broke a *different* reference could not
/// pass by breaking exactly as many as it fixed.
const DANGLING: &[(&str, &str, usize)] = &[
    ("basilic", "~groupcraft", 2),
    ("fortcle", "~RED_NEON_TUN", 2),
    ("fortcle", "~blueflashlight", 14),
    ("outpostf", "~AIR_CON_FAN", 1),
    ("techder", "~neon", 2),
];

fn image(name: &str) -> Option<PathBuf> {
    oag_testdata::image(name)
}

/// The disc's `Data.wad`, mounted, with its directory parsed.
fn data_wad() -> Option<(DiscImage, oag_disc::Entry, Directory)> {
    let path = image("pulse-psp-usa.chd")?;
    let mut disc = DiscImage::open(&path).expect("open");
    let archive = disc
        .entries()
        .expect("entries")
        .iter()
        .find(|e| e.path == "PSP_GAME/USRDIR/Data.wad")
        .expect("Data.wad present")
        .clone();
    let header = disc
        .read_entry_range(&archive, 0, wad::HEADER_LEN as u64)
        .expect("header");
    let count = Directory::peek_entry_count(&header).expect("entry count");
    let dir_bytes = disc
        .read_entry_range(&archive, 0, Directory::directory_len(count))
        .expect("directory");
    let dir = Directory::parse(&dir_bytes, Some(archive.size)).expect("parse directory");
    Some((disc, archive, dir))
}

/// Every track model on the PSP disc, decompressed, with its archive name.
fn track_models() -> Option<Vec<(String, Vec<u8>)>> {
    let (mut disc, archive, dir) = data_wad()?;
    let mut out = Vec::new();
    for dir_name in TRACK_DIRS {
        for file in TRACK_FILES {
            let name = format!("Data\\Environments\\{dir_name}\\{file}");
            let hash = wad::hash_name(&name);
            let Some(entry) = dir.entries.iter().find(|e| e.name_hash == hash) else {
                continue;
            };
            if entry.size == 0 {
                continue;
            }
            let raw = disc
                .read_entry_range(&archive, u64::from(entry.offset), u64::from(entry.size))
                .expect("blob");
            let model = match entry.compression {
                Compression::None => raw,
                Compression::Lzss => {
                    oag_formats::lzss::decompress(&raw, entry.size_uncompressed as usize)
                        .expect("lzss")
                }
                Compression::Zlib => panic!("{name}: unexpected zlib entry"),
            };
            assert!(vex::has_magic(&model), "{name} is not a .vex model");
            out.push((name, model));
        }
    }
    assert!(
        out.len() >= MIN_FILES,
        "only {} track files found, expected at least {MIN_FILES}",
        out.len()
    );
    Some(out)
}

/// Every emitter every race circuit authors, tagged with the file it came from.
fn all_emitters() -> Option<Vec<(String, SoundEmitter)>> {
    let mut out = Vec::new();
    for (name, model) in track_models()? {
        let nodes = vex::nodes(&model).expect("nodes");
        for emitter in sound_emitters::emitters(&model, &nodes) {
            out.push((name.clone(), emitter));
        }
    }
    Some(out)
}

/// Cue names per bank label, read straight out of every `SBlk` in `Data.wad`.
///
/// The magic sits behind an offset the section table carries, so a cheap prefix
/// read is enough to tell a bank from anything else and only the banks are read
/// whole - 39 blobs out of 1,142.
fn banks() -> Option<BTreeMap<String, BTreeSet<String>>> {
    let (mut disc, archive, dir) = data_wad()?;
    let mut out: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for entry in &dir.entries {
        if entry.compression != Compression::None || entry.size < 64 {
            continue;
        }
        let head = disc
            .read_entry_range(&archive, u64::from(entry.offset), 64)
            .expect("prefix");
        if !sblk::looks_like_bank(&head) {
            continue;
        }
        let blob = disc
            .read_entry_range(&archive, u64::from(entry.offset), u64::from(entry.size))
            .expect("blob");
        let Ok(bank) = sblk::Bank::parse(&blob) else {
            continue;
        };
        let cues = out.entry(bank.name.clone()).or_default();
        for sound in bank.sound_names() {
            cues.insert(sound.name);
        }
    }
    Some(out)
}

#[test]
#[ignore = "needs a disc image under data/images/"]
fn every_race_circuit_authors_emitters_and_no_zone_circuit_does() {
    let Some(models) = track_models() else {
        return;
    };
    let mut race = 0;
    let mut total = 0;
    let mut cones = 0;
    for (name, model) in &models {
        let nodes = vex::nodes(model).expect("nodes");
        let emitters = sound_emitters::emitters(model, &nodes);
        let zone = name.contains("zone_track");
        if zone {
            assert!(
                emitters.is_empty(),
                "{name}: a Zone circuit authors {} emitter(s)",
                emitters.len()
            );
            continue;
        }
        assert!(!emitters.is_empty(), "{name}: a race circuit authors none");
        race += 1;
        total += emitters.len();
        cones += emitters.iter().filter(|e| e.cone.is_some()).count();
    }
    assert_eq!(race, 24, "twelve circuits, two directions each");
    assert_eq!(total, TOTAL_EMITTERS);
    assert_eq!(cones, TOTAL_CONES);
    println!("{total} emitters over {race} circuit files, {cones} of them cones");
}

/// `speaker` `0x3cc` is registered by the executable and authored by nothing.
#[test]
#[ignore = "needs a disc image under data/images/"]
fn no_track_on_the_disc_authors_a_speaker() {
    let Some(models) = track_models() else {
        return;
    };
    for (name, model) in &models {
        let nodes = vex::nodes(model).expect("nodes");
        let speakers = vex::nodes_by_class(&nodes, CLASS_SPEAKER).count();
        assert_eq!(speakers, 0, "{name} authors {speakers} speaker node(s)");
    }
    println!("swept {} track files, no speaker node", models.len());
}

/// The two constants out of `VexSound_SampleRadiusCurve` against the disc.
///
/// The curve, not the `f32`, is what the game plays, so the check that matters
/// is that the stored key reproduces the `f32` beside it under the executable's
/// own scale - which is a statement about the decode of four separate fields at
/// once (`+0x10`, `+0x2c`, `+0x30`, `+0x34`).
#[test]
#[ignore = "needs a disc image under data/images/"]
fn every_stored_radius_key_matches_the_executables_encoding() {
    let Some(emitters) = all_emitters() else {
        return;
    };
    let mut worst = 0.0f32;
    for (name, emitter) in &emitters {
        assert_eq!(
            emitter.radius_curve.times.len(),
            1,
            "{name}: {} authors {} key(s), not one",
            emitter.cue,
            emitter.radius_curve.times.len()
        );
        assert_eq!(emitter.radius_curve.times[0], 0, "{name}: {}", emitter.cue);
        assert!(
            (emitter.radius_curve.seconds_per_tick - 1.0 / 60.0).abs() < 1e-7,
            "{name}: {} ticks at {}",
            emitter.cue,
            emitter.radius_curve.seconds_per_tick
        );
        if emitter.cone.is_some() {
            // A cone stores no radius key at all - explained, not just pinned,
            // as of `VexSoundCone_Init`'s own decode: the curve mechanism that
            // would ever read this back (`VexSound_Update`) has exactly two
            // static call sites in the executable and both are gated on the
            // payload's own `+0x3c`, which is `0` on every one of the 1,298
            // authored nodes - so no cone (and no plain `sound` either) is
            // ever actually resampled, and this field is free to be `0`. See
            // `docs/ghidra/functions/psp-pulse-usa/track-sound-emitters.md`'s
            // own section on `VexSoundCone_Init`. Pinned so a title that did
            // exercise the curve would fail loudly here instead of silently.
            assert_eq!(emitter.radius_curve.values[0], 0, "{name}: {}", emitter.cue);
            continue;
        }
        assert_eq!(
            emitter.radius_curve.values[0],
            encode_radius(emitter.radius),
            "{name}: {} at radius {}",
            emitter.cue,
            emitter.radius
        );
        worst = worst.max((emitter.sample_radius(0.0) - emitter.radius).abs());
    }
    assert!(worst < 0.075, "round-trip error grew to {worst}");
    println!(
        "{} keys reproduce their f32, worst round trip {worst:.6} units",
        emitters.len() - TOTAL_CONES
    );
}

/// The bank and cue fields, against banks parsed by a decoder that has never
/// heard of `.vex`.
#[test]
#[ignore = "needs a disc image under data/images/"]
fn every_authored_cue_but_the_disc_s_own_five_bugs_resolves_in_its_bank() {
    let (Some(emitters), Some(banks)) = (all_emitters(), banks()) else {
        return;
    };
    for label in BANKS {
        assert!(banks.contains_key(*label), "no bank labelled {label}");
    }
    let mut dangling: BTreeMap<(String, String), usize> = BTreeMap::new();
    let mut resolved = 0;
    for (_, emitter) in &emitters {
        if banks
            .get(&emitter.bank)
            .is_some_and(|cues| cues.contains(&emitter.cue))
        {
            resolved += 1;
        } else {
            *dangling
                .entry((emitter.bank.clone(), emitter.cue.clone()))
                .or_default() += 1;
        }
    }
    let found: Vec<_> = dangling
        .iter()
        .map(|((bank, cue), n)| (bank.as_str(), cue.as_str(), *n))
        .collect();
    assert_eq!(found, DANGLING, "the set of dangling references moved");
    assert_eq!(resolved, TOTAL_EMITTERS - 21);
    println!("{resolved} of {} authored pairs resolve", emitters.len());
}

/// The ordering that licenses `Cone::wide` and `Cone::narrow` being derived.
///
/// `+0x04` is `40` degrees on every authored cone and `+0x00` is never below it.
/// **`+0x00` is also the one `VexSoundCone_Init` writes to the emitter's
/// `+0x40` half-angle** (confidence 90 - the write site is read now, see
/// `track-sound-emitters.md`), so `Cone::wide()` is not just an ordering this
/// module derives for its own naming - it is the half-angle the game
/// attenuates by. This test still pins the ordering itself: without it a
/// title that authored the two the other way round would decode swapped in
/// silence.
#[test]
#[ignore = "needs a disc image under data/images/"]
fn every_cone_authors_forty_degrees_at_0x04_and_no_less_at_0x00() {
    let Some(emitters) = all_emitters() else {
        return;
    };
    let cones: Vec<_> = emitters
        .iter()
        .filter_map(|(name, e)| e.cone.map(|cone| (name, e, cone)))
        .collect();
    assert_eq!(cones.len(), TOTAL_CONES);
    let mut widths = BTreeSet::new();
    for (name, emitter, cone) in &cones {
        assert!(
            (cone.angle_b.to_degrees() - 40.0).abs() < 1e-3,
            "{name}: {} authors {} degrees at +0x04",
            emitter.cue,
            cone.angle_b.to_degrees()
        );
        assert!(
            cone.angle_a >= cone.angle_b,
            "{name}: {} authors {} at +0x00, under the {} at +0x04",
            emitter.cue,
            cone.angle_a.to_degrees(),
            cone.angle_b.to_degrees()
        );
        let degrees = cone.angle_a.to_degrees();
        assert!(
            (degrees - degrees.round()).abs() < 1e-3,
            "{name}: {} authors {degrees} degrees at +0x00, not a whole one",
            emitter.cue
        );
        widths.insert(degrees.round() as i32);
    }
    assert_eq!(
        widths,
        BTreeSet::from([40, 50, 60, 70, 75, 80, 100, 120]),
        "the set of authored cone widths moved"
    );
    println!("{} cones, all 40 degrees at +0x04", cones.len());
}

/// Placement comes from the transform chain and nowhere else, so if the chain
/// were composed wrongly every emitter would sit on top of every other one -
/// the same failure `pads_ground_truth` guards against, and the same reason it
/// cannot be caught by reading a payload.
#[test]
#[ignore = "needs a disc image under data/images/"]
fn emitters_are_spread_around_a_circuit_rather_than_piled_at_the_origin() {
    let Some(emitters) = all_emitters() else {
        return;
    };
    let mut per_file: BTreeMap<String, Vec<[f32; 3]>> = BTreeMap::new();
    for (name, emitter) in &emitters {
        per_file
            .entry(name.clone())
            .or_default()
            .push(emitter.position());
    }
    for (name, points) in &per_file {
        let distinct: BTreeSet<_> = points
            .iter()
            .map(|p| (p[0].to_bits(), p[1].to_bits(), p[2].to_bits()))
            .collect();
        assert!(
            distinct.len() * 2 >= points.len(),
            "{name}: {} emitters share {} position(s)",
            points.len(),
            distinct.len()
        );
        let at_origin = points
            .iter()
            .filter(|p| p[0] == 0.0 && p[1] == 0.0 && p[2] == 0.0)
            .count();
        assert_eq!(at_origin, 0, "{name}: {at_origin} emitter(s) at the origin");
        let span = |axis: usize| {
            let lo = points.iter().map(|p| p[axis]).fold(f32::MAX, f32::min);
            let hi = points.iter().map(|p| p[axis]).fold(f32::MIN, f32::max);
            hi - lo
        };
        assert!(
            span(0) > 100.0 && span(2) > 100.0,
            "{name}: emitters span only {} x {} units",
            span(0),
            span(2)
        );
    }
    println!("{} circuit files, all spread", per_file.len());
}
