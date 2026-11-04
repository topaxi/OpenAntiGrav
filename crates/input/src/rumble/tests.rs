use oag_tables::rumble::{Class, FallOff, Kind, Library, Pattern, Priority};

use super::*;

fn pattern(large: &[u8], small: &[u8]) -> Pattern {
    Pattern {
        kind: Kind::OneShot,
        loop_start: 0,
        loop_end: 0,
        class: Some(Class::Game),
        priority: Some(Priority::Medium),
        positional: None,
        large: large.to_vec(),
        small: small.to_vec(),
    }
}

fn looping(large: &[u8], start: u32, end: u32) -> Pattern {
    Pattern {
        kind: Kind::Looping,
        loop_start: start,
        loop_end: end,
        ..pattern(large, &[])
    }
}

fn player(patterns: Vec<Pattern>, law: SmallMotor) -> Player {
    let mut library = Library::new();
    for (i, p) in patterns.into_iter().enumerate() {
        library.insert(&format!("p{i}"), p);
    }
    Player::new(library, law)
}

const RACE: StepContext = StepContext {
    race: true,
    paused: false,
    listener: [0.0; 3],
};

#[derive(Default)]
struct Recorder {
    sets: Vec<(usize, Motors)>,
    silenced: u32,
}

impl MotorOutput for Recorder {
    fn set(&mut self, seat: usize, motors: Motors) {
        self.sets.push((seat, motors));
    }
    fn silence(&mut self) {
        self.silenced += 1;
    }
}

fn run(player: &mut Player, ctx: StepContext, ticks: usize) -> Vec<Motors> {
    let mut out = Recorder::default();
    for _ in 0..ticks {
        player.drive(ctx, 0, &mut out);
    }
    out.sets.into_iter().map(|(_, m)| m).collect()
}

fn large(levels: &[Motors]) -> Vec<u8> {
    levels.iter().map(|m| m.large).collect()
}

#[test]
fn the_ring_holds_twenty_voices_and_drops_the_twenty_first() {
    let mut p = player(vec![pattern(&[1; 10], &[])], SmallMotor::OnOff);
    assert_eq!(VOICES, 20, "docs/gameplay/rumble.md: a ring of 20");
    for i in 0..20 {
        assert!(p.one_shot(0, None), "voice {i}");
    }
    assert!(!p.one_shot(0, None), "all 20 busy: dropped");
    assert_eq!(p.active_voices(), 20);
    run(&mut p, RACE, 11);
    assert_eq!(p.active_voices(), 0);
    assert!(p.one_shot(0, None), "a free voice takes a request again");
}

#[test]
fn up_to_five_voices_all_sum_and_the_large_motor_clamps_at_255() {
    let mut p = player(vec![pattern(&[40, 40], &[])], SmallMotor::OnOff);
    for _ in 0..5 {
        p.one_shot(0, None);
    }
    assert_eq!(large(&run(&mut p, RACE, 2)), [200, 200]);
    let mut p = player(vec![pattern(&[100], &[])], SmallMotor::OnOff);
    for _ in 0..5 {
        p.one_shot(0, None);
    }
    assert_eq!(large(&run(&mut p, RACE, 1)), [255]);
}

#[test]
fn from_six_voices_the_walk_overwrites_the_lowest_priority_of_the_five() {
    let mut low = pattern(&[10], &[]);
    low.priority = Some(Priority::Low);
    let mut high = pattern(&[50], &[]);
    high.priority = Some(Priority::High);
    // Ring order: four high (50 each), one low (10), then a sixth, high.
    let mut p = player(vec![high.clone(), low, high], SmallMotor::OnOff);
    for _ in 0..4 {
        p.one_shot(0, None);
    }
    p.one_shot(1, None);
    p.one_shot(2, None);
    // The sixth overwrites the low one: 5 x 50.
    assert_eq!(large(&run(&mut p, RACE, 1)), [250]);
}

#[test]
fn a_sixth_voice_displaces_a_higher_one_when_it_is_itself_the_lowest() {
    let mut high = pattern(&[50], &[]);
    high.priority = Some(Priority::High);
    let mut low = pattern(&[7], &[]);
    low.priority = Some(Priority::Low);
    // Five highs fill the slots; the low sixth still overwrites the first of them.
    let mut p = player(vec![high, low], SmallMotor::OnOff);
    for _ in 0..5 {
        p.one_shot(0, None);
    }
    p.one_shot(1, None);
    assert_eq!(large(&run(&mut p, RACE, 1)), [4 * 50 + 7]);
}

