//! Zone flies one shared hull on 2048 v1.04 and the Omega Collection, whatever
//! craft the player picked.
//!
//! **`#[ignore]`d and never run in CI**: needs the extracted packages under
//! `data/extracted/`. Run with
//! `OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-raceplay --run-ignored all -E 'test(/^zone_craft_ground_truth::/)'`.
//!
//! The law is [`oag_title::ZoneCraft::OwnShipAt`], read off both executables'
//! ship-model loader (`docs/ghidra/functions/vita-2048-eu-v104/zone-craft.md`,
//! `docs/ghidra/functions/ps4-omega-eu/zone-craft.md`): game mode 6 names
//! `Data\art\published\hdships\Zone\Ship.vex` without looking at the craft.
//! Each case asserts both halves, because either alone passes on a build that
//! fell back to the other: a Zone race does not name the race hull, and the
//! name it does build is in the archive set and not empty.

use oag_race::Mode;

const ZONE_HULL: &str = r"Data\art\published\hdships\Zone\Ship.vex";

fn check(
    title: &str,
    defaults: &oag_title::RaceDefaults,
    archives: &mut oag_assets::Archives,
    teams: &[&str],
) {
    for team in teams {
        let paths = defaults.ships_for(team);
        let racing = paths.hull_entry_name(team, None, None);
        let zoning = paths.hull_entry_name(team, Some(paths.zone), None);
        assert_ne!(
            racing, zoning,
            "{title}/{team}: Zone must not fly the race hull"
        );
        assert_eq!(
            zoning, ZONE_HULL,
            "{title}/{team}: every craft flies one Zone hull"
        );
        let blob = archives
            .read_name(&zoning)
            .unwrap_or_else(|e| panic!("{title}/{team}: reading {zoning}: {e}"));
        assert!(!blob.is_empty(), "{title}/{team}: {zoning} is empty");
    }
}

#[test]
#[ignore = "needs the decrypted Vita package in data/extracted/vita/"]
fn vita_2048_zone_hull_is_shared_by_native_and_guest_craft() {
    let Some(path) = oag_testdata::exact("data/extracted/vita/PCSF00007") else {
        return;
    };
    let mut archives = oag_2048::open(&path.display().to_string()).expect("opening 2048");
    check(
        "2048",
        oag_2048::race::DEFAULTS,
        &mut archives,
        &[
            r"feisar2048\1",
            r"auricom2048\4",
            oag_2048::race::DEFAULT_TEAM,
            "Assegai",
        ],
    );
}

#[test]
#[ignore = "needs the corrected Omega extraction"]
fn omega_zone_hull_is_shared_by_hd_and_2048_era_craft() {
    let Some(path) = oag_testdata::exact("data/extracted/ps4") else {
        return;
    };
    let mut archives = oag_omega::open(&path.display().to_string()).expect("opening Omega");
    check(
        "Omega",
        oag_omega::race::DEFAULTS,
        &mut archives,
        &[oag_omega::race::DEFAULTS.team, "assegai"],
    );
}

