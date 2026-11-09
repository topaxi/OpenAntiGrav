use oag_core::math::Vec3;
use oag_gameplay::cue::{Cue, CueEvent};
use oag_tables::rumble::{Class, FallOff, Kind, Library, Pattern, Priority};

use super::*;

fn flat(kind: Kind, large: u8, frames: usize, loops: (u32, u32)) -> Pattern {
    Pattern {
        kind,
        loop_start: loops.0,
        loop_end: loops.1,
        class: Some(Class::Game),
        priority: Some(Priority::Medium),
        positional: None,
        large: vec![large; frames],
        small: vec![0; frames],
    }
}

fn library() -> Library {
    let mut library = Library::new();
    for name in ["pad_speed", "enter_mag_rumble", "exit_mag_rumble"] {
        library.insert(name, flat(Kind::OneShot, 100, 4, (0, 0)));
    }
    for name in ["autopilot_active_rumble", "travel_mag_rumble"] {
        library.insert(name, flat(Kind::Looping, 80, 10, (2, 6)));
    }
    for name in ["plasma_hits_wall", "missile_rebound_wall_rumble"] {
        let mut p = flat(Kind::OneShot, 200, 6, (0, 0));
        p.positional = Some(FallOff {
            inner: 10.0,
            outer: 20.0,
        });
        library.insert(name, p);
    }
    library
}

fn feed(set: Set, drop_position: bool) -> Feed {
    Feed::new(library(), SmallMotor::Sum, Rules { set, drop_position })
}

const RACE: StepContext = StepContext {
    race: true,
    paused: false,
    listener: [0.0; 3],
};

#[derive(Default)]
struct Recorder(Vec<(usize, Motors)>);

impl MotorOutput for Recorder {
    fn set(&mut self, seat: usize, motors: Motors) {
        self.0.push((seat, motors));
    }
    fn silence(&mut self) {}
}

fn ticks(feed: &mut Feed, ctx: StepContext, n: usize) -> Vec<u8> {
    let mut out = Recorder::default();
    (0..n).map(|_| feed.drive(ctx, 0, &mut out).large).collect()
}

fn none() -> Levels {
    Levels::default()
}

#[test]
fn an_edge_on_the_own_ship_plays_its_pattern() {
    let mut f = feed(Set::Ps3, false);
    f.event(&CueEvent::new(Cue::SpeedupPad, 0), 0);
    assert_eq!(ticks(&mut f, RACE, 6), [100, 100, 100, 100, 0, 0]);
}

#[test]
fn a_rivals_edge_reaches_no_motor() {
    let mut f = feed(Set::Ps4, false);
    f.event(&CueEvent::new(Cue::SpeedupPad, 3), 0);
    assert_eq!(ticks(&mut f, RACE, 6), [0; 6]);
}

#[test]
fn a_cue_the_set_does_not_fire_is_silent() {
    let at = CueEvent::at_point(Cue::MissileHitWall, Vec3::ZERO);
    let mut ps3 = feed(Set::Ps3, false);
    ps3.event(&at, 0);
    assert_eq!(ticks(&mut ps3, RACE, 3), [0; 3]);
    let mut ps4 = feed(Set::Ps4, false);
    ps4.event(&at, 0);
    assert_eq!(ticks(&mut ps4, RACE, 3), [200; 3]);
}

#[test]
fn a_world_event_is_heard_by_distance_and_the_original_drops_the_position() {
    let far = CueEvent::at_point(Cue::PlasmaHitWall, Vec3::new(500.0, 0.0, 0.0));
    let near_ctx = StepContext {
        listener: [500.0, 0.0, 0.0],
        ..RACE
    };
    let mut ours = feed(Set::Ps4, false);
    ours.event(&far, 0);
    assert_eq!(ticks(&mut ours, near_ctx, 1), [200], "at the ship");
    let mut original = feed(Set::Ps4, true);
    original.event(&far, 0);
    assert_eq!(
        ticks(&mut original, near_ctx, 1),
        [0],
        "the origin is 500 away"
    );
}

#[test]
fn a_level_holds_a_voice_until_it_falls_and_then_runs_out() {
    let mut f = feed(Set::Ps3, false);
    let up = Levels {
        autopilot: true,
        ..none()
    };
    let mut seen = Vec::new();
    for _ in 0..30 {
        f.levels(up);
        seen.extend(ticks(&mut f, RACE, 1));
    }
    assert!(seen.iter().all(|&b| b == 80), "still held: {seen:?}");
    f.levels(none());
    let tail = ticks(&mut f, RACE, 12);
    assert_eq!(f.player().active_voices(), 0, "ran to its end: {tail:?}");
    assert_eq!(*tail.last().unwrap(), 0);
}

#[test]
fn a_level_that_never_rises_plays_nothing() {
    let mut f = feed(Set::Ps3, false);
    for _ in 0..5 {
        f.levels(none());
    }
    assert_eq!(ticks(&mut f, RACE, 5), [0; 5]);
}

#[test]
fn the_mag_strip_plays_enter_travel_and_exit() {
    let mut f = feed(Set::Ps3, false);
    let over = Levels {
        mag_strip: true,
        ..none()
    };
    f.levels(over);
    assert_eq!(f.player().active_voices(), 2, "enter and travel");
    f.levels(none());
    assert_eq!(
        f.player().active_voices(),
        3,
        "exit joins, travel finishing"
    );
}

#[test]
fn pausing_silences_the_motors_and_the_pattern_runs_on() {
    let mut f = feed(Set::Ps3, false);
    f.event(&CueEvent::new(Cue::SpeedupPad, 0), 0);
    let paused = StepContext {
        paused: true,
        ..RACE
    };
    assert_eq!(ticks(&mut f, paused, 5), [0; 5]);
    assert_eq!(f.player().active_voices(), 0, "elapsed while paused");

    let mut held = feed(Set::Ps3, false);
    let up = Levels {
        autopilot: true,
        ..none()
    };
    held.levels(up);
    assert_eq!(ticks(&mut held, paused, 3), [0; 3]);
    held.levels(up);
    assert_eq!(ticks(&mut held, RACE, 1), [80], "resumes");
}

#[test]
fn clear_ends_every_voice() {
    let mut f = feed(Set::Ps3, false);
    f.levels(Levels {
        autopilot: true,
        ..none()
    });
    f.clear();
    assert_eq!(ticks(&mut f, RACE, 2), [0, 0]);
}
