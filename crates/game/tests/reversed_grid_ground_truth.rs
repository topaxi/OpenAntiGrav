//! The reversed grid lands on the track too, same as
//! `race_ground_truth.rs::the_whole_grid_lands_on_the_track` checks for the
//! forward one.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this project
//! does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all \
//!     -E 'binary(reversed_grid_ground_truth)'
//! ```
//!
//! Its own file rather than a fourth-thousand line added to `race_ground_truth.rs`,
//! which is already over `scripts/check-file-size.py`'s frozen ceiling and may not
//! grow it further.
//!
//! **`16_Track`'s own reversed grid happens to pass under both the old
//! straight-line grid formula and the new curve-following one** - it is one of
//! the few HD circuits `hd_trackwall_ground_truth.rs`'s per-archive grid sweep
//! (see its own history for the numbers) found clean either way, and Pulse ships
//! the same track data. So this is not the regression test for the bug that fix
//! closed - the HD test is - but it is Pulse's own confirmation that fixing the
//! straight-line extrapolation did not cost it anything on the one reversed track
//! this project's grid has ever been measured against. See
//! `docs/ghidra/functions/psp-pulse-usa/grid.md#reversed-grids-the-straight-line-ran-off-the-curve`.

use std::path::PathBuf;

use oag_core::math::Vec3;
use oag_game::race;
use oag_physics::Raycaster;

fn image() -> Option<PathBuf> {
    oag_testdata::image("data/images/pulse-psp-usa.chd")
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_reversed_grid_lands_on_the_track_too() {
    let Some(image) = image() else { return };
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        track: Some(r"Data\Environments\16_Track\track_reversed.vex".to_string()),
        opponents: true,
        ..race::Options::default()
    })
    .expect("loading the race");
    // Twice the track's own widest half-width: the same "still roughly on the
    // track" scale `race_ground_truth.rs::envelope` uses.
    let bound = loaded.setup.spline.max_half_width() * 2.0;
    let colliders = loaded.setup.collision.clone();
    let race = race::Race::start(loaded.setup);

    assert_eq!(
        race.ship_count(),
        8,
        "16_Track reversed should field a full grid"
    );
    let ships: Vec<_> = race.sim.world.ships.iter().filter(|s| s.active).collect();
    assert_eq!(ships.len(), 8);

    let player = ships[0].physics.body;
    let forward = player.forward();
    for (index, ship) in ships.iter().enumerate().skip(1) {
        let ahead = (ship.physics.body.position - player.position).dot(forward);
        assert!(
            ahead > 0.0,
            "ship {index} is {ahead:.2} along the player's forward, so it is behind them"
        );
    }

    for (index, ship) in ships.iter().enumerate() {
        let position = ship.physics.body.position;
        let distance = race
            .spline()
            .distance_to(position)
            .expect("the track has samples");
        assert!(
            distance < bound,
            "ship {index} is {distance:.2} from the driveable line, \
             outside the {bound:.2} envelope"
        );
        let origin = position + Vec3::Y * 20.0;
        let ray = oag_physics::Ray::new(origin, Vec3::NEG_Y, 80.0);
        assert!(
            Raycaster::raycast(&colliders, ray, None, false).is_some(),
            "ship {index} has no collision surface under it"
        );
    }
}