/// The Zone livery is read off the definition and put on the hull: each
/// HD-era team's own `zone` model names a `Zoneship_<x>\Team.gnf` that is on
/// the disc, the swap reaches the hull's texture request, and a 2048-era team
/// (which authors none) keeps the default and says so.
#[test]
#[ignore = "needs the corrected Omega extraction"]
fn omega_zone_hull_wears_the_livery_its_definition_names() {
    let Some(path) = oag_testdata::exact("data/extracted/ps4") else {
        return;
    };
    let mut archives = oag_omega::open(&path.display().to_string()).expect("opening Omega");
    let xml = archives
        .read_name(oag_omega::TITLE.plugin_definition)
        .expect("teams definition");
    let teams = oag_raceplay::catalogue::teams(&String::from_utf8_lossy(&xml));
    let liveries: Vec<(String, String)> = teams
        .iter()
        .filter_map(|team| Some((team.id.clone(), team.zone_livery.clone()?)))
        .collect();
    // Twelve teams name a `Zoneship_<x>` directory and all of them are on the
    // disc; `Tigron` and `VanUber` name `livery1`, an HD paint name with no
    // Zone directory, so the swap finds nothing and the hull keeps its default.
    let named: Vec<_> = liveries
        .iter()
        .filter(|(_, livery)| livery.starts_with("zoneship_"))
        .collect();
    assert_eq!(named.len(), 12, "{liveries:?}");
    for (team, livery) in named {
        let name = format!(r"Data\art\published\hdships\Zone\{livery}\Team.gnf");
        assert!(
            archives.read_name(&name).is_ok_and(|blob| !blob.is_empty()),
            "{team}: {name} is not on the disc"
        );
    }
    let load = |archives: &mut oag_assets::Archives, team: &str| {
        let mut report = Vec::new();
        oag_livery::load(
            archives,
            &[team.to_string()],
            &oag_livery::LoadContext {
                store: Default::default(),
                race: oag_omega::TITLE.race,
                mode: Mode::Zone,
                flare: oag_omega::TITLE.flare,
                hull_overlay: false,
                leach_overlay: false,
                hull_shine: false,
                hull_wreck: false,
                absorb_shell: false,
                own_shield: false,
                zone_liveries: &liveries,
                forced_skins: &[],
                circuit_hull: None,
                player_livery: None,
            },
            None,
            None,
            &[],
            &mut report,
        )
        .expect("the Zone hull loads");
        report.join("\n")
    };
    let swapped = load(&mut archives, "Qirex");
    assert!(
        swapped.contains("Zone livery zoneship_quirex replaces zoneship_team"),
        "{swapped}"
    );
    let absent = load(&mut archives, "Tigron");
    assert!(
        absent.contains("which the archive does not hold"),
        "{absent}"
    );
    let kept = load(&mut archives, r"Feisar2048\3");
    assert!(kept.contains("authors no Zone livery"), "{kept}");
}

/// HD/Fury's Zone hull wears the picked team's livery the way 2048's does: its
/// texture requests all sit under `data/ships/zone/zoneship_zone/`, and the ship
/// loader's mode-6 case swaps that component for the craft's `zone` model
/// `texturelocation` (`docs/ghidra/functions/ps3-hdfury-eu/zone-craft.md`). Without the
/// swap the hull reads the default `zoneship_zone` set, which ships only four of
/// the nine textures it asks for. The report line names the swap and the
/// absence of "does not hold" says every swapped texture was on the disc.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn hd_zone_hull_wears_the_picked_teams_livery() {
    let Some(path) = oag_testdata::image("data/images/hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let mut archives =
        oag_source::title::open_source(&path.display().to_string(), Vec::new(), Vec::new())
            .expect("opening HD/Fury")
            .archives;
    assert_eq!(
        oag_hd::race::DEFAULTS.zone_craft.livery_key(),
        Some("zoneship_zone"),
        "HD's Zone hull takes a livery"
    );
    let blob = archives
        .read_name(oag_hd::TITLE.plugin_definition)
        .expect("teams definition");
    let xml = oag_tables::fexml::text(&blob).expect("the definition reads");
    let liveries: Vec<(String, String)> = oag_raceplay::catalogue::teams(&xml)
        .iter()
        .filter_map(|team| Some((team.id.clone(), team.zone_livery.clone()?)))
        .collect();
    assert_eq!(liveries.len(), 12, "{liveries:?}");
    for (team, livery) in &liveries {
        let name = format!(r"Data\Ships\Zone\{livery}\team.gtf");
        assert!(
            archives.read_name(&name).is_ok_and(|blob| !blob.is_empty()),
            "{team}: {name} is not on the disc"
        );
    }
    let mut load = |team: &str| {
        let mut report = Vec::new();
        oag_livery::load(
            &mut archives,
            &[team.to_string()],
            &oag_livery::LoadContext {
                store: Default::default(),
                race: oag_hd::TITLE.race,
                mode: Mode::Zone,
                flare: oag_hd::TITLE.flare,
                hull_overlay: false,
                leach_overlay: false,
                hull_shine: false,
                hull_wreck: false,
                absorb_shell: false,
                own_shield: false,
                zone_liveries: &liveries,
                forced_skins: &[],
                circuit_hull: None,
                player_livery: None,
            },
            None,
            None,
            &[],
            &mut report,
        )
        .expect("the Zone hull loads");
        report.join("\n")
    };
    for (team, livery) in [
        ("Qirex", "zoneship_quirex"),
        ("AG_Systems", "zoneship_ag_sys"),
        ("Feisar", "zoneship_faisar"),
        ("Icaras", "zoneship_icaras"),
    ] {
        let report = load(team);
        assert!(
            report.contains(&format!(
                "{team}: Zone livery {livery} replaces zoneship_zone on "
            )),
            "{report}"
        );
        assert!(!report.contains("does not hold"), "{report}");
    }
}

