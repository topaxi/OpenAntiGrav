//! Omega's and 2048's engine light, off the discs: the 2048-era craft's authored numbers,
//! the base each craft takes, and the records a started race submits.
//!
//! `docs/ghidra/functions/ps4-omega-eu/engine-light.md` and
//! `docs/ghidra/functions/vita-2048-eu-v104/engine-light.md`. **`#[ignore]`d and never run in
//! CI**; needs `data/extracted/ps4` and `data/extracted/vita/PCSF00007`.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-raceplay --run-ignored all engine_light_ground_truth
//! ```
//!
//! Each race test fails if the title's `engine_light` goes back to `None`, if the era is keyed
//! on the wrong directory, or if the colour is read off the skin alone.

use oag_gameplay::PlayerInputs;
use oag_raceplay as race;

const ERA_2048: [f32; 3] = [3.0, 0.5, 0.1];
const FURY: [f32; 3] = [40.0, 10.0, 4.0];
const CLASSIC: [f32; 3] = [4.0, 10.0, 40.0];

fn omega() -> Option<String> {
    let path = match std::env::var_os("OAG_OMEGA_SOURCE") {
        Some(path) => oag_testdata::repo_root().join(path),
        None => oag_testdata::exact("data/extracted/ps4")?,
    };
    Some(path.display().to_string())
}

fn vita() -> Option<String> {
    Some(
        oag_testdata::exact("data/extracted/vita/PCSF00007")?
            .display()
            .to_string(),
    )
}

/// Every id of a variant table, joined as the title's roster joins them.
fn ids(table: &oag_title::TeamVariants) -> Vec<String> {
    table
        .teams
        .iter()
        .flat_map(|team| {
            table
                .variants
                .iter()
                .map(|variant| table.join.combine(team, variant.suffix))
        })
        .collect()
}

/// All twenty 2048-era craft author `Distance 3` and `Radius 30`, the numbers the wide dim
/// light is built on. Read at the file, so a parser that read the wrong element shows.
fn every_2048_era_craft_authors_three_and_thirty(
    archives: &mut oag_assets::Archives,
    dir: &str,
    table: &oag_title::TeamVariants,
) {
    let all = ids(table);
    assert_eq!(all.len(), 20, "five teams of four craft");
    for id in all {
        let name = oag_tables::enginelight::entry_name_in(dir, &id);
        let blob = archives
            .read_name(&name)
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        let data = oag_tables::enginelight::parse(&String::from_utf8(blob).expect("text"))
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!((data.distance, data.radius), (3.0, 30.0), "{name}");
    }
}

#[test]
#[ignore = "needs data/extracted/ps4"]
fn omegas_twenty_2048_era_craft_author_distance_three_radius_thirty() {
    let Some(source) = omega() else { return };
    let mut archives = oag_omega::open(&source).expect("opening Omega");
    every_2048_era_craft_authors_three_and_thirty(
        &mut archives,
        oag_omega::race::ERA_2048_SHIP_DIR,
        &oag_omega::race::ERA_2048_VARIANTS,
    );
}

#[test]
#[ignore = "needs data/extracted/vita/PCSF00007"]
fn the_2048_native_twenty_craft_author_distance_three_radius_thirty() {
    let Some(source) = vita() else { return };
    let mut archives = oag_2048::open(&source).expect("opening 2048");
    every_2048_era_craft_authors_three_and_thirty(
        &mut archives,
        oag_2048::race::SHIP_DIR,
        &oag_2048::race::TEAM_VARIANTS,
    );
}

/// A race started on `source`, its slots' authored lights, and the records after a few ticks:
/// the craft's and the track's.
struct Started {
    authored: Vec<Option<oag_livery::engine_light::EngineLight>>,
    craft: Vec<oag_mesh::mesh_render::SpuLight>,
    track: Vec<oag_mesh::mesh_render::SpuLight>,
}

