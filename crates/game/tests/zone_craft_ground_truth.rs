//! Zone flies one shared hull on 2048 v1.04 and the Omega Collection, whatever
//! craft the player picked.
//!
//! **`#[ignore]`d and never run in CI**: needs the extracted packages under
//! `data/extracted/`. Run with
//! `OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all -E 'binary(zone_craft_ground_truth)'`.
//!
//! The law is [`oag_title::ZoneCraft::OwnShipAt`], read off both executables'
//! ship-model loader (`docs/ghidra/functions/vita-2048-eu-v104/zone-craft.md`,
//! `docs/ghidra/functions/ps4-omega-eu/zone-craft.md`): game mode 6 names
//! `Data\art\published\hdships\Zone\Ship.vex` without looking at the craft.
//! Each case asserts both halves, because either alone passes on a build that
//! fell back to the other: a Zone race does not name the race hull, and the
//! name it does build is in the archive set and not empty.

use oag_race::Mode;

const ZONE_HULL: &str = r"Data\art\published\hdships\Zone\Ship.vex";

fn check(
    title: &str,
    defaults: &oag_title::RaceDefaults,
    archives: &mut oag_assets::Archives,
    teams: &[&str],
) {
    for team in teams {
        let paths = defaults.ships_for(team);
        let racing = oag_livery::entry::ship_entry_name(paths, team, Mode::TimeTrial, None);
        let zoning = oag_livery::entry::ship_entry_name(paths, team, Mode::Zone, None);
        assert_ne!(
            racing, zoning,
            "{title}/{team}: Zone must not fly the race hull"
        );
        assert_eq!(
            zoning, ZONE_HULL,
            "{title}/{team}: every craft flies one Zone hull"
        );
        let blob = archives
            .read_name(&zoning)
            .unwrap_or_else(|e| panic!("{title}/{team}: reading {zoning}: {e}"));
        assert!(!blob.is_empty(), "{title}/{team}: {zoning} is empty");
    }
}

#[test]
#[ignore = "needs the decrypted Vita package in data/extracted/vita/"]
fn vita_2048_zone_hull_is_shared_by_native_and_guest_craft() {
    let Some(path) = oag_testdata::exact("data/extracted/vita/PCSF00007") else {
        return;
    };
    let mut archives = oag_2048::open(&path.display().to_string()).expect("opening 2048");
    check(
        "2048",
        oag_2048::race::DEFAULTS,
        &mut archives,
        &[
            r"feisar2048\1",
            r"auricom2048\4",
            oag_2048::race::DEFAULT_TEAM,
            "Assegai",
        ],
    );
}

#[test]
#[ignore = "needs the corrected Omega extraction"]
fn omega_zone_hull_is_shared_by_hd_and_2048_era_craft() {
    let Some(path) = oag_testdata::exact("data/extracted/ps4") else {
        return;
    };
    let mut archives = oag_omega::open(&path.display().to_string()).expect("opening Omega");
    check(
        "Omega",
        oag_omega::race::DEFAULTS,
        &mut archives,
        &[oag_omega::race::DEFAULTS.team, "assegai"],
    );
}
