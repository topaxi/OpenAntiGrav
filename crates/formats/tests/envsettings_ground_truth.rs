//! What the disc says about `.envsettings`.
//!
//! `#[ignore]`d because it needs `data/images/hdfury-ps3-eu-dec.iso`; run with
//! `just test-data`.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use oag_formats::envsettings::{self, EnvSettings};

/// Every archive on the disc.
const ARCHIVES: &[&str] = &[
    "DATA00", "DATA01", "DATA02", "DATA03", "DATA04", "DATA05", "DATA06",
];

/// The one file that is not a circuit's.
///
/// 436 lines of `Feedback`, `Equaliser` and `Music Pulse` keys for the Fury
/// front end. Named rather than filtered by line count, so that a second
/// non-circuit file shows up as a failure instead of being absorbed.
const NOT_A_CIRCUIT: &str = "/data/fe/fury.envsettings";

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

/// Every `.envsettings` on the disc, as `(path, parsed)`.
fn every_file() -> Vec<(String, EnvSettings)> {
    let image = image().expect("checked by the caller");
    let mut out = Vec::new();
    for archive in ARCHIVES {
        let spec = format!("{}:PS3_GAME/USRDIR/{archive}.PSARC", image.display());
        let mut open = oag_assets::psarc::Archive::open(&spec).expect("the archive opens");
        let paths: Vec<String> = open
            .paths()
            .iter()
            .filter(|p| p.ends_with(".envsettings"))
            .cloned()
            .collect();
        for path in paths {
            let bytes = open.read_path(&path).expect("the entry reads");
            let text = String::from_utf8(bytes)
                .unwrap_or_else(|_| panic!("{path} is not UTF-8, so it is not this format"));
            let parsed = EnvSettings::parse(&text).unwrap_or_else(|e| panic!("{path}: {e}"));
            out.push((path, parsed));
        }
    }
    out
}

/// The circuits' files alone.
fn circuits() -> Vec<(String, EnvSettings)> {
    every_file()
        .into_iter()
        .filter(|(path, _)| path != NOT_A_CIRCUIT)
        .collect()
}

/// **Every line of every file is `key=value`.**
///
/// The whole syntax claim, and it is all-or-nothing: `EnvSettings::parse`
/// refuses the first line that is not, so a file that is a different format
/// fails here rather than decoding to a sparse map.
#[test]
#[ignore]
fn every_file_on_the_disc_parses() {
    if image().is_none() {
        return;
    }
    let files = every_file();
    println!("{} .envsettings files", files.len());
    for (path, parsed) in &files {
        println!("  {:3} entries  {path}", parsed.entries.len());
    }
    assert_eq!(files.len(), 33, "the disc's count");
    assert_eq!(
        files.iter().filter(|(p, _)| p == NOT_A_CIRCUIT).count(),
        1,
        "exactly one non-circuit file, and it is the one named"
    );
}

/// **The 32 circuits share one schema**, which is what makes a key a field
/// rather than a value someone happened to write.
///
/// Four keys are on some circuits and not all, and they are listed rather than
/// tolerated silently: a key present on 12 of 32 is a fact about the corpus.
#[test]
#[ignore]
fn the_circuits_share_a_schema() {
    if image().is_none() {
        return;
    }
    let files = circuits();
    assert_eq!(files.len(), 32);
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for (_, parsed) in &files {
        for key in parsed.entries.keys() {
            *counts.entry(key.as_str()).or_default() += 1;
        }
    }
    let everywhere: Vec<&str> = counts
        .iter()
        .filter(|(_, n)| **n == files.len())
        .map(|(k, _)| *k)
        .collect();
    println!("{} keys on all 32 circuits", everywhere.len());
    for (key, n) in counts.iter().filter(|(_, n)| **n != files.len()) {
        println!("  {n:2}/32  {key}");
    }
    assert!(
        everywhere.len() >= 30,
        "the shared schema should be most of it, not {}",
        everywhere.len()
    );
    for key in [
        envsettings::SUN_DIRECTION,
        envsettings::SUN_COLOUR,
        envsettings::AMBIENT_COLOUR,
        envsettings::SKY_COLOUR,
        envsettings::FOG_COLOUR,
        envsettings::FOG_DENSITY,
    ] {
        assert!(
            everywhere.contains(&key),
            "{key} is one this project reads, so it must be on every circuit"
        );
    }
}

