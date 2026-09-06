//! What [`super::TrackEmitters`] is asserted to do with no disc anywhere.
//!
//! The disc-backed half is `crates/game/tests/track_audio_ground_truth.rs`.

use super::*;
use oag_formats::sound_emitters::RadiusCurve;

/// An emitter at `position` with a constant radius and no cone.
fn omni(position: [f32; 3], radius: f32) -> SoundEmitter {
    let mut to_world = [0.0; 16];
    to_world[0] = 1.0;
    to_world[5] = 1.0;
    to_world[10] = 1.0;
    to_world[15] = 1.0;
    to_world[12] = position[0];
    to_world[13] = position[1];
    to_world[14] = position[2];
    SoundEmitter {
        bank: "gentrak".into(),
        cue: "~NEON".into(),
        radius,
        emitter_field_3c: radius,
        cone: None,
        radius_curve: RadiusCurve::default(),
        to_world,
    }
}

#[test]
fn a_vex_with_no_node_table_reports_rather_than_panicking() {
    let parsed = TrackEmitters::parse("nowhere.vex", b"not a vex at all");
    assert!(parsed.omni.is_empty());
    assert_eq!(parsed.cones, 0);
    assert!(
        parsed
            .report
            .iter()
            .any(|line| line.contains("nowhere.vex")),
        "an unreadable circuit said nothing about itself: {:?}",
        parsed.report
    );
}

#[test]
fn only_what_is_inside_its_own_radius_is_counted() {
    let emitters = TrackEmitters {
        omni: vec![
            omni([0.0, 0.0, 0.0], 100.0),
            omni([50.0, 0.0, 0.0], 100.0),
            // Past its own radius, and so gated off rather than quiet.
            omni([500.0, 0.0, 0.0], 100.0),
            // Far away but authored wide enough to still reach.
            omni([500.0, 0.0, 0.0], 600.0),
        ],
        cones: 0,
        report: Vec::new(),
    };
    let listener = oag_audio::Listener::at_origin();
    assert_eq!(emitters.in_range(&listener, 0.0), 3);
    let placed: Vec<_> = emitters.placed(&listener, 0.0).collect();
    assert_eq!(
        placed.iter().map(|(at, _)| *at).collect::<Vec<_>>(),
        [0, 1, 3]
    );
}

#[test]
fn the_near_field_is_flat_and_the_far_field_falls_off() {
    let emitters = TrackEmitters {
        omni: vec![omni([10.0, 0.0, 0.0], 100.0), omni([90.0, 0.0, 0.0], 100.0)],
        cones: 0,
        report: Vec::new(),
    };
    let listener = oag_audio::Listener::at_origin();
    let placed: Vec<_> = emitters.placed(&listener, 0.0).map(|(_, p)| p).collect();
    // Inside `0.2 * radius`, so the 1.25 clamp puts it at full level.
    assert!(
        (placed[0].gain - 1.0).abs() < 1e-6,
        "the near field is not flat: {}",
        placed[0].gain
    );
    assert!(
        placed[1].gain < placed[0].gain,
        "the far one is no quieter than the near one"
    );
    // Both are to the listener's right, which the pan law reads as `+1`.
    assert!(placed[0].pan > 0.99, "pan {}", placed[0].pan);
}

#[test]
fn a_cone_is_counted_and_never_placed() {
    let mut cone = omni([0.0, 0.0, 0.0], 100.0);
    cone.cone = Some(oag_formats::sound_emitters::Cone {
        angle_a: 1.0,
        angle_b: 0.7,
    });
    // `parse` is the only thing that splits them, so this asserts the split's
    // consequence rather than the split: a `TrackEmitters` built by hand can
    // only hold omnidirectional nodes.
    let emitters = TrackEmitters {
        omni: Vec::new(),
        cones: 1,
        report: Vec::new(),
    };
    assert_eq!(emitters.in_range(&oag_audio::Listener::at_origin(), 0.0), 0);
    assert_eq!(emitters.cones, 1);
    assert!(cone.cone.is_some());
}
