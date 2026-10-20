//! `--pose`, `--pose-from` and `--camera-pose`: where a captured frame puts
//! the craft, and where it puts the camera.

use anyhow::{Context, Result, ensure};
use log::warn;

use oag_raceplay as race;

/// Parses `--pose`: three or four comma-separated numbers, the fourth a yaw in
/// degrees.
pub(crate) fn parse_pose(text: &str) -> Result<(oag_core::math::Vec3, f32)> {
    let parts: Vec<&str> = text.split(',').map(str::trim).collect();
    ensure!(
        matches!(parts.len(), 3 | 4),
        "--pose takes x,y,z or x,y,z,yaw, not {} value(s)",
        parts.len()
    );
    let mut values = [0.0f32; 4];
    for (slot, text) in values.iter_mut().zip(&parts) {
        *slot = text
            .parse()
            .with_context(|| format!("--pose component {text:?} is not a number"))?;
    }
    Ok((
        oag_core::math::Vec3::new(values[0], values[1], values[2]),
        values[3].to_radians(),
    ))
}

/// Resolves `--pose-from`: one row of a capture into an exact ship pose, plus
/// the recorded camera when the row carries one.
///
/// The ship's basis goes through [`oag_trace::replay::orientation_of`] under
/// the measured reading - the capture's `right_*` columns are the ship's left,
/// and that reconciliation must happen in the one crate that owns it rather
/// than be restated here. The camera goes through
/// [`oag_trace::replay::camera_orientation_of`], which has its own, weaker
/// contract; see its docs.
pub(crate) fn pose_from_trace(
    path: &std::path::Path,
    tick: u64,
    no_camera: bool,
    camera_fov: Option<f32>,
) -> Result<(race::PoseRequest, Option<race::CameraOverride>)> {
    use oag_trace::replay::{self, Basis};

    let text = std::fs::read_to_string(path)
        .with_context(|| format!("reading {} for --pose-from", path.display()))?;
    let trace = oag_trace::Trace::parse(&text)
        .with_context(|| format!("{} is not a trace", path.display()))?;
    let frame = trace
        .frames
        .iter()
        .find(|frame| frame.tick == tick)
        .with_context(|| {
            format!(
                "{} has no tick {tick}; it covers {:?}..={:?}",
                path.display(),
                trace.frames.first().map(|f| f.tick),
                trace.frames.last().map(|f| f.tick)
            )
        })?;

    let pose = race::PoseRequest::Exact(oag_gameplay::spawn::Pose {
        position: frame.position,
        orientation: replay::orientation_of(frame, Basis::LeftUpForward),
    });
    let camera = match (no_camera, replay::camera_orientation_of(frame)) {
        (true, _) | (false, None) => {
            if !no_camera {
                warn!(
                    "{}: no camera columns (captured without --camera); using the chase camera",
                    path.display()
                );
            }
            None
        }
        (false, Some(orientation)) => Some(race::CameraOverride {
            // `camera_orientation_of` answered, so the pose group is present
            // and the eye is too: the columns are all-or-nothing at parse.
            eye: frame
                .camera_position
                .context("a parsed camera pose has an eye")?,
            orientation,
            fov_deg: camera_fov,
        }),
    };
    Ok((pose, camera))
}

/// One opponent of a captured field: where the original had it and which team it flew.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct FieldCraft {
    pub(crate) pose: oag_gameplay::spawn::Pose,
    pub(crate) team: String,
}

/// Resolves `--field-from`: a CSV of the opponents the original had on one tick, one row per
/// grid slot in slot order (`slot,team,left_x..left_z,up_x..up_z,fwd_x..fwd_z,pos_x..pos_z`),
/// the rows `scripts/psp-race-field.py` writes out of the emulator's RAM.
///
/// The basis is the ship's own recorded one, read exactly as `--pose-from` reads it
/// ([`oag_trace::replay::orientation_of_rows`], the measured left/up/forward reading), so a
/// field craft and the player go through one reconciliation. The slot column is the grid slot
/// 1-based; the rows must be exactly `1..=N` in order, because that is the index
/// `race::Options::field` and `grid_teams` share.
pub(crate) fn field_from_csv(path: &std::path::Path) -> Result<Vec<FieldCraft>> {
    use oag_core::math::Vec3;
    use oag_trace::replay::{self, Basis};

    let text = std::fs::read_to_string(path)
        .with_context(|| format!("reading {} for --field-from", path.display()))?;
    let mut lines = text.lines().filter(|line| !line.trim().is_empty());
    let header: Vec<&str> = lines
        .next()
        .with_context(|| format!("{} is empty", path.display()))?
        .split(',')
        .map(str::trim)
        .collect();
    let column = |name: &str| {
        header
            .iter()
            .position(|h| *h == name)
            .with_context(|| format!("{} has no `{name}` column", path.display()))
    };
    let vec3 = |cells: &[&str], prefix: &str| -> Result<Vec3> {
        let mut v = [0.0f32; 3];
        for (slot, axis) in v.iter_mut().zip(["x", "y", "z"]) {
            let at = column(&format!("{prefix}_{axis}"))?;
            let text = cells.get(at).copied().unwrap_or("");
            *slot = text
                .parse()
                .with_context(|| format!("{prefix}_{axis} {text:?} is not a number"))?;
        }
        Ok(Vec3::from_array(v))
    };
    let (slot_at, team_at) = (column("slot")?, column("team")?);
    let mut field = Vec::new();
    for (row, line) in lines.enumerate() {
        let cells: Vec<&str> = line.split(',').map(str::trim).collect();
        ensure!(
            cells.len() == header.len(),
            "{} row {}: {} cells for {} columns",
            path.display(),
            row + 1,
            cells.len(),
            header.len()
        );
        let slot: usize = cells[slot_at]
            .parse()
            .with_context(|| format!("row {}: slot {:?} is not a number", row + 1, cells[slot_at]))?;
        ensure!(
            slot == row + 1,
            "{} row {}: slot {slot}, but rows must run 1, 2, 3 ... in order",
            path.display(),
            row + 1
        );
        field.push(FieldCraft {
            pose: oag_gameplay::spawn::Pose {
                position: vec3(&cells, "pos")?,
                orientation: replay::orientation_of_rows(
                    vec3(&cells, "left")?,
                    vec3(&cells, "up")?,
                    vec3(&cells, "fwd")?,
                    Basis::LeftUpForward,
                ),
            },
            team: cells[team_at].to_string(),
        });
    }
    ensure!(!field.is_empty(), "{} has no craft rows", path.display());
    Ok(field)
}

