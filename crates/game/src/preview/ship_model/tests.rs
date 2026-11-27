use super::*;

const FIT: ShipFit = oag_hd::frontend::SHIP_FIT;
const ASPECT: f32 = 16.0 / 9.0;

/// HD's authored `ShipModel` at `origin`.
fn widget(origin: [f32; 2]) -> ShipModel {
    ShipModel {
        origin,
        position: [0.0; 2],
        z: -24.0,
        scale: 1.0,
        near: NEAR,
        fov_degrees: 57.3,
        zoom: 1.0,
        rot_x: 0.4,
        rot_y: -0.5,
        obey_safe_zone: true,
    }
}

#[test]
fn the_authored_origin_and_depth_place_the_craft() {
    let widget = widget([960.0, 540.0]);
    let (_, model) = matrices(&widget, &Craft::default(), Space::HD, 1.0);
    let at = model.transform_point3(Vec3::ZERO);
    assert!((at.z + 24.0).abs() < 1e-4);
}

/// The rest pair straddles the window's edge: the box fits a twentieth of a
/// unit farther than the rest depth and not a twentieth nearer.
#[test]
fn the_fit_rests_where_the_box_just_fits_the_window() {
    for length in [3.8, 4.1, 4.5] {
        let box_z = [-length, length];
        let framing = rest(&FIT, ASPECT, box_z);
        assert!(framing.depth > FIT.floor, "{length}: {framing:?}");
        let half = 0.5 * FIT.step;
        assert!(measure(&FIT, [0.0; 2], ASPECT, box_z, framing.depth + half).1);
        assert!(!measure(&FIT, [0.0; 2], ASPECT, box_z, framing.depth - half).1);
    }
}

/// A longer model rests farther away, and a box short enough to fit at the
/// floor rests on it (the disc's shortest, `length="3.2"`, does).
#[test]
fn a_longer_model_rests_farther_and_a_short_one_on_the_floor() {
    let depth = |length: f32| rest(&FIT, ASPECT, [-length, length]).depth;
    assert!(depth(4.5) > depth(3.8) + 1.0);
    assert!(depth(3.8) > depth(3.2) + 0.5);
    assert!((depth(3.2) - FIT.floor).abs() < 1e-4);
}

/// The window is in the fit's own unit, a device coordinate times the whole
/// screen: Feisar's box (`length="3.8"`) rests 535 of them wide, which is
/// 535 pixels of a 1920 grid across both halves, not 1070.
#[test]
fn the_window_is_measured_in_device_coordinates_times_the_screen() {
    let box_z = [-3.8, 3.8];
    let framing = rest(&FIT, ASPECT, box_z);
    let (view_projection, model) = fitted(&FIT, framing, &Craft::default(), Space::HD, 1.0);
    let (mut low, mut high) = (f32::MAX, f32::MIN);
    for corner in 0..8 {
        let at = Vec3::new(
            if corner & 2 == 0 { -1.125 } else { 1.125 },
            if corner & 1 == 0 { -0.75 } else { 0.75 },
            box_z[corner >> 2],
        );
        let x = (view_projection * model).project_point3(at).x * 960.0;
        low = low.min(x);
        high = high.max(x);
    }
    let wide = high - low;
    assert!((wide - 535.0).abs() < 8.0, "{wide} px at {framing:?}");
}

/// The origin the fit slides is the screen's own constant, not the
/// widget's authored one, and the Zone hull is fitted to the Zone box.
#[test]
fn the_fit_uses_its_own_origin_and_the_zone_box() {
    assert_eq!(FIT.origin, [1220.0, 500.0]);
    let zone = Some(oag_race::Mode::Zone);
    let detonator = Some(oag_race::Mode::Detonator);
    assert_eq!(box_z(&FIT, zone, Some(3.8)), Some([-4.2, 4.2]));
    assert_eq!(box_z(&FIT, detonator, Some(3.8)), Some([-2.0, 0.0]));
    assert_eq!(box_z(&FIT, None, Some(3.8)), Some([-3.8, 3.8]));
    assert_eq!(box_z(&FIT, None, None), None);
    let widget = widget([1220.0, 412.0]);
    let framing = rest(&FIT, ASPECT, [-3.8, 3.8]);
    let authored = framed(&widget, None, &Craft::default(), Space::HD, 1.0);
    let fit = framed(
        &widget,
        Some((&FIT, framing)),
        &Craft::default(),
        Space::HD,
        1.0,
    );
    assert_ne!(authored, fit);
    assert_eq!(
        fit,
        fitted(&FIT, framing, &Craft::default(), Space::HD, 1.0)
    );
}

