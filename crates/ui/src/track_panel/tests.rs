use super::*;

#[test]
fn the_panel_fades_in_over_seven_tenths_and_the_wipe_takes_two_seconds() {
    let at = |entered| Progress {
        entered,
        left: None,
    };
    assert_eq!(at(0.0).alpha(), 0.0);
    assert!((at(0.35).alpha() - 0.5).abs() < 1e-6);
    assert_eq!(at(0.7).alpha(), 1.0);
    assert!((at(1.0).wipe() - 0.5).abs() < 1e-6);
    assert_eq!(at(3.0).wipe(), 1.0);
}

#[test]
fn leaving_runs_seven_tenths_from_full_width_whatever_the_wipe_had_reached() {
    let leaving = |left| Progress {
        entered: 1.0,
        left: Some(left),
    };
    assert_eq!(leaving(0.0).wipe(), 1.0);
    assert!((leaving(0.35).wipe() - 0.5).abs() < 1e-6);
    assert!((leaving(0.35).alpha() - 0.5).abs() < 1e-6);
    assert!(!leaving(0.7).visible());
}

#[test]
fn a_panel_skipped_before_it_had_faded_in_leaves_from_where_it_was() {
    let early = Progress {
        entered: 0.35,
        left: Some(0.35),
    };
    assert_eq!(early.alpha(), 0.0);
}
