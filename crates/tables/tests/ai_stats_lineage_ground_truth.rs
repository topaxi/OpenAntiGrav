//! The AI's two tuning files are one table across four titles: HD (disc and
//! PSN), 2048 and Omega author `AIControlStats.xml` and
//! `AIRaceStats_<class>.xml` with **Pulse's own values**, attribute for
//! attribute.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! ```
//!
//! # What this is for
//!
//! "HD's opponents should drive like HD's, not Pulse's" reduces, on the data
//! side, to this file: HD's tables *are* Pulse's. The only authored additions
//! are `SplitScreenMultiplier` (HD's `DATA03.PSARC`, 2048 and Omega), which
//! HD's parser stores at `+0xc4` and its steering reads only with more than
//! one local player, and 2048's `SuperPhantomStats`, a copy of Phantom's.
//! Both are left out of the comparison and named here. The evidence page is
//! `docs/ghidra/functions/ps3-hdfury-eu/ai-stats.md`. No value is written in
//! this file: each title is compared against Pulse's own parse.

use oag_assets::Archive;
use oag_tables::ai_race_stats;
use oag_tables::fexml::{self, Node};
use oag_tables::handling::SpeedClass;

/// The elements an HD-era file adds over Pulse's, left out on purpose.
const ADDED: &[&str] = &["SplitScreenMultiplier", "SuperPhantomStats"];

/// Every `(element path, attribute, value)` under `node`, numbers parsed so
/// `0.5` and `0.5000` compare equal, the `ADDED` elements skipped.
fn flatten(node: &Node, path: &str, out: &mut Vec<(String, String, String)>) {
    if ADDED.iter().any(|a| node.name.eq_ignore_ascii_case(a)) {
        return;
    }
    let here = format!("{path}/{}", node.name.to_ascii_lowercase());
    for (key, raw) in &node.attrs {
        let value = raw
            .trim()
            .parse::<f32>()
            .map_or_else(|_| raw.trim().to_string(), |v| format!("{v:?}"));
        out.push((here.clone(), key.to_ascii_lowercase(), value));
    }
    for child in &node.children {
        flatten(child, &here, out);
    }
}

fn attributes(blob: &[u8]) -> Vec<(String, String, String)> {
    let text = fexml::text(blob).expect("the file expands");
    let mut out = Vec::new();
    flatten(&fexml::parse(&text), "", &mut out);
    out.sort();
    out
}

/// One file's name per title, by its own spelling.
fn files() -> Vec<String> {
    let mut names = vec![r"Data\XML\AIControlStats.xml".to_string()];
    names.extend(SpeedClass::ALL.into_iter().map(ai_race_stats::entry_name));
    names
}

/// Pulse PSP EU's five files, the reference.
fn pulse() -> Option<Vec<(String, Vec<u8>)>> {
    let path = oag_testdata::image("pulse-psp-eu.chd")?;
    let spec = format!("{}:PSP_GAME/USRDIR/Data.wad", path.display());
    let mut wad = Archive::open(&spec).unwrap_or_else(|e| panic!("{spec}: {e}"));
    Some(
        files()
            .into_iter()
            .map(|name| {
                let blob = wad
                    .read_name(&name)
                    .unwrap_or_else(|e| panic!("{name}: {e}"));
                (name, blob)
            })
            .collect(),
    )
}

/// `name` (a `Data\XML\...` entry) as a PSARC path.
fn psarc_path(name: &str) -> String {
    name.replace('\\', "/")
}

/// Asserts each of `archive`'s five files carries Pulse's attributes.
fn assert_same_as_pulse(label: &str, archive: &mut oag_assets::psarc::Archive) {
    let Some(reference) = pulse() else {
        return;
    };
    for (name, pulse_blob) in reference {
        let path = psarc_path(&name);
        let blob = archive
            .read_path(&path)
            .unwrap_or_else(|e| panic!("{label} {path}: {e}"));
        let ours = attributes(&blob);
        assert!(!ours.is_empty(), "{label} {path} has no attributes");
        assert_eq!(
            ours,
            attributes(&pulse_blob),
            "{label} {path} differs from Pulse's"
        );
    }
}