#[test]
fn a_game_pattern_is_silent_outside_a_race_and_a_ui_pattern_is_not() {
    let mut ui = pattern(&[9, 9], &[]);
    ui.class = Some(Class::Ui);
    let mut p = player(vec![pattern(&[5, 5], &[]), ui], SmallMotor::OnOff);
    let menu = StepContext {
        race: false,
        ..RACE
    };
    p.one_shot(0, None);
    p.one_shot(1, None);
    assert_eq!(large(&run(&mut p, menu, 2)), [9, 9]);
}

#[test]
fn a_pause_silences_everything_but_ui() {
    let mut ui = pattern(&[9, 9, 9], &[]);
    ui.class = Some(Class::Ui);
    let mut p = player(vec![pattern(&[5, 5, 5], &[]), ui], SmallMotor::OnOff);
    p.one_shot(0, None);
    p.one_shot(1, None);
    let paused = StepContext {
        paused: true,
        ..RACE
    };
    assert_eq!(large(&run(&mut p, paused, 1)), [9]);
    assert_eq!(
        large(&run(&mut p, RACE, 1)),
        [14],
        "resume: the frame moved on"
    );
}

#[test]
fn a_looping_pattern_queued_as_a_one_shot_plays_through_once() {
    let mut p = player(vec![looping(&[1, 2, 3, 4], 1, 3)], SmallMotor::OnOff);
    p.one_shot(0, None);
    assert_eq!(large(&run(&mut p, RACE, 6)), [1, 2, 3, 4, 0, 0]);
}

#[test]
fn a_held_looping_pattern_wraps_until_finished() {
    let mut p = player(vec![looping(&[1, 2, 3, 4], 1, 3)], SmallMotor::OnOff);
    let handle = p.hold(0).expect("a free voice");
    assert_eq!(large(&run(&mut p, RACE, 7)), [1, 2, 3, 2, 3, 2, 3]);
    p.finish(handle);
    assert_eq!(
        large(&run(&mut p, RACE, 4)),
        [4, 0, 0, 0],
        "runs on to the end of the track"
    );
    assert_eq!(p.active_voices(), 0);
}

#[test]
fn a_held_one_shot_does_not_stick_on_frame_zero() {
    let mut p = player(vec![pattern(&[8, 6], &[])], SmallMotor::OnOff);
    p.hold(0);
    assert_eq!(large(&run(&mut p, RACE, 3)), [8, 6, 0]);
    assert_eq!(p.active_voices(), 0);
}

#[test]
fn looping_follows_the_release_flag_and_the_range_not_the_type() {
    let mut odd = pattern(&[1, 2, 3, 4], &[]);
    odd.loop_start = 1;
    odd.loop_end = 3;
    assert_eq!(odd.kind, Kind::OneShot);
    let mut p = player(vec![odd], SmallMotor::OnOff);
    p.hold(0);
    assert_eq!(large(&run(&mut p, RACE, 6)), [1, 2, 3, 2, 3, 2]);
}

#[test]
fn stop_is_silent_at_once_and_a_stale_handle_cannot_stop_a_reused_voice() {
    let mut p = player(vec![looping(&[9, 9], 0, 2)], SmallMotor::OnOff);
    let handle = p.hold(0).unwrap();
    assert_eq!(large(&run(&mut p, RACE, 1)), [9]);
    p.stop(handle);
    assert_eq!(large(&run(&mut p, RACE, 1)), [0]);
    // The slot is reused by a new voice; the old handle must not touch it.
    let mut reused = None;
    for _ in 0..VOICES {
        let h = p.hold(0).unwrap();
        if h.slot == handle.slot {
            reused = Some(h);
            break;
        }
    }
    assert!(reused.is_some());
    p.stop(handle);
    assert!(
        p.voices[usize::from(handle.slot)].active,
        "the stale handle did nothing"
    );
}

#[test]
fn a_finished_voice_leaves_silence() {
    let mut p = player(vec![pattern(&[30, 20], &[1, 1])], SmallMotor::OnOff);
    p.one_shot(0, None);
    let levels = run(&mut p, RACE, 4);
    assert_eq!(levels[2], Motors::OFF);
    assert_eq!(levels[3], Motors::OFF);
    assert_eq!(p.active_voices(), 0);
}

