//! Photo mode's rewind and accumulated shot on a fixture race: the live race comes back to the
//! bit, a seek lands on the recorded run, a ship blur holds the camera while a track blur
//! follows it, and a lens sample keeps the focus plane sharp. The same claims through the game,
//! on a disc, are `oag-game`'s `photo_mode_ground_truth`.

use super::reconcile_predict::fixture;
use super::*;
use crate::photo::LensShift;
use crate::photo::effects::{Blur, Row};
use crate::photo::rewind::Rewind;
use crate::photo::shot::{Mover, Plan, Shot};
use crate::timeline::Keeper;
use oag_gameplay::PlayerInputs;
use oag_replay::Header;
use oag_title::photo::Accumulation;

const LIVE: u64 = 300;

fn inputs(tick: u64) -> PlayerInputs {
    PlayerInputs::single(
        InputSnapshot {
            stick_x: (tick as f32 * 0.07).sin() * 0.8,
            ..InputSnapshot::default()
        }
        .sanitised(),
    )
}

/// A recorded fixture race driven to `LIVE`, with its keeper fed as the session feeds it, and
/// the text of its simulation at every tick.
fn driven() -> (Box<Race>, Keeper, Vec<String>) {
    let (race, keeper, text, _) = driven_with_hashes();
    (race, keeper, text)
}

fn driven_with_hashes() -> (Box<Race>, Keeper, Vec<String>, Vec<u64>) {
    // Boxed: a `Race` is large, and a test thread's stack holds the few that a debug build copies.
    let mut race = Box::new(Race::start(fixture()));
    race.start_recording(Header::new("fixture", "t", "m", "c", "d", 0));
    let mut keeper = Keeper::new(fixture(), &race);
    let mut text = vec![format!("{:?}", race.sim)];
    let mut hashes = vec![race.sim.state_hash()];
    for tick in 0..LIVE {
        race.tick(&inputs(tick));
        race.drain_cues();
        keeper.keep(&race);
        text.push(format!("{:?}", race.sim));
        hashes.push(race.sim.state_hash());
    }
    (race, keeper, text, hashes)
}

fn open(race: &mut Race) {
    assert!(race.photo_begin(false, false, None));
    race.photo_offer(Some(Accumulation::HD), true);
}

/// Fails if a seek put the wrong state in, if the live state was not put back, or if the
/// recording took in the ticks the rewind played.
#[test]
fn a_rewind_lands_on_the_recorded_run_and_leaves_the_live_race_to_the_bit() {
    let (mut race, mut keeper, text, hashes) = driven_with_hashes();
    let recorded = race.recorded().expect("recording").ticks();
    open(&mut race);
    let mut rewind = Rewind::begin(&mut keeper, &mut race).expect("a race that is running");
    assert!(rewind.resimulates());
    // Out of order on purpose, across keyframes and on them, forward and back.
    for target in [0, 61, 17, 150, 299, 120, 121, 300, 1, 240] {
        assert_eq!(rewind.seek(&mut keeper, &mut race, target), target);
        assert_eq!(race.sim.world.tick, target);
        // Tick 0 is the keyframe before the plans were learned: the same state, a flag apart in
        // the text, so the hash judges it.
        if target > 0 {
            assert!(
                format!("{:?}", race.sim) == text[target as usize],
                "tick {target} differs from the recorded run"
            );
        } else {
            assert_eq!(race.sim.state_hash(), hashes[0]);
        }
    }
    rewind.end(&mut race);
    assert!(
        format!("{:?}", race.sim) == text[LIVE as usize],
        "the live race was not put back"
    );
    assert_eq!(
        race.recorded().expect("still recording").ticks(),
        recorded,
        "the ticks the rewind played are not in the recording"
    );
}

/// Fails if the race that goes on after a visit is not the one that never had it.
#[test]
fn a_race_that_goes_on_after_a_rewind_hashes_as_one_that_was_not_visited() {
    let (mut visited, mut keeper, _) = driven();
    let (mut control, _, _) = driven();
    open(&mut visited);
    let mut rewind = Rewind::begin(&mut keeper, &mut visited).expect("running");
    rewind.seek(&mut keeper, &mut visited, 77);
    rewind.seek(&mut keeper, &mut visited, 200);
    rewind.end(&mut visited);
    visited.photo_end();
    for tick in LIVE..LIVE + 200 {
        visited.tick(&inputs(tick));
        control.tick(&inputs(tick));
        keeper.keep(&visited);
        assert_eq!(
            visited.sim.state_hash(),
            control.sim.state_hash(),
            "tick {tick}"
        );
    }
}

/// A race that records nothing has no inputs to play: a seek stops at a whole keyframe.
#[test]
fn with_no_recording_a_seek_stops_at_the_keyframe_before_it() {
    let mut race = Box::new(Race::start(fixture()));
    let mut keeper = Keeper::new(fixture(), &race);
    for tick in 0..LIVE {
        race.tick(&inputs(tick));
        keeper.keep(&race);
    }
    open(&mut race);
    let mut rewind = Rewind::begin(&mut keeper, &mut race).expect("running");
    assert!(!rewind.resimulates());
    assert_eq!(rewind.seek(&mut keeper, &mut race, 150), 120);
    assert_eq!(race.sim.world.tick, 120);
    rewind.end(&mut race);
    assert_eq!(race.sim.world.tick, LIVE);
}

