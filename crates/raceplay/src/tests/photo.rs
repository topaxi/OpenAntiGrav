//! Photo mode on a synthetic race: the camera is imposed and put back, the world is not
//! touched, and a split screen is refused.

use super::*;
use crate::photo::{Action, Lens, Pad};
use oag_gameplay::PlayerInputs;

fn pad() -> Pad {
    Pad::default()
}

#[test]
fn photo_mode_opens_on_the_frame_the_pause_left() {
    let mut race = race_with_a_grid();
    for _ in 0..20 {
        race.tick(&PlayerInputs::none());
    }
    let (view, fov) = (
        race.view.view(&race.core),
        race.view
            .vertical_fov(&race.core, 1.5, oag_display::display::Fov::AUTHORED),
    );
    assert!(race.photo_begin(false, false, None));
    assert_eq!(
        race.photo().map(crate::photo::Photo::lens),
        Some(Lens::Free)
    );
    let opened = race.view.view(&race.core);
    for (a, b) in view.to_cols_array().iter().zip(opened.to_cols_array()) {
        assert!(
            (a - b).abs() < 2e-3,
            "view moved: {view:?} against {opened:?}"
        );
    }
    let opened_fov = race
        .view
        .vertical_fov(&race.core, 1.5, oag_display::display::Fov::AUTHORED);
    assert!(
        (fov - opened_fov).abs() < 1e-4,
        "{fov} against {opened_fov}"
    );
}

#[test]
fn choosing_the_free_lens_from_another_starts_where_that_lens_was() {
    let mut race = race_with_a_grid();
    race.tick(&PlayerInputs::none());
    assert!(race.photo_begin(false, false, None));
    for _ in 0..2 {
        race.photo_cycle_lens();
    }
    assert_eq!(
        race.photo().map(crate::photo::Photo::lens),
        Some(Lens::Close)
    );
    let close = race.view.view(&race.core);
    race.photo_cycle_lens();
    assert_eq!(
        race.photo().map(crate::photo::Photo::lens),
        Some(Lens::Free)
    );
    for (a, b) in close
        .to_cols_array()
        .iter()
        .zip(race.view.view(&race.core).to_cols_array())
    {
        assert!((a - b).abs() < 2e-3, "the picture jumped");
    }
}

#[test]
fn photo_mode_imposes_a_camera_and_leaving_puts_back_the_one_that_was_in_force() {
    let mut race = race_with_a_grid();
    race.tick(&PlayerInputs::none());
    let before = race.view.camera_position(&race.core);
    assert!(race.photo_begin(false, false, None));
    assert!(race.photo().is_some());
    for _ in 0..2 {
        race.photo_cycle_lens();
    }
    assert_eq!(
        race.photo().map(crate::photo::Photo::lens),
        Some(Lens::Close)
    );
    for _ in 0..30 {
        race.photo_step(Pad {
            stick: (1.0, 0.5),
            ..pad()
        });
    }
    assert_ne!(
        race.view.camera_position(&race.core),
        before,
        "the eye moved off the chase camera"
    );
    race.photo_end();
    assert!(race.photo().is_none());
    assert_eq!(
        race.view.camera_position(&race.core),
        before,
        "the chase camera is back"
    );
}

#[test]
fn flying_the_free_lens_moves_the_camera_and_leaving_puts_the_race_camera_back() {
    let mut race = race_with_a_grid();
    race.tick(&PlayerInputs::none());
    let before = race.view.camera_position(&race.core);
    assert!(race.photo_begin(false, false, None));
    for _ in 0..30 {
        race.photo_step(Pad {
            stick: (0.0, 1.0),
            right: (0.3, 0.1),
            ..pad()
        });
    }
    assert!((race.view.camera_position(&race.core) - before).length() > 5.0);
    race.photo_end();
    assert_eq!(race.view.camera_position(&race.core), before);
}

#[test]
fn a_race_taken_through_photo_mode_hashes_as_one_that_was_not() {
    let mut control = race_with_a_grid();
    let mut photographed = race_with_a_grid();
    for tick in 0..120 {
        control.tick(&PlayerInputs::none());
        photographed.tick(&PlayerInputs::none());
        if tick == 60 {
            assert!(photographed.photo_begin(false, false, None));
            for step in 0..90 {
                let moved = Pad {
                    stick: ((step as f32 * 0.2).sin(), (step as f32 * 0.1).cos()),
                    right: ((step as f32 * 0.3).cos(), (step as f32 * 0.05).sin()),
                    drag: (step as f32, -(step as f32)),
                    scroll: if step % 7 == 0 { 1.0 } else { 0.0 },
                    next: step % 30 == 29,
                    reset: step == 70,
                    ..pad()
                };
                photographed.photo_step(moved);
            }
            photographed.photo_end();
        }
        assert_eq!(
            control.core.sim.state_hash(),
            photographed.core.sim.state_hash(),
            "diverged at tick {tick}"
        );
    }
}

