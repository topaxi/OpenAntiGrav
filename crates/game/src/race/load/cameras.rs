//! The three in-race camera rigs, read from the team's own `stats.xml`.
//!
//! Split out of [`super`] under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change. It is a fair
//! seam: everything here reads one document and produces the three rigs plus
//! the lines describing them, and nothing else in the loader touches it.

use super::*;

/// The far chase rig, the close one, and the cockpit, with their report lines.
pub(super) fn resolve(
    stats: &oag_formats::handling::Stats,
    stats_name: &str,
    options: &Options,
    handling: &oag_physics::params::Handling,
    team: &str,
    report: &mut Vec<String>,
) -> (ChaseParams, ChaseParams, InternalParams) {
    let far = stats.external_camera_far;
    let chase = chase_params(far);
    // The other two views the player can cycle to, read from the same document
    // and through the same conversion - so the sign convention is reconciled in
    // exactly one place for both external blocks, which is what
    // `chase_pos_length` exists for.
    let chase_close = chase_params(stats.external_camera_close);
    let internal = InternalParams {
        fov: stats.internal_camera.fov,
        headtilt: stats.internal_camera.headtilt,
        height: stats.internal_camera.height,
        length: stats.internal_camera.length,
        pitch: stats.internal_camera.pitch,
    };
    report.push(format!(
        "{stats_name}: team {:?}, {:?} class, mass {}, ride_height {}",
        stats.team, options.class, handling.physical.mass, handling.antigrav.ride_height
    ));
    report.extend(assets::zone_handling_note(options.mode, team));
    // The eye offsets are reported **after** `craft_scale`, because that is
    // where the eye actually ends up, and the two differ now that the scale is a
    // rig parameter rather than a factor folded into the four offsets. Reporting
    // the authored numbers under the word "eye" would be a report that lies -
    // "eye 4 up" for a rig whose eye is 3 up.
    let eye_offsets = |chase: &ChaseParams| {
        (
            chase.pos_height * chase.craft_scale,
            chase.pos_length * chase.craft_scale,
        )
    };
    let (far_up, far_back) = eye_offsets(&chase);
    let (close_up, close_back) = eye_offsets(&chase_close);
    report.push(format!(
        "<ExternalCameraFar>: fov {} as vertical degrees (confirmed against the \
         original's own frame), pos_length {} on disc -> eye {far_up} up and \
         {far_back} back (authored offsets times the craft's {} scale)",
        chase.fov, far.pos_length, chase.craft_scale
    ));
    report.push(format!(
        "<ExternalCameraClose>: fov {}, eye {close_up} up and {close_back} back; \
         <InternalCamera>: fov {}, eye {} up and {} forward, pitch {} over {} units",
        chase_close.fov,
        internal.fov,
        internal.height,
        internal.length,
        internal.pitch,
        oag_render::camera::internal::AIM_DISTANCE,
    ));

    (chase, chase_close, internal)
}
