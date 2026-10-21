//! `--pose-lean` puts the pose row's own drawn lean and roll phase on the posed craft, and nothing else does.
//!
//! A posed capture (`--pose-from --ticks 0`) runs no tick, so the hull was drawn flat whatever the original's stick
//! was doing at the captured instant (talons-boost: `steer_lean` 0.32, a 16 degree bank, in the original's row, none
//! in ours). The row is read back off ours' own `--trace-out` so the check is the value the drawn matrix reads.

use std::path::{Path, PathBuf};
use std::process::Command;

fn oag_game(image: &Path, scratch: &Path) -> Command {
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
        .env("XDG_CONFIG_HOME", scratch.join("config"))
        .env("XDG_DATA_HOME", scratch.join("data"))
        .env("XDG_STATE_HOME", scratch.join("state"));
    command
}

/// The value of column `name` in the last row of a trace.
fn last(path: &Path, name: &str) -> f32 {
    let text = std::fs::read_to_string(path).expect("reading the trace");
    let mut lines = text.lines();
    let header: Vec<&str> = lines.next().expect("a header").split(',').collect();
    let at = header.iter().position(|h| *h == name).expect(name);
    lines
        .last()
        .expect("a row")
        .split(',')
        .nth(at)
        .expect("the column")
        .parse()
        .expect("a number")
}

fn scratch() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("oag-pose-lean-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("creating the scratch directory");
    dir
}

#[test]
#[ignore = "needs a Pulse disc image in data/images/"]
fn the_pose_rows_lean_and_roll_phase_reach_the_drawn_craft() {
    let Some(image) = oag_testdata::image("data/images/pulse-psp-usa.chd") else {
        return;
    };
    let scratch = scratch();
    // A pose row with a lean the original's stick would have made (`talons-boost` read 0.32), at Amphiseum Black's
    // start slot, in the capture's own basis (identity: right, up, forward).
    let source = scratch.join("source.csv");
    std::fs::write(
        &source,
        "tick,dt,grounded,throttle,brake,steer,airbrake_l,airbrake_r,speed_cached,\
right_x,right_y,right_z,up_x,up_y,up_z,fwd_x,fwd_y,fwd_z,pos_x,pos_y,pos_z,vel_x,vel_y,vel_z,speed,\
steer_lean,roll_phase\n\
0,0.016683,1,0,0,0,0,0,0,1,0,0,0,1,0,0,0,1,-527.0,3.5039,107.0888,0,0,0,0,0.32,0\n",
    )
    .expect("writing the pose row");

    let posed = |lean: bool| {
        let out = scratch.join(format!("posed-{lean}.csv"));
        let mut command = oag_game(&image, &scratch);
        command
            .arg("--pose-from")
            .arg(&source)
            .args(["--ticks", "0", "--trace-out"])
            .arg(&out);
        if lean {
            command.arg("--pose-lean");
        }
        assert!(command.status().expect("running oag-game").success());
        out
    };
    let flat = posed(false);
    let leaning = posed(true);
    let (flat_lean, drawn_lean) = (last(&flat, "steer_lean"), last(&leaning, "steer_lean"));
    std::fs::remove_dir_all(&scratch).ok();

    assert_eq!(flat_lean, 0.0, "a posed craft is flat without the flag");
    assert_eq!(drawn_lean, 0.32, "the row's lean is the drawn lean");
}
