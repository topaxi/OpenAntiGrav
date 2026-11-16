//! What a craft's shine pass uploads: its coordinates follow the pose, so the views of a split
//! that draw one craft share one upload.

use std::cell::RefCell;

use oag_gpu::writes::Log;

use super::scene_build::test_gpu;
use super::*;

#[test]
fn a_pose_the_buffer_holds_is_not_written_again() {
    let Some((device, queue)) = test_gpu() else {
        return;
    };
    let shine = super::bindless::drawable(&device, &queue, 1, false);
    let log = RefCell::new(Log::default());
    let mut scratch = Vec::new();
    let turned = Mat4::from_rotation_y(0.5);

    shine.write_environment_map(&log, Mat4::IDENTITY, [0.0, 0.0], [1.0; 3], &mut scratch);
    assert_eq!(log.borrow().len(), 1, "the first view to draw the craft");
    shine.write_environment_map(&log, Mat4::IDENTITY, [0.0, 0.0], [1.0; 3], &mut scratch);
    assert_eq!(log.borrow().len(), 1, "a later view at the same pose");
    shine.write_environment_map(&log, Mat4::IDENTITY, [0.3, 0.0], [1.0; 3], &mut scratch);
    assert_eq!(log.borrow().len(), 2, "the flaps moved");
    shine.write_environment_map(&log, turned, [0.3, 0.0], [1.0; 3], &mut scratch);
    assert_eq!(log.borrow().len(), 3, "the craft moved");
    shine.write_environment_map(&log, turned, [0.3, 0.0], [0.7, 0.6, 0.4], &mut scratch);
    assert_eq!(
        log.borrow().len(),
        4,
        "the light the craft stands in changed"
    );
    assert!(
        scratch.iter().all(|v| v.colour[..3] == [0.7, 0.6, 0.4]),
        "the pass's white vertices take the colour"
    );
}
