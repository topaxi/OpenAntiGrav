//! Wipeout HD's `xfship_<team>.xfx` crossfade tables, every one on the disc.
//! (Thirteen files, not twelve: the lane brief and the old handover miscounted.)
//!
//! **`#[ignore]`d and never run in CI.** They need game content, which this
//! project does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! ```
//!
//! The evidence page is `docs/formats/hd-xfx.md`. The headline: the reader
//! accounts for every byte of all thirteen files, and feisar's 2,100-byte
//! difference is exactly one layer record (`0x830`) plus its pointer (`4`).

use std::path::PathBuf;

use oag_formats::byte_order::ByteOrder;
use oag_formats::sblk::Bank;
use oag_formats::xfx::{self, Xfx};

/// The thirteen team tables HD ships: `xfship_<team>.xfx`, twelve in
/// `DATA01.PSARC` and `det` in `DATA00.PSARC`.
const TEAMS: [&str; 13] = [
    "ag_systems",
    "assegai",
    "auricom",
    "det",
    "egx",
    "feisar",
    "goteki",
    "harimau",
    "icaras",
    "mirage",
    "piranha",
    "qirex",
    "triakis",
];

fn image() -> Option<PathBuf> {
    oag_testdata::image("data/images/hdfury-ps3-eu-dec.iso")
}

/// Read one entry out of whichever PSARC carries it.
fn entry(iso: &std::path::Path, path: &str) -> Option<Vec<u8>> {
    for n in 0..7 {
        let spec = format!("{}:PS3_GAME/USRDIR/DATA0{n}.PSARC", iso.display());
        let Ok(mut archive) = oag_assets::psarc::Archive::open(&spec) else {
            continue;
        };
        if archive.paths().iter().any(|p| p == path) {
            return archive.read_path(path).ok();
        }
    }
    None
}

fn every_table(iso: &std::path::Path) -> Vec<(String, Vec<u8>)> {
    TEAMS
        .iter()
        .copied()
        .map(|team| {
            let path = format!("/data/sound/xfship_{team}.xfx");
            let bytes = entry(iso, &path).unwrap_or_else(|| panic!("{path} is missing"));
            (team.to_string(), bytes)
        })
        .collect()
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn all_thirteen_tables_parse_and_account_for_every_byte() {
    let Some(iso) = image() else {
        return;
    };
    let tables = every_table(&iso);
    assert_eq!(tables.len(), 13);
    for (team, bytes) in &tables {
        let xfx = Xfx::parse(bytes).unwrap_or_else(|e| panic!("{team}: {e}"));
        assert_eq!(
            xfx.accounted_bytes(),
            bytes.len(),
            "{team}: bytes unaccounted for"
        );
        assert_eq!(xfx.channels().len(), 4, "{team}");
    }
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn feisar_is_one_layer_smaller_and_that_is_the_whole_difference() {
    let Some(iso) = image() else {
        return;
    };
    let tables = every_table(&iso);
    let size = |team: &str| {
        tables
            .iter()
            .find(|(t, _)| t == team)
            .map(|(_, b)| b.len())
            .unwrap()
    };
    assert_eq!(size("goteki"), 20_976);
    assert_eq!(size("goteki") - size("feisar"), xfx::LAYER_LEN + 4);
    for (team, bytes) in &tables {
        let layers = Xfx::parse(bytes).unwrap().layers().len();
        assert_eq!(layers, if team == "feisar" { 8 } else { 9 }, "{team}");
    }
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_channel_assignment_is_jet_then_two_resonances_then_the_afterburner() {
    let Some(iso) = image() else {
        return;
    };
    for (team, bytes) in every_table(&iso) {
        let xfx = Xfx::parse(&bytes).unwrap();
        let on = |c: usize| -> Vec<&str> { xfx.layers_on(c).map(|l| l.name()).collect() };
        assert_eq!(on(1), ["~ABResLoL", "~ABResHiL"], "{team}");
        assert_eq!(on(2), ["~ABResLoR", "~ABResHiR"], "{team}");
        let jets = on(0);
        assert!(jets[0].starts_with("~jet"), "{team}: {jets:?}");
        assert!(jets[1].starts_with("~jet"), "{team}: {jets:?}");
        // Detonator carries its afterburner on channel 0; everyone else on 3.
        let afterburner = if team == "det" { 0 } else { 3 };
        assert!(on(afterburner).contains(&"~afterburner"), "{team}");
    }
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn no_channel_zero_trigger_names_a_sound() {
    let Some(iso) = image() else {
        return;
    };
    for (team, bytes) in every_table(&iso) {
        let xfx = Xfx::parse(&bytes).unwrap();
        assert_eq!(xfx.channels()[0].trigger_count(), 26, "{team}");
        for c in &xfx.channels()[1..] {
            assert_eq!(c.trigger_count(), 0, "{team}");
        }
        for trigger in xfx.channels()[0].triggers() {
            // `+0x2b` is the byte the game tests for a name, `+0x3c` the cue
            // index it falls back to; both are empty, so the trigger path in
            // `XFadeSystem_Trigger` returns without starting a voice.
            assert_eq!(trigger[0x2b], 0, "{team}");
            assert!(ByteOrder::Big.i16(trigger, 0x3c) < 0, "{team}");
        }
    }
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn every_layer_name_is_a_cue_in_the_ship_sound_bank() {
    let Some(iso) = image() else {
        return;
    };
    let bank_bytes = entry(&iso, "/data/sound/shiphd.bnk").expect("shiphd.bnk");
    let bank = Bank::parse_as(&bank_bytes, ByteOrder::Big).expect("shiphd.bnk parses");
    let names: std::collections::BTreeSet<String> =
        bank.sound_names().into_iter().map(|n| n.name).collect();
    assert!(!names.is_empty(), "the bank has a name table");
    for (team, bytes) in every_table(&iso) {
        for layer in Xfx::parse(&bytes).unwrap().layers() {
            assert!(
                names.contains(layer.name()),
                "{team}: layer {:?} is not in shiphd.bnk",
                layer.name()
            );
        }
    }
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn every_curve_is_in_range_and_gain_never_exceeds_unity() {
    let Some(iso) = image() else {
        return;
    };
    for (team, bytes) in every_table(&iso) {
        for layer in Xfx::parse(&bytes).unwrap().layers() {
            assert!(
                layer.gain().all(|g| (0..=0x400).contains(&g)),
                "{team} {}",
                layer.name()
            );
            assert!(
                layer.pitch().all(|p| (0..=0x400).contains(&p)),
                "{team} {}",
                layer.name()
            );
        }
    }
}