#[test]
fn unequal_tracks_run_to_the_longer_and_read_zero_past_the_shorter() {
    let mut p = player(vec![pattern(&[10, 10], &[1, 1, 1, 1])], SmallMotor::OnOff);
    p.one_shot(0, None);
    let levels = run(&mut p, RACE, 5);
    assert_eq!(large(&levels), [10, 10, 0, 0, 0]);
    let small: Vec<u8> = levels.iter().map(|m| m.small).collect();
    assert_eq!(
        small,
        [255, 255, 255, 255, 0],
        "the short track does not end the voice"
    );
}

#[test]
fn ps3_small_motor_is_full_on_for_any_non_zero_voice() {
    let mut p = player(vec![pattern(&[0], &[1])], SmallMotor::OnOff);
    for _ in 0..3 {
        p.one_shot(0, None);
    }
    assert_eq!(
        p.step(RACE).small,
        255,
        "docs/gameplay/rumble.md: HD, any non-zero is full on"
    );
}

#[test]
fn ps4_small_motor_is_the_byte_sum_of_the_mixed_voices() {
    let mut p = player(vec![pattern(&[0], &[1])], SmallMotor::Sum);
    for _ in 0..3 {
        p.one_shot(0, None);
    }
    assert_eq!(
        p.step(RACE).small,
        3,
        "docs/gameplay/rumble.md: Omega, 1..5 of 255"
    );
    let mut p = player(vec![pattern(&[0], &[1])], SmallMotor::Sum);
    for _ in 0..7 {
        p.one_shot(0, None);
    }
    assert_eq!(p.step(RACE).small, 5, "only five are mixed");
}

#[test]
fn positional_gain_falls_from_one_to_half_and_cuts_to_zero() {
    let mut shot = pattern(&[200], &[]);
    shot.positional = Some(FallOff {
        inner: 10.0,
        outer: 20.0,
    });
    let mut p = player(vec![shot], SmallMotor::OnOff);
    let at = |x: f32, p: &mut Player| {
        p.one_shot(0, Some([x, 0.0, 0.0]));
        p.step(RACE).large
    };
    assert_eq!(at(10.0, &mut p), 200, "d = inner: 1");
    assert_eq!(at(15.0, &mut p), 150, "(10 - 15) / 20 + 1 = 0.75");
    assert_eq!(at(20.0, &mut p), 0, "d = outer: cut");
    p.one_shot(0, None);
    let far = StepContext {
        listener: [100.0, 0.0, 0.0],
        ..RACE
    };
    assert_eq!(
        p.step(far).large,
        0,
        "no position: measured from the origin"
    );
}

#[test]
fn an_unknown_id_plays_nothing() {
    let mut p = player(vec![pattern(&[1], &[])], SmallMotor::OnOff);
    assert!(!p.one_shot(5, None));
    assert!(p.hold(5).is_none());
}

#[test]
fn drive_hands_each_step_to_the_seat_it_was_given() {
    let mut p = player(vec![pattern(&[3, 4], &[])], SmallMotor::OnOff);
    p.one_shot(0, None);
    let mut out = Recorder::default();
    p.drive(RACE, 2, &mut out);
    p.drive(RACE, 2, &mut out);
    assert_eq!(
        out.sets,
        [
            (2, Motors { large: 3, small: 0 }),
            (2, Motors { large: 4, small: 0 })
        ]
    );
}

#[test]
fn a_machine_with_no_pad_has_no_motors_and_does_not_fail() {
    let mut pad = crate::Pad::none();
    pad.set(
        0,
        Motors {
            large: 255,
            small: 255,
        },
    );
    pad.silence();
    assert!(pad.motor_pads().is_empty());
    assert_eq!(pad.live_effects(), 0);
}

#[test]
fn the_setting_picks_the_law_and_original_follows_the_title() {
    let hd = Some(SmallMotor::OnOff);
    let omega = Some(SmallMotor::Sum);
    assert_eq!(Mode::None.law(hd), None);
    assert_eq!(Mode::Ps3.law(None), Some(SmallMotor::OnOff));
    assert_eq!(Mode::Ps4.law(None), Some(SmallMotor::Sum));
    assert_eq!(Mode::Original.law(hd), Some(SmallMotor::OnOff));
    assert_eq!(Mode::Original.law(omega), Some(SmallMotor::Sum));
    assert_eq!(Mode::Original.law(None), None, "no patterns, no motors");
    assert_eq!(Mode::default(), Mode::Ps4);
    for mode in Mode::ALL {
        assert_eq!(Mode::from_name(mode.name()), Some(mode));
    }
}
