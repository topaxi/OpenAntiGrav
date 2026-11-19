use super::*;
use bytemuck::Zeroable;
use oag_core::math::Vec4;

fn widget() -> TrackModel {
    TrackModel {
        origin: [1308.0, 440.0],
        offset: [0.0, 0.0],
        z: -180.0,
        ortho_scale: [1.0; 3],
    }
}

fn law() -> oag_title::CircuitTurntable {
    oag_hd::frontend::CIRCUIT_TURNTABLE
}

/// Vineta K's entry on the disc: `RotX="0.35" PosZ="-135" PosY="-20"`.
fn vineta_k() -> oag_title::CircuitPose {
    oag_title::CircuitPose {
        rot_x: 0.35,
        pos_y: -20.0,
        pos_z: -135.0,
    }
}

fn entry(yaw: f32) -> Posing {
    Posing::Entry {
        pose: vineta_k(),
        law: law(),
        yaw,
    }
}

fn pixel_of_origin(posing: Posing) -> (f32, f32) {
    let (view_projection, model) = matrices(&widget(), Space::HD, posing);
    let clip = view_projection * model * Vec4::new(0.0, 0.0, 0.0, 1.0);
    let ndc = (clip.x / clip.w, clip.y / clip.w);
    (
        (ndc.0 + 1.0) * 0.5 * Space::HD.size.0,
        (1.0 - ndc.1) * 0.5 * Space::HD.size.1,
    )
}

/// The widget's authored `(1308, 440)` is where an unread title's model sits.
#[test]
fn the_standin_origin_lands_on_the_authored_pixel() {
    let pixel = pixel_of_origin(Posing::Standin { seconds: 3.0 });
    assert!((pixel.0 - 1308.0).abs() < 0.5, "x {}", pixel.0);
    assert!((pixel.1 - 440.0).abs() < 0.5, "y {}", pixel.1);
}

/// The update puts the origin at `440 + PosY` down: the circuit's own entry moves the
/// model, the widget's `OriginY` does not.
#[test]
fn the_entry_moves_the_origin_by_its_pos_y() {
    let pixel = pixel_of_origin(entry(0.0));
    assert!((pixel.0 - 1308.0).abs() < 0.5, "x {}", pixel.0);
    assert!((pixel.1 - 420.0).abs() < 0.5, "y {}", pixel.1);
}

/// The entry's depth and pitch replace the widget's `z` and the chosen pitch: the model
/// stands `PosZ` in front of the camera, and a pitch of `RotX` tips the axis.
#[test]
fn the_entry_sets_the_depth_and_the_pitch() {
    let (_, model) = matrices(&widget(), Space::HD, entry(0.0));
    let at = model * Vec4::new(0.0, 0.0, 0.0, 1.0);
    assert!((at.z + 135.0).abs() < 1e-3, "{}", at.z);
    let up = model * Vec4::new(0.0, 100.0, 0.0, 0.0);
    assert!((up.y - 100.0 * 0.35_f32.cos()).abs() < 1e-3, "{}", up.y);
    assert!((up.z - 100.0 * 0.35_f32.sin()).abs() < 1e-3, "{}", up.z);
}

/// An unread title's circuit turns: a later second is a different pose, one
/// revolution later the same one.
#[test]
fn the_standin_turns_once_per_revolution() {
    let at = |seconds| matrices(&widget(), Space::HD, Posing::Standin { seconds }).1;
    assert_ne!(at(0.0), at(1.0));
    assert!(at(0.0).abs_diff_eq(at(TURN_SECONDS), 1e-4));
}

/// The angle is `0.01` rad a frame and a second is sixty frames: HD's one revolution is 629
/// frames, a little over ten seconds.
#[test]
fn the_turntable_steps_a_hundredth_of_a_radian_a_frame() {
    let law = law();
    let mut table = Turntable::default();
    for _ in 0..60 {
        table.advance(&law, 1.0 / 60.0, false);
    }
    assert!((table.angle() - 0.6).abs() < 1e-4, "{}", table.angle());
    // A second in one call is the same sixty frames.
    assert!((Turntable::after(&law, 1.0, false).angle() - 0.6).abs() < 1e-4);
    // A tick shorter than a frame is carried, not dropped.
    let mut table = Turntable::default();
    for _ in 0..240 {
        table.advance(&law, 1.0 / 240.0, false);
    }
    assert!((table.angle() - 0.6).abs() < 1e-4, "{}", table.angle());
}

