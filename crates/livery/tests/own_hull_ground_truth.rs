//! Pulse's own Detonator hull resolves on every team's `Zone.vex`: each part a mesh node of the
//! disc's model, each surface a texture of it (`docs/formats/own-hull.md`).
//!
//! **`#[ignore]`d and never run in CI.** It needs game content.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-livery --run-ignored all own_hull_ground_truth::
//! ```
//!
//! The files are read from the official mod `oag.detonator-pulse` (ADR-0098), which carries them:
//! the test fails under `OAG_REQUIRE_GAME_DATA=1` without the official-mods submodule. Each
//! pressing must take its own: the PSP discs `hull.glb`, the PS2 disc, whose `Zone.vex` is
//! another model, `hull_ps2.glb`.

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
