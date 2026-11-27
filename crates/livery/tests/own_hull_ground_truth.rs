//! Pulse's own Detonator hull resolves on every team's `Zone.vex`: each part a mesh node of the
//! disc's model, each surface a texture of it (`docs/formats/own-hull.md`).
//!
//! **`#[ignore]`d and never run in CI.** It needs game content.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-livery --run-ignored all own_hull_ground_truth::
//! ```
//!
//! The files are read from the official mod `oag.detonator-pulse` (ADR-0100), which carries them:
//! the test fails under `OAG_REQUIRE_GAME_DATA=1` without the official-mods submodule. Each
//! pressing must take its own: the PSP discs `hull.glb`, the PS2 disc, whose `Zone.vex` is
//! another model, `hull_ps2.glb`. On both, the two airbrake flaps sit on the wings and rise when
//! deflected ([`flaps_on_the_wings`]).

use oag_mesh::mesh::{self, own_hull};

/// Each of Pulse's own hull files that is committed, by entry name, parsed.
fn committed() -> Vec<(&'static str, own_hull::OwnHull)> {
    let craft = oag_pulse::race::DETONATOR
        .craft
        .expect("Pulse flies a Detonator craft");
    let Some(manifest) = oag_testdata::exact("assets/mods/official/detonator-pulse/oag-mod.toml")
    else {
        return Vec::new();
    };
    let root = manifest
        .parent()
        .expect("the mod's folder")
        .join("fallback/wipeout-pulse");
    craft
        .own_hull
        .iter()
        .filter_map(|entry| {
            let bytes = std::fs::read(root.join(entry.replace('\\', "/"))).ok()?;
            Some((*entry, own_hull::parse(&bytes).expect(entry)))
        })
        .collect()
}

fn check(image: &str, teams: &[&str], file: &str) {
    let Some(image) = oag_testdata::image(image) else {
        return;
    };
    let hulls = committed();
    if hulls.is_empty() {
        eprintln!("skipped: the official-mods submodule is not checked out");
        return;
    }
    assert_eq!(hulls.len(), 2, "the mod carries both pressings' hulls");
    let mut archives = oag_pulse::open(&image.display().to_string()).expect("opening the disc");
    for team in teams {
        let entry = format!(r"Data\Ships\{team}\Zone.vex");
        let blob = archives.read_name(&entry).expect(&entry);
        let (fitting, hull) = hulls
            .iter()
            .find(|(_, hull)| own_hull::fits(&blob, hull))
            .unwrap_or_else(|| panic!("{entry}: no own hull fits"));
        assert!(fitting.ends_with(file), "{entry} takes {fitting}");
        let mut host = mesh::build_with_textures(&entry, &blob, None).expect(&entry);
        // A PS2 model's textures are a set beside it, as the race reads them.
        if host.textures.iter().all(Option::is_none)
            && let Some(set) = oag_livery::entry::ps2_texture_set(&mut archives, &entry)
        {
            host = mesh::build_with_textures(&entry, &blob, Some(&set)).expect(&entry);
        }
        let mut report = Vec::new();
        let model = own_hull::substitute(&host, &blob, hull, &mut report)
            .unwrap_or_else(|| panic!("{entry}: {file} resolved nothing: {report:#?}"));
        let unresolved: Vec<&String> = report
            .iter()
            .filter(|line| line.contains("not drawn"))
            .collect();
        assert!(
            unresolved.is_empty(),
            "{entry} with {file}: {unresolved:#?}"
        );
        let (_, triangles) = hull.counts();
        assert!(
            model.indices.len() / 3 >= triangles,
            "{entry}: every triangle of {file} draws at least once"
        );
        let parts = hull.parts.len();
        assert_eq!(
            model.mesh_count, parts,
            "{entry}: every part of {file} draws"
        );
        flaps_on_the_wings(&entry, file, &model);
        println!("{entry}: {file}, {}", report.last().expect("a summary"));
        if team == &teams[0] {
            // A hull glb may carry images, reserved for textures of its own: none is read, and
            // the summary line says so.
            let textured = own_hull::OwnHull {
                images: 1,
                ..hull.clone()
            };
            let mut report = Vec::new();
            let _ = own_hull::substitute(&host, &blob, &textured, &mut report);
            assert!(
                report
                    .last()
                    .is_some_and(|line| line.contains("1 image(s) in the file not read")),
                "{report:#?}"
            );
        }
    }
}

/// Outboard of every fuselage part of both hulls: the craft's flank ends inside 1.3 units of the
/// centre line where the wings' trailing edges begin (`docs/gameplay/detonator-pulse.md`, "The
/// craft").
const WING_ROOT: f32 = 1.3;

/// The left flap (`+X`) hinges on the left wing's trailing edge and the right flap on its mirror
/// image, each flap's vertices lie outboard of the fuselage, and a positive deflection, a braking
/// flap, lifts the flap's trailing edge (its mean height rises).
fn flaps_on_the_wings(entry: &str, file: &str, model: &mesh::Model) {
    let [Some(left), Some(right)] = &model.airbrakes else {
        panic!("{entry} with {file}: both airbrake flaps recorded");
    };
    let (l, r) = (left.hinge.w_axis, right.hinge.w_axis);
    assert!(
        l.x > WING_ROOT && r.x < -WING_ROOT,
        "{entry} with {file}: the hinges are on the wings, left at x {}, right at x {}",
        l.x,
        r.x
    );
    assert!(
        (l.x + r.x).abs() < 1e-4 && (l.y - r.y).abs() < 1e-4 && (l.z - r.z).abs() < 1e-4,
        "{entry} with {file}: the right hinge mirrors the left ({l:?}, {r:?})"
    );
    for (side, flap, outward) in [("left", left, 1.0_f32), ("right", right, -1.0)] {
        let rest = &model.vertices[flap.vertices.start as usize..flap.vertices.end as usize];
        assert!(
            rest.iter().all(|v| v.position[0] * outward > WING_ROOT),
            "{entry} with {file}: the {side} flap lies on its wing"
        );
        let mut swung = Vec::new();
        flap.swung(&model.vertices, 0.4, &mut swung)
            .expect("the flap's vertices are the model's");
        let height = |vs: &[mesh::GpuVertex]| {
            vs.iter().map(|v| v.position[1]).sum::<f32>() / vs.len() as f32
        };
        assert!(
            height(&swung) > height(rest) + 0.02,
            "{entry} with {file}: braking raises the {side} flap"
        );
    }
}

#[test]
#[ignore = "needs data/images/pulse-psp-eu.chd"]
fn the_own_detonator_hull_resolves_on_every_psp_eu_team() {
    check(
        "data/images/pulse-psp-eu.chd",
        &oag_pulse::race::TEAMS,
        "hull.glb",
    );
}

#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn the_own_detonator_hull_resolves_on_every_psp_usa_team() {
    check(
        "data/images/pulse-psp-usa.chd",
        &oag_pulse::race::TEAMS,
        "hull.glb",
    );
}

/// The PS2 disc carries the four pack teams as well.
#[test]
#[ignore = "needs data/images/pulse-ps2-eu.chd"]
fn the_own_detonator_hull_resolves_on_every_ps2_team() {
    let teams: Vec<&str> = oag_pulse::race::TEAMS
        .into_iter()
        .chain(oag_pulse::race::DLC_TEAMS)
        .collect();
    check("data/images/pulse-ps2-eu.chd", &teams, "hull_ps2.glb");
}