/// A reversed circuit turns the other way, and the angle wraps at `+-pi` to the other end
/// (not by the excess).
#[test]
fn the_turntable_runs_backwards_on_a_reversed_circuit_and_wraps() {
    let law = law();
    assert!((Turntable::after(&law, 1.0, true).angle() + 0.6).abs() < 1e-4);
    // 314 frames reach 3.14, 315 pass pi (3.1415927) and land on -pi.
    let at = |frames: f32| Turntable::after(&law, frames / 60.0, false).angle();
    assert!((at(314.0) - 314.0 * law.step).abs() < 1e-3, "{}", at(314.0));
    assert_eq!(at(315.0), -law.limit);
    assert!((at(316.0) - (-law.limit + 0.01)).abs() < 1e-5);
    let back = Turntable::after(&law, 315.0 / 60.0, true).angle();
    assert_eq!(back, law.limit);
}

/// A title with the law and an entry that authors a pose draws the entry's pose; one without
/// either keeps the stand-in.
#[test]
fn the_posing_follows_the_law_and_the_entry() {
    let table = Turntable::after(&law(), 1.0, false);
    assert_eq!(
        Posing::of(Some(law()), Some(vineta_k()), &table, 1.0),
        entry(table.angle())
    );
    let standin = Posing::Standin { seconds: 1.0 };
    assert_eq!(Posing::of(None, Some(vineta_k()), &table, 1.0), standin);
    assert_eq!(Posing::of(Some(law()), None, &table, 1.0), standin);
}

fn ramp() -> Ramp {
    Ramp {
        row: vec![[0.8; 3], [0.6; 3], [0.4; 3]],
    }
}

/// The ramp is read at `N.V`: a surface facing the camera takes its far end
/// and a grazing one its near end, and a back face is clamped to the near end.
#[test]
fn the_ramp_is_read_at_the_facing() {
    let ramp = ramp();
    assert_eq!(ramp.at(0.0), [0.8; 3]);
    assert_eq!(ramp.at(0.5), [0.6; 3]);
    assert_eq!(ramp.at(1.0), [0.4; 3]);
    assert_eq!(ramp.at(-0.7), [0.8; 3]);
}

/// A vertex 180 units in front of the camera, facing it, is dark; the same
/// vertex turned edge-on is bright.
#[test]
fn shading_follows_the_view_vector() {
    let ramp = ramp();
    let mut vertices = [GpuVertex::zeroed(), GpuVertex::zeroed()];
    vertices[0].normal = [0.0, 0.0, 1.0];
    vertices[1].normal = [1.0, 0.0, 0.0];
    ramp.shade(
        &mut vertices,
        Mat4::from_translation(Vec3::new(0.0, 0.0, -180.0)),
    );
    assert_eq!(vertices[0].colour, [0.4, 0.4, 0.4, 1.0]);
    assert_eq!(vertices[1].colour, [0.8, 0.8, 0.8, 1.0]);
}

/// Omega's widget authors `x y z` of `0.525 0.125 -2.05` under an
/// `orthoScale` of `0.012`, so the circuit stands `1 / 0.012` times as far:
/// 43.75 right, 10.4 up and 170.8 in front, close to HD's 180.
#[test]
fn an_ortho_scale_scales_the_placement() {
    let widget = TrackModel {
        origin: [960.0, 540.0],
        offset: [0.525, 0.125],
        z: -2.05,
        ortho_scale: [0.012; 3],
    };
    let (_, model) = matrices(&widget, Space::HD, Posing::Standin { seconds: 0.0 });
    let at = model.transform_point3(Vec3::ZERO);
    assert!((at.x - 43.75).abs() < 1e-3, "{at:?}");
    assert!((at.y - 10.417).abs() < 1e-3, "{at:?}");
    assert!((at.z + 170.833).abs() < 1e-3, "{at:?}");
}

