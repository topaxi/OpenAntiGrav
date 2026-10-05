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
