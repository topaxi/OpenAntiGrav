use super::*;

const DT: f32 = 1.0 / 60.0;

/// The frames a run draws, from the arm to the first frame past its end.
fn run() -> Vec<Option<[Vertex; VERTICES]>> {
    let mut node = Node::default();
    let mut rng = Rng::new(1);
    node.arm();
    (0..60)
        .map(|_| {
            node.advance(DT, &mut rng);
            node.strips()
        })
        .collect()
}

/// Live, on two boots, the node ran for exactly 36 frames, `p` from 0 at the hit.
#[test]
fn a_run_lasts_thirty_six_frames() {
    let frames = run();
    let drawn = frames.iter().take_while(|frame| frame.is_some()).count();
    assert_eq!(drawn, 36);
    assert!(frames[36..].iter().all(Option::is_none), "it ran on");
}

/// `p` is `age * (1 / 0.6)`, 0 on the frame of the hit and `35 / 36` on the last.
#[test]
fn the_progress_steps_from_zero_by_a_thirty_sixth() {
    let mut node = Node::default();
    let mut rng = Rng::new(1);
    node.arm();
    node.advance(DT, &mut rng);
    assert_eq!(node.progress(), 0.0);
    assert_eq!(node.width(), STRIP_WIDTH);
    node.advance(DT, &mut rng);
    assert!((node.progress() - 1.0 / 36.0).abs() < 1e-5);
    for _ in 0..34 {
        node.advance(DT, &mut rng);
    }
    assert!((node.progress() - 35.0 / 36.0).abs() < 1e-4);
    assert!((node.width() - STRIP_WIDTH / 36.0).abs() < 0.01);
    assert!(node.running());
}

/// An arm while it runs is ignored: no restart, no stacking.
#[test]
fn an_arm_while_it_runs_changes_nothing() {
    let mut node = Node::default();
    let mut rng = Rng::new(1);
    node.arm();
    for _ in 0..10 {
        node.advance(DT, &mut rng);
    }
    let before = node.clone();
    node.arm();
    assert_eq!(node, before);
    // Past the end a new arm starts a fresh run.
    for _ in 0..40 {
        node.advance(DT, &mut rng);
    }
    assert!(!node.running());
    node.arm();
    node.advance(DT, &mut rng);
    assert!(node.running());
    assert_eq!(node.progress(), 0.0);
}

/// At the hit each strip is 288 of 480 pixels wide: 0.6 of the view, the left
/// from the screen's left edge and the right mirrored; opaque white at the
/// edge, `0x00000000` at the inner edge, `u` to `288 / 64 * 0.125`, `v` to
/// `272 / 64`.
#[test]
fn the_strips_at_the_hit_are_the_recovered_geometry() {
    let frames = run();
    let [l0, l1, l2, l3, r0, r1, r2, r3] = frames[0].expect("the first frame draws");
    let offset = |vertex: &Vertex| vertex.uv;
    let [offset_u, offset_v] = offset(&l0);
    assert!((0..8).any(|i| offset_u == i as f32 / 8.0), "{offset_u}");
    assert!((0..8).any(|i| offset_v == i as f32 / 8.0), "{offset_v}");

    assert_eq!(l0.position, [-1.0, 1.0]);
    assert_eq!(l2.position, [-1.0, -1.0]);
    assert!((l1.position[0] - (-1.0 + 2.0 * 0.6)).abs() < 1e-6, "{l1:?}");
    assert_eq!(l1.position[1], 1.0);
    assert_eq!(l3.position[1], -1.0);
    assert_eq!(r0.position, [1.0, 1.0]);
    assert_eq!(r2.position, [1.0, -1.0]);
    assert!((r1.position[0] - (1.0 - 2.0 * 0.6)).abs() < 1e-6, "{r1:?}");

    assert_eq!(l0.colour, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(l2.colour, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(l1.colour, [0.0; 4]);
    assert_eq!(l3.colour, [0.0; 4]);
    assert_eq!(r0.colour, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(r1.colour, [0.0; 4]);
    assert_eq!(r3.colour, [0.0; 4]);

    let reach = 288.0 / 64.0 * UV_SCALE_U;
    assert!((l1.uv[0] - (offset_u + reach)).abs() < 1e-6, "{l1:?}");
    assert!((r1.uv[0] - (offset_u + reach)).abs() < 1e-6, "{r1:?}");
    assert_eq!(l0.uv[0], offset_u);
    assert!((l2.uv[1] - (offset_v + 4.25)).abs() < 1e-6, "{l2:?}");
    assert!((r3.uv[1] - (offset_v + 4.25)).abs() < 1e-6, "{r3:?}");
}

/// Half way (`p = 1/2`) the strips are 144 pixels and the edge alpha is
/// `(1 - p) * 255` truncated to a byte.
#[test]
fn the_strips_close_in_and_fade_with_p() {
    let mut node = Node::default();
    let mut rng = Rng::new(1);
    node.arm();
    for _ in 0..19 {
        node.advance(DT, &mut rng);
    }
    assert!((node.progress() - 0.5).abs() < 1e-3);
    let strips = node.strips().expect("running");
    assert!(
        (strips[1].position[0] - (-1.0 + 0.6)).abs() < 2e-3,
        "{:?}",
        strips[1]
    );
    assert_eq!(strips[0].colour[3], 127.0 / 255.0);
    assert_eq!(strips[4].colour[3], 127.0 / 255.0);
}

/// The offset pair is redrawn every frame, and always a whole number of eighths.
#[test]
fn the_offsets_are_redrawn_in_eighths_every_frame() {
    let frames = run();
    let offsets: Vec<[f32; 2]> = frames.iter().flatten().map(|strips| strips[0].uv).collect();
    for [u, v] in &offsets {
        assert!(
            u * 8.0 == (u * 8.0).trunc() && (0.0..1.0).contains(u),
            "{u}"
        );
        assert!(
            v * 8.0 == (v * 8.0).trunc() && (0.0..1.0).contains(v),
            "{v}"
        );
    }
    assert!(
        offsets.windows(2).any(|pair| pair[0] != pair[1]),
        "the offsets never moved"
    );
}
