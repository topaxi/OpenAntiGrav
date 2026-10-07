//! Every Wipeout HD team's hull yields two hinged airbrake flaps.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-render --run-ignored all \
//!     -E 'binary(hd_airbrake_flaps_ground_truth)'
//! ```
//!
//! HD's `Ship.vex` authors the same `Airbrake` tree Pulse does and the
//! `.rcsmodel` beside it supplies the triangles, so a flap is the vertices of
//! the `Mesh` under an `Airbrake_Left`/`Airbrake_Right` node. Dropping the
//! collection in `mesh::rcs::build` leaves every ship with no flaps, which no
//! other test notices because a missing flap draws perfectly well.
//!
//! **The hinge is an `Anim Transform` here** (`Airbrake_L`/`Airbrake_R`, class
//! `0x3c0`, one static key), not a plain `Transform` as on Pulse, so the flap's
//! vertices are in the hinge's own space and `Flap::hinge` is the identity.
//! What is asserted instead is the physical claim Pulse's recovered axis makes:
//! a positive deflection about the hinge's local X raises the flap and flares
//! it outward, on both sides of every team, in world space.

mod archive_cache;

use oag_core::math::{Mat4, Vec3};
use oag_mesh::mesh;
use oag_vex::vex;

/// The twelve teams and Zone's own shared hull (`oag_hd::race::ZONE_SHIP`),
/// which is what a Zone race flies whoever the player picked.
const TEAMS: [&str; 13] = [
    "zone",
    "ag_systems",
    "assegai",
    "auricom",
    "egx",
    "feisar",
    "goteki",
    "harimau",
    "icaras",
    "mirage",
    "piranha",
    "qirex",
    "triakis",
];

/// The hull, and the world matrix of its `Airbrake_L` and `Airbrake_R` hinges.
fn build(spec: &str, path: &str) -> Option<(mesh::Model, [Mat4; 2])> {
    let data = archive_cache::read(spec, path)?;
    let geometry = mesh::rcs::sibling_geometry(spec, path, &data)?;
    let (model, _) = mesh::rcs::build(
        path,
        &data,
        &geometry,
        &mut |name| archive_cache::read(spec, name),
        |c| c.mesh,
    )
    .ok()?;
    let nodes = vex::nodes(&data).ok()?;
    let world = vex::world_transforms(&data, &nodes);
    let mut hinges = [Mat4::IDENTITY; 2];
    for (i, n) in nodes.iter().enumerate() {
        match n.name.as_deref() {
            Some("Airbrake_L") => hinges[0] = Mat4::from_cols_array(&world[i]),
            Some("Airbrake_R") => hinges[1] = Mat4::from_cols_array(&world[i]),
            _ => {}
        }
    }
    Some((model, hinges))
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn every_hd_team_has_two_flaps_that_rise_and_flare_outward() {
    let Some(image) = oag_testdata::image("hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let mut bad = Vec::new();
    for team in TEAMS {
        let built = ["DATA02", "DATA06"].iter().find_map(|a| {
            let spec = format!("{}:PS3_GAME/USRDIR/{a}.PSARC", image.display());
            ["", "_c1"]
                .iter()
                .find_map(|v| build(&spec, &format!("/data/ships/{team}{v}/ship.vex")))
        });
        let Some((model, hinges)) = built else {
            bad.push(format!("{team}: no hull built"));
            continue;
        };
        for (side, sign) in [(0, 1.0f32), (1, -1.0)] {
            let Some(flap) = &model.airbrakes[side] else {
                bad.push(format!("{team}: side {side} has no flap"));
                continue;
            };
            let mut moved = Vec::new();
            let Some(span) = flap.swung(&model.vertices, 0.5, &mut moved) else {
                bad.push(format!("{team}: side {side} span outside the buffer"));
                continue;
            };
            let delta = moved
                .iter()
                .zip(&model.vertices[span.clone()])
                .map(|(a, b)| Vec3::from_array(a.position) - Vec3::from_array(b.position))
                .sum::<Vec3>()
                / span.len() as f32;
            let world = hinges[side].transform_vector3(delta);
            println!(
                "{team} side {side}: {} verts, world delta {world:.3?}",
                span.len()
            );
            // Auricom's flap is authored to swing out sideways like a door
            // (`y` is 0.000, `x` 0.76), so it only has to not sink.
            if world.y < -1e-3 || world.x * sign <= 0.0 {
                bad.push(format!("{team}: side {side} swings {world:?}"));
            }
        }
    }
    assert!(bad.is_empty(), "{bad:#?}");
}
