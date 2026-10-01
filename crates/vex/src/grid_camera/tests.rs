use super::*;

fn flyby(cut_frames: Vec<u32>) -> GridCamera {
    GridCamera {
        data: Vec::new(),
        nodes: Vec::new(),
        camera: 0,
        length: 25.0,
        cut_frames,
    }
}

#[test]
fn a_cut_is_reported_on_the_step_that_crosses_its_second_key() {
    let grid = flyby(vec![361, 721]);
    // 359/60 -> 360/60: the second key (361) is not crossed.
    assert!(!grid.cuts_between(359.0 / 60.0, 360.0 / 60.0));
    // 360/60 -> 361/60 crosses key 361.
    assert!(grid.cuts_between(360.0 / 60.0, 361.0 / 60.0));
    // 361/60 -> 362/60 does not cross it again.
    assert!(!grid.cuts_between(361.0 / 60.0, 362.0 / 60.0));
}

#[test]
fn a_step_that_spans_two_cuts_reports_one_jump() {
    let grid = flyby(vec![361, 721]);
    assert!(grid.cuts_between(0.0, 25.0));
}

#[test]
fn a_flyby_without_cuts_never_jumps() {
    assert!(!flyby(Vec::new()).cuts_between(0.0, 25.0));
}

#[test]
fn a_file_that_is_not_a_vex_is_refused() {
    assert!(GridCamera::read(&[0u8; 64]).is_none());
}
