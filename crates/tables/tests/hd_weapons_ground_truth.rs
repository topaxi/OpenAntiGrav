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
