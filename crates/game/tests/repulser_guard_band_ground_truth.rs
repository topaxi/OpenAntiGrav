//! The Repulser's wave-start `shazzam` quad is dropped by the GE's guard band.
//!
//! A standing Repulser fired on Talon's Junction submits `WO_REPULSER`'s `shazzam`
//! templates at view depth ~12, corners ~2000 px off the screen centre; the
//! original draws no band there (two replayed GE dumps, `particle-system.md`,
//! "the guard band is the law"). Dropping `oag_fx::psys::guard`'s call takes a
//! vertex out of range back into the draw list.
//!
//! **`#[ignore]`d and never run in CI**: it needs a disc image.

use oag_core::math::Vec3;
use oag_fx::psys::guard::GuardBand;
use oag_gameplay::PlayerInputs;
use oag_gameplay::input::{Button, Input};
use oag_raceplay as race;

#[test]
#[ignore = "needs a disc image in data/images/"]
fn no_wave_start_quad_leaves_the_guard_band() {
    let Some(path) = oag_testdata::image("pulse-psp-usa.chd") else {
        return;
    };
    let loaded = race::load(&race::Options {
        source: path.display().to_string(),
        mode: oag_race::Mode::TimeTrial,
        ..race::Options::default()
    })
    .expect("loading the race");
    let mut race = race::Race::start(loaded.setup);
    for _ in 0..oag_race::COUNTDOWN_TICKS + 5 {
        race.tick(&PlayerInputs::none());
    }
    race.sim.world.ships[0].pickup.weapon = Some(oag_tables::weapons::Weapon::Repulser);
    let mut buttons = Input::new();
    buttons.begin_frame(0);
    let mut fire = oag_gameplay::InputSnapshot::new();
    fire.buttons = buttons;
    fire.buttons.begin_frame(Button::Square.bit());
    race.tick(&PlayerInputs::single(fire));
    let mut checked = 0;
    for tick in 0..70 {
        race.tick(&PlayerInputs::none());
        let view = race.view();
        let (right, up) = (
            Vec3::new(view.x_axis.x, view.y_axis.x, view.z_axis.x),
            Vec3::new(view.x_axis.y, view.y_axis.y, view.z_axis.y),
        );
        let guard = GuardBand {
            eye: race.camera_position(),
            right,
            up,
            forward: right.cross(up).normalize() * -1.0,
        };
        let (mut additive, mut alpha) = (Vec::new(), Vec::new());
        race.extend_stage_vertices(&mut additive, &mut alpha, right, up);
        for vertex in additive.iter().chain(&alpha) {
            checked += 1;
            assert!(
                !guard.drops([Vec3::from(vertex.position)]),
                "tick {tick}: a particle vertex {:?} is outside the GE's screen space",
                vertex.position
            );
        }
    }
    assert!(checked > 0, "the Repulser drew nothing at all");
}
