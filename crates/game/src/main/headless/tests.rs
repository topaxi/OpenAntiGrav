//! Disc-backed proof that `write_trace` drives the craft from
//! `--input-script` rather than silently ignoring it.
//!
//! `#[ignore]`d and needs `data/images/pulse-psp-usa.chd`; see
//! `oag_testdata`'s own doc for the skip-or-fail contract this follows
//! through [`oag_testdata::image`]. Run it with:
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all \
//!     -E 'binary(oag-game) and test(trace_out)'
//! ```

use clap::Parser;
use oag_gameplay::ControlScheme;
use oag_raceplay as race;

use super::write_trace;
use crate::cli::Cli;

fn loaded() -> Option<race::Loaded> {
    let image = oag_testdata::image("data/images/pulse-psp-usa.chd")?;
    Some(
        race::load(&race::Options {
            source: image.display().to_string(),
            class: "VENOM".to_string(),
            ..race::Options::default()
        })
        .expect("loading the race"),
    )
}

/// Reads back `column`'s values, in row order, from a `write_trace` CSV.
fn column(csv: &str, column: &str) -> Vec<f32> {
    let mut lines = csv.lines();
    let header: Vec<&str> = lines.next().expect("a header row").split(',').collect();
    let index = header
        .iter()
        .position(|&c| c == column)
        .unwrap_or_else(|| panic!("no {column:?} column in {header:?}"));
    lines
        .map(|line| {
            line.split(',')
                .nth(index)
                .expect("a value in every row")
                .parse::<f32>()
                .expect("a numeric value")
        })
        .collect()
}

/// The bug this pins: `write_trace` used to read `--hold`/`--press` and never
/// look at `cli.input_script` at all, so a scripted `--trace-out` run wrote a
/// plausible-looking CSV of a craft sitting still - no error, right row
/// count, right columns, `throttle`/`steer` zero throughout. See
/// `verification/scenarios/steer-left.inputs`: "200 cross left" from tick 0.
///
/// `oag_race::COUNTDOWN_TICKS` (272) gates thrust regardless of input, so the
/// run goes past it - 300 ticks - and the assertion looks at a row well after
/// the gate opens rather than at tick 0, which is legitimately still zero
/// (`write_trace`'s own "Row alignment" doc: a row is the state *before* the
/// tick it labels).
#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd; run under `just test-data`"]
fn trace_out_drives_the_craft_from_its_input_script() {
    let Some(scripted_race) = loaded() else {
        return;
    };
    // Absolute: nextest runs a bin target's own tests from the crate
    // directory rather than the checkout root, unlike an integration test
    // under `crates/game/tests/`.
    let script = oag_testdata::repo_root().join("verification/scenarios/steer-left.inputs");
    let cli = Cli::parse_from([
        "oag-game".to_string(),
        "--race".to_string(),
        "--input-script".to_string(),
        script.display().to_string(),
        "--ticks".to_string(),
        "300".to_string(),
    ]);
    let path = std::env::temp_dir().join("oag-game-trace-out-input-script-test.csv");
    write_trace(scripted_race, &cli, ControlScheme::default(), &path).expect("writing the trace");
    let csv = std::fs::read_to_string(&path).expect("reading the trace back");

    let throttle = column(&csv, "throttle");
    let steer = column(&csv, "steer");
    // Row 280 is comfortably past both the countdown gate (272) and the
    // script's own start (0), and the script holds "cross left" the whole
    // 300 ticks, so both columns must be nonzero there.
    assert_ne!(
        throttle[280], 0.0,
        "row 280's throttle is 0 even though the script holds cross the whole run - \
         the script is not reaching the ship"
    );
    assert_ne!(
        steer[280], 0.0,
        "row 280's steer is 0 even though the script holds left the whole run - \
         the script is not reaching the ship"
    );

    let Some(unscripted_race) = loaded() else {
        return;
    };
    let cli_unscripted = Cli::parse_from(["oag-game", "--race", "--ticks", "300"]);
    let path2 = std::env::temp_dir().join("oag-game-trace-out-no-script-test.csv");
    write_trace(
        unscripted_race,
        &cli_unscripted,
        ControlScheme::default(),
        &path2,
    )
    .expect("writing the unscripted trace");
    let csv2 = std::fs::read_to_string(&path2).expect("reading the unscripted trace back");
    let steer_unscripted = column(&csv2, "steer");
    // The bug this pins is specifically "the script is ignored", not "steer
    // is broken for everyone" - an unscripted run with no `--hold`/`--press`
    // must still coast straight, so the two runs diverge on the column the
    // script actually drives.
    assert_eq!(
        steer_unscripted[280], 0.0,
        "an unscripted run with no --hold/--press should coast straight"
    );
    assert_ne!(
        steer[280], steer_unscripted[280],
        "the scripted and unscripted runs must diverge on the column the script drives"
    );
}
