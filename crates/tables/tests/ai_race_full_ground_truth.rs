//! Validates the whole-file reader of
//! [`ai_race_stats`](oag_tables::ai_race_stats) (`parse_class_race`) against
//! the real discs: Pulse's four `AIRaceStats_<class>.xml` on both PSP pressings
//! and the PS2 disc, and the same family on HD, 2048, Omega and Pure.
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
//! page is `docs/ghidra/functions/psp-pulse-usa/ai-stats.md`; the cross-title
//! result is `docs/formats/ai-tables.md`.

mod common;

use common::{outcomes, psarc_ai_files, race_class, race_outcome};
use oag_tables::ai_race_stats::{self, ClassRace};
use oag_tables::fexml;
use oag_tables::handling::SpeedClass;

fn pulse(archive: &mut oag_assets::Archive) -> [ClassRace; 4] {
    SpeedClass::ALL.map(|class| {
        let name = ai_race_stats::entry_name(class);
        let blob = archive
            .read_name(&name)
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        race_class(&blob, class).unwrap_or_else(|e| panic!("{name}: {e}"))
    })
}

fn pulse_usa() -> Option<[ClassRace; 4]> {
    Some(pulse(&mut common::pulse_psp("pulse-psp-usa.chd")?))
}

/// The same four with `SplitScreenMultiplier` taken off, which Pulse does not
/// author.
fn without_split(all: [ClassRace; 4]) -> [ClassRace; 4] {
    all.map(|c| ClassRace {
        split_screen: None,
        ..c
    })
}

/// Reads class `class`'s file from `files` (by lower-cased base name).
fn class_file(files: &[(String, Vec<u8>)], name: &str, class: SpeedClass) -> ClassRace {
    let blob = &files
        .iter()
        .find(|(n, _)| n == name)
        .unwrap_or_else(|| panic!("no {name}"))
        .1;
    race_class(blob, class).unwrap_or_else(|e| panic!("{name}: {e}"))
}

fn per_class_names() -> [String; 4] {
    SpeedClass::ALL.map(|c| format!("airacestats_{}.xml", c.as_str().to_ascii_lowercase()))
}

/// The full record closes on the shape `ai-stats.md` derived: eight places,
/// eight grid slots, three skill points and three pack-swap rows, with every
/// figure finite, and the thrust view is the one the finished-player law uses.
#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn pulse_usa_authors_the_whole_record_for_each_of_the_four_classes() {
    let Some(all) = pulse_usa() else {
        return;
    };
    let mut archive = common::pulse_psp("pulse-psp-usa.chd").unwrap();
    for (class, race) in SpeedClass::ALL.into_iter().zip(all) {
        let name = ai_race_stats::entry_name(class);
        let thin = ai_race_stats::from_blob(&archive.read_name(&name).unwrap(), class).unwrap();
        assert_eq!(race.thrust, thin, "{class:?}: the thrust view is unchanged");
        assert!(race.spread_dist.iter().all(|&v| v > 0.0), "{class:?}");
        assert!(race.grid.iter().all(|g| g.start_boost > 0.0), "{class:?}");
        assert!(
            race.pack_swapping
                .iter()
                .all(|p| p.num_ai_position_swaps >= 1.0 && p.num_intermediate_positions >= 1.0),
            "{class:?}"
        );
        assert!(race.when_leading.speed_mul > 0.0 && race.when_behind.speed_mul > 0.0);
        assert!(race.split_screen.is_none(), "{class:?}: Pulse authors none");
    }
}

#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd and pulse-psp-eu.chd"]
fn pulse_eu_authors_the_usa_records() {
    let (Some(usa), Some(mut eu)) = (pulse_usa(), common::pulse_psp("pulse-psp-eu.chd")) else {
        return;
    };
    assert_eq!(pulse(&mut eu), usa);
}

#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd and pulse-ps2-eu.chd"]
fn pulse_ps2_authors_the_psp_records() {
    let (Some(usa), Some(mut ps2)) = (pulse_usa(), common::pulse_ps2()) else {
        return;
    };
    assert_eq!(pulse(&mut ps2), usa);
}

