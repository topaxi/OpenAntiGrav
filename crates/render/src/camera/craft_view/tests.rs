//! The two views against the matrices the original read and wrote on the same frame
//! (`scripts/psp-spectator-capture.py`, PPSSPP, 2026-10-02): the craft's matrix as the original
//! holds it (rows left, up, forward, position) and the view matrix it wrote (four columns of
//! the view, so its translation row is the negated eye).

use super::*;
use oag_core::math::Mat3;

/// The original's craft rows turned into this project's orientation: right is the negated
/// first row, up the second, back the negated third.
fn orientation_of(rows: &[f32; 16]) -> Quat {
    let left = Vec3::new(rows[0], rows[1], rows[2]);
    let up = Vec3::new(rows[4], rows[5], rows[6]);
    let forward = Vec3::new(rows[8], rows[9], rows[10]);
    Quat::from_mat3(&Mat3::from_cols(-left, up, -forward))
}

fn position_of(rows: &[f32; 16]) -> Vec3 {
    Vec3::new(rows[12], rows[13], rows[14])
}

/// The eye the original wrote: the view's translation row is the negated eye.
fn eye_written(out: &[f32; 16]) -> Vec3 {
    -Vec3::new(out[12], out[13], out[14])
}

/// The camera's right, up and back as the original wrote them: the view's rotation rows,
/// stored as columns.
fn axes_written(out: &[f32; 16]) -> [Vec3; 3] {
    [
        Vec3::new(out[0], out[4], out[8]),
        Vec3::new(out[1], out[5], out[9]),
        Vec3::new(out[2], out[6], out[10]),
    ]
}

fn close(a: Vec3, b: Vec3, tolerance: f32) {
    assert!((a - b).abs().max_element() < tolerance, "{a:?} against {b:?}");
}

/// Frame 129 of the capture, mode 2, the most banked craft of that mode (`up.y` 0.958).
const REAR_CRAFT: [f32; 16] = [
    0.346_915_84, 0.272_004_8, -0.897_587_24, 0.0, -0.180_598_38, 0.958_485_4, 0.220_658_42, 0.0,
    0.920_344_35, 0.085_552_9, 0.381_637_3, 0.0, 328.838_1, -42.159_06, -153.364_87, 1.0,
];
const REAR_WRITTEN: [f32; 16] = [
    -0.346_915_84, -0.180_598_38, -0.920_344_35, 0.0, -0.272_004_8, 0.958_485_4, -0.085_552_9, 0.0,
    0.897_587_24, 0.220_658_42, -0.381_637_3, 0.0, -323.010_83, 40.207_458, 155.281_77, 1.0,
];

/// Frame 397, mode 3, the most banked of that mode (`up.y` 0.864).
const FRONT_CRAFT: [f32; 16] = [
    0.507_442_36, 0.497_203_35, 0.703_769_1, 0.0, -0.358_998_3, 0.864_464_6, -0.351_882_1, 0.0,
    -0.783_340_45, -0.074_092_02, 0.617_161_06, 0.0, 542.907_8, -5.167_188_6, 210.173_73, 1.0,
];
const FRONT_WRITTEN: [f32; 16] = [
    0.507_442_36, -0.358_998_3, -0.783_340_45, 0.0, 0.497_203_35, 0.864_464_6, -0.074_092_02, 0.0,
    0.703_769_1, -0.351_882_1, 0.617_161_06, 0.0, -532.430_73, 3.462_899, -216.524_03, 1.0,
];

fn check(pose: Pose, written: &[f32; 16]) {
    close(pose.eye, eye_written(written), 2e-3);
    let [right, up, back] = axes_written(written);
    close(pose.orientation * Vec3::X, right, 1e-4);
    close(pose.orientation * Vec3::Y, up, 1e-4);
    close(pose.orientation * Vec3::Z, back, 1e-4);
}

#[test]
fn the_rear_view_reproduces_the_matrix_the_original_wrote_for_a_banked_craft() {
    let pose = rear(position_of(&REAR_CRAFT), orientation_of(&REAR_CRAFT));
    check(pose, &REAR_WRITTEN);
}

#[test]
fn the_front_view_reproduces_the_matrix_the_original_wrote_for_a_banked_craft() {
    let pose = front(position_of(&FRONT_CRAFT), orientation_of(&FRONT_CRAFT));
    check(pose, &FRONT_WRITTEN);
}

#[test]
fn a_level_craft_is_watched_from_behind_and_from_ahead() {
    let at = Vec3::new(10.0, -20.0, 30.0);
    let level = Quat::IDENTITY;
    let rear = rear(at, level);
    assert!(
        (rear.eye - (at + Vec3::new(0.0, 2.5, 6.0))).length() < 1e-5,
        "{:?}",
        rear.eye
    );
    assert!((rear.orientation * Vec3::NEG_Z - Vec3::NEG_Z).length() < 1e-5);
    let front = front(at, level);
    assert!((front.eye - (at + Vec3::new(0.0, 3.0, -12.0))).length() < 1e-5);
    assert!(
        (front.orientation * Vec3::NEG_Z - Vec3::Z).length() < 1e-5,
        "the front view looks back at the craft"
    );
}