/// A material's uniforms make the ramp: `colourDiffAlpha + Constant1 *
/// (min + scale * (1 - N.V)^power)` with the ambient colour taken as 1. Face
/// on it is the base plus the minimum, edge on the base plus min plus scale.
#[test]
fn the_fresnel_ramp_is_the_materials_own_uniforms() {
    use oag_rcs::rcsmaterial::name_hash;
    let uniform = |name: &str, value: &[f32]| oag_rcs::rcsmodel::psp2::material::Param {
        hash: name_hash(name),
        bits: value.iter().map(|v| v.to_bits()).collect(),
    };
    let material = oag_rcs::rcsmodel::psp2::material::Material {
        name: String::new(),
        technique: None,
        textures: Vec::new(),
        lightmap: None,
        params: vec![
            uniform("colourDiffAlpha", &[0.06, 0.048, 0.029, 1.0]),
            uniform("Constant1", &[1.0; 3]),
            uniform("ReflectivityMin", &[0.05]),
            uniform("ReflectivityScale", &[2.0]),
            uniform("ReflectivityPower", &[3.0]),
        ],
        samplers: Vec::new(),
        state: None,
    };
    let mut model = Model::none("test");
    let ramp = Ramp::fresnel(&material, &mut model).expect("every uniform is authored");
    let (face, edge) = (ramp.at(1.0), ramp.at(0.0));
    assert!((face[0] - 0.11).abs() < 1e-5, "{face:?}");
    assert!((edge[0] - 2.11).abs() < 1e-5, "{edge:?}");
    assert!((ramp.at(0.5)[1] - (0.048 + 0.05 + 2.0 * 0.125)).abs() < 5e-3);
    // A material that authors none of them is refused, not guessed.
    let mut bare = material.clone();
    bare.params.clear();
    assert!(Ramp::fresnel(&bare, &mut Model::none("test")).is_err());
}

fn track_picker(
    reversed: bool,
    pose: Option<oag_title::CircuitPose>,
) -> oag_ui_screens::picker::Picker {
    use oag_ui_screens::picker::{Details, Entry, Kind, Picker};
    let entries = vec![Entry {
        id: "01_Track".to_string(),
        label: "VINETA K".to_string(),
        details: Details::Track {
            info: ["-".into(), "-".into(), "-".into()],
            emblem: None,
            icon: None,
            reversed,
            model_pose: pose,
        },
    }];
    Picker::new(Kind::Track, entries, Some("01_Track"), None)
}

/// What a capture draws: on HD the selected entry's own pose and the turntable counted from zero
/// in its direction; on a title whose update is unread, the stand-in. Dropping the wiring (the
/// capture passing the stand-in, or the entry's pose not reaching the picker) fails this.
#[test]
fn a_capture_poses_the_selected_circuit_by_its_entry() {
    let hd = Some(oag_hd::frontend::FRONT_END);
    let forward = track_picker(false, Some(vineta_k()));
    let Posing::Entry { pose, law, yaw } = Posing::after(hd, &forward, 1.0) else {
        panic!("HD with an authored pose draws the entry's pose");
    };
    assert_eq!(pose, vineta_k());
    assert_eq!(law, oag_hd::frontend::CIRCUIT_TURNTABLE);
    assert!((yaw - 0.6).abs() < 1e-4, "{yaw}");
    let reversed = track_picker(true, Some(vineta_k()));
    let Posing::Entry { yaw, .. } = Posing::after(hd, &reversed, 1.0) else {
        panic!("a reversed circuit is posed the same way");
    };
    assert!((yaw + 0.6).abs() < 1e-4, "{yaw}");
    let standin = Posing::Standin { seconds: 1.0 };
    assert_eq!(
        Posing::after(Some(oag_omega::frontend::FRONT_END), &forward, 1.0),
        standin
    );
    assert_eq!(Posing::after(hd, &track_picker(false, None), 1.0), standin);
}