/// `DATA02` is Pulse's four files plus three more, each refused by name:
/// the combined `airacestats.xml` (`ShipSpacingVariation`), and the `_kart`
/// file, which has no `<RaceBalancing>` at all. `DATA03` re-ships the four with
/// `SplitScreenMultiplier` added.
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso and pulse-psp-usa.chd"]
fn hd_disc_ships_pulses_records_and_refuses_two_families() {
    let (Some(usa), Some(mut data02), Some(mut data03)) = (
        pulse_usa(),
        common::hd_disc("DATA02"),
        common::hd_disc("DATA03"),
    ) else {
        return;
    };
    let files = psarc_ai_files(&mut data02);
    let ok = "ok(1)".to_string();
    assert_eq!(
        outcomes(&files, "airace", race_outcome),
        [
            (
                "airacestats.xml".to_string(),
                "unknown <ShipSpacingVariation>".to_string()
            ),
            ("airacestats_flash.xml".to_string(), ok.clone()),
            (
                "airacestats_kart.xml".to_string(),
                "no <RaceBalancing>".to_string()
            ),
            ("airacestats_phantom.xml".to_string(), ok.clone()),
            ("airacestats_rapier.xml".to_string(), ok.clone()),
            ("airacestats_venom.xml".to_string(), ok.clone()),
        ]
    );
    let hd = per_class_names()
        .iter()
        .zip(SpeedClass::ALL)
        .map(|(n, c)| class_file(&files, n, c))
        .collect::<Vec<_>>();
    assert_eq!(hd, usa);

    let files03 = psarc_ai_files(&mut data03);
    assert_eq!(files03.len(), 4);
    let split = per_class_names()
        .iter()
        .zip(SpeedClass::ALL)
        .map(|(n, c)| class_file(&files03, n, c))
        .collect::<Vec<_>>();
    assert!(split.iter().all(|c| c.split_screen.is_some()));
    assert_eq!(without_split(split.try_into().unwrap()), usa);
}

#[test]
#[ignore = "needs data/extracted/ps3/hd-psn-eu and pulse-psp-usa.chd"]
fn hd_psn_ships_the_same_records() {
    let (Some(usa), Some(mut data02), Some(mut data03)) = (
        pulse_usa(),
        common::loose("data/extracted/ps3/hd-psn-eu/USRDIR/data02.psarc"),
        common::loose("data/extracted/ps3/hd-psn-eu/USRDIR/data03.psarc"),
    ) else {
        return;
    };
    let (f2, f3) = (psarc_ai_files(&mut data02), psarc_ai_files(&mut data03));
    for (class, (name, expected)) in SpeedClass::ALL
        .into_iter()
        .zip(per_class_names().iter().zip(usa))
    {
        assert_eq!(class_file(&f2, name, class), expected, "{name}");
        let with_split = class_file(&f3, name, class);
        assert_eq!(
            ClassRace {
                split_screen: None,
                ..with_split
            },
            expected,
            "{name}"
        );
    }
}

