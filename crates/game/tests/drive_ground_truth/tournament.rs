//! A Tournament ends with its run: a leg that will not load ends it, and a leg watched again
//! comes back to its own end menu with the tournament still going.

use std::path::Path;

use serde_json::Value;

use super::{drive_args, drive_with, pulse, text};

const MOD: &str = r#"
toml-schema = { location = "https://oag.topaxi.com/mods/schemas/v1/mod.tosd", version = "1.0.0" }

[mod]
id = "test.broken-leg"
name = "Broken leg"
version = "1.0.0"
authors = ["test"]
license = "MIT"
description = "A two-leg tournament whose first circuit an override breaks."
titles = ["wipeout-pulse"]
"#;

const CAMPAIGN: &str = r#"
toml-schema = { location = "https://oag.topaxi.com/mods/schemas/v1/campaign.tosd", version = "1.0.0" }

[campaign]
id = "broken"
name = "BROKEN LEG"

[[grid]]
id = "only"
name = "Only"

[[grid.event]]
slot = [2, 1]
id = "tour"
name = "Tour"
mode = "tournament"
tracks = ["16_Track", "03_Track"]
class = "venom"
unlock = { open = true }
medals = { gold = 1, silver = 2, bronze = 3 }
"#;

fn write(path: &Path, contents: &str) {
    std::fs::create_dir_all(path.parent().expect("a parent")).expect("a directory");
    std::fs::write(path, contents).expect("a fixture file");
}

/// The Tournament cell's first leg is a circuit that will not load (a player's override that is
/// not a track). The game goes back to the menus with no tournament, and the Race Box race
/// launched next is no leg of it: a tournament left behind would offer that race NEXT RACE and
/// fold its board into the old standings. Fails on a build whose failed load keeps the
/// tournament.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_tournament_leg_that_will_not_load_ends_the_tournament() {
    let Some(image) = pulse() else {
        return;
    };
    let fixture = oag_testdata::scratch::dir("drive-broken-leg-fixture");
    let mods = fixture.join("mods");
    write(&mods.join("broken-leg/oag-mod.toml"), MOD);
    write(&mods.join("broken-leg/campaigns/broken.toml"), CAMPAIGN);
    let overrides = fixture.join("overrides");
    write(
        &overrides.join("wipeout-pulse/Data/Environments/16_Track/track.vex"),
        "not a track",
    );

    let run = drive_with(
        &image,
        "broken-leg",
        "boot; select campaign; press cross; until page=campaigns; \
         select campaign-custom:test.broken-leg/broken; press cross; until screen=campaign; \
         ticks 60; press cross; until screen.id=ship-select; press cross; \
         until stage=loading within 600; until stage=menu within 3000; ticks 30; \
         select racebox; press cross; until page=race; set race.mode=single_race; \
         select launch_race; press cross; until screen=picker; select 03_Track; press cross; \
         until screen.id=ship-select; press cross; until stage=race within 4000; ticks 5",
        None,
        &[
            ("OAG_MODS", mods.to_str().expect("a path")),
            ("OAG_OVERRIDES", overrides.to_str().expect("a path")),
        ],
    );
    assert!(run.output.status.success(), "{}", run.why_not_success());
    assert!(
        run.stderr().contains("cannot start a race"),
        "the leg was meant to fail to load:\n{}",
        run.stderr()
    );
    let stage = |state: &Value| text(state, &["stage"]).map(str::to_string);
    let tournament = |state: &Value| text(state, &["tournament"]).map(str::to_string);
    let loading = run
        .states
        .iter()
        .position(|state| stage(state).as_deref() == Some("loading") && state["race"].is_object())
        .expect("the leg began loading");
    assert_eq!(
        tournament(&run.states[loading]).as_deref(),
        Some("1/2"),
        "the load that fails is a tournament's first leg: {}",
        run.states[loading]
    );
    let back = run.states[loading..]
        .iter()
        .find(|state| stage(state).as_deref() == Some("menu"))
        .expect("the failed load returns to the menus");
    assert_eq!(tournament(back), None, "{back}");
    let last = run.last();
    assert_eq!(stage(last).as_deref(), Some("race"), "{last}");
    assert_eq!(text(last, &["race", "mode"]), Some("single_race"), "{last}");
    assert_eq!(tournament(last), None, "{last}");
}

const WATCH_MOD: &str = r#"
toml-schema = { location = "https://oag.topaxi.com/mods/schemas/v1/mod.tosd", version = "1.0.0" }

[mod]
id = "test.watched-leg"
name = "Watched leg"
version = "1.0.0"
authors = ["test"]
license = "MIT"
description = "A two-leg one-lap tournament, for watching a leg again between the legs."
titles = ["wipeout-pulse"]
"#;

const WATCH_CAMPAIGN: &str = r#"
toml-schema = { location = "https://oag.topaxi.com/mods/schemas/v1/campaign.tosd", version = "1.0.0" }

[campaign]
id = "watched"
name = "WATCHED LEG"

[[grid]]
id = "only"
name = "Only"

[[grid.event]]
slot = [2, 1]
id = "tour"
name = "Tour"
mode = "tournament"
tracks = ["03_Track", "16_Track"]
laps = 1
class = "venom"
unlock = { open = true }
medals = { gold = 1, silver = 2, bronze = 3 }
"#;

/// Through a finished leg's end screens to `EndRace Menu`.
const TO_THE_END_MENU: &str = "until endrace=photo within 20000; ticks 100; press cross; ticks 100; \
     press cross; ticks 100; press cross; until endrace_options=ER_RETURN_GRID within 600; ticks 10";

/// Down from the first row to WATCH REPLAY, the last, and confirmed.
const WATCH: &str = "press down; ticks 10; press down; ticks 10; press down; ticks 10; \
     expect endrace_option=OAG_ENDRACE_WATCH_REPLAY; press cross; until replay=playing within 4000; \
     ticks 30; escape";

