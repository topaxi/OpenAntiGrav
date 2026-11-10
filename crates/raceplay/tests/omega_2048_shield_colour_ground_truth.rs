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

fn shield_of(
    archives: &mut oag_assets::Archives,
    title: &'static oag_title::Title,
    team: &str,
) -> (Option<[f32; 4]>, bool, Vec<String>, Option<Lightning>) {
    let mut report = Vec::new();
    let liveries = oag_livery::load(
        archives,
        std::slice::from_ref(&team.to_string()),
        &oag_livery::LoadContext {
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
        },
        None,
        None,
        &[],
        &mut report,
    )
    .expect("the livery loads");
    let livery = &liveries[0];
    let built = livery
        .shield
        .as_ref()
        .is_some_and(|model| !model.indices.is_empty() && model.shield_shell);
    let lightning = livery.shield.as_ref().and_then(|model| {
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
    });
    (livery.shield_colour, built, report, lightning)
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
            let lightning = lightning
                .unwrap_or_else(|| panic!("{team}: no Lightning layer\n{}", report.join("\n")));
            assert!(lightning.picture, "{team}: no lightning picture bound");
            assert!(lightning.weight > 0.0, "{team}: no lightning weight");
            assert!((lightning.diffuse_scroll - 3.0).abs() < 1e-6, "{team}: A");
            assert!((lightning.noise_scale - 0.8).abs() < 1e-6, "{team}: B");
            assert!((lightning.noise_rate - 0.16).abs() < 1e-6, "{team}: C");
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
