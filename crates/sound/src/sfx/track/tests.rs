//! What [`super::TrackEmitters`] is asserted to do with no disc anywhere (the
//! disc-backed half is `crates/game/tests/track_audio_ground_truth.rs`).

use super::*;
use oag_vex::sound_emitters::RadiusCurve;

/// An emitter at `position` with a constant radius, no cone and no audio:
/// `sound: None` throughout, since these assert placement and a `Loaded` needs a
/// real bank (the resolved half is the disc-backed test).
fn omni(position: [f32; 3], radius: f32) -> Authored {
    Authored {
        emitter: node(position, radius),
        sound: None,
    }
}

/// A directional emitter, facing `+Z`, with the given half-angle in degrees.
/// `placed_emitter` reads the axis off row `1` of the world matrix
/// (`to_world[4..7]`), where `node`'s identity puts `+Y`, so this overwrites it.
fn cone(position: [f32; 3], radius: f32, half_angle_degrees: f32) -> Authored {
    let mut emitter = node(position, radius);
    emitter.to_world[4] = 0.0;
    emitter.to_world[5] = 0.0;
    emitter.to_world[6] = 1.0;
    emitter.cone = Some(oag_vex::sound_emitters::Cone {
        angle_a: half_angle_degrees.to_radians(),
        angle_b: 40.0_f32.to_radians(),
    });
    Authored {
        emitter,
        sound: None,
    }
}

/// The decoded node an [`Authored`] wraps, facing `+Z`.
fn node(position: [f32; 3], radius: f32) -> SoundEmitter {
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
    assert!(parsed.directional.is_empty());
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
        directional: Vec::new(),
        report: Vec::new(),
    };
    let listener = oag_audio::Listener::at_origin();
    assert_eq!(emitters.in_range(&listener), 3);
    let placed: Vec<_> = emitters.placed(&listener).collect();
    assert_eq!(
        placed.iter().map(|(at, _)| *at).collect::<Vec<_>>(),
        [0, 1, 3]
    );
}

