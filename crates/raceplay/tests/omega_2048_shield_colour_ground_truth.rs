//! Omega and 2048 build a shield shell for every team and read its own
//! `ShieldColour` (four `f32` on PS4, four halves on the Vita), not HD's.
//!
//! The expected tables are the censuses on
//! `docs/ghidra/functions/ps4-omega-eu/shield.md` and `.../vita-2048-eu-v104/shield.md`
//! ("The `ShieldColour` value, per team"). They live here and nowhere in the
//! engine.

use std::path::{Path, PathBuf};

const OMEGA: &[(&str, [f32; 3])] = &[
    ("feisar", [0.9584, 0.9956, 0.0]),
    ("assegai", [0.2110, 0.6567, 1.0]),
    ("piranha", [1.0, 1.0, 0.0]),
    ("qirex", [1.0, 0.0417, 1.0]),
    ("goteki", [0.7490, 0.2039, 0.7725]),
    ("egx", [0.2196, 1.0, 0.2196]),
    ("ag_systems", [0.2652, 1.0, 1.0]),
    ("triakis", [1.0, 0.9327, 0.2793]),
    ("zone", [1.0, 0.3022, 0.0]),
    ("auricom", [0.1050, 0.1050, 1.0]),
    ("harimau", [0.0, 1.0, 1.0]),
    ("icaras", [0.0, 0.0314, 0.9961]),
    ("mirage", [1.0, 1.0, 0.0]),
];

fn present(rel: &str) -> Option<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel);
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

/// One team's livery as a single race loads it, and the loader report.
fn livery_of(
    archives: &mut oag_assets::Archives,
    title: &'static oag_title::Title,
    team: &str,
) -> (oag_livery::Livery, Vec<String>) {
    let mut report = Vec::new();
    let mut liveries = oag_livery::load(
        archives,
        std::slice::from_ref(&team.to_string()),
        &oag_livery::LoadContext {
            store: Default::default(),
            race: title.race,
            mode: oag_race::Mode::SingleRace,
            flare: title.flare,
            hull_overlay: false,
            leach_overlay: false,
            hull_shine: false,
            hull_wreck: false,
            absorb_shell: false,
            own_shield: false,
            zone_liveries: &[],
            forced_skins: &[],
            circuit_hull: None,
            player_livery: None,
        },
        None,
        None,
        &[],
        &mut report,
    )
    .expect("the livery loads");
    (liveries.remove(0), report)
}

fn shield_of(
    archives: &mut oag_assets::Archives,
    title: &'static oag_title::Title,
    team: &str,
) -> (Option<[f32; 4]>, bool, Vec<String>, Option<Lightning>) {
    let (livery, report) = livery_of(archives, title, team);
    let built = livery
        .shield
        .as_ref()
        .is_some_and(|model| !model.indices.is_empty() && model.shield_shell);
    let lightning = livery.shield.as_ref().and_then(lightning_of);
    (livery.shield_colour, built, report, lightning)
}

fn lightning_of(model: &oag_mesh::mesh::Model) -> Option<Lightning> {
    let entry = model
        .emissive
        .iter()
        .find(|e| e.program[0] == oag_mesh::mesh::PROGRAM_SHIELD_LIGHTNING)?;
    Some(Lightning {
        weight: entry.tint[0],
        diffuse_scroll: entry.tint[1],
        noise_rate: entry.tint[2],
        noise_scale: entry.offset,
        picture: model.pad_masks.iter().any(Option::is_some),
    })
}

/// The layer's rates are literals of the program on both consoles: `3.0`, `0.8`, `0.16`.
fn assert_lightning(lightning: Option<Lightning>, what: &str) {
    let lightning = lightning.unwrap_or_else(|| panic!("{what}: no Lightning layer"));
    assert!(lightning.picture, "{what}: no lightning picture bound");
    assert!(lightning.weight > 0.0, "{what}: no lightning weight");
    assert!((lightning.diffuse_scroll - 3.0).abs() < 1e-6, "{what}: A");
    assert!((lightning.noise_scale - 0.8).abs() < 1e-6, "{what}: B");
    assert!((lightning.noise_rate - 0.16).abs() < 1e-6, "{what}: C");
}

/// The cockpit sphere, `hd_shipshield_old`: the same program as the shell's.
fn cockpit_lightning(archives: &mut oag_assets::Archives) -> Option<Lightning> {
    let mut report = Vec::new();
    let (sphere, _) = oag_livery::cockpit_shield(archives, Default::default(), &mut report);
    sphere.as_ref().and_then(lightning_of)
}

