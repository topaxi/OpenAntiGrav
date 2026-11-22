//! Wipeout HD / Fury's own weapon tables, and Omega's copies of them.
//!
//! `#[ignore]`d because they need `data/images/hdfury-ps3-eu-dec.iso` and, for
//! Omega, `data/extracted/ps4/omega-eu/uroot/data00.psarc`; run by `just
//! test-data`.
//!
//! What this holds, and why it is its own file: `weapons_ground_truth.rs`
//! reads Pulse's PSP disc only, so until this file no test said that an HD
//! race's weapon numbers come off HD's own disc rather than Pulse's. The
//! title axis (`oag_title::weapons::Weapons`) names the entry; this file pins
//! that the name resolves, parses, and is the file we say it is.

use std::path::{Path, PathBuf};

use oag_tables::weapons::{self, Weapon, WeaponStats};

const HD_ARCHIVES: [&str; 3] = ["DATA00", "DATA02", "DATA05"];

fn require(path: PathBuf) -> Option<PathBuf> {
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

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn hd_image() -> Option<PathBuf> {
    require(root().join("data/images/hdfury-ps3-eu-dec.iso"))
}

fn omega_data00() -> Option<PathBuf> {
    require(root().join("data/extracted/ps4/omega-eu/uroot/data00.psarc"))
}

/// One entry out of one HD archive, or `None` when that archive does not ship
/// it - which is itself recorded, because the archives disagree.
fn hd_entry(image: &Path, archive: &str, path: &str) -> Option<Vec<u8>> {
    let spec = format!("{}:PS3_GAME/USRDIR/{archive}.PSARC", image.display());
    let mut psarc = oag_assets::psarc::Archive::open(&spec).expect("the archive opens");
    psarc.read_path(path).ok()
}

fn omega_entry(archive: &Path, path: &str) -> Option<Vec<u8>> {
    let mut psarc = oag_assets::psarc::Archive::open_file(archive).expect("the archive opens");
    psarc.read_path(path).ok()
}

fn parse(label: &str, blob: &[u8]) -> WeaponStats {
    weapons::from_blob(blob).unwrap_or_else(|e| panic!("{label}: {e}"))
}

/// The weapons a `<Pickupodds>` class authors only zeros for.
fn zero_odds(stats: &WeaponStats, class: &str) -> Vec<Weapon> {
    let table = stats
        .pickups_for(class)
        .unwrap_or_else(|| panic!("no {class} pickup table"));
    table
        .odds
        .iter()
        .filter(|(_, o)| o.ai == 0.0 && o.back == 0.0 && o.front == 0.0 && o.human == 0.0)
        .map(|(w, _)| *w)
        .collect()
}

#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn every_shipped_hd_table_decodes_with_the_pulse_roster() {
    let Some(image) = hd_image() else { return };
    let mut seen = 0;
    for archive in HD_ARCHIVES {
        for entry in [
            "/data/xml/weaponstats_race.xml",
            "/data/xml/weaponstats_elimination.xml",
            "/data/xml/weaponstats_detonator.xml",
        ] {
            let Some(blob) = hd_entry(&image, archive, entry) else {
                continue;
            };
            seen += 1;
            let label = format!("{archive}:{entry}");
            let stats = parse(&label, &blob);
            assert!(
                stats.skipped.is_empty(),
                "{label} skipped {:?}",
                stats.skipped
            );
            // Detonator's own roster: a Mine, a Cannon and a Bomb decode; its
            // `EMP` block carries no `absorb` and no Pulse class names it.
            let roster = if entry.contains("detonator") { 3 } else { 13 };
            assert_eq!(
                stats.absorb.len(),
                roster,
                "{label}: weapons carrying absorb"
            );
            assert_eq!(stats.pickups.len(), 4, "{label}: four speed classes");
            assert_eq!(stats.slowdown_limit.to_bits(), 1.3_f32.to_bits(), "{label}");
        }
    }
    assert_eq!(seen, 7, "race x3, elimination x2, detonator x2 ship");
}

/// Reading `Archives::read_name` is what a race does, so this is the pin that
/// an HD race's table is the `DATA00` copy and nothing else.
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn an_hd_race_reads_the_data00_copy_through_the_title_axis() {
    let Some(image) = hd_image() else { return };
    let mut archives = oag_hd::open(&image.display().to_string()).expect("HD opens");
    let weapons = oag_hd::TITLE.weapons;
    for (entry, path) in [
        (weapons.race, "/data/xml/weaponstats_race.xml"),
        (
            weapons.elimination.expect("HD names an Eliminator table"),
            "/data/xml/weaponstats_elimination.xml",
        ),
    ] {
        let served = archives.read_name(entry).expect("the name resolves");
        let fury = hd_entry(&image, "DATA00", path).expect("DATA00 ships it");
        assert_eq!(served, fury, "{entry} is served from DATA00");
        parse(entry, &served);
    }
    // The base game's copy is a different file, so the precedence matters.
    let base = hd_entry(&image, "DATA02", "/data/xml/weaponstats_race.xml").expect("DATA02");
    let fury = hd_entry(&image, "DATA00", "/data/xml/weaponstats_race.xml").expect("DATA00");
    assert_ne!(base, fury);
}

