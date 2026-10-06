//! The Vita disc's 2048 craft and Omega's 2048-era craft are two sources
//! Race Remix offers as two CRAFT TITLE entries when both are mounted. This
//! reads all twenty craft from each and says where they differ.
//!
//! **`#[ignore]`d and never run in CI**: needs `data/extracted/vita/PCSF00007`
//! and `data/extracted/ps4`. Run with
//! `OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all -E 'binary(remix_2048_sources_ground_truth)' --no-capture`.

use oag_tables::handling;

struct Craft {
    id: String,
    stats: handling::Stats,
    model: usize,
}

fn read(
    archives: &mut oag_assets::Archives,
    defaults: &oag_title::RaceDefaults,
    team: &str,
) -> Vec<Craft> {
    let table = defaults.team_variants_for(team).expect("a numbered roster");
    table
        .variants
        .iter()
        .map(|variant| {
            let id = table.join.combine(team, variant.suffix);
            let paths = defaults.ships_for(&id);
            let model = archives
                .read_name(&format!(r"{}\{id}\ship.rcsmodel", paths.dir))
                .unwrap_or_else(|e| panic!("{id}: model: {e}"))
                .len();
            let name = handling::entry_name_in(defaults.handling_dir_for(&id), &id);
            let blob = archives
                .read_name(&name)
                .unwrap_or_else(|e| panic!("{name}: {e}"));
            Craft {
                stats: handling::from_blob(&blob).expect("handlingstats parses"),
                id,
                model,
            }
        })
        .collect()
}

fn both() -> Option<(Vec<Craft>, Vec<Craft>)> {
    let vita = oag_testdata::exact("data/extracted/vita/PCSF00007")?;
    let omega = oag_testdata::exact("data/extracted/ps4")?;
    let mut vita = oag_2048::open(&vita.display().to_string()).expect("opening 2048");
    let mut omega = oag_omega::open(&omega.display().to_string()).expect("opening Omega");
    let mut vita_craft = Vec::new();
    let mut omega_craft = Vec::new();
    for team in oag_2048::race::NATIVE_TEAMS {
        vita_craft.extend(read(&mut vita, oag_2048::race::DEFAULTS, team));
        omega_craft.extend(read(&mut omega, oag_omega::race::DEFAULTS, team));
    }
    Some((vita_craft, omega_craft))
}

#[test]
#[ignore = "needs the Vita 2048 and Omega extractions"]
fn census_of_the_two_2048_sources() {
    let Some((vita, omega)) = both() else { return };
    assert_eq!(vita.len(), 20);
    assert_eq!(omega.len(), 20);
    let mut differing = 0;
    for (v, o) in vita.iter().zip(&omega) {
        assert!(v.id.eq_ignore_ascii_case(&o.id), "{} vs {}", v.id, o.id);
        let same = format!("{:?}", v.stats) == format!("{:?}", o.stats);
        differing += usize::from(!same);
        println!(
            "{:<18} stats {} model bytes vita {} omega {}",
            v.id,
            if same { "identical" } else { "DIFFER" },
            v.model,
            o.model
        );
    }
    println!("{differing} of 20 craft carry different handling");
    // Measured 2026-10-06: Omega kept 2048's tuning byte for byte, so a race
    // cannot tell the two sources apart by handling; the models it can.
    assert_eq!(differing, 0, "handling now differs: update the census page");
    assert!(
        vita.iter().zip(&omega).all(|(v, o)| v.model != o.model),
        "a craft model is the same size on both sources"
    );
}