/// `ObeySafeZone` shrinks the picture and pulls its origin towards the
/// middle of the screen by the Safe Area; a widget without it is untouched.
#[test]
fn an_obeying_widget_is_drawn_inside_the_safe_area() {
    let mut widget = widget([1220.0, 412.0]);
    let fit = Some((&FIT, rest(&FIT, ASPECT, [-3.8, 3.8])));
    let at = |matrices: (Mat4, Mat4), point: Vec3| (matrices.0 * matrices.1).project_point3(point);
    let full = framed(&widget, fit, &Craft::default(), Space::HD, 1.0);
    let inset = framed(&widget, fit, &Craft::default(), Space::HD, 0.96);
    for point in [Vec3::ZERO, Vec3::new(1.0, 0.5, -3.0)] {
        let (full, inset) = (at(full, point), at(inset, point));
        assert!((inset.x - 0.96 * full.x).abs() < 1e-5);
        assert!((inset.y - 0.96 * full.y).abs() < 1e-5);
    }
    widget.obey_safe_zone = false;
    assert_eq!(
        framed(&widget, fit, &Craft::default(), Space::HD, 0.96),
        full
    );
}

/// A step to a model that rests deeper keeps the fit's state: the target
/// walks a step a frame from where the last model left it, and the drawn
/// depth eases to the target at the fit's rate. A plain ease from the old
/// rest to the new one would move the first frame by an eighth of the whole
/// 1.6, not by an eighth of the 0.1 the target has walked.
#[test]
fn a_step_walks_the_target_and_the_drawn_depth_follows_it() {
    let (from, to) = ([-3.8, 3.8], [-4.5, 4.5]);
    let (old, new) = (rest(&FIT, ASPECT, from).depth, rest(&FIT, ASPECT, to).depth);
    let mut motion = FitMotion::open(&FIT);
    for _ in 0..600 {
        motion.frame(&FIT, ASPECT, from);
    }
    let before = motion.target();
    let (mut drawn, mut target) = (vec![motion.framing().depth], vec![before]);
    assert!((drawn[0] - old).abs() < 0.02, "{drawn:?} against {old}");
    for _ in 0..80 {
        motion.frame(&FIT, ASPECT, to);
        drawn.push(motion.framing().depth);
        target.push(motion.target());
    }
    for frame in 0..80 {
        let eased = drawn[frame] + FIT.depth_ease * (target[frame + 1] - drawn[frame]);
        assert!(
            (drawn[frame + 1] - eased).abs() < 1.0e-4,
            "frame {frame}: {} against {eased}",
            drawn[frame + 1]
        );
    }
    for frame in 1..8 {
        assert!(
            (target[frame + 1] - target[frame] - FIT.step).abs() < 1.0e-4,
            "{target:?}"
        );
    }
    assert!(drawn[1] - drawn[0] < 0.05, "{drawn:?}");
    assert!(drawn[10] < new - 0.5, "{drawn:?}");
    assert!(
        (drawn[80] - new).abs() < 0.02,
        "{} against {new}",
        drawn[80]
    );
}

/// One sample of the original's screen: the frames since the selection moved, the depth
/// target, the drawn depth and the drawn slide.
type Sample = (usize, f32, f32, f32, f32);