fn hd_disc(archive: &str) -> Option<oag_assets::psarc::Archive> {
    let iso = oag_testdata::image("hdfury-ps3-eu-dec.iso")?;
    let spec = format!("{}:PS3_GAME/USRDIR/{archive}.PSARC", iso.display());
    Some(oag_assets::psarc::Archive::open(&spec).unwrap_or_else(|e| panic!("{spec}: {e}")))
}

fn loose(path: &str) -> Option<oag_assets::psarc::Archive> {
    let file = oag_testdata::exact(path)?;
    Some(
        oag_assets::psarc::Archive::open_file(&file)
            .unwrap_or_else(|e| panic!("{}: {e}", file.display())),
    )
}

#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso and pulse-psp-eu.chd"]
fn hd_disc_data02_authors_pulses_ai_tables() {
    let Some(mut archive) = hd_disc("DATA02") else {
        return;
    };
    assert_same_as_pulse("HD DATA02", &mut archive);
}

/// `DATA03` carries the four race files again, with `SplitScreenMultiplier`
/// added; `AIControlStats.xml` is `DATA02`'s alone.
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso and pulse-psp-eu.chd"]
fn hd_disc_data03_race_files_are_pulses_plus_the_split_screen_row() {
    let (Some(mut archive), Some(reference)) = (hd_disc("DATA03"), pulse()) else {
        return;
    };
    for (name, pulse_blob) in reference.into_iter().skip(1) {
        let path = psarc_path(&name);
        let blob = archive.read_path(&path).expect("DATA03 race file");
        let text = fexml::text(&blob).expect("expands");
        assert!(text.contains("SplitScreenMultiplier"), "{path}");
        assert_eq!(attributes(&blob), attributes(&pulse_blob), "{path}");
    }
}

#[test]
#[ignore = "needs data/extracted/ps3/hd-psn-eu and pulse-psp-eu.chd"]
fn hd_psn_authors_pulses_ai_tables() {
    let Some(mut archive) = loose("data/extracted/ps3/hd-psn-eu/USRDIR/data02.psarc") else {
        return;
    };
    assert_same_as_pulse("HD PSN data02", &mut archive);
}

#[test]
#[ignore = "needs data/extracted/vita/PCSF00007 and pulse-psp-eu.chd"]
fn wipeout_2048_authors_pulses_ai_tables() {
    let Some(mut archive) = loose("data/extracted/vita/PCSF00007/base/PSP2/data.psarc") else {
        return;
    };
    assert_same_as_pulse("2048 EU base", &mut archive);
}

#[test]
#[ignore = "needs data/extracted/ps4/omega-eu and pulse-psp-eu.chd"]
fn omega_authors_pulses_ai_tables() {
    let Some(mut archive) = loose("data/extracted/ps4/omega-eu/uroot/data00.psarc") else {
        return;
    };
    assert_same_as_pulse("Omega data00", &mut archive);
}

/// The reader this project already runs on Pulse parses HD's copy to the
/// identical struct, which is what lets `oag_raceplay`'s finished-player
/// thrust read HD's own file with no title branch.
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso and pulse-psp-eu.chd"]
fn pulses_race_stats_reader_parses_hds_files_to_pulses_struct() {
    let (Some(mut archive), Some(reference)) = (hd_disc("DATA02"), pulse()) else {
        return;
    };
    for (class, (name, pulse_blob)) in SpeedClass::ALL
        .into_iter()
        .zip(reference.into_iter().skip(1))
    {
        let hd = ai_race_stats::from_blob(
            &archive.read_path(&psarc_path(&name)).expect("HD race file"),
            class,
        )
        .expect("HD parses");
        let pulse = ai_race_stats::from_blob(&pulse_blob, class).expect("Pulse parses");
        assert_eq!(hd, pulse, "{name}");
    }
}
