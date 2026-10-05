//! Confirms `oag_pulse::textures::is_blink_light_texture` actually fires on every
//! real ship, not just on the mined texture name it was written against.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this project
//! does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! # or only this crate:
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-render --run-ignored all
//! ```
//!
//! # What this is for
//!
//! The heuristic in `oag_pulse::textures::is_blink_light_texture` was built from
//! decoding real materials on real ships (see `docs/formats/vex.md`, "Ship
//! lights: one shared texture, not a mesh-naming convention"): every one of
//! the 8 playable teams turned out to carry a mesh whose material resolves to
//! the exact same shared texture, `colours_flashing_GLOW.tga`. That discovery
//! is not exercised by any other code path, so a rename or a decoding change
//! could silently stop the heuristic from ever matching real data while every
//! synthetic name test keeps passing. This walks every playable team's real
//! `Ship.vex` and asserts that **all eight** decode at least one vertex with
//! `glow == 1.0` - the actual claim being made, and a stronger one than "at
//! least one ship" precisely because the signal generalised across every ship
//! once it was keyed on the texture instead of the mesh name.

use std::path::PathBuf;

use oag_mesh::mesh;
use oag_pulse::race::TEAMS;

/// The archive entry name for a team's ship model, assembled the way
/// `oag_raceplay::ship_entry_name` assembles it. Not shared from there: no
/// crate may depend on `oag-game`, the composition root.
fn ship_entry_name(team: &str) -> String {
    format!(r"Data\Ships\{team}\Ship.vex")
}

fn image() -> Option<PathBuf> {
    oag_testdata::image("data/images/pulse-psp-usa.chd")
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn every_team_has_a_glow_tagged_vertex() {
    let Some(image) = image() else { return };
    let mut archives = oag_pulse::open(&image.display().to_string()).expect("opening archives");

    let mut teams_without_glow = Vec::new();
    let mut teams_without_the_texture = Vec::new();
    for team in TEAMS {
        let name = ship_entry_name(team);
        let blob = archives.read_name(&name).expect("reading Ship.vex");
        let model = mesh::build(&name, &blob).expect("decoding Ship.vex");
        let glow_vertices = model.vertices.iter().filter(|v| v.anim != 0).count();
        // The predicate itself, against the label the decoder actually
        // produced. `anim` is now set from the material's own authored
        // keyframe block rather than from `ANIMATED_TEXTURES`, so the two
        // halves of this test are now genuinely independent: the count says
        // every ship authors a texture transform, and this says every ship
        // still carries the shared blink palette the ship-specific claim
        // `is_blink_light_texture` is about.
        let labelled = model
            .textures
            .iter()
            .flatten()
            .any(|t| oag_pulse::textures::is_blink_light_texture(&t.label));
        println!(
            "{team}: {glow_vertices} glow-tagged vertex/vertices of {}, \
             blink texture present: {labelled}",
            model.vertices.len()
        );
        if glow_vertices == 0 {
            teams_without_glow.push(team);
        }
        if !labelled {
            teams_without_the_texture.push(team);
        }
    }

    assert!(
        teams_without_the_texture.is_empty(),
        "team(s) whose decoded texture labels no longer match \
         is_blink_light_texture: {teams_without_the_texture:?}"
    );
    assert!(
        teams_without_glow.is_empty(),
        "team(s) with no vertex on an authored texture-transform track: \
         {teams_without_glow:?} - every team's blink-light mesh authors one \
         (offset v from 0 to -256 over frames 1..60, loop 1.0 s), so an empty \
         count means the block is no longer being read"
    );
}