/// What a built shell carries of `2048_shipshield_old`'s Lightning layer.
struct Lightning {
    weight: f32,
    diffuse_scroll: f32,
    noise_rate: f32,
    noise_scale: f32,
    picture: bool,
}

#[test]
#[ignore = "needs data/extracted/ps4"]
fn every_omega_team_builds_a_shell_and_settles_to_its_own_colour() {
    let Some(path) = present("data/extracted/ps4") else {
        return;
    };
    let mut archives = oag_omega::open(&path.display().to_string()).expect("omega");
    for (team, expected) in OMEGA {
        let (colour, built, report, lightning) = shield_of(&mut archives, oag_omega::TITLE, team);
        // The layer's rates are literals of the pixel program (`3.0`, `0.8`, `0.16`), its
        // picture is `cf_plasma` on the third binding, and the weight is the team's own
        // `Lightning_RGB_Multiplier` (Assegai authors 3, Icaras 1).
        // Zone's shell is its own material (`zonebattle_shield`), a different program.
        if *team == "zone" {
            assert!(
                lightning.is_none(),
                "zone: another program, no Lightning layer"
            );
        } else {
            assert_lightning(lightning, team);
        }
        let colour =
            colour.unwrap_or_else(|| panic!("{team}: no ShieldColour\n{}", report.join("\n")));
        assert!(built, "{team}: no shell model\n{}", report.join("\n"));
        for (channel, want) in expected.iter().enumerate() {
            assert!(
                (colour[channel] - want).abs() < 1.0 / 255.0,
                "{team}: channel {channel} is {}, the census has {want}",
                colour[channel]
            );
        }
    }
}

#[test]
#[ignore = "needs data/extracted/ps4"]
fn omegas_cockpit_sphere_carries_the_shells_lightning() {
    let Some(path) = present("data/extracted/ps4") else {
        return;
    };
    let mut archives = oag_omega::open(&path.display().to_string()).expect("omega");
    assert_lightning(cockpit_lightning(&mut archives), "cockpit");
}

/// The Vita's own `.gxp` programs state the same literals for the HD-content shells and the
/// cockpit sphere.
#[test]
#[ignore = "needs data/extracted/vita/PCSF00007/base"]
fn every_2048_hd_content_shell_carries_the_programs_lightning() {
    let Some(path) = present("data/extracted/vita/PCSF00007/base") else {
        return;
    };
    let mut archives = oag_2048::open(&path.display().to_string()).expect("2048");
    for team in ["Feisar", "Assegai", "Qirex", "Piranha", "Goteki", "EGX"] {
        let (_, _, _, lightning) = shield_of(&mut archives, oag_2048::TITLE, team);
        assert_lightning(lightning, team);
    }
    assert_lightning(cockpit_lightning(&mut archives), "cockpit");
}

/// The magnetic shell's vertex program adds `time * Speed_multiplier` (`1`) to `u`, on the
/// Vita and on the PS4.
fn assert_magnetic_scroll(archives: &mut oag_assets::Archives, title: &'static oag_title::Title) {
    // Capitalised: Omega's directory lookup for the 2048-era craft is by this spelling.
    let (livery, report) = livery_of(archives, title, r"Feisar2048\1");
    let shell = livery
        .shield
        .as_ref()
        .unwrap_or_else(|| panic!("a shell\n{}", report.join("\n")));
    let track = shell
        .anim_tracks
        .iter()
        .find(|t| matches!(t, oag_mesh::mesh::AnimTrack::Scroll(_)))
        .expect("a scroll track");
    let (_, early) = track.sample(1000.0);
    let (_, later) = track.sample(1000.25);
    assert!(
        ((later[0] - early[0]).rem_euclid(1.0) - 0.25).abs() < 1e-3,
        "u moves a quarter in a quarter second: {early:?} {later:?}"
    );
    assert!((later[1] - early[1]).abs() < 1e-6, "v stays");
    assert!(
        shell.vertices.iter().any(|v| v.anim != 0),
        "no vertex names the track"
    );
}

#[test]
#[ignore = "needs data/extracted/vita/PCSF00007/base"]
fn the_2048_magnetic_shell_scrolls_its_u_at_the_authored_rate() {
    let Some(path) = present("data/extracted/vita/PCSF00007/base") else {
        return;
    };
    let mut archives = oag_2048::open(&path.display().to_string()).expect("2048");
    assert_magnetic_scroll(&mut archives, oag_2048::TITLE);
}

