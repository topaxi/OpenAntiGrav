//! Every seated person takes a back row of the grid, whichever way the screen is cut into
//! windows and views. Disc-backed: the grid is a circuit's own.
//!
//! The grid is `people` rows from the back, so the opponents' rows are exactly `1` to
//! `8 - people`. `people` is the number of seats across every window, not the view count of
//! one: `--window-seats 1,1` is two people on one view each window, and before this it counted
//! one and left the second person on the front row as an opponent.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game window_seats_grid_ground_truth:: --run-ignored all
//! ```

/// The row each seat starts on, from the log's `people on the grid: 1:8 2:7 ...` (seat:row,
/// counted from the front), in seat order.
fn seat_rows(mode: &str, extra: &[&str]) -> Option<Vec<usize>> {
    let image = oag_testdata::image("data/images/pulse-psp-usa.chd")?;
    let dir = std::env::temp_dir().join("oag-window-seats-grid-ground-truth");
    std::fs::create_dir_all(&dir).expect("a temp directory");
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_oag-game"))
        .arg(image)
        .args([
            "--no-audio",
            "--race",
            "--mode",
            "single_race",
            "--class",
            "venom",
        ])
        .args(["--size", "320x180", "--ticks", "2", "--screenshot"])
        .arg(dir.join("grid.png"))
        .args(extra)
        .env("XDG_CONFIG_HOME", dir.join("config"))
        .env("XDG_STATE_HOME", dir.join("state"))
        .env("XDG_DATA_HOME", dir.join("data"))
        .output()
        .expect("running oag-game");
    assert!(out.status.success(), "oag-game {extra:?}");
    let log =
        String::from_utf8_lossy(&out.stderr).into_owned() + &String::from_utf8_lossy(&out.stdout);
    let line = log
        .lines()
        .find_map(|line| line.split("people on the grid: ").nth(1))
        .unwrap_or_else(|| panic!("no people line for {extra:?} in:\n{log}"));
    Some(
        line.split_whitespace()
            .filter_map(|entry| entry.split(':').nth(1)?.parse().ok())
            .collect(),
    )
}

/// Seats across windows (`1,1`, `2,2`, `3,1`) and one window's `--players 4` all put the people on
/// the back rows in seat order: 8, 7, 6, ... Fails on the build where the head count was the
/// first window's view count, which put `1,1`'s second person on the front row.
#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn every_seat_across_every_window_takes_a_back_row() {
    for (extra, people) in [
        (vec!["--window-seats", "1,1"], 2),
        (vec!["--window-seats", "2,2"], 4),
        (vec!["--window-seats", "3,1"], 4),
        (vec!["--players", "4"], 4),
        (vec!["--players", "2"], 2),
    ] {
        let Some(rows) = seat_rows("single_race", &extra) else {
            return;
        };
        let back: Vec<usize> = (0..people).map(|seat| 8 - seat).collect();
        assert_eq!(
            rows, back,
            "{extra:?}: {people} people take the back rows in order"
        );
    }
}

/// Head to Head across two windows is the two people's two craft on rows 8 and 7: the same count
/// reaches the grid, so the second person is not left on the front row as the AI opponent.
#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn head_to_head_across_two_windows_seats_both_people_at_the_back() {
    let Some(rows) = seat_rows("head_to_head", &["--window-seats", "1,1"]) else {
        return;
    };
    assert_eq!(rows, [8, 7]);
}
