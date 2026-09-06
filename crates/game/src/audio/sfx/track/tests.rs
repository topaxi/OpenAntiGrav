//! What [`super::TrackEmitters`] is asserted to do with no disc anywhere.
//!
//! The disc-backed half is `crates/game/tests/track_audio_ground_truth.rs`.

use super::*;
use oag_formats::sound_emitters::RadiusCurve;

/// An emitter at `position` with a constant radius, no cone and no audio.
///
/// `sound: None` throughout: what these assert is placement, and a `Loaded`
/// needs a real bank behind it. The resolved half is the disc-backed test.
fn omni(position: [f32; 3], radius: f32) -> Authored {
    Authored {
        emitter: node(position, radius),
        sound: None,
    }
}

/// The decoded node an [`Authored`] wraps.
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
    let mut cone = node([0.0, 0.0, 0.0], 100.0);
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

/// One decoded waveform, `frames` long, with the loop flag the caller wants.
fn cue(frames: usize, looping: bool) -> Loaded {
    let sound = oag_audio::Sound::new(vec![8000; frames], 1, 8_000).expect("a sound");
    Loaded {
        waveforms: vec![(std::sync::Arc::new(sound), looping)],
    }
}

/// A one-shot alternate is played once and **not** restarted while in range.
///
/// The bug this exists for is silent to every other check here: a cue re-opened
/// every tick still counts as one voice, still renders samples and still stops
/// when the emitter goes out of range. It just sounds like the first few
/// milliseconds of a sample sixty times a second. Not hypothetical -
/// `platinu~BIRDS` binds sixteen waveforms and only some of them loop, and five
/// cues across the Pulse disc are mixed like that, so the uniform draw in
/// [`super::pick`] does reach a non-looping one.
#[test]
fn a_finished_one_shot_is_not_restarted_while_its_emitter_is_in_range() {
    let mut emitters = TrackEmitters {
        omni: vec![omni([0.0, 0.0, 0.0], 100.0)],
        cones: 0,
        report: Vec::new(),
    };
    emitters.omni[0].sound = Some(cue(64, false));

    let mut mixer = oag_audio::Mixer::new(8_000);
    let mut ambience = Ambience::default();
    let listener = oag_audio::Listener::at_origin();
    let mut rng = oag_core::Rng::new(1);
    let mut out = vec![0.0; 2 * 256];

    ambience.tick(&mut mixer, &emitters, &listener, 0.0, &mut rng);
    assert_eq!(mixer.active_voices(), 1, "the cue never opened a voice");

    // Long enough to run the 64-frame sample out several times over.
    for tick in 1..10 {
        mixer.render(&mut out);
        #[expect(clippy::cast_precision_loss, reason = "a tick count")]
        let frame = tick as f32;
        ambience.tick(&mut mixer, &emitters, &listener, frame, &mut rng);
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
        cones: 0,
        report: Vec::new(),
    };
    emitters.omni[0].sound = Some(cue(64, true));

    let mut mixer = oag_audio::Mixer::new(8_000);
    let mut ambience = Ambience::default();
    let listener = oag_audio::Listener::at_origin();
    let mut rng = oag_core::Rng::new(1);
    let mut out = vec![0.0; 2 * 256];

    for tick in 0..10 {
        #[expect(clippy::cast_precision_loss, reason = "a tick count")]
        let frame = tick as f32;
        ambience.tick(&mut mixer, &emitters, &listener, frame, &mut rng);
        mixer.render(&mut out);
    }
    assert_eq!(mixer.active_voices(), 1, "the loop stopped or was doubled");
    assert_eq!(ambience.playing(&mixer), 1);

    // Out of range: stopped, and the slot cleared so a return can re-open it.
    let far = oag_audio::Listener {
        position: [1000.0, 0.0, 0.0],
        right: [1.0, 0.0, 0.0],
    };
    ambience.tick(&mut mixer, &emitters, &far, 10.0, &mut rng);
    assert_eq!(ambience.playing(&mixer), 0, "the latch did not close it");
    ambience.tick(&mut mixer, &emitters, &listener, 11.0, &mut rng);
    assert_eq!(ambience.playing(&mixer), 1, "coming back opened nothing");
}