/// Parses `--hud-state`: `PLACE,LAP,LAP_SECONDS,RACE_SECONDS[,SPEED_KMH[,SHIELD]]`, the clocks
/// rounded to the tick.
pub(crate) fn parse_hud_state(text: &str) -> Result<race::CaptureHud> {
    let parts: Vec<&str> = text.split(',').map(str::trim).collect();
    ensure!(
        (4..=6).contains(&parts.len()),
        "--hud-state takes PLACE,LAP,LAP_SECONDS,RACE_SECONDS[,SPEED_KMH[,SHIELD]], not {} value(s)",
        parts.len()
    );
    let whole = |name: &str, text: &str| -> Result<u32> {
        text.parse()
            .with_context(|| format!("--hud-state {name} {text:?} is not a whole number"))
    };
    let seconds = |name: &str, text: &str| -> Result<u64> {
        let value: f64 = text
            .parse()
            .with_context(|| format!("--hud-state {name} {text:?} is not a number"))?;
        ensure!(
            value.is_finite() && value >= 0.0,
            "--hud-state {name} {text:?} is not a time"
        );
        // Non-negative and finite, and a race clock is far below `u64::MAX` ticks.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        Ok((value * 60.0).round() as u64)
    };
    let hud = race::CaptureHud {
        place: whole("PLACE", parts[0])?,
        lap: whole("LAP", parts[1])?,
        lap_ticks: seconds("LAP_SECONDS", parts[2])?,
        race_ticks: seconds("RACE_SECONDS", parts[3])?,
        speed_kmh: parts
            .get(4)
            .map(|text| {
                text.parse::<f32>()
                    .with_context(|| format!("--hud-state SPEED_KMH {text:?} is not a number"))
            })
            .transpose()?,
        shield: parts
            .get(5)
            .map(|text| {
                text.parse::<f32>()
                    .with_context(|| format!("--hud-state SHIELD {text:?} is not a number"))
            })
            .transpose()?,
    };
    ensure!(
        hud.place >= 1 && hud.lap >= 1,
        "--hud-state place and lap are 1-based"
    );
    Ok(hud)
}

/// Parses `--camera-pose`: an eye, a look direction and an up vector.
///
/// The nine numbers `scripts/ps3_pose.py` decomposes a captured `viewProj`
/// into. The basis is orthonormalised here rather than trusted: it arrives
/// through a `f32` matrix that was itself a product of two, so `up` is
/// reliably *almost* perpendicular to `fwd` and a [`Quat`] built from an
/// almost-orthonormal basis is not a rotation.
pub(crate) fn parse_camera_pose(text: &str) -> Result<race::CameraOverride> {
    use oag_core::math::{Mat3, Quat, Vec3};

    let parts: Vec<&str> = text.split(',').map(str::trim).collect();
    ensure!(
        parts.len() == 9,
        "--camera-pose takes nine numbers (eye, forward, up), not {}",
        parts.len()
    );
    let mut v = [0.0f32; 9];
    for (slot, text) in v.iter_mut().zip(&parts) {
        *slot = text
            .parse()
            .with_context(|| format!("--camera-pose component {text:?} is not a number"))?;
    }
    let eye = Vec3::new(v[0], v[1], v[2]);
    let forward = Vec3::new(v[3], v[4], v[5]);
    let up = Vec3::new(v[6], v[7], v[8]);
    ensure!(
        forward.length() > 1e-6 && up.length() > 1e-6,
        "--camera-pose needs a non-zero forward and up"
    );
    let forward = forward.normalize();
    let right = forward.cross(up);
    ensure!(
        right.length() > 1e-6,
        "--camera-pose's forward and up are parallel, so there is no camera"
    );
    let right = right.normalize();
    let up = right.cross(forward);
    // `-Z` is the look direction in this project's camera convention, the same
    // one `oag_trace::replay::camera_orientation_of` produces - see
    // `race::CameraOverride::orientation`.
    let basis = Mat3::from_cols(right, up, -forward);
    Ok(race::CameraOverride {
        eye,
        orientation: Quat::from_mat3(&basis),
        fov_deg: None,
    })
}