/// Seven steps of `TeamSelection_Screen` read out of RPCS3's RAM a game frame at a time
/// (`st+0x1014`, `+0x1020`, `+0x1018`, `+0x101c`, the frame being a change of the tuple;
/// `docs/ghidra/functions/ps3-hdfury-eu/team-selection-model.md`, "The step against RAM"):
/// the old and new model's `<FE Length>`, the target and drawn depth on the frame the
/// selection moved (the fit still ran on the old model), and the frames after it.
#[allow(clippy::type_complexity, reason = "a table of reads")]
const STEPS_IN_RAM: [(f32, f32, (f32, f32), [Sample; 9]); 7] = [
    (
        3.2,
        3.8,
        (11.0, 11.0000),
        [
            (1, 11.100, 11.0125, 95.32, -82.46),
            (2, 11.200, 11.0359, 97.49, -83.70),
            (3, 11.300, 11.0689, 99.37, -84.74),
            (5, 11.500, 11.1590, 102.34, -86.27),
            (8, 11.800, 11.3405, 105.15, -87.41),
            (12, 12.000, 11.6160, 106.47, -87.29),
            (18, 12.000, 11.8534, 107.32, -86.68),
            (24, 12.000, 11.9599, 107.90, -86.27),
            (30, 12.000, 12.0077, 108.29, -85.99),
        ],
    ),
    (
        3.8,
        4.5,
        (12.0, 12.0467),
        [
            (1, 12.100, 12.0533, 112.08, -87.07),
            (2, 12.200, 12.0717, 114.69, -88.52),
            (3, 12.300, 12.1002, 116.96, -89.75),
            (5, 12.500, 12.1830, 120.60, -91.62),
            (8, 12.800, 12.3566, 124.15, -93.20),
            (12, 13.200, 12.6504, 126.08, -93.52),
            (18, 13.600, 13.1425, 124.82, -91.40),
            (24, 13.600, 13.4204, 122.83, -89.13),
            (30, 13.600, 13.5451, 121.48, -87.59),
        ],
    ),
    (
        4.5,
        3.9,
        (13.7, 13.6533),
        [
            (1, 13.600, 13.6467, 116.75, -83.31),
            (2, 13.500, 13.6283, 115.03, -82.37),
            (3, 13.400, 13.5998, 113.50, -81.56),
            (5, 13.200, 13.5170, 110.99, -80.30),
            (8, 12.900, 13.3434, 108.45, -79.22),
            (12, 12.500, 13.0496, 107.06, -79.09),
            (18, 12.300, 12.6235, 108.05, -80.94),
            (24, 12.300, 12.4195, 109.07, -82.48),
            (30, 12.300, 12.3279, 109.77, -83.52),
        ],
    ),
    (
        3.9,
        3.8,
        (12.3, 12.2533),
        [
            (1, 12.200, 12.2467, 110.82, -85.47),
            (2, 12.100, 12.2283, 110.53, -85.32),
            (3, 12.000, 12.1998, 110.39, -85.28),
            (5, 12.000, 12.1639, 110.24, -85.29),
            (8, 12.100, 12.1319, 110.10, -85.36),
            (12, 12.100, 12.0994, 109.89, -85.38),
            (18, 12.100, 12.0740, 109.67, -85.40),
            (24, 12.100, 12.0626, 109.51, -85.41),
            (30, 12.100, 12.0575, 109.41, -85.42),
        ],
    ),
    (
        3.8,
        3.9,
        (12.0, 12.0467),
        [
            (1, 12.100, 12.0533, 109.56, -85.66),
            (2, 12.200, 12.0717, 109.85, -85.81),
            (3, 12.300, 12.1002, 110.00, -85.85),
            (5, 12.300, 12.1361, 110.15, -85.83),
            (8, 12.200, 12.1681, 110.28, -85.76),
            (12, 12.200, 12.2006, 110.48, -85.74),
            (18, 12.200, 12.2260, 110.70, -85.72),
            (24, 12.200, 12.2374, 110.85, -85.70),
            (30, 12.200, 12.2425, 110.96, -85.69),
        ],
    ),
    (
        3.9,
        4.5,
        (12.2, 12.2467),
        [
            (1, 12.300, 12.2533, 113.66, -87.08),
            (2, 12.400, 12.2717, 115.83, -88.27),
            (3, 12.500, 12.3002, 117.70, -89.28),
            (5, 12.700, 12.3830, 120.65, -90.76),
            (8, 13.000, 12.5566, 123.36, -91.88),
            (12, 13.400, 12.8504, 124.50, -91.79),
            (18, 13.600, 13.2765, 122.97, -89.65),
            (24, 13.600, 13.4805, 121.57, -87.94),
            (30, 13.600, 13.5721, 120.62, -86.78),
        ],
    ),
    (
        4.5,
        3.8,
        (13.6, 13.6467),
        [
            (1, 13.500, 13.6283, 116.49, -83.17),
            (2, 13.400, 13.5998, 114.58, -82.15),
            (3, 13.300, 13.5623, 112.87, -81.26),
            (5, 13.100, 13.4649, 110.02, -79.84),
            (8, 12.800, 13.2755, 107.04, -78.56),
            (12, 12.400, 12.9684, 105.13, -78.20),
            (18, 12.000, 12.4878, 105.70, -79.94),
            (24, 12.000, 12.2447, 106.80, -81.69),
            (30, 12.000, 12.1355, 107.55, -82.88),
        ],
    ),
];