/// A Tournament leg watched again from its end menu (WATCH REPLAY) comes back to that end menu
/// with NEXT RACE, which races the next leg on the same standings (ADR-0099, migration step 4).
/// The watch is pushed over the finished leg and popped back to it; a build whose watch replaces
/// the leg fails here, at the menus or at the REPLAYS page the watch would land on. On the last
/// leg the same watch comes back to its end menu, and leaving that ends the tournament.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_tournament_leg_watched_again_comes_back_to_its_next_race() {
    let Some(image) = pulse() else {
        return;
    };
    let fixture = oag_testdata::scratch::dir("drive-watched-leg-fixture");
    let mods = fixture.join("mods");
    write(&mods.join("watched-leg/oag-mod.toml"), WATCH_MOD);
    write(
        &mods.join("watched-leg/campaigns/watched.toml"),
        WATCH_CAMPAIGN,
    );

    let run = drive_args(
        &image,
        "watched-leg",
        &format!(
            "boot; select campaign; press cross; until page=campaigns; \
             select campaign-custom:test.watched-leg/watched; press cross; until screen=campaign; \
             ticks 60; press cross; until screen.id=ship-select; press cross; \
             until stage=race within 4000; {TO_THE_END_MENU}; {WATCH}; \
             until endrace_options=ER_NEXT_RACE within 600; ticks 10; press down; ticks 10; \
             expect endrace_option=ER_NEXT_RACE; press cross; \
             until stage=loading within 600; until stage=race within 4000; {TO_THE_END_MENU}; \
             {WATCH}; until endrace_options=ER_RETURN_GRID within 600; ticks 10; press down; \
             ticks 10; expect endrace_option=ER_RETURN_GRID; press cross; \
             until stage=menu within 600; ticks 10"
        ),
        None,
        &[("OAG_MODS", mods.to_str().expect("a path"))],
        &["--autopilot"],
    );
    assert!(run.output.status.success(), "{}", run.why_not_success());
    let stage = |state: &Value| text(state, &["stage"]).map(str::to_string);
    let tournament = |state: &Value| text(state, &["tournament"]).map(str::to_string);
    let points = |state: &Value| state["tournament_points"].as_u64();
    let offers = |state: &Value, id: &str| {
        state["endrace_options"]
            .as_array()
            .is_some_and(|rows| rows.iter().any(|row| row == id))
    };
    let watches: Vec<usize> = run
        .states
        .iter()
        .enumerate()
        .filter(|(at, state)| {
            text(state, &["replay"]) == Some("playing")
                && *at > 0
                && text(&run.states[at - 1], &["replay"]) != Some("playing")
        })
        .map(|(at, _)| at)
        .collect();
    assert_eq!(watches.len(), 2, "one watch after each leg");
    let leg_two = run
        .states
        .iter()
        .position(|state| tournament(state).as_deref() == Some("2/2"))
        .expect("the tournament reached its second leg");
    assert!(watches[0] < leg_two && leg_two < watches[1]);
    // Back from the first watch: the leg's own end menu, NEXT RACE first, its standings kept.
    let back = run.states[watches[0]..leg_two]
        .iter()
        .find(|state| offers(state, "ER_NEXT_RACE"))
        .expect("the watch came back to the leg's end menu");
    assert_eq!(tournament(back).as_deref(), Some("1/2"), "{back}");
    let leg_one = points(back).expect("the standings after leg 1");
    assert!(leg_one > 0, "leg 1 scored: {back}");
    assert!(
        run.states[watches[0]..leg_two]
            .iter()
            .all(|state| text(state, &["page"]) != Some("replays")),
        "the watch did not land on REPLAYS"
    );
    // Leg 2 starts on leg 1's standings and adds to them.
    assert_eq!(
        points(&run.states[leg_two]),
        Some(leg_one),
        "{}",
        run.states[leg_two]
    );
    let end_two = run.states[leg_two..watches[1]]
        .iter()
        .rev()
        .find(|state| offers(state, "ER_RETURN_GRID"))
        .expect("leg 2's end menu");
    assert_eq!(tournament(end_two).as_deref(), Some("2/2"), "{end_two}");
    assert!(
        points(end_two).is_some_and(|total| total > leg_one),
        "the standings after leg 2 count leg 1 and leg 2: {end_two}"
    );
    assert!(!offers(end_two, "ER_NEXT_RACE"), "the last leg: {end_two}");
    // Back from the last leg's watch to its end menu, still the tournament's, and leaving that
    // ends it.
    let back = run.states[watches[1]..]
        .iter()
        .find(|state| offers(state, "ER_RETURN_GRID"))
        .expect("the last watch came back to the last leg's end menu");
    assert_eq!(tournament(back).as_deref(), Some("2/2"), "{back}");
    let last = run.last();
    assert_eq!(stage(last).as_deref(), Some("menu"), "{last}");
    assert_eq!(
        tournament(last),
        None,
        "no tournament outlives its run: {last}"
    );
    // Each leg is recorded once: a pop back to a finished leg does not re-arm its result
    // (ADR-0099, decision 4), or each watch would count the leg's lap again.
    let records = std::fs::read_to_string(run.profile.join("config/oag/records.toml"))
        .expect("the records file");
    let records: toml::Table = toml::from_str(&records).expect("records parse");
    let laps = records["lifetime"]
        .as_array()
        .and_then(|rows| {
            rows.iter()
                .find(|row| row["title"].as_str() == Some("wipeout pulse"))
        })
        .and_then(|row| row["laps"].as_integer());
    assert_eq!(laps, Some(2), "two one-lap legs, each recorded once");
}