fn blur(race: &mut Race, kind: i32, shutter: i32) {
    race.photo_effects(|effects| {
        effects.select(Row::MotionBlur);
        effects.adjust(kind);
        effects.select(Row::Shutter);
        effects.adjust_row(Row::Shutter, shutter - effects.shutter());
    });
}

#[test]
fn the_plan_spreads_its_frames_over_the_shutter_and_ends_on_the_shot() {
    let (mut race, _, _) = driven();
    open(&mut race);
    let photo = race.photo().expect("open");
    assert!(
        Plan::new(photo, LIVE, true).is_none(),
        "nothing asked, the frame on screen"
    );
    blur(&mut race, 1, 100);
    let photo = race.photo().expect("open");
    let plan = Plan::new(photo, LIVE, true).expect("a track blur");
    assert_eq!(plan.frames(), 120);
    assert_eq!(plan.blur(), Some((Blur::Track, 15.0)));
    let first = plan.frame(0);
    assert_eq!((first.stand, first.alpha), (LIVE - 15, 1.0));
    let last = plan.frame(119);
    assert_eq!((last.stand, last.alpha), (LIVE, 1.0));
    let mut time = f32::MIN;
    for index in 0..120 {
        let frame = plan.frame(index);
        let t = (frame.stand as f32 - 1.0) + frame.alpha;
        assert!(t >= time, "frame {index} goes back in time");
        time = t;
    }
    assert!(
        Plan::new(photo, LIVE, false).is_none(),
        "with no recording there is no blur to draw"
    );
    // A short shutter stays within a tick or two of the shot.
    blur(&mut race, 0, 20);
    let photo = race.photo().expect("open");
    let short = Plan::new(photo, LIVE, true).expect("still a blur");
    assert!(short.frame(0).stand >= LIVE - 1);
    // 2048's ship shutter is a twentieth of the track's.
    race.photo_offer(Some(Accumulation::WIPEOUT_2048), true);
    blur(&mut race, 1, 100);
    let photo = race.photo().expect("open");
    let ship = Plan::new(photo, LIVE, true).expect("a ship blur");
    assert_eq!(ship.frames(), 130);
    let (kind, ticks) = ship.blur().expect("blur");
    assert_eq!(kind, Blur::Ship);
    assert!((ticks - 0.75).abs() < 1e-4, "{ticks}");
}

#[test]
fn the_lens_samples_surround_the_centre_and_average_to_it() {
    let (mut race, _, _) = driven();
    open(&mut race);
    race.photo_effects(|effects| {
        effects.select(Row::DepthOfField);
        effects.adjust(1);
    });
    let photo = race.photo().expect("open");
    let plan = Plan::new(photo, LIVE, false).expect("a depth of field");
    assert_eq!(plan.frames(), 7);
    assert!(plan.has_lens());
    let shifts: Vec<LensShift> = (0..7)
        .map(|i| plan.frame(i).lens.expect("a sample"))
        .collect();
    assert_eq!((shifts[0].dx, shifts[0].dy), (0.0, 0.0));
    let radius = shifts[1].dx.hypot(shifts[1].dy);
    assert!(radius > 0.0);
    for shift in &shifts[1..] {
        assert!((shift.dx.hypot(shift.dy) - radius).abs() < 1e-4);
    }
    let mean = shifts
        .iter()
        .fold((0.0, 0.0), |m, s| (m.0 + s.dx, m.1 + s.dy));
    assert!(mean.0.abs() < 1e-3 && mean.1.abs() < 1e-3, "{mean:?}");
    assert!(
        shifts.iter().all(|s| s.focus == shifts[0].focus),
        "one focus plane"
    );
    for i in 0..7 {
        assert_eq!(plan.frame(i).stand, LIVE, "the same tick drawn again");
    }
}

/// The focus plane stays where it was and a nearer point moves: the lens, not a pan.
#[test]
fn a_lens_sample_keeps_the_focus_plane_sharp() {
    let view = Mat4::IDENTITY;
    let shift = LensShift {
        dx: 1.5,
        dy: -0.5,
        focus: 40.0,
    };
    let moved = shift.apply(view);
    let at_focus = moved.transform_point3(Vec3::new(0.0, 0.0, -40.0));
    assert!(
        at_focus.x.abs() < 1e-4 && at_focus.y.abs() < 1e-4,
        "{at_focus:?}"
    );
    let near = moved.transform_point3(Vec3::new(0.0, 0.0, -10.0));
    assert!(near.x.abs() > 0.1 || near.y.abs() > 0.1, "{near:?}");
    let far = moved.transform_point3(Vec3::new(0.0, 0.0, -200.0));
    assert!(
        far.x * near.x < 0.0 || far.y * near.y < 0.0,
        "near and far smear opposite ways"
    );
}

/// Where the camera is, in world space.
fn eye(race: &Race) -> Vec3 {
    race.view().inverse().w_axis.truncate()
}