/// The screen's own frames after a step, against ours. The depth target and drawn depth
/// agree to the last digit read and the slide to 0.1 grid units, once ours is on the same
/// side of the rest pair's alternation as the original was when the selection moved (which
/// frame the player's press fell on is nobody's law).
#[test]
fn a_step_follows_the_screens_own_frames_in_ram() {
    for (from, to, (target0, depth0), samples) in STEPS_IN_RAM {
        let mut motion = FitMotion::open(&FIT);
        for _ in 0..600 {
            motion.frame(&FIT, ASPECT, [-from, from]);
        }
        if (motion.target() - target0).abs() > 0.05 {
            motion.frame(&FIT, ASPECT, [-from, from]);
        }
        let at = motion.framing();
        assert!(
            (motion.target() - target0).abs() < 0.01 && (at.depth - depth0).abs() < 1.0e-3,
            "{from} to {to} before the step: {} {} against {target0} {depth0}",
            motion.target(),
            at.depth
        );
        let mut frame = 0;
        for (k, target, depth, across, down) in samples {
            while frame < k {
                motion.frame(&FIT, ASPECT, [-to, to]);
                frame += 1;
            }
            let at = motion.framing();
            let label = format!("{from} to {to}, frame {k}");
            assert!((motion.target() - target).abs() < 0.01, "{label}: target");
            assert!(
                (at.depth - depth).abs() < 2.0e-3,
                "{label}: depth {}",
                at.depth
            );
            assert!(
                (at.slide[0] - across).abs() < 0.15,
                "{label}: slide {:?}",
                at.slide
            );
            assert!(
                (at.slide[1] - down).abs() < 0.15,
                "{label}: slide {:?}",
                at.slide
            );
        }
    }
}

/// The tick may be any length: sixty ticks of a sixtieth and thirty of a
/// thirtieth play the same sixty frames, and the box
/// asked for on each tick is the one the frame uses, so a step needs no
/// event of its own.
#[test]
fn a_tick_of_any_length_plays_the_same_frames() {
    let play = |ticks: u32| {
        let mut player = ShipPlayer::open(Some(&FIT), None);
        let dt = 1.0 / ticks as f32;
        for tick in 0..ticks {
            let length = if tick < ticks / 2 { 3.2 } else { 4.5 };
            player.tick(
                Some((&FIT, ASPECT, [-length, length])),
                &control::CraftInput::default(),
                dt,
            );
        }
        player.framing().expect("a fit")
    };
    let (a, b) = (play(60), play(30));
    assert!((a.depth - b.depth).abs() < 0.05, "{a:?} against {b:?}");
    assert!(a.depth > FIT.floor + 0.05, "{a:?}");
}

/// A widget's pose is its position, rotation and scale through its lens, slid to its
/// origin and zoomed about it: the model's origin lands at the widget's position, a point
/// `zoom` times as far from the origin on screen, and the Safe Area pulls both in.
#[test]
fn a_widgets_own_pose_places_scales_and_zooms_the_craft() {
    let mut posed = widget([960.0, 540.0]);
    posed.position = [0.4, 0.1];
    posed.z = -2.05;
    posed.scale = 0.1;
    posed.fov_degrees = 100.0;
    let on_screen = |widget: &ShipModel, safe_area: f32, point: Vec3| {
        let (view_projection, model) = matrices(widget, &Craft::default(), Space::HD, safe_area);
        (view_projection * model).project_point3(point)
    };
    let origin = on_screen(&posed, 1.0, Vec3::ZERO);
    let expected_x = 0.4 / (2.05 * 50.0_f32.to_radians().tan() * ASPECT);
    assert!((origin.x - expected_x).abs() < 1e-4, "{origin:?}");
    let mut zoomed = posed;
    zoomed.zoom = 2.5;
    let origin_zoomed = on_screen(&zoomed, 1.0, Vec3::ZERO);
    assert!((origin_zoomed.x - 2.5 * origin.x).abs() < 1e-4);
    assert!((origin_zoomed.y - 2.5 * origin.y).abs() < 1e-4);
    let inset = on_screen(&zoomed, 0.96, Vec3::ZERO);
    assert!((inset.x - 0.96 * origin_zoomed.x).abs() < 1e-4);
}

