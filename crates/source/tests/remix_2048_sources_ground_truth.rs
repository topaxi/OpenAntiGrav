//! The Vita disc's 2048 craft and Omega's 2048-era craft are two sources
//! Race Remix offers as two CRAFT TITLE entries when both are mounted. This
//! reads all twenty craft from each and says where they differ.
//!
//! **`#[ignore]`d and never run in CI**: needs `data/extracted/vita/PCSF00007`
//! and `data/extracted/ps4`. Run with
//! `OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-source --run-ignored all -E 'test(/^remix_2048_sources_ground_truth::/)' --no-capture`.

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

/// Each native craft's own rocket and mine counts, off its `<Weapons>` line, on both sources
/// (`docs/ghidra/functions/ps4-omega-eu/weapon-counts.md`). Roles are the numbered folder:
/// 1 Fighter (no count authored, so the executable's default 3 and 5), 2 Agility, 3 Speed,
/// 4 Prototype.
#[test]
#[ignore = "needs the Vita 2048 and Omega extractions"]
fn rocket_and_mine_counts_per_craft() {
    use oag_weapons::counts::WeaponCounts;
    let Some((vita, omega)) = both() else { return };
    let counts = |craft: &Craft| WeaponCounts::from_stats(&craft.stats);
    let named = |rockets, mines| WeaponCounts { rockets, mines };
    for (source, craft) in [("2048", &vita), ("Omega", &omega)] {
        assert_eq!(craft.len(), 20);
        let mut prototypes = Vec::new();
        for (i, craft) in craft.iter().enumerate() {
            let got = counts(craft);
            match i % 4 {
                0 => assert_eq!(got, named(3, 5), "{source} {} (Fighter)", craft.id),
                1 => assert_eq!(got, named(2, 4), "{source} {} (Agility)", craft.id),
                2 => assert_eq!(got, named(1, 3), "{source} {} (Speed)", craft.id),
                _ => prototypes.push((craft.id.clone(), got)),
            }
        }
        println!("{source} prototypes: {prototypes:?}");
        let by_team: Vec<_> = prototypes
            .iter()
            .map(|(id, counts)| (id.to_ascii_lowercase(), counts.rockets, counts.mines))
            .collect();
        assert_eq!(
            by_team,
            [
                ("ag_systems2048\\4".to_string(), 2, 4),
                ("auricom2048\\4".to_string(), 3, 5),
                ("feisar2048\\4".to_string(), 1, 3),
                ("piranha2048\\4".to_string(), 1, 3),
                ("qirex2048\\4".to_string(), 3, 5),
            ],
            "{source} Prototypes"
        );
    }
    let omega_counts: Vec<_> = omega.iter().map(counts).collect();
    let vita_counts: Vec<_> = vita.iter().map(counts).collect();
    assert_eq!(omega_counts, vita_counts, "Omega's counts equal 2048's");
}

/// Pulse, Pure and HD author neither count (their executables carry neither attribute string),
/// so every team's file must parse to no `<Weapons>` line: the one place a stray authored line
/// would switch the count on for a title whose executable never read it.
#[test]
#[ignore = "needs the Pulse, Pure and HD images"]
fn the_other_titles_author_no_weapon_counts() {
    fn census(
        what: &str,
        mut archives: oag_assets::Archives,
        defaults: &oag_title::RaceDefaults,
        teams: &[&str],
    ) {
        for team in teams {
            let name = handling::entry_name_in(defaults.handling_dir_for(team), team);
            let blob = archives
                .read_name(&name)
                .unwrap_or_else(|e| panic!("{what} {name}: {e}"));
            let stats = handling::from_blob(&blob).unwrap_or_else(|e| panic!("{what} {name}: {e}"));
            assert_eq!(stats.weapons, None, "{what} {name} authors a weapon count");
            println!("{what}: {name} authors none");
        }
    }
    let ids = |roster: oag_title::team::Roster| roster.iter().map(|t| t.id).collect::<Vec<_>>();
    if let Some(image) = oag_testdata::exact("pulse-psp-eu.chd") {
        let archives = oag_pulse::open(&image.display().to_string()).expect("Pulse");
        let defaults = oag_pulse::race::DEFAULTS;
        census("Pulse", archives, defaults, &[defaults.team]);
    }
    if let Some(image) = oag_testdata::exact("pure-psp-eu.chd") {
        let archives = oag_pure::open(&image.display().to_string()).expect("Pure");
        census(
            "Pure",
            archives,
            oag_pure::race::DEFAULTS,
            &ids(oag_pure::race::TEAM_IDENTITIES),
        );
    }
    if let Some(image) = oag_testdata::exact("hdfury-ps3-eu-dec.iso") {
        let archives = oag_hd::open(&image.display().to_string()).expect("HD");
        census(
            "HD",
            archives,
            oag_hd::race::DEFAULTS,
            &ids(oag_hd::race::TEAM_IDENTITIES),
        );
    }
}
