//! Every craft on Omega's own team list starts a race: its model, hull and
//! tuning resolve through `ships_for`/`handling_dir_for`, the 2048-era ones
//! included.
//!
//! **`#[ignore]`d and never run in CI**: needs the extracted Omega packages.
//! Run with
//! `OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-raceplay --run-ignored all -E 'test(/^omega_2048_era_craft_ground_truth::/)'`.
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
    let mut hd_era_variants = 0;
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
            if id != team.id {
                hd_era_variants += usize::from(paths.dir == defaults.ship_dir);
            }
            if paths.dir != defaults.ship_dir {
                era_2048 += 1;
                // Dropping the wiring sends the tuning back under hdships, where it is not.
                let wrong = oag_tables::handling::entry_name_in(defaults.ship_dir, &id);
                assert!(archives.read_name(&wrong).is_err(), "{wrong} exists");
            }
        }
    }
    assert_eq!(era_2048, 20, "five teams of four craft");
    // Twelve HD-era teams, each offered under `_c1` and `_n1` besides its own
    // directory: dropping `team_variants` leaves the three-directory teams
    // with one craft each and fails here.
    assert_eq!(hd_era_variants, 24, "twelve teams of two more directories");
    assert!(matches!(
        oag_omega::race::ERA_2048_VARIANTS.join,
        VariantJoin::Subdirectory
    ));
}

/// The honeycomb of Omega's `Team Selection` opens the models the build can
/// race and rates each from its own `<FE>`: Feisar's `normal`, `concept1` and
/// `nitro` are open with the ratings the disc authors (tenths), and `Tigron_x`,
/// whose models all share one directory, still has one open row. Dropping
/// `team_variants` pads every row.
#[test]
#[ignore = "needs the corrected Omega extraction"]
fn omegas_honeycomb_opens_the_variants_it_races_with_the_discs_own_ratings() {
    let Some(mut archives) = opened() else { return };
    let xml = archives
        .read_name(oag_omega::TITLE.plugin_definition)
        .expect("teams definition");
    let teams = oag_raceplay::catalogue::teams(&String::from_utf8_lossy(&xml));
    let table = oag_omega::race::DEFAULTS
        .team_variants_for("Feisar")
        .expect("Feisar carries the HD-era variants");
    let ids: Vec<&str> = table.variants.iter().map(|v| v.suffix).collect();
    let feisar = teams
        .iter()
        .find(|team| team.id == "Feisar")
        .expect("Feisar");

    let cells = feisar.hex_cells(table.join, ids.iter().copied());
    assert_eq!(cells.len(), 9, "the nine models that author an <FE>");
    assert_eq!(cells.iter().filter(|cell| cell.open).count(), 3);
    let rate = |speed, thrust, handling, shield| {
        Some(oag_ui_screens::picker::hd::Stats {
            speed,
            thrust,
            handling,
            shield,
        })
    };
    // `normal` 7/8/10/8, `concept1` and `nitro` 8/8.5/10/8.
    assert_eq!(
        feisar.variant_stats(table.join, ids.iter().copied()),
        vec![
            rate(70, 80, 100, 80),
            rate(80, 85, 100, 80),
            rate(80, 85, 100, 80)
        ]
    );

    // The two Tigrons: no `_c1`/`_n1` directory to table, one open row each, and it is the
    // model called `normal` (every model of both names one directory), with `normal`'s own
    // ratings: `Tigron_x` 10/7.5/7/7, `Tigron` (the disc's "VanUber") 8.5/9/10/6.5.
    for (id, name, want) in [
        ("Tigron_x", "normal", (100, 75, 70, 70)),
        ("Tigron", "normal", (85, 90, 100, 65)),
    ] {
        let team = teams.iter().find(|t| t.id == id).expect("a Tigron");
        assert!(
            oag_omega::race::DEFAULTS
                .team_variants_for(&team.id)
                .is_none(),
            "{id} has no `_c1`/`_n1` directory to table"
        );
        let own = team.own_directory_variant().expect("its own directory");
        assert_eq!(own.1, name, "{id}: the open model");
        let cells = team.hex_cells(VariantJoin::Suffix, [own.0.as_str()]);
        assert_eq!(cells.iter().filter(|cell| cell.open).count(), 1, "{id}");
        let stats = team.variant_stats(VariantJoin::Suffix, [own.0.as_str()])[0].expect("rated");
        assert_eq!(
            (stats.speed, stats.thrust, stats.handling, stats.shield),
            want,
            "{id}"
        );
    }
}

