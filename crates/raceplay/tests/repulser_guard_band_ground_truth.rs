//! A standing Repulser leaves no particle vertex out of the GE's screen space.
//!
//! The PSP draws no triangle with a vertex more than 2048 px off the screen centre, a sprite
//! template's or an emitter's own (`particle-system.md`, "Near the eye"), so neither may our
//! draw lists hold one. The particles round the effect draw from the stage's random stream:
//! the claim is read over the fixed arrangement and twenty others
//! ([`oag_raceplay::Race::reseed_stage`]), several of which put a `WO_REPULSER` bar or a
//! `WO_REPULSER_BLAST` bead a few units from the eye.
//!
//! **`#[ignore]`d and never run in CI**: it needs a disc image.

use oag_core::math::Vec3;
use oag_fx::psys::guard::GuardBand;
use oag_gameplay::PlayerInputs;
use oag_gameplay::input::{Button, Input};
use oag_raceplay as race;

/// The first particle vertex out of the GE's screen space over the 70 ticks after a
/// standing Repulser is fired, as `(tick, position)`, and how many vertices were checked.
/// `stage_seed` restarts the particle stage's stream; `None` keeps the fixed one.
fn first_escape(path: &std::path::Path, stage_seed: Option<u64>) -> (Option<(u32, Vec3)>, usize) {
    let loaded = race::load(&race::Options {
        source: path.display().to_string(),
        mode: oag_race::Mode::TimeTrial,
        ..race::Options::default()
    })
    .expect("loading the race");
    let mut race = race::Race::start(loaded.setup, loaded.view, loaded.modules);
    if let Some(seed) = stage_seed {
        race.reseed_stage(seed);
    }
    for _ in 0..oag_race::COUNTDOWN_TICKS + 5 {
        race.tick(&PlayerInputs::none());
    }
    race.core.sim.world.ships[0].pickup.weapon = Some(oag_tables::weapons::Weapon::Repulser);
    let mut buttons = Input::new();
    buttons.begin_frame(0);
    let mut fire = oag_gameplay::InputSnapshot::new();
    fire.buttons = buttons;
    fire.buttons.begin_frame(Button::Square.bit());
    race.tick(&PlayerInputs::single(fire));
    let mut checked = 0;
    for tick in 0..70 {
        race.tick(&PlayerInputs::none());
        let view = race.view_state().view(&race.core);
        let (right, up) = (
            Vec3::new(view.x_axis.x, view.y_axis.x, view.z_axis.x),
            Vec3::new(view.x_axis.y, view.y_axis.y, view.z_axis.y),
        );
        let guard = GuardBand {
            eye: race.view_state().camera_position(&race.core),
            right,
            up,
            forward: right.cross(up).normalize() * -1.0,
        };
        let (mut additive, mut alpha) = (Vec::new(), Vec::new());
        race.extend_stage_vertices(&mut additive, &mut alpha, right, up);
        for vertex in additive.iter().chain(&alpha) {
            checked += 1;
            let position = Vec3::from(vertex.position);
            if guard.drops([position]) {
                return (Some((tick, position)), checked);
            }
        }
    }
    (None, checked)
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn no_repulser_particle_leaves_the_guard_band() {
    let Some(path) = oag_testdata::image("pulse-psp-usa.chd") else {
        return;
    };
    for stage_seed in std::iter::once(None).chain((1..=20).map(Some)) {
        let (escape, checked) = first_escape(&path, stage_seed);
        assert!(checked > 0, "the Repulser drew nothing at all");
        if let Some((tick, position)) = escape {
            panic!(
                "stage seed {stage_seed:?}, tick {tick}: a particle vertex {position:?} is outside the GE's screen space"
            );
        }
    }
}
