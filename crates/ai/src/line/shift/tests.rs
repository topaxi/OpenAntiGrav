use super::*;

fn straight(n: usize) -> (Vec<Vec3>, Vec<Frame>) {
    let points = (0..n).map(|i| Vec3::new(0.0, 0.0, -(i as f32))).collect();
    let corridor = vec![
        Frame {
            lateral: Vec3::X,
            left: -10.0,
            right: 10.0,
        };
        n
    ];
    (points, corridor)
}

#[test]
fn a_target_moves_the_line_fully_inside_hold_and_not_past_hold_plus_ease() {
    let (mut points, mut corridor) = straight(200);
    let targets = [Target {
        index: 100,
        offset: 6.0,
    }];
    toward(&mut points, &mut corridor, &targets, 5.0, 20.0, 0.0);
    for point in &points[95..=105] {
        assert!((point.x - 6.0).abs() < 1e-5, "{}", point.x);
    }
    assert_eq!(points[70].x, 0.0);
    assert_eq!(points[130].x, 0.0);
    assert!(points[115].x > 0.0 && points[115].x < 6.0);
    // Monotone ease: nothing overshoots on the way in.
    for pair in points[75..96].windows(2) {
        assert!(pair[0].x <= pair[1].x + 1e-6);
    }
}

#[test]
fn the_corridor_is_rebased_onto_the_moved_line() {
    let (mut points, mut corridor) = straight(200);
    toward(
        &mut points,
        &mut corridor,
        &[Target {
            index: 100,
            offset: 6.0,
        }],
        5.0,
        20.0,
        0.0,
    );
    assert!((corridor[100].left - -16.0).abs() < 1e-5);
    assert!((corridor[100].right - 4.0).abs() < 1e-5);
    assert_eq!(corridor[10].left, -10.0);
}

#[test]
fn a_target_past_the_corridor_is_clamped_inside_it_by_the_margin() {
    let (mut points, mut corridor) = straight(200);
    toward(
        &mut points,
        &mut corridor,
        &[Target {
            index: 100,
            offset: 40.0,
        }],
        5.0,
        20.0,
        2.0,
    );
    assert!((points[100].x - 8.0).abs() < 1e-5);
}

#[test]
fn the_move_wraps_round_the_ring() {
    // A circle of radius 100, one unit of arc apart near enough; lateral is
    // radial outward, which is what "right" is driving clockwise seen from
    // above.
    let n = 628;
    let mut points: Vec<Vec3> = Vec::new();
    let mut corridor = Vec::new();
    for i in 0..n {
        let a = i as f32 / n as f32 * std::f32::consts::TAU;
        let radial = Vec3::new(a.cos(), 0.0, a.sin());
        points.push(radial * 100.0);
        corridor.push(Frame {
            lateral: radial,
            left: -10.0,
            right: 10.0,
        });
    }
    toward(
        &mut points,
        &mut corridor,
        &[Target {
            index: 0,
            offset: -3.0,
        }],
        2.0,
        10.0,
        0.0,
    );
    assert!((points[n - 1].length() - 97.0).abs() < 1e-3);
    assert!((points[1].length() - 97.0).abs() < 1e-3);
    assert!((points[n / 2].length() - 100.0).abs() < 1e-3);
}

#[test]
fn no_targets_moves_nothing() {
    let (mut points, mut corridor) = straight(50);
    let before = (points.clone(), corridor.clone());
    toward(&mut points, &mut corridor, &[], 5.0, 20.0, 0.0);
    assert_eq!((points, corridor), before);
}