/// 2048 v1.04 carries the same twelve HD-era Zone liveries as HD and Omega, under
/// HD's key `zoneship_zone` (`docs/ghidra/functions/vita-2048-eu-v104/zone-craft.md`):
/// an HD-era team's swap reaches the hull's textures and finds them all, and a
/// native 2048 craft authors none and keeps the default.
#[test]
#[ignore = "needs the decrypted Vita package in data/extracted/vita/"]
fn vita_2048_zone_hull_wears_the_livery_its_definition_names() {
    let Some(path) = oag_testdata::exact("data/extracted/vita/PCSF00007") else {
        return;
    };
    let mut archives = oag_2048::open(&path.display().to_string()).expect("opening 2048");
    let blob = archives
        .read_name(oag_2048::TITLE.plugin_definition)
        .expect("teams definition");
    let xml = oag_tables::fexml::text(&blob).expect("the definition reads");
    let liveries: Vec<(String, String)> = oag_raceplay::catalogue::teams(&xml)
        .iter()
        .filter_map(|team| Some((team.id.clone(), team.zone_livery.clone()?)))
        .collect();
    assert_eq!(liveries.len(), 12, "{liveries:?}");
    let mut load = |team: &str| {
        let mut report = Vec::new();
        oag_livery::load(
            &mut archives,
            &[team.to_string()],
            &oag_livery::LoadContext {
                store: Default::default(),
                race: oag_2048::TITLE.race,
                mode: Mode::Zone,
                flare: oag_2048::TITLE.flare,
                hull_overlay: false,
                leach_overlay: false,
                hull_shine: false,
                hull_wreck: false,
                absorb_shell: false,
                own_shield: false,
                zone_liveries: &liveries,
                forced_skins: &[],
                circuit_hull: None,
                player_livery: None,
            },
            None,
            None,
            &[],
            &mut report,
        )
        .expect("the Zone hull loads");
        report.join("\n")
    };
    for (team, livery) in [
        ("Qirex", "zoneship_quirex"),
        ("AG_Systems", "zoneship_ag_sys"),
    ] {
        let report = load(team);
        assert!(
            report.contains(&format!(
                "{team}: Zone livery {livery} replaces zoneship_zone on "
            )),
            "{report}"
        );
        assert!(!report.contains("does not hold"), "{report}");
    }
    let kept = load(r"feisar2048\1");
    assert!(kept.contains("authors no Zone livery"), "{kept}");
}