fn the_eye_and_the_craft_through(kind: i32) -> Vec<(Vec3, Vec3, Vec3)> {
    let (mut race, mut keeper, _) = driven();
    open(&mut race);
    blur(&mut race, kind, 100);
    let mut rewind = Rewind::begin(&mut keeper, &mut race).expect("running");
    let plan = Plan::new(race.photo().expect("open"), LIVE, rewind.resimulates()).expect("blur");
    let mut shot = Shot::start(
        plan,
        Some(Mover {
            rewind: &mut rewind,
            keeper: &mut keeper,
        }),
        &mut race,
    );
    let mut seen = Vec::new();
    while !shot.done() {
        let alpha = shot.prepare(
            Some(Mover {
                rewind: &mut rewind,
                keeper: &mut keeper,
            }),
            &mut race,
        );
        race.set_draw_alpha(alpha);
        let slot = race.player_slot();
        let craft = race.ship_model_matrix_of(slot).w_axis.truncate();
        seen.push((eye(&race), craft, race.view().transform_point3(craft)));
        shot.advance();
    }
    shot.finish(
        Some(Mover {
            rewind: &mut rewind,
            keeper: &mut keeper,
        }),
        &mut race,
    );
    assert_eq!(
        race.sim.world.tick, LIVE,
        "the race is back at the shot's tick"
    );
    seen
}

/// Fails if a ship blur's camera rides the craft (nothing would smear), or if a track blur's
/// camera holds still (the scenery would stay sharp and the craft would smear).
#[test]
fn a_ship_blur_holds_the_camera_and_a_track_blur_follows_the_craft() {
    let ship = the_eye_and_the_craft_through(2);
    let track = the_eye_and_the_craft_through(1);
    type Seen = (Vec3, Vec3, Vec3);
    let spread = |points: &[Seen], pick: fn(&Seen) -> Vec3| {
        let first = pick(&points[0]);
        points
            .iter()
            .map(|p| pick(p).distance(first))
            .fold(0.0, f32::max)
    };
    let eyes = |p: &Seen| p.0;
    let crafts = |p: &Seen| p.1;
    let in_view = |p: &Seen| p.2;
    assert!(
        spread(&ship, eyes) < 1e-3,
        "the held camera moved {}",
        spread(&ship, eyes)
    );
    assert!(
        spread(&ship, crafts) > 0.05,
        "the craft did not move in the held camera's view: {}",
        spread(&ship, crafts)
    );
    assert!(
        spread(&track, eyes) > 0.05,
        "the following camera stood still: {}",
        spread(&track, eyes)
    );
    // The craft stays where it was in the picture of the camera that rides it, and slides
    // across the held one.
    assert!(
        spread(&track, in_view) < 0.05,
        "the craft slid in a track blur's picture: {}",
        spread(&track, in_view)
    );
    assert!(
        spread(&ship, in_view) > 0.05,
        "the craft stood still in a ship blur's picture: {}",
        spread(&ship, in_view)
    );
}

#[test]
fn the_blur_and_rewind_rows_are_offered_only_where_the_title_and_the_race_have_them() {
    use crate::photo::effects::Effects;
    let bare = Effects::new();
    assert!(!bare.rows().contains(&Row::MotionBlur));
    assert!(!bare.rows().contains(&Row::Rewind));
    let mut full = Effects::new().offering(true, true);
    assert!(full.rows().contains(&Row::MotionBlur));
    assert!(full.rows().contains(&Row::Rewind));
    assert!(
        !full.rows().contains(&Row::Shutter),
        "the shutter is shown with a blur chosen"
    );
    full.select(Row::MotionBlur);
    full.adjust(1);
    assert_eq!(full.blur(), Some(Blur::Track));
    assert!(full.rows().contains(&Row::Shutter));
    full.adjust(1);
    assert_eq!(full.blur(), Some(Blur::Ship));
    full.adjust(5);
    assert_eq!(full.blur(), Some(Blur::Ship), "it stops at the last choice");
    full.adjust(-9);
    assert_eq!(full.blur(), None);
    assert!(!full.rows().contains(&Row::Shutter));
}

#[test]
fn the_rewind_row_goes_back_to_the_start_and_forward_to_the_live_tick() {
    use crate::photo::effects::Effects;
    let mut effects = Effects::new().offering(false, true);
    effects.set_live(100);
    assert_eq!(effects.target_tick(), 100, "it opens on the live tick");
    effects.select(Row::Rewind);
    effects.adjust(-1);
    assert_eq!(
        effects.target_tick(),
        70,
        "left is back, half a second a push"
    );
    effects.adjust(-1);
    effects.adjust(-1);
    effects.adjust(-1);
    assert_eq!(
        effects.target_tick(),
        0,
        "it stops at the start of the race"
    );
    effects.adjust(1);
    assert_eq!(effects.target_tick(), 30);
    effects.adjust(50);
    assert_eq!(effects.target_tick(), 100, "and at the live tick");
    effects.set_target_tick(45);
    assert_eq!(effects.target_tick(), 45);
    effects.set_target_tick(900);
    assert_eq!(effects.target_tick(), 100);
}
