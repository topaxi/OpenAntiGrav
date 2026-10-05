//! The pose a request names, resolved against the circuit's spline.

use super::*;

/// Resolved at load rather than in `Race::start`: the spline supplies the attitude, and `load` is
/// where it is.
pub(super) fn resolve(
    request: Option<PoseRequest>,
    spline: &Spline,
    report: &mut Vec<String>,
) -> Option<Pose> {
    request.and_then(|request| match request {
        PoseRequest::SplineAligned { position, yaw } => {
            let (_, sample, distance) = spline.nearest(position)?;
            report.push(format!(
                "pose override: {position:?} yaw {:.1} deg, attitude from the spline sample \
                 {distance:.1} units away",
                yaw.to_degrees()
            ));
            Some(Pose::from_position_on_sample(sample, position, yaw))
        }
        // Verbatim: the whole point of an exact pose is that nothing here
        // second-guesses the recorded basis against the spline.
        PoseRequest::Exact(pose) => {
            report.push(format!("pose override: exact, at {:?}", pose.position));
            Some(pose)
        }
    })
}