/// The five 2048-era teams open their four numbered craft on `Team Selection`:
/// the join is the roster's own (`Subdirectory`), not a suffix, so each of the
/// four `1`..`4` finds its model (`Feisar2048\3`, never `Feisar20483`), the four
/// `IsVR` models that share those numbers under `VR_2048` stay padlocks, and every
/// open cell's thumbnail and the team's logo are on the disc. Feisar2048's authored
/// ratings in table order: combat (`1`) 5/8.5/6/8, agility (`2`) 5/8/9/6, speed (`3`)
/// 8/8/7/4, prototype (`4`) 8/8/7/4. Dropping the join gives no open cell.
#[test]
#[ignore = "needs the corrected Omega extraction"]
fn the_2048_era_teams_open_their_four_numbered_craft_with_the_discs_own_ratings() {
    let Some(mut archives) = opened() else { return };
    let xml = archives
        .read_name(oag_omega::TITLE.plugin_definition)
        .expect("teams definition");
    let teams = oag_raceplay::catalogue::teams(&String::from_utf8_lossy(&xml));
    let defaults = oag_omega::race::DEFAULTS;
    let table = oag_omega::race::ERA_2048_VARIANTS;
    let suffixes: Vec<&str> = table.variants.iter().map(|v| v.suffix).collect();
    for id in table.teams {
        let team = teams.iter().find(|team| team.id == *id).expect("declared");
        assert_eq!(defaults.join_for(id), table.join, "{id}");
        assert!(
            defaults.names_no_craft(id),
            "{id}: no craft under the bare id"
        );
        let cells = team.hex_cells(defaults.join_for(id), suffixes.iter().copied());
        assert_eq!(cells.len(), 8, "{id}: four craft and their four VR copies");
        let open: Vec<_> = cells.iter().filter(|cell| cell.open).collect();
        assert_eq!(open.len(), 4, "{id}");
        assert!(cells[4..].iter().all(|cell| !cell.open), "{id}: VR rows");
        for cell in open {
            // The sprite is named `.gtf` as HD's are; Omega's files are `.gnf`.
            let thumb = cell.thumb.as_deref().expect("a thumbnail name");
            let thumb = thumb.replace(".gtf", ".gnf");
            assert!(archives.read_name(&thumb).is_ok(), "{id}: {thumb}");
        }
        let logo = oag_ui_screens::picker::hd::logo_src(&team.location).replace(".gtf", ".gnf");
        assert!(archives.read_name(&logo).is_ok(), "{id}: {logo}");
        assert!(
            team.variant_stats(defaults.join_for(id), suffixes.iter().copied())
                .iter()
                .all(Option::is_some),
            "{id}: every craft is rated"
        );
    }
    // Each table label is the authored name of the model its directory holds.
    for id in table.teams {
        let team = teams.iter().find(|team| team.id == *id).expect("declared");
        for variant in table.variants {
            let directory = table.join.combine(&team.location, variant.suffix);
            let model = team.model_for_directory(&directory).expect("a model");
            assert_eq!(model.name, variant.label, "{id} {}", variant.suffix);
        }
    }
    let feisar = teams
        .iter()
        .find(|team| team.id == "Feisar2048")
        .expect("Feisar2048");
    let rate = |speed, thrust, handling, shield| {
        Some(oag_ui_screens::picker::hd::Stats {
            speed,
            thrust,
            handling,
            shield,
        })
    };
    assert_eq!(
        feisar.variant_stats(table.join, suffixes.iter().copied()),
        vec![
            rate(50, 85, 60, 80),
            rate(50, 80, 90, 60),
            rate(80, 80, 70, 40),
            rate(80, 80, 70, 40),
        ]
    );
    let wrong = feisar.hex_cells(VariantJoin::Suffix, suffixes.iter().copied());
    assert_eq!(wrong.iter().filter(|cell| cell.open).count(), 0);
}
