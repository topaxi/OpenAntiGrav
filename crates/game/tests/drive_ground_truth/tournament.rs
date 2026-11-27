//! A Tournament leg that will not load ends the tournament.

use std::path::Path;

use serde_json::Value;

use super::{drive_with, pulse, text};

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