/// The three race copies agree on everything but the handful of fields a
/// later build retuned; this pins the ones a race spends.
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn the_race_copies_differ_where_fury_retuned_them() {
    let Some(image) = hd_image() else { return };
    let get = |a: &str| {
        parse(
            a,
            &hd_entry(&image, a, "/data/xml/weaponstats_race.xml").expect("ships"),
        )
    };
    let (fury, base, patch) = (get("DATA00"), get("DATA02"), get("DATA05"));

    let leach = |s: &WeaponStats| s.leach_beam().expect("LeachBeam decodes");
    assert_eq!(leach(&fury).slow_ship_factor.to_bits(), 0.4_f32.to_bits());
    assert_eq!(leach(&base).slow_ship_factor.to_bits(), 0.8_f32.to_bits());
    assert_eq!(leach(&patch).slow_ship_factor.to_bits(), 0.4_f32.to_bits());
    assert!(leach(&fury).damage > leach(&base).damage);

    // Everything the three share: the projectile weapons.
    assert_eq!(fury.rocket(), base.rocket());
    assert_eq!(fury.rocket(), patch.rocket());
    assert_eq!(fury.missile(), base.missile());
    assert_eq!(fury.plasma(), base.plasma());
    assert_eq!(fury.quake(), base.quake());
    assert_eq!(fury.repulser(), base.repulser());
    assert_eq!(fury.shuriken(), base.shuriken());
    assert_eq!(fury.pickups, base.pickups);
}

/// Per-mode gating, the way Pulse has it: a race table zeroes the Eliminator's
/// weapons out and the Eliminator table zeroes the race's.
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn the_race_table_gates_shuriken_and_repulser_off_and_the_eliminator_does_not() {
    let Some(image) = hd_image() else { return };
    let race = parse(
        "race",
        &hd_entry(&image, "DATA00", "/data/xml/weaponstats_race.xml").expect("race"),
    );
    let elim = parse(
        "elimination",
        &hd_entry(&image, "DATA00", "/data/xml/weaponstats_elimination.xml").expect("elim"),
    );
    for class in ["Venom", "Flash", "Rapier", "Phantom"] {
        assert_eq!(
            zero_odds(&race, class),
            vec![Weapon::Repulser, Weapon::Shuriken],
            "race {class}"
        );
        let elim_zero = zero_odds(&elim, class);
        assert!(
            !elim_zero.contains(&Weapon::Repulser) && !elim_zero.contains(&Weapon::Shuriken),
            "Eliminator {class} hands both out; zero: {elim_zero:?}"
        );
    }
}

/// Omega ships HD's own file names in `data00.psarc`, and the reader holds.
#[test]
#[ignore = "needs data/extracted/ps4/omega-eu/uroot/data00.psarc"]
fn omega_ships_hds_weapon_tables_and_the_reader_holds() {
    let (Some(omega), Some(image)) = (omega_data00(), hd_image()) else {
        return;
    };
    for name in [
        "weaponstats_race",
        "weaponstats_elimination",
        "weaponstats_detonator",
    ] {
        let path = format!("/data/xml/{name}.xml");
        let blob = omega_entry(&omega, &path).unwrap_or_else(|| panic!("Omega ships {path}"));
        let stats = parse(&format!("omega {name}"), &blob);
        assert!(
            stats.skipped.is_empty(),
            "omega {name}: {:?}",
            stats.skipped
        );
        assert_eq!(
            stats.absorb.len(),
            if name.ends_with("detonator") { 3 } else { 13 }
        );
        let classes: Vec<&str> = stats.pickups.iter().map(|t| t.class.as_str()).collect();
        assert_eq!(
            classes,
            ["Venom", "Flash", "Rapier", "Phantom", "SuperPhantom"],
            "omega {name}: HD's four classes plus the fifth Omega adds"
        );
        let hd = hd_entry(&image, "DATA00", &path).expect("DATA00");
        assert_ne!(blob, hd, "omega {name} is not byte-identical to Fury's");
        let h = parse("hd", &hd);
        assert_eq!(stats.rocket(), h.rocket(), "{name}");
        assert_eq!(stats.missile(), h.missile(), "{name}");
        assert_eq!(stats.plasma(), h.plasma(), "{name}");
        assert_eq!(stats.mine(), h.mine(), "{name}");
        assert_eq!(stats.bomb(), h.bomb(), "{name}");
        assert_eq!(stats.cannon(), h.cannon(), "{name}");
        assert_eq!(stats.quake(), h.quake(), "{name}");
        assert_eq!(stats.repulser(), h.repulser(), "{name}");
        assert_eq!(stats.leach_beam(), h.leach_beam(), "{name}");
        assert_eq!(stats.shuriken(), h.shuriken(), "{name}");
        assert_eq!(stats.slowdown_limit.to_bits(), h.slowdown_limit.to_bits());
        assert_eq!(stats.pickups[..4], h.pickups[..], "{name}: the shared four");
    }
    // The two 2048 tables are named so, and are 2048's own, not HD's.
    for name in ["weaponstats_race_2048", "weaponstats_elimination_2048"] {
        let path = format!("/data/xml/{name}.xml");
        let blob = omega_entry(&omega, &path).unwrap_or_else(|| panic!("Omega ships {path}"));
        let stats = parse(&format!("omega {name}"), &blob);
        println!(
            "omega {name}: {} weapons, skipped {:?}",
            stats.absorb.len(),
            stats.skipped
        );
    }
}

