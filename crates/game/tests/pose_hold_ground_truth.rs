//! `--pose-hold` puts a posed craft back on its pose at the last tick, and nothing else does.
//!
//! A craft resting after GO on a banked slot turns, in the original as in ours (bank-to-yaw, see
//! `docs/physics/grid-state.md`, "A resting craft after GO turns, in the original too"). A
//! comparison pair poses ours at the original's pose at race clock `T` and runs the countdown and
//! `T` seconds, so without the hold ours turns a second time. Amphiseum Black's start slot is
//! banked: 900 ticks turn it about 1.3 degrees.

use std::path::{Path, PathBuf};
use std::process::Command;

const TICKS: &str = "900";

/// The heading (degrees) and position of the first and last rows of a posed, inputless run.
fn run(image: &Path, scratch: &Path, hold: bool) -> ((f64, [f64; 3]), (f64, [f64; 3])) {
    let out = scratch.join(format!("hold-{hold}.csv"));
    let mut command = Command::new(env!("CARGO_BIN_EXE_oag-game"));
    command
        .arg(image)
        .args([
            "--race",
            "--no-audio",
            "--mode",
            "time_trial",
            "--class",
            "venom",
        ])
        .args(["--track", r"Data\Environments\09_Track\track.vex"])
        .arg("--pose=-527.0,3.5039,107.0888,-1.7")
        .args(["--ticks", TICKS])
        .arg("--trace-out")
        .arg(&out)
        .env("XDG_CONFIG_HOME", scratch.join("config"))
        .env("XDG_DATA_HOME", scratch.join("data"))
        .env("XDG_STATE_HOME", scratch.join("state"));
    if hold {
        command.arg("--pose-hold");
    }
    assert!(command.status().expect("running oag-game").success());
    let text = std::fs::read_to_string(&out).expect("reading the trace");
    let mut lines = text.lines();
    let header: Vec<&str> = lines.next().expect("a header").split(',').collect();
    let column = |name: &str| header.iter().position(|h| *h == name).expect(name);
    let rows: Vec<Vec<f64>> = lines
        .map(|line| line.split(',').map(|v| v.parse().unwrap_or(0.0)).collect())
        .collect();
    let pose = |row: &Vec<f64>| {
        let heading = row[column("fwd_x")]
            .atan2(-row[column("fwd_z")])
            .to_degrees();
        let position = ["pos_x", "pos_y", "pos_z"].map(|c| row[column(c)]);
        (heading, position)
    };
    let last = rows.last().expect("rows");
    assert_eq!(last[column("tick")].to_string(), TICKS);
    (pose(&rows[0]), pose(last))
}

fn scratch() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("oag-pose-hold-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("creating the scratch directory");
    dir
}

#[test]
#[ignore = "needs a Pulse disc image in data/images/"]
fn a_held_pose_is_the_start_pose_after_the_clock_has_run() {
    let Some(image) = oag_testdata::image("data/images/pulse-psp-usa.chd") else {
        return;
    };
    let scratch = scratch();
    let (start, end) = run(&image, &scratch, false);
    let (held_start, held_end) = run(&image, &scratch, true);
    std::fs::remove_dir_all(&scratch).ok();

    // Unheld, the resting craft turns: the double count the hold exists for.
    assert!(
        (end.0 - start.0).abs() > 0.5,
        "an unheld resting craft on a banked slot turns: {start:?} -> {end:?}"
    );
    // Held, the last row is the start pose, exactly (the same f32s written back).
    assert_eq!(held_end, held_start, "the held pose at tick {TICKS}");
    assert_eq!(held_start, start, "the hold does not move the start");
}