#[test]
#[ignore = "needs data/extracted/ps4"]
fn omegas_magnetic_shell_scrolls_its_u_at_the_authored_rate() {
    let Some(path) = present("data/extracted/ps4") else {
        return;
    };
    let mut archives = oag_omega::open(&path.display().to_string()).expect("omega");
    assert_magnetic_scroll(&mut archives, oag_omega::TITLE);
}

#[test]
#[ignore = "needs data/extracted/vita/PCSF00007/base"]
fn every_2048_team_builds_a_shell_and_settles_to_the_magnetic_colour() {
    let Some(path) = present("data/extracted/vita/PCSF00007/base") else {
        return;
    };
    let mut archives = oag_2048::open(&path.display().to_string()).expect("2048");
    for team in ["ag_systems", "auricom", "feisar", "piranha", "qirex"] {
        let team = format!(r"{team}2048\1");
        let (colour, built, report, lightning) = shield_of(&mut archives, oag_2048::TITLE, &team);
        assert!(
            lightning.is_none(),
            "{team}: the magnetic shield runs another program and has no Lightning layer"
        );
        let colour =
            colour.unwrap_or_else(|| panic!("{team}: no ShieldColour\n{}", report.join("\n")));
        assert!(built, "{team}: no shell model\n{}", report.join("\n"));
        assert!(colour[0].abs() < 1e-3, "{team}: red {}", colour[0]);
        assert!(
            (colour[1] - 4.5948).abs() < 0.01,
            "{team}: green {}",
            colour[1]
        );
        assert!(
            (colour[2] - 4.5948).abs() < 0.01,
            "{team}: blue {}",
            colour[2]
        );
    }
}

/// The team's wreck as a race that reads it loads it: `hull_wreck` on, the title's own rule.
fn wreck_of(
    archives: &mut oag_assets::Archives,
    title: &'static oag_title::Title,
    team: &str,
) -> (Option<oag_livery::Wreck>, Vec<String>) {
    let mut report = Vec::new();
    let mut liveries = oag_livery::load(
        archives,
        std::slice::from_ref(&team.to_string()),
        &oag_livery::LoadContext {
            store: Default::default(),
            race: title.race,
            mode: oag_race::Mode::SingleRace,
            flare: title.flare,
            hull_overlay: false,
            leach_overlay: false,
            hull_shine: false,
            hull_wreck: title.looks.hull_wreck.applies(archives.layout.platform),
            absorb_shell: false,
            own_shield: false,
            zone_liveries: &[],
            forced_skins: &[],
            circuit_hull: None,
            player_livery: None,
        },
        None,
        None,
        &[],
        &mut report,
    )
    .expect("the livery loads");
    (liveries.remove(0).wreck, report)
}

fn assert_wreck(wreck: Option<oag_livery::Wreck>, report: &[String], team: &str) {
    let wreck = wreck.unwrap_or_else(|| panic!("{team}: no wreck\n{}", report.join("\n")));
    assert!(
        wreck.model.label.ends_with("shipwreck.vex"),
        "{team}: {}",
        wreck.model.label
    );
    assert!(!wreck.model.indices.is_empty(), "{team}: an empty wreck");
}

#[test]
#[ignore = "needs data/extracted/ps4"]
fn every_omega_team_loads_a_wreck_and_so_does_every_2048_era_ship() {
    let Some(path) = present("data/extracted/ps4") else {
        return;
    };
    let mut archives = oag_omega::open(&path.display().to_string()).expect("omega");
    let era = ["AG_Systems", "Auricom", "Feisar", "Piranha", "Qirex"];
    let teams = OMEGA
        .iter()
        .map(|(team, _)| (*team).to_string())
        .chain(era.iter().map(|team| format!(r"{team}2048\1")));
    for team in teams {
        let (wreck, report) = wreck_of(&mut archives, oag_omega::TITLE, &team);
        assert_wreck(wreck, &report, &team);
    }
}

#[test]
#[ignore = "needs data/extracted/vita/PCSF00007/base"]
fn every_2048_team_loads_a_wreck() {
    let Some(path) = present("data/extracted/vita/PCSF00007/base") else {
        return;
    };
    let mut archives = oag_2048::open(&path.display().to_string()).expect("2048");
    let teams = ["ag_systems", "auricom", "feisar", "piranha", "qirex"]
        .map(|team| format!(r"{team}2048\1"))
        .into_iter()
        .chain(["Feisar", "Qirex"].map(String::from));
    for team in teams {
        let (wreck, report) = wreck_of(&mut archives, oag_2048::TITLE, &team);
        assert_wreck(wreck, &report, &team);
    }
}
