//! Validates [`ai_control_stats`](oag_tables::ai_control_stats) against the
//! real discs: Pulse's `AIControlStats.xml` on both PSP pressings and the PS2
//! disc, and the same family on HD, 2048 and Omega.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! ```
//!
//! No tuning value is written in this file: titles are compared against
//! Pulse's own parse, and the census is of names and outcomes. The evidence
//! page is `docs/ghidra/functions/psp-pulse-usa/ai-controls.md`; the cross-title
//! result is `docs/formats/ai-tables.md`.

mod common;

use common::{base_name, control_outcome, outcomes, psarc_ai_files};
use oag_tables::ai_control_stats::{self, Controller};
use oag_tables::fexml;
use oag_tables::handling::SpeedClass;

const NAME: &str = r"Data\XML\AIControlStats.xml";

fn controllers(blob: &[u8]) -> [Controller; 4] {
    SpeedClass::ALL.map(|class| {
        ai_control_stats::from_blob(blob, class).unwrap_or_else(|e| panic!("{class:?}: {e}"))
    })
}

fn pulse_usa() -> Option<[Controller; 4]> {
    let blob = common::pulse_psp("pulse-psp-usa.chd")?
        .read_name(NAME)
        .expect("USA file");
    Some(controllers(&blob))
}

/// Every class authors the four steering figures and `xtrackDamp`; `xtrackMax`
/// is never authored, so it stays `None` and is the consumer's default.
#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn pulse_usa_authors_a_controller_for_each_of_the_four_classes() {
    let Some(all) = pulse_usa() else {
        return;
    };
    for (class, c) in SpeedClass::ALL.into_iter().zip(all) {
        for (field, value) in [
            ("steer_mul", c.steer_mul),
            ("steer_damp", c.steer_damp),
            ("look_ahead_secs", c.look_ahead_secs),
            ("xtrack_mul", c.xtrack_mul),
            ("xtrack_damp", c.xtrack_damp),
        ] {
            assert!(value.is_some_and(|v| v > 0.0), "{class:?} {field}");
        }
        assert_eq!(c.xtrack_max, None, "{class:?}");
    }
}

#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd and pulse-psp-eu.chd"]
fn pulse_eu_authors_the_usa_controllers() {
    let (Some(usa), Some(mut eu)) = (pulse_usa(), common::pulse_psp("pulse-psp-eu.chd")) else {
        return;
    };
    assert_eq!(controllers(&eu.read_name(NAME).expect("EU file")), usa);
}

#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd and pulse-ps2-eu.chd"]
fn pulse_ps2_authors_the_psp_controllers() {
    let (Some(usa), Some(mut ps2)) = (pulse_usa(), common::pulse_ps2()) else {
        return;
    };
    let blob = ps2.read_name(NAME).expect("PS2 file");
    assert!(!fexml::is_fexml(&blob), "PS2 stores plain text");
    assert_eq!(controllers(&blob), usa);
}

/// `DATA02` ships two controller files: Pulse's own, and the `_kart` one,
/// whose `<ControllerPO>` rows are a different controller no Pulse reader
/// knows, refused by name.
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso and pulse-psp-usa.chd"]
fn hd_disc_ships_pulses_controllers_and_a_kart_family_it_refuses() {
    let (Some(usa), Some(mut data02)) = (pulse_usa(), common::hd_disc("DATA02")) else {
        return;
    };
    let files = psarc_ai_files(&mut data02);
    assert_eq!(
        outcomes(&files, "aicontrol", control_outcome),
        [
            ("aicontrolstats.xml".to_string(), "ok(4)".to_string()),
            (
                "aicontrolstats_kart.xml".to_string(),
                "unknown <ControllerPO>".to_string()
            ),
        ]
    );
    let hd = files
        .iter()
        .find(|(n, _)| n == "aicontrolstats.xml")
        .unwrap();
    assert_eq!(controllers(&hd.1), usa);
    assert!(
        psarc_ai_files(&mut common::hd_disc("DATA03").expect("DATA03"))
            .iter()
            .all(|(n, _)| !n.starts_with("aicontrol")),
        "DATA03 ships no controller file"
    );
}

#[test]
#[ignore = "needs data/extracted/ps3/hd-psn-eu and pulse-psp-usa.chd"]
fn hd_psn_ships_the_same_two_controller_files() {
    let (Some(usa), Some(mut data02)) = (
        pulse_usa(),
        common::loose("data/extracted/ps3/hd-psn-eu/USRDIR/data02.psarc"),
    ) else {
        return;
    };
    let files = psarc_ai_files(&mut data02);
    assert_eq!(
        outcomes(&files, "aicontrol", control_outcome).len(),
        2,
        "controller files"
    );
    let hd = files
        .iter()
        .find(|(n, _)| n == "aicontrolstats.xml")
        .unwrap();
    assert_eq!(controllers(&hd.1), usa);
}

