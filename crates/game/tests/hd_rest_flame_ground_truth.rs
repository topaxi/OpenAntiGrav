//! The engine flare's five shapes are drawn at a standstill on HD.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this project does not ship.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game hd_rest_flame_ground_truth:: --run-ignored all
//! ```
//!
//! The original submits all five `flame_test` draws of Harimau classic at 0 km/h from the first
//! race frame (RPCS3 draw lists, 2026-10-20, `hd-ship-materials.md` "The nozzle jewel, resolved").
//! The same grid frame with the flame's material filtered out (`OAG_SKIP_MATERIAL=flame_01`) must
//! submit at least five draws fewer; a rest gate on the flame makes the two equal.

fn draws(skip: Option<&str>) -> Option<u64> {
    let image = oag_testdata::image("data/images/hdfury-ps3-eu-dec.iso")?;
    let scratch = std::env::temp_dir().join("oag-hd-rest-flame");
    let mut run = std::process::Command::new(env!("CARGO_BIN_EXE_oag-game"));
    run.arg(&image)
        .args(["--race", "--no-audio", "--no-intro", "--team", "harimau"])
        .args(["--size", "480x272", "--ticks", "5"])
        .arg("--screenshot")
        .arg(scratch.join("frame.png"))
        .env("XDG_CONFIG_HOME", scratch.join("config"))
        .env("XDG_DATA_HOME", scratch.join("data"))
        .env("XDG_STATE_HOME", scratch.join("state"));
    if let Some(needle) = skip {
        run.env("OAG_SKIP_MATERIAL", needle);
    }
    std::fs::create_dir_all(&scratch).ok()?;
    let out = run.output().expect("running oag-game");
    assert!(out.status.success(), "oag-game failed");
    let log = String::from_utf8_lossy(&out.stderr);
    let line = log.lines().find(|l| l.contains("frame at tick"))?;
    line.split("tick 5: ")
        .nth(1)?
        .split(' ')
        .next()?
        .parse()
        .ok()
}

#[test]
#[ignore = "needs the HD disc image"]
fn the_flame_is_drawn_at_a_standstill() {
    let Some(with) = draws(None) else {
        return;
    };
    let without = draws(Some("flame_01")).expect("the filtered frame");
    assert!(
        with >= without + 5,
        "{with} draws with the flame, {without} without: the rest flame is not drawn"
    );
}
