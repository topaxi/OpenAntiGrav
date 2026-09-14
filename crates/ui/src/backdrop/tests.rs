use super::*;
use oag_tables::envsettings::EnvSettings;

fn settings() -> FuryBackdrop {
    let mut text = String::from("\"Particle Colour\"=0.5 0.1 0.0\n\"Feedback\"=0.6 0.6 0.6\n");
    for i in 0..8 {
        text.push_str(&format!(
            "\"staticPaths[{i}].pointSize\"=0.005\n\"staticPaths[{i}].fovy\"=40.0\n\"staticPaths[{i}].duration\"={}\n\"staticPaths[{i}].dofStart\"=3.0\n\"staticPaths[{i}].dofStrength\"=0.3\n\"staticPaths[{i}].dofFactor\"=1.0\n\"staticPaths[{i}].fogStart\"=3.0\n\"staticPaths[{i}].fogLength\"=4.0\n\"staticPaths[{i}].fogExponent\"=0.3\n\"staticPaths[{i}].start\"=3.0 2.0 0.0\n\"staticPaths[{i}].end\"=5.0 0.5 -5.0\n\"staticPaths[{i}].focusStart\"=0.0 0.0 0.0\n\"staticPaths[{i}].focusEnd\"=0.0 0.5 -3.0\n",
            1.0 + i as f32
        ));
    }
    FuryBackdrop::read(&EnvSettings::parse(&text).expect("parses"))
}

#[test]
fn a_clip_runs_its_duration_in_frames_then_the_next_path_is_picked() {
    let mut fury = Fury::new(settings(), 3, 1).expect("authored");
    let first = fury.path();
    let frames = (fury.settings().static_paths[first].duration * FRAMES_PER_SECOND) as u32;
    for _ in 0..frames - 1 {
        fury.tick();
    }
    assert_eq!(
        fury.path(),
        first,
        "still on the first path on its last frame"
    );
    assert!((fury.seconds() - (frames - 1) as f32 / 60.0).abs() < 1e-5);
    fury.tick();
    assert_ne!(fury.path(), first, "the next pick never repeats the last");
    assert_eq!(fury.seconds(), 0.0);
    assert_eq!(fury.cloud(), 3, "a static path never asks for a new cloud");
}

#[test]
fn the_picker_never_repeats_within_the_queue_and_never_stalls() {
    let mut fury = Fury::new(settings(), 0, 7).expect("authored");
    let mut history = vec![fury.path()];
    while history.len() < 60 {
        fury.tick();
        if fury.seconds() == 0.0 {
            history.push(fury.path());
        }
    }
    // Seven-deep queue over eight paths: any eight consecutive picks are
    // distinct.
    for window in history.windows(8) {
        let mut sorted = window.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), 8, "a repeat inside the queue: {window:?}");
    }
    assert!(
        fury.rerolled() > 0,
        "two of three rolls are kinds this build re-rolls"
    );
}

#[test]
fn a_file_with_no_static_path_is_no_backdrop() {
    let settings =
        FuryBackdrop::read(&EnvSettings::parse("\"Feedback\"=0.6 0.6 0.6\n").expect("parses"));
    assert!(Fury::new(settings, 0, 1).is_none());
}

#[test]
fn the_frame_carries_the_pages_constants_at_1080_lines() {
    let fury = Fury::new(settings(), 0, 1).expect("authored");
    let frame = fury.frame(1080.0, 16.0 / 9.0, [1.0; 4]);
    assert!(
        (frame.sprite_size - 0.005).abs() < 1e-4,
        "pointSize at 1080 lines"
    );
    let expect = |a: [f32; 3], b: [f32; 3]| a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-3);
    // `0.7` at rest, the bands at zero: `1 + (0 - 0.6) * 1.2`.
    let pulse = 0.7 * 0.28;
    assert!(expect(
        frame.particle_colour,
        [0.5 * pulse, 0.1 * pulse, 0.0]
    ));
    assert_eq!(
        frame.colour_ramp_factors,
        [20.0, 0.08, 0.5, 0.0],
        "at t = 0"
    );
    assert_eq!(frame.colour_ramp_factors2, [4.0, -3.0, 6.0, 0.0]);
    assert_eq!(frame.depth_fade_factors, [0.5, -0.5]);
    assert!(expect(
        [
            frame.dof_factors[0],
            frame.dof_factors[1],
            frame.dof_factors[2]
        ],
        [-3.0, -1.0 / 0.3, 24.0]
    ));
    assert!(expect(
        [
            frame.fog_factors[0],
            frame.fog_factors[1],
            frame.fog_factors[2]
        ],
        [-3.0, -0.25, -7.0]
    ));
    assert!((frame.fog_factors[3] - 0.3).abs() < 1e-6);
    // `0.016 * 24 * 0.6` is over the `0.15` ceiling, so the cap is the ceiling.
    assert_eq!(frame.source_max, [0.15; 3]);
    assert_eq!(frame.feedback, [0.6; 3]);
}