/// **The two number encodings, and that only one key uses the byte one.**
///
/// This is the claim `Value::is_integer` exists for. A parser that reads every
/// value as a normalised float reads `Sky colour` 255 times too bright, and the
/// only thing in the file that says otherwise is the missing decimal point.
#[test]
#[ignore]
fn sky_colour_is_the_only_byte_encoded_key() {
    if image().is_none() {
        return;
    }
    let mut integer_keys: BTreeMap<&str, (f32, f32)> = BTreeMap::new();
    let files = circuits();
    for (_, parsed) in &files {
        for (key, value) in &parsed.entries {
            if !value.is_integer {
                continue;
            }
            let entry = integer_keys
                .entry(key.as_str())
                .or_insert((f32::MAX, f32::MIN));
            for n in &value.numbers {
                entry.0 = entry.0.min(*n);
                entry.1 = entry.1.max(*n);
            }
        }
    }
    for (key, (lo, hi)) in &integer_keys {
        println!("  {lo:6} .. {hi:6}  {key}");
    }
    let over_one: Vec<&str> = integer_keys
        .iter()
        .filter(|(_, (_, hi))| *hi > 1.0)
        .map(|(k, _)| *k)
        .collect();
    assert_eq!(
        over_one,
        vec![envsettings::SKY_COLOUR],
        "every other integer-written key is a 0/1 flag"
    );
    for (_, parsed) in &files {
        assert!(
            parsed.rgba8(envsettings::SKY_COLOUR).is_some(),
            "every circuit's sky colour reads as four bytes"
        );
    }
}

/// **The file is authored for a linear HDR pipeline, and this one is not.**
///
/// Recorded as a test rather than a note because it is the reason
/// `oag_render` uses the authored *direction* and not the authored
/// *magnitudes*: a sun colour of 4.0 multiplied into an `Rgba8Unorm` target
/// with no tonemapper clips everything above a quarter brightness to white.
/// See `docs/formats/envsettings.md` and
/// `docs/architecture/adr/0020-gamma-authoritative-colour-space.md`.
///
/// If this ever fails because the corpus is all within `0..=1`, the hold it
/// justifies should be revisited - which is exactly what a test is for.
#[test]
#[ignore]
fn the_authored_colours_run_past_one() {
    if image().is_none() {
        return;
    }
    let mut worst: BTreeMap<&str, f32> = BTreeMap::new();
    for (_, parsed) in circuits() {
        for key in [
            envsettings::SUN_COLOUR,
            envsettings::AMBIENT_COLOUR,
            envsettings::FOG_COLOUR,
        ] {
            if let Some(triple) = parsed.vec3(key) {
                let high = triple.iter().fold(0.0f32, |a, b| a.max(*b));
                let entry = worst.entry(key).or_insert(0.0);
                *entry = entry.max(high);
            }
        }
    }
    for (key, high) in &worst {
        println!("  brightest {high:.3}  {key}");
    }
    assert!(
        worst[envsettings::SUN_COLOUR] > 1.5,
        "the sun colour is what needs headroom this pipeline does not have"
    );
}

/// **The sun direction is not a unit vector, and a reader that assumes one is
/// wrong on most of the corpus.**
///
/// Eight distinct lengths across 32 circuits. Whether the length carries an
/// intensity is not established - nothing here reads HD's executable - so
/// `EnvSettings::direction` normalises and says so, which is the reading that
/// cannot scale a light by an accident.
#[test]
#[ignore]
fn the_sun_direction_is_not_authored_as_a_unit_vector() {
    if image().is_none() {
        return;
    }
    let mut lengths: BTreeSet<String> = BTreeSet::new();
    let mut degenerate = 0usize;
    for (path, parsed) in circuits() {
        let [x, y, z] = parsed
            .vec3(envsettings::SUN_DIRECTION)
            .unwrap_or_else(|| panic!("{path} has no sun direction"));
        let length = (x * x + y * y + z * z).sqrt();
        lengths.insert(format!("{length:.3}"));
        if parsed.direction(envsettings::SUN_DIRECTION).is_none() {
            degenerate += 1;
        }
    }
    println!("sun direction lengths: {lengths:?}, {degenerate} degenerate");
    assert!(
        lengths.len() > 1,
        "if every circuit were unit, normalising would be pointless"
    );
    assert!(
        lengths.contains("1.000"),
        "most of them are unit, which is why the others are worth noticing"
    );
}