#[test]
fn the_near_field_is_flat_and_the_far_field_falls_off() {
    let emitters = TrackEmitters {
        omni: vec![omni([10.0, 0.0, 0.0], 100.0), omni([90.0, 0.0, 0.0], 100.0)],
        directional: Vec::new(),
        report: Vec::new(),
    };
    let listener = oag_audio::Listener::at_origin();
    let placed: Vec<_> = emitters.placed(&listener).map(|(_, p)| p).collect();
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
fn a_cone_is_placed_after_the_omni_nodes_and_only_inside_its_angle() {
    // `cone`'s axis is row 1 of its world matrix, `+Z`; per `oag_audio::spatial`
    // a listener at the cone's `-Z` is the on-axis side (as the original's
    // `e[0x10]`/`-row1` dot product).
    let listener = oag_audio::Listener {
        position: [0.0, 0.0, -10.0],
        right: [1.0, 0.0, 0.0],
    };
    let emitters = TrackEmitters {
        omni: vec![omni([0.0, 0.0, 0.0], 5.0)],
        directional: vec![cone([0.0, 0.0, 0.0], 100.0, 20.0)],
        report: Vec::new(),
    };
    // Index 0 is the omni node (out of its 5-unit radius at distance 10), index 1
    // the cone, placed because the listener is on its axis.
    let placed: Vec<_> = emitters.placed(&listener).collect();
    assert_eq!(
        placed.iter().map(|(at, _)| *at).collect::<Vec<_>>(),
        [1],
        "the omni node is past its own radius; the cone is on-axis and in range"
    );
}

#[test]
fn a_cone_outside_its_half_angle_is_silent_rather_than_dim() {
    // The listener is on `+X`, 90 degrees off the cone's `+Z` axis, past a
    // 20-degree half-angle but inside the radius: `place` returns a zero-gain
    // `Placed`, not a refusal (only the `d > radius` gate says "not in range",
    // per `oag_audio::spatial`).
    let listener = oag_audio::Listener {
        position: [10.0, 0.0, 0.0],
        right: [0.0, 0.0, 1.0],
    };
    let emitters = TrackEmitters {
        omni: Vec::new(),
        directional: vec![cone([0.0, 0.0, 0.0], 100.0, 20.0)],
        report: Vec::new(),
    };
    let placed: Vec<_> = emitters.placed(&listener).collect();
    assert_eq!(placed.len(), 1, "past the radius entirely, not off-angle");
    assert_eq!(placed[0].1.gain, 0.0, "90 degrees off a 20-degree cone");
}

/// One decoded waveform, `frames` long, with the loop flag the caller wants.
fn cue(frames: usize, looping: bool) -> Loaded {
    let sound = oag_audio::Sound::new(vec![8000; frames], 1, 8_000).expect("a sound");
    Loaded {
        waveforms: vec![(std::sync::Arc::new(sound), looping)],
    }
}

/// A one-shot alternate is played once and not restarted while in range.
///
/// The bug is silent to every other check: a cue re-opened every tick still
/// counts as one voice, renders samples and stops out of range, but sounds like
/// a sample's first milliseconds sixty times a second. Not hypothetical:
/// `platinu~BIRDS` binds sixteen waveforms of which only some loop, and five
/// cues on the Pulse disc are mixed so, so the draw in [`super::pick`] reaches a
/// non-looping one.
#[test]
fn a_finished_one_shot_is_not_restarted_while_its_emitter_is_in_range() {
    let mut emitters = TrackEmitters {
        omni: vec![omni([0.0, 0.0, 0.0], 100.0)],
        directional: Vec::new(),
        report: Vec::new(),
    };
    emitters.omni[0].sound = Some(cue(64, false));

    let mut mixer = oag_audio::Mixer::new(8_000);
    let mut ambience = Ambience::default();
    let listener = oag_audio::Listener::at_origin();
    let mut rng = oag_core::Rng::new(1);
    let mut out = vec![0.0; 2 * 256];

    ambience.tick(&mut mixer, &emitters, &listener, &mut rng, true, 1.0 / 60.0);
    assert_eq!(mixer.active_voices(), 1, "the cue never opened a voice");

    // Long enough to run the 64-frame sample out several times over.
    for _ in 1..10 {
        mixer.render(&mut out);
        ambience.tick(&mut mixer, &emitters, &listener, &mut rng, true, 1.0 / 60.0);
    }
    assert_eq!(
        mixer.active_voices(),
        0,
        "a one-shot is being re-opened every tick while its emitter is in range"
    );
    assert_eq!(
        ambience.playing(&mixer),
        0,
        "the count follows the slot rather than the mixer"
    );
}

/// A looping cue keeps the voice it opened, and the latch closes and reopens it.
#[test]
fn a_looping_cue_keeps_one_voice_for_as_long_as_it_is_in_range() {
    let mut emitters = TrackEmitters {
        omni: vec![omni([0.0, 0.0, 0.0], 100.0)],
        directional: Vec::new(),
        report: Vec::new(),
    };
    emitters.omni[0].sound = Some(cue(64, true));

    let mut mixer = oag_audio::Mixer::new(8_000);
    let mut ambience = Ambience::default();
    let listener = oag_audio::Listener::at_origin();
    let mut rng = oag_core::Rng::new(1);
    let mut out = vec![0.0; 2 * 256];

    for _ in 0..10 {
        ambience.tick(&mut mixer, &emitters, &listener, &mut rng, true, 1.0 / 60.0);
        mixer.render(&mut out);
    }
    assert_eq!(mixer.active_voices(), 1, "the loop stopped or was doubled");
    assert_eq!(ambience.playing(&mixer), 1);

    // Out of range: stopped, and the slot cleared so a return can re-open it.
    let far = oag_audio::Listener {
        position: [1000.0, 0.0, 0.0],
        right: [1.0, 0.0, 0.0],
    };
    ambience.tick(&mut mixer, &emitters, &far, &mut rng, true, 1.0 / 60.0);
    assert_eq!(ambience.playing(&mixer), 0, "the latch did not close it");
    ambience.tick(&mut mixer, &emitters, &listener, &mut rng, true, 1.0 / 60.0);
    assert_eq!(ambience.playing(&mixer), 1, "coming back opened nothing");
}
