//! The `blob` shadow tier's placement, against a real circuit's own geometry.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! # or only this test:
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all shadow
//! ```
//!
//! Its own file rather than another test in `race_ground_truth.rs`, which is
//! at its own size ratchet - see `scripts/check-file-size.py`.
//!
//! `oag_render::shadow`'s tests build a quad from a placement handed to them.
//! What they cannot ask is whether the *placement* is right on a real track,
//! which is a question about the cast: a wall hit taken for a floor, a reach
//! that does not reach, a normal read off the wrong surface. None of that
//! shows up in synthetic data.

use std::path::{Path, PathBuf};

use oag_game::race;
use oag_physics::SpeedClass;

fn image() -> Option<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/images/pulse-psp-usa.chd");
    if path.exists() {
        return Some(path);
    }
    assert!(
        std::env::var("OAG_REQUIRE_GAME_DATA").is_err(),
        "OAG_REQUIRE_GAME_DATA is set and {} is missing",
        path.display()
    );
    None
}

/// A full grid on the default circuit, so every slot's cast is exercised
/// rather than only the player's.
fn load_with_opponents() -> Option<race::Loaded> {
    let image = image()?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: SpeedClass::Venom,
        opponents: true,
        ..race::Options::default()
    })
    .expect("loading the race");
    for line in &loaded.report {
        println!("{line}");
    }
    Some(loaded)
}

/// Every craft on the grid gets a blob shadow, on the floor it is actually
/// hovering over.
///
/// The composition question `oag_render::shadow`'s own tests cannot ask: those
/// build a quad from a placement handed to them, and this checks the placement
/// against a real circuit's own collision geometry. What would break it is the
/// cast - a wall hit taken for a floor, a reach that does not reach, a normal
/// read off the wrong surface - and none of that shows up in synthetic data.
///
/// Nothing here pins a *size* or a darkness: the fade and the lift are this
/// project's own numbers (see `docs/rendering/shadows.md`), and a test that
/// pinned them would be pinning our own arithmetic and calling it a
/// measurement.
#[test]
#[ignore = "needs a real disc image under data/images/"]
fn every_craft_on_the_grid_casts_a_blob_shadow_onto_the_floor() {
    let Some(loaded) = load_with_opponents() else {
        return;
    };
    let race = race::Race::start(loaded.setup);
    let placements = race.shadow_placements();
    assert_eq!(
        placements.len(),
        usize::from(race.ship_count()),
        "a craft resting on the grid has ground under it, so every slot places"
    );

    for placement in &placements {
        let slot = placement.silhouette;
        let ship = &race.world.ships[slot];
        let body = &ship.physics.body;
        let up = body.up();
        // Below the craft, along its own up axis - not merely nearby.
        let height = (body.position - placement.contact).dot(up);
        assert!(
            height > 0.0,
            "slot {slot}: the contact is {height:.3} above the craft"
        );
        // On a floor, which is the whole point of filtering the cast: a wall
        // hit would give a normal pointing across the track rather than out of
        // it, and the quad would stand up on the barrier.
        let along_up = placement.normal.dot(up);
        assert!(
            along_up > 0.5,
            "slot {slot}: the surface normal is {along_up:.3} against the craft's own up, \
             which is a wall rather than a floor"
        );
        assert!(
            placement.strength > 0.0 && placement.strength <= 1.0,
            "slot {slot}: strength {}",
            placement.strength
        );
        // The hull's own footprint, so a zero here is a handling table that
        // did not arrive rather than a shadow that is merely small.
        assert!(
            placement.half_length > 0.0 && placement.half_width > 0.0,
            "slot {slot}: {:.3} x {:.3}",
            placement.half_length,
            placement.half_width
        );
        println!(
            "slot {slot}: {:.2} above a floor, strength {:.3}, {:.2} x {:.2} units",
            height,
            placement.strength,
            placement.half_length * 2.0,
            placement.half_width * 2.0
        );
    }
}
