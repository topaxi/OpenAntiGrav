//! `--pose-lean`: the player's drawn lean and barrel-roll phase, imported from the trace row a pose was read off.
//!
//! A posed capture (`--pose-from --ticks 0`) runs no tick, so ship 0 is drawn flat whatever the original's stick was
//! doing at the captured instant: the lean is `entity+0x854` (`steer_lean`, the model turns by `steer_lean * 0.5`
//! radians about its nose) and the roll is `roll_phase`, both written by the original's per-tick update, neither by
//! the pose. A trace row carries both columns (`psp-trace.py`, ours `--trace-out`), so the pose's own row is the one
//! source: what this applies was recorded, not chosen. The rate-limited follower behind `steer_lean`
//! (`entity+0x858`, read by the HUD's sway) has no column in a trace and is left as the race has it.

use anyhow::{Context, Result};
use oag_raceplay::Race;

/// The two display-state values of one captured row.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PoseLean {
    /// `entity+0x854`, radians-ish as the original stores it (`steer_lean * 0.5` turns the hull).
    pub steer_lean: f32,
    /// The barrel roll's phase, `-1..=1`.
    pub roll_phase: f32,
}

impl PoseLean {
    /// The row of `trace` at `tick` (the row `--pose-from` poses from), refusing a trace that was captured without the
    /// columns: a missing lean must not read as a captured zero.
    pub fn from_trace(path: &std::path::Path, tick: u64) -> Result<Self> {
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("reading {} for --pose-lean", path.display()))?;
        let trace = oag_trace::Trace::parse(&text)
            .with_context(|| format!("{} is not a trace", path.display()))?;
        let frame = trace
            .frames
            .iter()
            .find(|frame| frame.tick == tick)
            .with_context(|| format!("{} has no tick {tick}", path.display()))?;
        Ok(Self {
            steer_lean: frame
                .steer_lean
                .with_context(|| format!("{} has no steer_lean column", path.display()))?,
            roll_phase: frame
                .roll_phase
                .with_context(|| format!("{} has no roll_phase column", path.display()))?,
        })
    }

    /// Puts the values on ship 0, where the drawn matrix and the cockpit camera's roll read them.
    pub fn apply(self, race: &mut Race) {
        let ship = &mut race.sim.world.ships[0].physics;
        ship.steer_lean = self.steer_lean;
        ship.roll_phase = self.roll_phase;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HEAD: &str = "tick,dt,grounded,throttle,brake,steer,airbrake_l,airbrake_r,speed_cached,\
right_x,right_y,right_z,up_x,up_y,up_z,fwd_x,fwd_y,fwd_z,pos_x,pos_y,pos_z,vel_x,vel_y,vel_z,speed";
    const ROW: &str = "5,0.016683,1,100,0,0,0,0,22,1,0,0,0,1,0,0,0,1,10,2.5,-30,0,0,22,22";

    fn trace(name: &str, text: &str) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!("oag-pose-lean-{name}.csv"));
        std::fs::write(&path, text).unwrap();
        path
    }

    #[test]
    fn a_row_without_the_lean_columns_is_refused() {
        let path = trace("none", &format!("{HEAD}\n{ROW}\n"));
        let error = PoseLean::from_trace(&path, 5).expect_err("no steer_lean column");
        assert!(format!("{error:#}").contains("steer_lean"), "{error:#}");
    }

    #[test]
    fn the_columns_of_the_pose_row_are_what_is_imported() {
        let path = trace(
            "both",
            &format!("{HEAD},steer_lean,roll_phase\n{ROW},0.32,-0.25\n"),
        );
        let lean = PoseLean::from_trace(&path, 5).expect("a row with both columns");
        assert_eq!(
            lean,
            PoseLean {
                steer_lean: 0.32,
                roll_phase: -0.25
            }
        );
    }
}