/// Every Detonator-only attribute a title's `weaponstats_detonator.xml` authors is decoded,
/// present and finite; `DATA05`'s copy authors none and reports none.
fn assert_full_detonator_table(label: &str, stats: &WeaponStats) {
    let d = stats
        .detonator()
        .unwrap_or_else(|| panic!("{label}: no Detonator attributes decoded"));
    let emp = d.emp.unwrap_or_else(|| panic!("{label}: no EMP block"));
    let fields = [
        d.energy_recharge_per_stage,
        d.mine.points_per_metre,
        d.mine.max_distance,
        d.mine.velocity_reduction,
        d.cannon.num_rebounds,
        d.cannon.round_recharge_time,
        d.cannon.recharge_pause,
        d.bomb.number_of_shots_to_destroy,
        d.bomb.energy_recharge_per_shot,
        d.bomb.points_for_cannon_bullet_hit,
        d.bomb.points_for_destroying,
        emp.wave_time,
        emp.speed,
        emp.min_num_pads_to_collect,
        emp.max_num_pads_to_collect,
        emp.min_distance,
        emp.max_distance,
    ];
    for (i, f) in fields.iter().enumerate() {
        assert!(
            f.is_some_and(f32::is_finite),
            "{label}: Detonator field {i} is {f:?}"
        );
    }
    // The EMP block carries no `absorb`: it stays out of the weapon roster.
    assert_eq!(stats.absorb.len(), 3, "{label}: Mine, Cannon, Bomb");
}

#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn hds_detonator_attributes_decode_from_fury_and_not_from_the_older_copy() {
    let Some(image) = hd_image() else { return };
    let entry = "/data/xml/weaponstats_detonator.xml";
    let fury = parse(
        "DATA00",
        &hd_entry(&image, "DATA00", entry).expect("DATA00"),
    );
    assert_full_detonator_table("DATA00", &fury);
    let older = parse(
        "DATA05",
        &hd_entry(&image, "DATA05", entry).expect("DATA05"),
    );
    assert_eq!(older.detonator(), None, "DATA05 authors none of them");
    // The race table authors the Bomb's shot count and no more of them.
    let race = parse(
        "DATA00 race",
        &hd_entry(&image, "DATA00", "/data/xml/weaponstats_race.xml").expect("race"),
    );
    let race = race
        .detonator()
        .expect("the race Bomb authors a shot count");
    assert_eq!(race.bomb.number_of_shots_to_destroy, Some(1.0));
    assert_eq!(race.emp, None);
    assert_eq!(race.mine, Default::default());
    // The title axis names the entry and serves DATA00's bytes.
    let mut archives = oag_hd::open(&image.display().to_string()).expect("HD opens");
    let named = oag_hd::TITLE.weapons.detonator.expect("HD names it");
    assert_eq!(
        archives.read_name(named).expect("the name resolves"),
        hd_entry(&image, "DATA00", entry).unwrap()
    );
}

#[test]
#[ignore = "needs data/extracted/ps4/omega-eu/uroot/data00.psarc"]
fn omegas_detonator_attributes_decode() {
    let Some(omega) = omega_data00() else { return };
    let blob = omega_entry(&omega, "/data/xml/weaponstats_detonator.xml").expect("Omega ships it");
    assert_full_detonator_table("omega", &parse("omega", &blob));
}