/// 2048 adds `SuperPhantomStats`, a copy of Phantom's, and an
/// `AIControlStats2048.xml` that is the same file again.
fn check_2048(base: &str) {
    let Some(usa) = pulse_usa() else {
        return;
    };
    let Some(mut data) = common::loose(base) else {
        return;
    };
    let files = psarc_ai_files(&mut data);
    assert_eq!(
        outcomes(&files, "aicontrol", control_outcome),
        [
            ("aicontrolstats.xml".to_string(), "ok(5)".to_string()),
            ("aicontrolstats2048.xml".to_string(), "ok(5)".to_string()),
            (
                "aicontrolstats_kart.xml".to_string(),
                "unknown <ControllerPO>".to_string()
            ),
        ],
        "{base}"
    );
    for name in ["aicontrolstats.xml", "aicontrolstats2048.xml"] {
        let blob = &files.iter().find(|(n, _)| n == name).unwrap().1;
        assert_eq!(controllers(blob), usa, "{base} {name}");
        let text = fexml::text(blob).unwrap();
        let sp = ai_control_stats::parse_element(&text, "SuperPhantomStats").expect("SuperPhantom");
        assert_eq!(sp, usa[3], "{base} {name}: SuperPhantom is Phantom's");
    }
}

#[test]
#[ignore = "needs data/extracted/vita/PCSF00007 and pulse-psp-usa.chd"]
fn wipeout_2048_eu_controllers_are_pulses_plus_super_phantom() {
    check_2048("data/extracted/vita/PCSF00007/base/PSP2/data.psarc");
}

#[test]
#[ignore = "needs data/extracted/vita/PCSA00015 and pulse-psp-usa.chd"]
fn wipeout_2048_usa_controllers_are_pulses_plus_super_phantom() {
    check_2048("data/extracted/vita/PCSA00015/base/PSP2/data.psarc");
}

/// Omega's `data00` carries 2048's three controller files; the other eight
/// archives of the base package and day-one patch carry none.
#[test]
#[ignore = "needs data/extracted/ps4/omega-eu and pulse-psp-usa.chd"]
fn omega_ships_2048s_controller_files_in_data00_alone() {
    let Some(usa) = pulse_usa() else {
        return;
    };
    let Some(mut data00) = common::loose("data/extracted/ps4/omega-eu/uroot/data00.psarc") else {
        return;
    };
    let files = psarc_ai_files(&mut data00);
    let names: Vec<_> = files
        .iter()
        .filter(|(n, _)| n.starts_with("aicontrol"))
        .map(|(n, _)| n.as_str())
        .collect();
    assert_eq!(
        names,
        [
            "aicontrolstats.xml",
            "aicontrolstats2048.xml",
            "aicontrolstats_kart.xml"
        ]
    );
    let blob = &files
        .iter()
        .find(|(n, _)| n == "aicontrolstats.xml")
        .unwrap()
        .1;
    assert_eq!(controllers(blob), usa);
    for archive in [
        "omega-eu/uroot/data01",
        "omega-eu/uroot/data02",
        "omega-eu/uroot/data03",
        "omega-eu/uroot/data04",
        "omega-eu-patch/uroot/data05",
        "omega-eu-patch/uroot/data07",
        "omega-eu-patch/uroot/data08",
        "omega-eu-patch/uroot/data09",
    ] {
        let path = format!("data/extracted/ps4/{archive}.psarc");
        let mut other = common::loose(&path).expect("archive present beside data00");
        assert!(
            psarc_ai_files(&mut other).is_empty(),
            "{archive} ships an AI file"
        );
    }
}

/// Pure ships one `AIControlStats.xml` and it is the `_kart` family, not
/// Pulse's: refused by name on both pressings, and byte-for-byte the text of
/// HD's `aicontrolstats_kart.xml`.
#[test]
#[ignore = "needs data/images/pure-psp-usa.chd, pure-psp-eu.chd and hdfury-ps3-eu-dec.iso"]
fn pure_ships_the_kart_controller_family() {
    let Some(mut data02) = common::hd_disc("DATA02") else {
        return;
    };
    let hd = psarc_ai_files(&mut data02)
        .into_iter()
        .find(|(n, _)| n == "aicontrolstats_kart.xml")
        .expect("HD kart file")
        .1;
    for image in ["pure-psp-usa.chd", "pure-psp-eu.chd"] {
        let Some(mut pure) = common::pulse_psp(image) else {
            continue;
        };
        let blob = pure.read_name(NAME).expect("Pure file");
        assert_eq!(control_outcome(&blob), "unknown <ControllerPO>", "{image}");
        assert_eq!(
            fexml::text(&blob).unwrap(),
            fexml::text(&hd).unwrap(),
            "{image}"
        );
        assert_eq!(base_name(NAME), "aicontrolstats.xml");
    }
}