#[test]
fn a_race_split_across_views_does_not_open_photo_mode() {
    let mut race = race_with_a_grid();
    assert_eq!(race.set_viewport_count(2), 2);
    assert!(!race.photo_begin(false, false, None));
    assert!(race.photo().is_none());
    assert_eq!(race.photo_step(pad()), Action::None);
}

/// A grid on a straight whose control points each author their own section, so two craft
/// at opposite ends of it stand in different sections.
fn race_on_a_sectioned_straight() -> Race {
    let mut setup = setup(hulled_handling());
    let ai = straight_track_in_sections(12.0, 12.0, |k| u8::try_from(k + 1).unwrap_or(u8::MAX));
    setup.spline = Spline::from_track(&ai);
    setup.course = Course::from_track(&ai, None);
    setup.ai = ai;
    setup.mode = Mode::SingleRace;
    setup.start_position = Some(oag_vex::track::StartPosition {
        position: [0.0, 0.0, 0.0],
        left: [0.0, 0.0, -1.0],
        up: [0.0, 1.0, 0.0],
        forward: [1.0, 0.0, 0.0],
    });
    Race::start(
        setup,
        crate::tests::fixture_view(),
        crate::extensions::Prepared::default(),
    )
}

/// What the race's cull allows from the seat: each section sees only itself, so a seat in one
/// section is a set of one.
fn each_section_sees_itself() -> crate::visibility::TrackVisibility {
    let pvs = oag_vex::pvs::TrackPvs::from_sections((1u8..=8).map(|index| oag_vex::pvs::Section {
        index,
        visible: 1u64 << index,
        bounds: None,
    }));
    crate::visibility::TrackVisibility::declaring(pvs)
}

/// While photo mode is open nothing is culled by section, on whichever craft it frames: a shot
/// must not lose scenery to a set authored for a craft in a race. Leaving restores the cull.
#[test]
fn photo_mode_draws_every_section_and_leaving_restores_the_cull() {
    let mut race = race_on_a_sectioned_straight();
    let visibility = each_section_sees_itself();
    let view = Mat4::ZERO;
    let seat = race.visible_set(&visibility, &view);
    assert!(!seat.is_everything(), "the race culls: {seat:?}");

    assert!(race.photo_begin(false, false, None));
    assert!(
        race.visible_set(&visibility, &view).is_everything(),
        "the seat's own picture"
    );
    for _ in 0..3 {
        race.photo_step(Pad {
            next: true,
            ..pad()
        });
    }
    assert_eq!(race.photo().map(crate::photo::Photo::subject), Some(3));
    assert!(
        race.visible_set(&visibility, &view).is_everything(),
        "another craft's"
    );

    race.photo_end();
    assert_eq!(
        race.visible_set(&visibility, &view),
        seat,
        "the cull is back"
    );
}

/// The weather's section is the framed craft's, located from its driver where it is an
/// opponent (whose `Ship::segment` is never written).
#[test]
fn photo_modes_weather_section_is_the_subjects() {
    let mut race = race_on_a_sectioned_straight();
    let last = race.core.sim.spline().len() - 1;
    let pos = race.core.sim.spline().sample(last).expect("a sample").pos;
    let line = (0..race.core.sim.ai_order.len())
        .find(|&k| race.core.sim.ai_sample_index(k) == Some(last))
        .expect("the last sample is on the racing line");
    let ship = &mut race.core.sim.world.ships[3];
    ship.physics.body.position = Vec3::from_array(pos);
    ship.driver.index = u32::try_from(line).expect("a short fixture");
    let seat = race.section_of_framed();
    assert!(race.photo_begin(false, false, None));
    for _ in 0..3 {
        race.photo_step(Pad {
            next: true,
            ..pad()
        });
    }
    let subject = race.section_of_framed();
    assert_ne!(subject, UNPLACED);
    assert_ne!(subject, seat);
    assert_eq!(
        subject,
        race.core
            .sim
            .spline()
            .sample(last)
            .expect("a sample")
            .section_id
    );
}