/// Builds `team`'s Zone wreck through the swap the Zone hull wears and returns the loader report.
fn zone_wreck_report(
    archives: &mut oag_assets::Archives,
    defaults: &oag_title::RaceDefaults,
    liveries: &[(String, String)],
    team: &str,
) -> String {
    let ships = defaults.ships_for(team);
    let mut report = Vec::new();
    let folder = oag_livery::zone_livery::swap(ships, Mode::Zone, liveries, team, &mut report);
    let wreck = oag_livery::wreck::load(
        archives,
        team,
        ships,
        Mode::Zone,
        Default::default(),
        folder,
        &mut report,
    );
    assert!(wreck.is_some(), "{team}: the Zone wreck builds: {report:?}");
    report.join("\n")
}

fn zone_liveries(archives: &mut oag_assets::Archives, definition: &str) -> Vec<(String, String)> {
    let blob = archives.read_name(definition).expect("teams definition");
    let xml = oag_tables::fexml::text(&blob).expect("the definition reads");
    oag_raceplay::catalogue::teams(&xml)
        .iter()
        .filter_map(|team| Some((team.id.clone(), team.zone_livery.clone()?)))
        .collect()
}

/// The wreck's own line, not the hull's: it names the wreck entry and counts a swap.
fn assert_wreck_swapped(report: &str, entry: &str, team: &str, livery: &str, key: &str) {
    let line = report
        .lines()
        .find(|line| line.starts_with(&format!("{entry}: ")))
        .unwrap_or_else(|| panic!("no wreck line for {entry}:\n{report}"));
    let want = format!("{entry}: {team}: Zone livery {livery} replaces {key} on ");
    assert!(line.starts_with(&want), "{report}");
    assert!(!line.contains("on 0 texture"), "{report}");
    assert!(!report.contains("does not hold"), "{report}");
}

