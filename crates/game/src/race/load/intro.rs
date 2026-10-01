//! The circuit's pre-race flyby: `start_grid.vex`, beside `track.vex`.
//!
//! One file per circuit directory - `track_reversed.vex` and `zone_track.vex` share it, since
//! the directory holds no other spelling. Pulse on a PSP disc only: the camera was read and
//! measured there and nowhere else. See [`crate::race::intro_camera`].

use oag_vex::grid_camera::GridCamera;

/// The flyby for the circuit `track` (a `.vex` entry name) lives in, or `None` off Pulse's PSP
/// disc or when the circuit authors none. Says which in `report`.
pub(super) fn read(
    archives: &mut oag_assets::source::Archives,
    track: &str,
    pulse_psp: bool,
    report: &mut Vec<String>,
) -> Option<GridCamera> {
    if !pulse_psp {
        return None;
    }
    let (directory, _) = track.rsplit_once('\\')?;
    let entry = format!(r"{directory}\start_grid.vex");
    let blob = match archives.read_name(&entry) {
        Ok(blob) => blob,
        Err(e) => {
            report.push(format!("pre-race flyby: {entry} did not read ({e})"));
            return None;
        }
    };
    let grid = GridCamera::read(&blob);
    report.push(match &grid {
        Some(grid) => format!(
            "pre-race flyby: {entry}, {:.2} s, {} cut(s) at key frames {:?}",
            grid.length(),
            grid.cut_frames().len(),
            grid.cut_frames()
        ),
        None => format!("pre-race flyby: {entry} has no playable gridCamera"),
    });
    grid
}