fn started(source: String, track: &str, team: &str) -> Started {
    let loaded = race::load(&race::Options {
        source,
        track: Some(track.to_string()),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        team: Some(team.to_string()),
        ..race::Options::default()
    })
    .expect("loading the race");
    for line in loaded.report.iter().filter(|l| l.contains("engine light")) {
        println!("{line}");
    }
    let authored = loaded.setup.engine_lights.clone();
    let mut race = race::Race::start(loaded.setup);
    for _ in 0..5 {
        race.tick(&PlayerInputs::none());
    }
    Started {
        authored,
        craft: race.hd_craft_spu_lights(),
        track: race.hd_spu_lights(),
    }
}

/// The colour with its boost gain divided out is not needed at rest: a craft on the grid has
/// `blend = 0`, so the record's colour is the base itself.
fn colour(light: &oag_mesh::mesh_render::SpuLight) -> [f32; 3] {
    [light.colour[0], light.colour[1], light.colour[2]]
}

fn some_light_is(started: &Started, base: [f32; 3]) -> bool {
    started.craft.iter().any(|light| colour(light) == base)
}

/// Player craft first: slot 0's authored light and the era it resolved to.
fn player_era(started: &Started) -> bool {
    started.authored[0].expect("the player's engine light").era_2048
}

#[test]
#[ignore = "needs data/extracted/ps4"]
fn an_omega_2048_era_craft_on_a_2048_circuit_casts_the_wide_dim_red_light_and_the_strip_gate_is_open()
 {
    let Some(source) = omega() else { return };
    let run = started(source, r"Data\Environments2048\mall\track.vex", r"Feisar2048\3");
    assert!(player_era(&run));
    let player = run.authored[0].unwrap();
    assert_eq!((player.data.distance, player.data.radius), (3.0, 30.0));
    assert!(player.strip_circuit, "a 2048-lineage circuit opens the strip gate");
    assert!(
        run.craft.iter().any(|light| {
            colour(light) == ERA_2048 && (29.0..=31.0).contains(&light.colour[3])
        }),
        "the player's light is (3.0, 0.5, 0.1) at range 30 +- 1: {:?}",
        run.craft.iter().map(|l| l.colour).collect::<Vec<_>>()
    );
    // Not bound to the track alone: the circuit's switch is on, so the track takes the same.
    assert_eq!(run.track.len(), run.craft.len());
}

#[test]
#[ignore = "needs data/extracted/ps4"]
fn an_omega_fury_craft_on_an_hd_circuit_casts_the_tight_warm_light_and_the_strip_gate_is_shut() {
    let Some(source) = omega() else { return };
    let run = started(source, r"Data\Environments\amphiseum\track.vex", "FEISAR_C1");
    assert!(!player_era(&run));
    let player = run.authored[0].unwrap();
    assert!(
        !player.strip_circuit,
        "an HD circuit is not a 2048 mode: the strip does not recolour the light"
    );
    assert!(
        some_light_is(&run, FURY),
        "a Fury skin is (40, 10, 4): {:?}",
        run.craft.iter().map(|l| l.colour).collect::<Vec<_>>()
    );
    for light in &run.craft {
        assert!(
            light.colour[3] <= 2.0 + 1.0 && light.colour[3] > 0.01,
            "an HD-era craft's range is Radius (at most 2) +- 1: {}",
            light.colour[3]
        );
    }
}

#[test]
#[ignore = "needs data/extracted/vita/PCSF00007"]
fn a_2048_native_craft_is_the_wide_dim_red_light_and_the_strip_gate_is_open() {
    let Some(source) = vita() else { return };
    let run = started(
        source,
        r"Data\art\published\environments\mall\track.vex",
        r"Feisar2048\3",
    );
    assert!(player_era(&run), "the native Ships\\ roster is the 2048 era");
    assert!(run.authored[0].unwrap().strip_circuit);
    assert!(some_light_is(&run, ERA_2048));
}

#[test]
#[ignore = "needs data/extracted/vita/PCSF00007"]
fn a_2048_hd_derived_craft_is_the_blue_hd_era_light() {
    let Some(source) = vita() else { return };
    let run = started(
        source,
        r"Data\art\published\DLC1\environments\Moa_Therma\track.vex",
        "Feisar",
    );
    assert!(
        !player_era(&run),
        "the hdships roster is not the 2048 era"
    );
    assert!(some_light_is(&run, CLASSIC));
}