#[test]
fn the_camera_looks_from_the_paths_start_at_its_focus() {
    let fury = Fury::new(settings(), 0, 1).expect("authored");
    let frame = fury.frame(1080.0, 16.0 / 9.0, [1.0; 4]);
    // The eye is (3, 2, 0) at t = 0, so transforming it lands on the origin.
    let m = frame.world_view;
    let apply = |p: [f32; 3]| {
        std::array::from_fn::<f32, 3, _>(|r| {
            m[0][r] * p[0] + m[1][r] * p[1] + m[2][r] * p[2] + m[3][r]
        })
    };
    let at_eye = apply([3.0, 2.0, 0.0]);
    assert!(at_eye.iter().all(|c| c.abs() < 1e-5), "{at_eye:?}");
    // The focus (0, 0, 0) is straight ahead: on the -z axis.
    let at_focus = apply([0.0, 0.0, 0.0]);
    assert!(
        at_focus[0].abs() < 1e-5 && at_focus[1].abs() < 1e-5,
        "{at_focus:?}"
    );
    assert!(
        (at_focus[2] + (9.0f32 + 4.0).sqrt()).abs() < 1e-4,
        "{at_focus:?}"
    );
    // The projection keeps a point at -z in front: positive w.
    let p = frame.proj;
    let w = p[2][3] * at_focus[2] + p[3][3];
    assert!(w > 0.0);
}

#[test]
fn tints_read_the_widgets_screen_settings_and_fall_back_to_default() {
    let xml = r#"<Root><Screen><BackgroundAnimFury name="bgAnimFury">
        <ScreenSetting name="default" tint="0xFF202020" equaliser="0"/>
        <ScreenSetting name="Main Menu" tint="0xFFFFFFFF" equaliser="0"/>
    </BackgroundAnimFury></Screen></Root>"#;
    let tints = Tints::read(&oag_tables::fexml::parse(xml)).expect("authored");
    assert_eq!(tints.len(), 2);
    assert_eq!(tints.for_screen("Main Menu"), [1.0; 4]);
    let grey = 0x20 as f32 / 255.0;
    assert_eq!(tints.for_screen("Race Records"), [grey, grey, grey, 1.0]);
    assert!(Tints::read(&oag_tables::fexml::parse("<Root><BackgroundAnim/></Root>")).is_none());
}

#[test]
fn force_path_plays_that_path_from_its_start_and_refuses_an_unauthored_one() {
    let mut fury = Fury::new(settings(), 3, 1).expect("authored");
    for _ in 0..30 {
        fury.tick();
    }
    assert!(fury.force_path(5).is_some());
    assert_eq!(fury.path(), 5);
    assert_eq!(fury.seconds(), 0.0);
    // Path 5 authors six seconds: it runs them, then the picker takes over.
    for _ in 0..(6 * FRAMES_PER_SECOND as u32) - 1 {
        fury.tick();
    }
    assert_eq!(fury.path(), 5);
    fury.tick();
    assert_ne!(fury.path(), 5);
    let before = fury.path();
    assert!(fury.force_path(8).is_none(), "no ninth static path");
    assert_eq!(fury.path(), before, "a refused force leaves the clip alone");
}

#[test]
fn the_music_pulse_reads_silence_as_bands_at_zero() {
    let fury = Fury::new(settings(), 3, 1).expect("authored");
    let frame = fury.frame(1080.0, 16.0 / 9.0, [1.0; 4]);
    // No `Music Pulse Base` authored: the constructor's `0.6` and `1.2`, so
    // `1 + (0 - 0.6) * 1.2 = 0.28`, times the `0.7` brightness at rest.
    let expected = 0.5 * BRIGHTNESS_AT_REST * (1.0 + (0.0 - 0.6) * 1.2);
    assert!(
        (frame.particle_colour[0] - expected).abs() < 1e-6,
        "{}",
        frame.particle_colour[0]
    );
}

/// `RadioHead2_vp`'s ramp off a frame's constants, as `backdrop.wgsl`
/// computes it: `sat(crf2.x * (1 - frac t) * sat(0.4 d) + crf2.y) ^ crf2.z`.
fn ramp(frame: &Frame, d: f32, one_minus_frac: f32) -> f32 {
    let crf2 = frame.colour_ramp_factors2;
    let distance = (0.4 * d).clamp(0.0, 1.0);
    (crf2[0] * one_minus_frac * distance + crf2[1])
        .clamp(0.0, 1.0)
        .powf(crf2[2])
}

#[test]
fn the_ramp_is_a_band_bounded_at_one_never_the_sixth_power_of_a_distance() {
    let fury = Fury::new(settings(), 3, 1).expect("authored");
    let frame = fury.frame(720.0, 16.0 / 9.0, [1.0; 4]);
    let mut peak: f32 = 0.0;
    for d_tenths in 0..400 {
        let d = d_tenths as f32 / 10.0;
        for phase in 0..=100 {
            let one_minus_frac = phase as f32 / 100.0;
            let value = ramp(&frame, d, one_minus_frac);
            assert!(
                (0.0..=1.0).contains(&value),
                "d {d} phase {one_minus_frac}: {value}"
            );
            peak = peak.max(value);
            // Past two and a half units the distance term is one, and the
            // band is the quarter of the period where `1 - frac t > 0.75`.
            if d >= 2.5 && one_minus_frac < 0.75 {
                assert_eq!(value, 0.0, "d {d} phase {one_minus_frac}");
            }
        }
    }
    assert_eq!(peak, 1.0);
}