/// 2048's Detonator table is in its v1.04 patch, under its own spelling.
#[test]
#[ignore = "needs data/extracted/vita/PCSF00007/patch-v104/PSP2/data1.psarc"]
fn the_2048_detonator_table_decodes() {
    let path = root().join("data/extracted/vita/PCSF00007/patch-v104/PSP2/data1.psarc");
    let Some(path) = require(path) else { return };
    let mut psarc = oag_assets::psarc::Archive::open_file(&path).expect("the archive opens");
    let blob = psarc
        .read_path("/Data/XML/weaponstats_Detonator.xml")
        .or_else(|_| psarc.read_path("/data/xml/weaponstats_detonator.xml"))
        .expect("2048's patch ships it");
    let stats = parse("2048", &blob);
    assert!(stats.skipped.is_empty(), "{:?}", stats.skipped);
    assert_full_detonator_table("2048", &stats);
}

// Detonator's layout files (`oag_tables::detonator_layout`), read here beside the weapon tables
// because the same three archives are opened for both.

/// What the disc's `PatternFile.xml` and `GamePlayValues.xml` hold, as a reader sees them.
fn assert_detonator_layouts(label: &str, patterns: &[u8], values: &[u8]) {
    use oag_tables::detonator_layout::{parse_patterns, parse_values};
    let file = parse_patterns(&String::from_utf8_lossy(patterns));
    assert_eq!(file.patterns.len(), 18, "{label}: groups");
    let clusters: usize = file.patterns.iter().map(|p| p.clusters.len()).sum();
    assert_eq!(clusters, 79, "{label}: clusters");
    assert!(
        file.patterns
            .iter()
            .flat_map(|p| &p.clusters)
            .all(|c| c.kind == 0 && c.object_types == [0; 10]),
        "{label}: every cluster lays mines"
    );
    assert!(
        file.patterns.iter().any(|p| p.stage_to_appear_from == 14),
        "{label}: a pattern appears from the last stage"
    );
    let v = parse_values(&String::from_utf8_lossy(values));
    let counts: Vec<i32> = (1..=14).map(|s| v.mine_count(s).unwrap()).collect();
    assert_eq!(
        counts,
        [
            80, 90, 95, 100, 105, 110, 120, 130, 140, 150, 170, 180, 190, 200
        ],
        "{label}: the file overwrites the static counts"
    );
    assert!(v.mine_counts_represent_max_total_to_appear, "{label}");
    assert_eq!(v.energy_recharge_per_stage, 10, "{label}");
    assert_eq!(v.num_steps_to_ignore_after_jump, 20, "{label}");
    assert_eq!(v.distance_offset_between_mines, 3.5, "{label}");
}

#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn hds_detonator_layouts_decode_through_the_title_axis() {
    let Some(image) = hd_image() else { return };
    let layouts = oag_hd::race::DETONATOR.layouts.expect("HD names them");
    let mut archives = oag_hd::open(&image.display().to_string()).expect("HD opens");
    let patterns = archives
        .read_name(layouts.pattern_file)
        .expect("PatternFile");
    let values = archives
        .read_name(layouts.gameplay_values)
        .expect("GamePlayValues");
    assert_detonator_layouts("HD", &patterns, &values);
}

#[test]
#[ignore = "needs data/extracted/ps4/omega-eu/uroot/data00.psarc"]
fn omegas_detonator_layouts_are_hds_up_to_line_endings() {
    let (Some(omega), Some(image)) = (omega_data00(), hd_image()) else {
        return;
    };
    let path = |name: &str| format!("/data/xml/detonatorlayouts/{name}.xml");
    let patterns = omega_entry(&omega, &path("patternfile")).expect("Omega ships it");
    let values = omega_entry(&omega, &path("gameplayvalues")).expect("Omega ships it");
    assert_detonator_layouts("omega", &patterns, &values);
    let hd = hd_entry(&image, "DATA00", &path("patternfile")).expect("DATA00");
    let a = oag_tables::detonator_layout::parse_patterns(&String::from_utf8_lossy(&patterns));
    let b = oag_tables::detonator_layout::parse_patterns(&String::from_utf8_lossy(&hd));
    assert_eq!(a, b, "the same groups once line endings are dropped");
}

#[test]
#[ignore = "needs data/extracted/vita/PCSF00007/base/PSP2/data.psarc"]
fn the_2048_detonator_layouts_decode() {
    let path = root().join("data/extracted/vita/PCSF00007/base/PSP2/data.psarc");
    let Some(path) = require(path) else { return };
    let mut psarc = oag_assets::psarc::Archive::open_file(&path).expect("the archive opens");
    let mut read = |name: &str| {
        psarc
            .read_path(&format!("/data/xml/detonatorlayouts/{name}.xml"))
            .or_else(|_| psarc.read_path(&format!("/Data/XML/DetonatorLayouts/{name}.xml")))
            .expect("2048 ships it")
    };
    let patterns = read("PatternFile");
    let values = read("GamePlayValues");
    assert_detonator_layouts("2048", &patterns, &values);
}