/// 2048 ships the four, a `SuperPhantomStats` pair, and a `..._<class>2048.xml`
/// set equal to the first; the combined and `_kart` files are refused.
fn check_2048(base: &str) {
    let Some(usa) = pulse_usa() else {
        return;
    };
    let Some(mut data) = common::loose(base) else {
        return;
    };
    let files = psarc_ai_files(&mut data);
    let ok = |n: usize| format!("ok({n})");
    let got = outcomes(&files, "airace", race_outcome);
    let want: Vec<(String, String)> = [
        (
            "airacestats.xml",
            "unknown <ShipSpacingVariation>".to_string(),
        ),
        ("airacestats_flash.xml", ok(1)),
        ("airacestats_flash2048.xml", ok(1)),
        ("airacestats_kart.xml", "no <RaceBalancing>".to_string()),
        ("airacestats_phantom.xml", ok(1)),
        ("airacestats_phantom2048.xml", ok(1)),
        ("airacestats_rapier.xml", ok(1)),
        ("airacestats_rapier2048.xml", ok(1)),
        ("airacestats_superphantom.xml", ok(1)),
        ("airacestats_superphantom2048.xml", ok(1)),
        ("airacestats_venom.xml", ok(1)),
        ("airacestats_venom2048.xml", ok(1)),
    ]
    .map(|(n, o)| (n.to_string(), o))
    .into();
    assert_eq!(got, want, "{base}");
    for (class, expected) in SpeedClass::ALL.into_iter().zip(usa) {
        let plain = format!("airacestats_{}.xml", class.as_str().to_ascii_lowercase());
        let tagged = format!(
            "airacestats_{}2048.xml",
            class.as_str().to_ascii_lowercase()
        );
        let a = class_file(&files, &plain, class);
        assert_eq!(a, class_file(&files, &tagged, class), "{base} {plain}");
        assert!(a.split_screen.is_some(), "{base} {plain}");
        assert_eq!(
            ClassRace {
                split_screen: None,
                ..a
            },
            expected,
            "{base} {plain}"
        );
    }
    for name in [
        "airacestats_superphantom.xml",
        "airacestats_superphantom2048.xml",
    ] {
        let text = fexml::text(&files.iter().find(|(n, _)| n == name).unwrap().1).unwrap();
        ai_race_stats::parse_class_race(&text, "SuperPhantomStats")
            .unwrap_or_else(|e| panic!("{base} {name}: {e}"));
    }
}

#[test]
#[ignore = "needs data/extracted/vita/PCSF00007 and pulse-psp-usa.chd"]
fn wipeout_2048_eu_records_are_pulses_plus_split_screen() {
    check_2048("data/extracted/vita/PCSF00007/base/PSP2/data.psarc");
}

#[test]
#[ignore = "needs data/extracted/vita/PCSA00015 and pulse-psp-usa.chd"]
fn wipeout_2048_usa_records_are_pulses_plus_split_screen() {
    check_2048("data/extracted/vita/PCSA00015/base/PSP2/data.psarc");
}

/// Omega's `data00` is HD `DATA03`'s four plus 2048's set; the patch and the
/// other base archives carry none.
#[test]
#[ignore = "needs data/extracted/ps4/omega-eu and pulse-psp-usa.chd"]
fn omega_ships_hds_and_2048s_records_in_data00() {
    let Some(usa) = pulse_usa() else {
        return;
    };
    let Some(mut data00) = common::loose("data/extracted/ps4/omega-eu/uroot/data00.psarc") else {
        return;
    };
    let files = psarc_ai_files(&mut data00);
    let got = outcomes(&files, "airace", race_outcome);
    assert_eq!(got.len(), 12, "{got:?}");
    assert_eq!(got.iter().filter(|(_, o)| o == "ok(1)").count(), 10);
    for (class, expected) in SpeedClass::ALL.into_iter().zip(usa) {
        let name = format!("airacestats_{}.xml", class.as_str().to_ascii_lowercase());
        let c = class_file(&files, &name, class);
        assert_eq!(
            ClassRace {
                split_screen: None,
                ..c
            },
            expected,
            "{name}"
        );
    }
}

/// Pure's single `AIRaceStats.xml` is HD's `_kart` file: a different schema,
/// refused by name on both pressings.
#[test]
#[ignore = "needs data/images/pure-psp-usa.chd, pure-psp-eu.chd and hdfury-ps3-eu-dec.iso"]
fn pure_ships_the_kart_race_family() {
    let Some(mut data02) = common::hd_disc("DATA02") else {
        return;
    };
    let hd = psarc_ai_files(&mut data02)
        .into_iter()
        .find(|(n, _)| n == "airacestats_kart.xml")
        .expect("HD kart file")
        .1;
    for image in ["pure-psp-usa.chd", "pure-psp-eu.chd"] {
        let Some(mut pure) = common::pulse_psp(image) else {
            continue;
        };
        let blob = pure
            .read_name(r"Data\XML\AIRaceStats.xml")
            .expect("Pure file");
        assert_eq!(race_outcome(&blob), "no <RaceBalancing>", "{image}");
        assert_eq!(
            fexml::text(&blob).unwrap(),
            fexml::text(&hd).unwrap(),
            "{image}"
        );
    }
}
