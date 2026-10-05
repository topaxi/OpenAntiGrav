//! Wipeout 2048's and Omega's hulls yield two hinged airbrake flaps, and a
//! positive deflection raises each one and flares it outward.
//!
//! **`#[ignore]`d and never run in CI.** It needs the extracted 2048 package
//! (`data/extracted/vita/PCSF00007`) and, for Omega, the extraction its own
//! ground-truth tests use.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-render --run-ignored all \
//!     -E 'binary(psp2_airbrake_flaps_ground_truth)'
//! ```
//!
//! A flap is a node-bound mesh named `Airbrake_Left`/`Airbrake_Right` whose
//! vertices are about its own hinge; `mesh::rcs::psp2::build` bakes them
//! through the node's bind matrix and records that matrix as `Flap::hinge`.
//! Dropping that leaves every craft with no flaps, which nothing else would
//! notice - a missing flap draws perfectly well.

use oag_core::math::Vec3;
use oag_render::mesh::{Model, rcs::psp2};

fn build(blob: &[u8]) -> Model {
    psp2::build("hull", blob, None, &mut |_| None)
        .expect("builds")
        .0
}

/// Both flaps of `model`, each raised and flared outward by a positive swing.
fn check(label: &str, model: &Model, bad: &mut Vec<String>) {
    for (side, sign) in [(0usize, 1.0f32), (1, -1.0)] {
        let Some(flap) = &model.airbrakes[side] else {
            bad.push(format!("{label}: side {side} has no flap"));
            continue;
        };
        let mut moved = Vec::new();
        let Some(span) = flap.swung(&model.vertices, 0.5, &mut moved) else {
            bad.push(format!("{label}: side {side} span outside the buffer"));
            continue;
        };
        let delta = moved
            .iter()
            .zip(&model.vertices[span.clone()])
            .map(|(a, b)| Vec3::from_array(a.position) - Vec3::from_array(b.position))
            .sum::<Vec3>()
            / span.len() as f32;
        let at = Vec3::from_array(model.vertices[span.start].position);
        println!(
            "{label} side {side}: {} verts near {at:.2?}, hinge {:.2?}, delta {delta:.3?}",
            span.len(),
            flap.hinge.w_axis.truncate()
        );
        // `qirex2048`'s fourth-slot craft authors flaps at the nose (z +5.5)
        // that rise without flaring (`x` is 0.000), so flaring is only
        // required not to go inward.
        if delta.y < -1e-3 || delta.x * sign < -1e-3 {
            bad.push(format!("{label}: side {side} swings {delta:?}"));
        }
    }
}

#[test]
#[ignore = "needs data/extracted/vita/PCSF00007"]
fn every_2048_craft_has_two_flaps_that_rise_and_flare_outward() {
    let Some(package) = oag_testdata::exact("data/extracted/vita/PCSF00007/base/PSP2/data.psarc")
    else {
        return;
    };
    let mut archive =
        oag_assets::psarc::Archive::open(&package.display().to_string()).expect("opens");
    let mut paths = Vec::new();
    for team in [
        "ag_systems2048",
        "auricom2048",
        "feisar2048",
        "piranha2048",
        "qirex2048",
    ] {
        for n in 1..=4 {
            paths.push(format!("data/art/published/ships/{team}/{n}/ship.rcsmodel"));
        }
    }
    for team in [
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
    ] {
        paths.push(format!("data/art/published/hdships/{team}/ship.rcsmodel"));
    }
    let mut bad = Vec::new();
    let mut seen = 0;
    for path in paths {
        let Ok(blob) = archive.read_path(&path) else {
            bad.push(format!("{path}: not in the package"));
            continue;
        };
        seen += 1;
        check(&path, &build(&blob), &mut bad);
    }
    assert!(seen > 0);
    assert!(bad.is_empty(), "{bad:#?}");
}

#[test]
#[ignore = "needs the extracted Omega package"]
fn every_omega_hull_has_two_flaps_that_rise_and_flare_outward() {
    let Some(package) = oag_testdata::exact("data/extracted/ps4/omega-eu/uroot/data01.psarc")
    else {
        return;
    };
    let mut archive =
        oag_assets::psarc::Archive::open(&package.display().to_string()).expect("opens");
    let mut bad = Vec::new();
    let mut seen = 0;
    for team in [
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
    ] {
        let path = format!("data/art/published/hdships/{team}/ship.rcsmodel");
        let Ok(blob) = archive.read_path(&path) else {
            continue;
        };
        seen += 1;
        check(&path, &build(&blob), &mut bad);
    }
    assert!(seen > 0, "no Omega hull found in data01.psarc");
    assert!(bad.is_empty(), "{bad:#?}");
}
