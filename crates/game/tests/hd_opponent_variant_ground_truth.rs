//! Wipeout HD/Fury's opponents follow the player's variant family, each on its
//! own variant's tuning.
//!
//! **`#[ignore]`d and never run in CI.** It needs the HD disc image.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --test hd_opponent_variant_ground_truth --run-ignored all
//! ```
//!
//! The rule is `oag_hd::race::TEAM_VARIANTS`'s `opponents_match_player_variant`
//! (static read, confidence 55, `docs/ghidra/functions/ps3-hdfury-eu/opponent-handling.md`;
//! a live RPCS3 check is open): every opponent flies the player's exact
//! variant.

use oag_raceplay as race;
use oag_raceplay::Race;

const IMAGE: &str = "data/images/hdfury-ps3-eu-dec.iso";
const TRACK: &str = r"Data\Environments\Talons_Junction\track.vex";

fn options(image: &std::path::Path, team: &str) -> race::Options {
    race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        track: Some(TRACK.to_string()),
        team: Some(team.to_string()),
        ..race::Options::default()
    }
}

fn suffix(team: &str) -> &'static str {
    if team.ends_with("_c1") {
        "_c1"
    } else if team.ends_with("_n1") {
        "_n1"
    } else {
        ""
    }
}

fn started(image: &std::path::Path, team: &str) -> (Race, Vec<String>) {
    let loaded = race::load(&options(image, team)).expect("loading the race");
    let teams = loaded.setup.slot_teams.clone();
    (Race::start(loaded.setup), teams)
}

/// The tuning a lone craft of `team` flies, as the player.
fn own_handling(image: &std::path::Path, team: &str) -> oag_physics::Handling {
    race::load(&options(image, team))
        .expect("loading the race")
        .setup
        .handling
}

#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn the_opponents_variant_set_follows_the_players_pick() {
    let Some(image) = oag_testdata::image(IMAGE) else {
        return;
    };
    for (player, family) in [("Feisar", ""), ("Feisar_c1", "_c1"), ("Feisar_n1", "_n1")] {
        let (_, teams) = started(&image, player);
        assert_eq!(teams.len(), 8, "{player}: {teams:?}");
        let base = player.trim_end_matches("_c1").trim_end_matches("_n1");
        for team in &teams[1..] {
            assert!(suffix(team) == family, "{player} faces {team}: {teams:?}");
            assert!(
                !team.starts_with(&format!("{base}_")) && team != base,
                "{player} faces its own team as {team}: {teams:?}"
            );
        }
    }
}

#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn every_opponent_flies_its_own_variants_handling_not_the_players() {
    let Some(image) = oag_testdata::image(IMAGE) else {
        return;
    };
    for player in ["Feisar", "Feisar_c1", "Feisar_n1"] {
        let (race, teams) = started(&image, player);
        let mut own = std::collections::BTreeMap::new();
        let mut checked = 0;
        let mut moved = 0;
        for (slot, team) in teams.iter().enumerate().skip(1) {
            let ship = &race.sim.world.ships[slot];
            if !ship.active {
                continue;
            }
            let expected = *own
                .entry(team.clone())
                .or_insert_with(|| own_handling(&image, team));
            assert_eq!(
                ship.handling, expected,
                "{player}: slot {slot} ({team}) is not on its own tuning"
            );
            checked += 1;
            moved += usize::from(ship.handling != race.sim.world.ships[0].handling);
        }
        assert!(
            moved > 0,
            "{player}: every opponent flies the player's tuning"
        );
        assert_eq!(checked, 7, "{player}: seven opponents");
    }
}

#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn hd_and_fury_author_different_numbers_for_one_team() {
    let Some(image) = oag_testdata::image(IMAGE) else {
        return;
    };
    let hd = own_handling(&image, "Feisar");
    let c1 = own_handling(&image, "Feisar_c1");
    let n1 = own_handling(&image, "Feisar_n1");
    println!(
        "VENOM accelcap HD {} c1 {} n1 {}",
        hd.engine.accelcap, c1.engine.accelcap, n1.engine.accelcap
    );
    assert_ne!(hd, c1);
    assert_ne!(hd, n1);
    assert!(
        c1.engine.accelcap > hd.engine.accelcap,
        "Fury's top speed is above HD's"
    );
}

/// Every title, not only a variant one: Pulse's opponents fly their own team's
/// numbers too.
#[test]
#[ignore = "needs data/images/pulse-psp-eu.chd"]
fn pulse_opponents_fly_their_own_teams_handling() {
    let Some(image) = oag_testdata::image("data/images/pulse-psp-eu.chd") else {
        return;
    };
    let options = |team: Option<&str>| race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        team: team.map(str::to_string),
        ..race::Options::default()
    };
    let loaded = race::load(&options(None)).expect("loading the race");
    let teams = loaded.setup.slot_teams.clone();
    let race = Race::start(loaded.setup);
    let mut differing = 0;
    for (slot, team) in teams.iter().enumerate().skip(1) {
        let own = race::load(&options(Some(team)))
            .expect("load")
            .setup
            .handling;
        assert_eq!(
            race.sim.world.ships[slot].handling, own,
            "slot {slot} ({team})"
        );
        differing += usize::from(own != race.sim.world.ships[0].handling);
    }
    assert!(differing > 0, "every Pulse team authors the same numbers");
}