/// The fit measures its box through the rotation the player has turned the craft to, as the
/// original's projector adds the terms to the rest rotation: a craft turned about Y rests at
/// another depth or slide than the craft at rest.
#[test]
fn a_turned_craft_is_refitted_through_its_turn() {
    let box_z = [-3.8, 3.8];
    let settle = |turned: [f32; 2]| {
        let mut motion = FitMotion::open(&FIT);
        for _ in 0..600 {
            motion.frame_turned(&FIT, ASPECT, box_z, turned);
        }
        motion.framing()
    };
    let (rest, turned) = (settle([0.0; 2]), settle([0.0, 1.2]));
    assert!(
        (rest.depth - turned.depth).abs() > 0.3 || (rest.slide[0] - turned.slide[0]).abs() > 5.0,
        "{rest:?} against {turned:?}"
    );
    let mut plain = FitMotion::open(&FIT);
    for _ in 0..600 {
        plain.frame(&FIT, ASPECT, box_z);
    }
    assert_eq!(rest, plain.framing(), "no turn is the old law");
}

/// A step to another model drives none of the player's terms: with the devices idle a craft
/// stays at rest across it, and a turned craft stays turned across one until the idle count
/// returns it.
#[test]
fn a_step_leaves_the_players_terms_alone() {
    let law = oag_hd::frontend::SHIP_CONTROL;
    let mut player = ShipPlayer::open(Some(&FIT), Some(law));
    let idle = control::CraftInput::default();
    for frame in 0..120 {
        let length = if frame < 60 { 3.2 } else { 4.5 };
        player.tick(
            Some((&FIT, ASPECT, [-length, length])),
            &idle,
            FRAME_SECONDS,
        );
        assert_eq!(player.craft(), Craft::default(), "frame {frame}");
    }
    let turn = control::CraftInput {
        right: (1.0, 0.0),
        ..idle
    };
    for _ in 0..30 {
        player.tick(Some((&FIT, ASPECT, [-3.2, 3.2])), &turn, FRAME_SECONDS);
    }
    let turned = player.craft();
    for _ in 0..30 {
        player.tick(Some((&FIT, ASPECT, [-4.5, 4.5])), &idle, FRAME_SECONDS);
    }
    assert!(turned.turn[1] != 0.0 && player.craft().turn[1].abs() > turned.turn[1].abs() * 0.9);
}

/// The player's turn, zoom and pan reach the picture: zero is the rest picture exactly, a turn
/// rotates the model, a zoom narrows the lens and a pan moves it.
#[test]
fn the_craft_terms_change_the_picture_and_zero_leaves_it() {
    let framing = rest(&FIT, ASPECT, [-3.8, 3.8]);
    let draw = |craft: Craft| fitted(&FIT, framing, &craft, Space::HD, 1.0);
    let at_rest = draw(Craft::default());
    let turned = draw(Craft {
        turn: [0.0, 0.5],
        ..Craft::default()
    });
    let zoomed = draw(Craft {
        fov_delta: -30.0,
        ..Craft::default()
    });
    let panned = draw(Craft {
        pan: [1.0, 0.5],
        ..Craft::default()
    });
    assert_ne!(at_rest.1, turned.1);
    assert_ne!(at_rest.0, zoomed.0);
    assert_eq!(at_rest.1, zoomed.1);
    let origin = |matrices: (Mat4, Mat4)| matrices.1.transform_point3(Vec3::ZERO);
    assert!((origin(panned) - origin(at_rest) - Vec3::new(1.0, 0.5, 0.0)).length() < 1.0e-4);
    let widget = widget([960.0, 540.0]);
    let authored = matrices(&widget, &Craft::default(), Space::HD, 1.0);
    let turned_widget = matrices(
        &widget,
        &Craft {
            turn: [0.3, 0.0],
            ..Craft::default()
        },
        Space::HD,
        1.0,
    );
    assert_ne!(authored.1, turned_widget.1);
}

/// Omega's pose draws at its own scale and depth: a full pan at a full pull keeps the craft's
/// origin inside the screen rather than flinging it off.
#[test]
fn omegas_full_pan_stays_on_screen() {
    let law = oag_omega::frontend::SHIP_CONTROL;
    let mut posed = widget([960.0, 540.0]);
    posed.position = [0.4, 0.1];
    posed.z = -2.05;
    posed.scale = 0.1;
    posed.fov_degrees = 100.0;
    posed.zoom = 2.5;
    let craft = Craft {
        pan: [law.pan_scale[0], law.pan_scale[1]],
        fov_delta: -30.0,
        ..Craft::default()
    };
    let (view_projection, model) = matrices(&posed, &craft, Space::HD, 1.0);
    let at = (view_projection * model).project_point3(Vec3::ZERO);
    assert!(at.x.abs() < 1.0 && at.y.abs() < 1.0, "{at:?}");
}
