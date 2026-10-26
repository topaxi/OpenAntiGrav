//! A craft placed by a capture is in a section, so the visible set is not "draw everything".
//!
//! `Race::visibility_sections` read the sample a tick left in `Ship::segment`; a posed capture runs no tick,
//! `segment` was still its start value `0`, far from the pose, and the section came back `UNPLACED`, which the
//! visible set turns into every section. On `pulse-psp-race/talons-alongside` that drew the terrain and mountains
//! over the tunnel's ceiling (the original's recorded GE list holds none of Talon's `ground1` chunks at that
//! pose), and 2,102 draws where the located craft submits 638; mean grey difference to the original's frame
//! `14.14` to `5.84`.
//!
//! **`#[ignore]`d and never run in CI**: it needs the Pulse PSP disc. Run with `just test-data`.

use oag_core::math::Vec3;
use oag_render::pvs::UNPLACED;

/// Where the pair `pulse-psp-race/talons-alongside` put the player: inside Talon's Junction's tunnel.
const TUNNEL: Vec3 = Vec3::new(-118.38782, -49.23908, -702.0541);

#[test]
#[ignore = "needs the Pulse PSP disc in data/images/"]
fn a_craft_no_tick_has_located_is_in_a_section() {
    let Some(image) = oag_testdata::image("data/images/pulse-psp-usa.chd") else {
        return;
    };
    let loaded = oag_raceplay::load(&oag_raceplay::Options {
        source: image.display().to_string(),
        track: Some(r"Data\Environments\16_Track\track.vex".to_string()),
        class: "VENOM".to_string(),
        ..oag_raceplay::Options::default()
    })
    .expect("loading Talon's Junction");
    let mut race = oag_raceplay::Race::start(loaded.setup);
    race.sim.world.ships[0].physics.body.position = TUNNEL;
    let (craft, _camera) = race.visibility_sections();
    assert_ne!(
        craft, UNPLACED,
        "the craft stands on the track and must read its section, not the draw-everything sentinel"
    );
}
