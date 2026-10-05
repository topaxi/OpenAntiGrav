//! Confirms every playable team's `Ship.vex` yields two hinged airbrake flaps.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! # or only this crate:
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-render --run-ignored all
//! ```
//!
//! # What this is for
//!
//! `oag_mesh::mesh` finds a flap by shape - a `Mesh` whose parent is an
//! `Airbrake` (`0x3c5`) whose parent is the hinge `Transform` - and by the
//! artists' node names, `Airbrake_Left` and `Airbrake_Right`. Both halves are
//! assumptions about how the files are authored, taken from reading **one**
//! team's tree, and neither is checked by anything else: a ship that nests its
//! flap one level deeper, or names it `airbrake_l`, would silently produce a
//! model with no flaps and a ship whose airbrakes never move. Nothing else in
//! the renderer would notice, because a missing flap draws perfectly well - it
//! just never deflects.
//!
//! So this asserts the claim actually being made: **all eight** playable teams,
//! **both** sides, with a non-empty vertex span and a hinge that is not the
//! identity.
//!
//! The mirrored-hinge claim is the other half. Assegai's two locators are
//! `+1.5246` and `-1.5246` in X, and `Flap::deflect` leans on that opposition
//! to make the two sides swing apart without either angle being negated by
//! hand. If some team authored both hinges on the same side, the flaps would
//! deflect the same way and look wrong, so the sign of the two translations is
//! asserted rather than assumed.

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
fn every_team_has_two_hinged_airbrake_flaps() {
    let Some(image) = image() else { return };
    let mut archives = oag_pulse::open(&image.display().to_string()).expect("opening archives");

    let mut missing = Vec::new();
    let mut same_side = Vec::new();
    for team in TEAMS {
        let name = ship_entry_name(team);
        let blob = archives.read_name(&name).expect("reading Ship.vex");
        let model = mesh::build(&name, &blob).expect("decoding Ship.vex");

        let [Some(left), Some(right)] = &model.airbrakes else {
            missing.push(team);
            continue;
        };

        // `w_axis` is the translation column: where the hinge sits, in model
        // space. The two must straddle the centreline.
        let (left_x, right_x) = (left.hinge.w_axis.x, right.hinge.w_axis.x);
        println!(
            "{team}: left {} vertices hinged at x={left_x:+.4}, \
             right {} vertices hinged at x={right_x:+.4}",
            left.vertices.len(),
            right.vertices.len(),
        );

        if left.vertices.is_empty() || right.vertices.is_empty() || (left_x * right_x) >= 0.0 {
            same_side.push(team);
        }
    }

    assert!(
        missing.is_empty(),
        "team(s) whose Ship.vex produced fewer than two flaps: {missing:?} - \
         either the Mesh/Airbrake/Transform nesting or the Airbrake_Left / \
         Airbrake_Right naming is not universal after all"
    );
    assert!(
        same_side.is_empty(),
        "team(s) with an empty flap or two hinges on the same side of the \
         centreline: {same_side:?} - Flap::deflect relies on the mirrored \
         hinges to swing the two sides apart"
    );
}
