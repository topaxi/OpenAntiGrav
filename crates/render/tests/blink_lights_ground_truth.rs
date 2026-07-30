//! Confirms `oag_render::mesh::is_blink_light_texture` actually fires on every
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
//! The heuristic in `oag_render::mesh::is_blink_light_texture` was built from
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

use std::path::{Path, PathBuf};

use oag_formats::handling::TEAMS;
use oag_render::mesh;

/// The archive entry name for a team's ship model, assembled the way
/// `oag_game::race::ship_entry_name` assembles it. Not shared from there: no
/// crate may depend on `oag-game`, the composition root.
fn ship_entry_name(team: &str) -> String {
    format!(r"Data\Ships\{team}\Ship.vex")
}

fn image() -> Option<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/images/pulse-psp-usa.chd");

    if path.exists() {
        return Some(path);
    }
    assert!(
        std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
        "OAG_REQUIRE_GAME_DATA is set but {} is missing",
        path.display()
    );
    println!("skipping: {} not present", path.display());
    None
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn every_team_has_a_glow_tagged_vertex() {
    let Some(image) = image() else { return };
    let mut archives =
        oag_assets::pulse::Archives::open(&image.display().to_string()).expect("opening archives");

    let mut teams_without_glow = Vec::new();
    for team in TEAMS {
        let name = ship_entry_name(team);
        let blob = archives.read_name(&name).expect("reading Ship.vex");
        let model = mesh::build(&name, &blob).expect("decoding Ship.vex");
        let glow_vertices = model.vertices.iter().filter(|v| v.glow == 1.0).count();
        println!(
            "{team}: {glow_vertices} glow-tagged vertex/vertices of {}",
            model.vertices.len()
        );
        if glow_vertices == 0 {
            teams_without_glow.push(team);
        }
    }

    assert!(
        teams_without_glow.is_empty(),
        "team(s) with no glow-tagged vertex: {teams_without_glow:?} - \
         is_blink_light_texture no longer matches every ship's real data"
    );
}
