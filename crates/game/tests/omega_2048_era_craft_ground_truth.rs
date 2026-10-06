//! Every craft on Omega's own team list starts a race: its model, hull and
//! tuning resolve through `ships_for`/`handling_dir_for`, the 2048-era ones
//! included.
//!
//! **`#[ignore]`d and never run in CI**: needs the extracted Omega packages.
//! Run with
//! `OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all -E 'binary(omega_2048_era_craft_ground_truth)'`.
//!
//! The roster is read off the disc's own `Data\plugins\teams\Definition.xml`,
//! not a hand list, so a team the table forgets fails here. A 2048-era craft
//! was once looked up under `hdships\` and no race could start with it.

use oag_title::VariantJoin;

fn opened() -> Option<oag_assets::Archives> {
    let path = oag_testdata::exact("data/extracted/ps4")?;
    Some(oag_omega::open(&path.display().to_string()).expect("opening Omega"))
}

/// Every selectable id on the disc: a team with a variant table is offered once
/// per variant, any other bare.
fn craft_ids(team: &str) -> Vec<String> {
    let defaults = oag_omega::race::DEFAULTS;
    match defaults.team_variants_for(team) {
        Some(table) => table
            .variants
            .iter()
            .map(|variant| table.join.combine(team, variant.suffix))
            .collect(),
        None => vec![team.to_string()],
    }
}

#[test]
#[ignore = "needs the corrected Omega extraction"]
fn every_craft_on_omegas_team_list_resolves_its_model_hull_and_tuning() {
    let Some(mut archives) = opened() else { return };
    let xml = archives
        .read_name(oag_omega::TITLE.plugin_definition)
        .expect("teams definition");
    let teams = oag_raceplay::catalogue::teams(&String::from_utf8_lossy(&xml));
    let defaults = oag_omega::race::DEFAULTS;
    assert!(teams.len() >= 19, "only {} teams read", teams.len());

    let mut era_2048 = 0;
    for team in &teams {
        // The disc names each team's own directory; the table must agree.
        let declared = team
            .location
            .rsplit_once(['\\', '/'])
            .expect("a location")
            .0;
        let first = craft_ids(&team.id).remove(0);
        assert!(
            defaults
                .ships_for(&first)
                .dir
                .eq_ignore_ascii_case(declared),
            "{}: the disc declares {declared}, the title says {}",
            team.id,
            defaults.ships_for(&first).dir
        );
        for id in craft_ids(&team.id) {
            let paths = defaults.ships_for(&id);
            let ship = format!(r"{}\{id}\Ship.vex", paths.dir);
            let model = format!(r"{}\{id}\ship.rcsmodel", paths.dir);
            let stats = oag_tables::handling::entry_name_in(defaults.handling_dir_for(&id), &id);
            for name in [&ship, &model] {
                let blob = archives
                    .read_name(name)
                    .unwrap_or_else(|e| panic!("{id}: reading {name}: {e}"));
                assert!(!blob.is_empty(), "{id}: {name} is empty");
            }
            let blob = archives
                .read_name(&stats)
                .unwrap_or_else(|e| panic!("{id}: reading {stats}: {e}"));
            oag_tables::handling::from_blob(&blob).unwrap_or_else(|e| panic!("{stats}: {e}"));
            if paths.dir != defaults.ship_dir {
                era_2048 += 1;
                // Dropping the wiring sends the tuning back under hdships, where it is not.
                let wrong = oag_tables::handling::entry_name_in(defaults.ship_dir, &id);
                assert!(archives.read_name(&wrong).is_err(), "{wrong} exists");
            }
        }
    }
    assert_eq!(era_2048, 20, "five teams of four craft");
    assert!(matches!(
        oag_omega::race::ERA_2048_VARIANTS.join,
        VariantJoin::Subdirectory
    ));
}
