//! `--pose-flaps`: the player's two airbrake levels, imported from the trace row a pose was read off.
//!
//! A posed capture (`--pose-from --ticks 0`) runs no tick that swings the flaps, so they draw stowed whatever
//! the original's air brakes were doing at the captured instant. A trace row carries both ramped levels
//! (`airbrake_l`, `airbrake_r`, `0..=100`, `craft+0x2c4` and `+0x2c8`), the same scale the drawn flap chases
//! in ours (`oag_raceplay::Race::force_flaps`), so the row is the one source: what this applies was read from the
//! original's RAM, not chosen.

use anyhow::{Context, Result};
use oag_raceplay::Race;

/// The two air brake levels of one captured row, `0..=100`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PoseFlaps {
    /// `airbrake_l`.
    pub left: f32,
    /// `airbrake_r`.
    pub right: f32,
}

impl PoseFlaps {
    /// The row of `trace` at `tick` (the row `--pose-from` poses from).
    pub fn from_trace(path: &std::path::Path, tick: u64) -> Result<Self> {
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("reading {} for --pose-flaps", path.display()))?;
        let trace = oag_trace::Trace::parse(&text)
            .with_context(|| format!("{} is not a trace", path.display()))?;
        let frame = trace
            .frames
            .iter()
            .find(|frame| frame.tick == tick)
            .with_context(|| format!("{} has no tick {tick}", path.display()))?;
        Ok(Self {
            left: frame.airbrake_left,
            right: frame.airbrake_right,
        })
    }

    /// Puts the levels on ship 0's drawn flaps.
    pub fn apply(self, race: &mut Race) {
        race.force_flaps(0, [self.left, self.right]);
    }
}

/// Puts `flaps`, when `--pose-flaps` read some, on ship 0.
pub fn apply(flaps: Option<PoseFlaps>, race: &mut Race) {
    if let Some(flaps) = flaps {
        flaps.apply(race);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HEAD: &str = "tick,dt,grounded,throttle,brake,steer,airbrake_l,airbrake_r,speed_cached,\
right_x,right_y,right_z,up_x,up_y,up_z,fwd_x,fwd_y,fwd_z,pos_x,pos_y,pos_z,vel_x,vel_y,vel_z,speed";

    fn trace(
        name: &str,
        left: f32,
        right: f32,
    ) -> (oag_testdata::scratch::TempDir, std::path::PathBuf) {
        let row = format!(
            "7,0.016683,1,100,0,0,{left},{right},22,1,0,0,0,1,0,0,0,1,10,2.5,-30,0,0,22,22"
        );
        let dir = oag_testdata::scratch::dir(&format!("pose-flaps-{name}"));
        let path = dir.join("trace.csv");
        std::fs::write(
            &path,
            format!(
                "{HEAD}
{row}
"
            ),
        )
        .unwrap();
        (dir, path)
    }

    #[test]
    fn the_two_levels_of_the_pose_row_are_what_is_imported() {
        let (_scratch, path) = trace("levels", 100.0, 83.5);
        let flaps = PoseFlaps::from_trace(&path, 7).expect("a row");
        assert_eq!(
            flaps,
            PoseFlaps {
                left: 100.0,
                right: 83.5
            }
        );
    }

    #[test]
    fn a_tick_the_trace_lacks_is_refused() {
        let (_scratch, path) = trace("tick", 0.0, 0.0);
        let error = PoseFlaps::from_trace(&path, 9).expect_err("no tick 9");
        assert!(format!("{error:#}").contains("no tick 9"), "{error:#}");
    }
}