/// A Zone wreck reads its textures through the same swap as the Zone hull: the original's
/// substitution table is global (`docs/ghidra/functions/ps3-hdfury-eu/zone-craft.md`, "Open").
#[test]
#[ignore = "needs a disc image in data/images/"]
fn hd_zone_wreck_wears_the_picked_teams_livery() {
    let Some(path) = oag_testdata::image("data/images/hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let mut archives =
        oag_source::title::open_source(&path.display().to_string(), Vec::new(), Vec::new())
            .expect("opening HD/Fury")
            .archives;
    let liveries = zone_liveries(&mut archives, oag_hd::TITLE.plugin_definition);
    let report = zone_wreck_report(&mut archives, oag_hd::race::DEFAULTS, &liveries, "Qirex");
    assert_wreck_swapped(
        &report,
        r"Data\Ships\zone\shipwreck.vex",
        "Qirex",
        "zoneship_quirex",
        "zoneship_zone",
    );
}

#[test]
#[ignore = "needs the decrypted Vita package in data/extracted/vita/"]
fn vita_2048_zone_wreck_wears_the_picked_teams_livery() {
    let Some(path) = oag_testdata::exact("data/extracted/vita/PCSF00007") else {
        return;
    };
    let mut archives = oag_2048::open(&path.display().to_string()).expect("opening 2048");
    let liveries = zone_liveries(&mut archives, oag_2048::TITLE.plugin_definition);
    let report = zone_wreck_report(&mut archives, oag_2048::race::DEFAULTS, &liveries, "Qirex");
    assert_wreck_swapped(
        &report,
        r"Data\art\published\hdships\Zone\shipwreck.vex",
        "Qirex",
        "zoneship_quirex",
        "zoneship_zone",
    );
}

/// Omega: checked, differs. Its Zone wreck asks for `zoneship_zone\zone_tp_wrecked.gnf` and
/// `zone_glass.gnf`, not for the `zoneship_team` component its hull asks for and the loader's
/// pair replaces, so the hull's key rewrites no wreck request and the wreck keeps the default
/// skin. The swap is still wired, so a wreck whose paths did carry the key would be repainted.
#[test]
#[ignore = "needs the corrected Omega extraction"]
fn omega_zone_wreck_asks_for_a_folder_the_hulls_key_does_not_name() {
    let Some(path) = oag_testdata::exact("data/extracted/ps4") else {
        return;
    };
    let mut archives = oag_omega::open(&path.display().to_string()).expect("opening Omega");
    let liveries = zone_liveries(&mut archives, oag_omega::TITLE.plugin_definition);
    let report = zone_wreck_report(&mut archives, oag_omega::race::DEFAULTS, &liveries, "Qirex");
    let want = r"Data\art\published\hdships\Zone\shipwreck.vex: Qirex: Zone livery zoneship_quirex replaces zoneship_team on 0 texture request(s)";
    assert!(report.contains(want), "{report}");
}

/// A Zone end's shell and debris load from the Zone ship directory and split into the 112
/// pieces `DebrisManager_Construct` collects (`docs/ghidra/functions/ps3-hdfury-eu/zone-destroy.md`,
/// row Z11); the shell carries the one `Anim Transform` that plays it.
fn check_zone_end(
    title: &str,
    defaults: &oag_title::RaceDefaults,
    end: oag_title::destroyed::ZoneEnd,
    archives: &mut oag_assets::Archives,
    team: &str,
) {
    let mut report = Vec::new();
    let models = oag_livery::zone_end::load(
        archives,
        defaults.ships_for(team),
        Mode::Zone,
        Default::default(),
        end,
        &mut report,
    )
    .unwrap_or_else(|| panic!("{title}: the Zone end loads: {report:?}"));
    assert_eq!(
        models.pieces.len(),
        oag_livery::zone_end::PIECES,
        "{title}: the debris splits into its pieces: {report:?}"
    );
    assert!(
        models.pieces.iter().all(|piece| !piece.is_empty()),
        "{title}: every piece has vertices"
    );
    let vertices = models.debris.vertices.len() as u32;
    assert!(
        models.pieces.iter().flatten().all(|&v| v < vertices),
        "{title}: a piece's vertex is in the model"
    );
    assert!(
        !models.shell.anim_nodes.is_empty(),
        "{title}: the shell's Anim Transform survives the build"
    );
    assert!(
        models.shell.indices.len() >= 3,
        "{title}: the shell has geometry"
    );
}

#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn hd_zone_end_models_load_and_split_into_their_pieces() {
    let Some(image) = oag_testdata::image("data/images/hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let mut archives = oag_hd::open(&image.display().to_string()).expect("HD opens");
    check_zone_end(
        "HD",
        oag_hd::TITLE.race,
        oag_hd::TITLE
            .destroyed
            .value
            .zone_end
            .expect("HD ends Zone its own way"),
        &mut archives,
        oag_hd::TITLE.race.team,
    );
}

#[test]
#[ignore = "needs the decrypted Vita package in data/extracted/vita/"]
fn vita_2048_zone_end_models_load_and_split_into_their_pieces() {
    let Some(path) = oag_testdata::exact("data/extracted/vita/PCSF00007") else {
        return;
    };
    let mut archives = oag_2048::open(&path.display().to_string()).expect("opening 2048");
    check_zone_end(
        "2048",
        oag_2048::race::DEFAULTS,
        oag_2048::TITLE
            .destroyed
            .value
            .zone_end
            .expect("2048 inherits HD's Zone end"),
        &mut archives,
        oag_2048::race::DEFAULT_TEAM,
    );
}

#[test]
#[ignore = "needs the corrected Omega extraction"]
fn omega_zone_end_models_load_and_split_into_their_pieces() {
    let Some(path) = oag_testdata::exact("data/extracted/ps4") else {
        return;
    };
    let mut archives = oag_omega::open(&path.display().to_string()).expect("opening Omega");
    check_zone_end(
        "Omega",
        oag_omega::race::DEFAULTS,
        oag_omega::TITLE
            .destroyed
            .value
            .zone_end
            .expect("Omega inherits HD's Zone end"),
        &mut archives,
        oag_omega::race::DEFAULTS.team,
    );
}
