//! Confirms `oag_render::mesh::is_blink_light_name` actually fires on a real ship,
//! not just on the mined name strings it was written against.
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
//! The heuristic in `oag_render::mesh::is_blink_light_name` was built from grepping
//! mesh names out of the PSP `Data.wad` by hand (see `docs/formats/vex.md`, "Ship
//! lights: three separate `*flash*`/`*GLOW*` families, not one"). That grep is not
//! code path anything else exercises, so a rename or a decoding change could silently
//! stop the heuristic from ever matching real data while every synthetic name test
//! keeps passing. This walks every playable team's real `Ship.vex` and asserts that at
//! least one of them decodes at least one `blink`-tagged draw call, which is the
//! actual claim being made: that some ship in the game has a mesh this heuristic
//! catches.

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
fn at_least_one_team_has_a_blink_tagged_draw_call() {
    let Some(image) = image() else { return };
    let mut archives =
        oag_assets::pulse::Archives::open(&image.display().to_string()).expect("opening archives");

    let mut teams_with_blink = Vec::new();
    for team in TEAMS {
        let name = ship_entry_name(team);
        let blob = archives.read_name(&name).expect("reading Ship.vex");
        let model = mesh::build(&name, &blob).expect("decoding Ship.vex");
        let blink_draws = model.draws.iter().filter(|d| d.blink).count();
        println!(
            "{team}: {blink_draws} blink-tagged draw call(s) of {}",
            model.draws.len()
        );
        if blink_draws > 0 {
            teams_with_blink.push(team);
        }
    }

    assert!(
        !teams_with_blink.is_empty(),
        "no team's Ship.vex decoded a blink-tagged draw call - \
         is_blink_light_name no longer matches anything real"
    );
}
