//! The Quake wave's effect frame takes its `+Y` from the track.
//!
//! `Quake_Update` (`0x0891d268`) builds row 1 of the `WO_QUAKE` frame as
//! `-(+0x20)` of the record `AiTrack_LocatePosition` fills at the span's
//! midpoint - the track's interpolated `down` axis negated - and not as world
//! up. On a banked stretch the two differ; dropping the `orient` call in
//! `Race::advance_quake_visual` takes this to world up everywhere.
//!
//! **`#[ignore]`d and never run in CI**: it needs a disc image.

use oag_gameplay::PlayerInputs;
use oag_raceplay as race;

#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_quake_frame_leans_with_the_banked_track() {
    let Some(path) = oag_testdata::image("pulse-psp-usa.chd") else {
        return;
    };
    let loaded = race::load(&race::Options {
        source: path.display().to_string(),
        mode: oag_race::Mode::SingleRace,
        ..race::Options::default()
    })
    .expect("loading the race");
    let mut race = race::Race::start(loaded.setup);
    for _ in 0..5 {
        race.tick(&PlayerInputs::none());
    }
    let stats = race.quake_stats().expect("Pulse authors a Quake");
    let length = race.course().expect("a course").length();
    let mut leaned = 0;
    for step in 0..40 {
        let progress = length * (step as f32 / 40.0);
        race.sim.world.quake = Some(oag_weapons::projectile::quake::Wave::launch(
            0, progress, 1.0, &stats,
        ));
        race.tick(&PlayerInputs::none());
        let Some(up) = race.quake_frame_up() else {
            continue;
        };
        let point = race.quake_point().expect("a road point");
        let (_, sample, _) = race.spline().nearest(point).expect("a sample");
        // The midpoint is not the course point the wave samples at, so the
        // match is to a few hundredths, not exact.
        let expected = (-oag_core::math::Vec3::from_array(sample.down)).normalize();
        assert!(
            (up - expected).length() < 0.05,
            "step {step}: frame up {up:?}, track up {expected:?}"
        );
        if up.dot(oag_core::math::Vec3::Y) < 0.98 {
            leaned += 1;
        }
    }
    assert!(leaned > 0, "no banked stretch on the sampled circuit");
}
